# Reader, import, and SQLite work

The deterministic checks exercise bundled SQLite on synthetic data. Registration
queries use 4,096 vocabulary rows with tied timestamps. The EPUB delete test
checks the inline-image foreign key while deleting an unreferenced cover from
one of 20 imported books. Each comparison uses the same rows before and after
the index is present.

| SQLite statement | Before VM steps | After VM steps |
| --- | ---: | ---: |
| Registration, newest first | 40,581 | 5,919 |
| Registration, oldest first | 40,306 | 5,919 |
| Selective prefix, either registration order | 697 | 697 |
| EPUB cover asset delete | 86 | 29 |

| Work per operation | Before | After |
| --- | ---: | ---: |
| Full layouts per warm forward turn, core and storage combined | 3 | 0 |
| Full document loads per warm forward turn | 3 | 1 |
| Analysis decodes per warm page projection | 1 | 0 |
| Analysis decodes per warm word lookup with A/B available | 3 | 0 |
| Source SHA-256 hashes per new file import | 2 | 1 |
| Parses per new file import | 2 | 1 |
| Rust import buffer clones when accepting a typed event | 1 | 0 |

The before reader and import counts follow the call paths at `be612f1`. The
after reader counts are checked on a long TXT and a two-chapter EPUB, and the
word test checks Mode C and both A/B snapshots across repeated lookups and
invalidation. The import counts refer to the source file. Persisted canonical
text and EPUB asset digests are computed separately.

## Android release timings

Both release APKs ran sequentially on the same accelerated x86_64 AVD, Android
17 API 37, build `CE2A.260420.051.A3`. The original release APK was saved before
building the changes. Each metric has one warmup and five measured repetitions.
The tables show minimum, median and maximum wall-clock milliseconds.

The reader fixture hashes matched across both runs. The TXT had one chapter
with 56,000 characters. The EPUB had two chapters with 111,998 characters in
total. Forward timing covers reservation, commit and the returned reader page.
The benchmark loads the next page's tokens between forward samples. Page and
word timing cover the native dispatch, SQLite worker and returned snapshot.

| Fixture | Operation | Before (ms) | After (ms) |
| --- | --- | ---: | ---: |
| Long TXT | Warm forward | 68.894 / 77.633 / 94.622 | 2.917 / 3.634 / 4.627 |
| Long TXT | Page tokens | 28.474 / 33.425 / 49.048 | 1.281 / 2.266 / 2.540 |
| Long TXT | Selected word | 148.198 / 170.813 / 237.193 | 2.446 / 2.616 / 2.781 |
| Long TXT | Library distribution | 38.023 / 75.611 / 141.750 | 14.807 / 15.021 / 16.495 |
| Two-chapter EPUB | Warm forward | 44.219 / 47.371 / 56.623 | 4.559 / 4.747 / 6.367 |
| Two-chapter EPUB | Page tokens | 33.607 / 35.981 / 39.331 | 0.844 / 0.873 / 1.021 |
| Two-chapter EPUB | Selected word | 141.675 / 170.332 / 189.062 | 1.995 / 2.217 / 4.365 |
| Two-chapter EPUB | Library distribution | 48.054 / 49.008 / 54.431 | 27.891 / 28.577 / 29.265 |

The import transport fixture contained 8 MiB of repeated byte values 0–255.
It reaches the typed EPUB codec and JNI, then fails EPUB validation. The
encoding sample measures main-thread work in the original APK and worker work
in the updated APK. The JNI sample measures the synchronous native update on
main in both versions. URI acquisition, EPUB validation, and the asynchronous
storage result occur outside the measured intervals.

| Import transport operation | Before (ms) | After (ms) |
| --- | ---: | ---: |
| Typed event encoding | 297.945 / 454.953 / 779.064 | 95.862 / 123.541 / 246.242 |
| JNI submission | 29.359 / 43.620 / 194.774 | 18.156 / 20.117 / 40.525 |

The reader harness uses `yomokuMeasure=true` and `yomokuMeasureIterations=5`.
The transport harness uses `yomokuMeasureImport=main` for the original APK and
`yomokuMeasureImport=worker` for the updated APK.
[Reader harness](../../android/app/src/androidTest/java/app/yomoku/EffectiveReaderPerformanceTest.kt),
[import harness](../../android/app/src/androidTest/java/app/yomoku/FileImportPerformanceTest.kt).
