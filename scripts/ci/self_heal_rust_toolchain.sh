#!/usr/bin/env bash
set -euo pipefail

TOOLCHAIN="${1:-stable}"

if ! command -v rustup >/dev/null 2>&1; then
    echo "rustup not found; skipping Rust toolchain self-heal."
    exit 0
fi

RUSTUP_HOME_DIR="${RUSTUP_HOME:-$HOME/.rustup}"
CARGO_HOME_DIR="${CARGO_HOME:-$HOME/.cargo}"
mkdir -p "${RUSTUP_HOME_DIR}" "${CARGO_HOME_DIR}"

if rustup toolchain list | grep -q "^${TOOLCHAIN}"; then
    if ! rustup run "${TOOLCHAIN}" rustc --version >/dev/null 2>&1; then
        echo "Toolchain ${TOOLCHAIN} appears corrupted; reinstalling."
        rustup toolchain uninstall "${TOOLCHAIN}" || true
    fi
fi

rustup toolchain install "${TOOLCHAIN}" --profile minimal --no-self-update
rustup run "${TOOLCHAIN}" rustc --version
