#!/usr/bin/env bash
set -euo pipefail

export CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-2}"

cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets -- --test-threads=1
cargo test --all-targets --all-features -- --test-threads=1

# Release builds embed ../dist/ at compile time (rust-embed).
if [[ ! -f ../dist/index.html ]]; then
  echo "error: dist/index.html not found — run 'bun run build' first" >&2
  exit 1
fi
cargo build --release

runtime_root="$(mktemp -d)"
data_dir="${runtime_root}/data"
server_log="${runtime_root}/server.log"
background_data_dir="${runtime_root}/background-data"
binary="${runtime_root}/dopbase"
server_pid=""

cleanup() {
  if [[ -n "${server_pid}" ]] && kill -0 "${server_pid}" 2>/dev/null; then
    kill -TERM "${server_pid}" 2>/dev/null || true
    wait "${server_pid}" 2>/dev/null || true
  fi
  if [[ -x "${binary}" ]] && [[ -f "${background_data_dir}/dopbase.pid" ]]; then
    "${binary}" --data-dir "${background_data_dir}" server down --timeout 2 >/dev/null 2>&1 || true
  fi
  rm -rf "${runtime_root}"
}
trap cleanup EXIT

cp ./target/release/dopbase "${binary}"
cd "${runtime_root}"

# Exercise every public command path against the release binary. The Rust CLI
# tests cover parsing and behavior in isolated temporary directories; this
# matrix catches packaging or command-tree regressions in the Docker image.
"${binary}" --help >/dev/null
cli_command_paths=(
  "server"
  "server start"
  "server up"
  "server down"
  "server status"
  "server logs"
  "client"
  "client connect"
  "client status"
  "login"
  "logout"
  "status"
  "init"
  "project"
  "project create"
  "project list"
  "project show"
  "project rename"
  "project delete"
  "env"
  "env default"
  "env create"
  "env list"
  "env show"
  "env rename"
  "env delete"
  "secret"
  "secret list"
  "secret set"
  "secret get"
  "secret delete"
  "import"
  "export"
  "token"
  "token create"
  "token list"
  "token revoke"
  "run"
  "cache"
  "cache list"
  "cache clean"
  "admin"
  "admin reset-password"
  "admin factory-reset"
  "update"
  "backup"
  "restore"
)
for command_path in "${cli_command_paths[@]}"; do
  read -r -a command_parts <<<"${command_path}"
  "${binary}" "${command_parts[@]}" --help >/dev/null
done

"${binary}" --data-dir "${data_dir}" server start \
  --docs \
  --host 127.0.0.1 \
  --port 18376 \
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
grep --quiet "Config:     ${data_dir}" "${server_log}"

kill -TERM "${server_pid}"
wait "${server_pid}"
server_pid=""

"${binary}" --data-dir "${background_data_dir}" --json server up \
  --docs \
  --host 127.0.0.1 \
  --port 18377 \
  >"${runtime_root}/background-start.json"
curl --fail --silent http://127.0.0.1:18377/api/v1/health | grep --quiet '"success":true'
"${binary}" --data-dir "${background_data_dir}" --json server status \
  | grep --quiet '"status": "running"'
"${binary}" --data-dir "${background_data_dir}" server logs --lines 5 >/dev/null
"${binary}" --data-dir "${background_data_dir}" server down
if "${binary}" --data-dir "${background_data_dir}" server status >/dev/null; then
  echo "error: background server still reports as running" >&2
  exit 1
fi
