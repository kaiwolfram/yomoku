# Resume here
Updated: 2026-10-30. Statistics SQL/replay profiling and streaming reads complete.
Commit guidance: all commits need explanatory bodies; merges include provenance.

## Current slice
- Vocabulary browser/selection/bulk Compose integration remains complete. Frozen
  IDs/status/date, explicit Retry/Cancel, hidden selections and invalidation are
  documented in ARCHITECTURE.md/DATA_MODEL.md and prior SESSION_LOG.md entries.
- Opt-in host example: crates/yomoku-ffi/examples/measure_vocabulary.rs. Disposable
  production-seeded database, synthetic B fixture, effective-factory native FIFO,
  full Bincode view/event costs. Analyzer/dictionary loading is forbidden in reads.
- New diagnostics separate SQL stepping from predecoded pure status replay and
  full validated repository cost. Only diagnostics retain the full event vector.
- Measured ordered-UNION sorting dominated SQL cost. Production now reads links
  then primary-key-ordered events in one deferred transaction. Standalone timeline
  opens its own snapshot; calendar statistics reuse theirs. All checks retained.
- 100k entries/1.08m events: VM steps 65.88m→39.96m, sorts 2→0; full year statistics
  median 944.157→542.853 ms. 10k: 86.947→53.254 ms. Remaining decoding/replay and
  Frequency/LastSeen costs are material. Earlier bounded Newest optimization stands.
- 100k selected IDs plus frozen confirmation still make a ~1.6 MB view. All loaded
  rows serialize; fully loaded large lists and Kotlin/Compose costs remain open.
- No schema/dependency/wire/architecture/data-destructive change. Schema is 0020.
  No Nostr starter data; configuration stays empty. Ask Q188 before forwarding.

## Next tasks (ordered)
1. Profile residual status-row decoding and Frequency/LastSeen sort/history costs
   before further optimization. Extend actual Compose/peak-memory/diverse-book
   measurements; retain snapshot/provenance/read-only and replay guards.
2. Verify supported AnkiDroid provider/deck/model/duplicate APIs against primary
   documentation; record feasible exact-surface duplicate behavior before wiring.
3. Extend the remaining accessibility/process-death/lifecycle matrix. Physical
   arm64 remains user-deferred; emulator/host results do not establish viability.

## Verification / blockers
- PASS: 24 focused statistics cases, 819 workspace tests (17 opt-in ignored), two
  harness cases, workspace all-feature build, strict all-target/all-feature Clippy,
  Rustfmt and diff checks. New cases verify zero sorts, committed WAL erasure/new
  generation isolation between link/event reads and query-failure cleanup. Existing
  corrupt-history cases now also verify standalone transaction cleanup.
- PASS: four release runs (before/after, 10k/100k), seven samples after warmup;
  all 104 CSV rows checked against raw sample counts/extrema/medians. Exact replay,
  unchanged DB bytes and scratch cleanup pass. Zero-history/even-sample smoke passes.
  Initial test compile needed internal helper/SQL visibility; corrected. No blocker.
- Results/limits/hashes: VOCABULARY_PERFORMANCE.md (follow-up section). Raw data:
  measurements/statistics-host-2026-10-30.csv. Commands: DEVELOPMENT.md, final section.
- Commands: cargo test --workspace --all-features --locked --offline --quiet;
  cargo test -p yomoku-ffi --example measure_vocabulary --all-features --locked
  --offline; cargo build --workspace --all-features --locked --offline;
  cargo clippy --workspace --all-targets --all-features --locked --offline --
  -D warnings; cargo fmt --all -- --check. Use the pinned Rust PATH workaround.
- Benchmark: cargo build -p yomoku-ffi --example measure_vocabulary --release
  --locked --offline; target/release/examples/measure_vocabulary 100000 10 1000 7.
  Never overlap builds/tests with timings. TMPDIR controls scratch storage.
- Logs: /tmp/yomoku-status-stream-{tests,workspace,example,build,clippy,release}.log;
  /tmp/yomoku-status-profile-{before,after}-{10000,100000}.{csv,log}; zero smoke:
  /tmp/yomoku-status-profile-zero.{csv,log}. Preserved executables: same prefix
  with -before / -after in /tmp, hashes in the performance document.
- Android/JNI/Kotlin/codegen/Full-loader matrix/process-death/physical-arm64 not
  rerun. No Compose/frame/peak-memory/diverse-book claim; broader plan stays open.
  No emulator started; ignored .local/statistics-ui-avd remains for future UI runs.
