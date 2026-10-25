# Resume here
Updated: 2026-10-25. Verified Android TXT batch Cancel.
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
  Immutable index opens lazily on FIFO, reused until close/query failure.
  load_reader_word reads original B/live statuses/lifetime count atomically;
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
  Scrollable Library summary retains results. Teardown drops unsubmitted work;
  no persisted queue/progress or reader ruby. Display name publishes before bytes.
- TXT cancellation shares a fresh repository handle before FIFO submission;
  AnalyzerLifecycle/native cancelTxtImport() bypass FIFO, off main (COMMIT can
  wait). Cancelled clears core busy without error/summary. Completed commits stay.
  Registry clears before callbacks; weak control survives accepted closing work
  without retaining worker. Queued/load/analysis/close/retry/JNI fixtures pass.
  Android Cancel stops submissions, clears remaining/encoding input, invalidates
  provider callbacks and retires blocked readers. Dedicated executor waits BOTH
  native return + outcome; winning commits stay. Current name/number and completed
  summary survive recreation. Native close also waits queued/direct cancellation.
## Next tasks (ordered)
1. Permitted background progress/completion notifications + actual process-death/
   no-resume acceptance. Verify committed files survive; no unfinished auto-resume.
2. EPUB/SRT parsers/original-file imports; mixed-format picker. CI is independent.
3. User-run physical arm64 validation once usable.
## Verification
- PASS: 304 Rust tests, strict Clippy/fmt/diff; both-ABI app/test APK builds, strict
  Android lint/Kotlin fmt; 14 API-36 x86_64 phone/light picker/cancellation/core/
  clock/navigation tests. Reports: .local/test-reports/txt-cancel/phone-final/.
- Commands/fixed failures: DEVELOPMENT.md last section. No blocker/new schema or permission.
  Host JNI/Full not rerun; process-death/arm64 unverified. Nostr empty; Q188 before forwarding.
