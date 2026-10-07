#!/usr/bin/env bash
set -euo pipefail
# shellcheck source=scripts/release/common.sh
source "$(dirname "$0")/common.sh"
binary=${1:?Usage: package-archive.sh BINARY VERSION OS ARCH OUTPUT_DIR}
version=${2:?Missing version}
asset_os=${3:?Missing OS}
asset_arch=${4:?Missing architecture}
output_dir=${5:?Missing output directory}
require_version "$version"
case "${asset_os}_${asset_arch}" in darwin_amd64|darwin_arm64|linux_amd64|linux_arm64) ;; *) release_error "Unsupported platform" ;; esac
[[ -f "$binary" && ! -L "$binary" ]] || release_error "Missing regular binary: $binary"
mkdir -p "$output_dir"
output_dir=$(cd "$output_dir" && pwd)
archive="$output_dir/dopbase_${version}_${asset_os}_${asset_arch}.tar.gz"
package_dir=$(mktemp -d)
trap 'rm -rf "$package_dir"' EXIT
cp "$binary" "$package_dir/dopbase"
cp CHANGELOG.md LICENSE NOTICE "$package_dir/"
chmod 755 "$package_dir/dopbase"
chmod 644 "$package_dir/CHANGELOG.md" "$package_dir/LICENSE" "$package_dir/NOTICE"
TZ=UTC touch -t 197001010000.00 "$package_dir/"*
export COPYFILE_DISABLE=1 COPY_EXTENDED_ATTRIBUTES_DISABLE=1
if tar --version | grep -q 'GNU tar'; then
  tar --format=ustar --owner=0 --group=0 --numeric-owner -C "$package_dir" -cf - dopbase CHANGELOG.md LICENSE NOTICE
else
  tar --format=ustar --uid 0 --gid 0 --uname '' --gname '' -C "$package_dir" -cf - dopbase CHANGELOG.md LICENSE NOTICE
fi | gzip -n -9 > "$archive"
gzip -t "$archive"
[[ "$(tar -tzf "$archive")" == "$(archive_members)" ]] || release_error "Unexpected archive contents"
mkdir "$package_dir/extracted"
tar -xzf "$archive" -C "$package_dir/extracted"
[[ -x "$package_dir/extracted/dopbase" ]] || release_error "Packaged binary is not executable"
[[ "$("$package_dir/extracted/dopbase" --version)" == "v$version" ]] || release_error "Packaged binary version does not match $version"
printf 'Packaged %s (%s bytes)\n' "$archive" "$(wc -c < "$archive" | tr -d ' ')"
