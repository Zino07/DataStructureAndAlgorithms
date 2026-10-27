"""Noor storage layer — transactional, typed, single public entry point.

Single backend: SQLite (WAL) on disk at ``data/noor.sqlite3`` (or ``NOOR_DB``).
The site is hosted on Octenium cPanel only; the former Vercel/Upstash Redis
backend was removed so there is one place where data lives.

Why this replaces the previous file-KV layer
--------------------------------------------
The previous layer performed non-atomic read-modify-write on JSON files and
wrote whole lists back with ``os.replace``. Two callers touching
``products.json`` / ``orders.json`` in the same instant could interleave, and a
crash between the two writes left stock decremented with no matching order.
This module makes the store the single owner of consistency:

  * SQLite in WAL mode, ``BEGIN IMMEDIATE`` per mutation: real ACID, real
    cross-document transactions (``with transaction():``).
  * ``next_number`` is an atomic SQL sequence, never derived from
    ``len(orders)``, so invoice numbers cannot collide or regress.
  * ``once(key, fn)`` gives atomic, durable idempotency for POST retries.
  * Corruption recovery: ``PRAGMA quick_check`` on boot; a damaged database is
    quarantined and rebuilt from the JSON files, then their ``.bak`` siblings.
  * Catalog JSON files under ROOT (products, settings, shipping) keep being
    mirrored atomically (temp + ``os.replace`` under a lock) as a read-mirror
    and seed. Customer data (orders, users, tokens, reviews) is NOT mirrored:
    it lives only in SQLite and in the compressed backups (``backup_now``).
  * Rate limits and login lockouts live in SQLite too, so every Passenger
    worker process shares one view of them.

Public API:
  load_json(path, default)  save_json(path, data)
  analytics_append(ev)      analytics_tail(n)
  next_number()             MUTEX()                StoreError

Added (opt-in):
  transaction()             atomic cross-document unit of work
  once(key, fn)             idempotent mutation
  idem_get(key)             health()
  rate_hit(key, maxn, per)  shared fixed-window rate limit
  guard_check/fail/reset    shared login lockout
  kv_get/kv_set/kv_take     small expiring key-value records
  backup_now()              compressed online SQLite backup
"""

from __future__ import annotations

import contextlib
import gzip
import json
import os
import random
import re
import secrets
import shutil
import sqlite3
import threading
import time
from dataclasses import dataclass, field
from typing import Any, Callable, Dict, Iterator, List, Optional, Tuple

# --------------------------------------------------------------------------- #
# Configuration
# --------------------------------------------------------------------------- #

ROOT: str = os.path.dirname(os.path.abspath(__file__))

# Auto-load .env before evaluating environment-dependent storage settings
_env_path = os.path.join(ROOT, ".env")
if os.path.isfile(_env_path):
    try:
        with open(_env_path, "r", encoding="utf-8") as _ef:
            for _line in _ef:
                _line = _line.strip()
                if _line and not _line.startswith("#") and "=" in _line:
                    _k, _v = _line.split("=", 1)
                    _k = _k.strip()
                    _v = _v.strip().strip("\"'")
                    if _k and _k not in os.environ:
                        os.environ[_k] = _v
    except OSError:
        pass

Json = Any

_DOC_DEFAULT: Dict[str, Json] = {
    "products.json": [],
    "orders.json": [],
    "users.json": {},
    "tokens.json": {},
    "reviews.json": [],
    "settings.json": {},
    "shipping.json": {},
    "counter.json": {"last": 0},
    "logo.json": {"data": "", "mime": ""},
    "admin.json": {},
}


# Files whose previous content is rotated to ``<name>.bak`` on every mirror.
_ROTATE_BAK = frozenset({"products.json", "orders.json", "users.json"})

_DOC_NAMES = frozenset(_DOC_DEFAULT)

ANALYTICS_MAX = 3000
IDEM_TTL = 24 * 3600
INVOICE_PREFIX = os.environ.get("INVOICE_PREFIX", "NOOR")
SCHEMA_VERSION = 2

_DB_ENV = os.environ.get("NOOR_DB") or os.environ.get("STORE_DB")
if _DB_ENV:
    DB_PATH = os.path.abspath(_DB_ENV)
else:
    DB_PATH = os.path.join(ROOT, "data", "noor.sqlite3")

BACKUP_DIR = os.path.join(os.path.dirname(DB_PATH), "backups")
BACKUP_KEEP = max(1, int(os.environ.get("BACKUP_KEEP", "14") or 14))

