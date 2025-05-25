#!/bin/bash
set -e

TARGET_DIR="target/release"
HEADER_FILE="$TARGET_DIR/econfmanager.h"
STATIC_LIB="$TARGET_DIR/libeconfmanager.a"
DYN_LIB="$TARGET_DIR/libeconfmanager.so"

if [ -z "$1" ] || [ -z "$2" ]; then
    echo "Usage: $0 <library_directory> <headers_directory>"
    exit 1
fi

if [ ! -d "$TARGET_DIR" ]; then
    echo "Error: $TARGET_DIR does not exist. Have you built the project with 'cargo build --release'?"
    exit 1
fi

LIB_DIR="$1"
mkdir -p "$LIB_DIR"
HEADERS_DIR="$2"
mkdir -p "$HEADERS_DIR"

echo "Copying static library to $LIB_DIR and header Econfmanager to $HEADERS_DIR"
cp "$STATIC_LIB" "$LIB_DIR/"
cp "$DYN_LIB" "$LIB_DIR/"
cp "$HEADER_FILE" "$HEADERS_DIR/"
cp "econfmanager/cmake/CMakeLists.txt" "$HEADERS_DIR/"

echo "Econfmanager Installed"
