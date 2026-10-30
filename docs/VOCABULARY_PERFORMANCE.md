# Vocabulary host measurements

Measured 2026-10-30 with the opt-in `yomoku-ffi` example `measure_vocabulary`.
It creates a uniquely named scratch directory/database and accepts only fixture
sizes, never a user database or input file. All fixture writes use production
registration, analyzed-paste, completion, position and bulk-status APIs. Normal
exit removes only its own directory. No migration, dependency, wire-format or
product-behavior change is introduced.

## Method

- Fedora x86_64, Linux `7.2.12-200.fc44.x86_64`, AMD Ryzen 5 7533HS,
  Rust `1.99.1`, release optimization, bundled SQLite `3.53.2`.
- Fixtures contain 1,000 / 10,000 / 100,000 distinct synthetic surfaces
  `語000000` onward, registered at the same date. A synthetic B analysis links
  the first 25 surfaces to their own dictionary forms, twice per page. It does
  not invoke Sudachi; analyzer/dictionary loading during bridge reads panics.
- Each fixture commits 1,000 rereads, giving 50,000 occurrences concentrated on
  those oldest 25 entries. Ten manual status rounds rotate five disjoint ID
  groups through all statuses. Final counts are evenly distributed. There are
  10,825 / 108,025 / 1,080,025 status events, including registrations/completions.
  Database sizes are 4,681,728 / 12,828,672 / 96,768,000 bytes.
- One excluded warmup, then seven timed samples per metric. Repository calls
  include output destruction. All four sorts read first/last 100-row windows
  and the literal `語0000` prefix (100 matches); selection captures every ID;
  statistics projects the default last-year timeframe from the full history.
- FIFO metrics use the effective-reader bridge factory and production native
  worker/retained core. First-page samples navigate away/back to invalidate rows;
  Select All captures unloaded matches. Timing starts immediately before native
  update and ends at callback entry, excluding host wake-up/assertions/view reads
  and input event encoding. Only Render effects cross the serialized shell codec.
- Projection/encoding separately measures `bridge.view()` with 100 or 1,000
  retained rows, all IDs selected and an unexecuted frozen status prompt.
  View decoding uses Rust's Bincode codec, **not Kotlin/JNI/Compose**. Confirmation
  encoding measures the public Confirm event without applying a mutation.
  The Library has not been loaded into the retained model.
- Setup, integrity/FK checks, assertions and file comparison are outside timing.
  An exact streaming comparison with a scratch baseline copy verifies unchanged
  database bytes across measurements, using two 64 KiB buffers. Allow disk space
  for both copies. Scratch cleanup and byte preservation also have a data-free test.
- Before/after executables run sequentially without competing builds/tests.
  CPU frequency, OS caches and host scheduling are uncontrolled; these are warm
  host measurements, not latency thresholds or device responsiveness guarantees.

## Change and evidence

Newest formerly projected encounter summaries while considering candidates for
the registration-date sort. It now selects LIMIT/OFFSET candidates first, then
projects their summaries and explicitly orders the returned rows in the same
read transaction. Count, filters, ties, paging and visible-history validation are
unchanged. Frequency/LastSeen and alphabetical execution remain unchanged.
No index or cached counter is introduced; sorting registration fields still
examines matching entries, and visible entries still pay for their own history.

SQLite documents this deferred-work pattern in its
[subquery optimization guide](https://www.sqlite.org/optoverview.html#using_co_routines_to_defer_work_until_after_the_sorting).
A regression uses the documented [VM step counter](https://www.sqlite.org/c3ref/c_stmtstatus_counter.html)
to require identical Newest-page work before/after 200 offscreen original/effective
occurrences. The last page still reports their exact count/date and rejects invalid
provenance. Existing all-sort/filter/Unicode/read-only/paging tests remain in force.

Median milliseconds (seven samples after warmup):

| Entries | Newest first before → after | Newest last before → after | FIFO first-page callback before → after |
| ---: | ---: | ---: | ---: |
| 1,000 | 12.784 → 0.408 | 12.561 → 12.023 | 12.829 → 0.539 |
| 10,000 | 20.928 → 2.830 | 23.633 → 17.499 | 20.742 → 2.826 |
| 100,000 | 108.177 → 30.456 | 159.038 → 83.966 | 111.877 → 30.455 |

Unchanged paths remain material. Final medians:

| Entries | Frequency first | LastSeen first | Select All repository / callback | Year statistics | Prompt view encode, 1,000 rows |
| ---: | ---: | ---: | ---: | ---: | ---: |
| 1,000 | 12.479 | 12.510 | 0.088 / 0.147 | 10.489 | 0.059 |
| 10,000 | 21.416 | 23.904 | 0.706 / 0.746 | 96.554 | 0.092 |
| 100,000 | 116.290 | 116.310 | 8.116 / 8.046 | 1043.890 | 1.861 |

The 100,000-ID prompt view is 1,604,801 bytes with 100 rows, or
1,646,201 bytes with 1,000 rows; confirmation alone is 800,028 bytes.
Selection and frozen confirmation both retain the exact IDs. These payloads
remain linear in selection/loaded-row count; no wire or retention optimization
is made here. The speedup applies to Newest when most matching history is
offscreen. A prefix including those old entries still pays their history cost.
Unchanged metrics vary between runs; no general improvement is claimed for them.

All 132 before/after metric rows, including min/max and raw samples, are retained
in [the CSV](measurements/vocabulary-host-2026-10-30.csv). Setup is excluded;
the final 100,000-entry setup took 35.2 seconds. One million status events still
make year-statistics replay costly and deserve separate profiling.

## Reproduce

```sh
export PATH=/home/kai/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin:$PATH
cargo build -p yomoku-ffi --example measure_vocabulary --release --locked --offline
target/release/examples/measure_vocabulary 1000 10 1000 7
target/release/examples/measure_vocabulary 10000 10 1000 7
target/release/examples/measure_vocabulary 100000 10 1000 7
cargo test -p yomoku-ffi --example measure_vocabulary --all-features --locked --offline
cargo test -p yomoku-storage vocabulary_browser --locked --offline
```

Arguments are ENTRIES (100–100,000), STATUS_ROUNDS (0–100), REREADS (0–10,000),
ITERATIONS (1–100). Invalid arguments fail before creating a directory. stdout
is CSV with raw samples and min/median/max milliseconds; stderr contains fixture
counts, database/wire sizes and correctness guards. Use `TMPDIR` for a different
scratch filesystem. These runs use the host's RAM-backed `/tmp`.

Baseline production is `f73b85db3a41e4c39b7d3588cd34f479a2f42f0e` plus this
same measurement harness. Baseline executable SHA-256:
`ec83a51bfb23f29e1dbfaba6cf42b85994bc75a7446c3a52674fa379b9be3cc1`.
After executable SHA-256:
`e6417649e2efa7ed7dfe5a2a20f9c4586396dee957a52d1f8e5e0d724295437f`.
For comparisons, retain separately built executables before running either;
do not run builds/tests/profilers during timings.

Peak allocations/RSS, fully loaded 100,000-row views, dispersed encounter dates,
diverse many-book/image-heavy Libraries, cold storage, Android callbacks/decoding,
actual Compose frames, process death and physical arm64 remain unmeasured here.
The broader benchmark and device-matrix tasks remain unchecked.
