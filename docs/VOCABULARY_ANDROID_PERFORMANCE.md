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

## Request phases and equal confirmation replay, 2026-10-31

The same `yomokuMeasureRender=true` opt-in now records `REQUEST` rows as well
as render rows. Synchronous dispatch records generated event encoding,
`NativeBridge.update()` and generated `Requests.bincodeDeserialize` separately.
The JNI update interval includes native event processing, routing and returning
request bytes; it does not isolate Rust CPU time or measure the full asynchronous
SQLite/FIFO operation. Posted callbacks record only main-thread request decoding;
event-encoding/native-update cells are empty because callback native processing
and handler queue delay are outside these intervals. Request records contain only
source, timings, byte count and request count, with no event/snapshot/ID retention.
The normal path reads no clocks and creates no measurement records. Observers
remain main-thread-only and output stays outside action timings.

Each scenario also re-reads the settled confirmation through the production
native-view/decode/publication path, labelled `equal_confirmation` / `replay`.
An installed render observer and open session are required. It sends no event,
handles no requests and does not invoke onViewChanged. This diagnostic exists
because frozen prompts deliberately ignore unrelated product events; their
event guards remain unchanged. All decoded selected and frozen IDs must match
the fixture count, the original state instance must remain installed and the
render must report `published=false`. This deliberately constructed equal
snapshot is not a user action or a measured composition. The action CSV's
`dispatch_ms` column times the replay call for this metric. `publish_ms` still
includes the state setter/equality machinery rather than isolating equality.

Use the commands in DEVELOPMENT.md, final Vocabulary section. Separate debug
API-36 x86_64 phone/light processes retain 1,000 rows, one full scenario warmup
and five measured scenarios at each size. Environment and synthetic fixture
limitations match the earlier sections. No competing builds/tests/profilers run
during profiles. These samples include both observers and an extra replay;
they are not a controlled optimization comparison with earlier runs. Keep the
structural state policy. Do not add medians or assign the residual idle time to
composition; asynchronous native callback work, queue delay, instrumentation,
scheduling/JIT/GC and composition remain mixed outside these measured intervals.

Median synchronous request phases, milliseconds (five samples per row):

| Entries | Dispatch | Event encoding | Native update | Request decoding |
| ---: | --- | ---: | ---: | ---: |
| 10,000 | Select All | 0.022 | 4.604 | 0.041 |
| 100,000 | Select All | 0.022 | 0.298 | 0.034 |
| 10,000 | Confirmation | 0.019 | 0.210 | 0.024 |
| 100,000 | Confirmation | 0.026 | 0.335 | 0.038 |

Native update at 100k ranges 0.267–0.340 ms for Select All and 0.297–0.385 ms
for confirmation. The 10k Select All range is 0.293–9.598 ms; these wall-time
samples do not establish scaling or isolate lock/scheduling costs. Every measured
request contains one Render effect in 16 bytes. Across all 70 request samples,
main-thread request decoding is at most 0.090 ms. Async storage and native
callback application remain unmeasured; the small synchronous update intervals
do not establish that all native work is small.

Median render phases, milliseconds (five samples per row):

| Entries | Render | Native view | Kotlin decode | State publication |
| ---: | --- | ---: | ---: | ---: |
| 10,000 | Select All callback | 1.981 | 13.796 | 0.287 |
| 100,000 | Select All callback | 25.915 | 134.515 | 0.587 |
| 10,000 | Confirmation dispatch | 2.262 | 15.273 | 0.498 |
| 100,000 | Confirmation dispatch | 39.207 | 69.925 | 1.115 |
| 10,000 | Equal confirmation replay | 7.832 | 70.227 | 1.389 |
| 100,000 | Equal confirmation replay | 61.037 | 192.500 | 5.594 |

At 100k equal publication ranges 3.936–35.825 ms and decoding
129.063–339.958 ms. All ten equal snapshots retain the original state instance;
all other 70 render samples publish changed instances. This observes the
previously missing equal case without supporting a different equality policy.
Replay follows confirmation assertions and diagnostic serialization, shares the
same process/GC history and does not trigger composition. It is not a paired
decode microbenchmark against confirmation dispatch. Large-ID decoding remains
a measurable target; investigate its allocations and use controlled comparisons
before changing the generated codec/transport.

