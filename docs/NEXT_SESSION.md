# Resume here
Updated: 2026-10-30. Native read-only Vocabulary browser repository implemented.

## Current slice
- Pure VocabularyQuery/Sort/Row/Page values plus Storage::vocabulary_page(query,
  offset): literal indexed surface prefix, five-status subsets, matching count,
  100-row offset pages and newest/frequency/last-seen/alphabetical sorting.
- Count and rows share one deferred read snapshot. Existing BINARY surface and
  occurrence indexes suffice; no migration/dependency. Schema remains 0020.
- Frequency counts actual completed-reading occurrences, including repeats and
  original/effective rereads; last seen is maximum recorded completion date.
  Newest uses registration date then ID descending; metric ties share that order.
  Alphabetical uses exact BINARY spelling; unread dates sort last.
- Empty prefix matches all; default all statuses, explicit empty set none.
  Preserve Unicode/case/whitespace/NUL/wildcards literally, no reading/lemma match.
  Document deletion retains history; erased entries stay absent without rescans.
- Pages are stable only for an unchanged query/dataset. Future transport must
  restart after relevant mutations and protect stale callbacks. Frequency/date
  sorts can examine all matching histories; large-history costs are unmeasured.
- Vocabulary worker/core/FFI state and Compose browser remain unimplemented.
  Statistics UI/transport and effective reader remain as previously verified.
  Native/BoltFFI generation must stay sequential to host JNI execution.

## Next tasks (ordered)
1. Vocabulary typed FIFO worker/core/FFI transport: retained prefix/status/sort,
   lazy 100-row append, matching count, explicit exact Retry, query coalescing,
   stale-request rejection, mutation invalidation and Settings/navigation retention.
   Then connect Compose infinite scroll and verify phone/tablet recreation.
2. Global matching Select All and confirmed bulk status/deletion; verify unloaded
   rows, immutable status events, chart recalculation and fresh New generations
   without rescanning existing documents.
3. Profile repository/encoding/callback costs before optimizations; extend diverse
   many-book/peak-memory/actual Compose measurements. Reader limits remain in
   EFFECTIVE_READER_PERFORMANCE.md.
Nostr configuration stays empty; ask Q188 before forwarding.

## Verification / blockers
- PASS: nine new SQLite browser cases; full workspace Rust tests, strict Clippy,
  workspace build, Rustfmt and git diff check (see DEVELOPMENT.md for details).
- Tests cover exact prefixes/query plans, all 32 filter subsets, stable pages,
  all sorts/ties, original/effective repeats/rereads, read-only/reopen/deletion,
  pending/failed/uncommitted isolation and error cleanup without repairs.
- Commands (use Rust PATH workaround from DEVELOPMENT.md):
  cargo test -p yomoku-storage vocabulary_browser --locked --offline
  cargo test --workspace --all-features --locked --offline --quiet
  cargo clippy --workspace --all-targets --all-features --locked --offline -- -D warnings
  cargo build --workspace --locked --offline
  cargo fmt --all -- --check
  git diff --check
- Logs: /tmp/yomoku-vocabulary-browser-{storage,workspace,clippy,build}.log.
  Initial test compile lacked a VocabularyId import; corrected and rerun.
  No unresolved failure or blocker. Android/APK/lint/device, host JNI/codegen,
  separate Full-loader matrix, release/process-death/performance and physical
  arm64 were not rerun; no existing wire/Android source changed.
