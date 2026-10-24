# Resume here

Updated: 2026-10-24. Root README added; reader continuation unchanged.
Read AGENTS.md, plan phases 2–4 and PRODUCT_SPEC.md sections 3–4.
Update docs/commit each verified step. Physical arm64 is the later user trial.

## Working
- Pure Crux owns navigation/Library/Settings and typed effects; FIFO worker owns
  SQLite/analysis. Only Render reaches UI. Fresh sessions start Library.
- Exact UTF-8 position/Library percentage and five page presets persist; reflow
  preserves the offset. Scroll resets; column 720dp. NO Next/Previous/swipes yet.
- Migrations 0005–0008 store frozen completion IDs/page/position, vocabulary/history,
  immutable A/B/C anchors, reversible edits and exact pasted UTF-8 SHA-256 identity.
  Analyzed paste atomically deduplicates, saves Mode B and registers Japanese surfaces.
  Duplicates reuse oldest live exact match without rescans/progress/status changes.
- NativeBridge requires verified owned FD+1 (zero means failure); Rust rehashes/maps
  that same file and closes FD on success/failure. No production text-only fallback.
  Lazy non-Send analyzer stays on existing FIFO worker; healthy SQL retries reuse it.
- Activity onStart permits cache; onStop releases after accepted work except config
  recreation retains warmth. Final onCleared drains/closes without joining the UI.
  Library never eagerly loads Full. Immutable publication never rewrites live inodes.
- Created/Existing show dismissible typed Library summary; never open reader.
  Settings/refresh/recreation retain it; dismissal/tab/new paste/reader clear it.
- Pure active_time uses supplied monotonic ms, five-minute idle cap, checkpoints,
  dictionary interactions and restored durations paused in a fresh clock epoch.
- NEW migration 0009 saves durations at exact UTF-8 offsets with checkpoint revisions.
  Backward navigation retains time; all 25 reflow pairs assign each anchor once to
  its containing page (boundaries following, EOF final). Restore only anchor duration.
  Checkpoint before moving; freeze page sum in PageCompletion. History.active_time_ms,
  bucket consumption and position commit atomically. Stale tokens cannot revive time;
  duplicate completion cannot drain reread time. Zero revision rows remain until delete.
  No app/worker/FFI/Android timer wiring or occurrences/New→Known completion yet.
- Sudachi.rs v0.7.0/android-loader-v1 + Full 20261015 pinned; 138 source files verified
  by native builds. Full prepared outside Git; APK bundles dictionary/notices.
  Prior API-36 x86_64 process-death/pressure results: SUDACHI_MEASUREMENTS.md.
  No physical-arm64/LMKD claim. No dictionary/vocabulary/override UI yet.
- Paste text/IDs/provenance immutable; no inferred TXT chapters. File imports must
  hash original bytes separately. Document deletion retains vocabulary/history.

## Next tasks (ordered)
1. Extend atomic completion with every Japanese occurrence and New→Known events;
   preserve frozen command IDs/time and verify rollback, repetitions and rereads.
2. Wire core/worker/FFI checkpoints to Android monotonic clock/lifecycle/interactions,
   including periodic saves, close/reopen/reboot; then Next/Previous/swipes and Finish.
   Format imports and CI remain unchecked.
3. User-run physical arm64 validation once usable; defer until their app trial.

## Verification / environment
- README step: local Markdown links/anchors, command/source review and diff check
  pass. Documentation only; no Rust/Android/JNI checks rerun or new blocker.
- Prior implementation PASS: workspace build, 169 data-free Rust tests (88 storage, 12 new),
  fmt/strict Clippy, host real-Full Kotlin/JNI + seven Unicode exchanges, diff check.
  Twelve new tests cover SQLite reopen/fresh clock epoch, all 25 reflows, concurrent/
  stale/duplicate saves/completions, rereads/EOF/deletion/overflow and SQL/commit/
  migration rollback. Reboot is simulated at the duration boundary, not on Android.
- No open failure. Existing benign Crux shutdown logs in host JNI persist.
  No new dependency version. No Full standalone fixtures, Android build/lint/tests,
  unsigned release, probes/tablet/physical-arm64 rerun for this Rust repository step.
  Prior Android 19/19 and lint reports: .local/test-reports/analyzed-import/.
- Commands/env: DEVELOPMENT.md. Rust PATH workaround, then sequentially:
  cargo build --workspace --locked --offline
  cargo test --workspace --all-features --locked --offline
  cargo fmt --all -- --check
  cargo clippy --workspace --all-targets --all-features --locked --offline -- -D warnings
  tools/test-kotlin-bridge.sh
  git diff --check
  Never run native builds/generation/tests together or modify Full while mapped.
  Gradle/ADB can need socket escalation; emulator swangle fallback documented.
- Nostr configuration empty; ask Q188 before forwarding. No local reader blocker.
