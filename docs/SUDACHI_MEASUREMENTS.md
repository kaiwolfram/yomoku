# Sudachi Full Android measurements

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

APK decompression/private-file extraction, reclamation under memory pressure, combined
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

Remaining gate: implement/measure on-demand private extraction with verified
immutable publication and failure recovery; measure combined app memory and
lifecycle/low-memory behavior; execute on physical arm64. Production must never
truncate or rewrite a live backing file. Keep one reusable worker-owned analyzer,
and retain safe owned loading for uncontrolled paths. The overall storage-viability
task remains incomplete until these checks are resolved.
