# Resume here
Updated: 2026-10-27. Furigana switch/layout verified and complete.
## Working
- Pure Crux routes/Library/Settings; Render-only UI; FIFO SQLite/analysis; Library start.
- Chapter-local UTF-8 position/whole-book percentage/five presets persist; reflow
  preserves offsets. Scroll resets; 720dp column; controls below content; 48dp swipes.
  Previous only saves position; busy gates reject doubles; deliberate rereads count.
- Atomic SHA-256 imports store original B/unique registrations. Existing preserves
  title/status/progress without Full/rescan/writes. Summary returns Library; no auto-open.
- Generic loads are read-only; explicit finished-book open alone resets first
  chapter/page/preset/B atomically. Finish survives movement/time/reflow; deletion
  resets it while preserving history. Missing analysis never rescans.
- App/FIFO/JNI retains ordered ReaderDocument snapshots and frozen keyed original
  B/Missing per chapter. Plain loads preserve it; Previous crosses chapters.
- Token reads atomically load original B/live statuses/metadata + clipped UTF-8
  ranges; completion owns token start. Deleted IDs absent. Whole-chapter validation.
  Full page/base/result identities gate callbacks; movement/reflow/refresh invalidate.
  Strict UTF-8→UTF-16 failure falls back to plain text. Loads wait for pending saves.
- Five manual statuses atomically update history; same-status no-op. Deleted entries
  never register. Reads/choices need explicit retry. Phone sheet/wide panel shows
  original/base, expanded senses/POS, names/categories/counts. Opening read-only;
  navigation dismisses.
- Clock uses elapsedRealtime/15s checkpoints, restore-before-Resume, five-minute
  idle/background cap and movement epochs. Absolute revisions freeze retries,
  coalesce maxima and drain before movement/exit. CoreSession close retains callbacks
  until busy/BOTH pending/direct Cancel clear; retries teardown. Kill may lose pending.
- Completion freezes page/position/preset/B/time, drains final time then FIFO reserves
  ID + duration. One transaction writes occurrences, New-only Known/history/time/
  position/finish. Exact retries; AlreadyRecorded/zero silent, positive toast once.
  Only new final-book completion returns Library + 1.4s confetti. Images count no words.
- TXT/SRT strict UTF-8/Shift-JIS preserve original bytes/filename/encoding/B and
  canonical Aozora ruby or SRT cues; exact duplicate's first interpretation wins.
- Retained FileImports reads one mixed EPUB/TXT/SRT URI off main, routes literal
  case-insensitive extensions; wildcard picker accepts generic MIME. TXT/SRT have
  separate encoding/Skip/errors; EPUB none. Filename/X-of-N/progress/summary survive
  recreation/background. No persisted queue/process restart.
- Direct off-main cancelFileImport bypasses FIFO; COMMIT may wait. Silent full-file
  rollback; winning/prior commits stay. Android ignores late reads and awaits BOTH
  native return + outcome before clearing/draining. Dedicated cancellation executor.
- dataSync lease tracks counts/name only; OS limits apply. Encoding releases it;
  permission never blocks. Timeout stops lease then Cancel. Notification tap opens Library.
- Bounded EPUB parser ignores CSS; migration 0014 atomically saves original source,
  ordered content/formatting/ruby/images/B/registration. Snapshot loads verify all
  source/assets without parsing; large-book costs remain unmeasured.
- ReaderDocument carries keyed validated heading/emphasis/strong/HTTP(S) spans;
  ReaderPage clips to page-local UTF-8; generated JNI/Compose preserve exact text.
  Website taps open browser with token identity; other links inert. No CSS/TOC.
- ReaderDocument carries validated page-local image placeholders/alt/asset metadata;
  no image BLOBs in state. One-shot FIFO asset reads verify live document ownership,
  expected media/SHA-256 and 32 MiB cap; no dictionaries/core/state writes.
- CoreSession-owned decoder serializes asset read/decode across pages. Bounds-first
  BitmapFactory rejects >32768-edge/100 MP sources, samples to <=2048-edge and each
  distinct asset's share of 16 MiB page pixels. Repeated assets share one bitmap.
- Compose replaces exact U+FFFC scalars with source-position/ratio images;
  tall frames cap at 8192px; styles/word/link identities stay intact. Local zero text
  targets stop adjacent-link capture. Alt/loading/failure + Retry for unsupported data.
- Full-window image viewer reuses bounded bitmaps, 1–8x fit zoom/clamped pan.
  Selection/zoom/focus survive recreation; close/page change resets. Modal gestures
  cannot complete pages; input resets idle. Buttons/Back provide accessible controls.
- AndroidSVG 1.4 vectors/base64/nested SVG share viewer/covers; bounded/offline; v1 scope met.
- Library covers resolve live path/media/SHA-256 metadata without book BLOBs/Full.
  Media+digest LRU <=4 MiB/64 entries; misses verify bytes/digest and decode <=64 KiB.
- Noto Serif JP/credits; global font/spacing (16sp/150% default)/furigana persist (0015/16).
- Source ruby clips to page-local UTF-8 with complete crossing readings. Enabled
  reader_furigana prefers source over original B; keyed/validated, status independent.
- Settings furigana switch commits/retries using existing typography gates.
  Compose paints half-size readings above unchanged canonical Text/tap identities.
  Strict UTF-16; complete readings repeat across splits and fit base width; measured headroom.
## Next tasks (ordered)
1. Custom English meaning/reading for missing dictionary matches; immutable built-ins.
2. A/B/C/manual split/merge overrides; start with testable persisted anchors.
Physical arm64 deferred; Nostr empty; Q188 unresolved before forwarding.
## Verification
- PASS: both-ABI debug/test APKs, strict lint, Kotlin formatting/Rustfmt/diff checks;
  16 distinct phone cases + six tablet/dark cases. Commands/failures: DEVELOPMENT.md end.
  Reports: .local/test-reports/reader-furigana-layout/. No open blocker.
- Rust/core/FFI unchanged; prior 453-test/Clippy/host-JNI/codegen baseline not rerun.
  No release/physical-arm64/process-death rerun. Restored phone/light AVD remains running.
