#!/bin/sh

set -eu

repository_url="${DOPBASE_REPOSITORY_URL:-https://github.com/dopbase/dopbase}"
install_dir="${DOPBASE_INSTALL_DIR:-${HOME}/.local/bin}"

require_command() {
  if ! command -v "$1" >/dev/null 2>&1; then
    echo "dopbase installer: required command not found: $1" >&2
    exit 1
  fi
}

require_command curl
require_command unzip

case "$(uname -s)" in
  Darwin) asset_os="darwin" ;;
  Linux) asset_os="linux" ;;
  *)
    echo "dopbase installer: only macOS and Linux are supported" >&2
    exit 1
    ;;
esac

case "$(uname -m)" in
  x86_64 | amd64) asset_arch="amd64" ;;
  arm64 | aarch64) asset_arch="arm64" ;;
  *)
    echo "dopbase installer: unsupported architecture: $(uname -m)" >&2
    exit 1
    ;;
esac

if [ -n "${DOPBASE_VERSION:-}" ]; then
  release_tag=$DOPBASE_VERSION
else
  latest_url=$(curl -fsSLI -o /dev/null -w '%{url_effective}' "${repository_url}/releases/latest")
  release_tag=${latest_url##*/}
fi

case "$release_tag" in
  v[0-9]*.[0-9]*.[0-9]*) version=${release_tag#v} ;;
  [0-9]*.[0-9]*.[0-9]*) version=$release_tag ;;
  *)
    echo "dopbase installer: invalid release tag: ${release_tag}" >&2
    exit 1
    ;;
esac

if ! printf '%s\n' "$version" | awk -F. '
  NF == 3 && $1 ~ /^[0-9]+$/ && $2 ~ /^[0-9]+$/ && $3 ~ /^[0-9]+$/ { valid = 1 }
  END { exit valid ? 0 : 1 }
'; then
  echo "dopbase installer: invalid version: ${version}" >&2
  exit 1
fi

archive_name="dopbase_${version}_${asset_os}_${asset_arch}.zip"
release_base_url="${DOPBASE_DOWNLOAD_BASE_URL:-${repository_url}/releases/download/${release_tag}}"
temporary_dir=$(mktemp -d "${TMPDIR:-/tmp}/dopbase-install.XXXXXX")
trap 'rm -rf "$temporary_dir"' EXIT HUP INT TERM

archive_path="${temporary_dir}/${archive_name}"
checksums_path="${temporary_dir}/checksums.txt"

echo "Downloading Dopbase ${version} for ${asset_os}/${asset_arch}..."
curl -fsSL "${release_base_url}/${archive_name}" -o "$archive_path"
curl -fsSL "${release_base_url}/checksums.txt" -o "$checksums_path"

expected_checksum=$(awk -v archive="$archive_name" '$2 == archive { print $1 }' "$checksums_path")
if [ -z "$expected_checksum" ]; then
  echo "dopbase installer: checksum not found for ${archive_name}" >&2
  exit 1
fi

if command -v sha256sum >/dev/null 2>&1; then
  actual_checksum=$(sha256sum "$archive_path" | awk '{ print $1 }')
elif command -v shasum >/dev/null 2>&1; then
  actual_checksum=$(shasum -a 256 "$archive_path" | awk '{ print $1 }')
else
  echo "dopbase installer: sha256sum or shasum is required" >&2
  exit 1
fi

if [ "$actual_checksum" != "$expected_checksum" ]; then
  echo "dopbase installer: checksum verification failed" >&2
  exit 1
fi

extract_dir="${temporary_dir}/extract"
mkdir -p "$extract_dir"
unzip -q "$archive_path" dopbase -d "$extract_dir"

if [ ! -f "${extract_dir}/dopbase" ]; then
  echo "dopbase installer: release archive does not contain dopbase" >&2
  exit 1
fi

mkdir -p "$install_dir"
temporary_target="${install_dir}/.dopbase.install.$$"
cp "${extract_dir}/dopbase" "$temporary_target"
chmod 755 "$temporary_target"
mv -f "$temporary_target" "${install_dir}/dopbase"

echo "Installed Dopbase ${version} to ${install_dir}/dopbase"
case ":${PATH}:" in
  *":${install_dir}:"*) ;;
  *) echo "Add ${install_dir} to PATH before running dopbase." ;;
esac
