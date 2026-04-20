#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$SCRIPT_DIR/../../.."
DEMO_DIR="$ROOT_DIR/examples/demo"

export CARGO_TARGET_DIR="$SCRIPT_DIR/target"

cargo build --manifest-path "$DEMO_DIR/Cargo.toml" --lib --release

rm -rf "$SCRIPT_DIR/dist/android/kotlin" "$SCRIPT_DIR/dist/android/include"

cd "$DEMO_DIR"
cargo run --manifest-path "$ROOT_DIR/Cargo.toml" -p boltffi_cli -- \
    --overlay boltffi.benchmark.toml \
    generate header \
    --output ../../benchmarks/generated/boltffi/dist/android/include
cargo run --manifest-path "$ROOT_DIR/Cargo.toml" -p boltffi_cli -- \
    --overlay boltffi.benchmark.toml \
    generate kotlin

perl -0pi -e 's/DataPointReader\.read\(buffer, 0\)/DataPoint.decode(WireReader(buffer))/g' \
    "$SCRIPT_DIR/dist/android/kotlin/com/example/bench_boltffi/Demo.kt"
