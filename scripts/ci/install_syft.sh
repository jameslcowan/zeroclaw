#!/usr/bin/env bash
set -euo pipefail

# Install a pinned syft binary into a writable bin directory.
# Usage: ./scripts/ci/install_syft.sh <bin_dir> [version]

BIN_DIR="${1:-${RUNNER_TEMP:-/tmp}/bin}"
VERSION="${2:-${SYFT_VERSION:-v1.42.1}}"

download_file() {
  local url="$1"
  local output="$2"
  if command -v curl >/dev/null 2>&1; then
    curl -sSfL "${url}" -o "${output}"
  elif command -v wget >/dev/null 2>&1; then
    wget -qO "${output}" "${url}"
  else
    echo "Missing downloader: install curl or wget" >&2
    return 1
  fi
}

verify_sha256() {
  local file="$1"
  local expected="$2"
  if command -v sha256sum >/dev/null 2>&1; then
    echo "${expected}  ${file}" | sha256sum -c -
  elif command -v shasum >/dev/null 2>&1; then
    local actual
    actual="$(shasum -a 256 "${file}" | awk '{print $1}')"
    if [ "${actual}" != "${expected}" ]; then
      echo "SHA256 mismatch for ${file}" >&2
      return 1
    fi
  else
    echo "Missing checksum tool: install sha256sum or shasum" >&2
    return 1
  fi
}

os_name="$(uname -s | tr '[:upper:]' '[:lower:]')"
case "$os_name" in
  linux|darwin) ;;
  *)
    echo "Unsupported OS for syft installer: ${os_name}" >&2
    exit 2
    ;;
esac

arch_name="$(uname -m)"
case "$arch_name" in
  x86_64|amd64) arch_name="amd64" ;;
  aarch64|arm64) arch_name="arm64" ;;
  armv7l) arch_name="armv7" ;;
  *)
    echo "Unsupported architecture for syft installer: ${arch_name}" >&2
    exit 2
    ;;
esac

ARCHIVE="syft_${VERSION#v}_${os_name}_${arch_name}.tar.gz"
CHECKSUMS="syft_${VERSION#v}_checksums.txt"
BASE_URL="https://github.com/anchore/syft/releases/download/${VERSION}"

mkdir -p "${BIN_DIR}"
tmp_dir="$(mktemp -d)"
trap 'rm -rf "${tmp_dir}"' EXIT

download_file "${BASE_URL}/${ARCHIVE}" "${tmp_dir}/${ARCHIVE}"
download_file "${BASE_URL}/${CHECKSUMS}" "${tmp_dir}/${CHECKSUMS}"

expected_checksum="$(awk -v target="${ARCHIVE}" '$2 == target {print $1}' "${tmp_dir}/${CHECKSUMS}" | head -n1)"
if [ -z "${expected_checksum}" ]; then
  echo "Missing checksum entry for ${ARCHIVE} in ${CHECKSUMS}" >&2
  exit 1
fi

verify_sha256 "${tmp_dir}/${ARCHIVE}" "${expected_checksum}"

tar -xzf "${tmp_dir}/${ARCHIVE}" -C "${tmp_dir}" syft
install -m 0755 "${tmp_dir}/syft" "${BIN_DIR}/syft"

echo "Installed syft ${VERSION} to ${BIN_DIR}/syft"
