# Resume here
Updated: 2026-10-30. Phase-5 statistics transport verified.

## Current slice
- History audit, lifetime totals, five-status/Known-form timelines, UTC timeframe
  buckets and typed worker/core/FFI transport complete. Schema remains 0020.
- ViewModel.statistics retains selected timeframe (LastYear default), desired
  StatisticsQuery, loading/error and optional StatisticsBuckets. LoadStatistics
  takes explicit UTC as-of; SetStatisticsTimeframe takes both; RetryStatistics
  repeats the exact failed cutoff. Identical lazy loads never replay failures.
- Native skipped request/result and captured attempt IDs share the existing FIFO.
  Rapid selections coalesce; stale successes/errors cannot replace current lines
  or release another owner. Settings retains state; hidden unattempted selections
  resume on a visible lazy load. Unrelated busy work rejects requests unchanged.
- Committed imports/status/completions/effective edits invalidate values. Returning
  from another destination invalidates for external mutations. Retain timeframe.
  Capture wall time once per entry/selection and request when visible/idle, rather
  than supplying a new cutoff on every render. Errors stay separate from documents.
- All four projections preserve absolute lifetime occurrence/time, five-status and
  Known-form values plus opening lifetime state. Daily/monthly/Monday-week bins,
  UTC rolling calendar months, clipped edges, gaps, inclusive as-of/future exclusion
  and epoch–9999 validation remain as documented in DATA_MODEL.md.
- One read transaction, no content/dictionary/frozen JSON/cache reads or writes.
  Original/effective rereads count, zero-word time survives, pending/legacy do not.
  Document deletion preserves history; vocabulary erasure recalculates without
  rescans. Exact unsigned values survive generated Kotlin ULong codecs.
- Production reader/Library/completion use effective partitions. Exact Retry/Cancel
  and final time drain remain verified; original B stays immutable. Imports return
  Library atomically. Performance limits: EFFECTIVE_READER_PERFORMANCE.md.
- Statistics tab is still a Compose placeholder. New Event/ViewModel wire requires
  same-build regeneration; native/BoltFFI generation must not overlap host JNI.

## Next tasks (ordered)
1. Render Statistics overview/drilldowns for lifetime cumulative occurrences, five
   statuses and Known surface/dictionary-form lines using retained transport.
   Connect all four timeframes, visible/idle loading and explicit error Retry.
2. Verify chart datasets/lifetime origins and updates after reading/status/import,
   navigation/recreation and all timeframes through phone/tablet Compose tests.
3. Profile optimized repository/encoding/callback costs before changes; extend
   diverse many-book/peak-memory/actual Compose measurements.
Nostr configuration stays empty; ask Q188 before forwarding.

## Verification / blockers
- PASS: 780 workspace Rust tests, 17 opt-in ignored; nine new core/three worker/one
  protocol cases; full real-Full Kotlin/JNI suite, exact-u64 fixture exchanges,
  repeat codegen, workspace build, strict Clippy/Rustfmt/new-Kotlin-format/diff.
- Corrected corrupt-fixture CHECK/error expectations and Kotlin safe-call warning;
  fixed/tested Settings-hidden query resumption. No unresolved failures/blockers.
- Commands (Rust PATH/JDK setup in DEVELOPMENT.md):
  cargo test --workspace --all-features --locked --offline --quiet
  cargo clippy --workspace --all-targets --all-features --locked --offline -- -D warnings
  tools/test-kotlin-bridge.sh
  cargo run -p yomoku-ffi --bin generate-kotlin --features facet_typegen --locked --offline -- target/generated/kotlin-repeat
  diff -r target/generated/kotlin target/generated/kotlin-repeat
  cargo build --workspace --locked --offline
  cargo fmt --all -- --check
  git diff --check
- Logs: /tmp/yomoku-statistics-transport-{workspace,jni,clippy,codegen,build}.log.
  Android APK/lint/device, separate Full-loader matrix, release/process-death,
  performance and physical-arm64 checks were not rerun. No migration/dependency.
