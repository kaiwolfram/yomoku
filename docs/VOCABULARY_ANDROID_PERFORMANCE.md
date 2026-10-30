# Vocabulary Android measurements

The opt-in `VocabularyPerformanceTest` renders the production `YomokuApp` and
Vocabulary screen in the existing isolated debug `ComponentActivity`. It uses a
real `CoreSession`, generated JNI/Bincode codecs, native FIFO and SQLite reads.
The test is skipped unless `yomokuMeasure=true`. No dependency, production UI,
schema or protocol change is required.

## Method and correctness guards

- A fresh session creates/migrates a scratch database under `noBackupFilesDir`.
  After closing it, a single transaction seeds deterministic synthetic surfaces
  with valid New registration events. The measured session reopens that database.
  The app's normal `yomoku.sqlite3` is never opened by this harness.
- Every entry is New, registered at the same date, with no encounters or
  dictionary-form links. Surfaces are `測定-000001` through the requested count.
  This isolates browser/list/selection costs; it does not reproduce heavy history,
  mixed statuses, real language, document imports or a diverse Library.
- One complete scenario is excluded as warmup, followed by the requested number
  of scenarios. Each changes between two literal prefixes matching every entry,
  verifies the first 100 rows, scrolls to append 100 more, then loads the requested
  retained-row count. Setup and intermediate paging are outside measured actions.
- A measured scroll jumps between the first and a late retained row, stopping
  20 rows before the end to avoid another append. This exercises actual Compose
  lazy layout/semantics; it is an instrumented jump, not a continuous finger fling.
- Select All captures all matching IDs, including unloaded rows. Confirmation
  freezes those same IDs. Every scenario verifies exact descending row IDs and
  ascending selected IDs, zero counts, New statuses and visible selection/dialog.
  It cancels both confirmation and selection; no bulk write is executed.
- A dictionary provider fails and counts any analyzer dictionary request. Final
  guards require zero calls, identical database SHA-256, SQLite integrity/foreign
  keys, no completion/occurrence/pending-time rows, native handle closure and
  scratch-directory removal. Existing production regressions cover real imports,
  histories, mutation/Retry/recreation and provenance separately.

## Measurement definitions and limits

`MEASURE` rows retain each raw scenario sample. `idle_ms` spans the action and
Compose/native idle synchronization, including instrumentation polling, input,
semantics and repeated idle checks. It includes asynchronous callback decoding,
recomposition and layout but is not an exact tap-to-photon latency. `dispatch_ms`
is only the synchronous `CoreSession.dispatch` call on the main thread, excluding
async callback decoding; scrolling has no single dispatch and reports zero.
There are no timing pass/fail thresholds.

The main Activity window reports `FrameMetrics.TOTAL_DURATION` on a dedicated
HandlerThread. The listener immediately extracts primitive values, respecting
the API's reused-object lifetime. Retained frames have an intended-vsync timestamp
inside the action's `System.nanoTime` interval; reports delivered after the handler
barrier can be missed. `FRAMES` rows retain every observed duration. Summaries
include frame count, median/max duration, frames above a fixed 16.667 ms reference
and reported callback drops. This reference is not an Android jank classification.
The confirmation Dialog has a separate window and is excluded from frame data;
its work is included in action time and process memory. Idle actions need not
produce frames, so zero frames are represented explicitly.

A background thread samples used Java heap (`totalMemory - freeMemory`), native
heap allocations (`Debug.getNativeHeapAllocatedSize`) and `/proc/self/status`
VmRSS, approximately every 25 ms plus start/end observations. Each column is the
maximum of its own samples, not a simultaneously observed breakdown; values must
not be added. These are sampled high-water values, not exact peak allocations,
retained-object sizes, PSS or complete graphics accounting. No forced GC is used.
Earlier actions, test assertions, instrumentation and observers share the process
and influence the numbers. Observers are active during timings; their overhead
is not subtracted. `memory_samples` records coverage, including the boundaries.
`PAYLOAD` records complete confirmation-view Bincode size outside timing/memory
intervals; it serializes a separate diagnostic copy.

