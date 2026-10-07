#!/usr/bin/env bash
set -euo pipefail
# shellcheck source=scripts/release/common.sh
source "$(dirname "$0")/common.sh"
version=${1:?Usage: validate-source.sh VERSION}
require_version "$version"
package_version=$(cargo metadata --manifest-path app/Cargo.toml --locked --no-deps --format-version 1 | jq -r '.packages[] | select(.name == "app") | .version')
frontend_version=$(jq -r '.version' package.json)
[[ "$version" == "$package_version" ]] || release_error "Version $version does not match app version $package_version"
[[ "$version" == "$frontend_version" ]] || release_error "Version $version does not match frontend version $frontend_version"
awk -v version="$version" 'NF == 4 && $1 == "##" && $2 == version && $3 == "-" && $4 ~ /^[0-9][0-9][0-9][0-9]-[0-9][0-9]-[0-9][0-9]$/ { found=1 } END { exit !found }' CHANGELOG.md || release_error "CHANGELOG.md needs a dated release heading for $version"
printf 'Validated release %s\n' "$version"
