# Resume here
Updated: 2026-10-31. Vocabulary request phases and equal snapshots verified.
Commit guidance: explanatory bodies; merges include provenance. Keep verification
results in session/progress docs and reports, out of commit message bodies.

## Current slice
- Opt-in CoreSession diagnostics now separate event encoding, synchronous JNI
  update and generated request decoding; callback native intervals stay absent.
  Normal path reads no clocks and creates no measurement records. Native callback
  work/handler queue delay and composition remain outside these intervals.
- Diagnostic main-thread snapshot replay requires a render observer/open session,
  reuses production decode/publication, and sends no event/requests/notification.
  Equal frozen confirmations retain the existing state instance and all IDs.
  Product event guards and structural equality policy remain intact.
- Separate debug API-36 x86_64 phone/light 10k/100k processes: 1,000 retained rows,
  one full warmup/five samples. At 100k Select All/confirmation synchronous native
  update medians are 0.298/0.335 ms; request decoding max is 0.090 ms overall.
- At 100k equal-confirmation native/decode/publication medians are
  61.037/192.500/5.594 ms; publication range 3.936–35.825 ms. All ten equal
  renders retain prior state; 70 changed renders publish. No equality-policy,
  speed/frame-rate or isolated composition inference. Replay shares GC history
  with preceding serialization/assertions, so it is not a paired microbenchmark.
- Raw 80 render/70 request/60 action rows and 111 frames validate. Process-memory
  sampling/debug/dialog-window limits apply; no exact peak/PSS claim.
- No Rust/schema/dependency/wire/threading/destructive change. Schema 0020.
  No Nostr starter data; configuration empty. Ask Q188 before forwarding.

## Next tasks (ordered)
1. Investigate large-ID Kotlin decoding/allocation with controlled comparisons;
   profile asynchronous native callback work and composition separately. Keep
   exact frozen/read-only/replay guards; extend optimized/fully loaded/heavy-
   history/diverse-book cases and exact peaks before broader performance claims.
2. Verify supported AnkiDroid provider/deck/model/duplicate APIs against primary
   documentation; record feasible exact-surface duplicate behavior before wiring.
3. Extend remaining accessibility/process-death/lifecycle matrix. Physical arm64
   remains user-deferred; emulator/host results do not establish viability.

## Verification / blockers
- PASS: debug/test APKs, strict Android lint, repository Kotlin formatting/diff;
  native/bindings/assets reused Gradle outputs. Twenty-one passing regressions
  plus harness default assumption skip (22 reported, 240.384 s), including replay
  identity/no requests/no notification and observer thread/removal tests.
- PASS: separate opt-in 10k/100k profiles (38.092/60.011 s). Exact rows/selected/
  frozen IDs, unchanged bytes, zero analyzer/reading effects, integrity/FK/native
  closure/scratch cleanup and all raw metrics validate. ADB confirms scratch
  directories absent; task-started isolated AVD stopped with phone/light retained.
- Results/hashes/limits: VOCABULARY_ANDROID_PERFORMANCE.md, final request/replay
  section; measurements/vocabulary-request-android-2026-10-31*.csv. Exact pinned
  build/install/regression/profile commands: DEVELOPMENT.md, final Vocabulary
  sections; include CoreSessionTest in regression class list.
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
- Logs: /tmp/yomoku-request-{android,format-check,regressions,10000,100000,
  validation}.log. Initial sandbox Gradle lock-socket access needed approved
  rerun; no source/test failures or open blocker. Existing ignored caches kept.
- Host Rust/Clippy, independent JNI/codegen/Full-loader, optimized/tablet/dark/
  continuous-fling/heavy-history/diverse-book/fully loaded, exact peaks/PSS,
  process-death and physical arm64 not rerun. Broader tasks stay open.
