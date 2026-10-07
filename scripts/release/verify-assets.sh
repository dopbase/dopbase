#!/usr/bin/env bash
set -euo pipefail
# shellcheck source=scripts/release/common.sh
source "$(dirname "$0")/common.sh"
version=${1:?Usage: verify-assets.sh VERSION DIRECTORY}
directory=${2:?Missing directory}
expected=$(release_assets "$version")
cd "$directory"
actual=$(find . -maxdepth 1 -type f -name 'dopbase_*' -exec basename {} \; | LC_ALL=C sort)
[[ "$actual" == "$expected" ]] || release_error "Expected exactly four tar.gz archives for $version; found: $actual"
assets=()
while IFS= read -r archive; do
  gzip -t "$archive"
  [[ "$(tar -tzf "$archive")" == "$(archive_members)" ]] || release_error "Unexpected contents in $archive"
  assets+=("$archive")
done <<< "$expected"
sha256sum "${assets[@]}" > checksums.txt
sha256sum --check checksums.txt
