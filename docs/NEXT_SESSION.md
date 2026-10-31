# Resume here
Updated: 2026-10-31. Opt-in native Vocabulary host phase diagnostics verified.
Commit guidance: explanatory bodies; merges include provenance. Keep verification
results in session/progress docs and reports, out of commit message bodies.

## Current slice
- Default-off Rust storage-measurements feature observes Vocabulary page/ID reads
  over the existing effective-reader FIFO. Normal constructors read no diagnostic
  clocks or allocate metadata; default builds exclude hooks. No JNI API added.
- Boundaries: pre-send to worker start; repository/open/cleanup; resolver through
  return (core response + request serialization + synchronous host callback).
  Resolution components remain combined; view encoding is separately measured.
  Observer runs after callback on worker; do not block/reenter. Queue includes
  diagnostic allocation/send/scheduling; observer work can delay later jobs.
- Four sequential release host processes (10k off, 100k observed, 100k off, 10k
  observed), ten status rounds/1k rereads/50k occurrences, warmup + seven samples.
  136 metric rows/952 raw durations validate with exact full IDs/bytes/cleanup.
- 100k Select All queue/execute/resolve medians: 0.035759/7.748635/0.186407 ms;
  feature-off callback median 7.806115 ms. Repository dominates this host path;
  no production optimization, Android phase/speed/memory/hardware claim.
- Results/hashes/raw CSV: VOCABULARY_PERFORMANCE.md, final section;
  measurements/vocabulary-worker-host-2026-10-31.csv.
- Prior scalar decoder and Android callback/Compose observers remain intact.
  VOCABULARY_ANDROID_PERFORMANCE.md retains their separate debug-emulator results;
  do not subtract these host phases from Android idle time or add nested spans.
- Schema 0020. Nostr starter configuration empty; ask Q188 before forwarding.

## Next tasks (ordered)
1. Profile actual Android native phases and 100k Select All Compose costs; compare
   an observer-free optimized Android workload before a production change. Host
   phases do not explain device costs; combined core/serialization remains open.
   Preserve full IDs, structural equality and event/replay guards. Heavy-history/
   diverse-book/full-load/exact peak work remains.
2. Verify supported AnkiDroid provider/deck/model/duplicate APIs against primary
   documentation; record feasible exact-surface duplicate behavior before wiring.
3. Extend remaining accessibility/process-death/lifecycle matrix. Physical arm64
   remains user-deferred; emulator/host results do not establish viability.

## Verification / blockers
- PASS: workspace build, 826 workspace tests/17 ignored data-dependent fixtures,
  three harness cases in each feature mode, strict workspace/all-target/all-feature
  and feature-off harness Clippy, Rustfmt/diff. Two new FFI tests cover blocked
  FIFO, callback/sample order/thread, full IDs/frozen prompt, failure/exact Retry.
- All four profiles pass full-ID/read-only bytes/no-analyzer/dictionary/integrity/
  FK/cleanup guards. Validator checks all raw summaries/metric sets/phase bounds.
- Commands: DEVELOPMENT.md, final Native Vocabulary worker diagnostics. Pinned
  Rust PATH; cargo test --workspace --all-features --locked --offline; cargo clippy
  --workspace --all-targets --all-features --locked --offline -- -D warnings.
- Profile: build measure_vocabulary --release with/without storage-measurements;
  executable args 10000/100000 10 1000 7. No concurrent builds/tests/profilers.
- Logs: /tmp/yomoku-worker-{workspace,harness,harness-base,focused,clippy,
  clippy-base,build,build-base,build-measured,validation}.log, size/mode CSV/log
  pairs; validator /tmp/yomoku-validate-worker.py. Corrected initial replay-test
  setup and diagnostic enum-size lint. No remaining blocker; no emulator started.
- Android/Kotlin/JNI/codegen, independent Full-loader, physical arm64, lifecycle/
  accessibility, optimized Android, exact peaks and diverse books not run here.
  Broad tasks remain open. Schema/dependencies/product behavior unchanged.
