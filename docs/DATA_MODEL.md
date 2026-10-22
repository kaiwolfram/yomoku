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

## Document identities and text anchors
`document.rs` defines distinct `DocumentId`, `SourceId` and `ChapterId` newtypes.
Each is an integer in `1..=i64::MAX`; Rust constructors and Serde decoding reject
zero/negative values. Generated Kotlin represents each as a separate class with
a `Long` value. These identities are database-local, never derived from titles,
chapter order, page numbers, file paths or content hashes.

- Document identifies one live Library instance and its mutable progress/title.
  Deletion/reimport must allocate a fresh identity with fresh progress/checkmark.
- Source identifies retained historical provenance for that import. It survives
  document deletion; exact-duplicate detection is a separate future SHA-256 key.
- Chapter identifies one immutable canonical text unit owned by a source. EPUB
  uses its source chapters; paste/TXT/SRT use one text unit without inferred
  chapters. Chapter IDs and source ownership do not change on page reflow.

Migration 0002 allocates IDs with `INTEGER PRIMARY KEY AUTOINCREMENT` and
positive-value checks. Committed IDs are never manually reused; rollback may
reuse unpublished IDs. Tests verify nonreuse across deletion/reopen, independent
full-width allocation and atomic failure on exhaustion. Source/chapter provenance
is separate from live document content. Full replacement backups must preserve
IDs and allocation high-water marks; cross-database merging is not supported.
Production document deletion and restore remain future repository commands.

`ChapterText` carries source/chapter IDs and an exact canonical Unicode string.
`TextOffset` is a nonnegative signed 64-bit **UTF-8 byte offset within a chapter**;
0 and end-of-text are valid. `TextRange` is half-open `[start, end)`. Every use
must check ordered/in-bounds Unicode scalar boundaries against the exact chapter
text. `TextAnchor` includes source/chapter identity and a range; `slice` checks
both ownership and boundaries. It has no dependency on a live document or page.
Numeric decoding alone cannot validate range ordering, chapter ownership or text
boundaries. Generated Kotlin constructors do not enforce Rust validation.

Canonical text is not Unicode-normalized, trimmed or silently line-ending
normalized by these types. Importers define canonical extraction before anchors
are created. Source ruby/formatting/images must later refer to that same text;
image-only positions and import extraction rules remain future work. Offsets
are not a Japanese-character page-length metric or a progress percentage policy.
Combining sequences/ZWJ emoji may contain multiple scalar boundaries; these
helpers do not snap positions to graphemes. `TextOffset::from_utf16`/`to_utf16`
convert Kotlin/Compose indices explicitly and reject split surrogate pairs.

Crux Bincode payloads encode the numeric newtypes as fixed-width signed i64
values and strings as UTF-8. Seven Rust → generated Kotlin → Rust fixture
exchanges verify exact text/ID/range preservation, Kotlin-created suffix anchors,
empty text, supplementary kanji/emoji, combining/ZWJ sequences, CRLF, NUL and
IDs above 2^53 through i64::MAX. These test the generated FFI payload codecs;
live document commands and async JNI storage callbacks now use these types. This same-build
wire encoding is not the future backup format.

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
repairing data. Runtime opens/migrates on its FIFO worker using Android's private
files path. Vocabulary/history tables await their domain commands.

## Paste repository (migration 0002)
`PasteText` validates non-whitespace content without changing the text. Its initial
title is the literal first LF/CRLF-delimited line, without trimming/truncation or
a fallback for an empty first line. `PastedDocument` is a detached snapshot of a
document ID, live title and one `ChapterText`; mutating a snapshot does not edit
persisted content. PastedDocument is now a generated shell/FFI payload.

- `sources`: positive autoincrement ID, `kind = 'paste'`, original title.
- `chapters`: positive autoincrement ID, source FK, nonnegative chapter index,
  unique `(source_id, chapter_index)`. A paste has exactly one text unit at index 0;
  headings and hard breaks never infer chapters.
- `documents`: positive autoincrement ID, unique source FK, live title separate
  from historical original title and canonical text.
- `chapter_texts`: chapter ID PK/FK and exact canonical text. Source/chapter
  metadata can remain when future document deletion removes this text and the
  Library entry. No delete API or automatic cascading deletion is added yet.

`Storage::create_paste` inserts all four rows in one immediate transaction and
returns only after commit. No content-edit API exists. `load_paste` uses one read
transaction for a consistent snapshot, returns None only for an absent document
and reports missing/invalid content for an existing document as InvalidDocument.
SQL values are bound parameters, including titles/text containing NUL or quotes.
Both calls are synchronous I/O executed off the UI thread by runtime effects.
This repository is the persistence part of import; SHA-256 duplicate detection,
vocabulary registration, import timestamps and reader progress
remain separate unchecked tasks. No reading encounters are created here.

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
for the current navigation schema. Text/Unicode payload codecs are verified by
the fixture exchange above and live async JNI create/list/load/reopen tests.
A fresh core starts at Library.

RefreshLibrary/CreatePaste/LoadPaste use the pure StorageOperation protocol.
StorageFinished is skipped by Serde and Facet, so it cannot be dispatched over
FFI. Model/ViewModel add LibraryDocument (ID/title), an optional loaded snapshot,
storage_busy and typed document_error. Concurrent storage submissions are ignored
while busy; navigation remains available. A successful create adds its summary
without opening a reader; loading reads exact content without changing progress.
Listing is allocation order until the planned Library sorting task. Failed opens
can retry. No path or SQLite diagnostic string is exposed in application state.
The bridge constructor takes the path separately; Android uses filesDir/yomoku.sqlite3.
Submitted writes finish even if the session closes; shutdown does not block UI.

## Required future schema constraints
- Repositories must implement the identity/immutable-text contract above and
  persist current chapter ID plus byte offset separately from logical pages.
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
