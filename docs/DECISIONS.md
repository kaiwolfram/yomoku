# Architectural decisions

These are the active technical choices. [PRODUCT_SPEC.md](PRODUCT_SPEC.md) defines
product behavior; [ARCHITECTURE.md](ARCHITECTURE.md), [DATA_MODEL.md](DATA_MODEL.md)
and [DEVELOPMENT.md](DEVELOPMENT.md) describe current contracts and commands.
Performance reports and CSV files retain historical measurements independently.

## Ownership and bridge

Use Kotlin/Compose for Android and a pure Rust Crux core. SQLite, format I/O,
dictionary loading and analysis run through Rust's EffectRouter and one FIFO
worker. Kotlin owns lifecycle, descriptors, permissions, browser intents and the
AnkiDroid ContentResolver adapter. Core contains no SQL or platform I/O.

Crux 0.20.0 supplies typed events, commands, rendering and the Bincode middleware
bridge. Facet generates the Kotlin payload codecs; BoltFFI 0.30.1 owns JNI/platform
entry points. Generated artifacts live under ignored `target/`. The shell receives
compact pages and revision identities; full chapter/provenance values stay native.
Retained Android sessions reject stale callbacks and close native handles explicitly.

Supporting contracts: [Crux middleware](https://docs.rs/crux_core/0.20.0/crux_core/middleware/struct.Bridge.html),
[Bincode bridge](https://docs.rs/crux_core/0.20.0/crux_core/bridge/struct.BincodeFfiFormat.html),
[Facet generation](https://docs.rs/crux_core/0.20.0/crux_core/type_generation/facet/index.html),
[BoltFFI](https://docs.rs/crate/boltffi/0.30.1),
[JNI](https://docs.oracle.com/en/java/javase/17/docs/specs/jni/design.html) and
[Android ViewModel](https://developer.android.com/topic/libraries/architecture/viewmodel).

## Reader, imports and dictionaries

One production session reads immutable canonical text with stored Sudachi Mode C
segmentation. A/B analyses supply dictionary components. Reader entry, snapshots,
position saves and effective page/word lookup share this path. The reader renders
base text only. Page fragments retain complete
word identities. UTF-8 byte anchors and scalar boundaries are canonical; Kotlin
converts validated visible ranges to UTF-16 only at rendering boundaries.

Imports preserve original bytes, encoding and hashes, canonical base text
and EPUB image/format metadata. Recognized source readings are discarded; dictionary
readings remain available in word sheets. Exact duplicates retain their live document.
EPUB imports tolerate repeated spine references by keeping each manifest item's
first occurrence. This accepts books with duplicated cover entries without
duplicating canonical chapters, analysis or reading content; original bytes and
file hashes remain unchanged. Ambiguous manifest entries remain invalid.
SRT cue boundaries are transient and preserve canonical blank-line separators.
Internal and unsupported EPUB links retain text without annotation records;
HTTP(S) links retain browser targets. Imported content is not editable.

Sudachi and Full inputs remain pinned and checksum verified. Mapped dictionary
bytes must remain immutable while readers exist. Full loads lazily on the worker
and releases after queued background work. Bundled English JMdict/JMnedict remain
separate read-only assets. Lookup does not register vocabulary or manufacture
matches. No custom dictionary or boundary editor is included.

Supporting evidence: [pinned Sudachi source](https://github.com/WorksApplications/sudachi.rs/blob/5de0410b6edfd428fc510cc9763440772e9619d5/sudachi/src/analysis/morpheme.rs),
[Full release](https://github.com/WorksApplications/SudachiDict/releases/tag/v20261015),
[mapped-file lifetime](https://docs.rs/memmap2/0.9.12/memmap2/struct.Mmap.html),
[str boundaries](https://doc.rust-lang.org/std/primitive.str.html#method.is_char_boundary),
[UTF-16](https://doc.rust-lang.org/std/primitive.str.html#method.encode_utf16),
[strict decoding](https://docs.rs/encoding_rs/0.8.44/encoding_rs/struct.Encoding.html),
[ZIP metadata](https://docs.rs/zip/9.5.1/zip/read/struct.ZipFile.html) and
[XML parsing](https://docs.rs/roxmltree/0.21.2/roxmltree/struct.ParsingOptions.html).
Pinned inputs, patches, licenses and notices are in
[Sudachi notes](../third_party/sudachi/README.md) and
[dictionary notes](../third_party/dictionaries/README.md).

## Persistence and history

Use one initial SQL schema, version 1, journal `0001_base_text_initial_schema`,
with 26 application tables. Retain the immediate-transaction runner/journal for
future published upgrades. Previous development databases and backups, including
the prior schema-1 journal, fail validation without migration, clearing or repair.
Keep SQLite I/O in `yomoku-storage`, with foreign keys on each connection and
transactional rollback on statement or deferred-commit failures.

Vocabulary identity preserves exact surface spelling without normalization.
Completion reserves one frozen effective command and records occurrences,
New-to-Known transitions, history, next position and permanent finish state
atomically. Exact retries retain the reservation and remain safe after deletion;
deliberate rereads reserve fresh IDs and count again. Previous/reflow save position
without completion. Reading durations are not collected or persisted.

Document deletion is individual and confirmed. It removes live content/progress
while retaining source/chapter provenance, global vocabulary and history.
Vocabulary rows open individual word sheets with direct status persistence and
retained retry commands. No Vocabulary selection or bulk commands cross the core/runtime boundary.
Historical vocabulary deletion helpers erase events/occurrences without rescanning books. IDs retain
AUTOINCREMENT high-water marks so reimported content has fresh identity.

Supporting contracts: [rusqlite connection](https://docs.rs/rusqlite/0.42.0/rusqlite/struct.Connection.html),
[transactions](https://docs.rs/rusqlite/0.42.0/rusqlite/struct.Transaction.html),
[SQLite immediate locking](https://www.sqlite.org/lang_transaction.html) and
[AUTOINCREMENT](https://www.sqlite.org/autoinc.html).

## AnkiDroid

Persist only the optional default deck. Prepared fields, duplicate observations,
setup state and invocation ownership remain in memory. Fresh provider queries run
on lookup, return and before insertion; failed queries never imply absence.
Every write revalidates permission, deck and model, and typed invocation ownership
rejects duplicate dispatch and stale callbacks. Partial/incompatible models are
never overwritten. Existing-note navigation uses fresh verified note IDs.

An interrupted external write can survive. An explicit retry starts with fresh
queries; there is no durable export/recovery journal or cross-process exactly-once
promise. Startup and restore never replay external writes. This tradeoff keeps
backup/restore independent of Anki recovery. The
[provider contract](ANKIDROID_API.md) retains the pinned primary source links and
verification limits.

## Backup security

Keep bounded authenticated stream cryptography separate from SQLite snapshots and
app activation. Preserve the version-1 envelope: explicit Argon2id profile,
XChaCha20Poly1305 STREAM BE32 framing, fresh salt/nonce and authenticated finality.
Passwords are literal UTF-8 and must not enter serializable state or logs. Owned
Rust secret/plaintext buffers are zeroized; JVM/OS copies are outside that guarantee.

Create compact snapshots with bound `VACUUM INTO` into new private staging.
Authenticate/decrypt completely before inspecting a candidate; compare exact
trusted schema/journal, then integrity, foreign keys and semantic domains. Do not
execute candidate SQL, repair data or replay reservations. Opaque validation values
grant no replacement authority. Secret handoff, confirmation, crash cleanup and
atomic activation remain unfinished release work. The crypto profile, exact
reviewed dependency APIs and supporting RFC/SQLite/security sources remain in
[BACKUP_FORMAT.md](BACKUP_FORMAT.md); validation limits are in
[RESTORE_VALIDATION.md](RESTORE_VALIDATION.md).

## Dependencies and builds

Exact Rust dependencies and transitive graph are pinned in manifests/Cargo.lock;
Android versions are pinned in the Gradle catalog. Verify primary APIs before
adopting changes. Parsers use strict decoding and bounded ZIP/XML handling;
prepared dictionary assets verify pinned bytes rather than silently updating.
Normal Fedora-compatible development uses CLI tools and does not need Android Studio.

Use JDK 17, NDK 30.0.16248370 and Android min SDK 29. Package arm64-v8a and x86_64.
Gradle selects matching debug/release Rust profiles, separates generated outputs
and serializes native generation with a cross-process lock. Host JNI generation
and standalone Cargo checks must also be serialized because metadata expansion
shares host artifacts. Release excludes debug probes and remains unsigned until
publishing is approved. Verify 16-KB ELF/ZIP alignment and actual target runtime
separately from successful compilation.

Supporting contracts: [Rust Android targets](https://doc.rust-lang.org/rustc/platform-support/android.html),
[NDK toolchain layout](https://developer.android.com/ndk/guides/other_build_systems),
[page-size alignment](https://developer.android.com/guide/practices/page-sizes),
[AGP built-in Kotlin](https://developer.android.com/build/migrate-to-built-in-kotlin) and
[emulator CLI](https://developer.android.com/studio/run/emulator-commandline).

## Joined conjugations for new imports

The version-2 analysis adapter joins adjacent Mode C verb/adjective conjugations
and allowlisted non-independent `て`/`で` helper chains using the pinned Sudachi
POS and conjugation metadata. Exact surfaces/spans remain the vocabulary and
completion identity; the head supplies the base form. A/B remain raw for lookup.
No stored C snapshot or history is rewritten. Version-1 lazy A/B analysis stays
supported because its engine, dictionary, configuration and raw analysis semantics
are unchanged; unsupported profiles fail closed.
