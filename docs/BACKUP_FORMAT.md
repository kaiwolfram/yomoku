# Manual backup format, version 1

Status: strict metadata codecs and bounded authenticated snapshot Read/Write in
`yomoku-backup`, compact app-owned SQLite snapshots in `yomoku-storage` and complete
encrypted fixture round trips implemented on 2026-11-01. Private candidate
structural/schema validation and supported migration are implemented; application
validation, crash cleanup/orchestration, export/import UI and replacement remain. The wire
fixtures define the first format; this is not yet an exported-file compatibility
or end-to-end security claim.

Requirements: [PRODUCT_SPEC.md §7](PRODUCT_SPEC.md#7-secure-manual-backup-and-restore).
The format is independent of the internal Crux/Bincode/FFI representation.

## Complete local state

The encrypted payload contains one compact, consistent SQLite snapshot. Current
schema 0024 stores all durable user state in that database:

- Library titles/order, original import bytes and duplicate identities, canonical
  text, EPUB assets/covers/ruby/formatting and subtitle cues;
- immutable analysis snapshots/identities, global/local boundary overrides,
  vocabulary and status/encounter/completion history, retained source provenance,
  exact positions, permanent finish markers and pending reading durations;
- reader settings, custom dictionary records and configured Anki deck;
- both Anki recovery journals, terminal outcomes and all ID high-water marks,
  including `sqlite_sequence`, migration history and Library recency clock.

Copying these tables as a coherent database avoids rebuilding history, rescanning
books, changing IDs or recreating deleted vocabulary. Bundled Sudachi/JMdict/
JMnedict/font/license assets ship with the APK; they are not user state. Preserve
their referenced immutable analysis identities. Never silently reanalyze a backup
whose dictionary identity differs from the installed bundle; retain the existing
explicit missing/incompatible-analysis behavior.

Transient selections/routes, pending callbacks, invocation permits, UI caches,
provider discovery, Android permissions, external Anki collection contents,
external file-picker grants, signing keys and build artifacts are excluded. The
first MVP has no Nostr identity or relay data. A backed-up deck ID is a preference,
not proof that the destination device has that deck or the same collection.

`Storage::create_backup_snapshot` uses bound `VACUUM main INTO ?1`. Future runtime
integration must call it on the storage FIFO after reader checkpoint drain and
while new mutating work is gated. The private output
is a compact copy; the source is untouched. This choice also prevents deleted
records lingering in free pages from leaking into a backup. SQLite documents
[VACUUM INTO's consistency, deleted-content removal and interruption limits](https://www.sqlite.org/lang_vacuum.html).
Do not copy a live `.sqlite3`, WAL or journal with ordinary filesystem reads.
Only a successfully completed, validated snapshot can be encrypted for export.
Tests preserve IDs/sequences, source deletion semantics and pending durations.
No changes to the live database's vacuum/journal policy.

The caller supplies a new child of an app-private parent excluded from OS backup.
Snapshot creation refuses existing paths/symlinks, creates a 0700 directory and
0600 file, checks the completed copy's integrity/foreign keys/exact current migration
journal/page size/zero freelist, syncs the file and exposes only an owned read-only
reader plus schema/length. No migrations run. SQL validation opens only the owned
copy without CREATE and enables `query_only` before schema parsing. The bundled
SQLite 3.53.2 source (`sqlite3AddCheckConstraint`) omits CHECK expressions for
read-only B-trees; a read-only handle's integrity check alone missed the corrupt
settings fixture. The query-only check catches it. This is a structural export
check, not hardened untrusted-schema/application validation or restore authority.
[SQLite integrity checks](https://www.sqlite.org/pragma.html#pragma_integrity_check)
and [query_only](https://www.sqlite.org/pragma.html#pragma_query_only) govern these APIs.

Snapshot `discard` closes the reader and reports removal failures. Drop/error/
cancellation paths attempt cleanup without recursively deleting unexpected files.
Process-death cleanup and private-parent lifecycle remain runtime obligations.
Cancellation is polled between synchronous calls, including before/after VACUUM,
validation and sync; these calls cannot be interrupted. The reader can rewind for
fresh encryption attempts and remains frozen after live writes. Normal/WAL source
byte-preservation and excluded concurrent uncommitted work are tested. Encryption
is composed in storage tests through a dev dependency only; no production crypto
edge, worker, core/FFI/Android action or external file publication exists yet.

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
| 12 | 4 | SQLite migration version (positive, currently 24) |
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
schemas are explicit errors. Older databases migrate only on a private candidate
using the supported numbered migration chain; never merge or reimport content.

## Export and restore boundaries for subsequent slices

Rust storage owns SQLite validation/snapshot I/O. `yomoku-backup` owns Rust
encryption and bounded `Read`/`Write` processing. The runtime EffectRouter owns
sequencing/cancellation; pure core owns correlated intent/confirmation phases.
Kotlin owns password UI, file picker/descriptor lifecycle and Activity handling.
Do not put SQL/platform I/O in core or bulk database bytes in Crux view/FFI state.
No raw passwords enter serializable events/views, journals, diagnostics or saved
instance state. A narrow nonserializing secret handoff needs design before FFI.

The implemented stream API consumes an owned non-Debug/non-Clone/nonserialized
`Password`, clearing it after KDF. The KDF key and 64-MiB scratch and one reusable
65,552-byte chunk buffer are guarded/cleared; AEAD key clearing is enabled. No
allocation depends on declared database length. Header profile parsing precedes
KDF allocation, and frame parsing bounds ciphertext before reading. Read/Write
loops handle short calls/EINTR and cancellation at every call, plus before/after
KDF and final flush. Cancellation cannot interrupt an in-progress KDF, blocking
I/O or flush. The crate does not open files or choose paths/permissions.

`encrypt_snapshot` accepts a coherent, validated snapshot plus its manifest and
rejects short/extra database bytes. `decrypt_snapshot` strips the authenticated
manifest and writes database bytes only to caller-owned new private staging. The
manifest returns only after Last/tag/EOF/exact-length/output-flush success. Wrong
password/failed tags share one authentication error; diagnostics retain only I/O
error kinds, never arbitrary source messages. Errors/cancellation may leave partial
staging; disposal/crash cleanup and success sync/close remain caller obligations.

Export first stages in app-private storage, encrypts completely and closes the
output before reporting success. An incomplete external destination must never
be reported as a backup; clean up best effort and preserve the live DB on all
cancel/failure paths. No background/cloud/automatic export is implied.

Restore decrypts only to a new private candidate. No activation or confirmation
that could activate occurs until **every frame**, final tag, EOF and exact payload
length verify. Wrong password and failed tags give the same authentication error.
Delete failed/cancelled candidates, including after process death. Private staging
files must not become a plaintext external backup or enter OS automatic backup.

Then validate SQLite structure, migration journal/schema, foreign keys, permitted
schema objects and application invariants against the candidate without trusting
its SQL. Harden the candidate connection before introspection; never pass an
unvalidated candidate to the ordinary `Storage::open` migration path. Migrate a
supported older candidate, validate again, and retain it behind an opaque fresh
session-local confirmation handle. Authentication is not authorization to execute
schema-provided SQL or skip semantic validation. Implementation must enumerate
these checks and test hostile schemas before enabling replacement.

### Implemented structural preflight (not restore authority)

`BackupCandidate::create` owns a new private 0700 directory/0600 write-only file.
Stream decryption can write into it without SQL or adoption of an existing path.
Only after authenticated decryption returns should the caller consume it through
`check_schema` with that manifest's schema and exact length. No bulk bytes or
secrets enter core/FFI. Drop/error/cancel disposes best effort; explicit discard
reports cleanup failures. Only the fixed database/SQLite sidecar names are removed;
unexpected files are preserved. Process-death cleanup remains a runtime obligation.

The check refuses unsupported versions, mismatched lengths, non-SQLite headers,
WAL-format input, invalid page sizes/page counts and trailing bytes. Before schema
introspection the no-CREATE/no-URI READ_WRITE connection sets defensive mode,
trusted_schema off, triggers/views off, attach create/write off, query_only,
cell_size_check and memory temporary storage. Parser/SQL/column/expression/VM/
attachment/trigger/worker limits are reduced without lowering document/blob row
length limits. These measures reduce attack surface; they are not a fixed memory
budget or an in-statement cancellation/deadline guarantee.

Every `main.sqlite_schema` type/name/table/SQL value must exactly match an in-memory
reference built from the bundled migration prefix. This includes internal indexes
and sqlite_sequence; no extra objects (including statistics tables), missing
objects, edited constraints, views, triggers or virtual tables are accepted.
Candidate SQL is never executed as a statement; SQLite necessarily parses schema
definitions during introspection. Row comparisons borrow SQLite values rather
than allocating candidate-sized Rust strings. Only after that match is the exact
contiguous migration journal queried, then integrity/CHECK/index/foreign-key checks
run. All versions 1–24 use the existing numbered chain on the private candidate;
the resulting current schema/journal/integrity/FKs are checked again and the file
is synced/closed. The original source and live database are never opened for writes.
Exact SQL matching deliberately rejects schema variants; keep published archive
fixtures when future SQLite/migration changes require explicit compatibility work.

Before and after migration, partial application checks now require all singleton
rows, consistent Library read ordinals and valid sqlite_sequence contents derived
from the trusted AUTOINCREMENT table list. Every Anki journal payload, including
terminal rows, uses its existing versioned domain decoder; existing-note IDs must
be positive/distinct/nonempty and known/reconciled model IDs must agree. Historical
layouts and pending/uncertain records remain unchanged; exhausted/deleted-ID
high-water marks remain valid. Invalid data returns a fixed diagnostic.
Saved vocabulary rows reuse the repository's typed entry decoder; custom entries
reuse exact Surface and `CustomDictionaryEntry` constructors. Reject blank or
invalid UTF-8 fields without normalization, dictionary lookup or requiring a
custom entry's vocabulary registration. Literal text and deleted registrations
survive unchanged. Status-event chains/final status remain unchecked history work.

Detached version-1 effective records now use private typed wire decoders with
required/nullable fields and rejected unknown/duplicate fields. Validate internal
command/page/position/segment/metadata/occurrence consistency, surviving issued-ID
bounds and any completed receipt/header/surviving encounters. Preserve unfinished
reservations and deleted-content/vocabulary history without replay or current-rule
requirements. Frozen segments now match surviving canonical text, logical pages
derived with their own preset/position and exact original B/alternate A/C headers
and token metadata. Missing historical text/analysis cannot be reconstructed from
frozen segments. See RESTORE_VALIDATION.md.

Canonical source/chapter/text checks now enforce complete zero-based chapter order,
single-text format shape, positive typed identities, UTF-8 titles/live nonblank
text, deleted-source text absence and representable document byte totals. Saved
positions and every pending-time anchor validate document/source ownership and
scalar boundaries/range, including EOF. Preserve empty original titles, legacy
missing positions, exact canonical bytes and zero-time/max-revision tombstones.
These ownership/anchor checks run before/after private migration without format
parsing, writes or repair.
Retained analyses now pass borrowed-row coverage/index/scalar/surface/profile and
metadata domains from the existing analysis contract. Preserve whitespace gaps,
explicit legacy absence and retained classification without analyzer reruns.
Exact paste/file SHA-256 now matches canonical/original bytes respectively, with
required file/encoding/EPUB-header format ownership. Pure TXT/SRT adapters derive
exact canonical text and complete annotations using the saved explicit encoding;
all ruby rows pass existing format/index/scalar/reading domains. Preserve lazy
legacy paste hashes, duplicate pastes, exact originals, renamed titles and deleted
file provenance without repository imports, new IDs, analysis or EPUB extraction.
EPUB metadata/assets/chapter hashes/spans/images, override provenance and other
history relations remain separate unchecked groups.

The opaque `SchemaCheckedBackup` exposes only schema versions and discard. It has
no SQL/path/reader/confirmation/activation API. **Other application invariants
remain unchecked**, including EPUB metadata/assets/format associations, override
provenance, other historical page/anchor relations and history consistency.
[RESTORE_VALIDATION.md](RESTORE_VALIDATION.md)
inventories checked/remaining table groups and limits, including unverifiable
erased allocation history. Do not pass this result to replacement confirmation
or ordinary `Storage::open`. Further validation is required before a restore-ready
handle. Both Anki journals survive unchanged without mutation permits.
Cancellation polls between synchronous operations/rows, 1-MiB hash chunks and
around text decoding/parsing and frozen page derivation, not inside SQL/JSON/
UTF-8 scans/decoding/parsing/pagination/sync. Text derivation owns one file and
annotation vectors; frozen page checks own one chapter and its page ranges.
There is no total resource budget.

API/security evidence: [SQLite untrusted-input guidance](https://www.sqlite.org/security.html),
[connection configuration](https://www.sqlite.org/c3ref/c_dbconfig_defensive.html),
[schema-table semantics](https://www.sqlite.org/schematab.html),
[rusqlite 0.40.2 DbConfig](https://docs.rs/rusqlite/0.40.2/rusqlite/config/enum.DbConfig.html)
and the installed 0.40.2 `limits.rs`/`config.rs` primary sources. Only the existing
pinned rusqlite `limits` feature is enabled. Storage now directly uses the same
already-locked Serde 1.0.228 derive dependency for its private wire records;
[Serde container attributes](https://serde.rs/container-attrs.html) and
[field attributes](https://serde.rs/field-attrs.html) govern rejection/required
nullable-field handling. No dependency version is changed.

**User decision (2026-11-01): require Anki recovery first.** If the current
installation has pending/uncertain note/model writes, restore cannot replace it
until read-only recovery resolves them. Unloaded/failed recovery is not clear.
Recheck the authoritative pair when accepting confirmation and after draining
work; changing state invalidates the confirmation. Do not discard these records
through a generic destructive confirmation. The candidate's own journals remain
intact and restore through normal startup recovery, never as mutation permits.

The user must explicitly confirm complete replacement after validation. Drain
reader checkpoints and all workers; prevent new I/O, cancel provider/navigation
work and retire old native callbacks/handles/caches before activation. Stage on
the same filesystem, sync the candidate and atomically replace the closed live
database, with durable recovery for interrupted activation and correct handling
of sidecars. Failure/crash must leave a usable old or fully validated new state,
never a mixture. The exact activation/recovery protocol is a separate required
implementation step with fault injection before shipping. Never delete/overwrite
the live file to make room for validation and never merge databases. Restart
through normal startup; no active reader/selection/export authority is restored.

## Verification scope

Metadata cases freeze golden header/manifest/frame bytes, exact u64 lengths,
every profile/reserved/version/length rejection, all truncations, every final
segment length, hostile frame sizes and checked stream limits. Stream cases add
[RFC 9106 §5.3 Argon2id](https://www.rfc-editor.org/rfc/rfc9106.html#section-5.3),
[the XChaCha draft vector](https://datatracker.ietf.org/doc/html/draft-irtf-cfrg-xchacha-03#appendix-A.3.1),
production-profile/reference-Argon2 and reference-libsodium BE32 fixtures, actual
password/stream round trips, fresh salt/nonce and both entropy-failure paths.
[backup-reference-vectors.py](../tools/backup-reference-vectors.py) reproduces the
independent fixtures with system libraries; these are optional tooling only, not
application dependencies. This host used libargon2 20190702 and libsodium 1.0.22.

Cases reject every small-archive ciphertext-byte tamper/truncation, changed header/
frame metadata, wrong password, reordered/repeated/spliced/missing/extra frames,
authenticated invalid manifest/length, hostile lengths before payload reads,
short/interrupted/failed reads/writes/EOF probes/flushes, zero writes and cancellation
at every stream poll (including after flush/repeated EINTR). Earlier authenticated
chunks may exist in disposable staging after a late failure. No SQL is validated.

Before encrypted export is complete: full-state compact SQLite snapshot round
trips including deleted records and ID high-water marks, private staging lifecycle,
secret FFI handoff, Android UI/descriptors and real-phone resource trials. Before
restore is complete: unknown/newer/older database schemas,
malformed SQLite/application data, preserved target on every rejected stage,
confirmation correlation/Anki gates, migration and activation failure injection,
process-death recovery, Android descriptor/lifecycle and real phone resource trials.
