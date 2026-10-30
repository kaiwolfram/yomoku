# Vocabulary host measurements

Android JNI/Compose frames and sampled heap/RSS follow-up:
[Vocabulary Android measurements](VOCABULARY_ANDROID_PERFORMANCE.md). Its synthetic
debug-emulator results are separate from the host heavy-history timings below.

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

## Follow-up: Frequency/LastSeen candidate history work

The profiler now compiles the private production page-query builder directly.
For Frequency/LastSeen it prepares/steps/destroys the exact page SQL without
decoding columns, and separately reports statement VM steps and sort operations.
Before timing, every raw SQL field/order equals the validated repository page.
These diagnostics exclude the matching-count query and Rust row validation;
the existing repository measurements include both. They are independent timings,
not values to subtract for an exact cost decomposition.

The former queries projected both encounter summaries and checked provenance for
candidates later discarded by LIMIT/OFFSET. Select candidates using only the
necessary sorting metric, retain that value for output, then project the other
summary and check provenance for the selected page. Count/page remain in one
snapshot. Prefix/status filters, registration/ID ties, unread ordering, original/
effective encounters and maximum wall-clock dates are unchanged. Alphabetical
and the earlier Newest optimization retain their execution paths. No stored
counter, index, migration, dependency, public API or wire change is introduced.

