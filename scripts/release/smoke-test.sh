#!/usr/bin/env bash
# shellcheck source=scripts/release/common.sh
source "$(dirname "$0")/common.sh"
set -euo pipefail
binary=${1:?Usage: smoke-test.sh BINARY VERSION}
version=${2:?Usage: smoke-test.sh BINARY VERSION}
require_version "$version"
SMOKE_PORT=${SMOKE_PORT:-18376}
bind_address="127.0.0.1:${SMOKE_PORT}"
public_url="http://${bind_address}"
test "$("${binary}" --version)" = "v${version}"

runtime_root="$(mktemp -d)"
data_dir="${runtime_root}/data"
server_log="${runtime_root}/server.log"
server_pid=""
cleanup() {
  if [[ -n "${server_pid}" ]] && kill -0 "${server_pid}" 2>/dev/null; then
    kill -TERM "${server_pid}" 2>/dev/null || true
    wait "${server_pid}" 2>/dev/null || true
  fi
  rm -rf "${runtime_root}"
}
trap cleanup EXIT
trap 'exit 130' INT
trap 'exit 143' TERM

# A fresh instance with a disabled UI and no root email needs explicit setup.
if "${binary}" --data-dir "${data_dir}" server start --no-web-ui >"${runtime_root}/fresh-start.out" 2>&1; then
  echo "error: fresh startup unexpectedly succeeded" >&2
  exit 1
fi
grep --quiet 'dopbase server setup' "${runtime_root}/fresh-start.out"
test ! -e "${data_dir}"

DOPBASE_ROOT_EMAIL=smoke@example.com "${binary}" --data-dir "${data_dir}" server start \
  --docs \
  --host 127.0.0.1 \
  --port "${SMOKE_PORT}" \
  --public-url "${public_url}" \
  >"${server_log}" 2>&1 &
server_pid=$!

ready=false
for _ in $(seq 1 60); do
  if curl --fail --silent "${public_url}/api/v1/health" >/dev/null; then
    ready=true
    break
  fi
  if ! kill -0 "${server_pid}" 2>/dev/null; then
    cat "${server_log}"
    exit 1
  fi
  sleep 0.25
done
if [[ "${ready}" != "true" ]]; then
  cat "${server_log}"
  exit 1
fi

health="$(curl --fail --silent "${public_url}/api/v1/health")"
grep --quiet '"success":true' <<<"${health}"
grep --quiet '"product":"dopbase"' <<<"${health}"
grep --quiet "\"version\":\"${version}\"" <<<"${health}"
grep --quiet '"apiVersion":"v1"' <<<"${health}"
grep --quiet '"status":"ok"' <<<"${health}"
curl --fail --silent --show-error "${public_url}/" -o "${runtime_root}/index.html"
grep --quiet '<div id="app"></div>' "${runtime_root}/index.html"
curl --fail --silent --show-error "${public_url}/api/v1/openapi.json" -o "${runtime_root}/openapi.json"
grep --quiet '"openapi"' "${runtime_root}/openapi.json"
curl --fail --silent --location "${public_url}/api/docs/" >/dev/null

grep --quiet 'Password (shown once):' "${server_log}"
test -f "${data_dir}/dopbase.db"
test -f "${data_dir}/master.key"
test -f "${data_dir}/dopbase.db.lock"
test ! -e "${runtime_root}/dopbase.db"
grep --fixed-strings --quiet "Config:     ${data_dir}" "${server_log}"

kill -TERM "${server_pid}"
wait "${server_pid}"
server_pid=""
