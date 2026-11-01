# Resume here
Updated: 2026-11-01. First MVP excludes Nostr and Known forms statistics.
Commit guidance: explanatory bodies; merges include provenance. Keep verification
results in progress docs/reports, out of commit message bodies.

## User checkpoints
- Announce when no plan steps remain; ask about product ambiguities. Recommend
  physical testing when it provides more value than the next implementation step.
- User's phone trial was acceptable; detailed validation remains unchecked.
  Recommend a schema-24 phone upgrade with existing Library/history now, before
  further provider integration. Rebuild optimized APK only when requested.
- Feed is only `Coming Soon! 🌱`; Nostr/identity/Q188 are beyond the first MVP.
- Statistics retains cumulative occurrences and all five statuses including Known.
  Schema 0023 drops unused forms; 0024 adds initially unconfigured Anki settings.

## Current dictionary / Anki boundary
- NEW: both dictionary paths show Add after Ready acknowledged configuration and
  retained successful discovery containing that exact regular deck. Otherwise an
  explicit Settings route explains setup; composition/preparation starts no I/O.
  Settings dismisses the word; reopen after configuration. Discovery is no permit.
- Add sends only the authenticated loaded original/effective selection to native
  preparation. Rejected preparation shows generic Settings/recovery guidance,
  never automatic Retry. Original details must match presented token/key/selection.
- Accepted intent shows existing frozen export controls in the dictionary and
  Settings: explicit Check/recheck, correlated Create/Add/Clear, busy phases and
  durable outcomes. Selecting another word/occurrence or changing/clearing the
  preference cannot relabel/retarget it. Actions count as reader interaction.
- Settings explicitly loads/reloads the preference, selects a regular deck or
  clears it. Saves publish matching committed acknowledgements only. Failed/lost
  saves require authoritative FIFO readback before new preparation; no write Retry.
- Check AnkiDroid uses read-only decks() on the provider FIFO. CoreSession retains
  Unchecked/Loading/Ready/Failed, coalesces pending reads and keeps missing/filtered
  saved IDs without substitution. Names are presentation; IDs remain exact i64.
- Only PermissionDenied offers the explicit permission action/rationale. Grant
  rechecks if Settings remains open; denial never auto-prompts/retries. Actual
  dialogs untried. Activity recreation retains state; close skips queued/drops
  late work. New sessions start Unchecked/Unloaded. No schema/dependency/codec change.

## Existing Anki invariants
- Native preparation freezes selection/fields/layout/deck. Generations never
  recycle. Begin matches phase/generation and both journals; Clear preserves
  active/failed live work and obligations. Shell owns no fields/handles/permits.
- FIFO owns committed live attempts; matching acknowledgement grants one native
  permit. CoreSession posts once per Ready generation and rechecks phase/close.
  Provider close cancels queued work/drops late responses; restart replays no write.
- Model creation never auto-adds. Success/IDs stay behind durable acknowledgement.
  Uncertainty requires read-only recovery; empty observations resolve nothing.
- Provider refreshes host/spec/permission/frozen regular deck/full model/exact
  duplicates before writes. Settings owns no reader busy/errors/history or gates.
  Details: ARCHITECTURE.md, DATA_MODEL.md, ANKIDROID_API.md and DECISIONS.md.

## Next tasks (ordered)
1. Implement existing-note browser/open and return handling using supported APIs;
   retain exact frozen identity/IDs and verify unavailable handlers and lifecycle.
2. Validate a disposable real AnkiDroid collection/permission/browser flow and
   unchanged vocabulary/history; include the schema-24 phone upgrade checkpoint.
3. Encrypted-backup format/library review follows completed Anki integration.

## Verification / blockers
- PASS: seven new cases; 49/50 initial selected API-36 x86_64 emulator cases, then
  corrected legacy highlight assertion passes its targeted rerun. All 50 unique
  selected cases verified; final debug/test APKs, strict lintDebug, Kotlin format
  and diff checks pass. Commands/results: DEVELOPMENT.md, Dictionary Add to Anki
  presentation. No implementation/verification steps or blockers remain here.
- Legacy assertion inspected a dynamic link style removed by the earlier renderer;
  it now checks the existing background span, with no production highlight change.
  Gradle/ADB/emulator needed normal local-socket/KVM escalation. No downloads.
- Kotlin-only slice: host Rust/Clippy/JNI/codegen not rerun; emulator cases exercise
  actual packaged JNI/FIFO/SQLite with fake provider operations. No compatibility
  claim. Actual dialogs/collection/browser, phone/tablet, optimized release and
  performance trials remain unrun. Older phone APK is source 0374365.

## Latest documentation checkpoint
- README.md now contains release build/sign/install steps and upgrade-key guidance.
- PASS: four shell blocks (`bash -n`), local links, installed CLI options and
  `./android/gradlew -p android :app:assembleRelease :app:lintRelease -x :app:buildRustBridge --offline --console=plain --dry-run`.
- Used the DEVELOPMENT.md local JDK/SDK/Gradle environment; Gradle required normal
  socket escalation. Log: `/tmp/yomoku-readme-release-gradle.log`. No blocker.
- Documentation only: no APK rebuild, signing, installation or runtime tests.
  No implementation/verification steps remain for this request; next tasks above.