# Documents mirrored to ROOT as human-readable JSON and used to seed a fresh
# database. Customer data is deliberately absent; MIRROR_ALL=1 restores the old
# behaviour of mirroring every document.
_CATALOG_DOCS = frozenset({"products.json", "settings.json", "shipping.json"})
MIRROR_ALL = os.environ.get("MIRROR_ALL", "0") == "1"

# The ``*.json`` files are the recovery mirror for the local database and live
# in ROOT beside server.py.  A database redirected outside ROOT (a test or a
# scratch instance via NOOR_DB/STORE_DB) must never write those shipped files:
# that is exactly how a low-stock fixture once truncated the production catalog
# mirror.  Mirrors are therefore *written* only when the database lives inside
# ROOT; reads (seeding a fresh database) still come from ROOT.
try:
    MIRROR_ENABLED = os.path.commonpath([os.path.abspath(DB_PATH), ROOT]) == ROOT
except ValueError:
    MIRROR_ENABLED = False


class StoreError(Exception):
    """Raised for every store failure. Callers map it to a 500/503."""


# --------------------------------------------------------------------------- #
# Process / thread synchronisation
# --------------------------------------------------------------------------- #

_THREAD_MUTEX = threading.RLock()
_tls = threading.local()


def MUTEX() -> threading.RLock:
    """Process-wide re-entrant mutex guarding the JSON mirror.

    SQLite supplies the real cross-process serialisation (WAL + BEGIN
    IMMEDIATE); this lock only keeps mirror writes and legacy read-modify-write
    call sites from racing inside one interpreter.
    """
    return _THREAD_MUTEX


class FileLock:
    """Best-effort advisory inter-process lock (fcntl on POSIX, msvcrt on Windows)."""

    def __init__(self, path: str) -> None:
        self.path = path
        self._fh: Optional[Any] = None

    def __enter__(self) -> "FileLock":
        try:
            os.makedirs(os.path.dirname(self.path) or ".", exist_ok=True)
            self._fh = open(self.path, "a+b")
            if os.name == "nt":  # pragma: no cover - windows only
                import msvcrt

                msvcrt.locking(self._fh.fileno(), msvcrt.LK_LOCK, 1)
            else:
                import fcntl

                fcntl.flock(self._fh.fileno(), fcntl.LOCK_EX)
        except OSError:
            self._fh = None
        return self

    def __exit__(self, *exc: Any) -> bool:
        if self._fh is not None:
            try:
                if os.name == "nt":  # pragma: no cover - windows only
                    import msvcrt

                    msvcrt.locking(self._fh.fileno(), msvcrt.LK_UNLCK, 1)
                else:
                    import fcntl

                    fcntl.flock(self._fh.fileno(), fcntl.LOCK_UN)
            except OSError:
                pass
            finally:
                try:
                    self._fh.close()
                except OSError:
                    pass
                self._fh = None
        return False


def _atomic_write(path: str, payload: bytes) -> None:
    """Write ``payload`` to ``path`` via temp file + fsync + os.replace."""
    d = os.path.dirname(path) or "."
    os.makedirs(d, exist_ok=True)
    tmp = "%s.%s.tmp" % (path, secrets.token_hex(6))
    with open(tmp, "wb") as fh:
        fh.write(payload)
        fh.flush()
        try:
            os.fsync(fh.fileno())
        except OSError:
            pass
    os.replace(tmp, path)


def _read_json_file(path: str) -> Tuple[bool, Json]:
    try:
        with open(path, "r", encoding="utf-8") as fh:
            return True, json.load(fh)
    except (OSError, ValueError):
        return False, None


def _load_with_bak(name: str, default: Json) -> Json:
    """Read ``name``; on failure fall back to ``name.bak``; else ``default``."""
    live = os.path.join(ROOT, name)
    ok, data = _read_json_file(live)
    if ok:
        return data
    ok, data = _read_json_file(live + ".bak")
    if ok:
        return data
    return default


# --------------------------------------------------------------------------- #
# SQLite backend — authoritative, transactional
# --------------------------------------------------------------------------- #


@dataclass
class _TxnState:
    depth: int = 0
    conn: Optional[sqlite3.Connection] = None


