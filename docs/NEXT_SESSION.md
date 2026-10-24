# Resume here

Updated: 2026-10-24. Reader analysis identity restoration verified.
Read AGENTS.md, plan phases 2–4 and PRODUCT_SPEC.md sections 3–4.
Update docs/commit each verified step. Physical arm64 is the later user trial.

## Working
- Pure Crux owns navigation/Library/Settings and typed effects; FIFO worker owns
  SQLite/analysis. Only Render reaches UI. Fresh sessions start Library.
- Exact UTF-8 position/Library percentage and five presets persist; reflow keeps
  offset. Scroll resets; column 720dp. NO Next/Previous/swipes/Finish Book yet.
- Migrations 0005–0010 provide frozen completion IDs/page/position, vocabulary/
  history, immutable A/B/C anchors, reversible edits, paste SHA-256 and time.
  Analyzed paste atomically deduplicates, saves Mode B and registers surfaces.
  Duplicates reuse oldest live exact match without rescan/progress/status changes.
- NEW load_reader_snapshot reads document/exact position, preset and original
  Mode B AnalysisId in one read transaction; core/worker/generated FFI carry it.
  Earliest stored B stays fixed through reflow/position saves/plain LoadPaste;
  later profiles/current installed engine never replace it. No token scan/load
  of Sudachi or writes on open. ReaderAnalysis::Missing explicitly marks legacy
  absence; still readable. None outside reader; exits/replacements/cancelled
  loads cannot retain/attach it. Bad header/settings error without fallback/repair.
  Full token validation remains inside completion. No legacy forward policy yet.
- NativeBridge requires verified owned FD+1 (zero failure); Rust rehashes/maps
  that file and closes FD on all paths. No production text-only import fallback.
  Lazy non-Send analyzer stays on existing FIFO; healthy SQL retries reuse it.
  onStart permits cache; onStop releases after accepted work except config
  recreation retains warmth. onCleared drains/closes without joining the UI.
  Library never eagerly loads Full; immutable publication preserves live inodes.
- Created/Existing show dismissible Library summary, never open reader.
  Settings/refresh/recreation retain it; dismissal/tab/new paste/reader clear it.
- Pure active_time takes monotonic ms, five-minute idle cap and interactions;
  restored durations start paused in a fresh epoch. Migration 0009 stores exact
  offset durations/revisions. All reflow pairs assign each anchor once (boundary
  following, EOF final). Restore only anchor duration; checkpoint before moving.
- record_analyzed_page_completion freezes AnalysisId + command/page/date/time.
  Every Japanese repetition retains full anchors/surface/reading/lemma; tokens
  belong to page containing start. New-only Known events link to command.
  History/time consumption/encounters/status/position commit together. Recorded
  returns newly_known; AlreadyRecorded has no repeated effects. Replays validate
  frozen payload before live content; legacy completion cannot gain encounters.
  Document deletion retains history; vocabulary deletion cascades its history.
  Later completion registers only absent departing-page surfaces. Base analyses
  only; effective override resolution and app completion intents remain open.
- Sudachi.rs v0.7.0/android-loader-v1 + Full 20261015 pinned; 138 files verified.
  Full stays outside Git; APK bundles dictionary/notices. Prior emulator pressure/
  death results: SUDACHI_MEASUREMENTS.md. No physical-arm64/LMKD claim.
- Paste content/IDs/provenance immutable; no inferred TXT chapters. File imports
  must hash original bytes. No dictionary/vocabulary/override UI yet.

## Next tasks (ordered)
1. Wire core/FIFO-worker/FFI pending-time checkpoints and analyzed completion
   intents using the restored analysis ID; freeze command IDs/time across retries,
   preserve ordering, return newly_known only on new success. Handle Missing
   explicitly before completing a legacy page; no automatic import rescan exists.
2. Connect Android monotonic clock/lifecycle/interactions and periodic saves;
   verify close/reopen/reboot, then Next/Previous/swipes and Finish Book.
3. User-run physical arm64 validation once usable. Formats/CI remain open.

## Verification / environment
- PASS: workspace build, 187 data-free Rust tests (102 storage), fmt/strict Clippy,
  host real-Full Kotlin/JNI, seven Unicode + two analysis-state codec exchanges,
  Kotlin format, Android debug/test APK builds and strict lint, diff check.
  Tests cover original-profile selection, missing/foreign modes, corrupt headers,
  read-only opening, exact position/preset/reopen, frozen/cleared core state and
  no analyzer load on worker/JNI reopen. Prior benign Crux JNI shutdown logs and
  SDK XML warning persist. No open failure/dependency change. No instrumentation,
  standalone Full fixtures, release/probes/tablet/arm64 rerun for this slice.
- Commands/env: DEVELOPMENT.md (latest section). Rust PATH workaround; native
  builds/generation/tests must run sequentially. Never modify Full while mapped.
  Gradle/ADB may need socket escalation; swangle fallback is documented.
- Nostr config empty; ask Q188 before forwarding. No local reader blocker.
