#!/bin/bash
set -e

TARGET_DIR="target/release"
JSONRPC_SERVER="$TARGET_DIR/jsonrpc_server"

if [ -z "$1" ]; then
    echo "Usage: $0 <bin_directory>"
    exit 1
fi

if [ ! -d "$TARGET_DIR" ]; then
    echo "Error: $TARGET_DIR does not exist. Have you built the project with 'cargo build --release'?"
    exit 1
fi

BIN_DIR="$1"
mkdir -p "$BIN_DIR"

echo "Copying binaries $BIN_DIR"
cp "$JSONRPC_SERVER" "$BIN_DIR/"

echo "JSONRPC Server Installed"
