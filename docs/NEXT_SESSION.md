# Resume here
Updated: 2026-10-30. Vocabulary host profiling/Newest optimization implemented.
Commit guidance: all commits need explanatory bodies; merges include provenance.

## Current slice
- Vocabulary browser/selection/bulk Compose integration is complete. Retained
  frozen IDs/status/date, explicit Retry/Cancel, hidden selections and invalidation
  semantics are in ARCHITECTURE.md/DATA_MODEL.md and prior SESSION_LOG.md entries.
- New opt-in host example: crates/yomoku-ffi/examples/measure_vocabulary.rs.
  Disposable database only; synthetic B fixture, production registration/import/
  completion/status APIs, effective-factory native FIFO, full Bincode view/event
  encoding and Rust decoding. Analyzer/dictionary loads are forbidden in reads.
- Profiles 1k/10k/100k entries, 50k concentrated encounters, up to 1.08m status
  events. Newest now selects its ordered LIMIT/OFFSET candidates before history
  probes in the same snapshot; count/filters/ties/provenance validation unchanged.
- 100k release-host medians: Newest first page 108.177→30.456 ms; native callback
  111.877→30.455 ms. Frequency/LastSeen still ~116 ms; year-statistics ~1.04 s.
- 100k selected IDs plus frozen confirmation make a 1.6 MB view. All loaded rows
  still serialize; fully loaded large lists and Kotlin/Compose costs remain open.
- No schema/dependency/wire/architecture/data-destructive change. Schema is 0020.
  No Nostr starter data; configuration stays empty. Ask Q188 before forwarding.

## Next tasks (ordered)
1. Profile the measured heavy-history costs: separate status replay/SQL costs in
   statistics and Frequency/LastSeen before choosing optimizations. Extend actual
   Compose/peak-memory/diverse-book measurements; retain current correctness guards.
2. Verify supported AnkiDroid provider/deck/model/duplicate APIs against primary
   documentation; record feasible exact-surface duplicate behavior before wiring.
3. Extend the remaining accessibility/process-death/lifecycle matrix. Physical
   arm64 remains user-deferred; emulator/host results do not establish viability.

## Verification / blockers
- PASS: 817 workspace tests (17 opt-in ignored), two harness cases, workspace
  all-feature build, strict all-target/all-feature Clippy, Rustfmt and diff checks.
- New VM-step regression: 200 offscreen original/effective encounters add no work
  to Newest's first page; last page retains exact counts/date and corruption errors.
- PASS: all six release benchmark runs (before/after at three sizes), seven samples
  after warmup per metric; 132 CSV rows checked against raw extrema/medians.
  Exact streaming byte equality and cleanup pass. Zero-history/even-sample smoke
  passes. Initial example enum spelling Year corrected to LastYear; no open blocker.
- Results/limits/executable hashes: VOCABULARY_PERFORMANCE.md. Raw data:
  measurements/vocabulary-host-2026-10-30.csv. Commands: DEVELOPMENT.md, final section.
- Commands: cargo test --workspace --all-features --locked --offline --quiet;
  cargo test -p yomoku-ffi --example measure_vocabulary --all-features --locked
  --offline; cargo build --workspace --all-features --locked --offline;
  cargo clippy --workspace --all-targets --all-features --locked --offline --
  -D warnings; cargo fmt --all -- --check. Use the pinned Rust PATH workaround.
- Benchmark: cargo build -p yomoku-ffi --example measure_vocabulary --release
  --locked --offline; target/release/examples/measure_vocabulary 100000 10 1000 7.
  Never overlap builds/tests with timings. TMPDIR controls scratch storage.
- Logs: /tmp/yomoku-vocabulary-performance-{tests,example,build,clippy}.log and
  /tmp/yomoku-vocabulary-measure-{after-,}*.{csv,log}.
- Android/JNI/Kotlin/codegen/Full-loader matrix/process-death/physical-arm64 not
  rerun. No Compose/frame/peak-memory/diverse-book claim; broader plan stays open.
  No emulator started; ignored .local/statistics-ui-avd remains for future UI runs.