Median action-to-idle / synchronous call, milliseconds:

| Action | 10,000 entries | 100,000 entries |
| --- | ---: | ---: |
| First 100 rows | 182.868 / 1.082 | 281.412 / 1.127 |
| Append 100→200 | 214.358 / — | 346.951 / — |
| Jump between retained rows | 280.822 / — | 282.236 / — |
| Select every matching ID | 181.112 / 30.145 | 1,132.142 / 25.667 |
| Freeze/render confirmation | 314.909 / 18.276 | 405.611 / 119.095 |
| Diagnostic equal replay | 98.511 / 79.078 | 323.533 / 292.832 |

No equal replay produces an observed main-window frame. Across all actions,
111 observed frames exceed the fixed 16.667 ms reference, with zero reported
drops; separate dialog-window limits still apply. Maximum sampled Java/native
allocated heap/RSS is 19.303/22.658/246.016 MiB at 10k and
52.873/28.394/285.488 MiB at 100k. These are independent process high-water
observations, not additive components, exact peaks or per-action allocations.
The replay retains the existing state and allocates a fresh decoded view that
is then rejected by the unchanged state policy; no retained-byte saving is claimed.

All [80 render rows](measurements/vocabulary-request-android-2026-10-31.csv),
[70 request rows](measurements/vocabulary-request-android-2026-10-31-requests.csv),
[60 action rows](measurements/vocabulary-request-android-2026-10-31-actions.csv)
and [111 raw frame durations](measurements/vocabulary-request-android-2026-10-31-frames.csv)
validate: exact source/count/index/payload and published-instance expectations,
five iterations per action/size, finite nonnegative timings, absent callback
native intervals, exact frame summaries, positive memory observations and zero
drops. Payloads, fixture sizes and unchanged database hashes match the original
section. No analyzer/reading effects, integrity/FK, exact IDs, handle closure
and scratch-directory cleanup pass in both processes. ADB confirms only the
existing sudachi cache remains in no_backup; the task-started AVD was stopped.

PASS: debug/test APKs, strict Android lint, repository Kotlin formatting/diff;
21 ordinary regressions plus default assumption skip (22 reported, 240.384 s).
CoreSessionTest covers observer removal/thread guards and equal replay identity,
no requests and no onViewChanged notification. Separate 10k/100k profiles pass
in 38.092/60.011 s. Production source baseline: `cf63d10`; native/binding/assets
reuse Gradle outputs and the x86_64 JNI hash matches the earlier sections.
Debug APK SHA-256:
`f75c30ead000f74f40e0b011edf56f8cbc25fab8f47fda7c686c8df664a656d4`;
test APK:
`43c05d5bdd113d62622d84d5e9c80f54a770fa7ffb0a84b6d36251e9e6bf0221`.
Logs: `/tmp/yomoku-request-{android,format-check,regressions,10000,100000,validation}.log`.
Initial sandbox Gradle lock-socket access required approved rerun; no source/test
failure remains. No Rust/schema/dependency/wire or product event-policy change.
Host Rust/Clippy, independent JNI/codegen/Full-loader, optimized/tablet/dark/
continuous-fling/heavy-history/diverse-book/fully loaded lists, exact peak/PSS,
process-death and physical arm64 were not rerun or claimed. Broader tasks remain.

## Controlled ID-list decoding and allocation comparisons — 2026-10-31

`VocabularyDecodingTest` isolates one generated bincode `List<VocabularyId>`.
It opens no database/native handle, loads no analyzer, dispatches no event and
publishes no Compose state. All candidates consume the same immutable generated
bytes and return full `VocabularyId` lists. This is not a complete snapshot,
selection action, callback or composition benchmark. A frozen snapshot contains
both selection and confirmation lists; do not multiply these timings to predict
its decode time or compare them directly to the preceding full-render profiles.

