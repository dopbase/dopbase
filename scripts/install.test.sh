#!/bin/sh

set -eu

root_dir=$(CDPATH='' cd -- "$(dirname -- "$0")/.." && pwd)
test_dir=$(mktemp -d "${TMPDIR:-/tmp}/dopbase-installer-test.XXXXXX")
trap 'rm -rf "$test_dir"' EXIT HUP INT TERM
real_curl=$(command -v curl)

mock_bin="${test_dir}/bin"
repository_dir="${test_dir}/repository"
release_dir="${repository_dir}/releases/download/0.0.12"
legacy_release_dir="${repository_dir}/releases/download/v0.0.12"
install_dir="${test_dir}/install"
windows_install_dir="${test_dir}/windows-install"
mkdir -p "$mock_bin" "$release_dir" "$legacy_release_dir" "$install_dir" "$windows_install_dir"

# The variable references are intentionally literal: this writes a mock script.
# shellcheck disable=SC2016
printf '%s\n' '#!/bin/sh' 'case "$1" in' '  -s) printf "%s\\n" "${TEST_UNAME_S}" ;;' '  -m) printf "%s\\n" "${TEST_UNAME_M}" ;;' 'esac' >"${mock_bin}/uname"
chmod 755 "${mock_bin}/uname"

# The variable references are intentionally literal: this writes a mock script.
# shellcheck disable=SC2016
printf '%s\n' \
  '#!/bin/sh' \
  'case " $* " in' \
  '  *" %{url_effective} "*) printf "%s" "${TEST_LATEST_URL}" ;;' \
  '  *) exec "${TEST_REAL_CURL}" "$@" ;;' \
  'esac' >"${mock_bin}/curl"
chmod 755 "${mock_bin}/curl"

printf '%s\n' '#!/bin/sh' 'printf "%s\\n" "v0.0.12"' >"${test_dir}/dopbase"
chmod 755 "${test_dir}/dopbase"

(cd "$test_dir" && zip -q "${release_dir}/dopbase_0.0.12_darwin_arm64.zip" dopbase)
(cd "$test_dir" && cp dopbase dopbase.exe && chmod 755 dopbase.exe && zip -q "${release_dir}/dopbase_0.0.12_windows_amd64.zip" dopbase.exe)

if command -v sha256sum >/dev/null 2>&1; then
  darwin_checksum=$(sha256sum "${release_dir}/dopbase_0.0.12_darwin_arm64.zip" | awk '{ print $1 }')
  windows_checksum=$(sha256sum "${release_dir}/dopbase_0.0.12_windows_amd64.zip" | awk '{ print $1 }')
else
  darwin_checksum=$(shasum -a 256 "${release_dir}/dopbase_0.0.12_darwin_arm64.zip" | awk '{ print $1 }')
  windows_checksum=$(shasum -a 256 "${release_dir}/dopbase_0.0.12_windows_amd64.zip" | awk '{ print $1 }')
fi
{
  printf '%s  %s\n' "$darwin_checksum" "dopbase_0.0.12_darwin_arm64.zip"
  printf '%s  %s\n' "$windows_checksum" "dopbase_0.0.12_windows_amd64.zip"
} >"${release_dir}/checksums.txt"
cp "${release_dir}/dopbase_0.0.12_darwin_arm64.zip" "${legacy_release_dir}/"
cp "${release_dir}/dopbase_0.0.12_windows_amd64.zip" "${legacy_release_dir}/"
cp "${release_dir}/checksums.txt" "${legacy_release_dir}/"
printf '%s\n' '#!/bin/sh' 'echo old installation' >"${install_dir}/dopbase"
chmod 755 "${install_dir}/dopbase"

PATH="${mock_bin}:${PATH}" \
TEST_UNAME_S=Darwin \
TEST_UNAME_M=arm64 \
DOPBASE_VERSION=0.0.12 \
DOPBASE_INSTALL_DIR="$install_dir" \
DOPBASE_REPOSITORY_URL="file://${repository_dir}" \
TEST_REAL_CURL="$real_curl" \
  sh "${root_dir}/scripts/install.sh" >/dev/null