def _sqlite_connect(path: str) -> sqlite3.Connection:
    os.makedirs(os.path.dirname(path) or ".", exist_ok=True)
    conn = sqlite3.connect(path, timeout=5.0, isolation_level=None, check_same_thread=False)
    conn.execute("PRAGMA busy_timeout=5000")
    conn.execute("PRAGMA foreign_keys=ON")
    conn.execute("PRAGMA journal_mode=WAL")
    conn.execute("PRAGMA synchronous=NORMAL")
    return conn


_DDL = (
    "CREATE TABLE IF NOT EXISTS docs ("
    " name TEXT PRIMARY KEY,"
    " json TEXT NOT NULL,"
    " updated REAL NOT NULL)",
    "CREATE TABLE IF NOT EXISTS seq ("
    " name TEXT PRIMARY KEY,"
    " value INTEGER NOT NULL)",
    "CREATE TABLE IF NOT EXISTS events ("
    " id INTEGER PRIMARY KEY AUTOINCREMENT,"
    " ts REAL NOT NULL,"
    " ev TEXT NOT NULL,"
    " payload TEXT NOT NULL)",
    "CREATE TABLE IF NOT EXISTS idem ("
    " key TEXT PRIMARY KEY,"
    " value TEXT NOT NULL,"
    " created REAL NOT NULL)",
    "CREATE TABLE IF NOT EXISTS rate ("
    " key TEXT PRIMARY KEY,"
    " start REAL NOT NULL,"
    " n INTEGER NOT NULL)",
    "CREATE TABLE IF NOT EXISTS guard ("
    " key TEXT PRIMARY KEY,"
    " fails TEXT NOT NULL,"
    " until REAL NOT NULL,"
    " updated REAL NOT NULL)",
    "CREATE TABLE IF NOT EXISTS kv ("
    " key TEXT PRIMARY KEY,"
    " value TEXT NOT NULL,"
    " exp REAL NOT NULL)",
    "CREATE INDEX IF NOT EXISTS events_id_desc ON events (id DESC)",
    "CREATE INDEX IF NOT EXISTS idem_created ON idem (created)",
    "CREATE INDEX IF NOT EXISTS rate_start ON rate (start)",
    "CREATE INDEX IF NOT EXISTS kv_exp ON kv (exp)",
)


