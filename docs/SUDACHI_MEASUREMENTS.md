# Sudachi Full Android measurements

Current scope (user decision, 2026-10-24): physical arm64 testing is deferred to
the user once the app is usable. It does not block production integration or
emulator development. The remaining integration gate is emulator pressure/
reclamation evidence and the production worker lifecycle decision. Older gate
descriptions below record their original scope; no hardware validation is claimed.

Measured 2026-10-23, using the standalone `yomoku-analysis` example
`android_probe` and `tools/measure-sudachi-android.py`. This executes the real
checksum-verifying owned-byte constructor and adapter on Android, outside the
app/runtime/FFI. Engine, Full 20261015 and all checksums remain pinned as described
in [third-party notes](../third_party/sudachi/README.md).

## Environment and method

- API 36 `medium_phone` x86_64 emulator, two virtual CPUs, KVM/swangle;
  kernel 6.6.66-android15-8; total memory 2,532,300 KiB. Host is an AMD Ryzen 5
  7533HS. Build fingerprint:
  `google/sdk_gphone64_x86_64/emu64xa:16/BE2A.250530.026.D1/13818094:user/release-keys`.
- Rust 1.99.1, NDK 30.0.16248370/API 29 linker. Default Cargo debug and release
  profiles, without application profile changes or added dependencies.
- Three fresh processes per profile. The filesystem cache was **not flushed**;
  these are process initialization measurements, not cold-storage measurements.
  Build/push and other native builds/tests did not overlap timed probe execution.
- The prepared 330,506,339-byte Full file is pushed to a unique directory under
  `/data/local/tmp`. Its host SHA-256 and the actual bytes loaded by each Android
  constructor are checked against the pinned checksum. No APK is installed.
- Airplane mode enabled, Wi-Fi and mobile data disabled and checked before runs.
  Prior radio settings (off/on/on) were restored and verified, and probe files
  removed. The existing app and its private data were untouched.
- Initialization includes file read, SHA-256 and dictionary construction. Timed
  analysis calls include tokenizer creation, Mode B analysis, owned output and
  anchor validation. First analysis also includes lazy Japanese-script regex setup.
- Memory fields come from `/proc/self/status`: RSS and anonymous/file resident
  pages are snapshots; VmHWM is the process resident high-water mark, **not PSS**.
  No swap was reported for any measured probe process.

## Results

Values below are in process order. Each page upper median uses 20 calls; each long
chapter median uses three. Timings are milliseconds, memory is KiB.

| Metric | Release processes 1 / 2 / 3 | Debug processes 1 / 2 / 3 |
| --- | --- | --- |
| Initialization | 1199.002 / 1029.415 / 1026.122 | 12686.323 / 12365.312 / 12305.306 |
| RSS after initialization | 329664 / 329780 / 329788 | 332576 / 332736 / 332592 |
| Anonymous RSS after initialization | 325312 / 325300 / 325308 | 325536 / 325568 / 325552 |
| Page median | 0.477 / 0.479 / 0.482 | 3.697 / 3.694 / 3.672 |
| Long unbroken median | 23.374 / 26.897 / 23.985 | 167.228 / 171.731 / 168.637 |
| Peak RSS through long chapter | 341484 / 341668 / 341540 | 344448 / 344480 / 344464 |

The page fixture repeats `猫と犬。東京都と夏目漱石。食べる。` 30 times:
510 Unicode scalars, 1,530 UTF-8 bytes, 330 output tokens. The unbroken fixture
repeats `猫と犬` 8,000 times: 24,000 scalars, 72,000 bytes, 16,002 output tokens.
These synthetic cases establish a repeatable baseline, not book-wide throughput.
Output allocations are dropped between calls; allocator retention contributes to
the memory snapshots. After dropping the analyzer, release RSS was approximately
13 MiB and debug approximately 15 MiB; VmHWM naturally retains the previous peak.

All six processes passed real A/B/C golden segmentation, mixed Japanese/Latin/
numbers/punctuation/emoji, rare-kanji OOV handling, half-width normalization,
supplementary/combining characters and embedded NUL preservation. Core validates
exact source coverage and byte anchors for every result, including long input.

Raw reports are ignored local artifacts:
`.local/test-reports/sudachi-android/20261023T185613156613Z-x86_64-release.txt` and
`20261023T185653839458Z-x86_64-debug.txt` in the same directory. Earlier failed
reports remain there for diagnosis; they are not included in the results above.

## Loading implications and remaining measurements

