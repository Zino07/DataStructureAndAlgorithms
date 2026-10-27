"""Noor store backend — stdlib only. Static files + JSON API + admin + reviews."""
import base64
import email.utils
import functools
import gzip as _gzip
import hashlib
import hmac
import json
import os
import re
import secrets
import shutil
import signal
import struct
import subprocess
import sys
import threading
import time
import urllib.parse
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

ROOT = os.path.dirname(os.path.abspath(__file__))

# Auto-load .env if present (must happen before importing store)
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
    except Exception:
        pass

import store as _store  # noqa: E402
from store import (MUTEX, StoreError, analytics_append, analytics_tail,  # noqa: E402
                   load_json, save_json, next_number as store_next_number,
                   transaction, once)
import mailer  # noqa: E402  (reads SMTP settings from the .env loaded above)

PRODUCTS_FILE = os.path.join(ROOT, "products.json")
ORDERS_FILE = os.path.join(ROOT, "orders.json")
USERS_FILE = os.path.join(ROOT, "users.json")
TOKENS_FILE = os.path.join(ROOT, "tokens.json")
REVIEWS_FILE = os.path.join(ROOT, "reviews.json")
COUNTER_FILE = os.path.join(ROOT, "counter.json")
SHIPPING_FILE = os.path.join(ROOT, "shipping.json")
SETTINGS_FILE = os.path.join(ROOT, "settings.json")
LOGO_FILE = os.path.join(ROOT, "logo.json")
ADMIN_FILE = os.path.join(ROOT, "admin.json")   # SQLite document; never mirrored to disk

# --- Security configuration (fail-closed) ---------------------------------
# Admin sign-in is a username + password (+ authenticator code once 2-step
# verification is turned on), stored hashed in the database (admin.json).
# ADMIN_PIN is only a one-time bootstrap: it works until the owner creates
# that username and password in Admin -> Settings, then it stops working.
# The PIN has NO default. A missing, short, or well-known PIN leaves the
# bootstrap login DISABLED rather than silently exposing a shipped backdoor.
ADMIN_PIN = os.environ.get("ADMIN_PIN", "")
_WEAK_PINS = {
    "noor-admin", "luna-admin", "admin", "administrator", "password", "passwd",
    "pass", "pin", "secret", "changeme", "letmein", "1234", "12345", "123456",
    "1234567", "12345678", "123456789", "0000", "00000000", "qwerty",
}
PIN_USABLE = len(ADMIN_PIN) >= 8 and ADMIN_PIN.lower() not in _WEAK_PINS
ADMIN_USER_RE = re.compile(r"^[A-Za-z0-9._-]{3,40}$")
ADMIN_PASS_MIN = 10
TOTP_ISSUER = os.environ.get("TOTP_ISSUER", "") or "Noor store"

ADMIN_TOKEN_TTL = 12 * 3600
USER_TOKEN_TTL = 30 * 86400
TOKEN_BYTES = 32                      # secrets.token_hex(32) -> 256-bit bearer token

TRUST_PROXY = os.environ.get("TRUST_PROXY", "0") == "1"
TRUSTED_PROXIES = set(
    ip.strip()
    for ip in os.environ.get("TRUSTED_PROXIES", "127.0.0.1,::1").split(",")
    if ip.strip()
)
ENABLE_HSTS = os.environ.get("BEHIND_TLS", "1") == "1"
COOKIE_SECURE = ENABLE_HSTS           # Secure flag whenever TLS is terminated upstream

# Brute-force guard: LOGIN_MAX_FAILS failed admin logins inside LOGIN_WINDOW
# seconds trigger a LOGIN_LOCKOUT-second lockout for that client.
LOGIN_MAX_FAILS = int(os.environ.get("LOGIN_MAX_FAILS", "5") or 5)
LOGIN_WINDOW = int(os.environ.get("LOGIN_WINDOW", "900") or 900)
LOGIN_LOCKOUT = int(os.environ.get("LOGIN_LOCKOUT", "900") or 900)

COOKIE_ADMIN = "noor_admin"
COOKIE_USER = "noor_user"

# Automatic database backups: at most one per BACKUP_EVERY seconds, triggered
# by normal shop traffic (no cron job needed). BACKUP_EMAIL receives a copy so
# a backup also exists off the server.
BACKUP_EVERY = int(os.environ.get("BACKUP_EVERY", "86400") or 86400)
BACKUP_EMAIL = os.environ.get("BACKUP_EMAIL", "").strip()
RESET_TTL = 3600

THREAD_SEMAPHORE = threading.BoundedSemaphore(64)

MIME = {
    ".html": "text/html; charset=utf-8",
    ".css": "text/css; charset=utf-8",
    ".js": "text/javascript; charset=utf-8",
    ".json": "application/json; charset=utf-8",
    ".png": "image/png",
    ".jpg": "image/jpeg",
    ".jpeg": "image/jpeg",
    ".webp": "image/webp",
    ".woff2": "font/woff2",
    ".svg": "image/svg+xml",
    ".xml": "application/xml; charset=utf-8",
    ".txt": "text/plain; charset=utf-8",
    ".ico": "image/x-icon",
}

SHIP_DEFAULT = [800, 500]
SHIP_MAJOR = {
    6: [600, 350], 9: [500, 300], 13: [600, 350], 15: [600, 350],
    16: [500, 300], 19: [500, 300], 23: [500, 300], 25: [500, 300],
    30: [600, 350], 31: [500, 300], 35: [600, 350], 42: [600, 350],
}
STATUSES = ("pending", "confirmed", "shipped", "delivered", "cancelled")
PRODUCT_CATS = ("Thobes", "Djellabas", "Shirts", "Accessories")
TRACK_EVENTS = {"view_product", "view_shop", "add_to_bag", "begin_checkout",
                "order_placed", "promo_applied"}
PBKDF2_ITERS = max(100000, int(os.environ.get("PBKDF2_ITERS", "310000") or 310000))
MAX_LINE_QTY = 20
MAX_ORDER_ITEMS = 60
MAX_ORDER_LINES = 24

DEFAULT_THEME = {
    "preset": "rose", "shade": "#f6f0ea", "fg": "#171214",
    "rose": "#c98a94", "roseDeep": "#9d5c67", "blue": "#0071e3",
}
DEFAULT_SETTINGS = {
    "whatsapp": "",
    "siteName": "Noor",
    "logo": "",
    "theme": dict(DEFAULT_THEME),
    "banner": {"active": False, "text": "", "bg": "#171214", "fg": "#ffffff"},
    "promos": [{"code": "NOOR10", "pct": 10, "active": True, "label": "Welcome −10%"}],
    "social": {"instagram": "", "facebook": "", "tiktok": ""},
}
HEX_RE = re.compile(r"^#[0-9a-fA-F]{6}$")
LOGO_EXTS = (".png", ".jpg", ".jpeg", ".webp")

FORBIDDEN_NAMES = {
    "tokens.json", "users.json", "orders.json", "reviews.json",
    "settings.json", "counter.json", "logo.json", "ratelimit.json", "vercel.json",
    "server.py", "noor-server.py", "noor_app.py", "store.py", "wsgiapp.py",
    "passenger_wsgi.py", "analytics.jsonl", "readme.md", "noor-audit-report.txt",
}
FORBIDDEN_SUFFIX = (".bak", ".tmp", ".pyc", ".py", ".log", ".jsonl", ".zip", ".sh", ".env")
PUBLIC_EXTS = {
    ".html", ".css", ".js", ".json", ".png", ".jpg", ".jpeg",
    ".webp", ".svg", ".xml", ".txt", ".ico", ".woff2",
}

TRANSITIONS = {
    "pending": ("confirmed", "cancelled"),
    "confirmed": ("shipped", "cancelled"),
    "shipped": ("delivered", "cancelled"),
    "delivered": (),
    "cancelled": (),
}

# Allow 'unsafe-inline' in style-src so 3D carousel, dynamic theme variables, and admin charts work
CSP = (
    "default-src 'self'; img-src 'self' data:; "
    "style-src 'self' 'unsafe-inline'; font-src 'self'; "
    "script-src 'self'; connect-src 'self'; object-src 'none'; "
    "base-uri 'self'; frame-ancestors 'none'; form-action 'self'"
    + ("; upgrade-insecure-requests" if os.environ.get("BEHIND_TLS", "1") == "1" else "")
)


def next_order_num():
    """Monotonic invoice numbers (DB-02). Call under MUTEX()."""
    return "%s-%06d" % (_store.INVOICE_PREFIX, store_next_number())


def img_dims(path):
    """Stdlib JPEG/PNG dimension probe (FE-03: width/height attrs)."""
    try:
        with open(path, "rb") as f:
            head = f.read(128)
        if head[6:10] == b"JFIF" or head[:2] == b"\xff\xd8":
            with open(path, "rb") as f:
                data = f.read(300000)
            i = 2
            while i < len(data) - 8:
                if data[i] != 0xFF:
                    break
                m = data[i + 1]
                if m in (0xC0, 0xC1, 0xC2):
                    h = (data[i + 5] << 8) + data[i + 6]
                    w = (data[i + 7] << 8) + data[i + 8]
                    return (w, h) if w > 0 and h > 0 else None
                if m in (0xD8, 0xD9, 0x01) or 0xD0 <= m <= 0xD7:
                    i += 2
                else:
                    i += 2 + (data[i + 2] << 8) + data[i + 3]
        elif head[:8] == b"\x89PNG\r\n\x1a\n":
            w, h = struct.unpack(">II", head[16:24])
            return (w, h) if w > 0 and h > 0 else None
        elif head[:4] == b"RIFF" and head[8:12] == b"WEBP":
            chunk = head[12:16]
            if chunk == b"VP8 " and len(head) >= 30:
                w, h = struct.unpack("<HH", head[26:30])
                w, h = w & 0x3FFF, h & 0x3FFF
                return (w, h) if w > 0 and h > 0 else None
            elif chunk == b"VP8L" and len(head) >= 25:
                b0, b1, b2, b3 = head[21:25]
                w = 1 + (((b1 & 0x3F) << 8) | b0)
                h = 1 + (((b3 & 0x0F) << 10) | (b2 << 2) | ((b1 & 0xC0) >> 6))
                return (w, h) if w > 0 and h > 0 else None
            elif chunk == b"VP8X" and len(head) >= 30:
                w = 1 + int.from_bytes(head[24:27], "little")
                h = 1 + int.from_bytes(head[27:30], "little")
                return (w, h) if w > 0 and h > 0 else None
    except Exception:
        pass
    return None


