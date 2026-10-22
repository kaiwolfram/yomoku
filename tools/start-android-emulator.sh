#!/usr/bin/env bash
set -euo pipefail

sdk_dir="${ANDROID_HOME:-${ANDROID_SDK_ROOT:-$HOME/Android/Sdk}}"
emulator_bin="$sdk_dir/emulator/emulator"
if [[ ! -x "$emulator_bin" ]]; then
    echo 'Install an AVD with android emulator create medium_phone and set ANDROID_HOME.' >&2
    exit 1
fi

# Emulator 37.2.12's default SwiftShader GLES renderer crashes on this host.
# Explicit ANGLE/software graphics boots successfully; AVD config alone does not.
# Keep this in the foreground so Ctrl-C stops it. No snapshots or data reset.
exec "$emulator_bin" -avd "${1:-medium_phone}" \
    -no-window -no-audio -no-snapshot -gpu swangle \
    -camera-back none -camera-front none -accel on -cores 2
