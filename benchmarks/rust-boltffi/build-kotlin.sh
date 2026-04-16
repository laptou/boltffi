#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$SCRIPT_DIR/../.."

cd "$SCRIPT_DIR"

cargo build --lib --release

rm -rf dist/android/kotlin dist/android/include dist/apple/include

cargo run --manifest-path "$ROOT_DIR/Cargo.toml" -p boltffi_cli -- generate header
cargo run --manifest-path "$ROOT_DIR/Cargo.toml" -p boltffi_cli -- generate kotlin

# Keep the Android include layout used by pack android and benchmark scripts.
mkdir -p dist/android/include
cp dist/apple/include/bench_boltffi.h dist/android/include/bench_boltffi.h
