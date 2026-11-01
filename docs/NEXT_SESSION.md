# Resume here
Updated: 2026-11-01. First MVP excludes Nostr and Known forms statistics.
Commit guidance: concise subject/explanatory body; keep verification in docs.

## User checkpoints
- Announce when no slice steps remain; ask about product ambiguities. Recommend
  physical testing when it provides more value than the next implementation step.
- User's phone trial was acceptable; detailed matrix remains unchecked. Recommend
  a schema-24 phone upgrade retaining Library/history and disposable real Anki
  collection/permission/browser/Back trial now. Rebuild release only if requested.
- Feed is only `Coming Soon! 🌱`; Nostr/identity/Q188 are beyond the first MVP.
- Statistics keeps cumulative occurrences and all five statuses including Known.
- NEW user decision: resolve current pending/uncertain Anki writes before full
  restore replacement. Unknown/failed recovery is not clear. Preserve backup's
  own journals without replay. PRODUCT_SPEC.md §7 and BACKUP_FORMAT.md govern.

## Current backup boundary
- Version-1 design: docs/BACKUP_FORMAT.md. Reviewed exact crypto candidates/API/
  license/security limits; none adopted/compiled yet. No cryptographic audit claim.
- Dependency-free `yomoku-backup` leaf crate: strict 72-byte header, 32-byte
  encrypted-manifest metadata and 5-byte bounded frame-header codecs only.
  Parsed data never grants restore authority. No crypto, RNG, SQL or file I/O.
- Explicit Argon2id v1.3 64 MiB/3 passes/4 lanes → 32-byte key; fresh 16-byte
  salt and 19-byte STREAM BE32/XChaCha20Poly1305 prefix. Fixed 64-KiB chunks,
  mandatory authenticated Last/EOF; exact envelope+frame AAD, no downgrade.
- Full compact SQLite snapshot preserves original imports/images, analyses,
  history/provenance, pending time/settings/custom records, both Anki journals
  and ID sequences. Planned VACUUM INTO avoids deleted free-page leakage.
- No runtime/core/FFI/Android dependency or action. Secret handoff must bypass
  serialized events/views. Authentication/validation/private migration precede
  confirmed replacement; activation protocol/fault injection remain unimplemented.

## Existing Anki boundary
- Dictionary Add needs Ready acknowledged settings and retained discovery of the
  exact regular deck. Preparation authenticates current original/effective word,
  freezes fields/sentence/layout/deck and performs no I/O. No automatic Retry.
- Explicit Check/Create/Add/Clear and durable outcomes remain in both word paths
  and Settings. Settings dismisses word; reopen after configuration. Failed/lost
  preference saves need authoritative FIFO readback, never write Retry.
- Per-note Open accepts only frozen Existing/acknowledged NoteAdded IDs. Resolve
  package-scoped exact-ID links on provider FIFO; validate full current export.
  Launcher exists only while resumed. Pause/close/changed target cancel queued/
  late navigation, including same-note replacements. No return/recreation replay.
- Generations never recycle. Begin matches phase/generation/both journals; Clear
  preserves live work/obligations. FIFO owns committed attempts; acknowledgement
  grants one native permit, consumed once per Ready generation with close checks.
- Startup restores journals read-only; uncertainty needs read-only recovery and
  empty evidence resolves nothing. No model-create auto-add or restart write replay.
  Provider rechecks host/spec/permission/deck/full model/exact duplicates prewrite.
  Details: ARCHITECTURE.md, DATA_MODEL.md, ANKIDROID_API.md and DECISIONS.md.

## Next tasks (ordered)
1. Add/verify reviewed crypto dependencies and bounded authenticated stream I/O;
   known-answer vectors, wrong passwords/tampering/framing/I/O failure tests.
2. Implement validated compact full-state SQLite snapshots and complete round
   trips (including deleted records, pending time/journals and sequence marks).
3. Real phone schema-24 upgrade and disposable Anki collection/permission/browser/
   Back/lifecycle trial. Anki umbrella remains unchecked pending this validation.

## Verification / blockers
- PASS: nine new metadata tests; 1022 workspace tests/17 existing opt-in ignored,
  all-feature build, strict workspace Clippy/Rustfmt, doc links and diff checks.
  Two initial lint findings fixed; final focused rerun/full lint pass. No downloads.
- Commands/logs: DEVELOPMENT.md, Versioned backup metadata prerequisite.
  No implementation/verification steps or blockers remain for this format slice.
- No encryption, full-state round trip, restore/activation, Android/JNI/codegen,
  APK/lint/device/performance or optimized release run for this isolated crate.

## Latest release checkpoint
- Source 0ddb81d: `target/releases/yomoku-0.1.0-0ddb81d-release.apk` (~242.3 MiB).
- Optimized ABIs/strict release lint/signature/16-KiB alignment/assets verified;
  same certificate as earlier 0374365 phone APK. Checksum beside APK. No publishing
  or install/runtime trial; browser/backup changes are absent. DEVELOPMENT.md logs.
