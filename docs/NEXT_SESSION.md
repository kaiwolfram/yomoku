# Resume here

Updated: 2026-10-24. Atomic analyzed completion repository verified.
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
  that file and closes FD on success/failure. No production text-only fallback.
  Lazy non-Send analyzer stays on existing FIFO worker; healthy SQL retries reuse it.
- Activity onStart permits cache; onStop releases after accepted work except config
  recreation retains warmth. Final onCleared drains/closes without joining the UI.
  Library never eagerly loads Full. Immutable publication never rewrites live inodes.
- Created/Existing show dismissible typed Library summary; never open reader.
  Settings/refresh/recreation retain it; dismissal/tab/new paste/reader clear it.
- Pure active_time uses supplied monotonic ms, five-minute idle cap, checkpoints,
  dictionary interactions and restored durations paused in a fresh clock epoch.
- Migration 0009 saves durations at exact UTF-8 offsets with checkpoint revisions.
  Backward movement retains time; all 25 reflow pairs assign anchors once to their
  containing pages (boundary following, EOF final). Restore only anchor duration.
  Checkpoint before moving; freeze page sum in PageCompletion. History.active_time_ms,
  bucket consumption and position commit atomically; stale/duplicate tokens are safe.
- NEW migration 0010: record_analyzed_page_completion freezes AnalysisId with the
  command/page/date/time. completion_analyses retains historical profile/mode; every
  Japanese token (repetitions included) stores full anchors/surface/reading/lemma.
  Tokens belong to the page containing their start. New-only Known events link to
  command; Recorded returns newly_known; AlreadyRecorded repeats no effects.
  All joins the time/history/position transaction. Document deletion retains history;
  vocabulary deletion cascades its encounters/status events. Actual later completion
  reregisters only absent departing-page surfaces. Legacy completion lacks analyzed
  history and cannot be upgraded by replay. Base analyses only; no override resolver.
- Sudachi.rs v0.7.0/android-loader-v1 + Full 20261015 pinned; 138 source files verified.
  Full prepared outside Git; APK bundles dictionary/notices. Prior emulator pressure/
  death results: SUDACHI_MEASUREMENTS.md. No physical-arm64/LMKD claim.
- Paste content/IDs/provenance immutable; no inferred TXT chapters. File imports must
  hash original bytes separately. No dictionary/vocabulary/override UI yet.

## Next tasks (ordered)
1. Wire core/FIFO-worker/FFI pending-time checkpoints and analyzed completion intents;
   obtain the opened paste's stored analysis identity, freeze IDs/time across retries,
   preserve ordering and return newly_known only for new success. Handle legacy paste
   snapshot availability explicitly; no automatic import rescan exists today.
2. Connect Android monotonic clock/lifecycle/interactions and periodic saves; verify
   close/reopen/reboot, then Next/Previous/swipes and Finish Book. Formats/CI stay open.
3. User-run physical arm64 validation once usable; defer until their app trial.

## Verification / environment
- PASS: workspace build, 182 data-free Rust tests (99 storage), fmt/strict Clippy,
  eleven analysis + one worker real-Full fixture with each owned/mapped loader,
  138-file source integrity, host real-Full Kotlin/JNI + seven Unicode exchanges,
  and diff check. Eleven new storage + two pure tests cover repetition, all statuses,
  presets/boundaries, rereads/replays/concurrency, deletion, snapshot validation,
  SQL/deferred-commit rollback and migration/event-ID high-water preservation.
- No open failure or dependency change. Initial Clippy test-tuple complexity fixed
  with a type alias; final pass. Prior benign Crux JNI shutdown logging persists.
  No Android build/lint/tests, release/probes/tablet/arm64 rerun for this Rust slice.
  Prior Android 19/19 and lint: .local/test-reports/analyzed-import/.
- Commands/env: DEVELOPMENT.md. Rust PATH workaround; run sequentially:
  cargo build --workspace --locked --offline
  cargo test --workspace --all-features --locked --offline
  cargo fmt --all -- --check
  cargo clippy --workspace --all-targets --all-features --locked --offline -- -D warnings
  tools/test-sudachi.sh
  tools/test-kotlin-bridge.sh
  git diff --check
  Never run native builds/generation/tests together or modify Full while mapped.
  Gradle/ADB may need socket escalation; swangle fallback is documented.
- Nostr configuration empty; ask Q188 before forwarding. No local reader blocker.
