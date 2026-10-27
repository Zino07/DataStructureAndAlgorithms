"""Noor storage layer — transactional, typed, single public entry point.

Backend is selected by environment (unchanged contract):
  UPSTASH_REDIS_REST_URL + UPSTASH_REDIS_REST_TOKEN set -> Redis (serverless)
  otherwise                                            -> SQLite (WAL) on disk

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
  * JSON files under ROOT keep being mirrored atomically (temp + ``os.replace``
    under a lock) so the CDN and human inspection still work, but they are a
    read-mirror, never the source of truth.

Public API:
  load_json(path, default)  save_json(path, data)
  analytics_append(ev)      analytics_tail(n)
  next_number()             MUTEX()                StoreError

Added (opt-in):
  transaction()             atomic cross-document unit of work
  once(key, fn)             idempotent mutation
  idem_get(key)             health()
"""

from __future__ import annotations

import contextlib
import json
import os
import re
import secrets
import sqlite3
import threading
import time
import urllib.error
import urllib.request
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
}


# Files whose previous content is rotated to ``<name>.bak`` on every mirror.
_ROTATE_BAK = frozenset({"products.json", "orders.json", "users.json"})

_DOC_NAMES = frozenset(_DOC_DEFAULT)

ANALYTICS_MAX = 3000
IDEM_TTL = 24 * 3600
INVOICE_PREFIX = os.environ.get("INVOICE_PREFIX", "NOOR")
SCHEMA_VERSION = 2

RURL = os.environ.get("UPSTASH_REDIS_REST_URL", "").rstrip("/")
RTOKEN = os.environ.get("UPSTASH_REDIS_REST_TOKEN", "")
USE_REDIS = bool(RURL and RTOKEN)
PREFIX = os.environ.get("STORE_PREFIX", "noor:")

_DB_ENV = os.environ.get("NOOR_DB") or os.environ.get("STORE_DB")
if _DB_ENV:
    DB_PATH = os.path.abspath(_DB_ENV)
elif os.environ.get("VERCEL"):
    # Vercel's filesystem is read-only except /tmp; keep the DB out of ROOT.
    DB_PATH = os.path.join("/tmp", "noor.sqlite3")
