#!/usr/bin/env bash
set -euo pipefail
# shellcheck source=scripts/release/common.sh
source "$(dirname "$0")/common.sh"
version=${1:?Usage: publish.sh VERSION DIRECTORY}
directory=${2:?Missing directory}
require_version "$version"
: "${GH_REPO:?GH_REPO is required}" "${GH_TOKEN:?GH_TOKEN is required}"
directory=$(cd "$directory" && pwd)
[[ -s "$directory/release-notes.md" && -s "$directory/checksums.txt" ]] || release_error "Missing release notes or checksums"
runtime_dir=$(mktemp -d)
trap 'rm -rf "$runtime_dir"' EXIT
assets=(checksums.txt)
while IFS= read -r archive; do assets+=("$archive"); done < <(release_assets "$version")
manifest_names=$(awk '
  NF != 2 || length($1) != 64 || $1 ~ /[^0-9a-f]/ { invalid=1 }
  { print $2 }
  END { if (invalid || NR != 4) exit 1 }
' "$directory/checksums.txt") || release_error "Invalid checksum manifest"
[[ "$manifest_names" == "$(release_assets "$version")" ]] || release_error "Checksum manifest does not match the expected release assets"
(cd "$directory" && sha256sum --check checksums.txt)
gh api "repos/$GH_REPO/git/ref/tags/$version" > /dev/null
status=$(curl --silent --show-error --connect-timeout 10 --max-time 60 \
  --header "Authorization: Bearer $GH_TOKEN" --header 'Accept: application/vnd.github+json' \
  --output "$runtime_dir/release.json" --write-out '%{http_code}' \
  "https://api.github.com/repos/$GH_REPO/releases/tags/$version")
case "$status" in
  200) jq -e '.id and (.draft | type == "boolean")' "$runtime_dir/release.json" >/dev/null ;;
  404)
    gh release create "$version" --repo "$GH_REPO" --verify-tag --draft \
      --title "Dopbase $version" --notes-file "$directory/release-notes.md"
    printf '{"draft":true}\n' > "$runtime_dir/release.json"
    ;;
  *) release_error "Could not inspect release: GitHub HTTP $status" ;;
esac
verify_uploaded() {
  mkdir -p "$runtime_dir/download"
  for asset in "${assets[@]}"; do
    gh release download "$version" --repo "$GH_REPO" --pattern "$asset" --dir "$runtime_dir/download" --clobber
  done
  cmp "$directory/checksums.txt" "$runtime_dir/download/checksums.txt" || release_error "Published checksums differ; assets were preserved"
  (cd "$runtime_dir/download" && sha256sum --check "$directory/checksums.txt") || release_error "Uploaded assets differ; release was preserved"
}
if [[ "$(jq -r '.draft' "$runtime_dir/release.json")" == false ]]; then
  verify_uploaded
  printf 'Published release %s matches; no changes made.\n' "$version"
  exit 0
fi
gh release edit "$version" --repo "$GH_REPO" --title "Dopbase $version" --notes-file "$directory/release-notes.md"
for asset in "${assets[@]}"; do
  uploaded=false
  for attempt in 1 2 3; do
    if gh release upload "$version" "$directory/$asset" --repo "$GH_REPO" --clobber; then uploaded=true; break; fi
    if [[ "$attempt" != 3 ]]; then sleep 5; fi
  done
  [[ "$uploaded" == true ]] || release_error "Failed to upload $asset after three attempts; release remains a draft"
done
verify_uploaded
gh release edit "$version" --repo "$GH_REPO" --draft=false
