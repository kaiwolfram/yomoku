# Resume here

Updated: 2026-10-24. Typed core reader-clock driver verified.
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
  CheckpointReadingTime accepts absolute durations; failures retain exact payload/
  prior revision for RetryReadingTimeCheckpoint. Newer updates coalesce to one
  greatest-duration slot and saves chain with returned revisions. View exposes
  committed duration + save-pending, never revision. No checkpoint creates history.
- NEW ObserveReaderClock(document/exact position, monotonic_ms, action) drives pure
  ActiveReadingTime after restore. Resume/Interact update hidden accounting;
  Checkpoint/Pause save increased absolute durations. Five-minute idle cap; paused
  intervals never count. Invalid/foreign/regressing samples ignore. Settings only
  accepts Pause. Reflow/same-offset saves preserve clock; movement/completion/exit
  retire epoch. Explicit greater saved duration resets it paused. No Android producer.
- Final checkpoints also wait behind unrelated NONMOVING storage. Success drains
  the maximum slot; failure retains it for RetryReadingTimeCheckpoint. One first-wins
  departure/completion waits. Late samples behind already accepted movement/import
  reject: host MUST pause/checkpoint first. No general event queue.
- CompleteReaderPage freezes visible document/page/position/preset/AnalysisId/Unix
  timestamp, drains checkpoints, then FIFO atomically reserves ID + whole-page time.
  CompletePage commits every Japanese repetition's frozen anchors/surface/reading/
  lemma, New-only Known events, encounters/history/time/position together. Retry
  repeats exact failed preparation/transaction. MissingAnalysis blocks legacy
  completion without rescan. Stale/duplicate/negative/EOF requests ignore.
- Worker reloads persisted content afterward; failed read retries same command.
  AlreadyRecorded repeats no side effects/notice. Positive Recorded results expose
  command-keyed CompletionNotice. Dismiss/new completion/exit clear it; success
  retires consumed timer token. Reread reserves new command. Base analyses only.
- Android MUST restore, send Resume when ready, interactions/periodic Checkpoint,
  Pause before settings/background/movement/completion/departure, then await BOTH
  pending flags + busy before native teardown. CoreSession currently closes native
  immediately; no automatic observation/final-save producer or teardown ack yet.
- Migrations 0005–0010 preserve history/vocabulary/immutable A/B/C anchors/edits/
  hashes/time. Document deletion preserves history; vocabulary deletion cascades
  its history. Exact-offset durable durations/revisions are migration 0009.
- NativeBridge requires owned verified FD+1 (zero failure); Rust rehashes/maps/
  closes every path. No production text-only fallback. Lazy non-Send analyzer
  stays on FIFO; foreground reuse, background release after accepted work,
  recreation retains warmth. Native close drains submitted jobs only. Library
  never eagerly loads Full; live inodes immutable. Sudachi.rs v0.7.0/android-loader-v1
  + Full 20261015 pinned; 138 files verified. Full outside Git; notices bundled.
  SUDACHI_MEASUREMENTS.md has limits. No arm64/LMKD claim. No inferred TXT chapters.

## Next tasks (ordered)
1. Connect Android retained driver using ObserveReaderClock + monotonic/lifecycle/
   interactions/periodic/final samples. Schedule restore after busy work; handle
   failures and drain before native teardown. Verify actual close/reopen/reboot.
2. Connect Next/Previous/swipes/counter and positive completion toast, then permanent
   finished-at-least-once/Finish Book/reopen-at-page-1. Keep final payload frozen.
3. User-run physical arm64 validation once usable. Formats/CI remain open.

## Verification / environment
- PASS: 206 Rust tests, build/fmt/strict Clippy, real-Full Kotlin/JNI, reproducible
  generation, Kotlin format, both-ABI debug/test APKs, strict Android lint and diff.
  Initial retry-slot regression fixed. Benign Crux/ktfmt warnings remain.
- Current commands/results: DEVELOPMENT.md latest section. Native build/generation/
  test runners sequential; Rust PATH workaround. Never modify mapped Full.
- No device timer/close/reboot claim. No external dependency/schema change or local
  blocker. Nostr configuration empty; ask Q188 before forwarding.
