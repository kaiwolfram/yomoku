# Resume here
Updated: 2026-10-26. EPUB FIFO/typed JNI import added; app picker stays paste/TXT.
## Working
- Pure Crux routes/Library/Settings; Render-only UI; FIFO SQLite/analysis. Starts Library.
- Exact chapter-local UTF-8 position/whole-book Library percentage + five presets
  persist; reflow preserves offset. Scroll resets on movement/reopen; column 720dp.
  Next/Previous/counter below content; net 48dp swipes need no bottom. Previous saves
  position only. Busy/pending gates reject duplicates; deliberate rereads count.
- Paste atomically deduplicates SHA-256, saves original B + unique registrations.
  Existing preserves title/status/progress without rescan/writes. Created/Existing
  show dismissible Library summary; never auto-open. Nostr config remains empty.
- Generic read-only load versus explicit open: finished books reset to first
  chapter/page/preset/B atomically only on open. Finish survives movement/time/reflow;
  deletion resets it, preserving history. Missing never rescans.
- App/FIFO/typed bridge uses ReaderDocument/DocumentReaderSnapshot for loads,
  opens, saves and completion reloads, retaining all ordered chapters.
  Core validates/freezes keyed B/Missing per chapter on accepted open and selects
  current identity on movement; plain loads cannot replace it. Previous crosses chapters; global page counters/progress.
- Token reads load original B/live statuses/metadata + clipped UTF-8 ranges atomically;
  completion uses token start. Deleted IDs absent. Whole-chapter validation per lookup.
  Loads wait for busy/pending saves; full page/base/result identities gate callbacks.
  Movement/reflow/refresh invalidate. Strict UTF-8→UTF-16 failure renders plain text.
- Five manual choices atomically update status/history; same-status no-op. Deleted
  entries never register. Reads/choices require explicit retry after failure.
  Phone sheet/wide panel shows readings/base, expanded restriction-aware English
  senses/compact POS, separate names/categories/lifetime counts. Opening read-only;
  navigation/recreation dismisses; errors survive timer saves; busy/pending gates.
- Clock uses elapsedRealtime/15s checkpoints, restore-before-Resume, five-minute
  idle/pause/background cap, reflow deadlines and movement epochs. Absolute revisions
  freeze retries, coalesce maxima and drain before movement/exit. CoreSession close
  retains callbacks until busy/BOTH pending/direct Cancel clear; retries on teardown.
  Abrupt kill can lose pending. Time stays chapter-local, including EOF.
- Completion freezes page/position/preset/AnalysisId/time, drains final time then
  FIFO reserves ID + whole-page duration. One transaction commits occurrences,
  New-only Known/history/time/position/finish. Exact retries; AlreadyRecorded/zero
  silent, positive toast once. Chapter end advances to next chapter at 0; only new
  whole-book final completion returns Library + 1.4s confetti. Image placeholders
  complete without words. Fresh finished-book open resets to first chapter.
- NativeBridge consumes owned verified FD+1; Rust rehashes/maps/closes all paths.
  FIFO releases Full in background; live mapped inodes immutable. Pinned 2026-10-25
  JMdict English/JMnedict + notices/credits bundled. Word index lazy until close/query
  failure; full token/selection identities gate attach.
- TXT strict UTF-8/Shift-JIS/Aozora atomic original-byte hash, filename/encoding/ruby/
  Mode B/unique registrations. Duplicate skips decode/Full. SRT FIFO/JNI shares
  decoder (no Aozora), original hash/filename/LF/cues/B. First interpretation wins.
- Retained multiple-TXT helper reads one URI off main; sequences encoding/Skip/import.
  Filename/X-of-N/processed bar + summary survive recreation/background. Name before
  bytes. No persisted queue; teardown drops unsubmitted files, SIGKILL keeps commits.
- cancelFileImport bypasses FIFO off main (COMMIT may wait); TXT alias retained.
  Fresh attempt before submission; silent full-file rollback; winning/prior commits
  stay. Android clears inputs/remaining, ignores late reads and awaits BOTH native
  return + current outcome, including draining close. Dedicated cancellation executor.
- dataSync lease tracks counts/name only; OS limits apply. Encoding releases it;
  notification permission never blocks. Timeout stops lease before Cancel. Notification
  taps never open reader. No background queue/service restart after process death.
- EPUB bounded parser ignores CSS. Migration 0014 atomically saves source bytes/hash/
  metadata/ordered chapter formatting/ruby/links/shared images + chapter B/registration.
  Any failure/Cancel rolls back all rows/IDs. Loader validates saved digests/counts/
  anchors/ownership without parsing. Every reader operation verifies complete EPUB
  source/assets; large-book performance unmeasured. Reader asset payload still absent.
- Shared file preflight/Existing outcomes return Library references, validating the
  whole document atomically. Duplicate EPUB via TXT/SRT preserves renamed/later-chapter/
  finished state without Full/reset/writes. Corruption fails; later files continue.
- EPUB ImportEpub carries bytes/name, no text encoding; bounded parse before Full,
  atomic repository rechecks. InvalidEpub covers container errors. Shared cancel
  registry reaches queued/Full/chapter/draining work; cache evicts on engine failure.
  Cross-format preflight preserves complete first interpretation before name/parse.
## Next tasks (ordered)
1. Mixed EPUB/TXT/SRT picker/summary/Cancel, including format-specific failures and
   background notifications; never auto-open reader. Retain existing TXT behavior.
2. Reader formatting/ruby/images + external links/shared viewer. CI independent.
3. User-run physical arm64 validation once usable (still deferred).
## Verification
- 409 data-free Rust tests, Clippy/fmt/Kotlin fmt/diff. EPUB worker fixtures cover
  all write/later-chapter/deferred-COMMIT rollback, cancellation and file isolation.
- Real-Full Kotlin/JNI EPUB + existing suites/seven Unicode exchanges pass. Both-ABI
  APK/strict lint and eight API-36 persistence/clock tests pass; reports/logs under
  .local/test-reports/epub-worker/. Commands/results: DEVELOPMENT.md end.
- Repeated codegen matches. EPUB double-parse/integrity-read costs unmeasured.
  No new dependency/migration or open blocker/Q188. Deferred checks: DEVELOPMENT.md.
