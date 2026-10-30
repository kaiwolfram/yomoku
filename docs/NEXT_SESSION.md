# Resume here
Updated: 2026-10-30. Phase-5 Statistics Compose overview/drilldowns implemented.

## Current slice
- Statistics now renders cumulative lifetime Japanese occurrences, five absolute
  status lines and Known surface/dictionary-form lines. Tappable overview cards
  open charts plus exact lifetime opening/interval-end rows. Android Back returns
  to overview; session-keyed drilldown survives Settings/navigation/recreation.
- All four timeframe chips use retained core state (LastYear default), explicit
  UTC grouping/ranges and visible/idle lazy loading. Capture now once for first/
  invalidated queries or explicit selections; reuse retained cutoff on renders,
  Settings/recreation and errors. Only RetryStatistics retries a failed query.
- StatisticsDataset retains exact ULongs, actual elapsed x spacing and opening
  lifetime values. Step lines map interval-end samples. Subtract unsigned minimum
  before floating-point drawing conversion; constant/empty lines stay finite.
  Reuse status palette/Known green; dictionary-form line is dashed. Accessible
  summaries/legends/rows retain exact counts. No new library or migration.
- Native FIFO/core/FFI transport and read-only SQLite projections remain unchanged:
  rapid selections coalesce, stale callbacks cannot replace new lines, imports/
  status/completions/effective edits/tab reentry invalidate values, document
  deletion retains history and vocabulary erasure recalculates without rescans.
- Updated preview/three existing reader fixtures for the Statistics wire field
  added in the previous slice. Current Android builds regenerate both native ABIs
  and Kotlin together. Keep native/BoltFFI generation sequential to host JNI.
- Production reader/Library/completion use effective partitions. Exact Retry/Cancel
  and final-time drain remain verified; original B stays immutable. Performance
  limits: EFFECTIVE_READER_PERFORMANCE.md. Schema remains 0020.

## Next tasks (ordered)
1. Vocabulary browser: indexed surface-prefix query, five-status filters, matching
   count, 100-row paging and newest/frequency/last-seen/alphabetical sorting;
   implement/test typed worker/core/FFI transport before Compose integration.
2. Global matching Select All and confirmed bulk status/deletion; verify unloaded
   rows, immutable status events, history erasure/chart refresh and fresh New
   generations without rescanning existing documents.
3. Profile optimized repository/encoding/callback costs before changes; extend
   diverse many-book/peak-memory/actual Compose measurements.
Nostr configuration stays empty; ask Q188 before forwarding.

## Verification / blockers
- PASS: both-ABI debug/test APK build, strict Android lint, all changed Kotlin
  formatting and git diff check. Thirteen dataset/Compose/production/navigation
  cases pass on both phone/light and tablet/dark (26 executions), no skips.
- Final build: /tmp/yomoku-statistics-ui-android-final.log; phone/tablet logs:
  /tmp/yomoku-statistics-ui-{phone,tablet}.log. No source verification failures.
- Commands (environment setup in DEVELOPMENT.md):
  ./android/gradlew -p android :app:assembleDebug :app:assembleDebugAndroidTest :app:lintDebug --offline --console=plain
  adb -s emulator-5554 shell am instrument -w -r -e class app.yomoku.StatisticsDatasetTest,app.yomoku.StatisticsScreenTest,app.yomoku.StatisticsProductionTest,app.yomoku.NavigationTest,app.yomoku.EffectiveReaderNavigationTest app.yomoku.test/androidx.test.runner.AndroidJUnitRunner
  git diff --check
- Existing emulator lacked install space. Restored its temporary storage reserve
  adjustment, preserved Library data and used .local/statistics-ui-avd instead.
  A /tmp data-image attempt lacked host space; workspace image boots successfully. No unresolved blocker.
- Rust/host JNI/codegen reproducibility/Full-loader matrix, release/process-death,
  performance and physical arm64 were not rerun for this Kotlin-only slice.
