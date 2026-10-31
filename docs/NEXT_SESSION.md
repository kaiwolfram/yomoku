# Resume here
Updated: 2026-10-31. First MVP excludes Nostr and Known forms statistics.
Commit guidance: explanatory bodies; merges include provenance. Keep verification
results in progress docs/reports, out of commit message bodies.

## User checkpoints
- Announce when no plan steps remain; ask about product ambiguities. Recommend
  physical testing when it provides more value than the next implementation step.
- User's phone trial was acceptable; detailed device/workflow validation remains
  unchecked. Recommend a schema-23 upgrade trial with existing Library/history
  before enabling live integration; rebuild optimized APK when requested.
- Feed is only `Coming Soon! 🌱`; Nostr/identity/Q188 are beyond the first MVP.
- Statistics retains cumulative occurrences and all five statuses including Known.
  Schema 0023 drops only unused vocabulary_dictionary_forms; all other history,
  vocabulary, token dictionary metadata and Anki journals are preserved.

## Current Anki persistence slice
- Storage::begin_anki_live checks both gates, inserts the frozen note/model and
  validates its complete pair under one immediate transaction. Only successful
  COMMIT returns private nonserializable/noncloneable AnkiLiveAttempt.
- finish_anki_live compares the captured pair/payloads/known IDs, requires matching
  journal kind and pending state, validates the result and returns after COMMIT.
  Uncertain outcomes retain payload/known ID; terminal records remain durable.
- Existing journal APIs share internal begin/finish SQL. Statement/readback/commit
  failures roll back; lost begin/outcome responses require read-only restoration
  and reconciliation. Snapshots/IDs cannot recreate live tokens. No replay queue.
- These are storage-only APIs, not app-owned authorization. Runtime must retain
  the live attempt and authenticate callback/session ownership for one invocation.
  No core/Event/FFI/Kotlin/schema/dependency or enabled provider mutation change.

## Retained live-export prerequisites
- Native PrepareAnkiExport authenticates the current visible original/effective
  selection and freezes fields/surface/sentence/deck/full layout. Reader busy,
  checkpoint/completion/departure/mutation gates apply. Preparation stays unexposed.
- Begin/RetryAnkiExportPreflight dispatch typed read-only Crux/FFI operations;
  Rust retains handles/fields, Android reads complete deck/duplicate/model evidence.
  Later failure discards all evidence; exact-name ambiguity/conflicts fail safely.
- Recovery/preflight share CoreSession's separate worker/main/close gates. They
  do not hold reader FIFO ownership or run automatically. Retry captures fresh
  reads; late/stale/foreign/duplicate callbacks cannot cross owners or restart.
- Recheck both restored-empty journals at begin/acceptance, including pending or
  failed recovery commits. Clear drops only unsubmitted intent. Restart retains
  neither intent nor preflight. Recovery IDs never grant writes.
- Public events precede skipped callbacks to keep Rust/Kotlin compact indices
  aligned. Durable recovery evidence commits already use the SQLite FIFO.
- API evidence: ANKIDROID_API.md, pinned 2.25.3. Isolated mutating adapters remain
  unowned with no app call site/real-provider claim; no dependency/minimum adopted.

## Next tasks (ordered)
1. Own native live begin/outcome lifecycle through app and FIFO using the new
   storage APIs. Retain AnkiLiveAttempt in the worker; only a matching successful
   begin response may lead to one invocation. Gate stale/foreign/lost callbacks,
   errors and close/restart without restoring mutation authority.
2. Wire typed provider invocation with fresh provider/deck/layout/duplicate checks,
   then recovery/error/explicit Retry projection, deck settings and Add/existing
   note actions. Validate a disposable real AnkiDroid collection/browser.
3. Encrypted-backup format/library review follows completed Anki integration.

## Verification / blockers
- PASS this slice: 10 new storage cases; 46 focused Anki cases; 947 workspace
  cases/17 ignored; build, strict Clippy, Rustfmt and diff. Exact commands/logs:
  DEVELOPMENT.md, Atomic live Anki journal transactions.
- No blocker or remaining implementation/verification work for this storage slice.
  App/worker ownership, provider dispatch and the wider release plan remain open.
- Kotlin/JNI generation, Android APK/lint/emulator, real-provider and physical
  trials were not rerun for this storage-only slice. Previous typed preflight
  checks: full JNI/generation, both-ABI debug/test APKs/lint, 65 emulator cases.
- Physical schema-23 upgrade and broad device/performance matrix remain unchecked.
  Older phone APK is source 0374365; no release requested.
