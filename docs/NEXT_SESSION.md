# Resume here

Updated: 2026-10-24. Pure active-time accounting verified; Full gate remains.
Read AGENTS.md, plan phases 2–4 and PRODUCT_SPEC.md sections 3–4.
Update docs/commit each verified step.

## Working
- Pure Crux core owns navigation/Library/Settings and typed effects. FIFO worker
  owns SQLite; only Render reaches UI. Retained Android sessions/private DB,
  Paste→Library and explicit reader opening work; fresh start Library.
- Exact UTF-8 position/Library percentage persist; five presets preserve offset.
  Scroll resets; column 720dp. NO Next/Previous/swipes before atomic completion.
- Migrations 0005–0007 store frozen completion IDs/page/position, global surfaces/
  dated five-status history, immutable versioned A/B/C anchors and reversible
  global/local edits. No override application, forward action or vocabulary UI.
- Storage::create_analyzed_paste atomically composes canonical text, validated
  Mode B snapshot and unique Japanese registrations; preserves existing statuses.
- import_analyzed_paste wraps that composition with exact UTF-8 SHA-256
  dedup (migration 0008). Existing returns oldest live match, preserves progress/
  title/vocab and skips analysis/rescan. Legacy hashes are filled lazily under the
  write lock; legacy duplicates stay intact. All failures roll back, including
  backfill. Deletion removes hash; reimport gets fresh IDs/progress. Raw creation
  still allocates without dedup. Preload/reuse analyzer before callback; no DB
  work or provisional IDs may escape. No encounters/position.
  Repository only: production Android/runtime still use text-only create_paste.
- NEW pure active_time uses supplied monotonic ms; five-minute idle cap, reader/
  dictionary interactions reset idle; background/close pause. Checkpoints do not
  drain/reset idle. Restore only PendingReadingTime duration, paused in a fresh
  clock epoch. Thirteen domain tests pass; no SQLite/runtime/FFI/Android wiring.
- Sudachi.rs v0.7.0/android-loader-v1 and Full 20261015 stay pinned. Source verifier
  checks 138 files; never alter shared Cargo cache. Eleven real fixtures per loader.
  Full is prepared outside Git; compressed APK includes complete notices.
- Worker-only private publisher verifies size/SHA-256, syncs and atomically
  publishes read-only files and returns that same verified descriptor. Repair
  never writes/truncates live inodes. Eight extraction/recovery device tests pass.
- Debug JNI probe verifies fixtures, retry/affinity/stale IDs, recreation/stop/
  resume reuse, close/unmap/fresh-worker reopen. ADB driver verifies two actual
  force-stop/restart pairs, new PIDs and unchanged cached inode/mtime/read-only
  bytes. Results: SUDACHI_MEASUREMENTS.md; no pressure/LMKD/hardware claim.
- Safe owned loader remains; unsafe mapping requires inode immutability until
  analyzer drop. Initial checksums touch all pages; production lifecycle undecided.
- Paste text/IDs/provenance immutable; no inferred TXT chapters. Future file imports
  must hash original bytes separately. Document deletion keeps vocab.

## Next tasks (ordered)
1. Finish Full gate: actual memory-pressure/reclamation; choose production worker
   lifecycle/reuse. Physical arm64 required before claiming hardware viability.
2. After gate, wire import_analyzed_paste into runtime/FFI/Android with a reused
   analyzer and Created/Existing summary; extend format imports later.
3. Persist pending page time using active_time (page/reflow attribution open),
   extend completion with occurrences/New→Known/time, then wire frozen IDs and
   Android clock/lifecycle/interactions + Next/Previous/swipes. Finish/CI remain.

## Last verified checks
```sh
export PATH=/home/kai/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin:$PATH
cargo build --workspace --locked --offline
cargo test --workspace --all-features --locked --offline
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked --offline -- -D warnings
git diff --check
```
PASS: 147 data-free Rust tests (76 storage), workspace build, fmt/strict Clippy.
Thirteen time tests cover idle/lifecycle/checkpoint/restore and clock/overflow errors.
Close/reboot is simulated at the duration boundary, not on device.
No open failure. No Android/Kotlin/JNI/Full fixture rerun for this pure-domain step.
Prior tools/test-sudachi.sh (11 fixtures per loader), tools/test-kotlin-bridge.sh,
18 Android tests/lint/debug+release builds and death driver pass; commands/reports
remain in DEVELOPMENT.md and SUDACHI_MEASUREMENTS.md. Prior nonfatal JNI shutdown
logging remains. No emulator/user-file/radio/viewport changes.

## Environment/blockers
- Never run native builds/generation/tests concurrently. Prepare Full before APK
  builds; never prepare/replace/truncate dictionary bytes during mapped probes.
- Android env/commands: DEVELOPMENT.md. ADB/Gradle sockets may need escalation.
  Instrumentation uninstalls app: reinstall debug before separate death driver.
- medium_phone API 36 uses swangle/KVM; tools/start-android-emulator.sh.
- Actual pressure/LMKD, mid-extraction death/power loss and physical arm64 remain
  unverified. Stop/resume and force-stop are not memory-pressure tests.
- Nostr stays empty; ask Q188 before forwarding. No production edit/rename/delete.
