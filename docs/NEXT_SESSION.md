# Resume here
Updated: 2026-10-25. SRT repository verified; app imports remain paste/TXT.
Read AGENTS.md, plan phases 2–4 and PRODUCT_SPEC.md sections 3–4.
## Working
- Pure Crux owns routes/Library/Settings; only Render reaches UI. FIFO owns SQLite/analysis. Fresh sessions start Library. No I/O/SQL in core.
- Exact UTF-8 position/Library percentage + five presets persist; reflow preserves
  offset. Scroll resets on navigation/reopen; column capped at 720dp. Next/Previous/
  counter below content; 48dp net horizontal swipes require no bottom. Previous
  saves position only. Busy/pending gates reject duplicate callbacks; rereads count.
- Paste atomically deduplicates exact SHA-256, saves original Mode B + unique
  Japanese registrations. Existing reuses oldest live match without rescan/writes.
  Created/Existing show dismissible Library summary, never auto-open.
- load_reader_snapshot/load_paste stay read-only. Explicit open_reader_snapshot
  atomically resets finished books to page 1/preset/Mode B; failure rolls back.
  Permanent migration-0011 finish marker survives reflow/position/time, resets on
  deletion; history remains. Missing analysis never rescans; EOF needed for backfill.
- Tokens atomically read original B/live statuses/full metadata + clipped UTF-8
  ranges; completion uses token start. Deleted IDs absent. Whole-chapter validation
  per lookup (performance unmeasured); base-only for now.
- Compose requests after busy/pending saves clear through gated FIFO/JNI. Current/
  pending/result keys must match; refresh/movement/reflow invalidates snapshots.
  Failed reads need Retry even when clock saves clear errors. UTF-8→UTF-16 checks
  scalar/identity/clipped text; invalid is plain. Known/absent unhighlighted.
- SetReaderStatus validates loaded key/ID/time outside Settings. Atomic status/
  history updates repetitions; same-status no-op. Deleted IDs never register.
  Failure retains snapshot for explicit repeat; both outcomes drain final time.
- Phone sheet/wide panel: surface/reading/base form, five choices; read-only open,
  busy/pending/absent disables choices. Navigation/recreation/invalidation dismisses.
  Input extends idle clock; expanded restriction-aware senses/readings, compact POS,
  separate names/categories/lifetime count. Failure requires explicit Retry.
- Clock restore reserves FIFO revision; absolute checkpoint retries freeze payload/
  revision, coalesce maxima and chain revisions. Anchored samples enforce five-minute
  idle cap/pause/background/validity. Reflow preserves deadline; movement retires epoch.
- CoreSession uses elapsedRealtime + 15s Handler checkpoints, Activity resume/pause,
  scrolling and restore-before-Resume. Actions Pause/drain first. Retry in reader/
  Settings; pending saves disable presets. close retains callbacks until busy/BOTH
  pending flags/direct cancellation clear; retries after onCleared. Abrupt kill can lose pending.
- Completion freezes page/position/preset/AnalysisId/timestamp, drains final time,
  then FIFO reserves ID + whole-page duration for exact retries. One transaction
  commits occurrences/New-only Known/history/time/position/finish. Missing blocks.
  Reload retries exact command; AlreadyRecorded/zero silent. Positive toast once.
  Final new Recorded returns Library + consumed 1.4s confetti (zero/rereads too).
- NativeBridge consumes owned verified FD+1; Rust rehashes/maps/closes all paths. FIFO releases analyzer in background; keep live Full inodes immutable.
- Pinned 2026-10-25 JMdict English/JMnedict + sources/notices bundled; offline credits.
  Lazy FIFO index until close/query failure; load_reader_word reads B/statuses/count atomically;
  all exact surface/base word/name metadata/restrictions retained. See architecture.
- LoadReaderWord/DismissReaderWord reach typed JNI; full current/pending/selected
  token identity gates attach. Live statuses refresh repetitions. Errors survive
  clock saves; outcomes drain time. Close/recreation dismiss; late results cannot open.
- TXT strict UTF-8/Shift-JIS/Aozora uses atomic migration-0012 original-byte
  SHA-256 identity, filename/encoding/ruby/Mode B/unique registrations. Duplicates
  skip decode/Full; new parsing validates before Full and repeats atomically.
  Typed ImportTxt reaches lazy FIFO/JNI with reader-save/clock gates.
- Android multiple-TXT picker reads one URI off-main; retained helper sequences
  atomic imports/UTF-8/Shift-JIS/Skip. Filename/X-of-N + processed-files bar survive
  recreation/background; errors/duplicates/skips count, encoding retries do not.
  Library summary retains results; teardown drops unsubmitted work. No persisted queue/progress
  or reader ruby. Display name publishes before bytes.
- TXT cancellation shares a fresh repository handle before FIFO submission;
  cancelTxtImport() bypasses FIFO off main (COMMIT may wait). Silent rollback;
  winning commits stay. Android clears remaining/input, invalidates late reads and
  retires executors; waits BOTH native return + outcome, including draining close.
  Registry clears before callbacks; name/number/summary retain on recreation.
- Platform dataSync lease: counts/name, no queue/restart; OS limits apply. Encoding
  releases it; permission/denial never blocks, timeout stops before Cancel. Notices
  count processed/total; taps no reader. HOME/SIGKILL granted/denied preserve only commits.
- SRT repo: strict shared UTF-8/Shift-JIS (no Aozora), original-file SHA-256,
  filename/LF/cue ranges/Mode B/unique registrations atomic. TXT/SRT identical bytes
  reuse first interpretation. Migration 0013/read-only source/Cancel; no encounters/time; runtime pending.
- EPUB: bytes → spine chapters/title/UTF-8 formatting/ruby/links + image/cover bytes.
  CSS ignored/U+FFFC images; bounded ZIP/XML/64-depth cap. No I/O/IDs/persistence.
  Remote images/custom entities/fallback chains fail; no reader wiring.
## Next tasks (ordered)
1. SRT FIFO/typed FFI imports; encoding outcomes, duplicate-no-Full, Cancel/isolation.
2. EPUB atomic chapters/assets/Mode B + runtime; mixed picker/reader and CI follow.
3. User-run physical arm64 validation once usable.
## Verification
- PASS: 353 data-free Rust tests, Clippy/fmt/diff, storage both ABIs; Full 13+1 per loader.
- Commands/env: DEVELOPMENT.md end. One corrected synthetic count; no open blocker.
- JNI/APK/lint/device/release/reboot not rerun; arm64 unverified. Nostr empty/Q188; EPUB limits: DATA_MODEL.md.
