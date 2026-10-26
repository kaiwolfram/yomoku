# Resume here
Updated: 2026-10-26. EPUB reader text formatting/website links connected.
## Working
- Pure Crux routes/Library/Settings; Render-only UI; FIFO SQLite/analysis. Starts Library.
- Exact chapter-local UTF-8 position/whole-book Library percentage + five presets
  persist; reflow preserves offset. Scroll resets on movement/reopen; column 720dp.
  Next/Previous/counter below content; net 48dp swipes need no bottom. Previous saves
  position only. Busy/pending gates reject duplicates; deliberate rereads count.
- Imports atomically deduplicate original-byte SHA-256, store original B + unique
  registrations. Existing preserves title/status/progress without Full/rescan/writes.
  Created/Existing show dismissible Library summary; never auto-open. Nostr empty.
- Generic loads are read-only; explicit finished-book open alone resets first
  chapter/page/preset/B atomically. Finish survives movement/time/reflow; deletion
  resets it while preserving history. Missing analysis never rescans.
- App/FIFO/typed bridge uses ReaderDocument/DocumentReaderSnapshot for loads,
  opens, saves and completion reloads, retaining all ordered chapters. Core freezes
  keyed original B/Missing per chapter on accepted open, selects on movement;
  plain loads cannot replace it. Previous crosses chapters; global counters/progress.
- Token reads atomically load original B/live statuses/metadata + clipped UTF-8
  ranges; completion owns token start. Deleted IDs absent. Whole-chapter validation.
  Full page/base/result identities gate callbacks; movement/reflow/refresh invalidate.
  Strict UTF-8→UTF-16 failure falls back to plain text. Loads wait for pending saves.
- Five manual statuses atomically update history; same-status no-op. Deleted entries
  never register. Reads/choices need explicit retry. Phone sheet/wide panel shows
  original reading/base, expanded restriction-aware English senses/compact POS,
  separate names/categories/lifetime counts. Opening read-only; navigation dismisses.
- Clock uses elapsedRealtime/15s checkpoints, restore-before-Resume, five-minute
  idle/background cap and movement epochs. Absolute revisions freeze retries,
  coalesce maxima and drain before movement/exit. CoreSession close retains callbacks
  until busy/BOTH pending/direct Cancel clear; retries teardown. Kill may lose pending.
- Completion freezes page/position/preset/B/time, drains final time then FIFO reserves
  ID + duration. One transaction writes occurrences, New-only Known/history/time/
  position/finish. Exact retries; AlreadyRecorded/zero silent, positive toast once.
  Only new final-book completion returns Library + 1.4s confetti. Images count no words.
- NativeBridge consumes owned verified FD+1; Rust rehashes/maps/closes every path.
  FIFO releases Full in background; mapped inodes immutable. Pinned 2026-10-25
  JMdict English/JMnedict + notices bundled; word index lazy until close/query failure.
- TXT strict UTF-8/Shift-JIS/Aozora and SRT shared decoder (no Aozora) preserve
  original bytes/filename/encoding/canonical ruby or cues/B. First interpretation wins.
- Retained FileImports reads one mixed EPUB/TXT/SRT URI off main, routes literal
  case-insensitive extensions; wildcard picker accepts generic MIME. TXT/SRT have
  separate encoding/Skip/errors; EPUB none. Filename/X-of-N/progress/summary survive
  recreation/background. Name before bytes. No persisted queue/process restart.
- Direct off-main cancelFileImport bypasses FIFO; COMMIT may wait. Silent full-file
  rollback; winning/prior commits stay. Android ignores late reads and awaits BOTH
  native return + outcome before clearing/draining. Dedicated cancellation executor.
- dataSync lease tracks counts/name only; OS limits apply. Encoding releases it;
  permission never blocks. Timeout stops lease then Cancel. Notification tap opens Library.
- EPUB bounded parser ignores CSS. Migration 0014 stores original bytes/hash/metadata,
  ordered chapter formatting/ruby/links/shared images + B/registration atomically.
  Loader verifies digests/counts/anchors/ownership without parsing. Every reader
  operation still verifies full source/assets; large-book cost is unmeasured.
- ReaderDocument carries keyed validated heading/emphasis/strong/HTTP(S) spans;
  ReaderPage clips to page-local UTF-8; generated JNI/Compose preserve exact text.
  Website tap areas open browser, retain vocabulary backgrounds; remaining token
  fragments keep full dictionary identity. Internal/other source links stay inert.
  Recreation/chapter navigation retain styles. No CSS/TOC; ruby/images not transported.
## Next tasks (ordered)
1. Reader image asset transport/rendering + cover cache/shared pinch-zoom/pan viewer.
2. Bundle Noto Serif JP; font/line settings and default-off source/analyzer furigana.
3. Add CI host Rust/Kotlin checks and unsigned Android builds (independent).
## Verification
- 412 data-free Rust tests, strict Clippy/Rust+Kotlin formatting/diff, real-Full
  Kotlin/JNI + Unicode/formatting exchanges and repeatable Kotlin generation pass.
- Both-ABI debug/test APKs + strict lint pass. Eleven focused API-36 x86_64 formatting/
  word-span/navigation tests pass across initial ten + final corrected single rerun.
  Test initially invoked Activity Back instead of sheet Close; fixed test only.
  Lint KTX URI helper fixed. Reports: .local/test-reports/reader-formatting/.
- Commands/failures/limits: DEVELOPMENT.md end. Gradle/ADB need approved outside-
  sandbox sockets. No open blocker. Large EPUB costs remain unmeasured. No dependency/
  migration/Q188 change. Physical arm64 remains user-deferred; other deferred checks
  are recorded in DEVELOPMENT.md. Emulator remains running.
