# Resume here
Updated: 2026-11-02. First MVP excludes Nostr and Known forms statistics.

## User checkpoints
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
- Canonical checks: complete chapter order/live text/deleted provenance, UTF-8
  titles, every position/pending-time anchor's ownership/scalar bounds. Preserve
  empty original titles, absent legacy positions, EOF and zero-time tombstones.
- Private typed effective records validate detached domains/projections/issued IDs,
  receipts and surviving encounters. Preserve unfinished reservations, old rules,
  whitespace gaps and erased content/vocabulary without replay/live commands.
- Retained analyses validate exact coverage/indices/metadata; frozen effective
  records match live text, saved-preset pages and exact B/alternate A/C headers/
  tokens. Legacy absence/deleted history survives.
- Exact paste/file hashes and required format/encoding/EPUB-header ownership now
  validate. Pure TXT/SRT adapters match saved text and complete ruby/cue lists; all
  ruby domains/anchors validate. Preserve legacy hashes, encodings and deleted files.
- Opaque SchemaCheckedBackup exposes versions/discard ONLY. EPUB metadata/assets/
  spans/images, override provenance and history/status checks remain:
  RESTORE_VALIDATION.md. No confirmation/restore authority or erased-history recovery.
- Fixed-file/SQLite-sidecar cleanup preserves unexpected files; discard reports
  errors, Drop/error/cancel best effort. Crash cleanup remains undone.
  Cancellation between rows/segments/1-MiB hash chunks and around text parsing;
  SQL/JSON/KDF/decoding/parsing/pagination/sync stay synchronous. One file/chapter
  projection at a time; no total resource budget. Runtime drain/gating,
  secrets/descriptors/UI/activation undone.

## Existing Anki boundary
- Ready acknowledged configured deck/regular-deck discovery required. Explicit
  Check/Create/Add/Clear; no replay. FIFO/journal generations/commit/native permit
  own provider writes. Startup/retry read-only; empty evidence resolves none.
- Open uses frozen Existing/acknowledged NoteAdded IDs only, package-scoped FIFO and
  resumed launcher. Pause/close/change cancels queued/late work; no lifecycle replay.

## Next tasks (ordered)
1. Continue RESTORE_VALIDATION.md: EPUB metadata/assets/chapter hashes/spans/images
   and override provenance, then historical page/anchor/status chains before
   restore-ready. No confirmation/replacement authority.
2. Runtime export sequencing/checkpoint drain/mutation gating and private staging
   crash cleanup; design nonserialized secret handoff before FFI/UI.
3. Real phone schema-24 upgrade and disposable Anki collection/permission/browser/
   Back/lifecycle trial. Anki umbrella remains unchecked pending this validation.

## Verification / blockers
- PASS: 9 new/68 candidate cases; 1117 workspace tests/17 opt-in ignored,
  all-feature build, strict Clippy/Rustfmt, both Android storage builds, links/diff.
- Commands/logs: DEVELOPMENT.md, Restore candidate file identity/text annotations.
  Offline; test-only fixture path corrected. No failures/blockers or slice steps.
  No APK/lint/JNI/codegen/emulator/release/phone/resource trial run.

## Latest release checkpoint
- Source 0ddb81d: `target/releases/yomoku-0.1.0-0ddb81d-release.apk` (~242.3 MiB).
  Optimized ABIs/lint/signature/16-KiB alignment/assets verified, certificate 0374365.
  Checksum beside APK. No publishing/trial; browser/backup changes absent. See DEVELOPMENT.md.
