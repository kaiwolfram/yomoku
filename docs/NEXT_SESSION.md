# Resume here
Updated: 2026-10-30. Native Vocabulary selection/bulk transport implemented.
Commit guidance: all commits need explanatory bodies; merges include provenance.

## Current slice
- Browser retains prefix/status/four sorts/count/100-row infinite scroll/Retry.
  Schema remains 0020; no new dependency.
- VocabularyState now retains optional sorted selection, selecting_all,
  confirmation (exact IDs + Delete or SetStatus/status/date) and bulk_error.
- ToggleVocabularySelection accepts loaded IDs; SelectAllVocabulary captures all
  matching IDs on FIFO and unions them with hidden selections. Query changes keep
  selection; Settings keeps ordinary selection; navigation clears selection.
- RequestVocabularyBulkAction freezes the prompt. Confirm must echo it exactly.
  RetryVocabularyBulkAction reuses IDs/status/date and never reruns matching.
  Failed Select All instead retains its query for RetryVocabularySelection.
- Prompts/failed captures/writes gate unrelated work. In-flight work cannot be
  cancelled. CancelVocabularyBulkAction dismisses prompt/error, keeps selection;
  CancelVocabularySelection exits selection or abandons a failed capture.
- Native-only operations/results/captured callbacks bypass shell codecs. Worker
  uses the existing atomic repository APIs. Success invalidates browser,
  statistics, Library distributions, reader words/tokens/effective identity.
- Public events/view use generated Kotlin codecs. Compose bulk controls are NOT
  connected yet; existing preview/instrumentation fixtures set selectingAll=false.
- Repository deletion preserves page time/provenance/live analysis, recalculates
  charts, never rescans old books, and old IDs cannot touch fresh generations.
- Pages assume unchanged data between reads: changed counts restart at zero;
  same-count external changes need reentry. All loaded rows and selected IDs
  cross view serialization; large-list/history costs remain unmeasured.

## Next tasks (ordered)
1. Add Compose row selection, global Select All, status/delete confirmation,
   explicit capture/write Retry and Cancel/Back using retained native commands.
   Keep exact command/date on Retry; do not recompute matching in the shell.
2. Verify unloaded matches, hidden selections, failed capture/write gates,
   Cancel/Back, phone/tablet recreation and browser/statistics/Library/reader
   refresh with production JNI/SQLite. Core retains state only within a session.
3. Profile repository/encoding/callback costs before optimizations; extend diverse
   many-book/peak-memory/actual Compose measurements. Reader limits remain in
   EFFECTIVE_READER_PERFORMANCE.md.
Nostr configuration stays empty; ask Q188 before forwarding.

## Verification / blockers
- PASS: 816 workspace Rust tests (17 opt-in ignored), strict Clippy, Rustfmt/diff.
  Eight new Rust cases cover selection/confirmation/capture and write Retry,
  1,205 unloaded matches, canonical/frozen IDs, stale callbacks, all statuses,
  SQLite later-row rollback/fresh generations/reopen and dependent view refresh.
- PASS: complete real-Full host Kotlin/JNI and generated codec exchanges;
  repeatable Kotlin generation; both-ABI debug/test APKs and Android lint.
- Commands/environment: docs/DEVELOPMENT.md, native bulk transport section.
  Keep native/BoltFFI generation sequential to host JNI execution.
- Logs: /tmp/yomoku-vocabulary-bulk-transport-{workspace,clippy,host,android}.log.
- New worker-refresh fixture initially used incorrect test field names; corrected
  before focused/full passes. Initial sandbox Gradle startup failed wildcard-IP
  discovery; approved retry passed. No unresolved failure/blocker.
- Device/Full-loader matrix/release/process-death/performance/physical arm64 not
  rerun. Ignored .local/statistics-ui-avd image (8 GiB) remains; emulator stopped.
