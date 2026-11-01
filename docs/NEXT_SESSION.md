# Resume here
Updated: 2026-11-01. First MVP excludes Nostr and Known forms statistics.
Commit guidance: explanatory bodies; merges include provenance. Keep verification
results in progress docs/reports, out of commit message bodies.

## User checkpoints
- Announce when no plan steps remain; ask about product ambiguities. Recommend
  physical testing when it provides more value than the next implementation step.
- User's phone trial was acceptable; detailed device/workflow validation remains
  unchecked. Recommend a schema-23 upgrade trial with existing Library/history
  before enabling visible live integration; rebuild optimized APK when requested.
- Feed is only `Coming Soon! 🌱`; Nostr/identity/Q188 are beyond the first MVP.
- Statistics retains cumulative occurrences and all five statuses including Known.
  Schema 0023 drops only unused vocabulary_dictionary_forms; all other history,
  vocabulary, token dictionary metadata and Anki journals are preserved.

## Current Anki dispatch ownership
- BeginAnkiLive requires current-owner frozen preparation, successful matching
  preflight and empty restored journals with no recovery read/commit/error.
  Preparation/begin/ownership handles still have no shell codec or visible action.
- Existing SQLite FIFO retains the private attempt after atomic begin COMMIT.
  Matching acknowledgement grants Ready. Public DispatchAnkiLive consumes its
  native move-only permit once into a typed effect's owning response closure.
  Dispatch alone cannot prepare/begin; restore never grants a permit.
- Exact note fields/surface/deck/model/full layout cross the typed protocol.
  Model creation also receives frozen surface/deck for fresh prerequisite reads.
  Provider adapters refresh host/spec/permission/deck/layout/exact duplicates;
  creation rechecks deck/duplicates immediately before its first mutation.
- Every accepted typed outcome becomes FinishAnkiLive and uses the existing FIFO.
  Invalid IDs/wrong-kind/empty duplicate results and unexpected adapter exceptions
  retain uncertainty. Positive no-submission clears only its committed obligation.
- CoreSession routes live effects through its separate Anki executor. Session-local
  worker deduplicates effect IDs, cancels queued work on close, resolves on main
  while open and drops late/in-flight responses. Started provider work may finish;
  durable pending journals survive lost responses and restore read-only.
- Native Abandon invalidates lost callbacks before explicit Retry restores both
  journals; Retry drops live/prepared/preflight owners and worker attempts even
  on failed reads. Clear cannot erase active/failed work or durable obligations.
- No schema/dependency/native-export ABI change or reachable user mutation flow.
  ANKIDROID_API.md pins 2.25.3 evidence; no real-provider compatibility claim.
- IMPORTANT continuation: native preparation/begin remain unavailable to the shell.
  Add authenticated typed actions and visible projection before claiming a full
  CoreSession Add workflow. Worker tests establish dispatch/close ownership;
  native lifecycle and JNI codecs are checked separately from a real provider.

## Next tasks (ordered)
1. Project recovery/error/finished state and explicit Retry; expose authenticated
   current-selection preparation/begin actions and coordinate Ready dispatch.
   Preserve frozen intent, native ownership and no mutation replay on restart.
2. Add configured-deck settings, permission flow and Add/existing-note actions.
   Validate end-to-end CoreSession lifecycle, then a disposable real AnkiDroid
   collection/browser and unchanged vocabulary/history before compatibility claims.
3. Encrypted-backup format/library review follows completed Anki integration.

## Verification / blockers
- PASS: 5 new native/9 Android cases; 972 workspace tests/17 ignored; strict Clippy,
  Rustfmt/Kotlin formatting, identical generation, full host JNI, both-ABI debug/
  test APKs, lintDebug and 74 API-36 x86_64 emulator tests (zero failures/skips).
  Commands/logs: DEVELOPMENT.md, Typed committed Anki invocation dispatch.
- Initial new Retry test assumed it could abandon an active invocation; corrected
  to the existing Abandon-before-Retry gate. No open blocker or remaining
  implementation/verification work for this dispatch slice.
- Real AnkiDroid collection/browser, optimized release and physical upgrade or
  performance trials were not run. Native action/visible UI integration is next.
- Physical schema-23 upgrade and broad device/performance matrix remain open.
  Older phone APK is source 0374365; no release requested.