class _SqliteBackend:
    name = "sqlite"

    def __init__(self, path: str) -> None:
        self.path = path
        self._tls_key = "_noor_sqlite_state"
        self._bootstrap()

    # -- connection / transaction plumbing ------------------------------- #

    def _state(self) -> _TxnState:
        st: Optional[_TxnState] = getattr(_tls, self._tls_key, None)
        if st is None:
            st = _TxnState()
            setattr(_tls, self._tls_key, st)
        if st.conn is None:
            st.conn = _sqlite_connect(self.path)
            try:
                self._ensure_schema(st.conn)
            except sqlite3.Error:
                pass
        return st

    def conn(self) -> sqlite3.Connection:
        return self._state().conn

    def in_txn(self) -> bool:
        return self._state().depth > 0

    @contextlib.contextmanager
    def transaction(self) -> Iterator[sqlite3.Connection]:
        st = self._state()
        outermost = st.depth == 0
        if outermost:
            try:
                st.conn.execute("BEGIN IMMEDIATE")  # type: ignore[union-attr]
            except sqlite3.Error as exc:
                raise StoreError("begin: %s" % exc) from exc
        st.depth += 1
        try:
            yield st.conn  # type: ignore[misc]
        except BaseException:
            st.depth -= 1
            if outermost:
                try:
                    st.conn.execute("ROLLBACK")  # type: ignore[union-attr]
                except sqlite3.Error:
                    pass
            raise
        else:
            st.depth -= 1
            if outermost:
                try:
                    st.conn.execute("COMMIT")  # type: ignore[union-attr]
                except sqlite3.Error as exc:
                    raise StoreError("commit: %s" % exc) from exc

    # -- bootstrap / corruption recovery --------------------------------- #

    def _quick_check_ok(self, conn: sqlite3.Connection) -> bool:
        try:
            row = conn.execute("PRAGMA quick_check").fetchone()
            return bool(row) and str(row[0]).lower() == "ok"
        except sqlite3.Error:
            return False

    def _ensure_schema(self, conn: sqlite3.Connection) -> None:
        """Apply the DDL. Idempotent, so it is safe on every new connection and
        is also the self-heal path if the database file is replaced underneath a
        live process (e.g. an operator rotating data/, or a stray ``rm``)."""
        for stmt in _DDL:
            conn.execute(stmt)

    def _healed(self, fn: Callable[[], Any]) -> Any:
        """Run ``fn``; if the schema has vanished, rebuild it once and retry.

        Turns the previously fatal ``no such table: docs`` into a transparent
        recovery instead of an unhandled 500."""
        try:
            return fn()
        except sqlite3.Error as exc:
            if "no such table" not in str(exc).lower():
                raise
            self._ensure_schema(self.conn())
            return fn()

    def _bootstrap(self) -> None:
        conn: Optional[sqlite3.Connection] = None
        try:
            conn = _sqlite_connect(self.path)
            healthy = self._quick_check_ok(conn)
        except sqlite3.Error:
            healthy = False
            if conn is not None:
                try:
                    conn.close()
                except sqlite3.Error:
                    pass
            conn = None
        if not healthy:
            self._quarantine()
            conn = _sqlite_connect(self.path)
        assert conn is not None
        for stmt in _DDL:
            conn.execute(stmt)
        conn.execute(
            "INSERT INTO docs (name, json, updated) VALUES ('__schema__', ?, ?) "
            "ON CONFLICT(name) DO UPDATE SET json=excluded.json, updated=excluded.updated",
            (json.dumps({"version": SCHEMA_VERSION}), time.time()),
        )
        # Serialise seeding: two processes booting at once must not both INSERT
        # the same document. BEGIN IMMEDIATE makes the loser wait, then its
        # INSERT OR IGNORE is a no-op and it reads the winner's row.
        self._bootstrap_lock = FileLock(os.path.join(
            os.path.dirname(self.path) or ".", ".bootstrap.lock"))
        with self._bootstrap_lock:
            with self.transaction() as lock_conn:
                self._seed_missing(lock_conn)
                self._reconcile_sequence(lock_conn)
        try:
            conn.close()
        except sqlite3.Error:
            pass

    def _quarantine(self) -> None:
        stamp = time.strftime("%Y%m%d-%H%M%S")
        for suffix in ("", "-wal", "-shm"):
            src = self.path + suffix
            if os.path.exists(src):
                dst = "%s.corrupt-%s%s" % (self.path, stamp, suffix)
                try:
                    os.replace(src, dst)
                except OSError:
                    try:
                        os.remove(src)
                    except OSError:
                        pass

    def _seed_missing(self, conn: sqlite3.Connection) -> None:
        for name, default in _DOC_DEFAULT.items():
            exists = conn.execute("SELECT 1 FROM docs WHERE name=?", (name,)).fetchone()
            if exists:
                continue
            data = _load_with_bak(name, default)
            conn.execute(
                "INSERT OR IGNORE INTO docs (name, json, updated) VALUES (?, ?, ?)",
                (name, json.dumps(data, ensure_ascii=False), time.time()),
            )

    def _reconcile_sequence(self, conn: sqlite3.Connection) -> None:
        """Set the invoice sequence floor above every existing order number."""
        row = conn.execute("SELECT value FROM seq WHERE name='order'").fetchone()
        if row is not None:
            return
        floor = 0
        try:
            orders = json.loads(
                conn.execute("SELECT json FROM docs WHERE name='orders.json'").fetchone()[0]
            )
        except (TypeError, ValueError, IndexError):
            orders = []
        pattern = re.compile(r"^%s-(\d+)$" % re.escape(INVOICE_PREFIX))
        if isinstance(orders, list):
            for order in orders:
                if isinstance(order, dict):
                    m = pattern.match(str(order.get("num", "")))
                    if m:
                        floor = max(floor, int(m.group(1)))
        conn.execute(
            "INSERT INTO seq (name, value) VALUES ('order', ?) "
            "ON CONFLICT(name) DO UPDATE SET value=MAX(value, excluded.value)",
            (floor,),
        )

    # -- document access -------------------------------------------------- #

    def get_doc(self, name: str, default: Json) -> Json:
        try:
            row = self._healed(lambda: self.conn().execute(
                "SELECT json FROM docs WHERE name=?", (name,)).fetchone())
        except sqlite3.Error as exc:
            raise StoreError("read %s: %s" % (name, exc)) from exc
        if row is None:
            data = _load_with_bak(name, default)
            self.put_doc(name, data)
            return data
        try:
            return json.loads(row[0])
        except ValueError:
            return default

    def has_doc(self, name: str) -> bool:
        try:
            return self._healed(lambda: self.conn().execute(
                "SELECT 1 FROM docs WHERE name=?", (name,)).fetchone()) is not None
        except sqlite3.Error:
            return False

    def put_doc(self, name: str, data: Json) -> None:
        payload = json.dumps(data, ensure_ascii=False)
        sql = ("INSERT INTO docs (name, json, updated) VALUES (?, ?, ?) "
               "ON CONFLICT(name) DO UPDATE SET json=excluded.json, updated=excluded.updated")
        if self.in_txn():
            self._healed(lambda: self.conn().execute(sql, (name, payload, time.time())))
        else:
            def _write() -> None:
                with self.transaction() as conn:
                    conn.execute(sql, (name, payload, time.time()))
            self._healed(_write)

    # -- sequence --------------------------------------------------------- #

    def next_number(self) -> int:
        with self.transaction() as conn:
            try:
                row = conn.execute(
                    "INSERT INTO seq (name, value) VALUES ('order', 1) "
                    "ON CONFLICT(name) DO UPDATE SET value=value+1 RETURNING value"
                ).fetchone()
                return int(row[0])
            except sqlite3.OperationalError:
                conn.execute(
                    "INSERT INTO seq (name, value) VALUES ('order', 1) "
                    "ON CONFLICT(name) DO UPDATE SET value=value+1"
                )
                row = conn.execute("SELECT value FROM seq WHERE name='order'").fetchone()
                return int(row[0])

    # -- analytics -------------------------------------------------------- #

    def analytics_append(self, ev: Json) -> None:
        payload = json.dumps(ev, ensure_ascii=False)
        ts = float(ev.get("t", time.time())) if isinstance(ev, dict) else time.time()
        label = str(ev.get("ev", "")) if isinstance(ev, dict) else ""

        def _append() -> None:
            with self.transaction() as conn:
                conn.execute(
                    "INSERT INTO events (ts, ev, payload) VALUES (?, ?, ?)",
                    (ts, label, payload),
                )
                conn.execute(
                    "DELETE FROM events WHERE id <= (SELECT MAX(id) - ? FROM events)",
                    (ANALYTICS_MAX,),
                )
        self._healed(_append)

    def analytics_tail(self, n: int) -> List[Json]:
        try:
            rows = self._healed(lambda: self.conn().execute(
                "SELECT payload FROM events ORDER BY id DESC LIMIT ?", (max(0, int(n)),)
            ).fetchall())
        except sqlite3.Error as exc:
            raise StoreError("analytics: %s" % exc) from exc
        out: List[Json] = []
        for (payload,) in reversed(rows):
            try:
                out.append(json.loads(payload))
            except ValueError:
                continue
        return out

    # -- idempotency ------------------------------------------------------ #

    def idem_get(self, key: str) -> Optional[Json]:
        if not key:
            return None
        row = self.conn().execute("SELECT value, created FROM idem WHERE key=?", (key,)).fetchone()
        if row is None:
            return None
        if time.time() - float(row[1]) > IDEM_TTL:
            self.conn().execute("DELETE FROM idem WHERE key=?", (key,))
            return None
        try:
            return json.loads(row[0])
        except ValueError:
            return None

    def idem_claim(self, key: str) -> bool:
        """Atomically reserve ``key``. True if newly claimed, False if present."""
        if not key:
            return True
        if self.in_txn():
            cur = self.conn().execute(
                "INSERT INTO idem (key, value, created) VALUES (?, '', ?) "
                "ON CONFLICT(key) DO NOTHING",
                (key, time.time()),
            )
            return cur.rowcount == 1
        with self.transaction() as conn:
            cur = conn.execute(
                "INSERT INTO idem (key, value, created) VALUES (?, '', ?) "
                "ON CONFLICT(key) DO NOTHING",
                (key, time.time()),
            )
            return cur.rowcount == 1

    def idem_settle(self, key: str, value: Json) -> None:
        if not key:
            return
        with self.transaction() as conn:
            conn.execute(
                "INSERT INTO idem (key, value, created) VALUES (?, ?, ?) "
                "ON CONFLICT(key) DO UPDATE SET value=excluded.value, created=excluded.created",
                (key, json.dumps(value, ensure_ascii=False), time.time()),
            )

    # -- health ----------------------------------------------------------- #

    def health(self) -> "StoreHealth":
        ok = self._quick_check_ok(self.conn())
        docs = self.conn().execute("SELECT COUNT(*) FROM docs").fetchone()[0]
        ev = self.conn().execute("SELECT COUNT(*) FROM events").fetchone()[0]
        return StoreHealth(backend=self.name, target=self.path, healthy=ok,
                           documents=int(docs), events=int(ev))


    # -- shared rate limits / login lockout ------------------------------ #

    def _prune_small_tables(self, conn: sqlite3.Connection, now: float) -> None:
        conn.execute("DELETE FROM rate WHERE start < ?", (now - 6 * 3600,))
        conn.execute("DELETE FROM guard WHERE until < ? AND updated < ?", (now, now - 6 * 3600))
        conn.execute("DELETE FROM kv WHERE exp < ?", (now,))

    def rate_hit(self, key: str, maxn: int, per: float) -> bool:
        """Count one hit for ``key`` in a fixed window. True when over ``maxn``."""
        now = time.time()

        def _hit() -> bool:
            with self.transaction() as conn:
                row = conn.execute("SELECT start, n FROM rate WHERE key=?", (key,)).fetchone()
                if row is None or now - float(row[0]) > per:
                    conn.execute(
                        "INSERT INTO rate (key, start, n) VALUES (?, ?, 1) "
                        "ON CONFLICT(key) DO UPDATE SET start=excluded.start, n=1",
                        (key, now))
                    n = 1
                else:
                    n = int(row[1]) + 1
                    conn.execute("UPDATE rate SET n=? WHERE key=?", (n, key))
                if random.random() < 0.01:
                    self._prune_small_tables(conn, now)
                return n > maxn
        return self._healed(_hit)

    def _guard_row(self, conn: sqlite3.Connection, key: str, window: float, now: float) -> Tuple[List[float], float]:
        row = conn.execute("SELECT fails, until FROM guard WHERE key=?", (key,)).fetchone()
        if row is None:
            return [], 0.0
        try:
            fails = [float(t) for t in json.loads(row[0]) if now - float(t) < window]
        except (ValueError, TypeError):
            fails = []
        return fails, float(row[1] or 0)

    def _guard_put(self, conn: sqlite3.Connection, key: str, fails: List[float], until: float, now: float) -> None:
        conn.execute(
            "INSERT INTO guard (key, fails, until, updated) VALUES (?, ?, ?, ?) "
            "ON CONFLICT(key) DO UPDATE SET fails=excluded.fails, until=excluded.until, "
            "updated=excluded.updated",
            (key, json.dumps(fails[-50:]), until, now))

    def guard_check(self, key: str, max_fails: int, window: float, lockout: float) -> Tuple[bool, int]:
        now = time.time()

        def _check() -> Tuple[bool, int]:
            with self.transaction() as conn:
                fails, until = self._guard_row(conn, key, window, now)
                if until > now:
                    return False, int(until - now) + 1
                if len(fails) >= max_fails:
                    self._guard_put(conn, key, fails, now + lockout, now)
                    return False, int(lockout)
                return True, 0
        return self._healed(_check)

    def guard_fail(self, key: str, max_fails: int, window: float, lockout: float) -> None:
        now = time.time()

        def _fail() -> None:
            with self.transaction() as conn:
                fails, until = self._guard_row(conn, key, window, now)
                fails.append(now)
                if len(fails) >= max_fails:
                    until = now + lockout
                self._guard_put(conn, key, fails, until, now)
        self._healed(_fail)

    def guard_reset(self, key: str) -> None:
        def _reset() -> None:
            with self.transaction() as conn:
                conn.execute("DELETE FROM guard WHERE key=?", (key,))
        self._healed(_reset)

    # -- small expiring key/value records --------------------------------- #

    def kv_get(self, key: str) -> Optional[Json]:
        row = self._healed(lambda: self.conn().execute(
            "SELECT value, exp FROM kv WHERE key=?", (key,)).fetchone())
        if row is None or float(row[1]) <= time.time():
            return None
        try:
            return json.loads(row[0])
        except ValueError:
            return None

    def kv_set(self, key: str, value: Json, ttl: float, only_if_absent: bool = False) -> bool:
        """Store ``value`` for ``ttl`` seconds. With ``only_if_absent`` the write
        happens only when no live record exists; returns whether it was written."""
        now = time.time()
        payload = json.dumps(value, ensure_ascii=False)

        def _set() -> bool:
            with self.transaction() as conn:
                if only_if_absent:
                    row = conn.execute("SELECT exp FROM kv WHERE key=?", (key,)).fetchone()
                    if row is not None and float(row[0]) > now:
                        return False
                conn.execute(
                    "INSERT INTO kv (key, value, exp) VALUES (?, ?, ?) "
                    "ON CONFLICT(key) DO UPDATE SET value=excluded.value, exp=excluded.exp",
                    (key, payload, now + ttl))
                return True
        return self._healed(_set)

    def kv_take(self, key: str) -> Optional[Json]:
        """Atomically read and delete a live record (single-use tokens)."""
        now = time.time()

        def _take() -> Optional[Json]:
            with self.transaction() as conn:
                row = conn.execute("SELECT value, exp FROM kv WHERE key=?", (key,)).fetchone()
                if row is None:
                    return None
                conn.execute("DELETE FROM kv WHERE key=?", (key,))
                if float(row[1]) <= now:
                    return None
                try:
                    return json.loads(row[0])
                except ValueError:
                    return None
        return self._healed(_take)

    # -- backup ----------------------------------------------------------- #

    def backup_now(self, directory: str, keep: int) -> str:
        """Consistent online copy of the database, gzip-compressed.

        Uses SQLite's backup API, so it is safe while the shop is taking
        orders. Keeps the newest ``keep`` files and returns the new path."""
        os.makedirs(directory, exist_ok=True)
        stamp = time.strftime("%Y%m%d-%H%M%S", time.gmtime())
        raw = os.path.join(directory, "noor-%s-%s.sqlite3" % (stamp, secrets.token_hex(3)))
        dst = sqlite3.connect(raw)
        try:
            src = _sqlite_connect(self.path)
            try:
                src.backup(dst)
            finally:
                src.close()
        finally:
            dst.close()
        out = raw + ".gz"
        with open(raw, "rb") as fin, gzip.open(out + ".part", "wb", 6) as fout:
            shutil.copyfileobj(fin, fout)
        os.replace(out + ".part", out)
        os.remove(raw)
        try:
            os.chmod(out, 0o600)
        except OSError:
            pass
        old = sorted(f for f in os.listdir(directory)
                     if f.startswith("noor-") and f.endswith(".sqlite3.gz"))
        for f in old[:-keep]:
            try:
                os.remove(os.path.join(directory, f))
            except OSError:
                pass
        return out


