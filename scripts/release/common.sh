#!/usr/bin/env bash
# Shared release arguments and asset names.
set -euo pipefail

release_error() { printf 'Release: %s\n' "$*" >&2; exit 1; }
require_version() {
  [[ "${1:-}" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]] || release_error "Expected MAJOR.MINOR.PATCH, got ${1:-<empty>}"
}
release_assets() {
  local version=$1
  require_version "$version"
  printf 'dopbase_%s_%s.tar.gz\n' "$version" darwin_amd64 "$version" darwin_arm64 "$version" linux_amd64 "$version" linux_arm64
}
archive_members() { printf '%s\n' dopbase CHANGELOG.md LICENSE NOTICE; }
