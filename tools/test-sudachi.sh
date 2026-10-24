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
# Keep the prepared dictionary immutable until both processes exit. Never run
# preparation or replace/truncate these bytes concurrently with mapped tests.
for storage in owned mapped; do
    echo "Testing Full storage: $storage"
    YOMOKU_SUDACHI_STORAGE="$storage" cargo test -p yomoku-analysis --test full_dictionary --locked --offline -- --ignored
    YOMOKU_SUDACHI_STORAGE="$storage" cargo test -p yomoku-runtime --test imports --locked --offline -- --ignored
done
