# Resume here

Updated: 2026-10-24. Read-only stored reader-token/status foundation verified.
Read AGENTS.md, plan phases 2–4 and PRODUCT_SPEC.md sections 3–4.

## Working
- Pure Crux owns navigation/Library/Settings and typed effects; FIFO owns SQLite/
  analysis. Only Render reaches UI. Fresh sessions start Library.
- Exact UTF-8 position/Library percentage and five presets persist; reflow keeps
  offset. Scroll resets on navigation/reopen; column capped at 720dp.
- Next/Previous/counter below content; left/right swipes navigate without a
  bottom requirement. Axis-specific drags require 48dp net movement on release.
  Next completes the departing page; anchored Previous derives the preceding page
  in Rust and only saves position. Both use retained dispatch/Pause/checkpoint drain.
  Busy/pending saves disable actions. Duplicate callbacks count once; rereads again.
  Positive CompletionNotice shows a short toast and is consumed once; zero is silent.
  Finish Book/left swipe drains clock + atomic success before Library return; failure
  keeps Retry. Newly Recorded final commands expose book_completion_notice even at
  zero New/rereads; AlreadyRecorded is silent. Compose consumes before local 1.4s
  confetti; recreation cannot replay. Final return keeps positive toast. Migration
  0011's permanent marker/checkmarks survive backward/reflow/time saves independently
  of position. Deletion resets it; history stays. Backfill requires live EOF history.
- Analyzed paste atomically deduplicates exact SHA-256, saves Mode B and registers
  unique Japanese surfaces. Duplicates reuse oldest live exact match without
  rescan/progress/status changes. Created/Existing show dismissible Library summary.
- load_reader_snapshot/load_paste stay read-only, especially completion reloads.
  OpenReader uses open_reader_snapshot: one immediate transaction resets finished
  books to page 1 and reads marker/preset/original Mode B identity. Pending time/history
  stay; failure rolls back. Unfinished offsets and retained recreation stay exact.
  Identity stays fixed through reflow/position/plain loads; Missing never rescans.
- load_reader_tokens(document_id, page) separately reads original Mode B + current
  global statuses in one read transaction. Pure projection keeps original indices/
  full anchors/surfaces/readings/forms and clipped page-local UTF-8 display spans.
  Crossing tokens remain selectable from either fragment, with completion ownership
  unchanged. Missing/deleted entries stay absent; lookup never writes or rescans.
  Whole-chapter validation per lookup; performance unmeasured. Crux/FIFO/FFI/Compose
  unwired; reject stale document/page/analysis results there.
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
  MissingAnalysis blocks legacy completion; base only. Failed post-commit reload
  retries the same command; AlreadyRecorded has no effects/notice.
- NativeBridge requires owned verified FD+1; Rust rehashes/maps/closes all paths.
  Lazy analyzer stays on FIFO; background releases after accepted work, recreation
  retains warmth. Native close drains jobs. Never modify live Full inodes.
  Sudachi v0.7.0/android-loader-v1 + Full 20261015 pinned; 138 files verified.
  Full outside Git, notices bundled; limits: SUDACHI_MEASUREMENTS.md.

## Next tasks (ordered)
1. Wire typed token/status reads + manual changes through core/FIFO/FFI; then
   Compose taps/five direct choices. Start with load_reader_tokens; lookup must not mutate.
2. Verify JMdict English/JMnedict versions/licenses and reproducible offline index.
3. User-run physical arm64 validation once usable. Formats/CI remain open.

## Verification / environment
- PASS this slice: workspace build, 229 Rust tests (117 storage), strict Clippy/fmt/
  diff; tools/test-sudachi.sh (11 Full fixtures + 1 runtime per owned/mapped loader).
  Four pure + six repository tests verify Unicode/reflow/status/deletion/corruption;
  query-only SQLite + unchanged bytes prove no writes. DEVELOPMENT.md has commands.
- No schema/dependency/event/view codec change. No test failures or local blocker.
  Android/JNI/lint not rerun this slice; prior 15 API-36 x86_64 reader/clock/paste tests
  and both-ABI APKs passed. No process/reboot/release/tablet/arm64 rerun. Nostr empty;
  ask Q188 before forwarding.
