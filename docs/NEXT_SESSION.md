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
- Version-1 format: docs/BACKUP_FORMAT.md. Independent `yomoku-backup` leaf crate
  now has strict metadata and bounded authenticated snapshot Read/Write APIs.
- Pinned Argon2id 0.5.3 (v1.3, 64 MiB/3 passes/4 lanes → 32-byte key),
  chacha20poly1305 0.11.0, aead-stream 0.6.0, getrandom 0.4.3, zeroize 1.9.0.
  Fresh independent 16-byte salt/19-byte nonce on every export attempt. Defaults
  off for crypto; explicit zeroize/alloc features. No independent audit claim.
- Fixed 64-KiB chunks, exact envelope/frame AAD, mandatory authenticated Last/EOF,
  checked totals/counters, encrypted manifest and exact SQLite length. No bulk
  allocation from metadata. Wrong password/failed tags share Authentication.
- Owned non-Debug/nonserialized Password consumed after KDF. Password/key/scratch/
  chunk guards clear Rust-owned buffers. Cancellation is cooperative around KDF/
  bounded calls/flush; blocking calls cannot be interrupted. Caller copies remain.
- No paths/files opened or managed. Encrypt requires coherent validated snapshot.
  Decrypt writes only to caller-owned new private staging, returning Manifest after
  full authentication/length/EOF/flush. Late errors may leave prior plaintext;
  caller must discard failures, clean up after crashes and sync/close successes.
- No SQL/schema validation/restore authority, runtime/core/FFI/Android dependency
  or action. Secret handoff must bypass serialized events/views. Full compact
  snapshot, hardened validation/private migration and activation remain undone.
- Preserve all imports/images/analysis/history/deleted provenance/pending time/
  settings/custom records/both Anki journals/ID sequences. Planned VACUUM INTO
  purges deleted free-page content; never copy a live SQLite/WAL file normally.

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
1. Implement compact full-state SQLite snapshots and complete encrypted round
   trips, including deleted records, pending time/journals and sequence marks.
2. Implement hardened private SQLite/schema/application validation and supported
   migration before any restore confirmation; hostile schema/failure tests.
3. Real phone schema-24 upgrade and disposable Anki collection/permission/browser/
   Back/lifecycle trial. Anki umbrella remains unchecked pending this validation.

## Verification / blockers
- PASS: 18 new stream + nine metadata tests; 1040 workspace tests/17 opt-in ignored,
  all-feature build, strict Clippy/Rustfmt, independent reference vectors, both
  Android target library builds, local doc links and diff checks. Initial compile/
  helper-lint adjustments fixed; final focused/build/lint/target reruns pass.
- Commands/logs: DEVELOPMENT.md, Bounded authenticated backup stream prerequisite.
  Initial fetch downloaded new dependencies; remaining checks offline. No blockers
  or implementation/verification steps remain for this stream prerequisite.
- No snapshot/full-state SQL validation/replacement, APK/lint/JNI/codegen/emulator,
  physical phone/resource trial or optimized release run for this isolated crate.

## Latest release checkpoint
- Source 0ddb81d: `target/releases/yomoku-0.1.0-0ddb81d-release.apk` (~242.3 MiB).
- Optimized ABIs/lint/signature/16-KiB alignment/assets verified; same certificate
  as earlier 0374365 phone APK. Checksum beside APK. No publishing or runtime
  trial; browser/backup changes absent. DEVELOPMENT.md has the release log.
