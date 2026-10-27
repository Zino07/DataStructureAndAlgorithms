"""WSGI adapter for the Noor store (Octenium cPanel Passenger or Gunicorn).

Exposes ``application`` (and ``app``) so the backend in ``noor_app.py`` runs
under Phusion Passenger (cPanel "Setup Python App") or Gunicorn/uWSGI.
"""
from http import HTTPStatus
import io
import os
import sys

ROOT = os.path.dirname(os.path.abspath(__file__))
if ROOT not in sys.path:
    sys.path.insert(0, ROOT)

import noor_app as server  # noqa: E402

# Synchronise the catalog JSON mirrors on WSGI worker startup (best-effort);
# this also removes customer-data copies left by older versions.
try:
    server._store.mirror_all()
except Exception:
    pass


class _Headers:
    def __init__(self, environ):
        self._e = environ

    def get(self, name, default=None):
        key = "HTTP_" + name.upper().replace("-", "_")
        if key == "HTTP_CONTENT_TYPE":
            key = "CONTENT_TYPE"
        elif key == "HTTP_CONTENT_LENGTH":
            key = "CONTENT_LENGTH"
        v = self._e.get(key)
        if v is None and name.lower() == "content-length":
            v = self._e.get("CONTENT_LENGTH")
        return v if v is not None else default


def _resolve_path(environ):
    """Resolve the request path relative to the WSGI mount point."""
    script_name = (environ.get("SCRIPT_NAME") or "").rstrip("/")
    path_info = environ.get("PATH_INFO")
    if path_info:
        return path_info if path_info.startswith("/") else "/" + path_info
    req_uri = (environ.get("REQUEST_URI") or "/").split("?", 1)[0]
    if script_name and req_uri.startswith(script_name):
        req_uri = req_uri[len(script_name):] or "/"
    return req_uri if req_uri.startswith("/") else "/" + req_uri


def _status_line(code):
    try:
        phrase = HTTPStatus(int(code)).phrase
    except ValueError:
        phrase = "OK"
    return "%d %s" % (int(code), phrase)


def application(environ, start_response):
    method = (environ.get("REQUEST_METHOD") or "GET").upper()
    path = _resolve_path(environ)
    query = environ.get("QUERY_STRING", "")
    try:
        length = int(environ.get("CONTENT_LENGTH") or 0)
    except ValueError:
        length = 0
    body = b""
    if length > 0 and environ.get("wsgi.input") is not None:
        try:
            body = environ["wsgi.input"].read(min(length, 3000000))
        except Exception:
            body = b""

    # Pass REMOTE_ADDR as client_address so noor_app.Handler._ip() enforces
    # TRUST_PROXY / TRUSTED_PROXIES before trusting X-Forwarded-For / X-Real-IP.
    peer = (environ.get("REMOTE_ADDR") or "127.0.0.1").strip()

    h = server.Handler.__new__(server.Handler)
    h.command = method
    h.request_version = environ.get("SERVER_PROTOCOL", "HTTP/1.1")
    h.close_connection = False
    h.path = path + (("?" + query) if query else "")
    h.headers = _Headers(environ)
    h.client_address = (peer, 0)

    captured = {}

    def send_response(code, message=None):
        captured["code"] = code

    def send_header(k, v):
        captured.setdefault("headers", []).append((k, v))

    def end_headers():
        pass

    class _W:
        def write(self, b):
            captured["body"] = captured.get("body", b"") + b

    h.send_response = send_response
    h.send_header = send_header
    h.end_headers = end_headers
    h.wfile = _W()
    h.rfile = io.BytesIO(body)

    try:
        if method == "GET":
            h.do_GET()
        elif method == "HEAD":
            h.do_HEAD()
        elif method == "POST":
            h.do_POST()
        else:
            h._json(405, {"ok": False, "error": "method"})
    except server.StoreError:
        server.report_error("storage error on %s %s" % (method, path), sys.exc_info())
        captured = {"code": 500, "headers": [("Content-Type", "application/json; charset=utf-8")],
                    "body": b'{"ok": false, "error": "store"}'}
    except (BrokenPipeError, ConnectionResetError):
        captured = {"code": 500, "headers": [("Content-Type", "application/json; charset=utf-8")],
                    "body": b'{"ok": false, "error": "io"}'}
    except Exception:
        # Anything unexpected: log it, email the owner (throttled), and answer
        # with a plain JSON error instead of Passenger's error page.
        server.report_error("%s %s" % (method, path), sys.exc_info())
        captured = {"code": 500, "headers": [("Content-Type", "application/json; charset=utf-8")],
                    "body": b'{"ok": false, "error": "server"}'}

    code = captured.get("code", 500)
    headers = captured.get("headers", [("Content-Type", "application/json; charset=utf-8")])
    body_out = b"" if method == "HEAD" else captured.get("body", b"")
    start_response(_status_line(code), headers)
    return [body_out]


app = application