Primary API references checked 2026-10-30:
[frame callback lifetime and dropped reports](https://developer.android.com/reference/android/view/Window.OnFrameMetricsAvailableListener),
[frame metric meanings and unavailable values](https://developer.android.com/reference/android/view/FrameMetrics),
[native allocation measurement](https://developer.android.com/reference/android/os/Debug#getNativeHeapAllocatedSize()).

## Reproduce

Use the pinned tools, APK build/install and isolated emulator commands in
[DEVELOPMENT.md](DEVELOPMENT.md). Run each size in a separate instrumentation
process, sequentially, without competing builds/tests/profilers:

```sh
"$ANDROID_HOME/platform-tools/adb" -s emulator-5554 shell am instrument -w -r \
  -e class app.yomoku.VocabularyPerformanceTest -e yomokuMeasure true \
  -e yomokuVocabularyEntries 10000 -e yomokuVocabularyLoaded 1000 \
  -e yomokuMeasureIterations 5 \
  app.yomoku.test/androidx.test.runner.AndroidJUnitRunner
```

`yomokuVocabularyEntries` accepts 200–100,000; `yomokuVocabularyLoaded` accepts
200–entries in multiples of 100; iterations accept 1–100. Defaults are
10,000 / 1,000 / 5. Invalid arguments fail before creating scratch files.
Repeat with 100,000 entries. Larger retained-row runs are opt-in and remain
distinct from selecting all entries with only 1,000 rows loaded.

## Results, 2026-10-30

Fedora x86_64 host, existing API-36 Google Play x86_64 `medium_phone` emulator,
KVM/two cores/swangle with Vulkan disabled; phone/light 1080×2400 at 420 dpi.
Fingerprint: `google/sdk_gphone64_x86_64/emu64xa:16/BE2A.250530.026.D1/13818094:user/release-keys`.
Debug APK, unoptimized native debug build, 192 MiB maximum Java heap. Each size
ran in a fresh instrumentation process, sequentially after all builds/regressions
finished. Five samples after one complete scenario warmup; 1,000 retained rows.
CPU frequency, scheduling, JIT/GC and OS/storage caches remain uncontrolled.

Median action-to-idle / synchronous main dispatch, milliseconds:

| Action | 10,000 entries | 100,000 entries |
| --- | ---: | ---: |
| First 100 rows | 149.596 / 0.736 | 282.337 / 0.995 |
| Scroll and append 100→200 | 182.224 / — | 332.049 / — |
| Jump between retained rows | 263.304 / — | 264.962 / — |
| Select every matching ID | 153.075 / 17.777 | 865.486 / 16.955 |
| Freeze/render delete confirmation | 276.604 / 13.495 | 321.554 / 87.446 |

Select All action ranges were 146.614–565.046 ms at 10k and
732.723–1,049.362 ms at 100k. Observed main-window Select All frame durations
had medians/maxima 67.027/273.804 ms (seven frames across five 10k actions) and
119.149/663.275 ms (thirteen frames across five 100k actions). Overall, 110
observed frames exceeded the fixed 16.667 ms reference; no callback drops were
reported. These small, instrumented debug-emulator frame samples establish a
profiling target, not a portable frame-rate or user-visible jank percentage.

Maximum sampled values across **measured actions** (MiB):

| Entries | Used Java heap | Native allocated heap | RSS |
| ---: | ---: | ---: | ---: |
| 10,000 | 18.081 | 22.136 | 246.395 |
| 100,000 | 43.619 | 28.574 | 278.637 |

The columns peak at different times and overlap; do not sum them or treat their
difference as unattributed memory. Setup/seeding/intermediate loading/cleanup
are excluded. GC and prior retained views can move the highest sampled value to
the next action, so these are not allocations caused exclusively by that action.
Measured actions have 7–28 memory observations each, including boundaries.
Complete confirmation views are 210,216–210,217 bytes at 10k and
1,650,216–1,650,217 bytes at 100k; the one-byte variation is the alternating
prefix. Selection and confirmation each retain all matching IDs, even with only
1,000 rows loaded. Main callback decoding and selection composition are the next
profiling targets; these timings do not attribute the delay to one layer.

All 50 per-action rows and 110 raw frame durations are retained in the
[measurement CSV](measurements/vocabulary-compose-android-2026-10-30.csv) and
[frame CSV](measurements/vocabulary-compose-android-2026-10-30-frames.csv).
Frame counts/medians/maxima/reference counts validate against the raw durations;
each metric has exactly five iterations and the expected 100/200/1,000 rows.
Payload lengths, positive memory observations and zero reported drops validate.

Both runs pass exact rows/selection/confirmation, no analyzer access, no reading
effects, unchanged bytes, integrity/FK and scratch cleanup. Fixture database
sizes and unchanged SHA-256:

- 10k: 1,359,872 bytes;
  `dbaa96d39b8b3d425c2a6604332aff6f81e2554ce1cc1d6016172abc2ce61f68`.
- 100k: 11,890,688 bytes;
  `61a530e680d3d1ce1e6c6cecd7aae9b1d6441c0e2383bab825baf01b9a5d156c`.

APK SHA-256: `9b9ca43a77da02db939d2088c7fd4e2d9b2b86314cb44a97aa67fa0e325e16a6`.
Packaged-source x86_64 JNI SHA-256:
`d7193601bcd820b47b6fc6b8a14f7b8d7f93f37beddbebe99ba1917aa49bb726`.
Production source baseline: `8f9bd05` (no production changes in this slice).

PASS: debug/test APKs, both-ABI native rebuild/codegen, strict Android lint,
repository Kotlin formatting and diff checks. Sixteen existing Vocabulary/
navigation cases pass with the new default opt-out (17 reported, one assumption
skip; 205.770 s). The two opt-in processes pass in 32.330/49.535 s. Logs:
`/tmp/yomoku-vocabulary-compose-{android,format,regressions,10000,100000}.log`.
Initial sandbox Gradle lock-socket access required an approved rerun; no source
or test failure remains. Scratch directories are absent from app `no_backup`.

No optimized-build, continuous-fling, heavy-history device, fully loaded 100k-row,
diverse many-book/image-heavy, exact peak/PSS, tablet/dark measurement, process-death
or physical-arm64 claim. Rust host tests/Clippy, independent host JNI/codegen
reproducibility and the separate Full-loader matrix were not rerun. The broader
performance/lifecycle tasks remain open.

## Selection membership change, 2026-10-30

`VocabularyScreen` previously materialized the entire retained selection as a
hash set under `remember(state.selection)`. This adds a selection-sized collection
on the main composition path and structurally compares the full remember key on
equal subsequent snapshots. Core already retains sorted unique positive-i64 IDs,
so rows now binary-search that list directly, independently of browser row sort.
Only composed rows perform membership checks; no auxiliary selection collection
is built. Full IDs, selected counts and frozen confirmation commands remain
unchanged. Snapshot encoding/decoding, native reads and confirmation are unchanged.

Two new instrumentation cases check exact membership at gaps, empty/single lists,
values above 2^53 and Long.MAX_VALUE, plus a counting list that requires at most
17 indexed reads for each tested present/absent ID in a 100,000-entry selection.
A new Compose case checks a descending row order and an equal-size replacement
selection, including Long.MAX_VALUE, without stale checkbox/selected semantics.
The production bulk/history/recreation/navigation regressions also pass.

Compare the original APK from `37b70ce` with the changed APK using the same method,
API-36 x86_64 isolated phone/light AVD and 100,000-entry fixture above. Each runs
in a fresh instrumentation process, one complete scenario warmup followed by five
samples, with 1,000 retained rows. Builds and regressions finish before measuring;
no competing build/test/profiler runs during either measurement. These are two
sequential debug runs with uncontrolled scheduling/JIT/GC/caches, not a randomized
benchmark or attribution of total action cost to selection membership. The
counting-list regression establishes the bounded lookup work separately.

Median action-to-idle / synchronous dispatch, milliseconds:

| Action | Before | Binary-search membership |
| --- | ---: | ---: |
| First 100 rows | 265.597 / 2.267 | 264.898 / 2.962 |
| Append 100→200 | 298.958 / — | 315.197 / — |
| Jump between retained rows | 264.593 / — | 249.014 / — |
| Select every matching ID | 881.920 / 25.188 | 765.376 / 20.684 |
| Freeze/render delete confirmation | 350.962 / 98.772 | 371.830 / 79.947 |

Select All action ranges are 865.192–985.128 ms before and 665.854–933.088 ms
after. Observed Select All main-window frames have median/max durations of
142.993/484.738 ms before and 147.276/472.613 ms after (13 frames in each run).
The ranges overlap, confirmation median increases, and frame durations do not
show consistent improvement. This comparison does not establish a general
latency/frame-rate improvement or assign the remaining delay to one layer.
Main callback decoding, complete view comparison and other composition work
remain profiling targets.

Maximum sampled values across measured actions, MiB:

| Run | Used Java heap | Native allocated heap | RSS |
| --- | ---: | ---: | ---: |
| Before | 42.366 | 29.622 | 278.309 |
| Binary-search membership | 30.077 | 29.628 | 270.617 |

As above, these are independently sampled process high-water values at different
times; never sum them or interpret the difference as exact allocation savings
caused by this change. Confirmation payloads remain 1,650,216–1,650,217 bytes.
All 50 action rows and 116 observed frame durations validate against the
[comparison CSV](measurements/vocabulary-selection-android-2026-10-30.csv) and
[frame CSV](measurements/vocabulary-selection-android-2026-10-30-frames.csv):
five iterations per action/run, expected loaded counts, medians/extrema/reference
counts, positive memory observations, payload sizes and zero reported drops.

PASS: debug/test APKs, strict Android lint, repository Kotlin formatting/diff,
19 regression cases plus one default opt-in skip (20 reported, 228.889 s), and
both opt-in comparison runs (51.327/50.541 s). Both retain exact selected/frozen
IDs, unchanged bytes, zero analyzer calls/reading effects, integrity/FK, native
closure and scratch removal. Post-run ADB shows only the existing `sudachi`
directory in `no_backup`. Fixture size is 11,890,688 bytes and SHA-256 remains
`61a530e680d3d1ce1e6c6cecd7aae9b1d6441c0e2383bab825baf01b9a5d156c`.

Changed debug APK SHA-256:
`42c92850cad3f9481617afdccd565f0a2247ec34c42382f9b3135a987bc8bd50`;
test APK: `78570e15ec8d68a196ea211e40aaa43d6a2aaeee8a4ac4f5cf5366818b2e6df9`.
Baseline APK and packaged-source x86_64 JNI hashes match the earlier section;
native/binding/asset Gradle tasks are up-to-date, not rebuilt in this slice.
No Rust/schema/dependency/wire change. Logs:
`/tmp/yomoku-selection-{android,format-check,regressions,before-100000,after-100000}.log`.
Initial sandbox Gradle lock-socket access needed an approved rerun; no open
failure. Rust host/Clippy, separate JNI/codegen/Full-loader, optimized/tablet/dark/
continuous-fling/heavy-history/diverse-book/fully loaded lists, exact peak/PSS,
process-death and physical arm64 were not rerun or claimed.

## Snapshot phase profiling, 2026-10-30

Add `-e yomokuMeasureRender true` to the existing opt-in instrumentation command.
The default harness and normal app leave this observer disabled. During each
measured action, CoreSession records one `RENDER` row per actual Render effect,
distinguishing synchronous dispatch from posted native callbacks:

- `native_view_ms`: `NativeBridge.view()`, including Rust snapshot creation,
  encoding and JNI transfer. It excludes the earlier native update/request work
  and SQLite/FIFO request latency.
- `decode_ms`: generated `ViewModel.bincodeDeserialize`, including allocations.
- `publish_ms`: the existing Compose state assignment, including structural
  equality and state notification. Subsequent recomposition/layout is outside
  this interval; this does not isolate equality alone.
- Payload bytes, decoded loaded-row/selected-ID/frozen-confirmation counts and
  `published`: whether the new decoded instance became the state value. An equal
  snapshot retains the earlier state instance under the unchanged default policy.

Primary policy reference: Android's [mutableStateOf and structuralEqualityPolicy
API documentation](https://developer.android.com/reference/kotlin/androidx/compose/runtime/package-summary#mutableStateOf(kotlin.Any,androidx.compose.runtime.SnapshotMutationPolicy)).
No new dependency/API version is adopted. The observer setter requires the main
thread, records no snapshots/IDs and is installed only around timed actions.
Without it, renders take the original direct path with no clock reads or sample
objects. Sample bookkeeping and output lie outside the phase intervals; output
is deferred until after action timing. All phase clocks use System.nanoTime.

Phase intervals omit request decoding, native update, handler queue delay,
reader-clock/import observers and instrumentation synchronization. Scheduling,
JIT, GC and observer overhead still affect these debug-emulator wall times.
Do not subtract phase totals from action-to-idle to label the residual as
composition or add medians from different samples. These are profiling results,
not a before/after optimization or optimized-build/physical-device claim.

Separate API-36 x86_64 debug phone/light processes retain 1,000 rows with
10k/100k entries, one full scenario warmup and five measured scenarios each.
No competing builds/tests/profilers run during either profile. Median per-render
phase wall times, milliseconds (five samples in each row):

| Entries | Render | Native view | Kotlin decode | State publication |
| ---: | --- | ---: | ---: | ---: |
| 10,000 | Select All callback | 2.153 | 15.339 | 0.418 |
| 100,000 | Select All callback | 22.046 | 82.612 | 0.785 |
| 10,000 | Confirmation dispatch | 2.139 | 12.671 | 0.410 |
| 100,000 | Confirmation dispatch | 29.789 | 65.726 | 1.009 |

At 100k, Select All callback decode ranges 81.131–165.457 ms, native view
12.171–23.138 ms and publication 0.563–4.018 ms. Confirmation decode ranges
60.073–83.084 ms, native view 21.064–81.308 ms and publication 0.962–1.214 ms.
Publication is below 4.615 ms across all 70 retained renders. Every observed
render publishes a changed instance; these scenarios do not measure a large
structurally equal snapshot. Equality may short-circuit on differing fields.
Keep the existing equality policy; the results do not justify replacing it.

Select All median action-to-idle is 182.239 ms at 10k and 915.957 ms at 100k;
confirmation is 289.264/367.161 ms. Select All ranges are 163.428–748.114 and
715.929–1,282.920 ms. These are new instrumented samples, not evidence of a
regression or improvement against earlier runs. Decode and native view retrieval
are measurable targets; the remaining idle duration is unattributed. Profile
native update/request work and composition separately before choosing a change.
The Select All callback payload is 130,204–130,205 bytes at 10k and
850,204–850,205 bytes at 100k; confirmations remain 210,216–210,217 and
1,650,216–1,650,217 bytes. Complete selected/frozen IDs are preserved.

All [70 render rows](measurements/vocabulary-render-android-2026-10-30.csv),
[50 action rows](measurements/vocabulary-render-android-2026-10-30-actions.csv)
and [107 raw frame durations](measurements/vocabulary-render-android-2026-10-30-frames.csv)
validate: five iterations per action/size, source/index/count/payload constraints,
finite nonnegative phases, exact frame medians/extrema/reference counts, positive
memory observations and zero reported frame drops. The sampled process-memory
and separate-dialog-window limits above still apply; no exact peak claim.

PASS: 20 ordinary regressions plus default assumption skip (21 reported,
219.050 s), debug/test APKs, strict Android lint, repository Kotlin formatting
and diff. CoreSessionTest also exercises installing/removing the observer while
independent native handles and rendered destinations remain correct. Both opt-in
profiles pass (36.355/53.160 s), including exact rows/selected/frozen IDs,
unchanged database bytes, zero analyzer/reading effects, integrity/FK/native
closure and scratch removal. Database sizes/hashes and JNI hash match the
original fixture section. ADB confirms only the existing sudachi cache remains
in no_backup. The task-started isolated emulator was stopped afterward.

Debug APK SHA-256:
`758dcd1a51967f778504ec8c3fd131d1bc55fc18f66dce37d21adba781cf93ed`;
test APK:
`137515d96f4c39f61c899d50b96bc0dc7c6740a4016871a56a1556702a5a0bef`.
Production baseline: `3a59964`. Native/binding/assets reuse existing Gradle
outputs. Logs: `/tmp/yomoku-render-{android,format-check,regressions,10000,100000,validation}.log`.
Initial sandbox Gradle lock-socket failure required approved rerun; an initial
compile caught nullable selection counting and was corrected. No open failure.
No Rust/schema/dependency/wire change. Host Rust/Clippy, independent JNI/codegen/
Full-loader, optimized/tablet/dark/continuous-fling/heavy-history/diverse-book/
fully loaded lists, exact peak/PSS, process-death and physical arm64 were not
rerun or claimed; broader tasks remain open.
