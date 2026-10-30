# Resume here
Updated: 2026-10-30. Compose Vocabulary selection/bulk actions implemented.
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
- Compose row tap/long-press toggles selection with checkbox/selected semantics.
  Active selection count/actions stay above the scrolling browser; global Select
  All also works before selection. All five status choices request confirmation.
  Shell confirmation echoes the retained command; Retry creates no new date/IDs.
- Modal capture/write progress/errors pause paging and gate query/navigation/
  Settings. Back/Cancel preserves selection for prompts, exits ordinary selection
  or abandons a failed capture. Pending work cannot be cancelled. Native selection,
  prompts/errors and ordinary Settings selection survive Activity recreation.
- Repository deletion preserves page time/provenance/live analysis, recalculates
  charts, never rescans old books, and old IDs cannot touch fresh generations.
- Pages assume unchanged data between reads: changed counts restart at zero;
  same-count external changes need reentry. All loaded rows and selected IDs
  cross view serialization; large-list/history costs remain unmeasured.

## Next tasks (ordered)
1. Profile repository/encoding/callback costs before optimizations; extend diverse
   many-book/peak-memory/actual Compose measurements. Reader limits remain in
   EFFECTIVE_READER_PERFORMANCE.md; Vocabulary selection/history costs are open.
2. Verify supported AnkiDroid provider/deck/model/duplicate APIs against primary
   documentation; record feasible exact-surface duplicate behavior before wiring.
3. Extend the remaining accessibility/process-death/lifecycle matrix. Physical
   arm64 remains user-deferred; emulator measurements do not establish viability.
Nostr configuration stays empty; ask Q188 before forwarding.

## Verification / blockers
- PASS: debug/test APKs (existing both-ABI native libraries), strict Android lint,
  changed Kotlin formatting and diff checks. No Rust/native/schema/dependency change.
- PASS: 16 focused API-36 x86_64 cases on phone/light (1080x2400, 420 dpi) and
  landscape tablet/dark (2560x1600, 240 dpi), 32 final executions.
  Five new cases cover all statuses, exact prompt/date/Retry, 205 unloaded matches,
  hidden selections, capture/update/delete failures and rollback, later-match
  exclusion, Settings/Back/recreation and refreshed charts/Library/effective reader.
- Commands/environment: docs/DEVELOPMENT.md, Compose Vocabulary selection section.
  Explicitly reset display/density/night overrides before the phone run.
- Logs: /tmp/yomoku-vocabulary-bulk-ui-{android,phone-light,tablet}.log.
- Initial Back assertion raced selection recomposition; visible-count assertion
  fixes synchronization. Earlier 16-case pass used inherited 240-dpi/dark settings;
  explicit phone/light rerun passes. Sandbox Gradle needed approved socket access.
- Tablet ADB wrapper returned 143 after complete OK output; device TestRunner
  confirms 16 tests/0 failed/0 ignored and AndroidRuntime exit 0. No open blocker.
- Rust/Clippy/host JNI/codegen/Full-loader matrix/release/process-death/performance/
  physical arm64 not rerun. Keep generation sequential to host JNI execution.
  Ignored .local/statistics-ui-avd image (8 GiB) remains; display restored/emulator stopped.