@functools.lru_cache(maxsize=256)
def image_variants(img):
    """Return the pre-generated WebP widths available for a catalog image.

    The original JPEG remains the admin/social fallback. Cards request the
    smallest useful variant, which cuts the home page image weight by ~90%.
    """
    m = re.match(r"^images/([\w\-]+)\.(?:jpe?g|png|webp)$", str(img or ""), re.I)
    if not m:
        return ()
    base = m.group(1)
    found = []
    for w in (320, 640, 896):
        rel = "images/opt/%s-%d.webp" % (base, w)
        if os.path.isfile(os.path.join(ROOT, rel)):
            found.append({"w": w, "url": rel})
    return tuple(found)


def write_webp_variants(target, base):
    """Generate the same 320/640/896 WebP set used by the catalog.

    Uses ImageMagick (magick/convert) when present, falls back to Pillow (PIL)
    if installed on the hosting environment, and degrades gracefully if neither
    is available (serving the original uploaded image).
    """
    dims = img_dims(target)
    max_w = dims[0] if dims else 896
    outdir = os.path.join(ROOT, "images", "opt")
    try:
        os.makedirs(outdir, exist_ok=True)
    except OSError:
        return
    wrote = False
    tool = shutil.which("magick") or shutil.which("convert")
    if tool:
        for w in (320, 640, 896):
            if w > max_w:
                continue
            out = os.path.join(outdir, "%s-%d.webp" % (base, w))
            try:
                subprocess.run(
                    [tool, target, "-auto-orient", "-strip",
                     "-resize", "%dx%d>" % (w, w), "-quality", "82", out],
                    check=False, stdout=subprocess.DEVNULL,
                    stderr=subprocess.DEVNULL, timeout=30)
                wrote = wrote or os.path.isfile(out)
            except (OSError, subprocess.SubprocessError):
                continue
    else:
        try:
            from PIL import Image, ImageOps  # type: ignore
            with Image.open(target) as im:
                im = ImageOps.exif_transpose(im)
                for w in (320, 640, 896):
                    if w > max_w:
                        continue
                    out = os.path.join(outdir, "%s-%d.webp" % (base, w))
                    copy = im.copy()
                    copy.thumbnail((w, w * 2))
                    copy.save(out, format="WEBP", quality=82, method=4)
                    wrote = wrote or os.path.isfile(out)
        except Exception:
            pass
    if wrote:
        image_variants.cache_clear()


def public_products():
    """Catalog payload with responsive-image metadata, never mutating storage."""
    products = load_json(PRODUCTS_FILE, [])
    if not isinstance(products, list):
        return []
    out = []
    for p in products:
        if not isinstance(p, dict):
            continue
        q = dict(p)
        variants = image_variants(p.get("img", ""))
        if variants:
            q["imgset"] = [dict(v) for v in variants]
        out.append(q)
    return out


def valid_hex(s, fallback):
    s = str(s or "").strip()
    return s if HEX_RE.match(s) else fallback


def clean(s, n=120):
    s = re.sub(r"[\x00-\x1f\x7f]", "", str(s or "")).strip()
    return s[:n]


def clean_text(s, n=120):
    s = clean(s, n)
    if re.search(r"[<>&]", s):
        raise ValueError("markup not allowed")
    return s


SOCIAL_HOSTS = {
    "instagram": {"instagram.com", "www.instagram.com"},
    "facebook": {"facebook.com", "www.facebook.com", "m.facebook.com"},
    "tiktok": {"tiktok.com", "www.tiktok.com"},
}


def clean_social(key, value):
    """Accept only public HTTPS links for the configured social platforms."""
    value = clean(value, 240)
    if not value:
        return ""
    try:
        parsed = urllib.parse.urlsplit(value)
    except ValueError:
        return ""
    if parsed.scheme != "https" or parsed.hostname not in SOCIAL_HOSTS.get(key, set()):
        return ""
    if not parsed.path or parsed.path == "/":
        return ""
    return value


def safe_int(v, default=0):
    """P2: never let a corrupt record 500 the request."""
    try:
        return int(v)
    except (TypeError, ValueError):
        return default


_TOKEN_KEY_RE = re.compile(r"^[0-9a-f]{64}$")


def token_hash(token):
    """SHA-256 digest of a bearer token.

    Only this digest is ever persisted, so a leaked tokens.json / SQLite row is
    useless to an attacker (they cannot invert it into a usable session token)."""
    return hashlib.sha256(str(token or "").encode("utf-8")).hexdigest()


def const_eq(a, b):
    """Constant-time comparison for two secrets of arbitrary length.

    Both operands are hashed first so the comparison never varies with the
    candidate's length — closing the length side-channel of a raw ``==``."""
    return hmac.compare_digest(
        hashlib.sha256(str(a).encode("utf-8")).digest(),
        hashlib.sha256(str(b).encode("utf-8")).digest(),
    )


def sweep_tokens():
    """Return the live token table, dropping expired and legacy plaintext keys.

    Keys are SHA-256 digests (64 hex chars). Any key that is not a digest is a
    pre-hardening plaintext token and is deliberately evicted, forcing re-login.
    """
    with MUTEX():
        tokens = load_json(TOKENS_FILE, {})
        if not isinstance(tokens, dict):
            return {}
        now = time.time()
        return {
            k: v for k, v in tokens.items()
            if _TOKEN_KEY_RE.match(str(k))
            and isinstance(v, dict)
            and v.get("exp", 0) > now
        }


class LoginGuard:
    """Sliding-window brute-force guard for credential endpoints.

    State lives in SQLite (``store.guard_*``), so every Passenger worker and a
    restarted process all see the same failures and lockouts. Keys are
    ``"<scope>:<client-ip>"`` or ``"<scope>:<username>"``.
    """

    def __init__(self, max_fails, window, lockout):
        self.max_fails = max(1, int(max_fails))
        self.window = max(1, int(window))
        self.lockout = max(1, int(lockout))

    def check(self, key):
        """Return ``(allowed, retry_after_seconds)`` for the given key."""
        return _store.guard_check(key, self.max_fails, self.window, self.lockout)

    def fail(self, key):
        """Record one failed attempt, locking the key out at the threshold."""
        _store.guard_fail(key, self.max_fails, self.window, self.lockout)

    def reset(self, key):
        """Clear the failure history for a key after a successful login."""
        _store.guard_reset(key)


ADMIN_GUARD = LoginGuard(LOGIN_MAX_FAILS, LOGIN_WINDOW, LOGIN_LOCKOUT)
AUTH_GUARD = LoginGuard(20, 300, 300)


def require_admin(fn):
    """RBAC decorator for every administrative route handler.

    Apply to a ``Handler`` method shaped ``(self, data=None, query=None)``. It
    rejects the request with 401 unless a valid, unexpired admin session token
    was presented via ``Authorization: Bearer``, the HttpOnly session cookie,
    the JSON body, or ``?token=`` (kept for the legacy SPA). Used for the admin
    handlers and mirrored by the inline ``_require_admin`` choke point.
    """
    @functools.wraps(fn)
    def wrapper(self, data=None, query=None, *args, **kwargs):
        if not self._require_admin(data, query):
            self._json(401, {"ok": False, "error": "unauthorized"})
            return
        return fn(self, data, query, *args, **kwargs)
    return wrapper


def get_settings():
    st = load_json(SETTINGS_FILE, {})
    if not isinstance(st, dict):
        st = {}
    out = dict(DEFAULT_SETTINGS)
    out.update({k: st[k] for k in st if k in DEFAULT_SETTINGS})
    th = dict(DEFAULT_THEME)
    if isinstance(st.get("theme"), dict):
        for k in th:
            if k in st["theme"]:
                th[k] = clean(st["theme"][k], 20) if k == "preset" else valid_hex(st["theme"][k], th[k])
    out["theme"] = th
    bn = dict(DEFAULT_SETTINGS["banner"])
    if isinstance(st.get("banner"), dict):
        if "text" in st["banner"]:
            bn["text"] = clean(st["banner"]["text"], 200)
        bn["active"] = bool(st["banner"].get("active"))
        bn["bg"] = valid_hex(st["banner"].get("bg"), bn["bg"])
        bn["fg"] = valid_hex(st["banner"].get("fg"), bn["fg"])
    out["banner"] = bn
    pr = []
    if isinstance(st.get("promos"), list):
        for p in st["promos"][:20]:
            if not isinstance(p, dict):
                continue
            code = re.sub(r"[^A-Z0-9-]", "", str(p.get("code", "")).upper())[:20]
            try:
                pct = int(p.get("pct", 0))
            except (TypeError, ValueError):
                pct = 0
            if code and 1 <= pct <= 90:
                pr.append({
                    "code": code, "pct": pct,
                    "active": bool(p.get("active", True)),
                    "label": clean(p.get("label"), 80),
                })
    out["promos"] = pr if isinstance(st.get("promos"), list) else list(DEFAULT_SETTINGS["promos"])
    out["siteName"] = clean(out.get("siteName"), 40) or "Noor"
    logo = str(out.get("logo", ""))[:120]
    if logo and not (
        re.match(r"^(images/[\w\-]+\.(jpg|jpeg|png|webp))$", logo, re.I)
        or re.match(r"^api/logo\?v=\d{1,12}$", logo)
    ):
        logo = ""
    out["logo"] = logo
    social = {}
    raw_social = st.get("social") if isinstance(st.get("social"), dict) else {}
    for key in ("instagram", "facebook", "tiktok"):
        social[key] = clean_social(key, raw_social.get(key, ""))
    out["social"] = social
    return out


def find_promo(promos, code):
    code = re.sub(r"[^A-Z0-9-]", "", str(code or "").upper())[:20]
    for p in promos:
        if p.get("code") == code and p.get("active"):
            return p
    return None


def ship_config():
    return load_json(SHIPPING_FILE, {"freeOver": 8000, "providers": {}, "zones": {}})


def ship_zone(code):
    zones = ship_config().get("zones", {})
    for zname, codes in zones.items():
        if code in codes:
            return zname
    return "rest"


