# Resume here
Updated: 2026-10-30. Vocabulary snapshot render phases profiled and verified.
Commit guidance: explanatory bodies; merges include provenance. Keep verification
results in session/progress docs and reports, out of commit message bodies.

## Current slice
- CoreSession has an opt-in main-thread render observer separating native view
  retrieval, generated Kotlin decoding and Compose state publication. Distinguish
  synchronous dispatch/post-native callbacks; record payload and loaded/selected/
  frozen counts without retaining snapshots or IDs. Normal path reads no clocks.
- Production sequence, structural state equality, full hidden/unloaded IDs and
  frozen confirmations remain intact. Publication includes equality/notification,
  excluding later composition. Request decoding/update/queue delay are not timed.
- Separate debug API-36 x86_64 phone/light 10k/100k processes: 1,000 retained rows,
  one full warmup and five samples. At 100k Select All callback native/decode/
  publication medians: 22.046/82.612/0.785 ms; confirmation 29.789/65.726/1.009 ms.
  All 70 renders publish changed instances; equal-snapshot cost remains unmeasured.
- 100k Select All median idle 915.957 ms; do not assign residual time to composition
  or compare these observer runs as an optimization. Decoding/native work remain
  measurable targets. Retain equality policy; do not infer latency/frame gains.
- Raw 70 phase rows/50 actions/107 frames retained and validated. Existing sampled
  process-memory/dialog-window/debug limits apply; no exact peak/PSS claim.
- No Rust/schema/dependency/wire/threading/destructive change. Schema 0020.
  No Nostr starter data; configuration empty. Ask Q188 before forwarding.

## Next tasks (ordered)
1. Profile remaining Vocabulary native update/request and composition work;
   investigate large-ID decoding with exact frozen/read-only/replay guards.
   Include equal snapshots, optimized/fully loaded/heavy-history/diverse-book
   cases and exact peak memory before making broader performance claims.
2. Verify supported AnkiDroid provider/deck/model/duplicate APIs against primary
   documentation; record feasible exact-surface duplicate behavior before wiring.
3. Extend remaining accessibility/process-death/lifecycle matrix. Physical arm64
   remains user-deferred; emulator/host results do not establish viability.

## Verification / blockers
- PASS: debug/test APKs, strict Android lint, repository Kotlin formatting/diff;
  native/bindings/assets reused Gradle outputs. Twenty passing regressions plus
  harness default assumption skip (21 reported, 219.050 s), including observer
  installation/removal and independent-handle coverage in CoreSessionTest.
- PASS: separate opt-in 10k/100k processes (36.355/53.160 s). Exact rows/selected/
  frozen IDs, unchanged bytes, zero analyzer/reading effects, integrity/FK/native
  closure/scratch cleanup and all raw metrics validate. ADB shows no scratch dirs.
- Results/hashes/limits: VOCABULARY_ANDROID_PERFORMANCE.md, snapshot phase section,
  and measurements/vocabulary-render-android-2026-10-30{,-actions,-frames}.csv.
  Exact pinned environment/build/install/regression/profile commands:
  DEVELOPMENT.md, final Vocabulary sections; add CoreSessionTest to class list.
- Build: ./android/gradlew -p android :app:assembleDebug
  :app:assembleDebugAndroidTest :app:lintDebug --offline --console=plain
- Format: JAVA_HOME="$PWD/.local/jdk-17/jdk-17.0.21+9"
  tools/format-kotlin.sh --check; git diff --check.
- Profile: adb shell am instrument -w -r -e class
  app.yomoku.VocabularyPerformanceTest -e yomokuMeasure true
  -e yomokuMeasureRender true -e yomokuVocabularyEntries 100000
  -e yomokuVocabularyLoaded 1000 -e yomokuMeasureIterations 5
  app.yomoku.test/androidx.test.runner.AndroidJUnitRunner
  No competing builds/tests/profilers; no input corpus needed.
- Logs: /tmp/yomoku-render-{android,format-check,regressions,10000,100000,
  validation}.log. Initial sandbox Gradle lock-socket failure needed approved
  rerun; initial nullable-selection compile error corrected; no open failure.
  Task-started isolated .local/statistics-ui-avd stopped; phone/light retained;
  existing ignored caches preserved.
- Host Rust/Clippy, independent JNI/codegen/Full-loader, optimized/tablet/dark/
  continuous-fling/heavy-history/diverse-book/fully loaded, equal-snapshot,
  process-death, exact peaks/PSS and physical arm64 not rerun. Broader tasks open.
