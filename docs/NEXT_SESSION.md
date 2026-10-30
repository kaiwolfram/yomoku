# Resume here
Updated: 2026-10-30. Retained native Vocabulary paging transport implemented.

## Current slice
- VocabularyQuery/Sort/Row/Page + Storage::vocabulary_page(query, offset):
  literal indexed surface prefix, five-status subsets/count, 100-row pages,
  newest/frequency/last-seen/alphabetical. No migration/dependency; schema 0020.
- ViewModel.vocabulary retains canonical query, optional count, loaded rows,
  next offset, loading/error. None count is unattempted; Some(0) is empty.
- Shell: LoadVocabulary, SetVocabularyQuery, LoadMoreVocabulary, RetryVocabulary.
  Existing native FIFO runs read-only repository calls without dictionary loads.
  Native operations/results/attempt callbacks have no shell codec.
- Exact failed-page Retry retains successful rows. Changed queries clear pages
  and coalesce behind one read. IDs/obsolete markers reject stale callbacks,
  including A -> B -> A and equal-query reentry. Generic callbacks cannot release it.
- Settings keeps state and defers coalesced reads; LoadVocabulary resumes on close.
  Unrelated busy work rejects browser input. Keep query choices on navigation;
  reentry/import/status/completion/effective edits invalidate pages.
- Changed cross-page counts restart at zero. Separate pages need an unchanged
  dataset; external changes preserving the count require reentry refresh.
  All loaded rows cross view serialization; large-list/history costs unmeasured.
- Compose browser, phone/tablet recreation tests and bulk commands remain open.
  Native/BoltFFI generation must stay sequential to host JNI execution.

## Next tasks (ordered)
1. Connect retained Vocabulary state to Compose prefix/status/sort/count controls,
   explicit Retry and infinite scroll. Lazy-load/resume when visible and idle;
   never auto-retry errors. Verify unloaded results, phone/tablet recreation,
   Settings/Back, filtering/paging and real reader/import mutation refresh.
2. Global matching Select All and confirmed bulk status/deletion; verify unloaded
   rows, immutable status events, chart recalculation and fresh New generations
   without rescanning existing documents.
3. Profile repository/encoding/callback costs before optimizations; extend diverse
   many-book/peak-memory/actual Compose measurements. Reader limits remain in
   EFFECTIVE_READER_PERFORMANCE.md.
Nostr configuration stays empty; ask Q188 before forwarding.

## Verification / blockers
- PASS: eight new core/three worker/one native protocol cases; 801 workspace Rust
  tests (17 opt-in ignored), strict Clippy, Rustfmt and diff checks.
- Commands (Rust PATH workaround in DEVELOPMENT.md):
  cargo test -p yomoku-core --test vocabulary_transport --all-features --locked --offline
  cargo test -p yomoku-runtime --test imports vocabulary --all-features --locked --offline
  cargo test --workspace --all-features --locked --offline --quiet
  cargo clippy --workspace --all-targets --all-features --locked --offline -- -D warnings
  cargo fmt --all -- --check
  git diff --check
- PASS: full real-Full Kotlin/JNI and Rust/Kotlin/Rust exact codecs, 100 handle
  lifecycles, bridge workspace build and identical repeated codegen.
- PASS: both-ABI native regeneration, debug/test APKs, strict Android lint and
  changed Kotlin formatting. Gradle needed local-socket escalation after sandbox
  startup failed; offline rerun passes. Commands in DEVELOPMENT.md.
- Logs: /tmp/yomoku-vocabulary-transport-{core,worker,workspace,clippy,jni}.log.
  Corrected initial fixture table name, unused mut and missing view initializer.
  No unresolved failure/blocker. Device instrumentation, separate Full-loader
  matrix, release/process-death/performance and physical arm64 were not rerun.
