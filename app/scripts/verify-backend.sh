#!/usr/bin/env bash
set -euo pipefail

cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets
cargo test --all-targets --all-features
cargo build --release --features embedded-ui

runtime_root="$(mktemp -d)"
data_dir="${runtime_root}/data"
server_log="${runtime_root}/server.log"
binary="${runtime_root}/dopbase"
server_pid=""

cleanup() {
  if [[ -n "${server_pid}" ]] && kill -0 "${server_pid}" 2>/dev/null; then
    kill -TERM "${server_pid}" 2>/dev/null || true
    wait "${server_pid}" 2>/dev/null || true
  fi
  rm -rf "${runtime_root}"
}
trap cleanup EXIT

cp ./target/release/dopbase "${binary}"
cd "${runtime_root}"
"${binary}" --data-dir "${data_dir}" serve \
  --docs \
  --bind-address 127.0.0.1:18376 \
  --public-url http://127.0.0.1:18376 \
  >"${server_log}" 2>&1 &
server_pid=$!

for _ in $(seq 1 60); do
  if curl --fail --silent http://127.0.0.1:18376/api/v1/health >/dev/null; then
    break
  fi
  if ! kill -0 "${server_pid}" 2>/dev/null; then
    cat "${server_log}"
    exit 1
  fi
  sleep 0.25
done

curl --fail --silent http://127.0.0.1:18376/ | grep --quiet '<div id="app"></div>'
curl --fail --silent http://127.0.0.1:18376/api/v1/health | grep --quiet '"success":true'
curl --fail --silent http://127.0.0.1:18376/api/v1/openapi.json | grep --quiet '"openapi"'
curl --fail --silent --location http://127.0.0.1:18376/api/docs/ >/dev/null

test -f "${data_dir}/dopbase.db"
test -f "${data_dir}/master.key"
test -f "${data_dir}/dopbase.db.lock"
test ! -e "${runtime_root}/dopbase.db"
grep --quiet "Data:       ${data_dir}" "${server_log}"
grep --quiet "Database:   ${data_dir}/dopbase.db" "${server_log}"

kill -TERM "${server_pid}"
wait "${server_pid}"
server_pid=""
