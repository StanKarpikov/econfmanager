#!/bin/bash
set -ex

PROJECT_DIR="$(pwd)"
RUST_PROJECT_DIR="$PROJECT_DIR/../../"
OUTPUT_LIB_DIR="$PROJECT_DIR/lib"
RUST_OUTPUT_FOLDER="target/release"

# 1. Build Rust library using Cargo
export PARAMETERS_PROTO_PATH=$PROJECT_DIR/proto
cd $RUST_PROJECT_DIR
# cargo clean
cargo build --release

# 2. Copy generated files to local directory
rm -rf "${OUTPUT_LIB_DIR}"
mkdir -p "${OUTPUT_LIB_DIR}"
cp "$RUST_PROJECT_DIR/$RUST_OUTPUT_FOLDER/libeconfmanager.a" "${OUTPUT_LIB_DIR}/"
cp "$RUST_PROJECT_DIR/$RUST_OUTPUT_FOLDER/econfmanager.h" "${OUTPUT_LIB_DIR}/"

# 3. Build C project
cd ${PROJECT_DIR}
mkdir -p build
cd build
cmake ..
make