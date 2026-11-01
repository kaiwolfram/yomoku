# Manual backup format, version 1

Status: design and strict metadata codecs implemented on 2026-11-01 in
`yomoku-backup`. Encryption, snapshot I/O, export/import UI and replacement are
not implemented. The wire fixtures define the first format; this is not yet an
exported-file compatibility or end-to-end security claim.

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

Snapshot creation will use bound `VACUUM main INTO ?1` on the storage FIFO after
reader checkpoint drain and while new mutating work is gated. The private output
is a compact copy; the source is untouched. This choice also prevents deleted
records lingering in free pages from leaking into a backup. SQLite documents
[VACUUM INTO's consistency, deleted-content removal and interruption limits](https://www.sqlite.org/lang_vacuum.html).
Do not copy a live `.sqlite3`, WAL or journal with ordinary filesystem reads.
Only a successfully completed, validated snapshot can be encrypted for export.
Tests must specifically preserve IDs/sequences, source deletion semantics and
pending durations. No changes to the live database's vacuum/journal policy.

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

Reviewed exact candidates for the next implementation step, **not yet added to
Cargo.toml/Cargo.lock or compiled for Android**:

| Library | Intended API/features and evidence |
| --- | --- |
| `argon2 =0.5.3` | Pure Rust, MIT/Apache-2.0. Explicit `Algorithm::Argon2id`, `Version::V0x13`, `Params::new(65536, 3, 4, Some(32))`. Use the binary key derivation API, never PHC password-verification strings. Disable defaults; enable `alloc`/`zeroize`, explicitly clear owned KDF working memory. [Key derivation](https://docs.rs/argon2/0.5.3/argon2/), [parameters](https://docs.rs/argon2/0.5.3/argon2/struct.Params.html), [features](https://docs.rs/crate/argon2/0.5.3/features). |
| `chacha20poly1305 =0.11.0` | Pure Rust, MIT/Apache-2.0; `XChaCha20Poly1305` has a 24-byte full nonce and a 32-byte key. Disable defaults, enable `alloc`/`zeroize`; no reduced-round cipher. The crate reports a historical NCC Group audit; that does not audit this application, STREAM composition or every later release. Its constant-time multiplication requirement fits the supported arm64/x86_64 targets; runtime verification remains. [Security notes](https://docs.rs/crate/chacha20poly1305/0.11.0), [manifest/features](https://docs.rs/crate/chacha20poly1305/0.11.0/source/Cargo.toml). |
| `aead-stream =0.6.0` | MIT/Apache-2.0, pure Rust; same `aead 0.6` dependency family as the selected cipher. `EncryptorBE32`/`DecryptorBE32`, `new`, `*_next_in_place`/`*_last_in_place` with AAD. `last` consumes the state. [API](https://docs.rs/aead-stream/0.6.0/aead_stream/struct.Encryptor.html), [manifest](https://docs.rs/crate/aead-stream/0.6.0/source/Cargo.toml), [counter/nonce source](https://docs.rs/aead-stream/0.6.0/src/aead_stream/lib.rs.html). |
| `getrandom =0.4.3` | Existing locked version; `fill` obtains OS randomness on Linux/Android. Fail closed on entropy errors; no clock/ID/random fallback. Generate independent fresh salt and prefix for every attempt, including Retry. [API/targets](https://docs.rs/getrandom/0.4.3/getrandom/). |
| `zeroize =1.9.0` | Existing locked version; MIT/Apache-2.0. Use `Zeroizing`/explicit clearing for password bytes, key, KDF memory and plaintext buffers. Owned Rust buffers can be cleared; this cannot promise elimination of Android/JVM copies, allocator copies, registers or OS caches. [Guarantees/limits](https://docs.rs/zeroize/1.9.0/zeroize/). |

No new native service, network access or runtime is required. Recheck actual
feature/API compilation and lock the complete graph when adding these dependencies;
include published known-answer vectors and both Android ABIs before integration.
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

Rust storage owns SQLite validation/snapshot I/O. `yomoku-backup` will own Rust
encryption and bounded `Read`/`Write` processing. The runtime EffectRouter owns
sequencing/cancellation; pure core owns correlated intent/confirmation phases.
Kotlin owns password UI, file picker/descriptor lifecycle and Activity handling.
Do not put SQL/platform I/O in core or bulk database bytes in Crux view/FFI state.
No raw passwords enter serializable events/views, journals, diagnostics or saved
instance state. A narrow nonserializing secret handoff needs design before FFI.

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

Current tests freeze golden header/manifest/frame bytes, exact u64 lengths,
every changed profile byte, reserved/version/length rejection, all truncations,
opaque salt/nonce values, every allowed final segment length, hostile frame sizes
and checked stream-size limits. They perform no cryptography, SQL or file I/O.

Before encrypted export is complete: library known-answer vectors, actual KDF/
AEAD/STREAM round trips, wrong-password/header/tag/payload tampering, missing or
extra Last, reordered/repeated/spliced/truncated frames, I/O faults/cancellation,
fresh entropy and full-state snapshot round trips including deleted records and
ID high-water marks. Before restore is complete: unknown/newer/older versions,
malformed SQLite/application data, preserved target on every rejected stage,
confirmation correlation/Anki gates, migration and activation failure injection,
process-death recovery, Android descriptor/lifecycle and real phone resource trials.
