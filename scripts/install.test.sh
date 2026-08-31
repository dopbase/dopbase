#!/bin/sh

set -eu

root_dir=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
test_dir=$(mktemp -d "${TMPDIR:-/tmp}/dopbase-installer-test.XXXXXX")
trap 'rm -rf "$test_dir"' EXIT HUP INT TERM

mock_bin="${test_dir}/bin"
release_dir="${test_dir}/release"
install_dir="${test_dir}/install"
mkdir -p "$mock_bin" "$release_dir" "$install_dir"

printf '%s\n' '#!/bin/sh' 'case "$1" in' '  -s) printf "%s\\n" "${TEST_UNAME_S}" ;;' '  -m) printf "%s\\n" "${TEST_UNAME_M}" ;;' 'esac' >"${mock_bin}/uname"
chmod 755 "${mock_bin}/uname"

printf '%s\n' '#!/bin/sh' 'printf "%s\\n" "dopbase 0.0.8"' >"${test_dir}/dopbase"
chmod 755 "${test_dir}/dopbase"

(cd "$test_dir" && zip -q "${release_dir}/dopbase_0.0.8_darwin_arm64.zip" dopbase)

if command -v sha256sum >/dev/null 2>&1; then
  checksum=$(sha256sum "${release_dir}/dopbase_0.0.8_darwin_arm64.zip" | awk '{ print $1 }')
else
  checksum=$(shasum -a 256 "${release_dir}/dopbase_0.0.8_darwin_arm64.zip" | awk '{ print $1 }')
fi
printf '%s  %s\n' "$checksum" "dopbase_0.0.8_darwin_arm64.zip" >"${release_dir}/checksums.txt"
printf '%s\n' '#!/bin/sh' 'echo old installation' >"${install_dir}/dopbase"
chmod 755 "${install_dir}/dopbase"

PATH="${mock_bin}:${PATH}" \
TEST_UNAME_S=Darwin \
TEST_UNAME_M=arm64 \
DOPBASE_VERSION=v0.0.8 \
DOPBASE_INSTALL_DIR="$install_dir" \
DOPBASE_DOWNLOAD_BASE_URL="file://${release_dir}" \
  sh "${root_dir}/scripts/install.sh" >/dev/null

test -x "${install_dir}/dopbase"
test "$("${install_dir}/dopbase" --version)" = "dopbase 0.0.8"

printf '%064d  %s\n' 0 "dopbase_0.0.8_darwin_arm64.zip" >"${release_dir}/checksums.txt"
if PATH="${mock_bin}:${PATH}" \
  TEST_UNAME_S=Darwin \
  TEST_UNAME_M=arm64 \
  DOPBASE_VERSION=0.0.8 \
  DOPBASE_INSTALL_DIR="$install_dir" \
  DOPBASE_DOWNLOAD_BASE_URL="file://${release_dir}" \
  sh "${root_dir}/scripts/install.sh" >/dev/null 2>&1; then
  echo "installer test: invalid checksum was accepted" >&2
  exit 1
fi

if PATH="${mock_bin}:${PATH}" TEST_UNAME_S=Plan9 TEST_UNAME_M=arm64 \
  DOPBASE_VERSION=0.0.8 sh "${root_dir}/scripts/install.sh" >/dev/null 2>&1; then
  echo "installer test: unsupported operating system was accepted" >&2
  exit 1
fi

if PATH="${mock_bin}:${PATH}" TEST_UNAME_S=Linux TEST_UNAME_M=riscv64 \
  DOPBASE_VERSION=0.0.8 sh "${root_dir}/scripts/install.sh" >/dev/null 2>&1; then
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
  DOPBASE_VERSION=0.0.8 \
  DOPBASE_INSTALL_DIR="$install_dir" \
  DOPBASE_DOWNLOAD_BASE_URL="file://${test_dir}/missing" \
  sh "${root_dir}/scripts/install.sh" >/dev/null 2>&1; then
  echo "installer test: unavailable release was accepted" >&2
  exit 1
fi

echo "installer tests passed"