Owned Full requires approximately 318 MiB of anonymous resident memory after
initialization, before Android app overhead. Keep it as a measured correctness
baseline; compare a verified immutable file mapping before choosing production
storage. Any private dictionary extraction must occur on demand on an I/O worker,
verify before publication, and never replace bytes while an analyzer uses them.
Keep a single reusable analyzer per worker rather than loading per page or eagerly
on Library startup. These are integration constraints, not implemented app behavior.

At this baseline, APK decompression/private-file extraction, reclamation under memory pressure, combined
JVM/Compose memory, app lifecycle/low-memory behavior, physical arm64 execution and
large-book imports remain unmeasured. Both Android probe ABIs link, but only x86_64
has executed. This baseline does not establish production memory viability or
finish the loading-strategy gate before runtime integration.

## Verified owned versus mapped comparison

Measured 2026-10-23 with probe version 2, the same engine/data/toolchain/device
as above, and the driver's `--storage both` option. Three fresh owned/mapped pairs
per profile execute sequentially, owned first in every pair. Neither filesystem
caches nor system memory pressure are controlled. The driver publishes one
dictionary in its unique mode-700 directory, sets the file mode to 400, and never
rewrites it until all processes have exited. Permissions alone do not enforce
immutability against the owner; the driver controls this lifetime. Each mapped
constructor hashes the actual mapping, then transfers that same storage to
Sudachi, with no owned dictionary copy. No APK/runtime/FFI integration occurred.

Values are per storage mode in pair order. Memory is KiB, timings milliseconds.

| Release metric | Owned 1 / 2 / 3 | Mapped 1 / 2 / 3 |
| --- | --- | --- |
| Initialization | 1066.844 / 1027.235 / 1131.230 | 943.940 / 932.408 / 944.444 |
| RSS after initialization | 329912 / 329880 / 329892 | 329856 / 329840 / 329848 |
| Anonymous RSS after initialization | 325304 / 325272 / 325284 | 2560 / 2544 / 2552 |
| File RSS after initialization | 4480 / 4480 / 4480 | 327168 / 327168 / 327168 |
| Page median | 0.483 / 0.477 / 0.503 | 0.474 / 0.482 / 0.488 |
| Long unbroken median | 25.134 / 21.693 / 22.077 | 21.570 / 21.814 / 23.027 |
| Peak RSS through long chapter | 341664 / 341612 / 341632 | 341576 / 341644 / 341600 |

| Debug metric | Owned 1 / 2 / 3 | Mapped 1 / 2 / 3 |
| --- | --- | --- |
| Initialization | 12154.532 / 12193.539 / 12250.057 | 11973.122 / 11969.292 / 12064.654 |
| RSS after initialization | 332528 / 332880 / 332484 | 332532 / 332792 / 332772 |
| Anonymous RSS after initialization | 325488 / 325584 / 325572 | 2804 / 2808 / 2788 |
| File RSS after initialization | 6912 / 7168 / 6784 | 329600 / 329856 / 329856 |
| Page median | 3.695 / 3.702 / 3.736 | 3.686 / 3.750 / 3.725 |
| Long unbroken median | 170.446 / 171.993 / 167.510 | 171.997 / 172.301 / 171.700 |
| Peak RSS through long chapter | 344404 / 344496 / 344348 | 344400 / 344460 / 344516 |

All twelve processes passed the same A/B/C, mixed-script/OOV/normalization/Unicode
and long-input validation, offline, with zero process swap reported. Radios were
restored/verified and temporary files removed after each profile. The first
attempt stopped before analysis because mobile-data settings had not yet applied;
the driver now waits for verified settings, bounded to ten seconds. It also waits
when verifying restoration. No failed run contributes to these tables.

Mapped release initialization uses about 2.5 MiB anonymous memory versus about
318 MiB owned, moving roughly 315 MiB into file-backed residency. Initial total
RSS and peak RSS remain similar because full integrity verification touches all
dictionary pages. These observations support continuing with an immutable mapped
file candidate; they do not prove reclamation, a lower app peak/PSS, or a startup
speedup across hardware/cache conditions. Both loaders retain the same analysis
profile: complete A/B/C output equality, including metadata/anchors, is also
checked on host against ten real Full fixtures per loader.

Reports under `.local/test-reports/sudachi-android/`:
`20261023T190838226632Z-x86_64-release-both.txt` and
`20261023T191034834502Z-x86_64-debug-both.txt`.

Private extraction/failure recovery is now measured separately below.
Remaining gate: consume the verified descriptor in an in-app analyzer harness,
measure combined app memory and
lifecycle/low-memory behavior; execute on physical arm64. Production must never
truncate or rewrite a live backing file. Keep one reusable worker-owned analyzer,
and retain safe owned loading for uncontrolled paths. The overall storage-viability
task remains incomplete until these checks are resolved.

## Private APK extraction and verified reopen

