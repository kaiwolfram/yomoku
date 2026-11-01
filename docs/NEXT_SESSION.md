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
  Unknown/failed recovery does not clear this gate. Preserve backup journals
  without replay. PRODUCT_SPEC.md §7/BACKUP_FORMAT.md govern; no replacement yet.

## Current backup boundary
- Version-1 format/profile: BACKUP_FORMAT.md. Independent `yomoku-backup` owns
  bounded authenticated streams; Argon2id/STREAM BE32/XChaCha20Poly1305, fresh
  salt/nonce, exact lengths/AAD and mandatory Last/EOF. Secrets bypass serialization.
- `Storage::create_backup_snapshot`: bound VACUUM INTO, NEW private 0700/0600 copy,
  integrity/FKs/current journal/compact size/sync, read-only stream + schema/length.
  Encrypted fixtures compare every durable table/schema object/high-water ID.
- `BackupCandidate::create`: NEW private write-only staging. Only after full
  authentication, consume with manifest schema/length via check_schema.
- Exact header/page/file length and rollback format before no-CREATE/no-URI SQLite
  READ_WRITE. Harden defensive/query_only/trusted_schema/triggers/views/attachments,
  parser/VM limits/cell checks/memory temp before schema parsing (retains CHECKs).
- Exact all-object type/name/table/SQL against bundled migration prefix, then
  journal/integrity/CHECK/FKs. Versions 1–24 migrate privately/recheck/sync/close.
- Partial checks before/after migration: required singleton rows, Library ordinals/
  clock, trusted AUTOINCREMENT counters, all Anki payloads/IDs, saved vocabulary/
  custom-entry domains. Preserve exact text/historical layouts/deleted counters.
- Private typed effective records now validate detached command/segment/metadata/
  projection/issued-ID bounds and receipts/headers/surviving encounters. No live
  command construction. Preserve unfinished reservations, old rules and erased
  content/vocabulary without replay. Saved segments can omit whitespace gaps.
- Opaque SchemaCheckedBackup exposes versions/discard ONLY. Surviving canonical
  text/logical pages/analysis associations and other anchor/history/status checks
  remain incomplete: RESTORE_VALIDATION.md inventories precise continuation points.
  Cannot reconstruct erased high-water history. No confirmation/restore authority.
- Fixed-file/SQLite-sidecar cleanup preserves unexpected files; explicit discard
  reports errors, Drop/error/cancel best effort. Crash cleanup remains undone.
  Cancellation between calls/rows/segments, not inside SQL/JSON/KDF/sync; no total
  resource budget. Runtime drain/gating, secrets/descriptors/UI/activation undone.

## Existing Anki boundary
- Ready acknowledged configured deck + retained regular-deck discovery required.
  Explicit Check/Create/Add/Clear; no replay. Frozen original/effective preparation.
- FIFO/journal generations, commit acknowledgement and one native permit own
  provider writes. Startup/retry read-only; empty evidence resolves none.
- Per-note Open accepts frozen Existing/acknowledged NoteAdded IDs only. Package-
  scoped FIFO resolution; resumed-only launcher; pause/close/change cancels queued/
  late work. No return/recreation replay. Architecture/data/API/decisions have detail.

## Next tasks (ordered)
1. Continue RESTORE_VALIDATION.md: surviving text/page/analysis associations and
   other anchor/history/status semantics; hostile-data tests before restore-ready.
   Detached effective checks complete; no confirmation/replacement authority yet.
2. Runtime export sequencing/checkpoint drain/mutation gating and private staging
   crash cleanup; design nonserialized secret handoff before FFI/UI.
3. Real phone schema-24 upgrade and disposable Anki collection/permission/browser/
   Back/lifecycle trial. Anki umbrella remains unchecked pending this validation.

## Verification / blockers
- PASS: 9 new/37 candidate cases; 1086 workspace tests/17 opt-in ignored,
  all-feature build, strict Clippy/Rustfmt, both Android storage builds, links/diff.
- Commands/logs: DEVELOPMENT.md, Restore candidate detached effective records.
  Initial decoder/fixture issues corrected. Checks offline; no blockers or remaining
  implementation/verification steps for this slice. No APK/lint/JNI/codegen/emulator/
  release/phone/resource trial run.

## Latest release checkpoint
- Source 0ddb81d: `target/releases/yomoku-0.1.0-0ddb81d-release.apk` (~242.3 MiB).
- Optimized ABIs/lint/signature/16-KiB alignment/assets verified; same certificate
  as earlier 0374365 phone APK. Checksum beside APK. No publishing/runtime trial;
  browser/backup changes absent. DEVELOPMENT.md has the release log.
