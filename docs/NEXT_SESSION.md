# Resume here
Updated: 2026-11-01. First MVP excludes Nostr and Known forms statistics.
Commit guidance: explanatory bodies; merges include provenance. Keep verification
results in progress docs/reports, out of commit message bodies.

## User checkpoints
- Announce when no plan steps remain; ask about product ambiguities. Recommend
  physical testing when it provides more value than the next implementation step.
- User's phone trial was acceptable; detailed validation remains unchecked.
  Recommend a schema-24 phone upgrade with existing Library/history before the
  full visible Add flow; rebuild optimized APK only when requested.
- Feed is only `Coming Soon! 🌱`; Nostr/identity/Q188 are beyond the first MVP.
- Statistics retains cumulative occurrences and all five statuses including Known.
  Schema 0023 drops unused forms; 0024 adds initially unconfigured Anki settings.

## Current Anki configuration boundary
- Settings explicitly loads/reloads the retained native preference, selects
  a regular provider deck or clears the default. Saves require native Ready and
  publish only matching committed acknowledgements. Failed/lost-response Retry
  reads authoritative settings on the same SQLite FIFO without repeating writes.
- Separate Check AnkiDroid uses existing read-only decks() on the provider FIFO.
  CoreSession retains Unchecked/Loading/Ready/Failed; repeated pending checks
  coalesce. Missing/filtered saved IDs stay visible without substitution/clear.
  Names are presentation; positive signed-64 IDs remain exact through i64::MAX.
- Only PermissionDenied offers an explicit permission action with rationale.
  MainActivity's RequestPermission grant rechecks if Settings remains open;
  denial leaves the error with no automatic prompt/retry. Actual dialogs untried.
- Composition/navigation starts no settings/provider I/O. Activity recreation
  retains observations; close skips queued/drops late discovery. A new session
  starts Unchecked/Unloaded. No schema/dependency/codec/native ABI changes here.
- NEW: PrepareReaderAnkiExport carries only the current selection. Native code
  requires Ready acknowledged configured settings and freezes that exact deck;
  other settings phases/no deck reject without implicit I/O. Settings changes/
  clear never retarget accepted intent; authoritative Retry readback must finish
  before new preparation after failed/lost saves. Settings dismisses the word;
  reopen it after configuration. Visible dictionary Add remains next.

## Existing Anki invariants
- Native preparation freezes the loaded original/effective selection, fields,
  layout and deck. Preparation generations never recycle. Begin matches phase/
  generation plus both journal gates; Clear preserves active/failed live work
  and obligations. Shell owns no fields/handles/permits. Settings never retargets.
- FIFO owns committed live attempts; acknowledgement grants one native permit.
  CoreSession posts once per Ready generation and rechecks phase/close. Provider
  worker cancels queued work/drops late responses; restart never replays writes.
- Settings export/recovery controls are explicit; model creation never auto-adds.
  IDs/success stay behind durable acknowledgement. Uncertainty requires read-only
  recovery; empty results do not resolve obligations. No real-provider claim.
- Provider refreshes host/spec/permission/nonfiltered frozen deck/full model/exact
  duplicates before writes. A preference/discovery snapshot grants no authority.
  Settings owns no reader busy/errors, history, export or journal gates. Details:
  ARCHITECTURE.md, DATA_MODEL.md, ANKIDROID_API.md and DECISIONS.md.

## Next tasks (ordered)
1. Add current dictionary Add presentation using the selection-only native action
   and existing frozen export projection/controls. Keep original/effective identity
   authentication and explicit provider/configuration behavior; no composition I/O.
2. Complete existing-note browser/return and remaining retained lifecycle checks;
   validate disposable real AnkiDroid collection/permission behavior and unchanged
   vocabulary/history. Include the schema-24 phone upgrade checkpoint.
3. Encrypted-backup format/library review follows completed Anki integration.

## Verification / blockers
- PASS: six new Rust cases; all 1013 workspace tests/17 opt-in ignored, strict
  Clippy, Rustfmt/Kotlin formatting, full host JNI, identical independent/Android
  Kotlin generation, both-ABI debug/test APKs, strict lintDebug, all 19 selected
  API-36 x86_64 emulator cases and diff checks. Commands/results: DEVELOPMENT.md,
  Native Anki preparation from the configured deck. No implementation/verification
  steps or blockers remain for this prerequisite. Broader Anki work remains above.
- Initial FIFO fixtures incorrectly retained a word after Settings dismissal;
  fixed by reopening it, with no production reader change. Local Gradle/ADB/
  emulator sockets/KVM required normal escalation; no dependency downloads.
- Actual permission dialogs, real collection/browser, tablet, optimized release
  and physical upgrade/performance trials remain unrun. Older phone APK is source
  0374365; no release requested. Recommend the schema-24 phone upgrade checkpoint
  before the next visible dictionary Add slice; no compatibility claim yet.
