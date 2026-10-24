# Resume here

Updated: 2026-10-24. Compose word highlights/taps and direct status choices wired.
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
  even at zero New/on rereads; AlreadyRecorded is silent. Consume before 1.4s confetti;
  recreation cannot replay. Migration 0011 marker survives position/reflow/time;
  deletion resets it, history stays. Backfill needs EOF.
- Paste atomically deduplicates exact SHA-256, saves Mode B and registers unique
  Japanese surfaces. Existing reuses oldest live exact match without rescan or state
  changes. Created/Existing show dismissible Library summary, never auto-open.
- load_reader_snapshot/load_paste stay read-only, especially completion reloads.
  Explicit open_reader_snapshot atomically resets finished books to page 1, reading
  marker/preset/Mode B. Pending time/history stay; failure rolls back. Unfinished
  offsets/recreation stay exact. Missing analysis never rescans.
- Tokens read original B + live statuses atomically, preserving full metadata and
  clipped page-local UTF-8 spans. Crossing selections keep start-based completion.
  Missing/deleted stays absent, no writes/rescan. Whole chapter validated per lookup;
  performance unmeasured. Base-only; overrides remain future work.
- Compose requests absent tokens after busy/pending saves clear via gated FIFO/JNI;
  pending/result/current keys must match. Snapshots clear on refresh/movement/reflow.
  Failed reads require Retry even after clock saves clear errors. UTF-8→UTF-16 checks
  scalars/identity/clipped surface; invalid is plain text. Known/absent unhighlighted.
- SetReaderStatus validates current loaded key/existing ID/timestamp outside Settings.
  Atomic FIFO status/history commit updates visible repetitions; same-status no-op.
  Deleted ID clears references without registration. Failure preserves snapshot for
  explicit repeat; no replay. Both outcomes drain final time.
- Phone sheet/wide side panel show full surface/analyzer reading/base form and five
  choices. Opens never mutate; busy/pending/absent disables choices. Selection follows
  committed state; navigation/Settings/recreation/invalidation dismisses. Own-window
  content/container input extends idle clock. Definitions/names/count remain future.
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
  Lazy FIFO analyzer releases after accepted background work; recreation retains it.
  Native close drains jobs. Never modify live Full inodes. Sudachi v0.7.0/android-loader-v1
  + Full 20261015 pinned (138 files verified); Full outside Git, notices bundled.
  Measurement limits: SUDACHI_MEASUREMENTS.md.

## Next tasks (ordered)
1. Verify JMdict English/JMnedict versions/licenses and reproducible offline index.
2. Connect dictionary senses/readings/POS, separate names and lifetime encounter
   count to the word panel; preserve read-only opens and existing status choices.
3. User-run physical arm64 validation once usable. Formats/CI remain open.

## Verification / environment
- PASS: 238 Rust tests (117 storage), strict Clippy/Rust fmt/Kotlin fmt/diff;
  both-ABI debug/test APK builds, strict Android lint and all 24 selected API-36
  x86_64 phone tests (7 new word + 17 reader/navigation/clock/paste regressions).
- PASS: all 7 word tests also on wide landscape/dark; original AVD overrides restored.
- Commands/failure fixes: DEVELOPMENT.md; reports: .local/test-reports/reader-words/.
  No schema/dependency/FFI change. Host JNI/generation/standalone Full, release,
  separate process/reboot/physical arm64 not rerun. Nostr empty; Q188 before forwarding.
  AVD disconnect recovered without reset; Gradle/ADB sockets required escalation.
