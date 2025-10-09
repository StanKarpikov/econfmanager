#!/bin/bash
set -e

TARGET_DIR="target/release"
HEADER_FILE="$TARGET_DIR/econfmanager.h"
STATIC_LIB="$TARGET_DIR/libeconfmanager.a"
DYN_LIB="$TARGET_DIR/libeconfmanager.so"

# Parse named parameters
while [[ $# -gt 0 ]]; do
    case "$1" in
        --libs-folder=*)
            LIB_DIR="${1#*=}"
            shift
            ;;
        --header-folder=*)
            HEADERS_DIR="${1#*=}"
            shift
            ;;
        *)
            echo "Unknown parameter: $1"
            echo "Usage: $0 --libs-folder=PATH --header-folder=PATH"
            exit 1
            ;;
    esac
done

if [ -z "$LIB_DIR" ] || [ -z "$HEADERS_DIR" ]; then
    echo "Usage: $0 --libs-folder=PATH --header-folder=PATH"
    exit 1
fi

if [ ! -d "$TARGET_DIR" ]; then
    echo "Error: $TARGET_DIR does not exist. Have you built the project with 'cargo build --release'?"
    exit 1
fi

mkdir -p "$LIB_DIR"
mkdir -p "$HEADERS_DIR"

echo "Copying static library to $LIB_DIR and header Econfmanager to $HEADERS_DIR"
cp "$STATIC_LIB" "$LIB_DIR/"
cp "$DYN_LIB" "$LIB_DIR/"
cp "$HEADER_FILE" "$HEADERS_DIR/"
cp "econfmanager/cmake/CMakeLists.txt" "$HEADERS_DIR/"

echo "Econfmanager Installed"