# --------------------------------------------------------------------------- #
# Backend (single owner of the module-level state)
# --------------------------------------------------------------------------- #

_BACKEND = _SqliteBackend(DB_PATH)


# --------------------------------------------------------------------------- #
# JSON mirror (read-only convenience surface; never the source of truth)
# --------------------------------------------------------------------------- #


def _mirrored(name: str) -> bool:
    return MIRROR_ENABLED and name in _DOC_NAMES and (MIRROR_ALL or name in _CATALOG_DOCS)


def _mirror(name: str, data: Json) -> None:
    if not _mirrored(name):
        return
    path = os.path.join(ROOT, name)
    payload = json.dumps(data, ensure_ascii=False, indent=1).encode("utf-8")
    try:
        with FileLock(os.path.join(ROOT, ".mirror.lock")):
            if name in _ROTATE_BAK and os.path.isfile(path):
                ok, old = _read_json_file(path)
                if ok:
                    try:
                        _atomic_write(path + ".bak", json.dumps(old, ensure_ascii=False, indent=1).encode("utf-8"))
                    except OSError:
                        pass
            _atomic_write(path, payload)
    except OSError:
        # The backend is authoritative; the mirror is a best-effort convenience
        # surface (e.g. a read-only deployment filesystem must not break writes).
        pass


