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
