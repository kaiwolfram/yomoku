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
- Native BeginAnkiExportPreflight captures the frozen export in a distinct shared
  handle. Complete typed deck/model/duplicate observations yield Existing,
  ReuseModel or CreateModel; no token, effect or provider dispatch is emitted.
- Require the frozen nonfiltered deck, valid positive IDs and complete reads.
  One exact-name full-layout match permits reuse; missing name permits creation;
  incompatible/ambiguous names fail without overwrite/rename. Exact duplicates
  prevent insertion. Failed/partial reads must fail the whole observation.
- Recheck both empty restored journal gates at begin/acceptance. Foreign/stale/
  duplicate callbacks cannot consume a current read. Explicit Retry invalidates
  lost/failed/completed reads; Clear invalidates unsubmitted intent/observations
  without touching journal obligations. Restart retains neither intent nor read.
- Native result accessors are coordinator prerequisites, not a provider lease or
  write authorization. Future durable begin must recheck gates; mutating adapters
  must repeat provider/deck/layout/duplicate validation immediately before writes.
- Existing typed recovery reads still dispatch through CoreSession's separate
  worker, resolve on main and drop late callbacks on close. Recovery evidence
  commits through the FIFO before releasing obligations. No mutation replay.
- Preflight Begin/Retry/callbacks have no shell codec or visible action. Recovery
  Begin/Retry have codecs; native recovery callbacks/commits/journals do not.

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
1. Dispatch the native prepared-export preflight through typed Crux/FFI and the
   Android worker, failing whole reads on any partial provider result. Then own
   durable begin/outcome commits before one live model/note invocation; validate
   permission/deck/model failures and death/stale/lost callbacks.
2. Add recovery/error/explicit Retry projection, deck settings, Add to Anki and
   existing-note actions after gates; validate a disposable real collection/browser.
3. Encrypted-backup format/library review follows completed Anki integration.

## Verification / blockers
- PASS: 11 new app cases; 54 focused core/FFI cases; 931 workspace cases/17
  ignored; workspace build, strict Clippy, Rustfmt and unchanged Kotlin generation.
  Commands/logs: DEVELOPMENT.md, native Anki export preflight ownership.
- Android/JNI/emulator checks were not rerun for this native-only slice. Previous
  typed recovery slice passed both-ABI APKs/lint, JNI and 60 emulator cases.
- No blocker or remaining implementation/verification work for this slice.
  Provider dispatch/live Anki integration and the wider release plan remain open.
- Physical schema-23 upgrade/real AnkiDroid trials and broad performance/device
  matrix remain unchecked. Older phone APK is source 0374365; no release requested.