test -x "${install_dir}/dopbase"
test "$("${install_dir}/dopbase" --version)" = "v0.0.12"

PATH="${mock_bin}:${PATH}" \
TEST_UNAME_S=Darwin \
TEST_UNAME_M=arm64 \
TEST_LATEST_URL="file://${release_dir}" \
TEST_REAL_CURL="$real_curl" \
DOPBASE_INSTALL_DIR="$install_dir" \
DOPBASE_REPOSITORY_URL="file://${repository_dir}" \
  sh "${root_dir}/scripts/install.sh" >/dev/null

PATH="${mock_bin}:${PATH}" \
TEST_UNAME_S=Darwin \
TEST_UNAME_M=arm64 \
DOPBASE_VERSION=v0.0.12 \
DOPBASE_INSTALL_DIR="$install_dir" \
DOPBASE_REPOSITORY_URL="file://${repository_dir}" \
TEST_REAL_CURL="$real_curl" \
  sh "${root_dir}/scripts/install.sh" >/dev/null

PATH="${mock_bin}:${PATH}" \
TEST_UNAME_S=MINGW64_NT-10.0 \
TEST_UNAME_M=x86_64 \
DOPBASE_VERSION=0.0.12 \
DOPBASE_INSTALL_DIR="$windows_install_dir" \
DOPBASE_REPOSITORY_URL="file://${repository_dir}" \
TEST_REAL_CURL="$real_curl" \
  sh "${root_dir}/scripts/install.sh" >/dev/null

test -f "${windows_install_dir}/dopbase.exe"
test "$("${windows_install_dir}/dopbase.exe" --version)" = "v0.0.12"

printf '%064d  %s\n' 0 "dopbase_0.0.12_darwin_arm64.zip" >"${release_dir}/checksums.txt"
if PATH="${mock_bin}:${PATH}" \
  TEST_UNAME_S=Darwin \
  TEST_UNAME_M=arm64 \
  DOPBASE_VERSION=0.0.12 \
  DOPBASE_INSTALL_DIR="$install_dir" \
  DOPBASE_REPOSITORY_URL="file://${repository_dir}" \
  TEST_REAL_CURL="$real_curl" \
  sh "${root_dir}/scripts/install.sh" >/dev/null 2>&1; then
  echo "installer test: invalid checksum was accepted" >&2
  exit 1
fi

if PATH="${mock_bin}:${PATH}" TEST_UNAME_S=Plan9 TEST_UNAME_M=arm64 \
  DOPBASE_VERSION=0.0.12 sh "${root_dir}/scripts/install.sh" >/dev/null 2>&1; then
  echo "installer test: unsupported operating system was accepted" >&2
  exit 1
fi

if PATH="${mock_bin}:${PATH}" TEST_UNAME_S=Linux TEST_UNAME_M=riscv64 \
  DOPBASE_VERSION=0.0.12 sh "${root_dir}/scripts/install.sh" >/dev/null 2>&1; then
  echo "installer test: unsupported architecture was accepted" >&2
  exit 1
fi

if PATH="${mock_bin}:${PATH}" TEST_UNAME_S=Darwin TEST_UNAME_M=arm64 \
  DOPBASE_VERSION=not-a-version sh "${root_dir}/scripts/install.sh" >/dev/null 2>&1; then
  echo "installer test: invalid version was accepted" >&2
  exit 1
fi

if PATH="${mock_bin}:${PATH}" \
  TEST_UNAME_S=Darwin \
  TEST_UNAME_M=arm64 \
  DOPBASE_VERSION=0.0.12 \
  DOPBASE_INSTALL_DIR="$install_dir" \
  DOPBASE_DOWNLOAD_BASE_URL="file://${test_dir}/missing" \
  sh "${root_dir}/scripts/install.sh" >/dev/null 2>&1; then
  echo "installer test: unavailable release was accepted" >&2
  exit 1
fi

echo "installer tests passed"
