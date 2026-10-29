# Resume here
Updated: 2026-10-29. Phase-5 history audit and lifetime aggregation verified.

## Current slice
- First two phase-5 tasks complete. Schema 0019 retains v1 completion/status/analysis
  headers and independent v1 effective reservations. Occurrences inherit the analyzed
  contract; reservation presence selects effective indices. No history-edit API.
- Every history-bearing schema (5–19) upgrades with exact history/ID high-water marks
  and joinable deleted source metadata. Late failures from 9/10/13/17/18 roll back
  byte-identically. Real completed/unused effective payloads survive upgrade/deletion;
  unused reservations never replay. DATA_MODEL.md documents the audited contract.
- Pure core statistics::LifetimeReadingTotals and Storage::lifetime_reading_totals
  derive both totals in one read-only SQLite statement. Count surviving analyzed
  occurrences; sum time once per analyzed command, including zero-Japanese pages.
  Repetitions/rereads count. Pending/import/reservation/legacy work contributes zero.
  Document deletion keeps both; vocabulary cascades reduce occurrences, retain time.
  Versions/corrupt durations/overflow fail without repair. No cache/dictionary load.
  Statistics transport/timelines/charts remain unimplemented; tab is a placeholder.
- Production reader/Library/completion use native effective A/B/C/manual partitions;
  original B stays immutable. Edits expose Reload; exact Retry/Cancel survives dismissal.
  Partial time saves every 15s with a five-minute idle/background cap and drains
  before movement/exit/close. Process death can lose pending in-memory work.
- Atomic/deduplicated imports return Library without opening. Reimport starts fresh.
  Android primitive-backed FileImportBytes fixes the real 17.9 MB EPUB boxed-list heap
  exhaustion. Debug JNI measurements/CSV: EFFECTIVE_READER_PERFORMANCE.md. Long-page
  regressions pass; optimized/frame-time/peak-memory/physical-arm64 remain unverified.

## Next tasks (ordered)
1. Add status timeline aggregation: registration jumps, transitions/deletion and
   Known surface/dictionary-form counts (any Known linked surface, spelling identity,
   unresolved excluded). Start with core statistics, storage vocabulary/analysis/
   imports. Durable global dictionary-form links remain absent: preserve links for
   registered-but-unread surfaces even after document deletion before claiming counts.
2. Add timeframe buckets (month daily, six months/year weekly, all time monthly),
   date-boundary coverage and lifetime cumulative origin; then typed UI transport.
3. Profile optimized Android repository/encoding/callback costs before changes;
   extend diverse many-book/peak-memory/actual Compose measurements.
Nostr configuration stays empty; ask Q188 before forwarding.

## Verification / blockers
- PASS: seven aggregation tests; 720 workspace Rust tests, 17 opt-in ignored;
  workspace build, strict Clippy/Rustfmt/diff. No failures or implementation blocker.
- Commands (Rust PATH workaround from DEVELOPMENT.md):
  cargo test -p yomoku-storage statistics --locked --offline
  cargo test --workspace --all-features --locked --offline --quiet
  cargo clippy --workspace --all-targets --all-features --locked --offline -- -D warnings
  cargo build --workspace --locked --offline
  cargo fmt --all -- --check
  git diff --check
- Logs: /tmp/yomoku-history-schema-*.log and /tmp/yomoku-lifetime-statistics-*.log.
  No migration/dependency/FFI/Kotlin change; Android/JNI/codegen/Full-loader/release/
  process-death/physical-arm64 were not rerun. Native/BoltFFI generation must remain
  sequential with host JNI execution.
