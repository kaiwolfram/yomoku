# Resume here

Updated: 2026-10-24. Checkpoint coalescing/departure ordering verified.
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
- load_reader_snapshot reads document/exact position, preset and earliest stored
  Mode B AnalysisId in one read-only transaction. No analyzer/token scan/writes.
  Identity stays fixed through reflow/position/plain loads; reader exit clears it.
  ReaderAnalysis::Missing explicitly identifies readable legacy absence. Bad
  headers/settings error without fallback/repair. No legacy forward policy yet.
- RestoreReadingTime separately reserves an exact-offset revision token on
  FIFO; CheckpointReadingTime accepts anchored absolute duration. Core retains
  frozen payload/prior revision on failure; RetryReadingTimeCheckpoint resubmits
  exactly. Other storage commands stay blocked until
  success. View exposes committed anchor/duration plus reading_time_save_pending,
  never revisions. Negative FFI durations reject; foreign/decreasing saves ignore.
  Reflow/same-offset saves preserve time; movement clears it for fresh restore.
  NEW Updates during an unresolved checkpoint coalesce to one greatest-duration
  slot; success chains it with the returned revision. Position-only movement,
  CloseDocument/OpenPaste/tab exit defer until every accepted checkpoint saves.
  First departure wins; failures retain queue/departure for explicit retry.
  Cancelled restores cannot attach to closed readers. No history/analyzer/timer.
  Unrelated busy operations still ignore events. Platform must restore, send its
  final checkpoint before departure and await drain before native teardown.
- Pure active_time takes monotonic ms, five-minute idle cap and interactions;
  restored durations start paused in a fresh epoch. Migration 0009 stores exact
  offset durations/revisions. Reflow assigns each anchor once (boundary following,
  EOF final). Restore only anchor duration; freeze whole page total for completion.
- record_analyzed_page_completion freezes AnalysisId + command/page/date/time.
  Every Japanese repetition retains anchors/surface/reading/lemma; tokens belong
  to page containing start. New-only Known events link to command. History/time
  consumption/encounters/status/position commit together. Recorded returns
  newly_known; AlreadyRecorded repeats nothing. Replay validates frozen payload
  before live content. Deletion preserves document history; vocabulary deletion
  cascades its history. Only absent departing-page surfaces register on completion.
  Base analyses only; effective override resolution/completion intents remain open.
- NativeBridge requires verified owned FD+1 (zero failure); Rust rehashes/maps and
  closes FD on every path. No production text-only fallback. Lazy non-Send analyzer
  stays on FIFO; healthy SQL retries reuse it. onStart permits cache; onStop releases
  after accepted work, except recreation retains warmth. onCleared drains/closes
  without joining UI. Library never eagerly loads Full; live inodes immutable.
- Created/Existing show dismissible Library summary, never open reader. Settings/
  refresh/recreation retain it; dismissal/tab/new paste/reader clear it.
- Sudachi.rs v0.7.0/android-loader-v1 + Full 20261015 pinned; 138 files verified.
  Full outside Git; APK bundles data/notices. Measurements: SUDACHI_MEASUREMENTS.md.
  No physical-arm64/LMKD claim. Paste content/IDs/provenance immutable; no inferred
  TXT chapters. File imports must hash original bytes. No dictionary/vocab/edit UI.

## Next tasks (ordered)
1. Sequence analyzed completion intents after checkpoint drain through
   core/FIFO/FFI. Freeze command IDs/timestamp/page total/AnalysisId across retries;
   return newly_known only on new success. Handle Missing explicitly, no rescan.
2. Connect Android monotonic clock/lifecycle/interactions/periodic/final saves;
   handle unrelated busy work and await drain before native teardown. Verify
   close/reopen/reboot, then Next/Previous/swipes and Finish Book.
3. User-run physical arm64 validation once usable. Formats/CI remain open.

## Verification / environment
- PASS: workspace build, 195 data-free Rust tests (102 storage), fmt/strict Clippy,
  real-Full Kotlin/JNI with seven Unicode/two analysis exchanges, negative-duration
  rejection, Android debug/test APK builds and strict lint, diff check. Three new
  core tests + blocked-SQLite async FFI test cover maxima, frozen retries, chained
  revisions, deferred departure and reopen with no history. No open failure,
  dependency/schema/public-codec change. Benign Crux shutdown warnings persist.
- No instrumentation, standalone Full suite, release/probes/tablet/arm64 rerun.
  Actual Android timer/close/reboot remains unverified. Commands: DEVELOPMENT.md
  latest section; Rust PATH workaround. Native builds/generation/tests sequential.
  Never modify mapped Full. Gradle/ADB may need socket escalation; swangle documented.
- Nostr config empty; ask Q188 before forwarding. No local reader blocker.