def ship_fee(provider, code, method):
    cfg = ship_config().get("providers", {}).get(provider)
    if not cfg:
        fees = SHIP_MAJOR.get(code, SHIP_DEFAULT)
        return fees[1] if method == "stop" else fees[0]
    table = cfg.get("stop" if method == "stop" else "home", {})
    return int(table.get(ship_zone(code), table.get("rest", 800)))


def ship_free(provider):
    cfg = ship_config().get("providers", {}).get(provider)
    if not cfg:
        return True
    return bool(cfg.get("free"))


def limited(ip, key, maxn, per=60):
    """Shared fixed-window rate limit (all worker processes see the same count).

    Fails open on a storage error: a broken limiter must not take the shop down;
    credential endpoints are additionally protected by LoginGuard."""
    try:
        return _store.rate_hit("%s:%s" % (key, ip), maxn, per)
    except StoreError:
        return False


def hash_pass(password, salt, iters=PBKDF2_ITERS):
    return hashlib.pbkdf2_hmac(
        "sha256", password.encode("utf-8"), bytes.fromhex(salt), int(iters)
    ).hex()


# --- Admin account + 2-step verification (RFC 6238 TOTP) -------------------

def admin_account():
    doc = load_json(ADMIN_FILE, {})
    return doc if isinstance(doc, dict) else {}


def admin_has_credentials(acct=None):
    acct = admin_account() if acct is None else acct
    return bool(acct.get("user") and acct.get("hash") and acct.get("salt"))


def admin_login_available():
    return admin_has_credentials() or PIN_USABLE


def admin_password_ok(acct, password):
    try:
        candidate = hash_pass(str(password), acct.get("salt", ""), int(acct.get("iter") or PBKDF2_ITERS))
    except (ValueError, TypeError):
        return False
    return hmac.compare_digest(str(acct.get("hash", "")), candidate)


def totp_new_secret():
    return base64.b32encode(secrets.token_bytes(20)).decode("ascii").rstrip("=")


def totp_code(secret, step):
    key = base64.b32decode(secret + "=" * (-len(secret) % 8), casefold=True)
    digest = hmac.new(key, struct.pack(">Q", step), hashlib.sha1).digest()
    off = digest[-1] & 0x0F
    return "%06d" % ((struct.unpack(">I", digest[off:off + 4])[0] & 0x7FFFFFFF) % 1000000)