This uses SQLite's documented
[deferred-work optimization](https://www.sqlite.org/optoverview.html#using_co_routines_to_defer_work_until_after_the_sorting)
and [statement work counters](https://www.sqlite.org/c3ref/c_stmtstatus_counter.html).
A deterministic regression adds 200 offscreen original/effective occurrences and
compares page-work growth against the isolated required sorting aggregate, allowing
at most one extra VM step per occurrence. The former Frequency query fails with
7,500 added VM steps versus 600 for the aggregate; both changed sorts pass.
The selected page stays exact, offscreen invalid provenance does not fail that
page, and selecting the affected entry still rejects it and releases the snapshot.
A separate 257-entry test compares all row fields/page boundaries against a pure
expected ordering across mixed histories, equal metrics/dates, regressing clocks,
unread entries, prefix/status subsets and deleted documents. All-sort corruption
and SQL-failure tests retain read-only bytes and transaction cleanup.

Measured 2026-10-30 on the same host/toolchain/SQLite and synthetic fixtures as
above: 10k/100k entries, 10 status rounds, 1,000 rereads and 50,000 occurrences
concentrated on 25 entries. One excluded warmup, seven samples, isolated sequential
release executables without concurrent builds/tests; RAM-backed `/tmp` and
uncontrolled OS caches/CPU scheduling. Full repository medians in milliseconds:

| Entries | Frequency first before → after | Frequency last before → after | LastSeen first before → after | LastSeen last before → after |
| ---: | ---: | ---: | ---: | ---: |
| 10,000 | 20.755 → 16.798 | 23.079 → 9.320 | 20.601 → 17.410 | 23.064 → 11.566 |
| 100,000 | 108.717 → 64.419 | 155.940 → 103.059 | 109.834 → 65.600 | 155.573 → 107.287 |

At 100k entries, statement VM steps change as follows. All return 100 rows and
retain one statement sort; computing each candidate's metric remains necessary.

| Sort / offset | Before VM steps | After VM steps |
| --- | ---: | ---: |
| Frequency / 0 | 6,800,815 | 4,404,220 |
| Frequency / 99,900 | 6,700,915 | 2,554,322 |
| LastSeen / 0 | 6,800,815 | 4,504,120 |
| LastSeen / 99,900 | 6,700,915 | 2,854,222 |

First pages still include all 25 heavily encountered entries and pay their full
selected-row history/provenance cost. Last pages exclude those entries but still
scan their sorting metric and sort/skip 99,900 candidates. This is a bounded
projection improvement, not constant-time metric paging. SQL stepping medians
after the change at 100k are 60.339/95.597 ms (Frequency first/last) and
62.677/100.874 ms (LastSeen first/last). No improvement is claimed for other paths.

All 120 metric rows from four before/after runs, including unchanged statistics/
FIFO/encoding paths, are retained in
[the metric-sort CSV](measurements/vocabulary-metric-sorts-host-2026-10-30.csv).
Every row's seven raw samples validate its min/median/max. Exact unchanged database
bytes, FK/integrity, replay and scratch cleanup guards pass. The 100-entry,
zero-round/zero-reread/two-sample smoke passes all 21 metrics, including null dates
and even-sample medians; those smoke samples are outside the comparison CSV.

Baseline production is `e1a5aba` plus the profiler and query-builder extraction,
before changing execution. Baseline executable SHA-256:
`d5fc72ab776fce59c20b975178cb9f900b7c99e22a30acb30f513a5066217ae1`.
After executable SHA-256:
`92d663feaa1261b54843bc9051e17dc017902b6419c385a715fe92d6d6a50369`.
Use the existing reproduction commands and arguments; diagnostics run after
status profiling. Preserved executables: `/tmp/yomoku-browser-profile-before`
and `-after`. Logs/CSV: `/tmp/yomoku-browser-profile-{before,after}-{10000,100000}`
with `.log`/`.csv`; zero smoke uses `/tmp/yomoku-browser-profile-zero`.

PASS: 12 focused browser cases, 821 workspace tests (17 opt-in ignored), two
harness cases, all-feature workspace build, strict all-target/all-feature Clippy,
Rustfmt/diff. Initial diagnostics used unsupported rusqlite u64 bindings; corrected
to checked i64 conversions before verification. No open blocker. Status decoding/
replay, deep offsets, large views, diverse books, peak-memory and Compose/device
measurements remain follow-ups. Android/JNI/Kotlin/codegen, separate Full-loader
matrix, lifecycle/process-death and physical-arm64 checks were not rerun.

## Follow-up: status row decoding and temporary text allocations

The profiler now measures SQL stepping plus the exact production event decoder
and provenance validation, without replay or retaining events. It compiles the
private decoder and existing status parser directly; no public storage/FFI API is
added. Predecoded replay extraction also uses that decoder. Exact equality with
the public timeline, byte preservation, FK/integrity and scratch cleanup guards
still apply. Link reads remain outside this new metric. Like the other layers,
these independent timings are not an additive decomposition.

Status events formerly allocated temporary Strings for old status (when present),
new status and cause. Those enum spellings are only used while validating one
row. Borrow them with the pinned rusqlite 0.40.2
[Row::get_ref](https://docs.rs/rusqlite/0.40.2/rusqlite/struct.Row.html#method.get_ref)
and [ValueRef text accessors](https://docs.rs/rusqlite/0.40.2/rusqlite/types/enum.ValueRef.html#method.as_str_or_null),
removing up to three allocations per event. The documented lifetime prevents
references escaping the row; replay still receives the same owned numeric/enum
event. Dictionary-form strings remain owned because replay retains them. SQL,
VM work, snapshot ownership, provenance checks, core replay, schema and wire
formats are unchanged. No dependency or cache is added.

The decoder rejects non-TEXT values, invalid UTF-8, whitespace/case changes,
embedded NUL and unknown spellings. NULL is accepted only for absent old status.
Normal STRICT tables forbid BLOB values, so raw SQLite rows exercise type/NULL
gates separately. Public timeline/calendar read tests inject malformed UTF-8 and
nonliteral spellings, require unchanged bytes and released snapshots on failure,
then verify exact successful reads after fixture repair. Existing v1 provenance,
causal-chain, clock regression, Known-form, deletion/replay, concurrent generation
snapshot and read-only regressions remain in place.

Measured 2026-10-30 on the host/toolchain and synthetic fixtures above: 10k/100k
entries, ten status rounds, 1,000 rereads, one excluded warmup and seven samples.
Preserved release executables ran sequentially without overlapping builds/tests;
RAM-backed `/tmp`, OS caches, CPU frequency and scheduling remain uncontrolled.
Median milliseconds:

| Entries | SQL + decode/validate before → after | Public timeline before → after | Year statistics before → after |
| ---: | ---: | ---: | ---: |
| 10,000 | 54.325 → 52.808 | 54.619 → 52.007 | 56.841 → 54.123 |
| 100,000 | 596.927 → 532.782 | 584.478 → 545.795 | 633.523 → 545.436 |

At 100k, split SQL stepping medians are 198.389→193.717 ms; predecoded pure replay
100.719→103.093 ms. Query work stays 39,960,891 VM steps and zero sorts. Borrowing
removes temporary text allocation without addressing the SQL/provenance or pure
replay work. The complete timeline sample ranges overlap (571.979–589.955 ms
before, 534.714–587.051 ms after); the measured timing improvements must not be
read as portable guarantees. No performance change is claimed for unrelated
Vocabulary, FIFO or encoding metrics.

All 124 metric rows from four runs are retained in
[the decoding CSV](measurements/statistics-decoding-host-2026-10-30.csv). Every
row's seven raw samples validate its extrema/median. Exact replay/row counts,
unchanged database bytes, FK/integrity and scratch cleanup pass. The 100-entry,
zero-round/zero-reread/two-sample smoke passes all 22 metrics and even-sample
median checks; smoke samples are outside the comparison CSV.

Baseline production is `b51d650c7d7a359894ef5553268de562a5dcb8cb` plus extraction
of the unchanged private decoder/parser and the new diagnostic, before borrowing.
Baseline executable SHA-256:
`fcc43655ee8fb7f800564b9b8187db7e37811a05abef6c94471d2bf23a02eb77`.
After executable SHA-256:
`f9539774d34a1f941a8618544ae663ded4a293072b63bad93635450ab3089ac4`.
Preserved executables: `/tmp/yomoku-status-decode-{before,after}`. Run logs/CSV:
`/tmp/yomoku-status-decode-{before,after}-{10000,100000}.{log,csv}`; zero smoke:
`/tmp/yomoku-status-decode-zero.{log,csv}`.

PASS: 26 focused statistics cases, 823 workspace tests (17 opt-in ignored), three
harness cases, all-feature build, strict all-target/all-feature Clippy, Rustfmt/
diff. The initial integration corruption fixture attempted a BLOB write into a
STRICT TEXT column; SQLite correctly rejected it. Raw rows now test those type
and NULL gates; public reads cover malformed TEXT and cleanup. No open blocker.
Remaining work includes required row decoding/replay, deep metric offsets, large
views, diverse books, peak-memory and Compose/device measurements. Android/JNI/
Kotlin/codegen, separate Full-loader matrix, lifecycle/process-death and physical-
arm64 were not rerun; no emulator started and no device performance claim is made.
