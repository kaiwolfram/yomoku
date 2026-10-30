# Resume here
Updated: 2026-10-30. Vocabulary Compose selection membership bounded and verified.
Commit guidance: explanatory bodies; merges include provenance. Keep verification
results in session/progress docs and reports, out of commit message bodies.

## Current slice
- VocabularyScreen binary-searches the core's sorted unique positive-i64 IDs
  per composed row. Removed the selection-sized hash set and structural remember
  key; complete hidden/unloaded IDs, counts and frozen confirmations are retained.
- Two exact-ID/resource-bound instrumentation cases and one Compose case cover
  gaps, empty/single lists, >2^53/Long.MAX_VALUE, at most 17 indexed reads at 100k,
  arbitrary row sort and equal-size selection replacement without stale semantics.
- Existing native bulk/history/recreation/navigation guards pass. Full callback
  decoding, view equality, encoding and native storage remain unchanged.
- Sequential baseline/changed debug API-36 x86_64 phone/light processes: 100k
  entries, 1,000 retained rows, one warmup and five samples. Select All median
  action-to-idle 881.920→765.376 ms; ranges overlap. Confirmation/frame durations
  do not consistently improve. No general latency/frame-rate claim.
- Fifty action rows/116 observed frames retained and validated. Sampled Java/
  native/RSS maxima are separate process high-water observations, not exact peaks
  or allocations attributable to this change. Confirmation views remain ~1.65 MB.
- No Rust/schema/dependency/wire/architecture/destructive change. Schema 0020.
  No Nostr starter data; configuration empty. Ask Q188 before forwarding.

## Next tasks (ordered)
1. Profile large Vocabulary main callback decoding/view equality and remaining
   composition. Preserve exact frozen IDs/read-only/replay guards. Extend optimized
   Android, fully loaded lists, heavy-history/diverse-book and exact peak-memory work.
2. Verify supported AnkiDroid provider/deck/model/duplicate APIs against primary
   documentation; record feasible exact-surface duplicate behavior before wiring.
3. Extend remaining accessibility/process-death/lifecycle matrix. Physical arm64
   remains user-deferred; emulator/host results do not establish viability.

## Verification / blockers
- PASS: debug/test APKs, strict Android lint, repository Kotlin formatting/diff;
  native/bindings/assets reused existing Gradle outputs, not rebuilt. Nineteen
  passing cases plus harness default assumption skip (20 reported, 228.889 s).
- PASS: separate baseline/changed opt-in 100k runs (51.327/50.541 s). All exact
  rows/selection/confirmation, no analyzer/reading effects, unchanged bytes,
  integrity/FK/native closure/scratch cleanup guards pass. ADB confirms no scratch
  directories. Fifty action rows/116 raw frame durations validate metrics, payload
  sizes, positive memory observations and zero reported drops.
- Results/hashes/limits: VOCABULARY_ANDROID_PERFORMANCE.md, final section, and
  measurements/vocabulary-selection-android-2026-10-30{,-frames}.csv.
  Exact commands/pinned Rust/JDK/SDK/Gradle environment: DEVELOPMENT.md, final sections.
- Build: ./android/gradlew -p android :app:assembleDebug
  :app:assembleDebugAndroidTest :app:lintDebug --offline --console=plain
- Format: JAVA_HOME="$PWD/.local/jdk-17/jdk-17.0.21+9"
  tools/format-kotlin.sh --check; git diff --check.
- Regression classes now include app.yomoku.VocabularySelectionLookupTest alongside
  VocabularyScreenTest/VocabularyProductionTest/NavigationTest/
  EffectiveReaderNavigationTest/VocabularyPerformanceTest.
- Measure: adb shell am instrument -w -r -e class
  app.yomoku.VocabularyPerformanceTest -e yomokuMeasure true
  -e yomokuVocabularyEntries 100000 -e yomokuVocabularyLoaded 1000
  -e yomokuMeasureIterations 5 app.yomoku.test/androidx.test.runner.AndroidJUnitRunner
  No competing builds/tests/profilers; no input corpus needed.
- Logs: /tmp/yomoku-selection-{android,format-check,regressions,before-100000,
  after-100000}.log. Initial sandbox Gradle lock-socket failure required approved
  rerun; no open failure. Task-started isolated .local/statistics-ui-avd stopped;
  phone/light settings retained; existing ignored caches preserved.
- Rust host/Clippy, independent host JNI/codegen, separate Full-loader matrix,
  optimized/tablet/dark/continuous-fling/heavy-history/diverse-book/fully loaded
  device measurements, process-death, exact peaks/PSS and physical arm64 not rerun.
  Broader tasks remain unchecked.
