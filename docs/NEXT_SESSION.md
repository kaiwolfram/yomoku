# Resume here

Updated: 2026-10-24. Finish checkmarks and explicit page-1 reopening verified.
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
  EOF disables Next. Migration 0011 atomically marks final-page completion finished.
  Marker crosses document/Library/reader snapshots; accessible checkmarks show it
  independently of position. Backward/reflow/time saves retain it; deletion resets
  it while history stays. Migration backfills live EOF history, never position alone.
- Analyzed paste atomically deduplicates exact SHA-256, saves Mode B and registers
  unique Japanese surfaces. Duplicates reuse oldest live exact match without
  rescan/progress/status changes. Created/Existing show dismissible Library summary.
  Source content/IDs/provenance stay immutable. Reader opening never loads Sudachi.
- load_reader_snapshot/load_paste stay read-only, especially completion reloads.
  NEW OpenReader uses open_reader_snapshot: one immediate transaction resets finished
  books to page 1 and reads marker/preset/original Mode B identity. Pending time/history
  stay; failure rolls back. Unfinished offsets and retained recreation stay exact.
  Identity stays fixed through reflow/position/plain loads; Missing never rescans.
- RestoreReadingTime separately reserves an exact-offset revision on FIFO.
  Checkpoints save absolute durations; failures retain exact payload/prior revision
  for RetryReadingTimeCheckpoint. Newer updates coalesce to one maximum slot and
  chain returned revisions. View exposes committed duration + save-pending only.
  Checkpoints create no history. Reflow/same-offset saves preserve idle deadline.
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
  CompletePage commits every Japanese repetition's anchors/surface/reading/lemma,
  New-only Known events, encounters/history/time/position together. Exact retries;
  MissingAnalysis blocks legacy completion without rescan. Base analyses only.
  Worker reloads persisted content; failed read retries same command. AlreadyRecorded
  has no effects/notice. New positive Recorded exposes keyed CompletionNotice.
- Migrations 0005–0011 preserve history/vocabulary/immutable A/B/C anchors/edits/
  hashes/time. Document deletion preserves history; vocabulary deletion cascades it.
- NativeBridge requires owned verified FD+1; Rust rehashes/maps/closes all paths.
  Lazy analyzer stays on FIFO; background releases after accepted work, recreation
  retains warmth. Native close drains jobs. Never modify live Full inodes.
  Sudachi.rs v0.7.0/android-loader-v1 + Full 20261015 pinned; 138 files verified.
  Full outside Git; notices bundled. SUDACHI_MEASUREMENTS.md has limits.

## Next tasks (ordered)
1. Finish Book return + brief confetti on EACH newly recorded final-page completion,
   including rereads/zero newly Known. Use command-keyed transient state, never the
   permanent flag alone. Test duplicate/retry/recreation and final clock drain.
2. User-run physical arm64 validation once usable. Formats/dictionary/CI remain open.

## Verification / environment
- PASS: 217 Rust tests (111 storage); host Kotlin/JNI + Unicode exchanges;
  strict Clippy/fmt/Kotlin/diff, both-ABI debug/test APKs and strict Android lint.
  14 distinct API-36 x86_64 Android tests verified: 13 regression passes + final
  focused PasteReaderTest pass after lazy-list key scrolling fixed the fixture.
  Prior full suite not rerun. Commands/fixed failures: DEVELOPMENT.md latest.
- Prior real-process/reboot clock recovery remains verified; not rerun this slice.
- Schema stays 0011; new OpenReader storage operation + finish fields in codecs.
  No local blocker; no release/tablet/arm64 rerun. Nostr empty; ask Q188 before forwarding.