Four candidates: `generated` calls the production generated list/element helper;
`capacity` uses the same higher-order loop and generated element decoder with
validated initial ArrayList capacity; `buffered` and `buffered_capacity` read
fixed-width little-endian longs through ByteBuffer into growing/preallocated
lists. The latter pair also omit generated per-ID container calls and generic
element dispatch: their difference from generated is not solely byte reading.
Candidates are test-only and cannot replace arbitrary/nested schema decoding.
Lengths are checked against available fixed-width bytes before preallocation.
Regular tests cover all IDs at 0/1/2/100/10k/100k, gapped IDs, >2^53, signed wire
extremes, every truncation of a three-ID payload, invalid/oversized lengths and
trailing bytes. Codec signed-range coverage does not change valid domain IDs.

Two separate instrumentation processes per size, debug API-36 x86_64 medium_phone
(1080×2400), same build fingerprint/environment as earlier sections. Run order:
10k/1, 100k/1, 100k/2, 10k/2; no competing builds/tests/profilers. Each process
does four full warmup rounds and eight measured forward/reverse rotated rounds;
each candidate occupies every order position twice. Decode runs on the main
thread. Timing excludes scheduling, counter reads, assertions, re-encoding and
output. Four results remain alive until round-end validation; re-encoding then
shares GC history with the next round. No forced GC. Balanced ordering reduces
order bias without removing JIT/scheduling/GC/debug/emulator effects.

Median decode milliseconds (eight samples per candidate/process):

| IDs | Process | Generated | Capacity | Buffered | Buffered + capacity |
| ---: | ---: | ---: | ---: | ---: | ---: |
| 10,000 | 1 | 4.551 | 4.379 | 0.648 | 0.526 |
| 10,000 | 2 | 4.199 | 4.136 | 0.562 | 0.498 |
| 100,000 | 1 | 25.502 | 25.341 | 3.243 | 3.303 |
| 100,000 | 2 | 23.098 | 23.795 | 3.694 | 3.029 |

Capacity-only timing ranges overlap generated in every process. The combined
buffered/preallocated candidate ranges 2.583–46.911 ms and 2.473–39.388 ms at
100k: lower medians do not remove long samples. At 10k, median reported allocated
bytes for generated/capacity/buffered/buffered+capacity are respectively
888,832/692,224/397,312/200,704 in both processes, with zero reported GC during
sample intervals. At 100k these medians are 7,880,704/6,819,840/3,035,136/2,002,944
(process 1) and 7,827,456/6,852,608/3,035,136/2,002,944 (process 2).

