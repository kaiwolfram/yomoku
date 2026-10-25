# Resume here
Updated: 2026-10-26. EPUB repository verified; app picker remains paste/TXT.
Read AGENTS.md, plan phases 2–4 and PRODUCT_SPEC.md sections 3–4.
## Working
- Pure Crux routes/Library/Settings; Render-only UI; FIFO SQLite/analysis. Starts Library.
- Exact chapter-local UTF-8 position/Library percentage + five presets persist;
  reflow preserves offset. Scroll resets on navigation/reopen; column max 720dp.
  Next/Previous/counter below content; net 48dp horizontal swipes need no bottom.
  Previous saves position only. Busy/pending gates reject duplicates; rereads count.
- Paste atomically deduplicates SHA-256, saves original Mode B + unique Japanese
  registrations. Existing preserves title/status/progress without rescan/writes.
  Created/Existing show dismissible Library summary; never auto-open.
- load_reader_snapshot/load_paste are read-only. Explicit open_reader_snapshot
  atomically resets finished books to page 1/preset/Mode B; failures roll back.
  Migration-0011 finish marker survives position/reflow/time; deletion resets it,
  preserving history. Missing analysis never rescans; legacy backfill needs EOF.
- Token reads atomically load original B/live statuses/metadata + clipped UTF-8
  ranges; completion uses token start. Deleted IDs absent. Whole-chapter validation
  per lookup (performance unmeasured); base-only. Compose FIFO/JNI loads only after
  busy/pending saves clear. Current/pending/result keys must match; movement/reflow/
  refresh invalidate snapshots. UTF-8→UTF-16 validates scalars/identity/text; invalid
  is plain. Known/absent unhighlighted. Failed reads require explicit Retry.
- SetReaderStatus validates loaded key/ID/time outside Settings. Atomic status/
  history updates repetitions; same-status no-op. Deleted IDs never register.
  Failure retains snapshot for explicit repeat; both outcomes drain final time.
- Phone sheet/wide panel shows surface/readings/base, five choices and expanded
  restriction-aware English senses/readings/compact POS, separate names/categories/
  lifetime counts. Opening is read-only. Busy/pending/absent disables status choices;
  navigation/recreation/invalidation dismisses; errors survive clock saves.
- Clock restore reserves FIFO revision; absolute checkpoint retries freeze payload/
  revision, coalesce maxima and chain revisions. Anchored samples enforce five-minute
  idle cap/pause/background/validity. Reflow preserves deadline; movement retires epoch.
- CoreSession uses elapsedRealtime, 15s Handler checkpoints, Activity resume/pause,
  scrolling and restore-before-Resume. Actions Pause/drain first. Reader/Settings
  retry; pending saves disable presets. close retains callbacks until busy/BOTH
  pending flags/direct cancellation clear; retries after onCleared. Abrupt kill can lose pending.
- Completion freezes page/position/preset/AnalysisId/time, drains final time, then
  FIFO reserves ID + whole-page duration for exact retries. One transaction commits
  occurrences/New-only Known/history/time/position/finish. Missing blocks. Reload
  retries exact command; AlreadyRecorded/zero silent. Positive toast once. New final
  Recorded returns Library + consumed 1.4s confetti, including zero/rereads.
- NativeBridge consumes owned verified FD+1; Rust rehashes/maps/closes all paths.
  FIFO releases Full in background; keep live mapped inodes immutable. Pinned
  2026-10-25 JMdict English/JMnedict + sources/notices/offline credits bundled.
  Lazy index until close/query failure; reader-word B/status/count reads are atomic.
  Full current/pending/selected-token identity gates JNI attach; late results cannot open.
- TXT strict UTF-8/Shift-JIS/Aozora uses atomic migration-0012 original-byte SHA-256,
  filename/encoding/ruby/Mode B/unique registration. Duplicate skips decode/Full;
  new parsing validates before Full and repeats atomically. Typed FIFO/JNI gates saves.
- Android multiple-TXT retained helper reads one URI off main and sequences imports/
  encoding/Skip. Filename/X-of-N + processed bar survive recreation/background;
  errors/duplicates/skips count, encoding retries do not. Summary retains results.
  No persisted queue/progress; teardown drops unsubmitted work; name precedes bytes.
- cancelFileImport() bypasses FIFO off main (COMMIT may wait); TXT alias retained.
  Fresh shared handle precedes submission. Silent full-file rollback; winning/prior
  commits stay. Android clears remaining/input, invalidates late reads, retires
  executors, waits BOTH native return + current outcome, including draining close.
- dataSync lease tracks counts/name only; OS limits apply. Encoding releases it;
  notification permission never blocks. Timeout stops lease before Cancel. Notices
  count processed/total; taps no reader. HOME/SIGKILL granted/denied retain only commits.
- SRT repo/FIFO/JNI: shared strict decoder (no Aozora), hash/filename/LF/cues/B/unique
  registration atomic; TXT/SRT retains first interpretation. Typed errors/save gates; picker pending.
- EPUB bounded parser ignores CSS; migration 0014 now atomically stores original
  bytes/hash/title/filename, spine chapters/formatting/ruby/links, shared cover/image
  bytes/placements, per-chapter original B and unique registrations. Cancellation/
  later-chapter failures roll back all rows/IDs. Read-only reopen checks digests,
  counts/indexes/anchors/ownership without parsing. Library progress sums all chapters.
## Next tasks (ordered)
1. Generalize single-text import references + reader snapshots/completion/time/tokens
   for ordered chapters, then EPUB FIFO/typed JNI. Old APIs reject EPUB; use generic
   find_imported_file_id/EPUB Existing(DocumentId), never truncate chapters.
2. Mixed EPUB/TXT/SRT picker/summary/Cancel, reader images/ruby; CI independent.
3. User-run physical arm64 validation once usable.
## Verification
- PASS: 379 data-free Rust tests, Clippy/fmt/diff, both Android storage targets;
  fourteen real-Full analysis + one worker fixture per owned/mapped loader;
  existing real-Full Kotlin/JNI + seven Unicode exchanges. Commands: DEVELOPMENT.md end.
- No blocker. APK/Android lint/device/codegen-repeat/index/Python/release/reboot/
  process-death/tablet/dark not rerun; physical arm64 unverified. Nostr empty/Q188.
