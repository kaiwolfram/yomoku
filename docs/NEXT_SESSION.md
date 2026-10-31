# Resume here
Updated: 2026-10-31. Controlled Vocabulary ID-list decoding comparisons verified.
Commit guidance: explanatory bodies; merges include provenance. Keep verification
results in session/progress docs and reports, out of commit message bodies.

## Current slice
- Added test-only VocabularyDecodingTest: exact generated list/element decoder,
  validated preallocation and growing/preallocated fixed-width ByteBuffer readers.
  No production runtime/generation/schema/dependency/wire/equality/event change.
- Two correctness cases cover empty/100k/gapped/>2^53/signed wire IDs, generated
  byte round trips, all short-payload truncations, bad lengths and trailing bytes.
- Four opt-in debug API-36 x86_64 processes (two per 10k/100k size), four warmup/
  eight measured balanced rounds. All 128 raw rows validate and inputs stay exact.
- At 100k generated medians are 25.502/23.098 ms versus capacity 25.341/23.795;
  buffered+capacity 3.303/3.029 ms (long samples up to 46.911 ms). These are single
  lists, not full snapshots/actions; specialized readers also omit generic and
  per-ID container calls. No production speed/composition/peak-memory claim.
- Bytecode shows RangesKt.until/step for each deserialize_u64/getLong invocation
  from the pinned runtime's `0 until 64 step 8` loops. Target these allocations;
  current comparisons do not isolate their contribution. ART allocation/GC
  counters are approximate/process-wide, lag (including zero samples) and include
  unrelated work. Keep full selected/frozen IDs and production replay guards.
- Schema 0020. Nostr starter configuration empty; ask Q188 before forwarding.

## Next tasks (ordered)
1. Test an allocation-free scalar 64-bit loop through reproducible Kotlin
   generation, preserving byte order, errors/offsets and signed/unsigned values;
   compare against legacy scalar loops, then full frozen snapshots. Avoid adopting
   specialized list readers as arbitrary schema decoders. Async native callback/
   composition, optimized/heavy-history/diverse-book/full-load/peak work remains.
2. Verify supported AnkiDroid provider/deck/model/duplicate APIs against primary
   documentation; record feasible exact-surface duplicate behavior before wiring.
3. Extend remaining accessibility/process-death/lifecycle matrix. Physical arm64
   remains user-deferred; emulator/host results do not establish viability.

## Verification / blockers
- PASS: debug/test APKs, strict Android lint, repository Kotlin formatting/diff;
  native/bindings/assets reused Gradle outputs. Production debug APK unchanged.
- PASS: CoreSessionTest, VocabularySelectionLookupTest, VocabularyDecodingTest:
  six passing cases plus measurement default assumption skip (seven, 0.720 s).
- PASS: four profiles (10k/1 0.671 s, 100k/1 2.156 s, 100k/2 2.135 s,
  10k/2 0.651 s). All 128 raw rows/positions/payloads/counters/round trips validate.
- Results/hashes/method/limits: VOCABULARY_ANDROID_PERFORMANCE.md, controlled
  ID-list section; measurements/vocabulary-decoding-android-2026-10-31.csv.
  Exact commands: DEVELOPMENT.md, final Controlled Vocabulary ID-list section.
- Build: PATH="$HOME/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin:$PATH"
  JAVA_HOME="$PWD/.local/jdk-17/jdk-17.0.21+9" GRADLE_USER_HOME="$PWD/.local/gradle"
  ANDROID_HOME=/home/kai/Android/Sdk ./android/gradlew -p android :app:assembleDebug
  :app:assembleDebugAndroidTest :app:lintDebug --offline --console=plain
- Format: JAVA_HOME="$PWD/.local/jdk-17/jdk-17.0.21+9"
  tools/format-kotlin.sh --check; git diff --check.
- Profile: adb shell am instrument -w -r -e class
  'app.yomoku.VocabularyDecodingTest#measureIdenticalIdPayloads'
  -e yomokuMeasureDecode true -e yomokuVocabularyEntries 100000
  -e yomokuMeasureIterations 8 app.yomoku.test/androidx.test.runner.AndroidJUnitRunner
  No competing builds/tests/profilers; no database/corpus/analyzer needed.
- Logs: /tmp/yomoku-decoding-{android,format-check,regressions,10000-1,10000-2,
  100000-1,100000-2,validation}.log; bytecode -bytecode.txt. Initial sandbox Gradle
  socket startup needed approved rerun; no source/test failure or open blocker.
- Task-started isolated AVD stopped; existing ignored caches/image retained.
- Host Rust/Clippy/JNI/independent generation, full Compose/native/optimized/
  tablet/dark/continuous-fling/heavy-history/diverse-book/fully loaded/exact peaks,
  process-death and physical arm64 not rerun. Broader tasks stay unchecked.
