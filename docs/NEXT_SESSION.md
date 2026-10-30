# Resume here
Updated: 2026-10-30. Phase-5 timeframe buckets verified.

## Current slice
- History audit, lifetime totals, status/Known-form timelines and timeframe
  buckets complete. Storage schema remains 0020. DATA_MODEL.md documents contracts.
- Pure statistics::statistics_buckets and Storage::statistics_buckets return
  absolute occurrence/time, five-status and Known-form values plus explicit
  opening lifetime state. Last month daily; six months/year weekly (year default);
  all time monthly. UTC rolling calendar months clamp month-end days, weeks start
  Monday, first/last intervals clip to the range, gaps carry values. Include the
  supplied as-of instant; exclude future events. Never reset cumulative y-values.
  All time starts at the earliest visible event month (current month when empty).
  Calendar dates support epoch through year 9999; storage timestamps unchanged.
- One deferred read transaction gives all lines one snapshot. Reading aggregates
  actual surviving analyzed occurrences and time once per command, including
  zero-word pages. No cache, dictionary, live text or frozen JSON loads/writes.
  Original/effective rereads count; duplicates/pending/unused/legacy work do not.
  Query-only works; corrupt versions/dates/durations/overflow fail without repair.
- Document deletion preserves history/links; vocabulary erasure recalculates
  occurrences/statuses/forms, retaining completed time. New IDs start New without
  rescanning. Undated exact-spelling dictionary links apply throughout their
  generation's history. Schema-5–20 upgrade/history audits remain covered.
- Production reader/Library/completion use native effective A/B/C/manual partitions;
  original B stays immutable. Edits expose Reload; exact Retry/Cancel survives
  dismissal. Partial time saves every 15s with five-minute idle/background cap,
  draining before movement/exit/close. Process death can lose in-memory work.
- Atomic/deduplicated imports return Library without opening. Reimport starts fresh.
  Android primitive-backed FileImportBytes fixes the real 17.9 MB EPUB heap issue.
  Debug JNI measurements/CSV: EFFECTIVE_READER_PERFORMANCE.md. Long-page regressions
  pass; optimized/frame-time/peak-memory/physical-arm64 remain unverified.
- Statistics transport/charts remain unimplemented; tab is still a placeholder.

## Next tasks (ordered)
1. Add typed statistics transport through worker/core/FFI: native bucket request,
   timeframe/as-of input and retained loading/error/Retry state. Test exact results,
   default year, stale callbacks and mutation refresh before Compose wiring.
2. Render overview/drilldown charts for lifetime cumulative occurrences, five
   statuses and Known surface/dictionary-form lines in all four timeframes.
3. Profile optimized Android repository/encoding/callback costs before changes;
   extend diverse many-book/peak-memory/actual Compose measurements.
Nostr configuration stays empty; ask Q188 before forwarding.

## Verification / blockers
- PASS: eight new core/seven SQLite tests; 767 workspace Rust tests, 17 opt-in
  ignored; workspace build, strict Clippy/Rustfmt/diff. No unresolved failures or
  implementation blockers. Initial exhaustive-error match and resolved-form test
  fixture errors were corrected before final verification.
- Commands (Rust PATH workaround from DEVELOPMENT.md):
  cargo test -p yomoku-core --test statistics_buckets --all-features --locked --offline
  cargo test -p yomoku-storage statistics --locked --offline
  cargo test --workspace --all-features --locked --offline --quiet
  cargo clippy --workspace --all-targets --all-features --locked --offline -- -D warnings
  cargo build --workspace --locked --offline
  cargo fmt --all -- --check
  git diff --check
- Logs: /tmp/yomoku-statistics-buckets-{storage,workspace,clippy,build}.log.
  No dependency/migration/wire/Kotlin change. Android/JNI/codegen/Full-loader/
  release/process-death/physical-arm64 were not rerun. Native/BoltFFI generation
  must remain sequential with host JNI execution.
