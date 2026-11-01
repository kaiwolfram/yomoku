# Resume here
Updated: 2026-11-01. First MVP excludes Nostr and Known forms statistics.
Commit guidance: explanatory bodies; merges include provenance. Keep verification
results in progress docs/reports, out of commit message bodies.

## User checkpoints
- Announce when no plan steps remain; ask about product ambiguities. Recommend
  physical testing when it provides more value than the next implementation step.
- User's phone trial was acceptable; detailed device/workflow validation remains
  unchecked. Recommend a schema-23 upgrade trial with existing Library/history
  before the full visible Add flow; rebuild optimized APK only when requested.
- Feed is only `Coming Soon! 🌱`; Nostr/identity/Q188 are beyond the first MVP.
- Statistics retains cumulative occurrences and all five statuses including Known.
  Schema 0023 drops only unused vocabulary_dictionary_forms; other history,
  vocabulary, token dictionary metadata and Anki journals are preserved.

## Current Anki boundary
- Typed shell PrepareReaderAnkiExport carries current original/effective selection
  and signed-64 deck ID; native code freezes loaded dictionary data. Session-local
  checked u64 preparation generations never recycle on Clear/Retry. Begin matches
  generation/projected phase and both native journal gates; Clear cannot drop
  active/failed live work or durable obligations. No shell fields/handles/tokens.
- FIFO privately owns the committed attempt; begin acknowledgement grants one
  native Ready permit. CoreSession now posts one dispatch per Ready preparation,
  rechecking generation/phase/close before delivery. Dispatch consumes the permit
  once into its typed response closure. Worker cancels queued work/drops late
  callbacks on close; restart never recreates authority or replays provider writes.
- Settings now renders frozen export surface/deck, explicit Check/recheck and
  correlated Create model/Add controls, busy/error/uncertain/durable outcomes.
  Composition/navigation starts no operations. Model success never auto-adds a
  note. Success/IDs remain behind durable outcome acknowledgement. Active/failed
  phases offer no dismiss/write retry; uncertainty points to read-only recovery.
- Recovery remains Settings-only, quiet on clean startup, with explicit native-gated
  Check note/Check model/Retry. Retry restores journals read-only, invalidates lost
  reads and requires a subsequent provider check. Empty observations stay unresolved;
  only durable acknowledgement clears obligations. Async Anki renders are independent
  of reader busy/errors. Reader selection dismissal never changes frozen export data.
- Provider refreshes host/spec/permission/nonfiltered frozen deck/full layout/exact
  duplicates; invalid results/exceptions retain uncertainty. No real-provider
  compatibility claim. Schema/dependencies/generated/native-export ABI unchanged.
  Details: ARCHITECTURE.md, DATA_MODEL.md, ANKIDROID_API.md (2.25.3 evidence).
- IMPORTANT: dictionary Add entry, configured deck, permission request and browser
  flow remain unfinished. The Settings panel has no ordinary preparation entry yet.

## Next tasks (ordered)
1. Persist configured-deck settings and add explicit provider/deck/permission UI;
   then connect current dictionary selection to preparation and export presentation.
2. Complete existing-note browser/return and retained CoreSession lifecycle checks;
   validate a disposable real AnkiDroid collection plus unchanged vocabulary/history
   before compatibility claims. Include phone schema-23 Library/history upgrade.
3. Encrypted-backup format/library review follows completed Anki integration.

## Verification / blockers
- PASS: seven new Android cases; all 20 selected API-36 x86_64 cases have zero
  failures/errors/skips, plus eight native FIFO cases, debug/test APK builds,
  strict lint, Kotlin formatting and diff checks. Commands/logs and initial test
  fixture corrections: DEVELOPMENT.md, Retained Anki Ready dispatch. No blocker
  or implementation/verification work remains for this slice; full Add is unfinished.
- Real collection/browser, optimized release and physical upgrade/performance
  trials remain unrun. Broad device/schema-23 upgrade validation remains open.
  Older phone APK is source 0374365; no release requested.
