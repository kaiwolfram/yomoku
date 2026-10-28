#!/usr/bin/env bash
set -euo pipefail

repo_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_dir"
export YOMOKU_SUDACHI_DICTIONARY="${YOMOKU_SUDACHI_DICTIONARY:-$repo_dir/.local/sudachi/full-20261015/system_full.dic}"
if [[ ! -f "$YOMOKU_SUDACHI_DICTIONARY" ]]; then
    echo 'Prepare Sudachi Full first; production JNI imports require the real dictionary.' >&2
    exit 1
fi

export YOMOKU_WORD_DICTIONARY="${YOMOKU_WORD_DICTIONARY:-$repo_dir/.local/dictionaries/index-v1-20261025/dictionary.sqlite3}"
if [[ ! -f "$YOMOKU_WORD_DICTIONARY" ]]; then
    echo 'Prepare the pinned word/name index first; selected-word JNI fixtures require it.' >&2
    exit 1
fi

boltffi_bin="${BOLTFFI_BIN:-$repo_dir/.local/bin/boltffi}"
kotlinc_bin="${KOTLINC_BIN:-$repo_dir/.local/kotlin-2.4.21/kotlinc/bin/kotlinc}"
java_bin="${JAVA_BIN:-java}"
jni_include_dir="${JNI_INCLUDE_DIR:-${JAVA_HOME:-$repo_dir/.local/jni-25}/include}"

for tool in "$boltffi_bin" "$kotlinc_bin" "$java_bin" "${CC:-cc}" cargo; do
    command -v "$tool" >/dev/null || { echo "Missing tool: $tool" >&2; exit 1; }
done
if [[ "$("$boltffi_bin" --version)" != "boltffi 0.30.1" ]]; then
    echo 'BoltFFI CLI 0.30.1 is required to match the Rust library.' >&2
    exit 1
fi
if [[ ! -f "$jni_include_dir/jni.h" || ! -f "$jni_include_dir/linux/jni_md.h" ]]; then
    echo 'Set JAVA_HOME to a Linux JDK or JNI_INCLUDE_DIR to its include directory.' >&2
    exit 1
fi

cargo run -p yomoku-ffi --bin generate-kotlin --features facet_typegen --locked --offline -- target/generated/kotlin
(
    cd crates/yomoku-ffi
    CARGO_NET_OFFLINE=true "$boltffi_bin" generate kotlin --deny-skipped
)
# BoltFFI's metadata expansion replaces the host library artifact. Rebuild its
# native exports after generation before linking JNI against it.
cargo build --workspace --locked --offline
cargo run -p yomoku-ffi --example unicode-fixtures --locked --offline -- write target/unicode-fixtures
"${CC:-cc}" -std=c11 -shared -fPIC -O2 -Wall -Wextra -Werror \
    -I"$jni_include_dir" -I"$jni_include_dir/linux" \
    -Itarget/generated/native/jni target/generated/native/jni/jni_glue.c \
    -Ltarget/debug -lyomoku_ffi '-Wl,-rpath,$ORIGIN/debug' -Wl,-z,defs \
    -o target/libyomoku_jni.so
"$kotlinc_bin" target/generated/kotlin/app target/generated/kotlin/com \
    target/generated/native/app tests/kotlin \
    android/app/src/main/java/app/yomoku/ReaderTokenRanges.kt \
    -jvm-target 17 -Werror -include-runtime -d target/bridge-tests.jar
"$java_bin" --enable-native-access=ALL-UNNAMED -Xcheck:jni \
    --add-opens=java.base/java.io=ALL-UNNAMED \
    --add-opens=java.base/sun.nio.fs=ALL-UNNAMED \
    -Djava.library.path=target -jar target/bridge-tests.jar target/unicode-fixtures
cargo run -p yomoku-ffi --example unicode-fixtures --locked --offline -- verify target/unicode-fixtures
