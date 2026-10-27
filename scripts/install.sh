#!/bin/sh

set -eu

script_dir=$(CDPATH='' cd -- "$(dirname -- "$0")" && pwd)
repo_root=$(CDPATH='' cd -- "${script_dir}/.." && pwd)

from_source=false
for arg in "$@"; do
  case "$arg" in
    --from-source) from_source=true ;;
    -h | --help)
      cat <<'EOF'
Usage: install.sh [--from-source]

Download a release archive from GitHub, verify its checksum, and install dopbase.
When a release archive is missing (for example Windows builds before the first
Windows release), a repository clone can fall back to a local release build.

Environment:
  DOPBASE_FROM_SOURCE=1     Build from the repository instead of downloading
  DOPBASE_REPOSITORY_URL    GitHub repository (default: origin remote or dopbase/dopbase)
  DOPBASE_VERSION           Release tag to install
  DOPBASE_INSTALL_DIR       Installation directory
  DOPBASE_DOWNLOAD_BASE_URL Override the release download base URL
EOF
      exit 0
      ;;
  esac
done

case "${DOPBASE_FROM_SOURCE:-}" in
  1 | true | TRUE | yes | YES) from_source=true ;;
esac

repository_url="${DOPBASE_REPOSITORY_URL:-}"
if [ -z "$repository_url" ] && command -v git >/dev/null 2>&1; then
  remote=$(git -C "$repo_root" remote get-url origin 2>/dev/null || true)
  case "$remote" in
    *github.com/*)
      repository_url=$(printf '%s' "$remote" | sed -E 's#^git@github.com:#https://github.com/#; s#\.git$##; s#^https://github.com/##; s#^http://github.com/##')
      repository_url="https://github.com/${repository_url%%#*}"
      ;;
  esac
fi
repository_url="${repository_url:-https://github.com/dopbase/dopbase}"

auto_source_on_miss=true
if [ -n "${DOPBASE_DOWNLOAD_BASE_URL:-}" ] || [ -n "${DOPBASE_REPOSITORY_URL:-}" ]; then
  case "$repository_url" in
    file://*) auto_source_on_miss=false ;;
  esac
fi
if [ -n "${DOPBASE_DOWNLOAD_BASE_URL:-}" ]; then
  auto_source_on_miss=false
fi

require_command() {
  if ! command -v "$1" >/dev/null 2>&1; then
    echo "dopbase installer: required command not found: $1" >&2
    exit 1
  fi
}

install_built_binary() {
  built_binary=$1
  binary_name=$2
  if [ ! -f "$built_binary" ]; then
    echo "dopbase installer: build did not produce ${built_binary}" >&2
    exit 1
  fi
  mkdir -p "$install_dir"
  temporary_target="${install_dir}/.dopbase.install.$$"
  if [ "$asset_os" = "windows" ]; then
    temporary_target="${temporary_target}.exe"
  fi
  cp "$built_binary" "$temporary_target"
  chmod 755 "$temporary_target" 2>/dev/null || true
  mv -f "$temporary_target" "${install_dir}/${binary_name}"
  echo "Installed Dopbase to ${install_dir}/${binary_name}"
  print_path_instructions
}

print_path_instructions() {
  normalized_install_dir=$install_dir
  if command -v cygpath >/dev/null 2>&1; then
    normalized_install_dir=$(cygpath -u "$install_dir")
  else
    normalized_install_dir=$(printf '%s' "$install_dir" | tr '\\' '/')
  fi

  path_entry=":${PATH}:"
  case "$path_entry" in
    *":${install_dir}:"* | *":${normalized_install_dir}:"*) return 0 ;;
  esac

  echo "Add ${install_dir} to PATH before running dopbase."
  if [ "$asset_os" = "windows" ]; then
    path_line="export PATH=\"${normalized_install_dir}:\$PATH\""
    echo ""
    echo "Git Bash (this session):"
    echo "  ${path_line}"
    echo ""
    echo "Persist in Git Bash:"
    echo "  echo '${path_line}' >> ~/.bashrc && source ~/.bashrc"
    echo ""
    echo "PowerShell (User PATH):"
    echo "  [Environment]::SetEnvironmentVariable('Path', \$env:Path + ';${install_dir}', 'User')"
    if [ "${DOPBASE_INSTALL_ADD_PATH:-}" = 1 ]; then
      bashrc="${HOME}/.bashrc"
      if ! [ -f "$bashrc" ] || ! grep -Fq "$normalized_install_dir" "$bashrc" 2>/dev/null; then
        printf '%s\n' "$path_line" >>"$bashrc"
        echo "dopbase installer: appended PATH to ${bashrc}"
      fi
    fi
  fi
}

install_from_source() {
  if [ ! -f "${repo_root}/app/Cargo.toml" ]; then
    echo "dopbase installer: --from-source requires a Dopbase repository checkout" >&2
    exit 1
  fi
  require_command cargo
  echo "dopbase installer: building from source in ${repo_root}..."
  if [ ! -f "${repo_root}/dist/index.html" ]; then
    if command -v bun >/dev/null 2>&1; then
      (cd "$repo_root" && bun run build:ui)
    else
      echo "dopbase installer: dist/index.html is missing and bun was not found" >&2
      echo "Install Bun, run 'bun install' and 'bun run build:ui', then retry." >&2
      exit 1
    fi
  fi
  (cd "$repo_root" && cargo build --manifest-path app/Cargo.toml --release --locked)
  if [ "$asset_os" = "windows" ]; then
    install_built_binary "${repo_root}/app/target/release/dopbase.exe" "dopbase.exe"
  else
    install_built_binary "${repo_root}/app/target/release/dopbase" "dopbase"
  fi
}

require_command curl
require_command unzip

uname_s=$(uname -s)
case "$uname_s" in
  Darwin) asset_os="darwin" ;;
  Linux) asset_os="linux" ;;
  MINGW* | MSYS* | CYGWIN*)
    asset_os="windows"
    ;;
  *)
    echo "dopbase installer: unsupported operating system: ${uname_s}" >&2
    exit 1
    ;;
esac

case "$(uname -m)" in
  x86_64 | amd64) asset_arch="amd64" ;;
  arm64 | aarch64)
    if [ "$asset_os" = "windows" ]; then
      echo "dopbase installer: Windows releases are available only for amd64" >&2
      exit 1
    fi
    asset_arch="arm64"
    ;;
  *)
    echo "dopbase installer: unsupported architecture: $(uname -m)" >&2
    exit 1
    ;;
esac

if [ -n "${DOPBASE_INSTALL_DIR:-}" ]; then
  install_dir=$DOPBASE_INSTALL_DIR
elif [ "$asset_os" = "windows" ]; then
  if [ -n "${LOCALAPPDATA:-}" ]; then
    install_dir="${LOCALAPPDATA}/Dopbase/bin"
  else
    install_dir="${HOME}/.local/bin"
  fi
else
  install_dir="${HOME}/.local/bin"
fi

if [ "$from_source" = true ]; then
  install_from_source
  exit 0
fi

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
if ! curl -fsSL "${release_base_url}/${archive_name}" -o "$archive_path"; then
  if [ "$auto_source_on_miss" = true ] && [ -f "${repo_root}/app/Cargo.toml" ]; then
    echo "dopbase installer: ${archive_name} is not published for ${release_tag}; building from source..."
    install_from_source
    exit 0
  fi
  echo "dopbase installer: failed to download ${archive_name} from ${release_base_url}" >&2
  if [ "$asset_os" = "windows" ]; then
    echo "Windows release archives appear after the first release that includes them." >&2
    echo "From a repository clone, run: ./scripts/install.sh --from-source" >&2
  fi
  exit 1
fi
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
if [ "$asset_os" = "windows" ]; then
  unzip -q "$archive_path" dopbase.exe -d "$extract_dir"
  binary_name="dopbase.exe"
else
  unzip -q "$archive_path" dopbase -d "$extract_dir"
  binary_name="dopbase"
fi

if [ ! -f "${extract_dir}/${binary_name}" ]; then
  echo "dopbase installer: release archive does not contain ${binary_name}" >&2
  exit 1
fi

install_built_binary "${extract_dir}/${binary_name}" "$binary_name"
