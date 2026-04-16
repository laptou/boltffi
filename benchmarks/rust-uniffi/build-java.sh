#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

DIST_DIR="dist/java"

cargo build --lib --release

rm -rf "$DIST_DIR"
mkdir -p "$DIST_DIR"

if [ "$(uname)" == "Darwin" ]; then
    LIBRARY_FILE="libbench_uniffi.dylib"
elif [ "$(expr substr $(uname -s) 1 5)" == "Linux" ]; then
    LIBRARY_FILE="libbench_uniffi.so"
else
    echo "Unknown platform: $(uname)"
    exit 1
fi

BINDGEN_JAVA="${UNIFFI_BINDGEN_JAVA:-uniffi-bindgen-java}"

"$BINDGEN_JAVA" generate \
    --out-dir "$DIST_DIR" \
    "target/release/$LIBRARY_FILE"

echo "Java FFM bindings generated in $DIST_DIR"