def totp_match(secret, code, now=None):
    """Return the matching 30-second step (current +/- 1) or None."""
    code = re.sub(r"\D", "", str(code or ""))
    if len(code) != 6 or not secret:
        return None
    step = int((now or time.time()) // 30)
    for s in (step, step - 1, step + 1):
        try:
            if hmac.compare_digest(totp_code(secret, s), code):
                return s
        except (ValueError, TypeError):
            return None
    return None


def totp_accept(secret, code):
    """Verify a code once: a code that was already used is rejected (replay)."""
    step = totp_match(secret, code)
    if step is None:
        return False
    last = _store.kv_get("totp:last") or 0
    if step <= int(last):
        return False
    _store.kv_set("totp:last", step, 400 * 86400)
    return True


def totp_uri(secret, user, issuer):
    label = urllib.parse.quote("%s:%s" % (issuer, user))
    return "otpauth://totp/%s?secret=%s&issuer=%s&algorithm=SHA1&digits=6&period=30" % (
        label, secret, urllib.parse.quote(issuer))


def revoke_sessions(admin=False, email=None, keep_digest=None):
    with MUTEX():
        tokens = sweep_tokens()
        for k in list(tokens):
            v = tokens[k]
            if k == keep_digest:
                continue
            if (admin and v.get("admin")) or (email and v.get("email") == email):
                tokens.pop(k, None)
        save_json(TOKENS_FILE, tokens)


# --- Automatic backups ------------------------------------------------------

def _run_backup(send_copy=True):
    try:
        path = _store.backup_now()
    except Exception as exc:
        sys.stderr.write("BACKUP failed: %s\n" % exc)
        return ""
    if send_copy and BACKUP_EMAIL:
        try:
            size = os.path.getsize(path)
            if size <= 15 * 1024 * 1024:
                with open(path, "rb") as fh:
                    blob = fh.read()
                mailer.send(BACKUP_EMAIL, "[%s] Backup %s" % (get_settings().get("siteName", "Noor"),
                                                             time.strftime("%Y-%m-%d", time.gmtime())),
                            "Automatic database backup attached (%d KB).\n"
                            "Keep it private: it contains customer orders.\n" % (size // 1024),
                            attachment=(os.path.basename(path), blob, "application/gzip"))
        except OSError as exc:
            sys.stderr.write("BACKUP email failed: %s\n" % exc)
    return path


def maybe_backup():
    """Start a background backup if none ran in the last BACKUP_EVERY seconds."""
    if BACKUP_EVERY <= 0:
        return
    try:
        if _store.kv_set("backup:due", int(time.time()), BACKUP_EVERY, only_if_absent=True):
            threading.Thread(target=_run_backup, daemon=True).start()
    except StoreError:
        pass


def report_error(where, exc_info=None):
    """Log an unexpected error and email the owner (at most every 15 minutes)."""
    sys.stderr.write("ERROR in %s\n" % where)
    if exc_info:
        import traceback
        traceback.print_exception(*exc_info)
    try:
        if _store.kv_set("alert:error", int(time.time()), 900, only_if_absent=True):
            mailer.error_alert(where, exc_info)
    except Exception:
        pass


def money(n):
    return "{:,}".format(int(n)).replace(",", " ") + " DZD"


def order_total(o):
    try:
        return int(o.get("totalNum"))
    except (TypeError, ValueError):
        pass
    try:
        return int(re.sub(r"\D", "", str(o.get("total", "0"))))
    except ValueError:
        return 0


class BoundedThreadingHTTPServer(ThreadingHTTPServer):
    daemon_threads = True
    allow_reuse_address = True

    def process_request(self, request, client_address):
        if not THREAD_SEMAPHORE.acquire(blocking=False):
            try:
                request.close()
            except OSError:
                pass
            return
        try:
            super().process_request(request, client_address)
        except Exception:
            THREAD_SEMAPHORE.release()
            raise

    def process_request_thread(self, request, client_address):
        try:
            super().process_request_thread(request, client_address)
        finally:
            THREAD_SEMAPHORE.release()


def accepts_gzip(value):
    """True only when the client explicitly accepts gzip with a non-zero q."""
    for part in str(value or "").split(","):
        bits = part.strip().split(";")
        coding = bits[0].strip().lower()
        if coding not in ("gzip", "*"):
            continue
        q = 1.0
        for param in bits[1:]:
            param = param.strip()
            if param.lower().startswith("q="):
                try:
                    q = float(param[2:])
                except ValueError:
                    q = 0.0
        if q > 0:
            return True
    return False


@functools.lru_cache(maxsize=96)
def _static_bytes(full, mtime_ns, size):
    with open(full, "rb") as fh:
        return fh.read()


@functools.lru_cache(maxsize=96)
def _static_gzip(full, mtime_ns, size):
    return _gzip.compress(_static_bytes(full, mtime_ns, size), 6)


class Handler(BaseHTTPRequestHandler):
    server_version = "Noor/1.1"
    protocol_version = "HTTP/1.1"   # keep-alive; every response path sets Content-Length
    timeout = 10
    daemon_threads = True

    def log_message(self, *args):
        try:
            path = (args[0] % args[1:]).split("?")[0][:80] if args else "-"
            sys.stderr.write("%s %s\n" % (time.strftime("%H:%M:%S"), path))
        except Exception:
            pass

    def _headers(self, code, ctype, length, cache="no-store", extra=()):
        self.send_response(code)
        self.send_header("Content-Type", ctype)
        self.send_header("Content-Length", str(length))
        self.send_header("Cache-Control", cache)
        self.send_header("X-Content-Type-Options", "nosniff")
        self.send_header("X-Frame-Options", "DENY")
        self.send_header("Referrer-Policy", "strict-origin-when-cross-origin")
        self.send_header("Permissions-Policy", "camera=(), microphone=(), geolocation=(), payment=()")
        self.send_header("Cross-Origin-Opener-Policy", "same-origin")
        self.send_header("Cross-Origin-Resource-Policy", "same-origin")
        self.send_header("X-Permitted-Cross-Domain-Policies", "none")
        for k, v in extra:
            self.send_header(k, v)
        self.send_header("Content-Security-Policy", CSP)
        if ENABLE_HSTS:
            self.send_header("Strict-Transport-Security", "max-age=31536000; includeSubDomains")
        self.end_headers()

    def _write(self, code, ctype, body, cache="no-store", extra=()):
        # One choke point: gzip text payloads when the client explicitly allows it.
        if len(body) > 1024:
            zt = ctype.split(";")[0]
            gzippable = zt.startswith("text/") or zt in ("application/json", "application/javascript")
            if gzippable:
                extra = tuple(extra) + (("Vary", "Accept-Encoding"),)
                if accepts_gzip(self.headers.get("Accept-Encoding")):
                    body = _gzip.compress(body, 6)
                    self._headers(code, ctype, len(body), cache,
                                  extra=(("Content-Encoding", "gzip"),) + tuple(extra))
                    if getattr(self, "command", "") != "HEAD":
                        self.wfile.write(body)
                    return
        self._headers(code, ctype, len(body), cache, extra=tuple(extra))
        if getattr(self, "command", "") != "HEAD":
            self.wfile.write(body)

    def _json(self, code, obj, extra=()):
        body = json.dumps(obj, ensure_ascii=False).encode("utf-8")
        self._write(code, "application/json; charset=utf-8", body, extra=extra)

    def _static(self, path, versioned=False):
        if path == "/":
            path = "/index.html"
        rel = urllib.parse.unquote(path).lstrip("/")
        base = os.path.basename(rel)
        base_lower = base.lower()
        rel_lower = rel.lower()
        if (
            not rel
            or base_lower in FORBIDDEN_NAMES
            or base.startswith(".")
            or any(part.startswith(".") for part in rel.replace("\\", "/").split("/"))
            or rel_lower.endswith(FORBIDDEN_SUFFIX)
        ):
            self._json(404, {"ok": False, "error": "not found"})
            return
        full = os.path.realpath(os.path.normpath(os.path.join(ROOT, rel)))
        root_real = os.path.realpath(ROOT)
        if not full.startswith(root_real + os.sep) or not os.path.isfile(full):
            self._json(404, {"ok": False, "error": "not found"})
            return
        ext = os.path.splitext(full)[1].lower()
        if ext not in PUBLIC_EXTS:
            self._json(404, {"ok": False, "error": "not found"})
            return
        if ext == ".json" and base_lower not in ("products.json", "shipping.json", "models.json"):
            self._json(404, {"ok": False, "error": "not found"})
            return
        try:
            st = os.stat(full)
        except OSError:
            self._json(404, {"ok": False, "error": "not found"})
            return
        etag = '"%x-%x"' % (st.st_mtime_ns, st.st_size)
        # CSS/JS are cached for a year only when the URL carries a version
        # (styles.css?v=...); a bare URL is re-checked every visit (cheap 304),
        # so a design update can never get stuck in visitors' browsers.
        if ext == ".woff2" or (ext in (".css", ".js") and versioned):
            cache = "public, max-age=31536000, immutable"
        elif ext in (".css", ".js"):
            cache = "no-cache"
        elif ext in (".jpg", ".jpeg", ".png", ".webp", ".svg", ".ico"):
            cache = "public, max-age=604800"
        else:
            cache = "no-store"
        base_headers = (
            ("ETag", etag),
            ("Last-Modified", email.utils.formatdate(st.st_mtime, usegmt=True)),
        )
        inm = self.headers.get("If-None-Match") or ""
        if etag and etag in [x.strip() for x in inm.split(",")]:
            self._headers(304, MIME.get(ext, "application/octet-stream"), 0, cache,
                          extra=base_headers)
            return
        if getattr(self, "command", "") == "HEAD":
            self._headers(200, MIME.get(ext, "application/octet-stream"),
                          st.st_size, cache, extra=base_headers)
            return
        try:
            body = _static_bytes(full, st.st_mtime_ns, st.st_size)
        except OSError:
            self._json(404, {"ok": False, "error": "not found"})
            return
        zt = MIME.get(ext, "application/octet-stream").split(";")[0]
        gzippable = zt.startswith("text/") or zt in ("application/json", "application/javascript",
                                                     "image/svg+xml")
        if gzippable and len(body) > 1024 and accepts_gzip(self.headers.get("Accept-Encoding")):
            body = _static_gzip(full, st.st_mtime_ns, st.st_size)
            self._headers(200, MIME.get(ext, "application/octet-stream"), len(body), cache,
                          extra=base_headers + (("Content-Encoding", "gzip"),
                                                ("Vary", "Accept-Encoding")))
        else:
            self._headers(200, MIME.get(ext, "application/octet-stream"), len(body), cache,
                          extra=base_headers + ((("Vary", "Accept-Encoding"),) if gzippable else ()))
        self.wfile.write(body)

    def _body(self, limit=50000):
        """Read a JSON object, failing closed on oversized or malformed bodies."""
        self._body_error = 0
        try:
            length = int(self.headers.get("Content-Length") or 0)
        except ValueError:
            self._body_error = 400
            self.close_connection = True
            return {}
        if length <= 0:
            return {}
        if length > limit:
            # Do not leave unread bytes on a keep-alive socket (request smuggling).
            self._body_error = 413
            self.close_connection = True
            return {}
        try:
            raw = self.rfile.read(length)
            data = json.loads(raw.decode("utf-8"))
        except (ValueError, OSError):
            self._body_error = 400
            return {}
        if not isinstance(data, dict):
            self._body_error = 400
            return {}
        return data

    def _ip(self):
        peer = self.client_address[0] if self.client_address else "?"
        if TRUST_PROXY and peer in TRUSTED_PROXIES:
            for hdr in ("CF-Connecting-IP", "x-real-ip"):
                v = (self.headers.get(hdr) or "").strip()
                if re.match(r"^[0-9a-fA-F:.]{3,45}$", v):
                    return v
            fwd = self.headers.get("X-Forwarded-For") or ""
            first = fwd.split(",")[0].strip()
            if re.match(r"^[0-9a-fA-F:.]{3,45}$", first):
                return first
        return peer

    def _cookie(self, name):
        raw = self.headers.get("Cookie") or ""
        for part in raw.split(";"):
            if "=" in part:
                key, val = part.split("=", 1)
                if key.strip() == name:
                    return urllib.parse.unquote(val.strip())
        return ""

    def _cookie_header(self, name, value, max_age):
        parts = ["%s=%s" % (name, urllib.parse.quote(str(value), safe="")),
                 "Path=/", "HttpOnly", "SameSite=Strict", "Max-Age=%d" % int(max_age)]
        if COOKIE_SECURE:
            parts.append("Secure")
        return "; ".join(parts)

    def _presented_token(self, data=None, query=None):
        """Resolve the caller's bearer token from a header or HttpOnly cookie.

        Query-string tokens are no longer accepted: they leak into access logs,
        browser history and Referer headers. Body tokens remain for the legacy
        SPA logout path, but every new call site uses the Authorization header.
        """
        header = self.headers.get("Authorization") or ""
        if header[:7].lower() == "bearer ":
            return header[7:].strip()
        for name in (COOKIE_ADMIN, COOKIE_USER):
            value = self._cookie(name)
            if value:
                return value
        if isinstance(data, dict) and data.get("token"):
            return str(data["token"])
        return ""

    def _auth_entry(self, token):
        """Return the session record for a token digest, or ``None``."""
        token = str(token or "")
        if not token or len(token) > 512:
            return None
        digest = token_hash(token)
        tokens = load_json(TOKENS_FILE, {})
        if not isinstance(tokens, dict):
            return None
        entry = tokens.get(digest)
        if entry is None:
            # Constant-time fallback scan: a dict hit leaks nothing useful, but
            # this keeps token validation itself timing-independent.
            for key, value in tokens.items():
                if hmac.compare_digest(str(key), digest):
                    entry = value
                    break
        if not isinstance(entry, dict):
            return None
        try:
            if float(entry.get("exp", 0)) <= time.time():
                return None
        except (TypeError, ValueError):
            return None
        return entry

    def _user(self, token):
        entry = self._auth_entry(token)
        if entry and not entry.get("admin"):
            return str(entry.get("email", ""))
        return ""

    def _require_admin(self, data=None, query=None):
        """Single RBAC choke point: True iff a live admin session is presented."""
        entry = self._auth_entry(self._presented_token(data, query))
        return bool(entry and entry.get("admin"))

    def _admin(self, data, query):
        return self._require_admin(data, query)

    @require_admin
    def admin_backup(self, data=None, query=None):
        """GET/POST /api/admin/backup — RBAC enforced by the decorator.

        Returns the snapshot to the authenticated admin (the SPA downloads it)
        and writes a full compressed database backup to ``data/backups`` (and to
        BACKUP_EMAIL when set). There is no caller-supplied path, so no
        traversal primitive exists."""
        payload = {"ok": True, "orders": load_json(ORDERS_FILE, []),
                   "products": [{"id": p.get("id"), "stock": p.get("stock")}
                                for p in load_json(PRODUCTS_FILE, [])]}
        threading.Thread(target=_run_backup, daemon=True).start()
        self._json(200, payload)

    def do_HEAD(self):
        self.do_GET()

    def do_GET(self):
        parsed = urllib.parse.urlparse(self.path)
        query = urllib.parse.parse_qs(parsed.query)
        if parsed.path in ("/api/health", "/healthz"):
            health = _store.health()
            self._json(200 if health.healthy else 503,
                       {"ok": bool(health.healthy), "backend": health.backend})
            return
        if parsed.path == "/api/logo":
            doc = load_json(LOGO_FILE, {})
            raw = doc.get("data", "") if isinstance(doc, dict) else ""
            mime = str(doc.get("mime", "")) if isinstance(doc, dict) else ""
            if mime not in ("image/png", "image/jpeg", "image/webp"):
                self._json(404, {"ok": False, "error": "not found"})
                return
            try:
                blob = base64.b64decode(raw, validate=True)
            except Exception:
                self._json(404, {"ok": False, "error": "not found"})
                return
            etag = '"%s"' % hashlib.sha256(blob).hexdigest()[:24]
            inm = self.headers.get("If-None-Match") or ""
            if etag in [x.strip() for x in inm.split(",")]:
                self._headers(304, mime, 0, "public, max-age=31536000, immutable",
                              extra=(("ETag", etag),))
                return
            self._headers(200, mime, len(blob), "public, max-age=31536000, immutable",
                          extra=(("ETag", etag),))
            if getattr(self, "command", "") != "HEAD":
                self.wfile.write(blob)
            return
        if parsed.path == "/api/products":
            self._json(200, {"ok": True, "products": public_products()})
            return
        if parsed.path == "/api/orders":
            email = clean((query.get("email") or [""])[0]).lower()
            token = self._presented_token(query=query)
            if not email or self._user(token) != email:
                self._json(401, {"ok": False, "error": "unauthorized"})
                return
            orders = load_json(ORDERS_FILE, [])
            mine = [o for o in orders if o.get("email") == email]
            self._json(200, {"ok": True, "orders": mine[-20:]})
            return
        if parsed.path == "/api/reviews":
            pid = clean((query.get("product") or [""])[0], 60)
            allr = load_json(REVIEWS_FILE, [])
            self._json(200, {"ok": True, "reviews": [r for r in allr if r.get("product") == pid][-30:]})
            return
        if parsed.path == "/api/admin/orders":
            if not self._require_admin(query=query):
                self._json(401, {"ok": False, "error": "unauthorized"})
                return
            orders = load_json(ORDERS_FILE, [])
            self._json(200, {"ok": True, "orders": orders[::-1][:200]})
            return
        if parsed.path == "/api/admin/stats":
            if not self._require_admin(query=query):
                self._json(401, {"ok": False, "error": "unauthorized"})
                return
            orders = load_json(ORDERS_FILE, [])
            products = load_json(PRODUCTS_FILE, [])
            settings = get_settings()
            revenue, by_status, items_sold = 0, {}, 0
            paid_orders = [o for o in orders if o.get("status", "pending") != "cancelled"]
            for o in paid_orders:
                revenue += order_total(o)
                try:
                    items_sold += int(o.get("items", 0))
                except (TypeError, ValueError):
                    pass
                st = o.get("status", "pending")
                by_status[st] = by_status.get(st, 0) + 1
            now = int(time.time())
            day = 86400
            day_start = (now // day) * day
            by_day = [{"day": time.strftime("%m-%d", time.localtime(day_start - i * day)),
                       "revenue": 0, "orders": 0} for i in range(13, -1, -1)]
            undated = 0
            for o in paid_orders:
                ts = o.get("ts") or 0
                try:
                    ts = int(ts)
                except (TypeError, ValueError):
                    ts = 0
                if ts <= 0:
                    undated += 1
                    continue
                idx = (ts - (day_start - 13 * day)) // day
                if 0 <= idx < 14:
                    by_day[idx]["revenue"] += order_total(o)
                    by_day[idx]["orders"] += 1
            wilayas = {}
            for o in orders:
                try:
                    w = int(o.get("wilaya") or 0)
                except (TypeError, ValueError):
                    w = 0
                if 1 <= w <= 58:
                    wilayas[str(w)] = wilayas.get(str(w), 0) + 1
            top_wilayas = sorted(wilayas.items(), key=lambda kv: kv[1], reverse=True)[:6]
            stock_value, oos = 0, 0
            for p in products:
                try:
                    s = int(p.get("stock", 0))
                    stock_value += s * int(p.get("price", 0))
                except (TypeError, ValueError):
                    s = 0
                if s <= 0:
                    oos += 1
            low = [{"id": p.get("id"), "name": p.get("name"), "stock": p.get("stock", 0)}
                   for p in products if safe_int(p.get("stock", 99), 99) <= 5]
            views, adds, placed = {}, {}, 0
            for ev in analytics_tail(2000):
                if not isinstance(ev, dict):
                    continue
                pid = (ev.get("d") or {}).get("id", "")
                if ev.get("ev") == "view_product" and pid:
                    views[pid] = views.get(pid, 0) + 1
                elif ev.get("ev") == "add_to_bag" and pid:
                    adds[pid] = adds.get(pid, 0) + 1
                elif ev.get("ev") == "order_placed":
                    placed += 1
            names = {p.get("id"): p.get("name") for p in products}
            top = sorted(set(list(views) + list(adds)),
                         key=lambda k: (adds.get(k, 0) * 3 + views.get(k, 0)),
                         reverse=True)[:8]
            avg_basket = round(revenue / len(paid_orders)) if paid_orders else 0
            self._json(200, {"ok": True, "revenue": money(revenue), "revenueNum": revenue,
                             "orders": len(orders), "paidOrders": len(paid_orders),
                             "avgBasket": money(avg_basket), "itemsSold": items_sold,
                             "byStatus": by_status, "lowStock": low,
                             "byDay": by_day, "undated": undated,
                             "topWilayas": [{"wilaya": k, "orders": v} for k, v in top_wilayas],
                             "stockValue": money(stock_value), "outOfStock": oos,
                             "funnel": {"views": sum(views.values()), "adds": sum(adds.values()),
                                        "orders": placed or len(orders)},
                             "activePromos": [p for p in settings.get("promos", []) if p.get("active")],
                             "promos": settings.get("promos", []),
                             "top": [{"id": k, "name": names.get(k, k),
                                       "views": views.get(k, 0), "adds": adds.get(k, 0)} for k in top]})
            return
        if parsed.path == "/api/settings":
            st = get_settings()
            self._json(200, {"ok": True, "whatsapp": st.get("whatsapp", ""),
                             "siteName": st.get("siteName", "Noor"),
                             "logo": st.get("logo", ""),
                             "theme": st.get("theme", DEFAULT_THEME),
                             "banner": st.get("banner", DEFAULT_SETTINGS["banner"]),
                             "social": st.get("social", DEFAULT_SETTINGS["social"]),
                             "promos": [p for p in st.get("promos", []) if p.get("active")]})
            return
        if parsed.path == "/api/shipping":
            self._json(200, {"ok": True, "shipping": ship_config()})
            return
        self._static(parsed.path, versioned="v=" in parsed.query)

    def do_POST(self):
        parsed = urllib.parse.urlparse(self.path)
        data = self._body(limit=2600000 if parsed.path in ("/api/admin/logo", "/api/admin/product-image") else 50000)
        if getattr(self, "_body_error", 0):
            code = self._body_error
            self._json(code, {"ok": False, "error": "too-large" if code == 413 else "invalid"})
            return
        ip = self._ip()
        if parsed.path == "/api/track":
            if limited(ip, "track", 60):
                self._json(429, {"ok": False})
                return
            ev = clean(data.get("ev"), 40)
            if ev:
                if ev not in TRACK_EVENTS:
                    self._json(400, {"ok": False, "error": "event"})
                    return
                try:
                    analytics_append({"t": int(time.time()), "ev": ev,
                                      "d": {"id": clean((data.get("d") or {}).get("id"), 60)}})
                except StoreError:
                    pass
            self._json(200, {"ok": True})
            return
        if parsed.path == "/api/reviews":
            if limited(ip, "reviews", 6):
                self._json(429, {"ok": False, "error": "rate"})
                return
            try:
                pid = clean(data.get("product"), 60)
                name = clean_text(data.get("name"), 60)
                text = clean_text(data.get("text"), 500)
                rating = int(data.get("rating", 0))
            except (TypeError, ValueError):
                self._json(400, {"ok": False, "error": "invalid"})
                return
            with MUTEX():
                products = load_json(PRODUCTS_FILE, [])
                if not any(p.get("id") == pid for p in products) or len(name) < 2 \
                        or not 1 <= rating <= 5 or len(text) < 3:
                    self._json(400, {"ok": False, "error": "invalid"})
                    return
                allr = load_json(REVIEWS_FILE, [])
                allr.append({"id": secrets.token_hex(6), "product": pid, "name": name,
                             "rating": rating, "text": text, "date": int(time.time())})
                save_json(REVIEWS_FILE, allr[-1000:])
            self._json(200, {"ok": True})
            return
        if parsed.path == "/api/admin/login":
            acct = admin_account()
            has_creds = admin_has_credentials(acct)
            if not has_creds and not PIN_USABLE:
                self._json(503, {"ok": False, "error": "admin-disabled"})
                return
            if limited(ip, "admin", 10):
                self._json(429, {"ok": False, "error": "rate"})
                return
            guard_key = "admin:" + ip
            allowed, retry = ADMIN_GUARD.check(guard_key)
            if not allowed:
                self._json(429, {"ok": False, "error": "locked", "retryAfter": retry},
                           extra=(("Retry-After", str(retry)),))
                return
            user = str(data.get("user", ""))[:60].strip()
            password = str(data.get("pass") or data.get("pin") or "")[:128]
            if has_creds:
                user_ok = const_eq(user.lower(), str(acct.get("user", "")).lower())
                if not (user_ok and admin_password_ok(acct, password)):
                    ADMIN_GUARD.fail(guard_key)
                    self._json(401, {"ok": False, "error": "wrong"})
                    return
                if acct.get("totp"):
                    code = str(data.get("code", ""))[:12]
                    if not code:
                        # Password was right; ask for the authenticator code.
                        self._json(401, {"ok": False, "error": "code-required"})
                        return
                    if not totp_accept(acct["totp"], code):
                        ADMIN_GUARD.fail(guard_key)
                        self._json(401, {"ok": False, "error": "code-wrong"})
                        return
            else:
                # Bootstrap: the .env PIN works only until an account exists.
                if not const_eq(password, ADMIN_PIN):
                    ADMIN_GUARD.fail(guard_key)
                    self._json(401, {"ok": False, "error": "wrong"})
                    return
            ADMIN_GUARD.reset(guard_key)
            token = secrets.token_hex(TOKEN_BYTES)
            with MUTEX():
                tokens = sweep_tokens()
                tokens[token_hash(token)] = {"admin": True, "iat": int(time.time()),
                                             "ip": ip, "exp": time.time() + ADMIN_TOKEN_TTL}
                save_json(TOKENS_FILE, tokens)
            maybe_backup()
            self._json(200, {"ok": True, "token": token, "expiresIn": ADMIN_TOKEN_TTL,
                             "setup": not has_creds},
                       extra=(("Set-Cookie",
                               self._cookie_header(COOKIE_ADMIN, token, ADMIN_TOKEN_TTL)),))
            return
        if parsed.path == "/api/admin/security":
            if not self._require_admin(data):
                self._json(401, {"ok": False, "error": "unauthorized"})
                return
            acct = admin_account()
            self._json(200, {"ok": True, "account": admin_has_credentials(acct),
                             "user": acct.get("user", ""), "totp": bool(acct.get("totp")),
                             "mail": mailer.enabled(), "adminEmail": bool(mailer.ADMIN_EMAIL),
                             "backupEmail": bool(BACKUP_EMAIL)})
            return
        if parsed.path in ("/api/admin/security/account", "/api/admin/security/totp-start",
                           "/api/admin/security/totp-enable", "/api/admin/security/totp-disable"):
            if not self._require_admin(data):
                self._json(401, {"ok": False, "error": "unauthorized"})
                return
            guard_key = "admin:" + ip
            allowed, retry = ADMIN_GUARD.check(guard_key)
            if not allowed:
                self._json(429, {"ok": False, "error": "locked", "retryAfter": retry})
                return
            acct = admin_account()
            has_creds = admin_has_credentials(acct)
            current = str(data.get("current", ""))[:128]
            # Every security change re-checks the current password (or the
            # bootstrap PIN before an account exists).
            current_ok = admin_password_ok(acct, current) if has_creds else \
                (PIN_USABLE and const_eq(current, ADMIN_PIN))
            my_digest = token_hash(self._presented_token(data))
            if parsed.path.endswith("/account"):
                if not current_ok:
                    ADMIN_GUARD.fail(guard_key)
                    self._json(403, {"ok": False, "error": "current-wrong"})
                    return
                user = str(data.get("user", "")).strip()
                new_pass = str(data.get("pass", ""))[:128]
                if not ADMIN_USER_RE.match(user):
                    self._json(400, {"ok": False, "error": "user-invalid"})
                    return
                if len(new_pass) < ADMIN_PASS_MIN or new_pass.lower() in _WEAK_PINS \
                        or new_pass.lower() == user.lower():
                    self._json(400, {"ok": False, "error": "pass-weak"})
                    return
                salt = secrets.token_hex(16)
                with MUTEX():
                    acct = admin_account()
                    acct.update({"user": user, "salt": salt, "iter": PBKDF2_ITERS,
                                 "hash": hash_pass(new_pass, salt), "updated": int(time.time())})
                    save_json(ADMIN_FILE, acct)
                revoke_sessions(admin=True, keep_digest=my_digest)
                self._json(200, {"ok": True, "user": user})
                return
            if parsed.path.endswith("/totp-start"):
                if not has_creds:
                    self._json(409, {"ok": False, "error": "account-first"})
                    return
                secret = totp_new_secret()
                _store.kv_set("totp:pending:" + my_digest, secret, 900)
                self._json(200, {"ok": True, "secret": secret,
                                 "uri": totp_uri(secret, acct.get("user", "admin"),
                                                 get_settings().get("siteName") or TOTP_ISSUER)})
                return
            if parsed.path.endswith("/totp-enable"):
                secret = _store.kv_get("totp:pending:" + my_digest)
                if not secret or not has_creds:
                    self._json(409, {"ok": False, "error": "start-again"})
                    return
                if not current_ok or totp_match(secret, data.get("code")) is None:
                    ADMIN_GUARD.fail(guard_key)
                    self._json(403, {"ok": False, "error": "code-wrong"})
                    return
                _store.kv_take("totp:pending:" + my_digest)
                with MUTEX():
                    acct = admin_account()
                    acct["totp"] = secret
                    acct["updated"] = int(time.time())
                    save_json(ADMIN_FILE, acct)
                revoke_sessions(admin=True, keep_digest=my_digest)
                self._json(200, {"ok": True, "totp": True})
                return
            # totp-disable
            if not acct.get("totp"):
                self._json(200, {"ok": True, "totp": False})
                return
            if not current_ok or totp_match(acct["totp"], data.get("code")) is None:
                ADMIN_GUARD.fail(guard_key)
                self._json(403, {"ok": False, "error": "code-wrong"})
                return
            with MUTEX():
                acct = admin_account()
                acct.pop("totp", None)
                acct["updated"] = int(time.time())
                save_json(ADMIN_FILE, acct)
            self._json(200, {"ok": True, "totp": False})
            return
        if parsed.path in ("/api/auth/logout", "/api/admin/logout"):
            token = self._presented_token(data)
            if token:
                with MUTEX():
                    tokens = sweep_tokens()
                    tokens.pop(token_hash(token), None)
                    save_json(TOKENS_FILE, tokens)
            self._json(200, {"ok": True}, extra=(
                ("Set-Cookie", self._cookie_header(COOKIE_ADMIN, "", 0)),
                ("Set-Cookie", self._cookie_header(COOKIE_USER, "", 0)),
            ))
            return
        if parsed.path == "/api/admin/backup":
            self.admin_backup(data)
            return
        if parsed.path == "/api/admin/status":
            if not self._require_admin(data):
                self._json(401, {"ok": False, "error": "unauthorized"})
                return
            num = clean(data.get("num"), 40)
            status = clean(data.get("status"), 20)
            if status not in STATUSES:
                self._json(400, {"ok": False, "error": "invalid"})
                return
            changed = None
            with transaction():
                orders = load_json(ORDERS_FILE, [])
                found = False
                for o in orders:
                    if o.get("num") != num:
                        continue
                    found = True
                    cur = o.get("status", "pending")
                    allowed = TRANSITIONS.get(cur, ("pending", "cancelled"))
                    if status != cur and status not in allowed:
                        self._json(409, {"ok": False, "error": "bad-transition", "from": cur})
                        return
                    if status != cur:
                        # Cancelling is the only reverse transition: put every
                        # packed line back into stock exactly once.
                        if status == "cancelled" and not o.get("restocked"):
                            products = load_json(PRODUCTS_FILE, [])
                            by_id = {p.get("id"): p for p in products}
                            lines = o.get("lines") if isinstance(o.get("lines"), list) else []
                            for line in lines:
                                if not isinstance(line, dict):
                                    continue
                                p = by_id.get(line.get("id"))
                                qty = safe_int(line.get("qty", 0))
                                if p is not None and 1 <= qty <= MAX_ORDER_ITEMS:
                                    p["stock"] = safe_int(p.get("stock", 0)) + qty
                            save_json(PRODUCTS_FILE, products)
                            o["restocked"] = True
                        o["status"] = status
                        hist = o.get("history") if isinstance(o.get("history"), list) else []
                        hist.append({"from": cur, "to": status, "ts": int(time.time())})
                        o["history"] = hist[-20:]
                        changed = dict(o)
                if not found:
                    self._json(404, {"ok": False, "error": "not-found"})
                    return
                save_json(ORDERS_FILE, orders)
            if changed and changed.get("email"):
                mailer.order_status(changed, status, get_settings().get("siteName", "Noor"))
            self._json(200, {"ok": True})
            return
        if parsed.path == "/api/auth/forgot":
            if limited(ip, "forgot", 5, 600):
                self._json(429, {"ok": False, "error": "rate"})
                return
            email = clean(data.get("email"), 120).lower()
            if not mailer.valid_email(email):
                self._json(400, {"ok": False, "error": "invalid"})
                return
            # Same answer whether or not the account exists (no enumeration).
            users = load_json(USERS_FILE, {})
            if mailer.enabled() and isinstance(users, dict) and email in users \
                    and _store.kv_set("resetmail:" + email, 1, 120, only_if_absent=True):
                token = secrets.token_urlsafe(32)
                _store.kv_set("reset:" + token_hash(token), {"email": email}, RESET_TTL)
                base = mailer.SITE_URL or ("https://" + (self.headers.get("Host") or "").strip())
                mailer.password_reset(email, "%s/#reset=%s" % (base, token),
                                      get_settings().get("siteName", "Noor"))
            self._json(200, {"ok": True, "mail": mailer.enabled()})
            return
        if parsed.path == "/api/auth/reset":
            if limited(ip, "reset", 10, 600):
                self._json(429, {"ok": False, "error": "rate"})
                return
            token = str(data.get("token", ""))[:128]
            password = str(data.get("pass", ""))[:128]
            if len(password) < 8:
                self._json(400, {"ok": False, "error": "weak"})
                return
            rec = _store.kv_take("reset:" + token_hash(token)) if token else None
            email = rec.get("email", "") if isinstance(rec, dict) else ""
            if not email:
                self._json(400, {"ok": False, "error": "expired"})
                return
            with MUTEX():
                users = load_json(USERS_FILE, {})
                if not isinstance(users, dict) or email not in users:
                    self._json(400, {"ok": False, "error": "expired"})
                    return
                salt = secrets.token_hex(16)
                users[email] = {"salt": salt, "hash": hash_pass(password, salt), "iter": PBKDF2_ITERS}
                save_json(USERS_FILE, users)
            revoke_sessions(email=email)
            AUTH_GUARD.reset("auth:" + ip)
            self._json(200, {"ok": True, "email": email})
            return
        if parsed.path in ("/api/auth/register", "/api/auth/login"):
            is_login = parsed.path.endswith("login")
            if limited(ip, "auth", 20):
                self._json(429, {"ok": False, "error": "rate"})
                return
            email = clean(data.get("email"), 120).lower()
            password = str(data.get("pass", ""))[:128]
            email_ok = bool(re.match(r"^[^@\s]+@[^@\s]+\.[^@\s]{2,}$", email))
            if not email_ok or len(password) < (1 if is_login else 8):
                self._json(400, {"ok": False, "error": "invalid"})
                return
            guard_key = "auth:" + ip
            if is_login:
                allowed, retry = AUTH_GUARD.check(guard_key)
                if not allowed:
                    self._json(429, {"ok": False, "error": "locked", "retryAfter": retry},
                               extra=(("Retry-After", str(retry)),))
                    return
            with MUTEX():
                users = load_json(USERS_FILE, {})
                if not isinstance(users, dict):
                    users = {}
                if not is_login:
                    if email in users:
                        self._json(409, {"ok": False, "error": "exists"})
                        return
                    salt = secrets.token_hex(16)
                    users[email] = {"salt": salt, "hash": hash_pass(password, salt),
                                    "iter": PBKDF2_ITERS}
                    save_json(USERS_FILE, users)
                else:
                    u = users.get(email)
                    stored = u.get("hash", "") if isinstance(u, dict) else ""
                    try:
                        candidate = hash_pass(password, u.get("salt", ""),
                                              int(u.get("iter") or 100000)) if stored else ""
                    except (ValueError, TypeError):
                        candidate = ""
                    if not stored or not hmac.compare_digest(stored, candidate):
                        AUTH_GUARD.fail(guard_key)
                        # 404/401 is kept deliberately: the SPA uses 404 to offer
                        # registration. The rate limiter above blunts enumeration.
                        self._json(401 if email in users else 404,
                                   {"ok": False, "error": "wrong" if email in users else "unknown"})
                        return
                    AUTH_GUARD.reset(guard_key)
                    # Transparently upgrade old 100k-round hashes after a good login.
                    if int(u.get("iter") or 100000) < PBKDF2_ITERS:
                        salt = secrets.token_hex(16)
                        u["salt"] = salt
                        u["hash"] = hash_pass(password, salt)
                        u["iter"] = PBKDF2_ITERS
                        users[email] = u
                        save_json(USERS_FILE, users)
                token = secrets.token_hex(TOKEN_BYTES)
                tokens = sweep_tokens()
                tokens[token_hash(token)] = {"email": email, "iat": int(time.time()),
                                             "ip": ip, "exp": time.time() + USER_TOKEN_TTL}
                save_json(TOKENS_FILE, tokens)
            self._json(200, {"ok": True, "token": token, "email": email},
                       extra=(("Set-Cookie",
                               self._cookie_header(COOKIE_USER, token, USER_TOKEN_TTL)),))
            return
        if parsed.path == "/api/orders":
            if limited(ip, "orders", 8):
                self._json(429, {"ok": False, "error": "rate"})
                return
            items = data.get("items") or []
            # Customer names may legitimately contain & (e.g. "Johnson & Sons"):
            # allow it here — output is escaped at render + enforced by CSP.
            name = clean(data.get("name"), 80)
            commune = clean(data.get("commune"), 80)
            phone = "".join(c for c in str(data.get("phone", "")) if c.isdigit())
            try:
                wilaya = int(data.get("wilaya") or 0)
            except (TypeError, ValueError):
                wilaya = 0
            method = data.get("method") if data.get("method") in ("home", "stop") else "home"
            if len(name) < 3 or not re.match(r"^0[567]\d{8}$", phone) \
                    or not 1 <= wilaya <= 58 or len(commune) < 2 \
                    or re.search(r"[<>]", name) or re.search(r"[<>]", commune):
                self._json(400, {"ok": False, "error": "invalid"})
                return
            if not isinstance(items, list) or not items:
                self._json(400, {"ok": False, "error": "empty"})
                return
            if len(items) > MAX_ORDER_LINES:
                self._json(400, {"ok": False, "error": "too-many-lines"})
                return
            # Signed-in customers: the order belongs to the session's email, not
            # to whatever the browser sent. Guests may add an email for the
            # confirmation message.
            session_email = self._user(self._presented_token(data))
            guest_email = clean(data.get("email"), 120).lower()
            order_email = session_email or (guest_email if mailer.valid_email(guest_email) else "")
            placed = {}
            # Idempotency key: prefer the HTTP header, fall back to the JSON body.
            idem = str(self.headers.get("Idempotency-Key") or data.get("idem") or "")
            idem = re.sub(r"[^A-Za-z0-9:_-]", "", idem)[:64]
            try:
                # Stock check + decrement + order record are ONE atomic unit:
                # any failure (or crash) rolls all three back together.
                with transaction():
                    settings = get_settings()

                    def place():
                        promo_code = re.sub(r"[^A-Z0-9-]", "",
                                            str(data.get("promo", "")).strip().upper())[:20]
                        promo = find_promo(settings.get("promos", []), promo_code)
                        promo_pct = promo.get("pct", 0) if promo else 0
                        products = load_json(PRODUCTS_FILE, [])
                        by_id = {p.get("id"): p for p in products}
                        # Aggregate duplicate item IDs to prevent duplicate-line overselling
                        req_qty = {}
                        line_map = {}
                        for it in items:
                            if not isinstance(it, dict):
                                raise StoreError("bad-item")
                            qty = safe_int(it.get("qty", 0))
                            pid = it.get("id")
                            if not pid or pid not in by_id or qty < 1 \
                                    or qty > MAX_LINE_QTY:
                                raise StoreError("bad-item")
                            req_qty[pid] = req_qty.get(pid, 0) + qty
                            sizes = by_id[pid].get("sizes") or []
                            raw_size = re.sub(r"[^0-9A-Za-z À-ÿ-]", "",
                                              str(it.get("size", ""))).strip()[:10]
                            if sizes and raw_size not in [str(s) for s in sizes]:
                                raise StoreError("bad-size::" + str(pid))
                            key = (pid, raw_size)
                            line_map[key] = line_map.get(key, 0) + qty
                        sub, count = 0, 0
                        for pid, qty in req_qty.items():
                            p = by_id[pid]
                            if qty > safe_int(p.get("stock", 0)):
                                raise StoreError("out-of-stock::" + str(pid))
                            sub += int(p.get("price", 0)) * qty
                            count += qty
                        if count > MAX_ORDER_ITEMS or len(line_map) > MAX_ORDER_LINES:
                            raise StoreError("bad-item")
                        lines = [{"id": pid, "name": str(by_id[pid].get("name", ""))[:120],
                                  "size": size, "qty": q}
                                 for (pid, size), q in line_map.items()]
                        disc = round(sub * promo_pct / 100) if promo_pct else 0
                        provider = str(data.get("provider", "standard"))
                        if provider not in ship_config().get("providers", {}):
                            provider = "standard"
                        free_over = ship_config().get("freeOver", 8000)
                        ship = 0 if (ship_free(provider) and sub - disc >= free_over) or sub == 0 \
                            else ship_fee(provider, wilaya, method)
                        for pid, qty in req_qty.items():
                            by_id[pid]["stock"] = safe_int(by_id[pid].get("stock", 0)) - qty
                        save_json(PRODUCTS_FILE, products)
                        orders = load_json(ORDERS_FILE, [])
                        num = next_order_num()
                        total_num = sub - disc + ship
                        orders.append({"num": num, "items": count, "total": money(total_num),
                                       "totalNum": total_num,
                                       "sub": sub, "disc": disc, "ship": ship, "status": "pending",
                                       "name": name, "phone": phone, "commune": commune,
                                       "wilaya": wilaya, "method": method, "provider": provider,
                                       "ts": int(time.time()), "promo": promo_code if promo else "",
                                       "idem": idem, "lines": lines,
                                       "email": order_email})
                        # Every order is kept: they are business records. (The
                        # old version silently dropped all but the last 500.)
                        save_json(ORDERS_FILE, orders)
                        placed["order"] = orders[-1]
                        return {"num": num, "items": count, "total": money(total_num)}

                    if idem:
                        payload, duplicate = once("order:" + idem, place)
                        payload = dict(payload) if isinstance(payload, dict) else {}
                        if duplicate:
                            payload["duplicate"] = True
                    else:
                        payload = place()
            except StoreError as exc:
                msg = str(exc)
                if msg.startswith("out-of-stock::"):
                    self._json(409, {"ok": False, "error": "out-of-stock",
                                     "id": msg.split("::", 1)[1]})
                elif msg.startswith("bad-size::"):
                    self._json(400, {"ok": False, "error": "bad-size",
                                     "id": msg.split("::", 1)[1]})
                elif msg == "bad-item":
                    self._json(400, {"ok": False, "error": "bad-item"})
                else:
                    report_error("order transaction rolled back: %s" % msg)
                    self._json(500, {"ok": False, "error": "transaction-failed"})
                return
            order = placed.get("order")
            if order:   # a duplicate retry re-sends nothing
                shop = get_settings().get("siteName", "Noor")
                if order.get("email"):
                    mailer.order_confirmation(order, shop)
                mailer.admin_new_order(order, shop)
                maybe_backup()
            self._json(200, {"ok": True, **payload})
            return
        if parsed.path == "/api/admin/settings":
            if not self._require_admin(data):
                self._json(401, {"ok": False, "error": "unauthorized"})
                return
            with MUTEX():
                st = get_settings()
                try:
                    if "whatsapp" in data:
                        wa = "".join(c for c in str(data.get("whatsapp", "")) if c.isdigit() or c == "+")[:20]
                        if wa.startswith("+"):
                            wa = wa[1:]
                        st["whatsapp"] = wa
                    if "siteName" in data:
                        st["siteName"] = clean_text(data.get("siteName"), 40) or "Noor"
                    if isinstance(data.get("social"), dict):
                        social = dict(st.get("social", DEFAULT_SETTINGS["social"]))
                        for key in ("instagram", "facebook", "tiktok"):
                            if key in data["social"]:
                                social[key] = clean_social(key, data["social"].get(key, ""))
                        st["social"] = social
                    if "logo" in data:
                        logo = str(data.get("logo", ""))[:120]
                        if logo and not (
                            re.match(r"^(images/[\w\-]+\.(jpg|jpeg|png|webp))$", logo, re.I)
                            or re.match(r"^api/logo\?v=\d{1,12}$", logo)
                        ):
                            logo = st.get("logo", "")
                        st["logo"] = logo
                        if not logo:
                            # Removing the brand mark must also drop the stored
                            # bytes, otherwise /api/logo would keep serving it.
                            save_json(LOGO_FILE, {"data": "", "mime": ""})
                except ValueError:
                    self._json(400, {"ok": False, "error": "invalid"})
                    return
                if isinstance(data.get("theme"), dict):
                    th = dict(st.get("theme", DEFAULT_THEME))
                    for k in ("shade", "fg", "rose", "roseDeep", "blue"):
                        if k in data["theme"]:
                            th[k] = valid_hex(data["theme"][k], th.get(k, DEFAULT_THEME[k]))
                    if "preset" in data["theme"]:
                        th["preset"] = clean(data["theme"]["preset"], 20) or "custom"
                    st["theme"] = th
                if isinstance(data.get("banner"), dict):
                    bn = dict(st.get("banner", DEFAULT_SETTINGS["banner"]))
                    if "text" in data["banner"]:
                        try:
                            bn["text"] = clean_text(data["banner"]["text"], 200)
                        except ValueError:
                            self._json(400, {"ok": False, "error": "invalid"})
                            return
                    if "active" in data["banner"]:
                        bn["active"] = bool(data["banner"]["active"])
                    for k in ("bg", "fg"):
                        if k in data["banner"]:
                            bn[k] = valid_hex(data["banner"][k], bn.get(k, "#171214" if k == "bg" else "#ffffff"))
                    st["banner"] = bn
                save_json(SETTINGS_FILE, st)
            self._json(200, {"ok": True, "settings": st, "whatsapp": st.get("whatsapp", ""),
                             "social": st.get("social", DEFAULT_SETTINGS["social"])})
            return
        if parsed.path == "/api/admin/promos":
            if not self._require_admin(data):
                self._json(401, {"ok": False, "error": "unauthorized"})
                return
            promos = data.get("promos")
            if not isinstance(promos, list):
                self._json(400, {"ok": False, "error": "invalid"})
                return
            clean_list = []
            for p in promos[:20]:
                if not isinstance(p, dict):
                    continue
                code = re.sub(r"[^A-Z0-9-]", "", str(p.get("code", "")).upper())[:20]
                try:
                    pct = int(p.get("pct", 0))
                    label = clean_text(p.get("label"), 80)
                except (TypeError, ValueError):
                    continue
                if code and 1 <= pct <= 90:
                    clean_list.append({"code": code, "pct": pct,
                                       "active": bool(p.get("active", True)),
                                       "label": label})
            if not clean_list:
                self._json(400, {"ok": False, "error": "empty"})
                return
            with MUTEX():
                st = get_settings()
                st["promos"] = clean_list
                save_json(SETTINGS_FILE, st)
            self._json(200, {"ok": True, "promos": clean_list})
            return
        if parsed.path == "/api/admin/logo":
            if not self._require_admin(data):
                self._json(401, {"ok": False, "error": "unauthorized"})
                return
            fname = os.path.basename(str(data.get("filename", "")))[:60]
            raw = str(data.get("data", ""))
            m = re.match(r"^data:image/(png|jpe?g|webp);base64,(.+)$", raw, re.S)
            ext = ""
            if m:
                ext = ".jpg" if m.group(1) in ("jpg", "jpeg") else "." + m.group(1)
                b64 = m.group(2)
            elif re.match(r"^[\w\-]+\.(png|jpe?g|webp)$", fname, re.I) and raw:
                ext = "." + fname.rsplit(".", 1)[1].lower()
                b64 = re.sub(r"^data:[^,]*,", "", raw)
            else:
                self._json(400, {"ok": False, "error": "invalid"})
                return
            if ext not in LOGO_EXTS:
                self._json(400, {"ok": False, "error": "invalid"})
                return
            try:
                blob = base64.b64decode(b64, validate=True)
            except Exception:
                self._json(400, {"ok": False, "error": "invalid"})
                return
            if len(blob) > 1500000 or len(blob) < 100:
                self._json(400, {"ok": False, "error": "size"})
                return
            if ext in (".jpg", ".jpeg") and blob[:2] != b"\xff\xd8":
                self._json(400, {"ok": False, "error": "invalid"})
                return
            if ext == ".png" and blob[:8] != b"\x89PNG\r\n\x1a\n":
                self._json(400, {"ok": False, "error": "invalid"})
                return
            if ext == ".webp" and (blob[:4] != b"RIFF" or blob[8:12] != b"WEBP"):
                self._json(400, {"ok": False, "error": "invalid"})
                return
            mime = {".jpg": "image/jpeg", ".jpeg": "image/jpeg",
                    ".png": "image/png", ".webp": "image/webp"}[ext]
            target = LOGO_FILE
            with MUTEX():
                save_json(target, {"data": base64.b64encode(blob).decode("ascii"),
                                   "mime": mime})
                st = get_settings()
                st["logo"] = "api/logo?v=%d" % int(time.time())
                save_json(SETTINGS_FILE, st)
            self._json(200, {"ok": True, "logo": st["logo"]})
            return
        if parsed.path == "/api/admin/images":
            if not self._require_admin(data):
                self._json(401, {"ok": False, "error": "unauthorized"})
                return
            try:
                imgs = sorted(f for f in os.listdir(os.path.join(ROOT, "images"))
                              if f.lower().endswith((".jpg", ".jpeg", ".png", ".webp")))
            except OSError:
                imgs = []
            self._json(200, {"ok": True, "images": imgs})
            return
        if parsed.path == "/api/admin/product-image":
            if not self._require_admin(data):
                self._json(401, {"ok": False, "error": "unauthorized"})
                return
            fname = os.path.basename(str(data.get("filename", "")))[:80]
            raw = str(data.get("data", ""))
            m = re.match(r"^data:image/(png|jpe?g|webp);base64,(.+)$", raw, re.S)
            if not m:
                self._json(400, {"ok": False, "error": "invalid"})
                return
            ext = ".jpg" if m.group(1) in ("jpg", "jpeg") else "." + m.group(1)
            try:
                blob = base64.b64decode(m.group(2), validate=True)
            except Exception:
                self._json(400, {"ok": False, "error": "invalid"})
                return
            if len(blob) > 1500000 or len(blob) < 100:
                self._json(400, {"ok": False, "error": "size"})
                return
            if ext in (".jpg", ".jpeg") and blob[:2] != b"\xff\xd8":
                self._json(400, {"ok": False, "error": "invalid"})
                return
            if ext == ".png" and blob[:8] != b"\x89PNG\r\n\x1a\n":
                self._json(400, {"ok": False, "error": "invalid"})
                return
            if ext == ".webp" and (blob[:4] != b"RIFF" or blob[8:12] != b"WEBP"):
                self._json(400, {"ok": False, "error": "invalid"})
                return
            base = re.sub(r"[^a-zA-Z0-9_-]+", "-", os.path.splitext(fname)[0]).strip("-_").lower()[:40] or "product"
            fn = "%s-%s%s" % (base, secrets.token_hex(6), ext)
            images_dir = os.path.join(ROOT, "images")
            try:
                os.makedirs(images_dir, exist_ok=True)
                target = os.path.join(images_dir, fn)
                tmp = target + ".part"
                with open(tmp, "wb") as fh:
                    fh.write(blob)
                os.replace(tmp, target)
            except OSError:
                self._json(500, {"ok": False, "error": "write"})
                return
            write_webp_variants(target, os.path.splitext(fn)[0])
            self._json(200, {"ok": True, "img": "images/" + fn})
            return
        if parsed.path == "/api/admin/product":
            if not self._require_admin(data):
                self._json(401, {"ok": False, "error": "unauthorized"})
                return
            p = data.get("product") or {}
            pid = re.sub(r"[^a-z0-9-]", "", str(p.get("id", "")).lower().replace(" ", "-"))[:60].strip("-")
            if not pid and str(p.get("name", "")).strip():
                pid = re.sub(r"[^a-z0-9-]", "", str(p.get("name")).lower().replace(" ", "-"))[:60].strip("-")
            if not pid:
                pid = "product-" + secrets.token_hex(4)
            try:
                price = int(p.get("price", 0))
                stock = int(p.get("stock", 0))
                pmin = int(p.get("min", price))
                pmax = int(p.get("max", price))
            except (TypeError, ValueError):
                price = stock = pmin = pmax = 0
            img = os.path.basename(str(p.get("img", "")))[:80]
            if not re.match(r"^[\w\-]+\.(jpg|jpeg|png|webp)$", img, re.I):
                img = ""
            try:
                name = clean_text(p.get("name"), 120)
                size = clean_text(p.get("size"), 20)
                desc = clean_text(p.get("desc"), 400)
                tag = clean_text(p.get("tag"), 20)
                fabric = clean_text(p.get("fabric"), 60)
                care = clean_text(p.get("care"), 120)
                fit = clean_text(p.get("fit"), 60)
            except ValueError:
                self._json(400, {"ok": False, "error": "invalid"})
                return
            raw_sizes = p.get("sizes")
            if isinstance(raw_sizes, str):
                raw_sizes = raw_sizes.split(",")
            sizes = []
            if isinstance(raw_sizes, list):
                for s in raw_sizes[:8]:
                    sv = str(s).strip()[:10]
                    if sv:
                        sizes.append(sv)
            images = []
            raw_images = p.get("images")
            if isinstance(raw_images, list):
                for item in raw_images[:8]:
                    fn = os.path.basename(str(item or ""))[:80]
                    if re.match(r"^[\w\-]+\.(jpg|jpeg|png|webp)$", fn, re.I):
                        full = "images/" + fn
                        if full not in images:
                            images.append(full)
            if img and ("images/" + img) not in images:
                images.insert(0, "images/" + img)
            if not img and images:
                img = os.path.basename(images[0])
            cover = ("images/" + img) if img else ""
            if not pid or not name or price <= 0 or stock < 0 \
                    or not 0 <= pmin <= price <= pmax \
                    or p.get("cat") not in PRODUCT_CATS or not cover:
                self._json(400, {"ok": False, "error": "invalid"})
                return
            dims = img_dims(os.path.join(ROOT, "images", img))
            with MUTEX():
                products = load_json(PRODUCTS_FILE, [])
                entry = {"id": pid, "name": name, "size": size,
                         "desc": desc, "price": price, "min": pmin, "max": pmax,
                         "tag": tag, "cat": p.get("cat"),
                         "img": cover, "images": images, "stock": stock,
                         "sizes": sizes, "fabric": fabric, "care": care, "fit": fit}
                if dims:
                    entry["w"], entry["h"] = dims
                found = False
                for i, old in enumerate(products):
                    if old.get("id") == pid:
                        products[i] = entry
                        found = True
                if not found:
                    products.append(entry)
                save_json(PRODUCTS_FILE, products)
            self._json(200, {"ok": True, "id": pid, "created": not found})
            return
        if parsed.path == "/api/admin/product/delete":
            if not self._require_admin(data):
                self._json(401, {"ok": False, "error": "unauthorized"})
                return
            pid = clean(data.get("id"), 60)
            with MUTEX():
                products = load_json(PRODUCTS_FILE, [])
                products = [p for p in products if p.get("id") != pid]
                save_json(PRODUCTS_FILE, products)
            self._json(200, {"ok": True})
            return
        if parsed.path == "/api/admin/products/delete":
            if not self._require_admin(data):
                self._json(401, {"ok": False, "error": "unauthorized"})
                return
            raw_ids = data.get("ids")
            if not isinstance(raw_ids, list):
                self._json(400, {"ok": False, "error": "invalid"})
                return
            ids = set()
            for item in raw_ids[:100]:
                pid = clean(item, 60)
                if pid:
                    ids.add(pid)
            if not ids:
                self._json(400, {"ok": False, "error": "empty"})
                return
            with MUTEX():
                products = load_json(PRODUCTS_FILE, [])
                before = len(products)
                products = [p for p in products if p.get("id") not in ids]
                save_json(PRODUCTS_FILE, products)
            self._json(200, {"ok": True, "deleted": before - len(products)})
            return
        self._json(404, {"ok": False, "error": "not found"})


def main():
    port = int(os.environ.get("PORT", "8124"))
    host = os.environ.get("HOST", "127.0.0.1")
    if not admin_login_available():
        sys.stderr.write(
            "SECURITY: admin sign-in is DISABLED — set a strong ADMIN_PIN "
            "(>= 8 chars, not a known default) once, then create the admin "
            "account in Admin -> Settings.\n")
    # Re-sync the JSON recovery mirrors from the authoritative backend so a
    # later database loss cannot re-seed from a stale/partial catalog.
    try:
        _mirrored = _store.mirror_all()
        print("Store mirrors re-synced: %d documents." % _mirrored, flush=True)
    except Exception as exc:  # never block boot on a mirror refresh
        sys.stderr.write("WARN: mirror re-sync failed: %s\n" % exc)
    server = BoundedThreadingHTTPServer((host, port), Handler)

    def _term(signum, frame):
        threading.Thread(target=server.shutdown, daemon=True).start()

    signal.signal(signal.SIGTERM, _term)
    signal.signal(signal.SIGINT, _term)
    print("Noor listening on http://%s:%d/  (admin=%s, tls=%s)" % (
        host, port, "enabled" if admin_login_available() else "DISABLED",
        "on" if ENABLE_HSTS else "off"), flush=True)
    server.serve_forever()


if __name__ == "__main__":
    main()
