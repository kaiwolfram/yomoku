# Resume here

Updated: 2026-10-24. Full emulator loading gate resolved; production wiring next.
Read AGENTS.md, plan phases 2–4 and PRODUCT_SPEC.md sections 3–4.
Update docs/commit each verified step. Physical arm64 is the later user trial.

## Working
- Pure Crux core owns navigation/Library/Settings and typed effects. FIFO worker
  owns SQLite; only Render reaches UI. Retained Android sessions/private DB,
  Paste→Library and explicit reader opening work; fresh start Library.
- Exact UTF-8 position/Library percentage persist; five presets preserve offset.
  Scroll resets; column 720dp. NO Next/Previous/swipes before atomic completion.
- Migrations 0005–0007 store frozen completion IDs/page/position, global surfaces/
  dated five-status history, immutable versioned A/B/C anchors and reversible
  global/local edits. No override application, forward action or vocabulary UI.
- Storage::import_analyzed_paste atomically composes canonical text, validated
  Mode B snapshot and unique Japanese registrations, preserving existing statuses.
  Migration 0008 dedups exact UTF-8 SHA-256; Existing skips analysis/rescan and
  returns oldest live match without changing title/progress/vocab. Legacy hashes
  fill lazily under write lock; duplicates stay intact. Every failure rolls back.
  Deletion/reimport allocates fresh IDs/progress; raw create stays non-dedup.
  Preload/reuse analyzer before callback; no DB work/provisional IDs may escape.
  Repository only: Android/runtime still use text-only create_paste. No encounters.
- Pure active_time uses supplied monotonic ms, five-minute idle cap and reader/
  dictionary interactions. Background/close pauses; checkpoints retain pending
  duration without draining/resetting idle. Restore duration paused in fresh clock
  epoch. Thirteen tests; no SQLite/runtime/FFI/Android wiring or page attribution.
- Sudachi.rs v0.7.0/android-loader-v1 and Full 20261015 stay pinned. Source verifier
  checks 138 files; never alter shared Cargo cache. Eleven fixtures per loader.
  Full is prepared outside Git; compressed APK includes complete notices.
- Worker-only private publisher verifies size/SHA-256, syncs/atomically publishes
  read-only files and returns the same verified descriptor. Repair never rewrites
  live inodes. Eight extraction/recovery device tests pass.
- Debug JNI/ADB probes verify fixture correctness, worker affinity/retry/stale IDs,
  retained reuse, close/unmap/reopen and two real force-stop/restart recovery pairs.
- NEW tools/test-sudachi-memory-pressure.py + NDK shell helper measure real reclaim:
  two x86_64 runs reclaim 67/143 MiB dictionary RSS under bounded anonymous load;
  original PID/generation reruns all fixtures. PSI denied; no LMKD/hardware claim.
  Reports/limits: SUDACHI_MEASUREMENTS.md. No synthetic trim/cache drop/madvise.
- Production decision (not wired): one lazy immutable mapped analyzer on existing
  FIFO worker, reuse foreground/config changes; release idle cache on UI-hidden/
  background and final session close, after accepted atomic work finishes. Later
  work lazily re-verifies/reopens. No eager Library load/per-page reload/new worker.
  Keep safe owned loader for uncontrolled paths. Never mutate live mapped bytes.
- Paste text/IDs/provenance immutable; no inferred TXT chapters. Future file imports
  must hash original bytes separately. Document deletion keeps vocab.

## Next tasks (ordered)
1. Wire deduplicating import_analyzed_paste through runtime/FFI/Android, verified
   descriptor delivery/lifecycle and Created/Existing summary; test atomic rollback,
   vocabulary/dedup, worker reuse/release/retry and no eager Library load end to end.
2. Persist pending page time (page/reflow attribution open), extend completion with
   occurrences/New→Known/time, then wire frozen IDs and Android clock/lifecycle/
   interactions + Next/Previous/swipes. Finish/format imports/CI remain unchecked.
3. User-run physical arm64 validation once usable; defer until their app trial.

## Verification / environment
- This step: debug/test APK assembly, strict lint and Kotlin format pass. Two real
  pressure/revalidation runs, final 18/18 Android tests and two death/recovery pairs
  pass; Python syntax/bound rejection, strict C compile and whitespace pass.
- First pressure run lost ADB transport, excluded; original PID survived. Reconnect,
  confirmed force-stop, then removed its owned directory/helper. Successful runs
  clean only their files and return to Library. No radio/viewport changes.
- Initial Android suite: 17/18 pass, existing notices dialog times out. Focused
  SudachiAssetsTest rerun: 2/2 pass; full rerun 18/18 pass. Reports under
  .local/test-reports/sudachi-pressure/. No timeout was relaxed or assertion removed.
- Last host baseline: 147 data-free Rust tests (76 storage), build/fmt/strict Clippy;
  no Rust changes/host tests rerun this step. Prior Full/host JNI results stand.
- Commands/env: DEVELOPMENT.md (JDK17, pinned NDK/SDK/Gradle). Run debug assembly/
  instrumentation/lint, explicit adb install -r (instrumentation uninstalls app),
  then python3 tools/test-sudachi-memory-pressure.py --serial emulator-5554 and
  tools/test-sudachi-process-death.py. ADB/Gradle sockets can need escalation.
- Never run native builds/generation/tests concurrently. Prepare Full before APK
  builds; never prepare/replace/truncate bytes during mapped probes. Emulator uses
  swangle/KVM; tools/start-android-emulator.sh. Nostr empty; Q188 before forwarding.
