# Resume here
Updated: 2026-10-26. Ordered-chapter Rust reader repositories verified; app paste/TXT.
Read AGENTS.md, plan phases 2–4 and PRODUCT_SPEC.md sections 3–4.
## Working
- Pure Crux routes/Library/Settings; Render-only UI; FIFO SQLite/analysis. Starts Library.
- Exact chapter-local UTF-8 position/Library percentage + five presets persist;
  reflow preserves offset. Scroll resets on navigation/reopen; column max 720dp.
  Next/Previous/counter below content; net 48dp swipes need no bottom. Previous
  saves position only. Busy/pending gates reject duplicates; rereads count.
- Paste atomically deduplicates SHA-256, saves original B + unique Japanese
  registrations. Existing preserves title/status/progress without rescan/writes.
  Created/Existing show dismissible Library summary; never auto-open.
- Legacy load_reader_snapshot/load_paste are read-only; open_reader_snapshot alone
  resets finished books to page 1/preset/B atomically. Finish marker survives
  position/reflow/time; deletion resets it, preserving history. Missing never rescans.
- Token reads load original B/live statuses/metadata + clipped UTF-8 ranges atomically;
  completion uses token start. Deleted IDs absent. Whole-chapter validation per lookup.
  FIFO/JNI loads after busy/pending saves clear; page/base/result identities must match.
  Movement/reflow/refresh invalidate. Strict UTF-8→UTF-16 failures render plain text.
- Five manual status choices update status/history atomically; same-status no-op.
  Deleted IDs never register. Failed reads/choices require explicit retry.
  Phone sheet/wide panel shows readings/base, expanded restriction-aware English
  senses/compact POS, separate names/categories/lifetime counts. Opening read-only;
  navigation/recreation dismisses, errors survive timer saves, busy/pending gates apply.
- Clock uses elapsedRealtime/15s Handler checkpoints, restore-before-Resume,
  five-minute idle/pause/background cap, reflow deadline and movement epochs.
  Absolute revisions freeze retries, coalesce maxima and drain before movement/exit.
  CoreSession close retains callbacks until busy/BOTH pending flags/direct Cancel clear;
  retries after onCleared. Abrupt kill can lose pending.
- Completion freezes page/position/preset/AnalysisId/time; drains final time then FIFO
  reserves ID + whole-page duration. One transaction commits occurrences/New-only
  Known/history/time/position/finish. Missing blocks. Exact retries; AlreadyRecorded/
  zero silent. Positive toast once. New final Recorded returns Library + 1.4s confetti.
- NativeBridge consumes owned verified FD+1; Rust rehashes/maps/closes all paths.
  FIFO releases Full in background; keep live mapped inodes immutable. Pinned
  2026-10-25 JMdict English/JMnedict + notices/offline credits bundled. Lazy index
  until close/query failure; full token/selection identity gates dictionary attach.
- TXT strict UTF-8/Shift-JIS/Aozora atomic original-byte SHA-256, filename/encoding/
  ruby/Mode B/unique registration. Duplicate skips decode/Full. Typed FIFO/JNI gates saves.
- Android multiple-TXT retained helper reads one URI off main; sequences encoding/
  Skip/import. Filename/X-of-N + processed bar and summary survive recreation/background.
  No persisted queue; teardown drops unsubmitted work. Name precedes bytes.
- cancelFileImport() bypasses FIFO off main (COMMIT may wait); TXT alias retained.
  Fresh handle precedes submission; silent full-file rollback; winning/prior commits stay.
  Android clears inputs/remaining, invalidates late reads, retires executors and waits
  BOTH native return + current outcome, including draining close.
- dataSync lease tracks counts/name only; OS limits apply. Encoding releases it;
  notification permission never blocks. Timeout stops lease before Cancel. Notices
  count processed/total; taps no reader. HOME/SIGKILL retain only commits, no resume.
- SRT repository/FIFO/JNI uses strict shared decoder (no Aozora), original hash/filename/
  LF/cues/B/unique registration; TXT/SRT retains first interpretation. Picker pending.
- EPUB bounded parser ignores CSS; migration 0014 atomically stores source bytes/hash/
  title/filename, ordered chapters/formatting/ruby/links, shared cover/images and per-
  chapter original B/unique registrations. Cancel/later failure rolls back all rows/IDs.
  Read-only loader validates digests/counts/indexes/anchors/ownership without parsing.
- NEW ReaderDocument + DocumentReaderSnapshot retain ordered chapters and keyed B IDs.
  Generic load/open/position repositories support all formats. Existing completion/time/
  token/selected-word repositories now use complete documents; next chapter starts at 0,
  only whole-book final page finishes. Time stays chapter-local; global progress/counters.
  New reader loads currently verify all EPUB source/assets per operation (unmeasured).
  Legacy app/worker/import-reference wire shapes still single-text and reject EPUB.
## Next tasks (ordered)
1. Migrate app/worker/typed JNI reader + import-reference payloads to ordered documents;
   wire EPUB FIFO/JNI. Use generic file-ID preflight, never truncate duplicate chapters.
2. Mixed EPUB/TXT/SRT picker/summary/Cancel, reader images/ruby; CI independent.
3. User-run physical arm64 validation once usable.
## Verification
- PASS: 393 data-free Rust tests, Clippy/fmt/diff, both Android storage targets;
  fourteen real-Full analysis + one worker fixture per owned/mapped loader;
  real-Full Kotlin/JNI + seven Unicode exchanges. Commands: DEVELOPMENT.md end.
- No blocker. APK/Android lint/device/codegen-repeat/index/Python/release/reboot/
  process-death/tablet/dark not rerun; physical arm64 unverified. Nostr empty/Q188.
