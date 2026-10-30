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
make year-statistics replay costly and motivated the follow-up below.

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

## Follow-up: statistics SQL versus pure replay

The same host/fixture harness now reports four additional metrics. Production
continues streaming events; only the diagnostic harness retains a predecoded
event vector. Memory usage of that vector is not a production-memory measurement.

- `status_sql_legacy_step_only`: prepare, step every row and destroy the frozen
  former ordered-UNION query, without decoding column values in Rust.
- `status_sql_split_step_only`: the same row stepping over the exact shared
  production link/event SQL files, in one explicit read transaction.
- `status_rust_replay_predecoded`: construct the pure builder, clone its links,
  push predecoded events, finish and destroy the output. Extraction is excluded.
- `repository_status_timeline`: the complete validated public repository call,
  including SQL, column decoding, cause/version/provenance checks and pure replay.
  The existing `repository_statistics_year` adds reading/date bucket projection.

Each timed operation has an excluded warmup and seven samples. The layers are
independent measurements: they have different allocations and cache behavior,
and must not be summed/subtracted as an exact CPU decomposition. Both the old
and new public timeline outputs equal the isolated replay on these fixtures.
Existing byte/FK/integrity/cleanup guards cover the new diagnostics. Timings use
sequential release executables with no overlapping builds/tests, RAM-backed
`/tmp`, warm and uncontrolled OS caches and host scheduling.

The ordered UNION performed two sorts to deliver links before events. Load
links first, then stream events in primary-key order in a single deferred read
transaction. Both internal helpers require a transaction; calendar statistics
reuse their existing snapshot and the standalone timeline opens its own.
All column/version/cause/provenance/chain/date validation remains. No cached
counter, index, migration, core/FFI/Android change or product behavior is added.

SQLite's [query-plan documentation](https://www.sqlite.org/eqp.html#temporary_sorting_b_trees)
describes temporary sorting; its [transaction documentation](https://www.sqlite.org/lang_transaction.html#read_transactions_versus_write_transactions)
establishes the snapshot guarantee across statements. A new WAL-only regression
commits vocabulary erasure/recreation between link and event reads and requires
the original generation's exact result. Another requires zero statement sort
operations and read-transaction cleanup after failures in either query. Existing
corrupt-history tests now also require the standalone transaction to be closed.

At 100k entries / 1,080,025 events / 25 links, the frozen query executes
65,882,372 VM steps and two sorts; split reads execute 39,960,891 steps and zero
sorts over the same 1,080,050 rows. Full year-statistics median falls from
944.157 to 542.853 ms (42.5%); the standalone timeline falls from 943.659 to
538.206 ms. After-run layered medians are 589.119 ms for legacy stepping,
182.040 ms for split stepping and 96.999 ms for predecoded replay. These SQL
measurements identified the optimization; the remaining decoding/validation and
replay costs remain material. Frequency/LastSeen queries are unchanged.

The 10k fixture has 108,025 events and 25 links. Its public timeline median falls
85.287→51.816 ms and full year statistics 86.947→53.254 ms. After-run layered
medians are 49.640 ms legacy stepping, 17.351 ms split stepping and 9.291 ms
predecoded replay. All 104 metric rows from the four before/after runs (including
unchanged Vocabulary/FIFO/view metrics) are retained in
[the follow-up CSV](measurements/statistics-host-2026-10-30.csv). Seven raw samples
per row validate against their reported min/median/max. The no-status-round,
no-reread 100-entry/even-two-sample smoke also passes; these extra smoke samples
are not included in the comparison CSV.

Baseline production is `e04c85cc49f22ced0b1d2d3b56df93af184689c1` plus the new
diagnostics, before the storage refactor. Executable SHA-256:
`fd03fb4965c94ccc780493fd0819cbc4c96cb80c3523136f1eab21f2d55edf6a`.
After executable SHA-256:
`7bb2fb5bd919e2f4228011fe5bc19426f57ea29639a8ae937e5e90fba73fb9a2`.
Use the existing reproduction commands and arguments; diagnostics run before
the other repository/FIFO/view metrics. Neither host timings nor synthetic
fixtures establish cold-storage, diverse-book, peak-memory, Compose/frame,
process-death or physical-arm64 performance.
