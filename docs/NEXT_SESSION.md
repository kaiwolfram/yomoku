# Resume here
Updated: 2026-10-30. Vocabulary bulk repository implemented and verified.
Commit guidance: all commits need explanatory bodies; merges include provenance.

## Current slice
- Vocabulary Compose browser has literal prefix/status filters/count/four sorts
  and retained 100-row infinite scroll with explicit Retry. Schema remains 0020.
- New Storage::vocabulary_matching_ids(query) captures every matching ID in one
  read statement, sharing the browser predicate; ascending IDs, no page limit.
- New set_vocabulary_statuses(ids, status, date) and delete_vocabulary_entries(ids)
  apply frozen selections in one immediate transaction. Actual status transitions
  append manual events; deletion cascades events/occurrences/form links while
  preserving page time/provenance/live analysis. Missing/duplicate IDs are harmless.
- Confirmations/Retry must freeze IDs/status/date and never rerun matching.
  Old IDs cannot affect fresh generations. These repository APIs are not yet
  routed through core/FIFO/FFI/Compose; no new protocol or dependency.
- Deletion recalculates all chart timeframes and does not recreate words when
  live readers/tokens/Library reload. Later analyzed imports register fresh New
  generations; successful original/effective completion replay cannot resurrect.
- Existing browser pages assume unchanged data between reads: changed counts
  restart at zero; same-count external changes need reentry refresh. All loaded
  rows cross view serialization; large-list/history costs remain unmeasured.

## Next tasks (ordered)
1. Add retained selection/Select All and exact confirmed bulk commands through
   core/FIFO/FFI using the new repository APIs. Freeze IDs before confirmation;
   keep failed IDs/status/date for explicit Retry and gate unrelated work.
2. Add Compose selection/confirmation controls; verify Cancel/Back/Retry, unloaded
   matches, phone/tablet recreation and browser/statistics/Library/reader refresh.
3. Profile repository/encoding/callback costs before optimizations; extend diverse
   many-book/peak-memory/actual Compose measurements. Reader limits remain in
   EFFECTIVE_READER_PERFORMANCE.md.
Nostr configuration stays empty; ask Q188 before forwarding.

## Verification / blockers
- PASS: seven new SQLite cases, 808 workspace Rust tests (17 opt-in ignored),
  strict Clippy, workspace build, Rustfmt and diff checks.
- Covers 1,205 matches/all filter subsets/sorts/literal Unicode/NUL prefixes,
  read-only unchanged bytes, immutable actual-transition events/all statuses,
  concurrent/no-op/frozen-ID retries, later-row/cascade/deferred-COMMIT rollback,
  reopen, history/chart recalculation, retained time/live scans and fresh IDs.
- Commands (Rust PATH/environment setup in DEVELOPMENT.md):
  cargo test -p yomoku-storage vocabulary_bulk --locked --offline
  cargo test --workspace --all-features --locked --offline --quiet
  cargo clippy --workspace --all-targets --all-features --locked --offline -- -D warnings
  cargo build --workspace --locked --offline
  cargo fmt --all -- --check
  git diff --check
- Initial new-test API assumptions corrected (mutable snapshot read, counts,
  Recorded payload); initial Clippy clone-to-slice warning corrected. Final
  focused tests/Clippy pass; no unresolved failure/blocker.
- Logs: /tmp/yomoku-vocabulary-bulk-{storage,workspace,clippy,build}.log.
- Android/JNI/codegen/Full-loader matrix/release/process-death/performance and
  physical arm64 not rerun for this storage-only slice. Keep native/BoltFFI
  generation sequential to host JNI execution.
- Last UI baseline: 22 phone/light + tablet/dark browser/navigation executions
  passed; ignored .local/statistics-ui-avd image (8 GiB) remains available.
  Startup/device commands are in DEVELOPMENT.md; emulator is stopped.
