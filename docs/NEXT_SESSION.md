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
- Shell `PrepareReaderAnkiExport` carries typed original/effective selection and
  signed-64 deck ID; native validation freezes only current loaded dictionary
  data. No shell-provided note fields/layout/handles. Invalid/stale requests do
  nothing. Each accepted preparation gets a checked session-local u64 generation;
  Clear/Retry never reuse it, exhaustion rejects and restart restores no intent.
- `AnkiExportView::preparation_id` correlates `BeginPreparedAnkiExport` and
  `ClearAnkiExport`. Begin also matches the exact projected phase before native
  preflight/both-empty-journal gates. Clear preserves active/failed work and all
  durable obligations. Native preparation/begin/storage/callback codecs stay absent.
- Export view freezes surface/deck and projects preflight/errors/live phases plus
  last durably acknowledged outcomes. IDs stay hidden in Saving until matching
  outcome acknowledgement. Failures show storage error; Clear or recovery Retry
  removes presentation. Async changes render independently of reader busy/errors.
- Settings exposes compact recovery phases/flags/errors and native-gated Check
  note/Check model/Retry. Clean startup hides it; completion persists in-session.
  Opening Settings never queries provider. Retry restores both journals read-only,
  invalidates lost reads and requires an explicit subsequent provider check.
  Empty observations remain unresolved; only durable acknowledgement clears them.
- FIFO privately owns the committed attempt; matching begin grants one Ready
  permit. DispatchAnkiLive consumes it into the typed response closure once.
  Provider refreshes host/spec/permission/deck/full layout/exact duplicates;
  invalid results/exceptions retain uncertainty. CoreSession's separate worker
  deduplicates effect IDs, cancels queued work and drops late/in-flight callbacks
  on close. Restart never recreates mutation authority or replays writes.
- IMPORTANT: no visible Add workflow yet. CoreSession Ready orchestration,
  Compose live UI, configured deck/permission/Add/browser flows remain.
  Schema/dependencies/native-export ABI unchanged; no real-provider compatibility
  claim. Details: ARCHITECTURE.md, DATA_MODEL.md, ANKIDROID_API.md (2.25.3 evidence).

## Next tasks (ordered)
1. Coordinate Ready dispatch in CoreSession and consume typed export status in
   Compose live outcome UI; preserve frozen intent, native ownership and no replay.
2. Add configured-deck settings, permission flow and Add/existing-note actions.
   Validate complete CoreSession lifecycle, then a disposable real AnkiDroid
   collection/browser and unchanged vocabulary/history before compatibility claims.
3. Encrypted-backup format/library review follows completed Anki integration.

## Verification / blockers
- PASS: four new core cases; 989 workspace tests/17 ignored, eight real FIFO
  cases, strict Clippy/formatting, full host JNI/build, identical generation,
  both-ABI debug/test APKs, lint and all 11 selected API-36 emulator cases
  (zero failures/errors/skips). Commands/logs: DEVELOPMENT.md, Correlated Anki
  shell actions. Initial JNI fixture missed the startup Library restore trigger;
  corrected it. No blocker or implementation/verification work remains for this
  shell-action slice; the complete visible Add workflow remains unfinished.
- Real collection/browser, optimized release and physical upgrade/performance
  trials remain unrun. Broad device/schema-23 upgrade validation remains open.
  Older phone APK is source 0374365; no release requested.
