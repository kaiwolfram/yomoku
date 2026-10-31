#!/usr/bin/env bash
set -euo pipefail

repo_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_dir"

formatter="${KTFMT_JAR:-$repo_dir/.local/ktfmt-0.64.jar}"
java_bin="${JAVA_BIN:-${JAVA_HOME:+$JAVA_HOME/bin/}java}"
options=(--kotlinlang-style)
case "${1:-}" in
    '') ;;
    --check) options+=(--dry-run --set-exit-if-changed) ;;
    *) echo 'Usage: tools/format-kotlin.sh [--check]' >&2; exit 2 ;;
esac
if [[ $# -gt 1 ]]; then
    echo 'Usage: tools/format-kotlin.sh [--check]' >&2
    exit 2
fi
if [[ ! -f "$formatter" ]]; then
    echo 'Install ktfmt 0.64 as documented in docs/DEVELOPMENT.md or set KTFMT_JAR.' >&2
    exit 1
fi
mapfile -d '' -t sources < <(rg --files -0 android tests/shared -g '*.kt' -g '*.kts')
"$java_bin" -jar "$formatter" "${options[@]}" "${sources[@]}"
