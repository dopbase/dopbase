#!/usr/bin/env bash
set -euo pipefail
# shellcheck source=scripts/release/common.sh
source "$(dirname "$0")/common.sh"
target=${1:?Usage: build-binary.sh TARGET}
case "$target" in
  x86_64-unknown-linux-musl|aarch64-unknown-linux-musl)
    cc_var="CC_${target//-/_}"
    linker_var=$(printf 'CARGO_TARGET_%s_LINKER' "$target" | tr '[:lower:]-' '[:upper:]_')
    export "${cc_var}=musl-gcc" "${linker_var}=musl-gcc"
    ;;
  x86_64-apple-darwin|aarch64-apple-darwin) ;;
  *) release_error "Unsupported target: $target" ;;
esac
cargo build --manifest-path app/Cargo.toml --locked --release --target "$target"