# --------------------------------------------------------------------------- #
# Public API
# --------------------------------------------------------------------------- #


def mirror_all() -> int:
    """Rewrite the catalog JSON mirrors from the authoritative backend.

    The ``*.json`` files next to the backend are a read-only convenience and
    disaster-recovery surface: ``_seed_missing`` re-seeds a lost/fresh database
    from them.  Any document written to the backend without going through
    ``save_json`` (seed, self-heal, migration) can therefore leave a stale
    mirror behind, which would silently truncate the recovered catalog.  Call
    this at startup to make drift self-healing.  Existing ``*.bak`` files are
    left untouched so they remain a second recovery copy.  Returns the number
    of documents re-synced.

    Customer documents (orders, users, tokens, reviews, ...) are not mirrored.
    Copies left behind by older versions are removed once the database holds
    the document, so personal data exists only in SQLite and its backups.
    """
    written = 0
    if not MIRROR_ENABLED:
        return written
    for name in sorted(_DOC_NAMES):
        if not _mirrored(name):
            if _BACKEND.has_doc(name):
                for suffix in ("", ".bak"):
                    try:
                        os.remove(os.path.join(ROOT, name + suffix))
                    except OSError:
                        pass
            continue
        try:
            data = _BACKEND.get_doc(name, _DOC_DEFAULT[name])
        except StoreError:
            continue
        if data is None:
            data = _DOC_DEFAULT[name]
        payload = json.dumps(data, ensure_ascii=False, indent=1).encode("utf-8")
        try:
            with FileLock(os.path.join(ROOT, ".mirror.lock")):
                _atomic_write(os.path.join(ROOT, name), payload)
        except OSError:
            continue
        written += 1
    return written


