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
- Resolve current pending/uncertain Anki writes before restore replacement.
  Unknown/failed recovery is not clear. Preserve backup journals without replay.
  PRODUCT_SPEC.md §7 and BACKUP_FORMAT.md govern; no replacement exists yet.

## Current backup boundary
- Version-1 format/profile: BACKUP_FORMAT.md. Independent `yomoku-backup` owns
  bounded authenticated streams; fixed Argon2id/STREAM BE32/XChaCha20Poly1305,
  fresh salt/nonce, exact lengths/AAD and mandatory Last/EOF. Secrets bypass
  serialization; guarded Rust buffers cannot clear caller/JVM copies.
- `Storage::create_backup_snapshot` now uses bound VACUUM INTO in a NEW private
  0700 directory/0600 file. Checks integrity/FKs/current migration journal/zero
  freelist/page length, syncs, exposes only a read-only reader plus schema/length.
  Explicit discard reports cleanup errors; Drop/error/cancel clean best effort.
- SQLite 3.53.2 read-only B-trees omit CHECK expressions. Validate only the owned
  copy through READ_WRITE without CREATE and query_only before schema parsing.
  Corrupt-settings regression covers this. This is not hardened import validation.
- Full encrypted fixtures populate/compare every durable table/schema object and
  sequence, preserving deleted provenance, imports/images/analysis/history/time/
  settings/custom records/both Anki journals; reads and allocations remain usable.
  Crypto is a storage dev dependency only; no new production dependency/schema.
- Runtime must still own checkpoint drain/mutation gating and private-parent crash
  cleanup. Cancellation polls between synchronous calls, not inside VACUUM/KDF/
  validation/sync. Never copy live SQLite/WAL bytes for backups.
- Decrypt only into new private disposable staging; late failure can leave prior
  plaintext. Full authentication/length/EOF/flush is not schema/restore authority.
  Hardened validation/private migration, activation and secret FFI/UI remain undone.

## Existing Anki boundary
- Exact original/effective preparation needs Ready acknowledged configured deck
  and retained regular-deck discovery. Explicit Check/Create/Add/Clear; no replay.
- FIFO/journal generations, commit acknowledgement and one native permit own
  provider writes. Startup/retry restores read-only; empty evidence resolves none.
- Per-note Open accepts only frozen Existing/acknowledged NoteAdded IDs. Exact
  package-scoped resolution on provider FIFO; resumed-only Activity launcher.
  Pause/close/changed projection cancels queued/late work; no return/recreation replay.
- Details: ARCHITECTURE.md, DATA_MODEL.md, ANKIDROID_API.md and DECISIONS.md.

## Next tasks (ordered)
1. Implement hardened private SQLite/schema/application validation and supported
   migration before any restore confirmation; hostile schema/failure tests.
2. Add runtime export sequencing/checkpoint drain/mutation gating and private
   staging crash cleanup; design nonserialized secret handoff before FFI/UI.
3. Real phone schema-24 upgrade and disposable Anki collection/permission/browser/
   Back/lifecycle trial. Anki umbrella remains unchecked pending this validation.

## Verification / blockers
- PASS: nine new snapshot cases; 1049 workspace tests/17 opt-in ignored, all-feature
  build, strict Clippy/Rustfmt, both Android storage target library builds, local
  doc links and diff checks. Initial API/fixture/CHECK-validation issues corrected.
- Commands/logs: DEVELOPMENT.md, Compact full-state backup snapshots. Checks offline.
  No blockers or implementation/verification steps remain for this snapshot slice.
- No hardened import validation/replacement, APK/lint/JNI/codegen/emulator, physical
  phone/resource trial or optimized release run for this storage-only slice.

## Latest release checkpoint
- Source 0ddb81d: `target/releases/yomoku-0.1.0-0ddb81d-release.apk` (~242.3 MiB).
- Optimized ABIs/lint/signature/16-KiB alignment/assets verified; same certificate
  as earlier 0374365 phone APK. Checksum beside APK. No publishing or runtime
  trial; browser/backup changes absent. DEVELOPMENT.md has the release log.
