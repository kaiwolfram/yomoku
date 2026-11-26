#!/usr/bin/env bash
set -euo pipefail
shopt -s nullglob

if [[ $# -ne 1 ]]; then
    echo "Usage: $0 <release-keystore>" >&2
    exit 1
fi

repo_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
build_tools="${ANDROID_HOME:?Set ANDROID_HOME to the Android SDK directory}/build-tools/36.0.0"
unsigned_apks=("$repo_dir"/android/app/build/outputs/apk/release/yomoku-*-unsigned.apk)
if (( ${#unsigned_apks[@]} == 0 )); then
    echo 'Build the release APKs with :app:assembleRelease before signing.' >&2
    exit 1
fi

output_dir="$repo_dir/.local/apks"
mkdir -p -- "$output_dir"
staging_dir="$(mktemp -d "$output_dir/.signing.XXXXXX")"
trap 'rm -r -- "$staging_dir"' EXIT

for unsigned_apk in "${unsigned_apks[@]}"; do
    unsigned_name="${unsigned_apk##*/}"
    signed_apk="$staging_dir/${unsigned_name%-unsigned.apk}.apk"
    "$build_tools/zipalign" -c -P 16 4 "$unsigned_apk"
    "$build_tools/apksigner" sign --ks "$1" --out "$signed_apk" "$unsigned_apk"
    "$build_tools/apksigner" verify "$signed_apk"
done

mv -- "$staging_dir"/* "$output_dir/"
