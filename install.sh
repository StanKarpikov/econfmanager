#!/bin/bash
set -euo pipefail

# -----------------------------
# Resolve cargo target directory
# -----------------------------

# Preferred order:
#  1. CARGO_BUILD_TARGET (cargo standard)
#  2. RUST_TARGET        (Yocto convention)
#  3. Host build fallback

if [[ -n "${CARGO_BUILD_TARGET:-}" ]]; then
    CARGO_TARGET="${CARGO_BUILD_TARGET}"
elif [[ -n "${RUST_TARGET:-}" ]]; then
    CARGO_TARGET="${RUST_TARGET}"
else
    CARGO_TARGET=""
fi

if [[ -n "$CARGO_TARGET" ]]; then
    TARGET_DIR="target/${CARGO_TARGET}/release"
else
    TARGET_DIR="target/release"
fi

# -----------------------------
# Artifacts
# -----------------------------

HEADER_FILE="${TARGET_DIR}/econfmanager.h"
STATIC_LIB="${TARGET_DIR}/libeconfmanager.a"
DYN_LIB="${TARGET_DIR}/libeconfmanager.so"

# -----------------------------
# Parse named parameters
# -----------------------------

LIB_DIR=""
HEADERS_DIR=""

while [[ $# -gt 0 ]]; do
    case "$1" in
        --libs-release-folder=*)
            LIB_DIR="${1#*=}"
            shift
            ;;
        --header-release-folder=*)
            HEADERS_DIR="${1#*=}"
            shift
            ;;
        *)
            echo "Unknown parameter: $1"
            echo "Usage: $0 --libs-release-folder=PATH --header-release-folder=PATH"
            exit 1
            ;;
    esac
done

if [[ -z "$LIB_DIR" || -z "$HEADERS_DIR" ]]; then
    echo "Usage: $0 --libs-release-folder=PATH --header-release-folder=PATH"
    exit 1
fi

# -----------------------------
# Validation
# -----------------------------

if [[ ! -d "$TARGET_DIR" ]]; then
    echo "Error: $TARGET_DIR does not exist."
    echo "Did you run: cargo build --release ${CARGO_TARGET:+--target $CARGO_TARGET} ?"
    exit 1
fi

for f in "$STATIC_LIB" "$DYN_LIB" "$HEADER_FILE"; do
    if [[ ! -f "$f" ]]; then
        echo "Error: missing build artifact: $f"
        exit 1
    fi
done

# -----------------------------
# Install
# -----------------------------

mkdir -p "$LIB_DIR" "$HEADERS_DIR"

echo "Installing econfmanager from:"
echo "  $TARGET_DIR"
    
install -m 0644 "$STATIC_LIB" "$LIB_DIR/"
install -m 0644 "$DYN_LIB" "$LIB_DIR/"
install -m 0644 "$HEADER_FILE" "$HEADERS_DIR/"
install -m 0644 "econfmanager/cmake/CMakeLists.txt" "$HEADERS_DIR/"

echo "Econfmanager installed successfully"
