# Resume here
Updated: 2026-10-30. Status row decoding/allocation profiling complete.
Commit guidance: explanatory bodies; merges include provenance. Keep verification
results in session/progress docs and reports, out of commit message bodies.

## Current slice
- Vocabulary browser/selection/bulk Compose integration remains complete. Frozen
  IDs/status/date, explicit Retry/Cancel and invalidation are documented in
  ARCHITECTURE.md/DATA_MODEL.md and prior SESSION_LOG.md entries.
- Opt-in host example: crates/yomoku-ffi/examples/measure_vocabulary.rs. Disposable
  production-seeded DB, synthetic B fixture, effective-factory native FIFO and
  full Bincode view/event costs. Analyzer/dictionary loading is forbidden in reads.
- Earlier slices bounded Newest/metric-sort history projections and split ordered
  status links/events within one snapshot. Diagnostics share production SQL.
- New status SQL-plus-decoding diagnostics compile the private production decoder
  and enum parser. Production borrows old/new status/cause TEXT within each row,
  removing up to three temporary String allocations per event. Required v1 cause/
  provenance/causal replay checks and owned dictionary-form links remain intact.
- At 100k entries/1.08m events: SQL plus decode/validate median 596.927→532.782 ms;
  complete timeline 584.478→545.795 ms. SQL still uses 39,960,891 VM steps/zero
  sorts; predecoded replay ~103 ms. Timing layers are independent, host-only.
- Metric sorts still scan matching histories and skip deep offsets. 100k selected
  IDs plus frozen confirmation still make a ~1.6 MB view. All loaded rows serialize;
  large lists/Kotlin/Compose/peak-memory/diverse-books remain open.
- No schema/dependency/wire/architecture/data-destructive change. Schema is 0020.
  No Nostr starter data; configuration stays empty. Ask Q188 before forwarding.

## Next tasks (ordered)
1. Extend actual Compose/peak-memory/diverse-book measurements; retain snapshot,
   provenance/read-only/replay guards. Required row decoding/replay and deep metric
   offsets remain measurable costs; do not infer device behavior from host results.
2. Verify supported AnkiDroid provider/deck/model/duplicate APIs against primary
   documentation; record feasible exact-surface duplicate behavior before wiring.
3. Extend remaining accessibility/process-death/lifecycle matrix. Physical arm64
   remains user-deferred; emulator/host results do not establish viability.

## Verification / blockers
- PASS: 26 focused statistics cases, 823 workspace tests (17 opt-in ignored),
  three harness cases, all-feature build, strict all-target/all-feature Clippy,
  fmt/diff. Raw SQLite rows test type/NULL gates; public timeline/calendar tests
  verify malformed UTF-8/nonliteral text, unchanged bytes, failure cleanup and
  exact fixture recovery. Existing provenance/replay/concurrent snapshot pass.
- PASS: four sequential release runs (10k/100k before/after), seven samples after
  warmup; all 124 CSV rows validate extrema/median. Exact replay, unchanged DB
  bytes, FK/integrity and scratch cleanup pass. Zero-history/two-sample smoke:
  all 22 metrics pass. Initial BLOB corruption fixture hit correct STRICT-table
  rejection; moved those gates to raw rows. No unresolved blocker.
- Results/hashes/limits: VOCABULARY_PERFORMANCE.md, final section. Raw data:
  measurements/statistics-decoding-host-2026-10-30.csv. Commands: DEVELOPMENT.md,
  final section. Use pinned Rust PATH workaround. Baseline production: b51d650.
- Commands: cargo test --workspace --all-features --locked --offline --quiet;
  cargo test -p yomoku-ffi --example measure_vocabulary --all-features --locked
  --offline; cargo build --workspace --all-features --locked --offline;
  cargo clippy --workspace --all-targets --all-features --locked --offline --
  -D warnings; cargo fmt --all -- --check. Focus: cargo test -p yomoku-storage
  statistics --locked --offline.
- Benchmark: cargo build -p yomoku-ffi --example measure_vocabulary --release
  --locked --offline; target/release/examples/measure_vocabulary 100000 10 1000 7.
  Never overlap builds/tests with timings. TMPDIR controls scratch storage.
- Logs: /tmp/yomoku-status-decode-{tests,workspace,example,build,clippy,release}.log;
  baseline extraction: -baseline-{tests,example,release}.log; timings:
  /tmp/yomoku-status-decode-{before,after}-{10000,100000}.{csv,log}; zero smoke:
  /tmp/yomoku-status-decode-zero.{csv,log}. Before/after executables kept in /tmp.
- Android/JNI/Kotlin/codegen/Full-loader/lifecycle/process-death/physical-arm64 not
  rerun. No Compose/frame/peak-memory/diverse-book claim; broader plan stays open.
  No emulator started; ignored .local/statistics-ui-avd remains.
