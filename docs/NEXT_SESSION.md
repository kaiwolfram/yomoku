# Resume here
Updated: 2026-10-27. Retained effective word/custom lifecycle done; live switch next.
## Working
- Pure Crux routes/Library/Settings; Render-only UI; FIFO SQLite/analysis; Library start.
- Chapter-local UTF-8 position/whole-book percentage/five presets persist; reflow
  preserves offsets. Scroll resets; 720dp column; controls below content; 48dp swipes.
  Previous only saves position; busy gates reject doubles; deliberate rereads count.
- Atomic SHA-256 imports store original B/unique registrations. Existing preserves
  title/status/progress without Full/rescan/writes. Summary returns Library; no auto-open.
- Loads are read-only; explicit finished-book open resets to page 1 atomically.
  Finish survives movement/time/reflow; deletion resets it, preserving history.
- App/FIFO/JNI retains ordered chapters and frozen original B/Missing identities;
  plain loads preserve them; Previous crosses chapters.
- Token reads atomically load original B/live statuses/metadata + clipped UTF-8
  ranges; completion owns token start. Deleted IDs absent. Whole-chapter validation.
  Full page/base/result identities gate callbacks; movement/reflow/refresh invalidate.
  Strict UTF-8→UTF-16 failure falls back to plain text. Loads wait for pending saves.
- Five manual statuses atomically update history; same-status no-op. Deleted entries
  never register. Reads/choices need explicit retry. Phone sheet/wide panel shows
  original/base, senses/POS/names/counts. Read-only opening; movement dismisses.
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
- dataSync lease tracks counts/name; encoding/timeout release it, timeout Cancels.
  OS limits apply; permission never blocks; notification opens Library.
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
- Full-window viewer: bounded 1–8x zoom/pan, retained selection; modal never completes.
- Verified Library covers: <=4 MiB/64-entry LRU; <=64 KiB decode.
- Noto Serif JP/credits; global font/spacing (16sp/150% default)/furigana persist (0015/16).
- Custom meanings/readings (0017): exact-surface CRUD core/FIFO/FFI/Android; frozen
  selections/built-in gates, literal drafts/Cancel/retries; no history/status writes.
- Furigana: persisted switch/retries, source-first/B fallback independent of status;
  half-size readings, strict UTF-16, full crossing readings/fitted width/headroom.
- Effective segmentation: B + A/B/C/manual; contiguous JP B matches, local/newer
  precedence/reversal; exact metadata provenance, unmatched unresolved.
- Pure effective furigana: source priority/crossings; unresolved manual has no B fallback.
- Effective APIs: eligibility/start-owned counts/live statuses; frozen word/custom
  provenance/base, unresolved surface-only; shared CRUD/built-in gates.
- Effective completion (0018): full key/occurrences/time frozen durably; page-only
  registration/New-only Known/history/time/position/finish atomic. Exact replay
  survives reversal/deletion/reopen; legacy calls reject its ID. Live reader uses B.
- Atomic override registration: winning parts only; retry no-op; reversal keeps history.
- Compact effective_session: one full key + view, fresh revisions, stale/foreign/
  duplicate guards, retry/Missing B; prototype Kotlin codecs, page/selection 60/68 B.
- Native effective_storage shares FIFO/dictionary; tokens/word/custom/completion.
  Retained compact word state: frozen keys, fresh attempts, retry/Cancel/stale guards.
## Next tasks (ordered)
1. Switch live core/FFI/Android completion/token/furigana/word/custom together using
   effective_storage + retained page/word requests + EFFECTIVE_READER_PAYLOAD.md.
2. Add typed reads/choices/save/reverse with rendered-key/intervening-edit retry guards.
3. Add Android A/B/C/manual + All/This occurrence; measure real-book/JNI/device cost.
Physical arm64 deferred; Nostr empty; Q188 unresolved before forwarding.
## Verification
- PASS: 552 Rust tests (17 ignored), strict Clippy/Rustfmt/Kotlin formatting/diff;
  15 Full + one runtime EACH owned/mapped; real-Full Kotlin/JNI/7 Unicode/100 handles.
- Codegen identical; APK/device/lint/release/arm64 not rerun. DEVELOPMENT.md end.