These ART counter deltas are approximate and process-wide, include counter-read
allocations/unrelated threads, and may reflect allocations later than the sample
that produced them. Some 100k samples report zero bytes and others up to
20,025,344; GC count/time deltas also include concurrent collector work. They
are not exact per-decode allocations, synchronous GC pauses, retained bytes,
PSS or peaks. Supported semantics verified against the primary
[Android Debug API](https://developer.android.com/reference/android/os/Debug#getRuntimeStat(java.lang.String)).

Bytecode inspection with the pinned JDK `javap -c -p` on compiled
`com/novi/serde/BinaryDeserializer.class` shows `RangesKt.until` and
`RangesKt.step` in `deserialize_u64` and `getLong`, before their eight-byte loops.
The pinned facet_generate 0.19.1 runtime uses `0 until 64 step 8` in both places;
`deserialize_i64` calls the unsigned method. These are concrete allocation
sites to remove with a separately verified scalar-loop change. The combined
buffered comparison does not isolate their contribution; do not claim the
entire timing/allocation difference comes from progressions, or change the
production wire/equality policy on this evidence.

All [128 raw rows](measurements/vocabulary-decoding-android-2026-10-31.csv)
validate exact sizes/payload lengths, eight samples/candidate/process, balanced
positions and finite nonnegative timings/counters. Every timed result passes
full-ID equality and generated-byte round trip after the round; original input
bytes remain unchanged. Payload bytes/SHA-256:

- 10k: 80,008 / `a3ef41217038ab501cc25165e2601d8a21f6d0217430bd834dd167b30b1781e0`.
- 100k: 800,008 / `4c743bd42937a700214622ce01f6ac07f82a5f7b415a9bf0a1a0ad1330ffad23`.

PASS: debug/test APKs, strict lint, repository Kotlin formatting/diff; six passing
codec/selection/CoreSession regressions plus default assumption skip (seven
reported, 0.720 s). Four opt-in processes pass in 0.671/2.156/2.135/0.651 s
(10k/1, 100k/1, 100k/2, 10k/2). Production baseline `fb5fc16`; debug APK unchanged:
`f75c30ead000f74f40e0b011edf56f8cbc25fab8f47fda7c686c8df664a656d4`;
test APK `a5132a9ed3430f99507131a49a27b97b19bfbd025e984b2a590425ebc212ed19`.
Native/bindings/assets reused Gradle outputs; no runtime/schema/dependency change.
Logs: `/tmp/yomoku-decoding-{android,format-check,regressions,10000-1,10000-2,
100000-1,100000-2,validation}.log`; bytecode `/tmp/yomoku-decoding-bytecode.txt`.
Initial sandbox Gradle socket startup needed approved rerun; no source failure.
Host Rust/Clippy/JNI/codegen, full Compose/native profiles, optimized/arm64 and
broader history/lifecycle/peak-memory checks were not rerun. Broad tasks stay open.


## Scalar-loop integration and frozen-snapshot comparisons — 2026-10-31

The reproducible Rust Kotlin generator now adapts exactly the pinned
facet_generate 0.19.1 unsigned/signed 64-bit reader loops to scalar `while` loops.
It fails if either complete legacy loop is missing, duplicated or changed.
All 17 generated Kotlin files match repeat generation; comparison with the
previous output changes only those two loops in BinaryDeserializer. License,
byte-at-a-time reads, bounds checks, partial-read offsets, wire representation,
container budgets, generated list helpers and serialization remain unchanged.
`javap -c -p` finds no RangesKt calls in production BinaryDeserializer; both
legacy test loops still call until/step. This establishes removal of those
allocation sites, not allocation-free complete decoding.

The shared host/Android oracle retains the legacy scalar loops in a test-only
BinaryDeserializer subclass, with the pinned bincode float/length/variant methods.
Both paths use the same generated element/list/nested ViewModel codecs. Scalar
checks include each bit position, 1,000 seeded random patterns, signed extremes,
>2^53 values, both Int.MAX_VALUE length boundaries, signed/unsigned 128-bit
values, raw floating-point zero/infinity/NaN payloads, every 8/16-byte truncation
and prefixed offsets. Generated snapshots cover 0/1/2/100/10k/100k IDs, Unicode,
unsigned maximum counts and exact byte round trips; all truncations of a small
nested snapshot and trailing bytes produce matching errors and offsets.

`ScalarDecodingTest` compares unchanged generated growing-list decoding against
that legacy scalar oracle. Its second shape is a complete synthetic frozen
ViewModel containing 100 loaded Vocabulary rows, the entire sorted selection and
the entire confirmation ID list. Both use gapped IDs ending with >2^53 and
Long.MAX_VALUE. The two shapes are measured separately, list first. The full
snapshot measures decoding only, not an actual native callback, publication,
equality check, composition or user interaction. No database or analyzer opens.
It is not the prior 1,000-row production profile and cannot directly establish
before/after application speed or predict it by multiplying list timings.

Each fresh instrumentation process runs four warmup and eight measured rounds
per shape, alternating the two decoder orders (four measured appearances in
each position). Decoding runs on Android's main thread; scheduling, ART counter
reads, hashes, assertions, serialization and output are outside timing. Both
results remain live until round-end equality/byte verification; original inputs
remain immutable. No forced GC. Re-encoding and prior rounds affect subsequent
heap/GC state. The two decoder classes also differ in dispatch/JIT opportunities;
these debug-emulator comparisons do not attribute the full difference solely to
progression allocations. Approximate process ART allocated-byte deltas can lag,
include counter reads/unrelated threads and report zero; they do not measure
per-thread allocations, retained bytes, GC pauses or peaks.

Environment: the existing isolated API-36 x86_64 medium_phone AVD, 1080×2400,
KVM/two cores/swangle/Vulkan disabled; build fingerprint
`google/sdk_gphone64_x86_64/emu64xa:16/BE2A.250530.026.D1/13818094:user/release-keys`.
No competing builds/tests/profilers during measurement. Source baseline
`7475f27`; debug APK SHA-256
`983eaf50d5ce139b1cdeaa98a9b4126b2945fca5ac5666d3935e17c83a8b2504`;
test APK `d85484e0636f9a8ab359d67f3fdf5c1fce37993b93c07eefdec8def6a6cdd51d`.


Four fresh processes ran in order 10k/1, 100k/1, 100k/2, 10k/2. Median decode
milliseconds (eight samples per decoder/shape/process):

| IDs | Process | List legacy | List scalar | Frozen legacy | Frozen scalar |
| ---: | ---: | ---: | ---: | ---: | ---: |
| 10,000 | 1 | 7.599 | 4.975 | 6.483 | 4.629 |
| 10,000 | 2 | 4.459 | 2.898 | 6.442 | 4.836 |
| 100,000 | 1 | 23.685 | 15.379 | 55.207 | 34.515 |
| 100,000 | 2 | 23.322 | 15.462 | 53.924 | 35.366 |

The 100k frozen scalar ranges are 33.302–36.810 and 30.729–42.865 ms, versus
legacy 52.591–58.667 and 50.543–55.819 ms in these processes. At 10k, list
ranges overlap in process 1 and frozen ranges overlap in process 2; retain the
raw long samples and avoid a universal latency claim. At 100k the reported ART
allocated-byte medians are legacy/scalar 7,827,456/3,043,328 for lists in both
processes, and 15,839,232/6,205,440 then 15,826,944/6,152,192 for frozen snapshots.
Some frozen scalar intervals report zero and others up to 22,163,456 bytes;
these approximate process counters do not establish exact allocation savings.

All [128 raw rows](measurements/vocabulary-scalar-android-2026-10-31.csv) validate
sizes, payload lengths, eight samples per decoder/shape/process, balanced order
positions and finite nonnegative timings/counters. Every timed result passes
full equality and generated-byte round trip; inputs stay exact. Payload bytes
and SHA-256 (identical across processes):

- 10k list: 80,008 / `a3ef41217038ab501cc25165e2601d8a21f6d0217430bd834dd167b30b1781e0`.
- 100k list: 800,008 / `4c743bd42937a700214622ce01f6ac07f82a5f7b415a9bf0a1a0ad1330ffad23`.
- 10k frozen: 164,880 / `2a132e769f4b37fc5055b1006ecb48bdf214e12f36c32d67206e72cf2610da91`.
- 100k frozen: 1,604,880 / `c0b1792aab81aee88eda1b9a6c071a3d9be2315bd4e3f27179723310433e4131`.

PASS: pinned-loop generator guard, targeted strict generator Clippy, Rustfmt,
repository/shared Kotlin formatting, full real-Full host JNI/codecs/Unicode
fixtures, latest shared JVM cases, reproducible generation, debug/test APKs,
strict Android lint and diff checks. Eight scalar/codec/selection/session cases
plus two default opt-in skips pass (10 reported, 1.226 s). Eight production
Vocabulary/Statistics/effective-reader cases pass (249.438 s), including exact
frozen retries, persistence, recreation, scope changes, Library and furigana.
Four measurement processes pass (1.441/3.433/3.416/0.922 s in run order).
Initial sandbox Gradle lock-socket startup required approved rerun; existing
nonfatal host teardown diagnostics remain, with no source/test failure.

Commands: DEVELOPMENT.md, Scalar Kotlin decoder integration. Logs:
`/tmp/yomoku-scalar-{generator-test,clippy,bridge,host,codegen,codegen-diff,android,
format,regressions,production,10000-1,10000-2,100000-1,100000-2,validation}.log`;
bytecode `-bytecode.txt` and `-legacy-bytecode.txt`. The task-started isolated AVD
was stopped; existing ignored image/caches retained. No schema, dependency,
product event, equality, threading or generated wire change. Full workspace
Rust/Clippy, independent Full-loader matrix, full native callback/Compose
profiles, optimized/arm64, diverse/heavy histories, full-load lists, lifecycle/
process-death and exact peak-memory work were not rerun or claimed. Broad tasks
remain open; next measure the actual callback/composition path with this decoder.
