# Resume here
Updated: 2026-10-28. Compact override contracts done; live switch next.
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
- Retained mixed FileImports reads off main by extension, accepts generic MIME;
  TXT/SRT encoding/Skip/errors and progress survive recreation. No restart queue.
- Direct off-main cancelFileImport bypasses FIFO; COMMIT may wait. Silent full-file
  rollback; winning/prior commits stay. Android ignores late reads and awaits BOTH
  native return + outcome before clearing/draining. Dedicated cancellation executor.
- dataSync lease/timeout Cancel; OS limits apply; permission never blocks imports.
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
- Viewer: 1–8x zoom/pan; no completion. Covers: 4 MiB/64-entry LRU.
- Noto Serif JP/credits; typography/furigana persist (0015/16).
- Custom entries (0017): exact-surface CRUD core/FIFO/FFI/Android; frozen selections,
  built-in gates, literal drafts/Cancel/retries; no history/status writes.
- Furigana: persisted switch/retries, source-first/B fallback independent of status;
  half-size readings, strict UTF-16, full crossing readings/fitted width/headroom.
- Effective segmentation: B + A/B/C/manual; contiguous JP B matches, local/newer
  precedence/reversal; exact metadata provenance, unmatched unresolved.
- Effective completion (0018): full key/occurrences/time frozen durably; page-only
  registration/New-only Known/history/time/position/finish atomic. Exact replay
  survives reversal/deletion/reopen; legacy calls reject its ID. Live reader uses B.
- Compact effective_session: one full key + view, fresh revisions, stale/foreign/
  duplicate guards, retry/Missing B; prototype Kotlin codecs, page/selection 60/68 B.
- Native effective_storage shares FIFO/dictionary; compact word state/retry/stale guards.
- Retained completion: freeze before time drain; exact retries/reopen/reversal count once.
- Effective status: atomic key/surface/ID guards; five choices/no-op retries/gates.
- Guarded overrides: stored A/B/C/shared key/rule revision; atomic edits/register/
  reverse, stale/ABA/concurrent guards. Retained fresh attempts/exact retries/shared
  gates; stale/lost success requires explicit reload. Rollbacks retry.
- Explicit A/C uses the FIFO/import analyzer cache, whole chapter/original profile;
  stored reuse skips Full. Rechecks key/revision at commit; metadata requires reload.
- Compact override prompt/actions: fresh same-page prompt identities; default All,
  literal choices/reverse/retry/Cancel guards; generated Kotlin codecs. Not live.
## Next tasks (ordered)
1. Switch live core/FFI/Android completion/token/status/furigana/word/custom together
   via effective_storage + retained requests; see EFFECTIVE_READER_PAYLOAD.md.
2. Wire compact override actions/state to Android A/B/C/manual + All/This/reversal.
3. Measure real-book/JNI/device cost, including whole-chapter on-demand generation.
Physical arm64 deferred; Nostr empty; Q188 unresolved before forwarding.
## Verification
- PASS: 611 Rust (17 ignored); Clippy/fmt/diff; Kotlin/JNI + repeat codegen.
- Commands/logs: DEVELOPMENT.md end. Standalone prompt schemas; live reader uses B.
- Full matrix/APK/device/lint/release/arm64 not rerun; no blocker.
