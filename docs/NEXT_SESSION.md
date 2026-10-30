# Resume here
Updated: 2026-10-30. Vocabulary Compose browser implemented and verified.
Commit guidance updated: all commits need explanatory bodies; merges include provenance.

## Current slice
- Vocabulary has literal surface-prefix input, five independent status filters,
  matching count, newest/frequency/last-seen/alphabetical and infinite scroll.
  Native retained state/repository supplies 100-row pages; schema remains 0020.
- Compose requests an unattempted query only while visible/idle. Last loaded row
  visibility triggers the next page. Errors require explicit Retry; successful
  rows survive append failures. Controls coalesce during Vocabulary reads and
  disable during unrelated storage work. No dictionary lookup or row actions.
- Query/pages/error stay in Rust. Session/query-keyed shell list state preserves
  scroll through Settings/recreation and resets on changed queries. Native
  reentry/import/status/completion/effective-edit invalidation remains in force.
- Read-only repository uses literal indexed prefixes, all status subsets and
  count/page snapshots. Separate pages assume an unchanged dataset: count changes
  restart at zero; same-count external changes need reentry refresh.
- All loaded rows cross view serialization; large-list/history costs unmeasured.
  Native/BoltFFI generation must remain sequential to host JNI execution.

## Next tasks (ordered)
1. Add global matching bulk status/deletion repository commands; verify unloaded
   matches, immutable status events, atomic rollback, history/chart recalculation
   and fresh New generations on later appearance without rescanning live books.
2. Wire retained selection/Select All and exact confirmed bulk operations through
   core/FIFO/FFI/Compose; verify Cancel/Back/Retry, phone/tablet recreation and
   refreshed browser/statistics/Library/reader state.
3. Profile repository/encoding/callback costs before optimizations; extend diverse
   many-book/peak-memory/actual Compose measurements. Reader limits remain in
   EFFECTIVE_READER_PERFORMANCE.md.
Nostr configuration stays empty; ask Q188 before forwarding.

## Verification / blockers
- PASS: debug/test APKs, strict Android lint, changed Kotlin formatting and diff.
- PASS: five new Compose/native plus six navigation cases on API-36 x86_64
  phone/light (1080x2400, 420 dpi; 114.611 s) and tablet/dark (2560x1600,
  240 dpi; 82.566 s), 22 executions without failures or skips.
- Covers 205-row paging/unloaded matches, filters/sorts/counts, exact u64 labels,
  initial/append explicit Retry, read failure/repair, busy gates, actual Full
  import/status/completion refresh, Settings/Back and session/scroll recreation.
- Commands (environment setup in DEVELOPMENT.md):
  ./android/gradlew -p android :app:assembleDebug :app:assembleDebugAndroidTest :app:lintDebug --offline --console=plain
  adb -s emulator-5554 install -r -t android/app/build/outputs/apk/debug/app-debug.apk
  adb -s emulator-5554 install -r -t android/app/build/outputs/apk/androidTest/debug/app-debug-androidTest.apk
  adb -s emulator-5554 shell am instrument -w -r -e class app.yomoku.VocabularyScreenTest,app.yomoku.VocabularyProductionTest,app.yomoku.NavigationTest,app.yomoku.EffectiveReaderNavigationTest app.yomoku.test/androidx.test.runner.AndroidJUnitRunner
- Reuse ignored .local/statistics-ui-avd isolated image (8 GiB partition); startup
  command in DEVELOPMENT.md. Display size/density/light restored; emulator stopped.
- Fixed initial tests' stale reopened-reader selection and offscreen Retry
  assumption. Sandbox Gradle/ADB socket failures recovered with escalation;
  initial install retried after boot. No unresolved failure/blocker.
- Logs: /tmp/yomoku-vocabulary-ui-{android,phone,phone-initial,tablet,emulator}.log;
  visual screenshots /tmp/yomoku-vocabulary-ui-{phone,tablet}.png.
- Rust/native sources unchanged; Rust tests/Clippy, host JNI/codegen, separate
  Full-loader matrix, release/process-death/performance and physical arm64 not
  rerun. Native generation was up-to-date; existing both-ABI libraries packaged.
