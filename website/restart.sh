#!/usr/bin/env bash
set -euo pipefail

# Noor — local/production process restart.
# Usage: ./restart.sh
# Reads .env, refuses to start without a strong ADMIN_PIN, and only stops the
# process recorded in its own PID file (never a test server on another port).

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ENV_FILE="${ROOT}/.env"
PID_FILE="/tmp/noor-server.pid"
LOG_FILE="/tmp/noor-server.log"

if [ -f "${ENV_FILE}" ]; then
  set -a
  # shellcheck disable=SC1090
  . "${ENV_FILE}"
  set +a
fi

: "${PORT:=8124}"
: "${HOST:=127.0.0.1}"
: "${TRUST_PROXY:=1}"
: "${BEHIND_TLS:=1}"
export PORT HOST TRUST_PROXY BEHIND_TLS

if [ -z "${ADMIN_PIN:-}" ] || [ "${#ADMIN_PIN}" -lt 8 ]; then
  echo "ERROR: set a strong ADMIN_PIN (8+ characters) in ${ENV_FILE} or the environment." >&2
  exit 1
fi

if [ -f "${PID_FILE}" ]; then
  OLD_PID="$(cat "${PID_FILE}" 2>/dev/null || true)"
  if [ -n "${OLD_PID}" ] && kill -0 "${OLD_PID}" 2>/dev/null; then
    kill "${OLD_PID}" 2>/dev/null || true
    for _ in $(seq 1 20); do
      kill -0 "${OLD_PID}" 2>/dev/null || break
      sleep 0.1
    done
  fi
  rm -f "${PID_FILE}"
fi

cd "${ROOT}"
setsid nohup python3 server.py > "${LOG_FILE}" 2>&1 < /dev/null &
echo "$!" > "${PID_FILE}"
sleep 1.5

if ! kill -0 "$(cat "${PID_FILE}")" 2>/dev/null; then
  echo "ERROR: server failed to start. See ${LOG_FILE}" >&2
  tail -n 20 "${LOG_FILE}" >&2 || true
  exit 1
fi

echo "Noor listening on http://${HOST}:${PORT}/ (PID $(cat "${PID_FILE}"))"
tail -n 4 "${LOG_FILE}"
