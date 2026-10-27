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
  case "$archive" in
    *.tar.gz)
      gzip -t "$archive"
      [[ "$(tar -tzf "$archive")" == "$(archive_members)" ]] || release_error "Unexpected contents in $archive"
      ;;
    *.zip)
      members=$(python3 -c 'import sys,zipfile; z=zipfile.ZipFile(sys.argv[1]); print("\n".join(sorted(name.replace("\\","/").split("/")[-1] for name in z.namelist() if name and not name.endswith("/"))))' "$archive" 2>/dev/null \
        || python -c 'import sys,zipfile; z=zipfile.ZipFile(sys.argv[1]); print("\n".join(sorted(name.replace("\\","/").split("/")[-1] for name in z.namelist() if name and not name.endswith("/"))))' "$archive") \
        || release_error "Could not read $archive"
      members=$(printf '%s\n' "$members" | tr -d '\r')
      expected_members=$(printf '%s\n' CHANGELOG.md LICENSE NOTICE dopbase.exe | tr -d '\r')
      [[ "$members" == "$expected_members" ]] || release_error "Unexpected contents in $archive: ${members//$'\n'/, }"
      ;;
    *) release_error "Unsupported archive $archive" ;;
  esac
  assets+=("$archive")
done <<< "$expected"
sha256sum "${assets[@]}" > checksums.txt
sha256sum --check checksums.txt
