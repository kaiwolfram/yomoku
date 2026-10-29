# Resume here
Updated: 2026-10-29. Android effective navigation staged; span/editor then activation.
- Single/bulk deletion: one transaction, history/vocabulary/rules retained; reimport fresh.
  Long press selects; Select All adds all matching titles, including offscreen rows.
- Library title-only search/Clear + Last read/Title/Import date; query/sort retained.
- Live bars use original B/live statuses; read-only FIFO; explicit Retry; batches defer scans.
  Effective Library API shares eligibility/resolution + transaction-local status lookups;
  Missing/zero differ. Staged app routes effective read on same FIFO/cache/Retry gates;
  drafts/deletion prompts retained. Stored text/rules, no source/assets/Full/writes.
- Chapter-local UTF-8 position/whole-book percentage/five presets persist; reflow
  preserves offsets. Scroll resets; 720dp column; controls below content; 48dp swipes.
  Previous only saves position; busy gates reject doubles; deliberate rereads count.
- Atomic SHA-256 imports store original B/unique registrations. Existing preserves
  title/status/progress without Full/rescan/writes. Summary returns Library; no auto-open.
- Loads are read-only; explicit finished-book open resets to page 1 atomically.
  Finish survives movement/time/reflow; deletion resets it, preserving history.
- Token reads atomically load original B/live statuses/metadata + clipped UTF-8
  ranges; completion owns token start. Deleted IDs absent. Whole-chapter validation.
  Strict UTF-8→UTF-16 failure falls back to plain text. Loads wait for pending saves.
- Five statuses atomically update history; same-status no-op/deleted entries absent.
  Read-only phone/wide details: original/base/senses/POS/names/counts; Retry; movement dismisses.
- Clock uses elapsedRealtime/15s checkpoints, restore-before-Resume, five-minute
  idle/background cap and movement epochs. Absolute revisions freeze retries,
  coalesce maxima and drain before movement/exit. CoreSession close retains callbacks
  until busy/BOTH pending/direct Cancel clear; retries teardown. Kill may lose pending.
- Completion freezes page/position/preset/B/time, drains final time then FIFO reserves
  ID + duration. One transaction writes occurrences, New-only Known/history/time/
  position/finish. Exact retries; AlreadyRecorded/zero silent, positive toast once.
- TXT/SRT preserve bytes/filename/encoding/B and canonical ruby/cues; first import wins.
- Mixed FileImports reads off main; encoding/Skip/errors survive recreation; no restart queue.
- Direct off-main cancelFileImport bypasses FIFO; COMMIT may wait. Silent full-file
  rollback; winning/prior commits stay. Android ignores late reads and awaits BOTH
  native return + outcome before clearing/draining. Dedicated cancellation executor.
- EPUB (0014) ignores CSS; atomic source/content/formatting/ruby/images/B/registration.
  Snapshot verifies source/assets without parsing; host costs measured, device pending.
- Keyed heading/emphasis/strong/HTTP(S) spans clip to page UTF-8; JNI preserves text.
- Image placeholders/alt/metadata, no BLOB state; FIFO validates owner/media/SHA/32 MiB.
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
- Compact effective_session: one shared full key + view, revisions/stale/retry/Missing B;
  Kotlin identity/selection 60/68 B; native protocol uses existing FIFO/dictionaries.
- Status/overrides: atomic key/surface/ID/rule-revision guards, five choices,
  stored A/B/C; atomic edits/register/reverse, stale/ABA/concurrent guards.
- Explicit A/C uses the FIFO/import analyzer cache, whole chapter/original profile;
  stored reuse skips Full. Rechecks key/revision at commit; metadata requires reload.
- ReaderBoundaryEditor: typed A/B/C/manual/All/This/reversal/Retry/Cancel/reload;
  literal scalar cuts + Rust whitespace; session/prompt-scoped saveable drafts. Isolated UI.
- Native Model::with_effective_reader(): LoadReaderTokens + compact effective_reader_tokens,
  effective furigana + CompleteEffectiveReaderPage; captured FIFO callbacks, both final-time
  slots before preparation, exact retry/finish/notice/departure. Production default stays B;
  legacy word/status/custom/completion events reject in opt-in model.
- Compact effective app word/status/custom: retained FIFO, literal Retry/Cancel, five statuses,
  repeats/deletion/built-in gates. Dismissed writes/errors retained; time drains before movement.
- Compact app overrides: FIFO/prompt writes, Retry/Cancel/reload, Library/time gates.
- Android compact word/status/custom + navigation wired with retained CoreSession UUID;
  Next/Finish/swipes use validated effective identity, Previous position only. Native
  mutation/time gates; current gesture revisions; final clock sample before completion.
## Next tasks (ordered)
1. Add original-B span selection and wire ReaderBoundaryEditor; activate core/FFI/
   Android together. Native Model::with_effective_reader stays staged until ready.
2. Investigate emulator long-page HWUI stall (old reader also fails); details below.
3. Measure JNI/device/diverse/image-heavy costs; host baselines in performance doc.
Physical arm64 deferred; Nostr empty; Q188 unresolved before forwarding.
## Verification / blockers
- PASS: 70 focused Rust, real-Full host JNI/codecs, Kotlin fmt, both-ABI APKs/lint.
- PASS: 36 distinct phone cases + 4 wide-dark shell cases; emulator restored/stopped.
- Long-page ReaderWords movement times out in Compose with old ReaderScreen too.
  ReaderNavigation controls then stalls; JDB main waits in HardwareRenderer draw.
  swangle repeats with Vulkan disabled; swiftshader crashes at boot. No product fix.
- Full device matrix incomplete; exact cases, commands/logs: DEVELOPMENT end.
- Prior full Rust/Clippy: 702 (17 ignored). Production B; no schema/dependency change.
