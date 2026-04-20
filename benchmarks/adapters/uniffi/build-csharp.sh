#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$SCRIPT_DIR/../../.."
DEMO_MANIFEST="$ROOT_DIR/examples/demo/Cargo.toml"

DIST_DIR="$SCRIPT_DIR/dist/csharp"
PACKAGE="demo"
BENCH_LIBRARY_BASENAME="bench_uniffi"
UNIFFI_BINDGEN_CS_TAG="v0.10.0+v0.29.4"

resolve_bindgen_cs() {
    if [[ -n "${UNIFFI_BINDGEN_CS:-}" && -x "${UNIFFI_BINDGEN_CS}" ]]; then
        printf '%s\n' "${UNIFFI_BINDGEN_CS}"
        return 0
    fi

    if command -v uniffi-bindgen-cs >/dev/null 2>&1; then
        command -v uniffi-bindgen-cs
        return 0
    fi

    local install_root="$SCRIPT_DIR/target/uniffi-bindgen-cs"
    local install_binary="$install_root/bin/uniffi-bindgen-cs"

    if [[ -x "$install_binary" ]]; then
        printf '%s\n' "$install_binary"
        return 0
    fi

    cargo install \
        uniffi-bindgen-cs \
        --git https://github.com/NordSecurity/uniffi-bindgen-cs \
        --tag "$UNIFFI_BINDGEN_CS_TAG" \
        --root "$install_root"

    printf '%s\n' "$install_binary"
}

if [[ "$(uname)" == "Darwin" ]]; then
    LIBRARY_FILE="lib${PACKAGE}.dylib"
    BENCH_LIBRARY_FILE="lib${BENCH_LIBRARY_BASENAME}.dylib"
elif [[ "$(expr substr "$(uname -s)" 1 5)" == "Linux" ]]; then
    LIBRARY_FILE="lib${PACKAGE}.so"
    BENCH_LIBRARY_FILE="lib${BENCH_LIBRARY_BASENAME}.so"
else
    echo "Unknown platform: $(uname)" >&2
    exit 1
fi

cd "$SCRIPT_DIR"

export CARGO_TARGET_DIR="$SCRIPT_DIR/target"
export BOLTFFI_DISABLE_EXPORTS=1

cargo build --manifest-path "$DEMO_MANIFEST" --lib --release --features uniffi

rm -rf "$DIST_DIR"
mkdir -p "$DIST_DIR"

BINDGEN_CS_BIN="$(resolve_bindgen_cs)"

"$BINDGEN_CS_BIN" \
    --library \
    --no-format \
    --out-dir "$DIST_DIR" \
    "$SCRIPT_DIR/target/release/$LIBRARY_FILE"

cp "$SCRIPT_DIR/target/release/$LIBRARY_FILE" "$SCRIPT_DIR/target/release/$BENCH_LIBRARY_FILE"

perl -0pi -e 's/\[DllImport\("demo"/[DllImport("bench_uniffi"/g' \
    "$DIST_DIR/demo.cs"
