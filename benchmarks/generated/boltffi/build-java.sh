#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$SCRIPT_DIR/../../.."
DEMO_DIR="$ROOT_DIR/examples/demo"
BENCH_OVERLAY="$DEMO_DIR/boltffi.benchmark.toml"

export CARGO_TARGET_DIR="$SCRIPT_DIR/target"

resolve_jdk_home() {
    if [[ -n "${JAVA_HOME:-}" && -f "${JAVA_HOME}/include/jni.h" && -f "${JAVA_HOME}/include/darwin/jni_md.h" ]]; then
        printf '%s\n' "$JAVA_HOME"
        return 0
    fi

    if [[ -n "${JAVA_HOME:-}" && -f "${JAVA_HOME}/libexec/openjdk.jdk/Contents/Home/include/jni.h" && -f "${JAVA_HOME}/libexec/openjdk.jdk/Contents/Home/include/darwin/jni_md.h" ]]; then
        printf '%s\n' "${JAVA_HOME}/libexec/openjdk.jdk/Contents/Home"
        return 0
    fi

    if [[ "$(uname)" == "Darwin" ]]; then
        local detected_java_home
        detected_java_home="$(/usr/libexec/java_home 2>/dev/null || true)"
        if [[ -n "$detected_java_home" && -f "${detected_java_home}/include/jni.h" && -f "${detected_java_home}/include/darwin/jni_md.h" ]]; then
            printf '%s\n' "$detected_java_home"
            return 0
        fi
    fi

    return 1
}

HOST_TRIPLE="$(rustc -Vv | awk '/^host:/ { print $2 }')"
HOST_JAVA_ENV_SUFFIX="$(printf '%s' "$HOST_TRIPLE" | tr '[:lower:]-' '[:upper:]_')"

if resolved_jdk_home="$(resolve_jdk_home)"; then
    export JAVA_HOME="$resolved_jdk_home"
    export "BOLTFFI_JAVA_HOME_${HOST_JAVA_ENV_SUFFIX}=$resolved_jdk_home"
fi

cd "$DEMO_DIR"

cargo build --release -p boltffi_cli --manifest-path "$ROOT_DIR/Cargo.toml"

"$SCRIPT_DIR/target/release/boltffi" \
    --overlay "$BENCH_OVERLAY" \
    pack java \
    --release \
    --regenerate
