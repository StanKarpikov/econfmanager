#!/bin/bash
set -e

TARGET_DIR="target/release"
HEADER_FILE="$TARGET_DIR/econfmanager.h"
STATIC_LIB="$TARGET_DIR/libeconfmanager.a"
JSONRPC_SERVER="$TARGET_DIR/jsonrpc_server"

if [ -z "$1" ]; then
    echo "Usage: $0 <output_directory>"
    exit 1
fi

if [ ! -d "$TARGET_DIR" ]; then
    echo "Error: $TARGET_DIR does not exist. Have you built the project with 'cargo build --release'?"
    exit 1
fi

OUTPUT_DIR="$1"
mkdir -p "$OUTPUT_DIR"

echo "Copying static library and header to $OUTPUT_DIR/"
cp "$STATIC_LIB" "$OUTPUT_DIR/"
cp "$HEADER_FILE" "$OUTPUT_DIR/"
cp "econfmanager/cmake/CMakeLists.txt" "$OUTPUT_DIR/"

cp "$JSONRPC_SERVER" "$OUTPUT_DIR/"

echo "Installed to $OUTPUT_DIR/"
