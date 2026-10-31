# Resume here
Updated: 2026-10-31. Actual Vocabulary callback/Compose interval profiles verified.
Commit guidance: explanatory bodies; merges include provenance. Keep verification
results in session/progress docs and reports, out of commit message bodies.

## Current slice
- Integrated scalar decoder remains guarded/reproducible with full exact IDs.
- Optional CoreSession callback observer records Kotlin listener arrival,
  arrival-to-main queue and posted handler time. Volatile publication; delivery
  stays on main and requires the same observer still installed. No disabled clocks
  or measurement records; production callback/render/clock sequence unchanged.
- Test-only pinned Compose observer records begin/end/depth for root/inherited
  subcompositions/animations and detaches through DisposableEffect. Use interval
  union; no overlap/nesting observed here. Do not sum with frame layout metrics
  or subtract phases from idle time to attribute missing/native work.
- Separate debug API-36 x86_64 10k/100k profiles use 1,000 loaded rows, all selected/
  frozen IDs, one full warmup/five measured scenarios each. All 60 actions/80
  renders/70 requests/30 callbacks/1,982 passes/108 frames validate. Every equal
  replay keeps the state instance with no request/callback/composition pass.
- 100k Select All median queue/handler/composition union: 54.153/71.460/192.414 ms;
  idle 936.683 ms. These observer/debug profiles establish no before/after speed,
  exact peak/retained-memory, native-worker breakdown or hardware viability claim.
- Results/hashes/six raw CSVs: VOCABULARY_ANDROID_PERFORMANCE.md, final section;
  measurements/vocabulary-callback-compose-android-2026-10-31-*.csv.
- Schema 0020. Nostr starter configuration empty; ask Q188 before forwarding.

## Next tasks (ordered)
1. Isolate native async FIFO/SQL/core-response/serialization work and 100k Select
   All Compose costs using the new profiles. Compare an observer-free optimized
   workload before choosing a production change; preserve full IDs, structural
   equality and event/replay guards. Heavy-history/diverse-book/full-load/exact
   peak work remains; arrival offsets include scheduling/setup, not just native.
2. Verify supported AnkiDroid provider/deck/model/duplicate APIs against primary
   documentation; record feasible exact-surface duplicate behavior before wiring.
3. Extend remaining accessibility/process-death/lifecycle matrix. Physical arm64
   remains user-deferred; emulator/host results do not establish viability.

## Verification / blockers
- PASS: debug/test APKs, strict Android lint, repository Kotlin formatting/diff.
  Native/bindings/assets reused outputs. Ten focused cases plus three default
  opt-in skips (13 reported, 11.216 s), all 17 Vocabulary UI/production/navigation
  regressions (221.766 s), separate 10k/100k profiles (37.639/54.632 s).
- Exact full IDs, unchanged database SHA-256, no analyzer/reading effects,
  integrity/FK/native close/scratch deletion pass; ADB confirms cleanup.
- Exact commands: DEVELOPMENT.md, final Vocabulary callback queue and Compose
  passes. Build uses pinned Rust PATH/JDK 17/Gradle/SDK environment, then
  ./android/gradlew -p android :app:assembleDebug :app:assembleDebugAndroidTest
  :app:lintDebug --offline --console=plain
- Profile: adb shell am instrument -w -r -e class app.yomoku.VocabularyPerformanceTest
  -e yomokuMeasure true -e yomokuMeasureRender true -e yomokuMeasureComposition true
  -e yomokuVocabularyEntries 100000 -e yomokuVocabularyLoaded 1000
  -e yomokuMeasureIterations 5 app.yomoku.test/androidx.test.runner.AndroidJUnitRunner
  No concurrent builds/tests/profilers. Separate processes per size; disposable DB.
- Logs: /tmp/yomoku-callback-{android,format,focused,regressions,10000,100000,
  validation}.log; validator /tmp/yomoku-validate-callback-profile.py. Initial
  sandbox Gradle socket startup needed approved rerun; no source/test failure
  or open blocker. Task-started isolated AVD stopped; ignored image/caches kept.
- Host Rust/Clippy/JNI/codegen, independent Full-loader matrix, native worker
  breakdown, optimized/tablet/dark/fling/heavy-history/diverse-book/full-load/
  exact peaks/lifecycle/process-death/physical arm64 not rerun. Broad tasks open.
