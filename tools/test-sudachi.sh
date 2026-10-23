#!/usr/bin/env bash
set -euo pipefail

repo_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_dir"
python3 tools/verify-sudachi-source.py
dictionary="${YOMOKU_SUDACHI_DICTIONARY:-$repo_dir/.local/sudachi/full-20261015/system_full.dic}"
if [[ ! -f "$dictionary" ]]; then
    echo 'Run python3 tools/prepare-sudachi.py first (or --archive PATH for offline setup).' >&2
    exit 1
fi
export YOMOKU_SUDACHI_DICTIONARY="$dictionary"
cargo test -p yomoku-analysis --test full_dictionary --locked --offline -- --ignored
