# Resume here

Updated: 2026-10-24. Typed reader-token reads wired through core/FIFO/FFI.
Read AGENTS.md, plan phases 2–4 and PRODUCT_SPEC.md sections 3–4.

## Working
- Pure Crux owns navigation/Library/Settings and typed effects; FIFO owns SQLite/
  analysis. Only Render reaches UI. Fresh sessions start Library.
- Exact UTF-8 position/Library percentage and five presets persist; reflow keeps
  offset. Scroll resets on navigation/reopen; column capped at 720dp.
- Next/Previous/counter below content; 48dp net horizontal swipes need no bottom.
  Next completes the departing page; Rust derives position-only Previous. Actions
  Pause/drain checkpoints first. Busy/pending saves disable duplicate callbacks;
  deliberate rereads count again. Positive CompletionNotice toasts once; zero silent.
- Finish Book/swipe drains final clock + atomic completion before Library return;
  failure keeps Retry. Newly Recorded final commands expose book_completion_notice
  even at zero New/on rereads; AlreadyRecorded is silent. Consume before local 1.4s
  confetti; recreation cannot replay. Migration 0011 marker/checkmark survives
  position/reflow/time saves. Deletion resets it; history stays. Backfill needs EOF.
- Paste atomically deduplicates exact SHA-256, saves Mode B and registers unique
  Japanese surfaces. Existing reuses oldest live exact match without rescan or state
  changes. Created/Existing show dismissible Library summary, never auto-open.
- load_reader_snapshot/load_paste stay read-only, especially completion reloads.
  Explicit open_reader_snapshot atomically resets finished books to page 1 and reads
  marker/preset/original Mode B identity. Pending time/history stay; failure rolls back.
  Unfinished offsets/recreation stay exact. Missing analysis never rescans.
- load_reader_tokens reads original B + live statuses in one read transaction.
  Full token indices/anchors/surfaces/readings/forms survive clipped page-local UTF-8
  spans; crossing tokens select from either page without changing completion ownership.
  Missing/deleted entries stay absent. No writes/rescan. Whole chapter validated each
  lookup; performance unmeasured. Base-only: overrides remain future work.
- Event::LoadReaderTokens(ReaderTokenKey) uses existing storage_busy gate/FIFO and
  generated Kotlin/JNI. Explicit request only; Compose does not request/render yet.
  Require visible document/page/analysis outside Settings. Attach only if pending,
  result and current keys match; discard cancelled errors/results. reader_tokens
  clears on refresh/navigation/position/completion callback or changed reflow span.
  Read failure can retry same key; even failure drains queued final time/departure.
- RestoreReadingTime reserves FIFO revision; absolute saves freeze exact retry
  payload/revision, coalesce newer maxima and chain revisions. View exposes duration,
  no history. Reflow/same-offset saves preserve the idle deadline.
- ObserveReaderClock uses anchored monotonic samples after restore. Five-minute idle
  cap, paused intervals excluded; Checkpoint/Pause save increases. Invalid/foreign/
  regressing samples ignore. Movement/completion/exit retire epoch. Greater explicit
  durations reset paused. One first-wins departure/completion waits for final saves.
- Android CoreSession uses elapsedRealtime + 15s Handler checkpoints, Activity resume/
  pause gating, scrolling interactions and restore-before-Resume. Settings/actions
  Pause first. Retry in reader/Settings; pending saves disable presets. close rejects
  dispatch, retains callbacks until busy/BOTH pending flags clear, then frees native;
  failed obligations retry each second after onCleared. Abrupt kill can lose pending.
- Completion freezes document/page/position/preset/AnalysisId/timestamp, drains time,
  then FIFO reserves ID + whole-page durable time. One transaction commits occurrences,
  New-only Known/history/time/position/finish. Missing blocks; base only. Post-commit
  reload failure retries exact command; AlreadyRecorded has no effects/notice.
- NativeBridge requires owned verified FD+1; Rust rehashes/maps/closes all paths.
  Lazy analyzer stays on FIFO; background releases after accepted work; recreation
  retains warmth. Native close drains jobs. Never modify live Full inodes.
  Sudachi v0.7.0/android-loader-v1 + Full 20261015 pinned; 138 files verified.
  Full outside Git, notices bundled; limits: SUDACHI_MEASUREMENTS.md.

## Next tasks (ordered)
1. Wire typed manual status changes through core/FIFO/FFI using existing repository;
   then Compose token taps/five direct choices. Opening lookup must not mutate.
2. Verify JMdict English/JMnedict versions/licenses and reproducible offline index.
3. User-run physical arm64 validation once usable. Formats/CI remain open.

## Verification / environment
- PASS: 234 Rust tests (117 storage), strict Clippy/fmt/diff; real-Full Kotlin/JNI
  including token metadata/New-to-Known, analyzer-free reopen and Unicode exchanges;
  repeat Kotlin generation and Kotlin formatting. DEVELOPMENT.md has exact commands.
- PASS: both-ABI debug/test APK builds, strict Android lint and 11 API-36 x86_64
  core/reader/clock regressions after restarting a disconnected emulator (2m38s).
- No schema/dependency change. New typed Event/View/Storage payloads. Standalone Full
  fixture suite, process/reboot/release/tablet/arm64 not rerun. Nostr empty; Q188 before
  forwarding. No open failures/local blocker; Gradle socket denial required escalation.
