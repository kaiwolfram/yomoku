# Data model

## Implemented domain values
- `Surface`: nonblank exact written Unicode string; equality/hash/order use exact
  contents. Different readings share one identity; inflected spellings differ.
  This is not a tokenizer or Japanese detector.
- `VocabularyStatus`: New (default), Learning, Familiar, Known, Ignored.
- `Registration`: CreateNew for absent entry, AlreadyRegistered preserving status
  otherwise. New registration never implies a completed-reading encounter.
- Completion status policy: only New changes to Known.

These enums have no persisted/FFI encoding contract.

## SQLite initialization
`yomoku-storage` owns the database connection. Migration
`0001_migration_history.sql` creates `schema_migrations(version INTEGER PRIMARY
KEY CHECK (version > 0), name TEXT NOT NULL UNIQUE)`. This journal is the sole
schema-version authority; SQLite `user_version` is unused. Applied rows must match
the ordered prefix of compiled migrations by number and name. Migration SQL and
its journal insertion commit together, with all pending migrations in one
transaction. Published migrations must remain immutable; append numbered files.

Opening creates an empty database in an existing caller-selected directory,
enables foreign keys per connection and waits up to five seconds for lock
contention. Newer versions, inconsistent history, nonempty databases without a
journal, invalid files and SQLite I/O failures return errors without resetting or
repairing data. The initial journal has no document/vocabulary/history tables;
their IDs, offsets and event schemas remain to be defined with domain commands.
Storage is not yet connected to runtime/FFI or Android's private files path.

## Transient application state
`app::Destination` is Library (default), Nostr, Statistics or Vocabulary.
`Model` owns the selected destination; `ViewModel` projects it for the shell.
`Event::SelectDestination` selects a destination and requests a render. Event,
destination and view derive Serde serialization. The host bridge now uses Crux
0.20.0 BincodeFfiFormat (fixed integers; upstream permits trailing bytes), with
`Requests<ShellEffectFfi>` for effect batches and Crux EffectId (u32) for responses.
Render is fire-and-forget: resolving its ID returns InvalidResponse. Malformed
or unknown event variants return InvalidEvent before mutation. These are internal
same-build schemas, not a persisted format or version-stable public protocol.
The optional Facet metadata now generates Kotlin payloads and Bincode codecs.
Kotlin event/request/view serialization compatibility is verified across host JNI
for the current navigation schema. This does not yet verify future text/Unicode
anchors or asynchronous effect responses. A fresh core starts at Library.

## Required future schema constraints
- Stable document/source/chapter IDs; immutable content with explicit offset
  encoding tested across Rust/Kotlin. Encoding/anchor design is still open.
- Unique active surface entry, linked analyzer dictionary form (nullable), dated
  registration and immutable status history. Reading never participates in identity.
- Completed-reading events with unique command IDs, occurrence rows preserving
  repeated tokens, provenance and active time; reread uses a new command identity.
- Pending page time and current text offset separate from completed events;
  permanent finished-at-least-once separate from current percentage.
- Document deletion removes content/progress but preserves historical provenance,
  encounters and global vocabulary. Reimport creates fresh document progress.
- Vocabulary deletion removes its encounter/status history, recalculates statistics
  and does not trigger rescans. Future registration starts New.
- Complete page events, counters and New→Known changes committed in one transaction
  using the numbered SQLite migration foundation above.

Refine schema with executable migration/transaction tests before claiming storage
or accurate historical statistics are complete.
