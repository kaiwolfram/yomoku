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
  bounded authenticated streams; Argon2id/STREAM BE32/XChaCha20Poly1305, fresh
  salt/nonce, exact lengths/AAD and mandatory Last/EOF. Secrets bypass serialization.
- `Storage::create_backup_snapshot`: bound VACUUM INTO, NEW private 0700/0600 copy,
  integrity/FKs/current journal/compact size/sync, read-only stream + schema/length.
  Full encrypted fixtures compare every durable table/schema object/high-water ID.
- `BackupCandidate::create`: NEW private write-only decryption staging. Only after
  full stream authentication, consume with manifest schema/length via check_schema.
  Header/page/file length and rollback-mode input checked before SQLite opens it.
- Harden no-CREATE/no-URI READ_WRITE before introspection: defensive/query_only,
  trusted_schema/triggers/views/attach off, cell checks, memory temp and parser/VM
  limits. READ_WRITE retains CHECK parsing (read-only B-trees omit it).
- Exact all-object type/name/table/SQL match against bundled migration prefix,
  then exact journal/integrity/CHECK/FKs. Versions 1–24 migrate privately, recheck,
  sync/close. Current full state and populated schema-23 pending Anki survive.
- Opaque SchemaCheckedBackup exposes versions/discard ONLY, no path/SQL/reader or
  confirmation authority. Application payload/text/analysis/anchor/history/singleton/
  sequence invariants remain unchecked. Never authorize replacement from this type.
- Fixed-file/SQLite-sidecar cleanup only, preserving unexpected files. Explicit
  discard reports errors; Drop/error/cancel best effort. Crash cleanup still undone.
  Cancellation polls between synchronous calls/rows, not inside SQL/KDF/sync.
  Limits reduce exposure, not a fixed SQLite memory/time budget.
- Runtime checkpoint drain/mutation gating, secret handoff, external descriptors/UI,
  application validation/confirmation and activation/recovery remain unimplemented.
  Never copy live SQLite/WAL bytes or decrypt outside new private disposable staging.

## Existing Anki boundary
- Ready acknowledged configured deck + retained regular-deck discovery required.
  Explicit Check/Create/Add/Clear; no replay. Frozen original/effective preparation.
- FIFO/journal generations, commit acknowledgement and one native permit own
  provider writes. Startup/retry read-only; empty evidence resolves none.
- Per-note Open accepts frozen Existing/acknowledged NoteAdded IDs only. Package-
  scoped FIFO resolution; resumed-only launcher; pause/close/change cancels queued/
  late work. No return/recreation replay. Architecture/data/API/decisions have detail.

## Next tasks (ordered)
1. Enumerate and implement complete candidate application invariants (including
   journals/serialized payloads, anchors/history, singleton/sequence contents),
   hostile-data tests and opaque restore-ready validation before any confirmation.
2. Add runtime export sequencing/checkpoint drain/mutation gating and private
   staging crash cleanup; design nonserialized secret handoff before FFI/UI.
3. Real phone schema-24 upgrade and disposable Anki collection/permission/browser/
   Back/lifecycle trial. Anki umbrella remains unchecked pending this validation.

## Verification / blockers
- PASS: 14 new candidate cases; 1063 workspace tests/17 opt-in ignored, all-feature
  build, strict Clippy/Rustfmt, both Android storage library builds, doc links/diff.
- Commands/logs/fixed initial API compile errors: DEVELOPMENT.md, Private restore
  schema preflight. Build/tests offline. No blockers or remaining implementation/
  verification steps for this structural preflight slice.
- No application validation/replacement, APK/lint/JNI/codegen/emulator, physical
  phone/resource trial or optimized release run for this storage-only slice.

## Latest release checkpoint
- Source 0ddb81d: `target/releases/yomoku-0.1.0-0ddb81d-release.apk` (~242.3 MiB).
- Optimized ABIs/lint/signature/16-KiB alignment/assets verified; same certificate
  as earlier 0374365 phone APK. Checksum beside APK. No publishing or runtime
  trial; browser/backup changes absent. DEVELOPMENT.md has the release log.
