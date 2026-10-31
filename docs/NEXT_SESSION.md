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
- Explicit BeginAnkiRecoveryRead(Note|Model) now emits a typed read-only Crux
  operation through EffectRouter/FFI. Only frozen surface or full layout/optional
  known model ID crosses; both journals and shared-allocation read handle remain
  in the native response closure. Begin and explicit Retry have shell codecs;
  callbacks/commits/journals still do not. No automatic read or UI action exists.
- CoreSessionOwner supplies AnkiRecoveryReader/AnkiProviderReader. CoreSession
  dispatches on a separate single-thread worker, resolves on main and drops late
  callbacks during/after close. Shutdown does not wait for blocked reads. Restarts
  restore journals without replay. Low-level sessions default to unavailable.
- Unknown models require one complete layout match; any later read failure drops
  earlier matches. Known model ID checks cannot substitute another model. Empty,
  failed, partial or ambiguous reads retain obligations. Notes prove existence,
  never deck placement. No model repair/rename/delete or retry mutation occurs.
- Model transport uses AnkiReadModelLayout with ordered list fields because the
  pinned generator mishandles fixed string arrays. Native stored four-field arrays
  and JSON stay unchanged. No schema/dependency/native ABI change.
- Matching responses validate kind/positive IDs and unchanged captured pair,
  then enter FIFO CommitAnkiRecovery. Gate stays closed until commit. Foreign,
  stale and duplicate callbacks cannot consume newer reads. Provider/read errors
  remain separate from reader and journal-persistence errors.
- Explicit Retry invalidates lost reads/read errors and restores BOTH journals.
  Pending commits cannot be abandoned. Failed/lost commit responses need restore
  plus fresh observations; never replay evidence or provider mutations.

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
1. App-owned prepared-export preflight with typed provider observations, then
   durable begin/outcome commits before one live model/note invocation. Validate
   provider/permission/deck/model failures and death/stale/lost callbacks.
2. Add recovery/error/explicit Retry projection, deck settings, Add to Anki and
   existing-note actions after gates; validate a disposable real collection/browser.
3. Encrypted-backup format/library review follows completed Anki integration.

## Verification / blockers
- PASS: 920 workspace cases/17 ignored; strict Clippy/Rustfmt/Kotlin formatting;
  full host JNI/Unicode exchange; identical host/Android generation; both-ABI
  debug/test APKs, strict lint and all 60 selected API-36 x86_64 emulator cases.
  Commands, resolved initial failures and logs: DEVELOPMENT.md, typed dispatch.
- No open blocker or remaining work for this slice. Physical upgrade/real AnkiDroid
  trials and broad performance/accessibility/lifecycle matrix remain unchecked.
- Older phone APK remains source 0374365; no new optimized release requested.