def _name(path: str) -> str:
    return os.path.basename(path)


def load_json(path: str, default: Json = None) -> Json:
    name = _name(path)
    if name not in _DOC_NAMES:
        ok, data = _read_json_file(path)
        return data if ok else default
    return _BACKEND.get_doc(name, _DOC_DEFAULT.get(name, default))


def save_json(path: str, data: Json) -> None:
    name = _name(path)
    if name not in _DOC_NAMES:
        _atomic_write(path, json.dumps(data, ensure_ascii=False, indent=1).encode("utf-8"))
        return
    _BACKEND.put_doc(name, data)
    _mirror(name, data)


def analytics_append(ev: Json) -> None:
    _BACKEND.analytics_append(ev)


def analytics_tail(n: int) -> List[Json]:
    return _BACKEND.analytics_tail(n)


def next_number() -> int:
    """Monotonic, gap-tolerant invoice sequence. Safe to call concurrently."""
    return _BACKEND.next_number()


def idem_get(key: str) -> Optional[Json]:
    return _BACKEND.idem_get(key)


def once(key: str, fn: Callable[[], Json]) -> Tuple[Json, bool]:
    """Run ``fn`` at most once per ``key``.

    Returns ``(value, deduplicated)``. Concurrent callers sharing ``key``
    execute ``fn`` exactly once; the rest receive the stored result. The claim
    and the result are written in one transaction, so a retried POST can never
    double-charge or double-decrement stock.
    """
    if not key:
        return fn(), False
    with transaction():
        if not _BACKEND.idem_claim(key):
            return _BACKEND.idem_get(key), True
        value = fn()
        _BACKEND.idem_settle(key, value)
        return value, False


