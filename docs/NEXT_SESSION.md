# Resume here

Updated: 2026-10-24. Finish Book + command-keyed confetti verified.
Read AGENTS.md, plan phases 2–4 and PRODUCT_SPEC.md sections 3–4.

## Working
- Pure Crux owns navigation/Library/Settings and typed effects; FIFO owns SQLite/
  analysis. Only Render reaches UI. Fresh sessions start Library.
- Exact UTF-8 position/Library percentage and five presets persist; reflow keeps
  offset. Scroll resets on navigation/reopen; centered column is capped at 720dp.
- Next/Previous/counter below content; left/right swipes navigate without a
  bottom requirement. Axis-specific drags require 48dp net movement on release.
  Next completes the departing page; anchored Previous derives the preceding page
  in Rust and only saves position. Both use retained dispatch/Pause/checkpoint drain.
  Busy/pending saves disable actions. Duplicate callbacks count once; rereads again.
  Positive CompletionNotice shows a short toast and is consumed once; zero is silent.
  Final control says Finish Book; its action/left swipe returns to Library only after
  final clock drain + atomic success. Failure stays with Retry. Recorded final commands
  expose book_completion_notice even at zero New/rereads; AlreadyRecorded is silent.
  Compose consumes the keyed notice before a local 1.4s confetti overlay; recreation
  cannot replay it. Final return preserves positive toast. Migration 0011 marks finished.
  Marker crosses document/Library/reader snapshots; accessible checkmarks show it
  independently of position. Backward/reflow/time saves retain it; deletion resets
  it while history stays. Migration backfills live EOF history, never position alone.
- Analyzed paste atomically deduplicates exact SHA-256, saves Mode B and registers
  unique Japanese surfaces. Duplicates reuse oldest live exact match without
  rescan/progress/status changes. Created/Existing show dismissible Library summary.
- load_reader_snapshot/load_paste stay read-only, especially completion reloads.
  OpenReader uses open_reader_snapshot: one immediate transaction resets finished
  books to page 1 and reads marker/preset/original Mode B identity. Pending time/history
  stay; failure rolls back. Unfinished offsets and retained recreation stay exact.
  Identity stays fixed through reflow/position/plain loads; Missing never rescans.
- RestoreReadingTime reserves FIFO revision; absolute saves retain exact retry payload/
  revision and coalesce newer maxima, chaining revisions. View shows committed duration
  + pending only; no history. Reflow/same-offset saves preserve idle deadline.
- ObserveReaderClock(document/exact position, monotonic_ms, action) drives pure
  ActiveReadingTime after restore. Five-minute idle cap; paused intervals excluded.
  Only Checkpoint/Pause save increases. Invalid/foreign/regressing samples ignore.
  Movement/completion/exit retire epoch. Explicit greater durations reset it paused.
- Final checkpoints wait behind unrelated NONMOVING storage; failures retain retry
  plus maximum slot. One first-wins departure/completion waits. Late samples behind
  accepted movement/import reject; host MUST pause/checkpoint first.
- Android CoreSession uses elapsedRealtime and 15-second Handler checkpoints.
  Activity resume/pause gates accounting separately from analyzer start/stop;
  interactions include sustained touch/generic scrolling. Restore waits for free
  storage and callback before Resume. Settings/background/reader actions Pause first.
- Reader/Settings expose Retry; pending saves disable presets. close() rejects UI
  dispatch immediately, retains callbacks until storage_busy and BOTH pending flags
  clear, then releases native. Failed obligations after onCleared retry each second;
  persistent failure retains the live session. Abrupt death can lose uncommitted time.
- CompleteReaderPage freezes document/page/position/preset/AnalysisId/timestamp,
  drains checkpoints, then FIFO reserves ID + whole-page durable time atomically.
  CompletePage commits occurrences/New-only Known/history/time/position together.
  MissingAnalysis blocks legacy completion without rescan; base analyses only.
  Worker reloads persisted content; failed read retries same frozen command.
  AlreadyRecorded has no effects/notice.
- Migrations 0005–0011 preserve history/vocabulary/immutable A/B/C anchors/edits/
  hashes/time. Document deletion preserves history; vocabulary deletion cascades it.
- NativeBridge requires owned verified FD+1; Rust rehashes/maps/closes all paths.
  Lazy analyzer stays on FIFO; background releases after accepted work, recreation
  retains warmth. Native close drains jobs. Never modify live Full inodes.
  Sudachi.rs v0.7.0/android-loader-v1 + Full 20261015 pinned; 138 files verified.
  Full outside Git; notices bundled. SUDACHI_MEASUREMENTS.md has limits.

## Next tasks (ordered)
1. Phase 3 tappable tokens + five direct status choices: begin with a small typed
   reader-token/status projection from stored Mode B; lookup opening must not mutate.
2. Verify JMdict English/JMnedict versions/licenses and reproducible offline index.
3. User-run physical arm64 validation once usable. Formats/CI remain open.

## Verification / environment
- PASS: 219 Rust tests (111 storage); real-Full host Kotlin/JNI + Unicode exchanges;
  strict Clippy/fmt/Kotlin/diff, reproducible Kotlin generation, both-ABI debug/test
  APKs + strict Android lint. All 15 selected API-36 x86_64 reader/clock/paste tests
  pass (5m42s build/run). Full suite not rerun. Exact commands: DEVELOPMENT.md latest.
- Initial Kotlin warning and stale reread fixture anchor fixed; final checks pass.
  Prior process/reboot clock recovery not rerun. No schema/dependency change;
  generated Event/View add keyed book-completion acknowledgment/notice. No local
  blocker; no release/tablet/arm64 rerun. Nostr empty; ask Q188 before forwarding.
