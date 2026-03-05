#!/usr/bin/env bash
set -euo pipefail

TOOLCHAIN="${1:-stable}"
STRICT="${ENSURE_CARGO_COMPONENT_STRICT:-false}"

require_or_warn() {
    local msg="$1"
    if [ "${STRICT}" = "true" ]; then
        echo "${msg}" >&2
        exit 1
    fi
    echo "WARN: ${msg}" >&2
    exit 0
}

if ! command -v rustup >/dev/null 2>&1; then
    require_or_warn "rustup not found; cannot validate cargo component."
fi

if rustup run "${TOOLCHAIN}" cargo --version >/dev/null 2>&1; then
    echo "cargo already available on toolchain ${TOOLCHAIN}."
    exit 0
fi

echo "cargo missing on toolchain ${TOOLCHAIN}; attempting install..."
rustup component add cargo --toolchain "${TOOLCHAIN}" || true

if rustup run "${TOOLCHAIN}" cargo --version >/dev/null 2>&1; then
    echo "cargo component ready on toolchain ${TOOLCHAIN}."
    exit 0
fi

require_or_warn "Failed to ensure cargo component for toolchain ${TOOLCHAIN}."