def rate_hit(key: str, maxn: int, per: float = 60) -> bool:
    """Shared fixed-window limiter. True when ``key`` is over ``maxn`` hits."""
    return _BACKEND.rate_hit(key, int(maxn), float(per))


def guard_check(key: str, max_fails: int, window: float, lockout: float) -> Tuple[bool, int]:
    return _BACKEND.guard_check(key, int(max_fails), float(window), float(lockout))


def guard_fail(key: str, max_fails: int, window: float, lockout: float) -> None:
    _BACKEND.guard_fail(key, int(max_fails), float(window), float(lockout))


def guard_reset(key: str) -> None:
    _BACKEND.guard_reset(key)


def kv_get(key: str) -> Optional[Json]:
    return _BACKEND.kv_get(key)


def kv_set(key: str, value: Json, ttl: float, only_if_absent: bool = False) -> bool:
    return _BACKEND.kv_set(key, value, float(ttl), only_if_absent)


def kv_take(key: str) -> Optional[Json]:
    return _BACKEND.kv_take(key)


def backup_now() -> str:
    """Write ``data/backups/noor-<utc-time>-<id>.sqlite3.gz``; returns its path."""
    return _BACKEND.backup_now(BACKUP_DIR, BACKUP_KEEP)


@contextlib.contextmanager
def transaction() -> Iterator[Any]:
    """Atomic unit of work spanning every store this module owns."""
    with MUTEX():
        with _BACKEND.transaction() as handle:
            yield handle


# --------------------------------------------------------------------------- #
# Health
# --------------------------------------------------------------------------- #


@dataclass
class StoreHealth:
    backend: str
    target: str
    healthy: bool
    documents: int = 0
    events: int = 0
    detail: Dict[str, Any] = field(default_factory=dict)


def health() -> StoreHealth:
    try:
        return _BACKEND.health()
    except StoreError:
        return StoreHealth(backend=_BACKEND.name, target=getattr(_BACKEND, "path", ""), healthy=False)
