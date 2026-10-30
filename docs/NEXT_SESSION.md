# Resume here
Updated: 2026-10-30. Frequency/LastSeen candidate history profiling complete.
Commit guidance: all commits need explanatory bodies; merges include provenance.

## Current slice
- Vocabulary browser/selection/bulk Compose integration remains complete. Frozen
  IDs/status/date, explicit Retry/Cancel and invalidation are documented in
  ARCHITECTURE.md/DATA_MODEL.md and prior SESSION_LOG.md entries.
- Opt-in host example: crates/yomoku-ffi/examples/measure_vocabulary.rs. Disposable
  production-seeded DB, synthetic B fixture, effective-factory native FIFO and
  full Bincode view/event costs. Analyzer/dictionary loading is forbidden in reads.
- Statistics link/event split reads removed full-history sorting in one snapshot;
  100k entries/1.08m events: year median 944.157→542.853 ms (previous slice).
- New metric-sort diagnostics compile the production query builder directly and
  verify every raw SQL field/order before measuring stepping/VM work. Frequency/
  LastSeen now compute only their ordering aggregate for candidates, reuse it and
  defer other history/provenance probes to selected rows; explicit outer order.
- At 100k: first-page medians 108.717→64.419 ms (Frequency), 109.834→65.600 ms
  (LastSeen); last-page 155.940→103.059 and 155.573→107.287 ms. Sorting metrics
  still visits matching histories; deep offsets remain costly. No cached counter.
- 100k selected IDs plus frozen confirmation still make a ~1.6 MB view. All loaded
  rows serialize; large lists/Kotlin/Compose/peak-memory/diverse-books remain open.
- No schema/dependency/wire/architecture/data-destructive change. Schema is 0020.
  No Nostr starter data; configuration stays empty. Ask Q188 before forwarding.

## Next tasks (ordered)
1. Profile residual status-row decoding/replay and extend actual Compose/peak-memory/
   diverse-book measurements; retain snapshot/provenance/read-only/replay guards.
   Metric sorts still scan matching aggregates and sort/skip deep offsets.
2. Verify supported AnkiDroid provider/deck/model/duplicate APIs against primary
   documentation; record feasible exact-surface duplicate behavior before wiring.
3. Extend remaining accessibility/process-death/lifecycle matrix. Physical arm64
   remains user-deferred; emulator/host results do not establish viability.

## Verification / blockers
- PASS: 12 focused browser cases, 821 workspace tests (17 opt-in ignored), two
  harness cases, all-feature build, strict all-target/all-feature Clippy, fmt/diff.
  New exact 257-row mixed-history paging/filter/tie regression and offscreen VM
  growth bound pass. Former SQL fails bound as expected (7,500 versus 600 extra
  Frequency steps). All-sort corruption and query-failure cleanup also pass.
- PASS: four sequential release runs (10k/100k before/after), seven samples after
  warmup; all 120 CSV rows validate extrema/median. Exact SQL/repository rows,
  replay, unchanged DB bytes, FK/integrity and scratch cleanup pass. Zero-history/
  two-sample smoke passes. Initial diagnostics u64 SQL bindings corrected to i64.
  No open blocker. No emulator started; ignored .local/statistics-ui-avd remains.
- Results/hashes/limits: VOCABULARY_PERFORMANCE.md, final section. Raw data:
  measurements/vocabulary-metric-sorts-host-2026-10-30.csv. Commands: DEVELOPMENT.md,
  final section. Use pinned Rust PATH workaround. Baseline production: e1a5aba.
- Commands: cargo test --workspace --all-features --locked --offline --quiet;
  cargo test -p yomoku-ffi --example measure_vocabulary --all-features --locked
  --offline; cargo build --workspace --all-features --locked --offline;
  cargo clippy --workspace --all-targets --all-features --locked --offline --
  -D warnings; cargo fmt --all -- --check. Focus: cargo test -p yomoku-storage
  vocabulary_browser --locked --offline.
- Benchmark: cargo build -p yomoku-ffi --example measure_vocabulary --release
  --locked --offline; target/release/examples/measure_vocabulary 100000 10 1000 7.
  Never overlap builds/tests with timings. TMPDIR controls scratch storage.
- Logs: /tmp/yomoku-browser-page-{tests,workspace,example,build,clippy,release}.log;
  /tmp/yomoku-browser-page-regression-baseline.log (expected failure);
  /tmp/yomoku-browser-profile-{before,after}-{10000,100000}.{csv,log}; zero smoke:
  /tmp/yomoku-browser-profile-zero.{csv,log}. Before/after executables kept in /tmp.
- Android/JNI/Kotlin/codegen/Full-loader/lifecycle/process-death/physical-arm64 not
  rerun. No Compose/frame/peak-memory/diverse-book claim; broader plan stays open.
