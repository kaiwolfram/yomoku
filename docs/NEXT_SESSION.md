# Resume here
Updated: 2026-11-01. First MVP excludes Nostr and Known forms statistics.
Commit guidance: explanatory bodies; merges include provenance. Keep verification
results in progress docs/reports, out of commit message bodies.

## User checkpoints
- Announce when no plan steps remain; ask about product ambiguities. Recommend
  physical testing when it provides more value than the next implementation step.
- User's phone trial was acceptable; detailed validation remains unchecked.
  Recommend a schema-24 phone upgrade with existing Library/history now.
  Rebuild optimized APK only when requested; existing artifact is below.
- Feed is only `Coming Soon! 🌱`; Nostr/identity/Q188 are beyond the first MVP.
- Statistics retains cumulative occurrences and all five statuses including Known.
  Schema 0023 drops unused forms; 0024 adds initially unconfigured Anki settings.

## Current Anki boundary
- Dictionary Add requires Ready acknowledged settings and retained successful
  discovery containing the exact regular deck. Otherwise route to Settings.
  Preparation authenticates only the loaded original/effective selection; no I/O.
  Rejected preparation offers generic Settings/recovery guidance, no auto-Retry.
- Accepted intent freezes surface/fields/layout/deck and retains explicit Check,
  correlated Create/Add/Clear, busy phases and durable outcomes in both dictionary
  paths and Settings. Changing word/occurrence/preference cannot retarget it.
- NEW: per-note Open uses only frozen Existing or acknowledged NoteAdded IDs.
  AnkiNoteBrowser validates the whole native export/current presentation and
  resolves the package-scoped exact-ID browser link on the provider FIFO.
  MainActivity attaches the launcher only while resumed; pause/close/target change
  cancels queued/late navigation. Resume/recreation never replay cancelled Open.
  Failures stay visible with explicit Open available again. No editing-result
  contract: return uses existing reader-clock lifecycle and changes no history,
  vocabulary, provider state or export. Real handler/Back behavior remains untried.
- Settings explicitly loads/reads back/selects/clears the preference. Only matching
  committed saves publish; failed/lost saves require authoritative FIFO readback.
  No write Retry. Settings dismisses the word; reopen after configuration.
- Check AnkiDroid is explicit read-only deck discovery on the same provider FIFO;
  retained/coalesced state preserves missing/filtered IDs without substitution.
  Only PermissionDenied offers an explicit prompt; grant rechecks if Settings is
  open, denial never auto-prompts/retries. New sessions start Unchecked/Unloaded.

## Existing Anki invariants
- Generations never recycle. Begin matches phase/generation and both journals;
  Clear preserves active/failed live work and obligations. Shell owns no permit.
- FIFO owns committed live attempts; matching acknowledgement grants one native
  permit. CoreSession invokes once per Ready generation and rechecks phase/close.
  Close cancels queued/drops late work; restart replays no write or navigation.
- Model creation never auto-adds. Success/IDs require durable acknowledgement.
  Uncertainty requires read-only recovery; empty observations resolve nothing.
- Provider refreshes host/spec/permission/frozen deck/full model/exact duplicates
  before writes. Settings owns no reader busy/errors/history or gates. Details:
  ARCHITECTURE.md, DATA_MODEL.md, ANKIDROID_API.md and DECISIONS.md.

## Next tasks (ordered)
1. Validate a disposable real AnkiDroid collection/permission/browser/Back flow,
   unchanged vocabulary/history and lifecycle; include the schema-24 phone upgrade.
2. Encrypted-backup format/library review follows completed Anki integration.

## Verification / blockers
- PASS: nine new cases; 89 initial selected API-36 x86_64 cases plus final six-case
  browser run including same-note cancellation regression. All 90 unique cases
  verified (95 executions), zero failures/errors/skips. Final debug/test APKs,
  strict lintDebug, Kotlin format and diff pass. DEVELOPMENT.md has commands/logs.
- Gradle/ADB/emulator needed normal local-socket/KVM escalation. No downloads.
  Kotlin-only: host Rust/Clippy/JNI/codegen not rerun; emulator exercises actual
  packaged JNI/FIFO/SQLite with fake provider operations. No compatibility claim.
- No implementation/verification steps or blockers remain for the browser slice.
  Actual provider/dialog/browser/Back, phone/tablet, performance and optimized
  release trials remain unrun. Anki umbrella stays unchecked pending real trial.

## Latest release checkpoint
- Source 0ddb81d: `target/releases/yomoku-0.1.0-0ddb81d-release.apk` (~242.3 MiB).
- PASS: optimized ABIs, assembly/strict lintRelease, signature, ZIP/ELF 16 KiB
  alignment, CRC/assets/JNI/manifest and debug-probe exclusion. Certificate matches
  older phone APK 0374365. No install/runtime trial or publishing. Checksum beside
  APK; commands/logs in DEVELOPMENT.md. Browser changes are not in that APK.
