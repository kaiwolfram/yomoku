# Manual backup format, version 1

The cryptographic stream primitives, compact SQLite snapshots and partial private
candidate validation exist. App secret handoff, export/import UI, crash cleanup,
complete semantic validation and atomic replacement remain **unfinished separate
release work**. This format is not an end-to-end release/security claim.

Requirements: [PRODUCT_SPEC.md §7](PRODUCT_SPEC.md#7-secure-manual-backup-and-restore).
The format is independent of Crux/Bincode/FFI payloads.

## Complete local state

The payload is one compact consistent SQLite snapshot of the **initial schema 1**,
with the [26 application tables](DATA_MODEL.md) and `sqlite_sequence`. It retains
file digests and encoding, canonical base text/images/formatting, analysis identities,
vocabulary/status history, counted occurrences/completions, progress,
reader settings, Statistics timeframe, default Anki deck, migration journal and ID high-water marks.
There are no custom entries, segmentation overrides, Anki export journals or
recovery records. SRT cues are transient and canonical blank-line separators
remain in the snapshot. Only HTTP(S) EPUB links have
annotation rows. Reading durations are not saved.

The journal is `0001_canonical_import_initial_schema`, schema version 1, including the
Statistics timeframe preference. Earlier development schemas and journals are
rejected unchanged; no clearing is attempted. The cryptographic envelope and stream format are unchanged.

Bundled dictionaries/fonts/notices, transient routes/selections/callbacks/caches,
provider permissions/results/active ownership, external Anki collections, file
picker grants and signing keys are excluded. No Nostr identity exists in this
release. A saved deck ID is a preference, never current collection authority.
Restore has no Anki recovery prerequisite and never replays external writes.

`Storage::create_backup_snapshot` uses bound `VACUUM main INTO ?1` on a new
private 0700 directory/0600 file, preserving IDs/sequences while excluding deleted
free-page contents. Never copy live SQLite/WAL/journal files with filesystem reads.
The caller must drain pending reader mutations and gate new mutations on the FIFO.
Completed snapshots verify journal/integrity/FKs/page size/zero freelist, sync and
expose only an owned reader plus schema/length. Existing paths/symlinks are refused.
Drop/error/cancel attempt cleanup; explicit discard reports failures. Process-death
cleanup remains an app obligation. No live database vacuum policy changes.

The [SQLite VACUUM contract](https://www.sqlite.org/lang_vacuum.html),
[integrity checks](https://www.sqlite.org/pragma.html#pragma_integrity_check) and
[query_only](https://www.sqlite.org/pragma.html#pragma_query_only) govern snapshot
validation. Query-only validation on an owned read/write handle is needed because
the pinned SQLite omits CHECK expressions for read-only B-trees. Snapshot checks
are not complete hostile-candidate validation or restore authority.

## Cryptographic profile and dependency review

Use RustCrypto Argon2id v1.3 with **65,536 KiB**, **3 passes**, **4 lanes**, a fresh
**16-byte salt**, and a **32-byte derived key**. These are the memory-constrained
profile in [RFC 9106 §4 and §7.4](https://www.rfc-editor.org/rfc/rfc9106.html#section-4).
Set every parameter explicitly. Version 1 import accepts exactly that profile,
rejecting all other algorithm IDs/costs before invoking the KDF. Measure phone
latency/memory before shipping; resource failure is an error, never a weaker KDF.

Use the library's **STREAM BE32 over XChaCha20Poly1305**, with a fresh **19-byte
random stream nonce prefix**, **16-byte authentication tags**, and **65,536-byte
nonfinal plaintext segments**. STREAM owns counters and finality. Never implement
the nonce construction, ChaCha/Poly1305 or Argon2 primitives in Yomoku. This uses
an established online authenticated-encryption construction to handle databases
larger than RAM. [RustCrypto's STREAM documentation](https://docs.rs/aead-stream/0.6.0/aead_stream/)
describes the construction and its ordering/truncation properties.

Adopted exact direct dependencies in Cargo.toml/Cargo.lock. Published documentation
and downloaded source APIs/features were rechecked, and the isolated crate builds
for both Android targets. It is not linked into the app yet:

| Library | Adopted API/features and evidence |
| --- | --- |
| `argon2 =0.5.3` | Pure Rust, MIT/Apache-2.0. Explicit `Algorithm::Argon2id`, `Version::V0x13`, `Params::new(65536, 3, 4, Some(32))`. Binary `hash_password_into_with_memory`, never PHC password-verification strings. Defaults disabled; only `zeroize` enabled. Caller-owned fallibly allocated `Zeroizing<Vec<Block>>` clears KDF scratch on every exit. [Key derivation](https://docs.rs/argon2/0.5.3/argon2/), [parameters](https://docs.rs/argon2/0.5.3/argon2/struct.Params.html), [features](https://docs.rs/crate/argon2/0.5.3/features). |
| `chacha20poly1305 =0.11.0` | Pure Rust, MIT/Apache-2.0; `XChaCha20Poly1305` has a 24-byte full nonce and a 32-byte key. Disable defaults, enable `alloc`/`zeroize`; no reduced-round cipher. The crate reports a historical NCC Group audit; that does not audit this application, STREAM composition or every later release. Its constant-time multiplication requirement fits the supported arm64/x86_64 targets; runtime verification remains. [Security notes](https://docs.rs/crate/chacha20poly1305/0.11.0), [manifest/features](https://docs.rs/crate/chacha20poly1305/0.11.0/source/Cargo.toml). |
| `aead-stream =0.6.0` | MIT/Apache-2.0, pure Rust; same `aead 0.6` dependency family as the selected cipher. `EncryptorBE32`/`DecryptorBE32`, `new`, `*_next_in_place`/`*_last_in_place` with AAD. `last` consumes the state. [API](https://docs.rs/aead-stream/0.6.0/aead_stream/struct.Encryptor.html), [manifest](https://docs.rs/crate/aead-stream/0.6.0/source/Cargo.toml), [counter/nonce source](https://docs.rs/aead-stream/0.6.0/src/aead_stream/lib.rs.html). |
| `getrandom =0.4.3` | MIT/Apache-2.0; `fill` obtains OS randomness on Linux/Android. Fail closed on entropy errors; no clock/ID/random fallback. Generate independent fresh salt and prefix for every attempt, including Retry. [API/targets](https://docs.rs/getrandom/0.4.3/getrandom/). |
| `zeroize =1.9.0` | MIT/Apache-2.0, `alloc` enabled. Use `Zeroizing`/explicit clearing for password bytes, key, KDF memory and plaintext buffers. Owned Rust buffers can be cleared; this cannot promise elimination of Android/JVM copies, allocator copies, registers or OS caches. [Guarantees/limits](https://docs.rs/zeroize/1.9.0/zeroize/). |

No new native service, network access or runtime is required. The complete graph
is locked, including shared `aead 0.6.1`; existing unrelated versions are unchanged.
Published known-answer vectors, independent reference fixtures and both Android
target library builds pass. Native runtime/resource checks and redistribution
notices for the added graph are required before Android integration/distribution.
The selected Argon2 and STREAM implementations are not claimed independently
audited here. This review is a design/API review, not a cryptographic audit.

Passwords are literal UTF-8 bytes, without trimming, normalization or case folding.
Do not persist/log them, their derived key or plaintext content. Password policy
and confirmation UI belong to the export slice. An archive reveals its format,
KDF profile and approximate size; it provides no rollback prevention or proof of
which installation created it. A strong password remains necessary.

## Outer header: exactly 72 bytes

All integer fields in Yomoku metadata use **unsigned little endian**. STREAM's
internal counter encoding belongs to the library and is not this framing.
Unknown versions/profiles, nonzero reserved bytes and incorrect declared lengths
are errors; no heuristic format detection or best-effort downgrade.

| Offset | Bytes | Field/value |
| --- | --- | --- |
| 0 | 8 | ASCII `YOMOKUBK` |
| 8 | 2 | Envelope version = 1 |
| 10 | 2 | Header length = 72 |
| 12 | 1 | KDF ID = 1 (Argon2id) |
| 13 | 1 | Argon2 version = 0x13 |
| 14 | 1 | AEAD ID = 1 (XChaCha20Poly1305) |
| 15 | 1 | Stream ID = 1 (STREAM BE32) |
| 16 | 4 | Memory KiB = 65,536 |
| 20 | 4 | Iterations = 3 |
| 24 | 4 | Lanes = 4 |
| 28 | 4 | Nonfinal plaintext segment bytes = 65,536 |
| 32 | 16 | Salt |
| 48 | 19 | Stream nonce prefix |
| 67 | 5 | Reserved = all zero |

The header is public and untrusted until authenticated. Structural parsing alone
cannot prove salt/nonce freshness or password correctness. Every encrypted frame
uses **exact outer-header bytes || exact frame-header bytes** (77 bytes total) as
AEAD associated data, including the public KDF settings, length and final flag.

## Frames

Each frame is a **5-byte header followed by exactly its declared ciphertext**:

| Offset | Bytes | Field/value |
| --- | --- | --- |
| 0 | 1 | Kind: 0 = Next; 1 = Last; all others rejected |
| 1 | 4 | Ciphertext length, including the 16-byte tag |

Next always contains 65,536 plaintext bytes (65,552 ciphertext bytes). Last
contains 0–65,535 plaintext bytes (16–65,551 ciphertext bytes). Emit full chunks
as Next, then exactly one short Last. An exact chunk multiple ends with an empty,
authenticated Last; no EOF-as-success or optional terminator. No bytes may follow
Last. A reader bounds the frame length before allocating/reading its payload.
Use `read_exact` semantics; short source reads need not mean EOF.

Only library STREAM state advances the counter; failed authentication terminates
the attempt. Its maximum counter is reserved for Last, allowing at most
`u32::MAX` Next frames plus Last. The maximum total plaintext is `2^48 - 1`
bytes, including the manifest; enforce counters/checked u64 totals without
allocation based on total size. Actual file/disk limits may be lower and fail
without replacing the installation. Framing intentionally has no paths, ZIP
entries, compression, decompression expansion or arbitrary-size metadata.

## Encrypted payload

The concatenated authenticated plaintext is a **32-byte manifest followed by
exactly one SQLite database**. No padding or trailing payload is accepted.
The manifest is encrypted inside the stream, not a public schema-version hint.

| Offset | Bytes | Field/value |
| --- | --- | --- |
| 0 | 8 | ASCII `YOMOKUDB` |
| 8 | 2 | Payload version = 1 |
| 10 | 2 | Manifest length = 32 |
| 12 | 4 | SQLite migration version (positive, currently 1) |
| 16 | 8 | Exact SQLite byte length |
| 24 | 8 | Reserved = all zero |

Database byte length must be at least 512, a multiple of 512, and fit with the
manifest within the stream's maximum plaintext. These are preliminary format
checks; they do not establish valid SQLite, schema support or valid user data.
No unauthenticated length may cause a whole-database allocation or preallocation.
The exported database includes its own migration journal; it must agree with the
authenticated manifest before migration. Envelope, payload and database schema
versions are independent. Retain support for published version-1 archives when
future writers change; unknown envelope/payload versions and newer database
schemas are explicit errors. The first release supports only its exact initial schema/journal. Development
databases/backups are unsupported. Future supported upgrades must migrate only on
a private candidate and revalidate; never merge or reimport content.

## Export and restore ownership

Rust storage owns snapshot/validation I/O; `yomoku-backup` owns bounded crypto
Read/Write. Runtime sequences cancellation/work; pure core correlates intent and
confirmation. Kotlin owns password UI/descriptors/lifecycle. Raw passwords must
not enter serializable events/views/journals/logs/saved state. A narrow secret
handoff remains to be implemented; bulk database bytes do not belong in FFI state.

Export encrypts only a completed validated private snapshot. Publication to an
external descriptor must complete without exposing plaintext. Retry generates a
fresh salt/nonce. Failed/cancelled partial exports cannot be advertised as success.

Restore decrypts only into an owned private candidate. Every frame, final tag,
EOF and exact manifest length must authenticate before SQLite inspection. Wrong
password/tag failures share an authentication error. Failed candidates are removed;
staging must be excluded from OS backup and cleaned after process death.

## Implemented candidate preflight

`BackupCandidate::check_schema` rejects unsupported versions, mismatched lengths,
invalid SQLite/WAL/page headers and trailing bytes. Before introspection a hardened
no-CREATE/no-URI connection enables defensive/query-only mode, disables trusted
schema/triggers/views/attachment writes and reduces parser/VM limits. Candidate SQL
is never executed as a statement. Every schema object must exactly match a trusted
in-memory initial-schema reference, including internal indexes and ID sequences.
Only then are the exact journal, integrity/CHECK/index/FK and application checks run.
The original source and live database remain untouched.

[RESTORE_VALIDATION.md](RESTORE_VALIDATION.md) lists implemented controls, compact
version-2 completion checks, content/analysis/identity/annotation domains and
remaining EPUB/history checks. `SchemaCheckedBackup` is opaque and grants no
confirmation/activation authority. No repair, import, analyzer rerun, provider
query or reservation replay occurs. Cancellation polls between records and
synchronous calls, not inside SQL/JSON/decoding/pagination/sync; there is no total
resource budget or real-device resource claim.

See [SQLite untrusted-input guidance](https://www.sqlite.org/security.html),
[defensive configuration](https://www.sqlite.org/c3ref/c_dbconfig_defensive.html) and
[schema semantics](https://www.sqlite.org/schematab.html) for the underlying contract.

## Replacement remains unfinished

Only complete semantic validation may lead to a fresh session-local confirmation.
Require explicit complete-replacement confirmation; never merge. Drain pending reader mutations
and workers, gate new I/O, retire provider/navigation work and old callbacks/handles/
caches, then atomically activate a synced candidate on the same filesystem with
correct sidecar handling. Interrupted activation must leave usable old or validated
new state, never a mixture. Do not delete the live database to make room. Incompatible
live databases produce an error and are never automatically cleared.

Required remaining verification includes hostile application data, target preservation
on rejected/cancelled stages, secret transport and Android descriptor/lifecycle cases,
confirmation correlation, crash cleanup, activation fault injection, process death,
and phone memory/KDF latency. Primitive host tests already cover authentication,
metadata/length bounds, stream tamper/truncation/reordering, I/O failures/cancellation,
compact snapshot round trips and candidate validation. Historical measurements and
independent [reference vectors](../tools/backup-reference-vectors.py) are preserved;
they do not establish completed app backup/restore.