Measured 2026-10-23 by `SudachiDictionaryFilesTest` in the installed debug APK on
the same API-36 x86_64 emulator. The production platform helper reads the normal
compressed APK asset into a unique test-private no-backup directory. It uses
64 KiB buffers, checks the pinned length and the complete on-disk SHA-256, syncs
data/permissions, publishes mode-400 bytes by atomic rename and syncs the directory.
The test gets a read-only descriptor at offset zero, then constructs a new helper
and verifies that reopening reuses the same inode after rehashing all bytes.
Both handles close before test-private files are deleted. Production user files
are untouched; viewport/radios are unchanged. The app has no network permission.

| Metric | Full-suite process (ms) | Final focused process (ms) |
| --- | --- | --- |
| Compressed Full extraction, verification and publication | 3272.655 | 2272.742 |
| Verified private-file reopen (no APK decompression) | 631.694 | 689.301 |

These are end-to-end elapsed-realtime readings, not separate decompression/hash/
sync timings. The emulator restarted between the full suite and the final focused
rerun (which verifies a tightened interrupted-input fixture). No caches were
flushed; these are two samples, not a distribution or a cold-storage/physical-arm64
measurement. No analyzer is initialized in this
test; combined JVM/Compose/Sudachi peak/PSS, worker reuse and reclamation/lifecycle
remain unmeasured. The compressed APK still needs approximately 315 MiB extra
private disk space; repair temporarily needs a separate staging copy too.

All seventeen phone instrumentation tests pass, including eight new extraction
tests; the final eight focused tests and lint also pass after the fixture change.
Synthetic fixtures exercise missing/interrupted/short/oversized/wrong-hash
assets, abandoned staging, corrupt/writable cached files, held readers/mappings,
concurrent requests, and actual rename denial by removing directory write access.
Failed publication preserves the prior inode, and retry removes abandoned staging.
This simulates interrupted-process artifacts; it does not inject a power failure,
kill a writer mid-copy, or test cross-process contention/ENOSPC on physical storage.

Reports/logs: `.local/test-reports/sudachi-extraction/{phone,final-focused}/`. Build/test commands
and the focused test selector are in DEVELOPMENT.md. The broader loading/memory
gate stays unchecked until in-app analyzer and lifecycle measurements are complete.

## Debug-app descriptor, worker reuse and lifecycle baseline

Measured 2026-10-23 using `SudachiAppProbeTest` and the `android_app_probe`
cdylib example in the installed **debug** Compose app. The native analyzer is
**release-optimized**, with the same pinned engine/data/NDK and API-36 x86_64
emulator as above. This includes the existing Rust CoreSession, Compose/JVM and
Android instrumentation overhead in the same process. It is not a release-app
or hardware viability result. The debug-only facade/library have no production
startup caller; the production runtime/FFI dependency graph is unchanged.

The test owns a unique no-backup directory and lazily opens the real compressed
APK asset through the verified private publisher. JNI duplicates that exact
descriptor, never reopens a pathname, and the native constructor verifies the
mapped bytes again. Both Kotlin descriptors close before A/B/C or other analysis
runs. Fixed golden modes, OOV/non-Japanese classification, normalization,
supplementary/combining Unicode, NUL, 510-scalar pages and 72 KB unbroken input
pass. Test-private bytes stay immutable until every mapping/worker closes.
The app has no Internet permission; radio/viewport settings were unchanged.

One test ViewModel retains a single lazy executor/analyzer through real Activity
recreation and stop/resume. Its positive generation stays identical and its
initialization count is one throughout; a fresh worker after close gets a new
generation and re-verifies the cached publication. Invalid dictionary load can
retry; second open, invalid fixture, stale generation and foreign-worker calls
are rejected without disturbing the active analyzer. Finishing the Activity
clears the owner and closes native state on its executor. `/proc/self/maps`
confirms the dictionary mapping exists while active and disappears on close;
directory cleanup waits for executor termination. This is a test owner, not the
production EffectRouter/analyzer lifecycle decision.

Two successful processes using the final mapping assertion are reported below:
focused test first, then the complete eighteen-test suite. The second process
has exercised other app flows before the probe; baseline/HWM therefore include
that earlier work. No filesystem caches were flushed or memory pressure injected.
PSS/private-dirty are Android Debug.MemoryInfo snapshots; kernel RSS/HWM and
anonymous/file/swap values are logged separately. PSS is not peak memory.

