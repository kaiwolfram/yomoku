#!/usr/bin/env bash
set -euo pipefail

repo_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_dir"

boltffi_bin="${BOLTFFI_BIN:-$repo_dir/.local/bin/boltffi}"
for tool in "$boltffi_bin" cargo rustc; do
    command -v "$tool" >/dev/null || { echo "Missing tool: $tool" >&2; exit 1; }
done
if [[ "$("$boltffi_bin" --version)" != "boltffi 0.30.1" ]]; then
    echo 'BoltFFI CLI 0.30.1 is required to match the Rust library.' >&2
    exit 1
fi

# Keep the NDK pin authoritative in boltffi.toml. BoltFFI otherwise falls back
# to another installed version when the requested version is missing.
ndk_version="$(sed -n 's/^ndk_version = "\([^"]*\)"$/\1/p' crates/yomoku-ffi/boltffi.toml)"
sdk_dir="${ANDROID_HOME:-${ANDROID_SDK_ROOT:-}}"
export ANDROID_NDK_HOME="${ANDROID_NDK_HOME:-$sdk_dir/ndk/$ndk_version}"
if [[ -z "$ndk_version" || ! -f "$ANDROID_NDK_HOME/source.properties" ]] ||
    ! rg -q "^Pkg.Revision = ${ndk_version//./\\.}$" "$ANDROID_NDK_HOME/source.properties"; then
    echo "Install NDK $ndk_version and set ANDROID_HOME or ANDROID_NDK_HOME." >&2
    exit 1
fi
for target in aarch64-linux-android x86_64-linux-android; do
    if [[ ! -d "$(rustc --print target-libdir --target "$target")" ]]; then
        echo "Install Rust target: rustup target add $target" >&2
        exit 1
    fi
done

# Generation and native builds share Cargo output; do not run another bridge
# generator/build concurrently. This command never downloads dependencies.
export CARGO_NET_OFFLINE=true
cargo run -p yomoku-ffi --bin generate-kotlin --features facet_typegen \
    --locked --offline -- target/generated/kotlin
(
    cd crates/yomoku-ffi
    "$boltffi_bin" pack android --deny-skipped --cargo-arg=--locked --cargo-arg=--offline
)
