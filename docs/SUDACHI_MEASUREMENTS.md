# Sudachi Full Android baseline

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

APK decompression/private-file extraction, mapped storage/reclaimability, combined
JVM/Compose memory, app lifecycle/low-memory behavior, physical arm64 execution and
large-book imports remain unmeasured. Both Android probe ABIs link, but only x86_64
has executed. This baseline does not establish production memory viability or
finish the loading-strategy gate before runtime integration.
