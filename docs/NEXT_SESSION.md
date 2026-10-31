# Resume here
Updated: 2026-10-31. First MVP excludes Nostr and Known forms statistics.
Commit guidance: explanatory bodies; merges include provenance. Keep verification
results in progress docs/reports, out of commit message bodies.

## User checkpoints
- Announce when no plan steps remain; ask about product ambiguities. Recommend
  physical testing when it provides more value than the next implementation step.
- User's phone trial was acceptable; detailed device/workflow validation remains
  unchecked. Recommend an upgrade trial with existing Library/history (schema 23)
  before further integration; rebuild optimized APK when requested.
- Feed is only `Coming Soon! 🌱`; Nostr/identity/Q188 are beyond the first MVP.
- Statistics retains cumulative occurrences and all five statuses including Known.
  Schema 0023 drops only unused vocabulary_dictionary_forms; all other history,
  vocabulary, token dictionary metadata and Anki journals are preserved.

## Current Anki slice
- Begin/RetryAnkiExportPreflight dispatch typed read-only Crux/FFI operations.
  Rust retains handle/selection/note fields; shell receives exact surface, deck
  and full frozen layout. Native preparation/clear/callbacks have no shell codec.
- Android AnkiPreflightReader uses complete deck/duplicate/model queries and full
  validation of every exact-name model. Any later failure discards all evidence.
  Core requires the frozen nonfiltered deck and selects Existing/Reuse/Create;
  incompatible/ambiguous models fail without overwrite or arbitrary selection.
- Preflight/recovery share CoreSession's separate single-thread executor, main
  response handling and close gate. No reader FIFO ownership or automatic reads.
  Late callbacks are dropped on close; native ownership rejects stale/foreign/
  duplicate callbacks. Retry recaptures; Clear drops only unsubmitted intent.
- Recheck both restored-empty journal gates at begin/acceptance. Results are read
  observations, never write tokens. Restart retains neither intent nor preflight.
- Public events precede skipped callbacks so Rust Serde round trips match Kotlin's
  compact indices. Durable recovery commits still run through the SQLite FIFO.

## Retained live-export prerequisites
- Native PrepareAnkiExport authenticates the current visible original/effective
  selection and freezes fields/surface/sentence/deck/full layout. Reader busy,
  checkpoint/completion/departure/mutation gates apply; journals must be restored
  empty without pending/failed commits. Clear drops only unsubmitted intent.
  Restart retains neither intent nor read handles; CheckAndAdd remains isolated.
- Both durable begin methods gate both journals and commit before one invocation;
  only newly committed preparation grants private live tokens. Recovery IDs never
  grant writes. Terminal journals survive deletion; no pruning or automatic retry.
- API evidence: ANKIDROID_API.md, pinned 2.25.3. Isolated mutating adapters remain
  unowned with no app call site/real-provider claim; no dependency/minimum adopted.

## Next tasks (ordered)
1. Own durable live begin/outcome commits after preflight and before one model/
   note invocation. Recheck both journals and fresh provider/deck/layout/duplicate
   observations; validate permission failures, death/stale/lost callbacks and no
   restart mutation replay. Native authenticated preparation remains unexposed.
2. Add recovery/error/explicit Retry projection, deck settings, Add to Anki and
   existing-note actions after gates; validate a disposable real collection/browser.
3. Encrypted-backup format/library review follows completed Anki integration.

## Verification / blockers
- PASS: 937 workspace cases/17 ignored; build/strict Clippy/Rustfmt/Kotlin format,
  full JNI/Unicode exchange, repeatable generation, both-ABI debug/test APKs/lint
  and 65 API-36 x86_64 emulator cases (zero failures/skips). Commands/logs:
  DEVELOPMENT.md, typed frozen-export Anki preflight dispatch.
- No blocker or remaining implementation/verification work for this slice.
  Durable live begin/outcome routing and the wider release plan remain open.
- Physical schema-23 upgrade/real AnkiDroid trials and broad performance/device
  matrix remain unchecked. Older phone APK is source 0374365; no release requested.
