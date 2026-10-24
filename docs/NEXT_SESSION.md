# Resume here

Updated: 2026-10-24. Android retained reader-clock/draining close verified.
Read AGENTS.md, plan phases 2–4 and PRODUCT_SPEC.md sections 3–4.
Update docs/commit each verified step. Physical arm64 is the later user trial.

## Working
- Pure Crux owns navigation/Library/Settings and typed effects; FIFO owns SQLite/
  analysis. Only Render reaches UI. Fresh sessions start Library.
- Exact UTF-8 position/Library percentage and five presets persist; reflow keeps
  offset. Scroll resets; column 720dp. NO Next/Previous/swipes/Finish Book yet.
- Analyzed paste atomically deduplicates exact SHA-256, saves Mode B and registers
  unique Japanese surfaces. Duplicates reuse oldest live exact match without
  rescan/progress/status changes. Created/Existing show dismissible Library summary.
  Source content/IDs/provenance stay immutable. Reader opening never loads Sudachi.
- load_reader_snapshot reads document/exact position, preset and earliest stored
  Mode B AnalysisId in one read-only transaction. Identity stays fixed through
  reflow/position/plain loads; exit clears it. Missing remains readable; no repair.
- RestoreReadingTime separately reserves an exact-offset revision on FIFO.
  Checkpoints are absolute durations; failures retain exact payload/prior revision
  for RetryReadingTimeCheckpoint. Newer updates coalesce to one maximum slot and
  chain returned revisions. View exposes committed duration + save-pending only.
  Checkpoints never create history. Reflow/same-offset saves preserve idle deadline.
- ObserveReaderClock(document/exact position, monotonic_ms, action) drives pure
  ActiveReadingTime after restore. Five-minute idle cap; paused intervals excluded.
  Only Checkpoint/Pause save increases. Invalid/foreign/regressing samples ignore.
  Movement/completion/exit retire epoch. Explicit greater durations reset it paused.
- Final checkpoints wait behind unrelated NONMOVING storage; failure retains exact
  retry + maximum slot. One first-wins departure/completion waits. Late samples
  behind accepted movement/import reject; host MUST pause/checkpoint first.
- NEW Android CoreSession uses elapsedRealtime + 15-second Handler checkpoints.
  Activity resume/pause gates accounting separately from analyzer start/stop;
  interactions include sustained touch/generic scrolling. Restore waits for busy
  work and successful callback before Resume. Settings/background/reader actions
  Pause first; fresh offsets restore independently; EOF starts no clock.
- Reader/Settings show Retry for time/completion failures; pending saves disable
  presets. close() rejects UI dispatch immediately but retains callbacks until
  storage_busy and BOTH pending flags clear, then releases native. Failed obligations
  after onCleared retry every second; persistent failure retains session while alive.
  Abrupt death can lose uncommitted time since last checkpoint; no OS guarantee.
- CompleteReaderPage freezes document/page/position/preset/AnalysisId/Unix timestamp,
  drains checkpoints, then FIFO atomically reserves ID + whole-page durable time.
  CompletePage commits every Japanese repetition's anchors/surface/reading/lemma,
  New-only Known events, encounters/history/time/position together. Exact retries;
  MissingAnalysis blocks legacy completion without rescan. Base analyses only.
- Worker reloads persisted content afterward; failed read retries same command.
  AlreadyRecorded has no side effects/notice. New positive Recorded exposes keyed
  CompletionNotice. Dismiss/new completion/exit clear it; reread reserves new command.
- Migrations 0005–0010 preserve history/vocabulary/immutable A/B/C anchors/edits/
  hashes/time. Document deletion preserves history; vocabulary deletion cascades it.
- NativeBridge requires owned verified FD+1 (zero failure); Rust rehashes/maps/
  closes all paths. No text-only fallback. Lazy non-Send analyzer stays on FIFO;
  background releases after accepted work; recreation retains warmth. Native close
  drains submitted jobs. Never modify live Full inodes. Sudachi.rs v0.7.0/
  android-loader-v1 + Full 20261015 pinned; 138 files verified. Full outside Git;
  notices bundled. SUDACHI_MEASUREMENTS.md has limits. No inferred TXT chapters.

## Next tasks (ordered)
1. Connect Next/Previous/swipes/counter and positive completion toast. Use the
   retained dispatch path so Pause precedes frozen completion; test duplicate inputs.
2. Permanent finished-at-least-once/Finish Book/reopen-at-page-1 and confetti.
3. User-run physical arm64 validation once usable. Formats/CI remain open.

## Verification / environment
- PASS: 206 Rust tests; debug/test APK builds; full 26-test Android suite followed
  by final 11 affected tests; strict Android lint, Kotlin format and diff checks.
- Real-clock close/new-process/reboot passes on API 36 x86_64: 259 ms restored after
  restart, then 268 ms restored after a confirmed new boot. No partial-page history.
  Exact commands/report: DEVELOPMENT.md latest section. Emulator left running.
- Initial Android locking fixture did not block native SQLite; corrected to final
  close assertions and explicit SQL-failure/coalescing retries. No open failure.
- No schema/dependency/codec change or local blocker. No release/tablet/arm64 or
  host Kotlin/JNI rerun. Nostr config empty; ask Q188 before forwarding.
