# Resume here
Updated: 2026-10-27. Effective segmentation metadata verified; reader integration next.
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
- Keyed image placeholders/alt/metadata; no BLOBs in state. One-shot FIFO reads
  verify live ownership/media/SHA-256/32 MiB cap without dictionary/core writes.
- Serial bounds-first decode: <=32768-edge/100 MP source, <=2048-edge sample,
  16 MiB shared page pixels; repeats share bitmaps. Ratio frames <=8192px preserve
  U+FFFC positions/styles/word/link identity; unsupported images keep alt/Retry.
- Full-window viewer: bounded bitmaps, 1–8x fit zoom/clamped pan, accessible controls;
  recreation retains selection/zoom/focus; close/page resets. Modal gestures cannot
  complete pages; input resets idle.
- AndroidSVG 1.4 vectors/base64/nested SVG share viewer/covers; bounded/offline; v1 scope met.
- Library covers resolve live path/media/SHA-256 metadata without book BLOBs/Full.
  Media+digest LRU <=4 MiB/64 entries; misses verify bytes/digest and decode <=64 KiB.
- Noto Serif JP/credits; global font/spacing (16sp/150% default)/furigana persist (0015/16).
- Custom meanings/readings (0017): global exact-surface CRUD through core/FIFO/FFI/
  Android; original-token/built-in gates, literal fields, Cancel/exact retries.
  Selection/drafts reset on dismissal/recreation; history/status/position unchanged.
- Furigana: persisted switch/retries; keyed source-first/original-B fallback, status
  independent. Half-size readings above canonical Text; strict UTF-16, complete
  readings across page/layout splits, fitted base width and measured headroom.
- Effective segmentation: original B, A/B/C/manual partitions, contiguous JP B
  matches, local/newer-ID precedence/reversal; read-only, no reader/history changes.
- Effective metadata: exact B/A/C profile/span provenance; unmatched spans unresolved.
## Next tasks (ordered)
1. Integrate effective eligibility/token/word/registration and frozen future-completion
   identities; retain original snapshots/past history. Reader still uses B today.
2. Wire override reads/choices/save/reverse through core/FIFO/FFI, then Android
   A/B/C/manual controls with default All occurrences versus This occurrence.
Physical arm64 deferred; Nostr empty; Q188 unresolved before forwarding.
## Verification
- PASS: 488 Rust tests (17 ignored), Clippy/format/diff, host Kotlin/JNI and
  repeat codegen; 15 Full + one runtime pass EACH owned/mapped. DEVELOPMENT.md end.
- Initial runtime teardown assertion failed once; isolated/full reruns pass. Commands
  and details: DEVELOPMENT.md end. No dependency/schema/bridge/Android changes;
  APK/device/lint/release/process-death/physical arm64 not rerun. No open blocker.
