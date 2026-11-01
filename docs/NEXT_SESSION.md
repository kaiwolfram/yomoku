# Resume here
Updated: 2026-11-01. First MVP excludes Nostr and Known forms statistics.
Commit guidance: explanatory bodies; merges include provenance. Keep verification
results in progress docs/reports, out of commit message bodies.

## User checkpoints
- Announce when no plan steps remain; ask about product ambiguities. Recommend
  physical testing when it provides more value than the next implementation step.
- User's phone trial was acceptable; detailed device/workflow validation remains
  unchecked. Recommend a schema-24 upgrade trial with existing Library/history
  before the full visible Add flow; rebuild optimized APK only when requested.
- Feed is only `Coming Soon! 🌱`; Nostr/identity/Q188 are beyond the first MVP.
- Statistics retains cumulative occurrences and all five statuses including Known.
  Schema 0023 drops only unused vocabulary_dictionary_forms; 0024 adds initially
  unconfigured Anki settings. Reader/history/vocabulary/journals stay intact.

## Current configured-deck boundary
- NEW: typed LoadAnkiSettings/SetAnkiDefaultDeck(Option<i64>)/RetryAnkiSettings
  connect migration-0024 repository to retained core state, SQLite FIFO and FFI.
  Events require Settings open; navigation/composition starts no settings I/O.
  No visible control dispatches them yet. Provider/deck/permission UI comes next.
- AnkiSettingsView separates Unloaded/Loading/Ready/Saving/Failed and retains the
  last acknowledged deck plus a separate storage error. Positive signed-64 IDs
  remain exact through i64::MAX; None clears. Only Ready accepts changed saves.
  Checked native request generations and exact expected save results gate publish.
- Retry always reads authoritative settings after old work on the same FIFO;
  lost/failed save responses cannot repeat an update. Old callbacks are ignored.
  Restart begins Unloaded. Missing/corrupt rows remain errors without repair.
- Settings work owns no reader busy/errors, export/preflight/live state or journal
  gate. A preference grants no provider authority and never retargets frozen work.
  Schema/dependencies/native-export ABI unchanged; Event/ViewModel codecs changed.

## Existing Anki invariants
- Native preparation freezes current loaded original/effective selection, fields,
  layout and explicit deck. Session-local preparation generations never recycle.
  Begin matches generation/phase plus both journal gates; Clear preserves active/
  failed live work and durable obligations. No shell fields/handles/permits.
- FIFO owns committed live attempts; acknowledgement grants one native permit.
  CoreSession posts once per Ready generation and rechecks phase/close. Provider
  worker cancels queued work and drops late responses; restart never replays writes.
- Settings export/recovery controls are explicit. Composition/navigation starts
  no provider operations. Model creation never auto-adds; IDs/success stay behind
  durable acknowledgement. Uncertainty uses read-only recovery; empty results do
  not resolve obligations. Reader selection dismissal cannot change frozen data.
- Provider refreshes host/spec/permission/nonfiltered frozen deck/full model/exact
  duplicates before writes. Invalid results retain uncertainty. No real-provider
  compatibility claim. Details: ARCHITECTURE.md, DATA_MODEL.md, ANKIDROID_API.md.

## Next tasks (ordered)
1. Add explicit provider/deck/permission controls using retained settings; wire
   configured-deck preparation and current dictionary Add presentation.
2. Complete existing-note browser/return and retained CoreSession lifecycle checks;
   validate a disposable real AnkiDroid collection plus unchanged vocab/history.
   Include the phone schema-24 Library/history upgrade checkpoint.
3. Encrypted-backup format/library review follows completed Anki integration.

## Verification / blockers
- PASS: 11 new Rust cases; 1007 workspace tests/17 opt-in ignored, strict Clippy,
  Rustfmt/Kotlin formatting and independently identical Kotlin generation.
- PASS: full host Kotlin/JNI including settings persistence and codec exchange.
- PASS: both-ABI debug/test APKs, lintDebug and identical Android generation.
  Commands/results: DEVELOPMENT.md, Retained configured Anki deck through core/FIFO/FFI.
  No implementation/verification steps or blockers remain for this slice.
- Real collection/browser, optimized release and physical upgrade/performance
  trials remain unrun. Older phone APK is source 0374365; no release requested.
