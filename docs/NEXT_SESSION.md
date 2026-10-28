# Resume here
Updated: 2026-10-28. EPUB/Library host scaling verified; live switch next.
## Working
- Rename: retained literal draft/Cancel/Retry; title-only atomic save, no-op retries.
- Single/bulk deletion: one transaction, history/vocabulary/rules retained; reimport fresh.
  Long press selects; Select All adds all matching titles, including offscreen rows.
  Filter/sort retain selection; confirmation freezes titles/IDs; Cancel/Back and Retry.
- Library title-only search/Clear + Last read/Title/Import date; query/sort retained.
  Read order (0019) on explicit open only; newest unknown/unopened afterward.
  Duplicates preserve recency; legacy history unknown.
- Live bars use original B/live statuses; read-only FIFO; explicit Retry; batches defer scans.
  Effective Library API shares eligibility/resolution + transaction-local status lookups;
  Missing/zero differ. Stored text/rules, no source/assets/Full/writes. Switch with reader.
- Chapter-local UTF-8 position/whole-book percentage/five presets persist; reflow
  preserves offsets. Scroll resets; 720dp column; controls below content; 48dp swipes.
  Previous only saves position; busy gates reject doubles; deliberate rereads count.
- Atomic SHA-256 imports store original B/unique registrations. Existing preserves
  title/status/progress without Full/rescan/writes. Summary returns Library; no auto-open.
- Loads are read-only; explicit finished-book open resets to page 1 atomically.
  Finish survives movement/time/reflow; deletion resets it, preserving history.
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
- Bounded EPUB parser ignores CSS; migration 0014 atomically saves original source,
  ordered content/formatting/ruby/images/B/registration. Snapshot loads verify all
  source/assets without parsing; limited host EPUB costs measured, device pending.
- ReaderDocument carries keyed validated heading/emphasis/strong/HTTP(S) spans;
  ReaderPage clips to page-local UTF-8; generated JNI/Compose preserve exact text.
  Website taps open browser with token identity; other links inert. No CSS/TOC.
- Keyed image placeholders/alt/metadata; no BLOBs in state. One-shot FIFO reads
  verify live ownership/media/SHA-256/32 MiB cap without dictionary/core writes.
- Serial bounds-first decode: <=32768-edge/100 MP source, <=2048-edge sample,
  16 MiB shared page pixels; repeats share bitmaps. Ratio frames <=8192px preserve
  U+FFFC positions/styles/word/link identity; unsupported images keep alt/Retry.
- Custom entries (0017): literal CRUD/Cancel/retries, built-in gates; no history/status writes.
- Furigana: persisted switch/retries, source-first/B fallback; strict UTF-16/full crossings.
- Effective segmentation: B + A/B/C/manual; contiguous JP B matches, local/newer
  precedence/reversal; exact metadata provenance, unmatched unresolved.
- Effective completion (0018): full key/occurrences/time frozen durably; page-only
  registration/New-only Known/history/time/position/finish atomic. Exact replay
  survives reversal/deletion/reopen; legacy calls reject its ID. Live reader uses B.
- Compact effective_session: one full key + view, revisions/stale/retry/Missing B;
  Kotlin page/selection 60/68 B; native effective_storage shares FIFO/dictionary/word state.
- Status/overrides: atomic key/surface/ID/rule-revision guards, five choices,
  stored A/B/C; atomic edits/register/reverse, stale/ABA/concurrent guards.
  Fresh attempts/exact retries/shared gates; stale/lost success requires reload.
- Explicit A/C uses the FIFO/import analyzer cache, whole chapter/original profile;
  stored reuse skips Full. Rechecks key/revision at commit; metadata requires reload.
- Compact override prompt/actions: fresh same-page identities; default All, literal
  choices/reverse/retry/Cancel; generated codecs. Retained furigana + strict Android
  effectiveDisplayTokens adapters ready (crossings/unresolved/source preference). Not live.
## Next tasks (ordered)
1. Switch live core/FFI/Android completion/token/status/furigana/word/custom + Library membership
   (effective_library_word_distributions) via retained requests/command/finish_request + render
   adapters; see EFFECTIVE_READER_PAYLOAD.md. No live adoption yet.
2. Wire compact override actions/state to Android A/B/C/manual + All/This/reversal.
3. Measure JNI/device/diverse/image-heavy costs; TXT/EPUB/100-copy host results in performance doc.
Physical arm64 deferred; Nostr empty; Q188 unresolved before forwarding.
## Verification
- PASS: 678 Rust (17 ignored) + benchmark test, Full owned/mapped, Clippy/fmt, 10 CLI rejects.
- EPUB/100-copy reads preserve DB bytes; no JNI/APKs/lint/device rerun. No blocker. DEVELOPMENT end.