| Metric | Focused process | Full-suite process |
| --- | --- | --- |
| Extraction + Kotlin verification + native map/verification/init (ms) | 3295.362 | 2964.842 |
| Cached verification + native map/verification/init on fresh worker (ms) | 3305.998 | 1974.219 |
| Page upper median, 20 calls (ms) | 0.516 | 0.501 |
| Long unbroken median, 3 calls (ms) | 23.326 | 25.747 |
| Compose baseline PSS (KiB) | 89573 | 94255 |
| Initialized mapped PSS (KiB) | 412321 | 419105 |
| PSS after page/long analysis (KiB) | 428467 | 435305 |
| PSS after Activity recreation (KiB) | 416400 | 436609 |
| PSS while stopped (KiB) | 412229 | 435481 |
| PSS after resume (KiB) | 412878 | 421307 |
| PSS after first analyzer close (KiB) | 91570 | 99865 |
| Kernel resident HWM at first close (KiB) | 556992 | 574148 |

Timing includes the Kotlin executor/JNI round trip and validation. Full asset
extraction/verification is not separated into individual stages here. Swap is
already present at baseline (22660/22156 KiB) and remains approximately that size;
this is not the zero-swap standalone environment. Closing removes about 315 MiB
of file-backed residency and returns PSS near baseline. Ordinary stop/resume does
not demonstrate kernel reclaim under pressure. Mapping and checksum verification
still touch the complete Full dictionary and produce substantial initial PSS.

Both Android ABIs link/package; only x86_64 executes. Exact API commands are in
DEVELOPMENT.md; preserved XML/logcat reports are under ignored
`.local/test-reports/sudachi-app/{focused-1,full-suite}/`. An initial focused
baseline also passed; a subsequent full run found the new mapping assertion
assumed Context and `/proc` used identical private-directory aliases. Matching
the unique test-directory suffix fixes the assertion; both final runs above pass.
The first connected Gradle command hit sandbox socket restrictions; escalation
resolved them. Test-owned dictionary files were removed; no live published inode
was rewritten/truncated. No tablet/host-JNI/standalone-probe rerun for this slice.

At that baseline the remaining gate was actual memory-pressure/reclamation and
process-death recovery, production worker ownership/reuse, arm64 measurements and a loading
decision based on those results. No production analyzer/import/reader wiring is
enabled by this baseline. The broader loading/memory gate stays unchecked.

## App process-death and cached Full recovery

Measured 2026-10-23 on the same API-36 x86_64 emulator/debug Compose app and
release-optimized native probe. `tools/test-sudachi-process-death.py` launches the
ordinary MainActivity plus a debug-only ADB Activity in the same app process.
The Activity uses a unique no-backup directory per pair, opens the real compressed
Full asset through the private publisher, passes that exact descriptor to JNI,
closes its Kotlin handles and validates A/B/C, mixed/OOV/normalization/Unicode/NUL,
510-scalar pages and 72 KB unbroken input. A completed JSON report confirms the
mapping remains present. The driver checks that PID is still alive, uses
`am force-stop`, and confirms all app processes exit before relaunch or cleanup.
An onDestroy/nativeClose marker must be absent: this exercises actual process
death rather than graceful worker closure. A new PID then repeats analysis using
the cached file; the inode, mtime, size and read-only mode must be unchanged.
Both publisher and mapped native constructor reverify bytes on every open.

| Metric | Pair 1 | Pair 2 |
| --- | --- | --- |
| Initial PID → recovery PID | 5700 → 5778 | 5860 → 5934 |
| Extraction + verification/map/init (ms) | 3854.586 | 3531.609 |
| Cached verification/map/init after death (ms) | 2704.057 | 3011.926 |
| Initial PSS after validation (KiB) | 428590 | 429326 |
| Recovery PSS after validation (KiB) | 428809 | 429693 |
| Published inode before/after | 131097 / 131097 | 131079 / 131079 |

All four processes pass and are force-stopped while mapped, including both
recovered processes. Test directories are removed only after confirmed exit, then
ordinary Library is launched. User data, viewport and radios are unchanged.
Report: `.local/test-reports/sudachi-process-death/20261023T194512308957Z.json`.
Both pairs also pass with the final driver/preflight; its report is
`20261023T194946330811Z.json` in the same directory (not included in this table).
Timings include Kotlin publication verification, JNI and native mapping/checksum;
PSS includes Compose/CoreSession/debug overhead. No filesystem-cache flush,
pressure injection, LMKD kill, mid-extraction kill, power loss or arm64 execution.
Android documents `am start`/`force-stop` in its
[ADB command reference](https://developer.android.com/tools/adb#am).

Process-death recovery is now verified for this measurement harness. Actual
emulator memory-pressure/reclamation and production worker lifecycle remain open;
the loading/memory integration gate remains unchecked. Physical arm64 measurements
are deferred to the user's later usable-app trial, independently of integration.
