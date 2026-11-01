# Resume here
Updated: 2026-11-01. First MVP excludes Nostr and Known forms statistics.
Commit guidance: explanatory bodies; merges include provenance. Keep verification
results in progress docs/reports, out of commit message bodies.

## User checkpoints
- Announce when no plan steps remain; ask about product ambiguities. Recommend
  physical testing when it provides more value than the next implementation step.
- User's phone trial was acceptable; detailed device/workflow validation remains
  unchecked. Recommend a schema-23 upgrade trial with existing Library/history
  before visible live integration; rebuild optimized APK only when requested.
- Feed is only `Coming Soon! 🌱`; Nostr/identity/Q188 are beyond the first MVP.
- Statistics retains cumulative occurrences and all five statuses including Known.
  Schema 0023 drops only unused vocabulary_dictionary_forms; other history,
  vocabulary, token dictionary metadata and Anki journals are preserved.

## Current Anki boundary
- Settings now exposes compact recovery phases/pending-kind flags/errors and
  native-gated Check note/Check model/Retry. Clean startup hides the panel; after
  attention is needed, completion stays visible only within that native session.
- Opening Settings never queries the provider. Retry reloads both SQLite journals
  and invalidates eligible lost reads; a subsequent provider check is explicit.
  Empty results remain unresolved; only durable acknowledgement clears recovery.
  Async visible changes render without RefreshLibrary; reader busy/errors stay
  independent. Read-only observers now wait for that acknowledgement before SQL.
- BeginAnkiLive still requires frozen current-owner preparation, matching preflight
  and both empty restored journals, with no recovery read/commit/error.
- The FIFO retains the private committed attempt. Matching begin grants Ready;
  public DispatchAnkiLive consumes its move-only native permit once into the
  typed effect response closure. Standalone dispatch/restore cannot mint authority.
- Exact fields/surface/deck/model/full layout cross the typed protocol. Provider
  adapters refresh host/spec/permission/deck/layout/duplicates before mutations.
  Invalid/wrong-kind results and unexpected exceptions retain uncertainty.
- CoreSession's separate Anki worker deduplicates live effect IDs, cancels queued
  work on close and drops late/in-flight results. Durable journals restore read-only.
  Abandon-before-Retry handles lost live callbacks; Clear cannot erase obligations.
- IMPORTANT: preparation/begin still have no shell codec or visible Add workflow.
  Authenticated actions, Ready orchestration and live outcome presentation remain.
  No schema/dependency/native-export ABI change or real-provider compatibility claim.
  Details: ARCHITECTURE.md, DATA_MODEL.md and ANKIDROID_API.md (2.25.3 evidence).

## Next tasks (ordered)
1. Expose authenticated current-selection preparation/begin actions, coordinate
   Ready dispatch and project preflight/model-conflict/live finished/error outcomes.
   Preserve frozen intent, native ownership and no mutation replay on restart.
2. Add configured-deck settings, permission flow and Add/existing-note actions.
   Validate complete CoreSession lifecycle, then a disposable real AnkiDroid
   collection/browser and unchanged vocabulary/history before compatibility claims.
3. Encrypted-backup format/library review follows completed Anki integration.

## Verification / blockers
- PASS: 7 new core/2 new Compose cases; 979 workspace tests/17 ignored; strict
  Clippy, Rustfmt/Kotlin formatting, workspace build, identical generation, full
  host JNI plus final focused recheck, both-ABI debug/test APKs, lintDebug and all
  13 selected API-36 x86_64 emulator cases (zero failures/errors/skips on final run).
  Commands/logs: DEVELOPMENT.md, Settings Anki recovery presentation.
- Initial compilation/legacy no-render assertions were corrected. Initial Android
  read-only SQL polling raced the commit; fixed by awaiting projected durable state.
  No open blocker or implementation/verification work remains for this slice.
- Real collection/browser, optimized release and physical upgrade/performance
  trials were not run. Broad device/schema-23 upgrade validation remains open.
  Older phone APK is source 0374365; no release requested.
