#!/usr/bin/env bash
set -euo pipefail

have_cmd() {
    command -v "$1" >/dev/null 2>&1
}

with_privileges() {
    if [ "$(id -u)" -eq 0 ]; then
        "$@"
    elif have_cmd sudo; then
        sudo -n "$@"
    else
        return 1
    fi
}

if have_cmd cc && have_cmd c++; then
    echo "C toolchain already available: cc=$(command -v cc), c++=$(command -v c++)"
    exit 0
fi

echo "C toolchain missing; attempting installation for $(uname -s)..."

case "$(uname -s)" in
Linux)
    if have_cmd apt-get; then
        with_privileges apt-get update -y
        with_privileges apt-get install -y --no-install-recommends build-essential
    elif have_cmd dnf; then
        with_privileges dnf install -y gcc gcc-c++ make
    elif have_cmd yum; then
        with_privileges yum install -y gcc gcc-c++ make
    elif have_cmd apk; then
        with_privileges apk add --no-cache build-base
    else
        echo "Unsupported Linux package manager; cannot install C toolchain automatically." >&2
        exit 1
    fi
    ;;
Darwin)
    if ! xcode-select -p >/dev/null 2>&1; then
        echo "Xcode Command Line Tools are required but not installed." >&2
        exit 1
    fi
    ;;
*)
    echo "Unsupported OS: $(uname -s)" >&2
    exit 1
    ;;
esac

if ! have_cmd cc || ! have_cmd c++; then
    echo "C toolchain installation failed: cc/c++ still unavailable." >&2
    exit 1
fi

echo "C toolchain ready: cc=$(command -v cc), c++=$(command -v c++)"
