# Resume here
Updated: 2026-10-30. Vocabulary Android Compose/sampled-memory profiling complete.
Commit guidance: explanatory bodies; merges include provenance. Keep verification
results in session/progress docs and reports, out of commit message bodies.

## Current slice
- Vocabulary browser/selection/bulk Compose integration remains complete. Frozen
  IDs/status/date, explicit Retry/Cancel and invalidation are documented in
  ARCHITECTURE.md/DATA_MODEL.md and prior SESSION_LOG.md entries.
- New opt-in VocabularyPerformanceTest renders production YomokuApp/CoreSession
  with a disposable migrated/seeded SQLite DB. Normal app DB is never opened by
  the harness. Exact rows/selection/confirmation, no analyzer, unchanged bytes,
  no reading effects, integrity/FK/native closure/scratch cleanup are guarded.
- Separate debug API-36 x86_64 phone/light processes: 10k/100k entries, 1,000
  retained rows, one scenario warmup then five samples; actual main-window frames
  and ~25 ms sampled Java/native heap/RSS. Timing includes instrumentation/idle
  and observers; Dialog frames excluded. Sampled maxima are not exact peaks/PSS.
- At 100k, Select All median action-to-idle 865.486 ms (732.723–1,049.362),
  synchronous dispatch 16.955 ms. Confirmation 321.554/87.446 ms; view ~1.65 MB.
  Highest measured-action samples: Java 43.619/native 28.574/RSS 278.637 MiB,
  at different times; never sum these. No claim about one responsible layer.
- Host heavy-history profiler remains measure_vocabulary (production SQL/private
  decoders); bounded history projections and borrowed row text are implemented.
  Deep metric offsets and required decoding/replay remain measurable costs.
- No production/schema/dependency/wire/architecture/destructive change. Schema
  0020. No Nostr starter data; configuration empty. Ask Q188 before forwarding.

## Next tasks (ordered)
1. Profile/fix large Vocabulary Select All main callback decoding/composition;
   preserve exact frozen IDs/read-only/replay guards. Extend optimized Android,
   fully loaded lists, heavy-history/diverse-book and exact peak-memory work.
2. Verify supported AnkiDroid provider/deck/model/duplicate APIs against primary
   documentation; record feasible exact-surface duplicate behavior before wiring.
3. Extend remaining accessibility/process-death/lifecycle matrix. Physical arm64
   remains user-deferred; emulator/host results do not establish viability.

## Verification / blockers
- PASS: regenerated both-ABI native/bindings, debug/test APKs, strict Android
  lint, repository Kotlin formatting/diff. Sixteen existing Vocabulary/navigation
  regressions plus default assumption skip (17 reported, 205.770 s). Opt-in
  10k/100k processes pass (32.330/49.535 s), all guards and scratch cleanup pass.
- PASS: 50 raw action samples/110 observed frame durations validate counts,
  medians/maxima/reference counts, payload lengths, positive memory observations
  and zero reported drops. All observed frames exceed fixed 16.667 ms reference;
  these small debug/observer samples are not a portable jank percentage.
- Results/hashes/limits: VOCABULARY_ANDROID_PERFORMANCE.md; measurements/
  vocabulary-compose-android-2026-10-30{,-frames}.csv. Commands: DEVELOPMENT.md,
  final section; use pinned Rust/JDK/SDK/Gradle environment there.
- Build: ./android/gradlew -p android :app:assembleDebug
  :app:assembleDebugAndroidTest :app:lintDebug --offline --console=plain
- Format: JAVA_HOME="$PWD/.local/jdk-17/jdk-17.0.21+9"
  tools/format-kotlin.sh --check; git diff --check.
- Measure: adb shell am instrument -w -r -e class
  app.yomoku.VocabularyPerformanceTest -e yomokuMeasure true
  -e yomokuVocabularyEntries 100000 -e yomokuVocabularyLoaded 1000
  -e yomokuMeasureIterations 5 app.yomoku.test/androidx.test.runner.AndroidJUnitRunner
  Run sizes sequentially without builds/tests/profilers. No input corpus needed.
- Logs: /tmp/yomoku-vocabulary-compose-{android,format,regressions,10000,100000}.log.
  Initial sandbox Gradle lock socket required approved rerun; no open failure.
  Task-started isolated .local/statistics-ui-avd emulator stopped; phone/light
  settings restored. Existing ignored image and dictionary caches preserved.
- Rust host tests/Clippy, host JNI/codegen reproducibility, separate Full-loader
  matrix, optimized/tablet/dark/continuous-fling/heavy-history/diverse-book device
  performance, process-death and physical arm64 not rerun. Broader tasks stay open.