else:
    DB_PATH = os.path.join(ROOT, "data", "noor.sqlite3")

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
    call sites from racing inside one interpreter. Redis uses a distributed
    lock instead (see :class:`_RedisBackend`).
    """
    if USE_REDIS:
        return _REDIS.lock()  # type: ignore[return-value]
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
    "CREATE INDEX IF NOT EXISTS events_id_desc ON events (id DESC)",
    "CREATE INDEX IF NOT EXISTS idem_created ON idem (created)",
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


# --------------------------------------------------------------------------- #
# Redis backend — serverless, lock-serialised, atomic single-key writes
# --------------------------------------------------------------------------- #

_UNLOCK_LUA = (
    "if redis.call('get', KEYS[1]) == ARGV[1] then "
    "return redis.call('del', KEYS[1]) else return 0 end"
)


class _RedisBackend:
    name = "redis"

    def __init__(self, url: str, token: str) -> None:
        self.url = url
        self.token = token
        self._locks = threading.local()

    def _call(self, *args: Json) -> Json:
        body = json.dumps(list(args)).encode("utf-8")
        req = urllib.request.Request(
            self.url,
            data=body,
            method="POST",
            headers={"Authorization": "Bearer " + self.token, "Content-Type": "application/json"},
        )
        try:
            with urllib.request.urlopen(req, timeout=8) as resp:
                out = json.loads(resp.read().decode("utf-8"))
        except (urllib.error.URLError, OSError, ValueError) as exc:
            raise StoreError("redis: %s" % exc) from exc
        if isinstance(out, dict) and out.get("error"):
            raise StoreError("redis: %s" % out["error"])
        return out.get("result") if isinstance(out, dict) else None

    def _key(self, name: str) -> str:
        return PREFIX + name

    # distributed, re-entrant, CAS-released lock
    def lock(self, ttl: int = 15, wait: float = 8.0) -> "threading.RLock":
        outer = self

        class _ReentrantRedisLock:
            def __enter__(self_inner) -> "_ReentrantRedisLock":  # noqa: N805
                depth = getattr(outer._locks, "depth", 0)
                if depth > 0:
                    outer._locks.depth = depth + 1
                    return self_inner
                token = secrets.token_hex(8)
                deadline = time.time() + wait
                while True:
                    if outer._call("SET", PREFIX + "mutex", token, "NX", "EX", str(ttl)) == "OK":
                        outer._locks.depth = 1
                        outer._locks.token = token
                        return self_inner
                    if time.time() > deadline:
                        raise StoreError("lock timeout")
                    time.sleep(0.05)

            def __exit__(self_inner, *exc: Any) -> bool:  # noqa: N805
                depth = getattr(outer._locks, "depth", 0)
                if depth > 1:
                    outer._locks.depth = depth - 1
                    return False
                token = getattr(outer._locks, "token", "")
                try:
                    outer._call("EVAL", _UNLOCK_LUA, "1", PREFIX + "mutex", token)
                except StoreError:
                    pass
                outer._locks.depth = 0
                return False

        return _ReentrantRedisLock()  # type: ignore[return-value]

    @contextlib.contextmanager
    def transaction(self) -> Iterator[Any]:
        """Redis has no cross-key ACID here; serialise on the distributed lock."""
        with self.lock():
            yield None

    # documents are a single JSON string per key -> replace is one atomic SET
    def get_doc(self, name: str, default: Json) -> Json:
        raw = self._call("GET", self._key(name))
        if raw is None:
            data = _load_with_bak(name, default)
            if data is not None:
                self._call("SET", self._key(name), json.dumps(data, ensure_ascii=False))
            return data
        try:
            return json.loads(raw)
        except ValueError:
            return default

    def put_doc(self, name: str, data: Json) -> None:
        self._call("SET", self._key(name), json.dumps(data, ensure_ascii=False))

    def next_number(self) -> int:
        key = self._key("counter.json")
        if self._call("GET", key) is None:
            floor = self._sequence_floor()
            self._call("SET", key, str(floor), "NX")
        try:
            return int(self._call("INCR", key))
        except (TypeError, ValueError) as exc:
            raise StoreError("counter: %s" % exc) from exc

    def _sequence_floor(self) -> int:
        orders = self.get_doc("orders.json", [])
        floor = 0
        pattern = re.compile(r"^%s-(\d+)$" % re.escape(INVOICE_PREFIX))
        if isinstance(orders, list):
            for order in orders:
                if isinstance(order, dict):
                    m = pattern.match(str(order.get("num", "")))
                    if m:
                        floor = max(floor, int(m.group(1)))
        return floor

    def analytics_append(self, ev: Json) -> None:
        key = self._key("analytics")
        self._call("RPUSH", key, json.dumps(ev, ensure_ascii=False))
        self._call("LTRIM", key, -ANALYTICS_MAX, -1)

    def analytics_tail(self, n: int) -> List[Json]:
        raw = self._call("LRANGE", self._key("analytics"), -max(0, int(n)), -1) or []
        out: List[Json] = []
        for item in raw:
            try:
                out.append(json.loads(item))
            except ValueError:
                continue
        return out

    def idem_get(self, key: str) -> Optional[Json]:
        if not key:
            return None
        raw = self._call("GET", self._key("idem:" + key))
        if raw is None:
            return None
        try:
            return json.loads(raw)
        except ValueError:
            return None

    def idem_claim(self, key: str) -> bool:
        if not key:
            return True
        return self._call("SET", self._key("idem:" + key), "", "NX", "EX", str(IDEM_TTL)) == "OK"

    def idem_settle(self, key: str, value: Json) -> None:
        if not key:
            return
        self._call("SET", self._key("idem:" + key), json.dumps(value, ensure_ascii=False),
                   "EX", str(IDEM_TTL))

    def health(self) -> "StoreHealth":
        try:
            self._call("PING")
            ok = True
        except StoreError:
            ok = False
        return StoreHealth(backend=self.name, target=self.url, healthy=ok)


# --------------------------------------------------------------------------- #
# Backend selection (single owner of the module-level state)
# --------------------------------------------------------------------------- #

_BACKEND: Any
if USE_REDIS:
    _REDIS = _RedisBackend(RURL, RTOKEN)
    _BACKEND = _REDIS
else:
    _REDIS = None  # type: ignore[assignment]
    _BACKEND = _SqliteBackend(DB_PATH)


# --------------------------------------------------------------------------- #
# JSON mirror (read-only convenience surface; never the source of truth)
# --------------------------------------------------------------------------- #


def _mirror(name: str, data: Json) -> None:
    if name not in _DOC_NAMES or not MIRROR_ENABLED:
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
    """Rewrite every JSON mirror from the authoritative backend.

    The ``*.json`` files next to the backend are a read-only convenience and
    disaster-recovery surface: ``_seed_missing`` re-seeds a lost/fresh database
    from them.  Any document written to the backend without going through
    ``save_json`` (seed, self-heal, migration) can therefore leave a stale
    mirror behind, which would silently truncate the recovered catalog.  Call
    this at startup to make drift self-healing.  Existing ``*.bak`` files are
    left untouched so they remain a second recovery copy.  Returns the number
    of documents re-synced.
    """
    written = 0
    if not MIRROR_ENABLED:
        return written
    for name in sorted(_DOC_NAMES):
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
