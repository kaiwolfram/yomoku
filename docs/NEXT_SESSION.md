# Resume here

Updated: 2026-10-24. Analyzed completion intent transport verified.
Read AGENTS.md, plan phases 2–4 and PRODUCT_SPEC.md sections 3–4.
Update docs/commit each verified step. Physical arm64 is the later user trial.

## Working
- Pure Crux owns navigation/Library/Settings and typed effects; FIFO owns SQLite/
  analysis. Only Render reaches UI. Fresh sessions start Library.
- Exact UTF-8 position/Library percentage and five presets persist; reflow keeps
  offset. Scroll resets; column 720dp. NO Next/Previous/swipes/Finish Book yet.
- Analyzed paste atomically deduplicates exact SHA-256, saves Mode B and registers
  unique Japanese surfaces. Duplicates reuse oldest live exact match without
  rescan/progress/status changes. Created/Existing return to dismissible Library
  summary, never open reader. Source content/IDs/provenance stay immutable.
- load_reader_snapshot reads document/exact position, preset and earliest stored
  Mode B AnalysisId in one read-only transaction. No analyzer/token scan/writes.
  Identity stays fixed through reflow/position/plain loads; exit clears it.
  ReaderAnalysis::Missing stays readable; bad headers/settings error, no repair.
- RestoreReadingTime separately reserves an exact-offset revision on FIFO.
  CheckpointReadingTime accepts anchored absolute duration; failures retain exact
  payload/prior revision for RetryReadingTimeCheckpoint. Newer updates coalesce
  to one greatest-duration slot; saves chain with returned revisions. First
  position-only move/CloseDocument/OpenPaste/tab exit waits for accepted saves.
  View exposes duration + reading_time_save_pending, never revision. Reflow/
  same-offset saves preserve time; movement requires fresh restore. No history.
- NEW CompleteReaderPage supplies visible document/page/position + Unix timestamp.
  Core freezes preset/AnalysisId/timestamp and drains checkpoints before FIFO
  prepare_page_completion atomically reserves ID + whole-page durable time.
  Then CompletePage runs the analyzed transaction. RetryReaderCompletion repeats
  the exact failed stage/payload; all other storage stays blocked. One first-wins
  departure waits for completion. Stale/duplicate/negative/EOF requests ignore;
  MissingAnalysis explicitly blocks legacy completion without rescan/fallback.
- Completion freezes every Japanese repetition's anchors/surface/reading/lemma;
  page owns token starts. New-only Known events, encounters, history, consumed
  time and position commit together. Worker loads persisted document afterward;
  failed read retries same command, AlreadyRecorded repeats no side effects or
  notice. New Recorded positive counts expose command-keyed CompletionNotice;
  dismissal/new completion/exit clear it. Success retires consumed timer token.
  Reread reserves a new command. Base analyses only; overrides still unresolved.
- Platform MUST restore, pause accounting, submit final checkpoint before movement/
  completion/departure and await both pending flags plus busy before teardown.
  Unrelated busy events still ignore. No Android timer/final-save producer yet.
- Pure active_time uses monotonic ms, five-minute idle cap and interactions;
  restored durations start paused. Migration 0009 stores exact-offset durations/
  revisions. Reflow assigns anchor once (boundary following, EOF final).
  Restore only anchor duration; completion freezes whole page total.
- Migrations 0005–0010 preserve command/page/position/history, vocabulary, immutable
  A/B/C anchors, reversible edits, hashes and time. Deleting documents preserves
  history; vocabulary deletion cascades its history. No new schema this step.
- NativeBridge requires owned verified FD+1 (zero failure); Rust rehashes/maps/
  closes every path. No production text-only fallback. Lazy non-Send analyzer
  stays on FIFO; foreground reuse, background release after accepted work,
  recreation retains warmth. Native close drains submitted jobs, not unsubmitted
  core obligations. Library never eagerly loads Full; live inodes immutable.
- Sudachi.rs v0.7.0/android-loader-v1 + Full 20261015 pinned; 138 files verified.
  Full outside Git; APK bundles notices/data. SUDACHI_MEASUREMENTS.md has limits.
  No physical-arm64/LMKD claim. No inferred TXT chapters; files hash original bytes.

## Next tasks (ordered)
1. Connect Android monotonic clock/lifecycle/interactions/periodic/final saves;
   handle restores/unrelated busy work, failures and drain before native teardown.
   Verify actual close/reopen/reboot; current transport alone does not establish it.
2. Connect Next/Previous/swipes/counter and positive completion toast, then permanent
   finished-at-least-once/Finish Book/reopen-at-page-1. Keep final payload frozen.
3. User-run physical arm64 validation once usable. Formats/CI remain open.

## Verification / environment
- PASS: workspace build, 200 data-free Rust tests (103 storage), fmt/strict Clippy,
  real-Full Kotlin/JNI + Unicode/analysis exchanges, both-ABI debug/test APK builds,
  Kotlin format, strict Android lint and diff check. Three new core, one repository
  and blocked-SQLite FFI tests cover drain/failure/retry/reread; JNI completes with
  unavailable dictionary. Initial compile/Clippy/preview omissions fixed. Gradle
  socket denial passed with escalation; benign Crux/ktfmt shutdown warnings remain.
- No instrumentation, standalone Full suite, release/probes/tablet/arm64 rerun.
  Commands: DEVELOPMENT.md latest section; Rust PATH workaround. Native build/
  generation/test runners sequential. Never modify mapped Full. No open failure,
  external dependency version change or local blocker. Nostr empty; ask Q188 first.
