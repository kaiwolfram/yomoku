# Data model

## Implemented domain values
- `Surface`: nonblank exact written Unicode string; equality/hash/order use exact
  contents. Different readings share one identity; inflected spellings differ.
  This is not a tokenizer or Japanese detector.
- `VocabularyStatus`: New (default), Learning, Familiar, Known, Ignored.
- `Registration`: CreateNew for absent entry, AlreadyRegistered preserving status
  otherwise. New registration never implies a completed-reading encounter.
- Completion status policy: only New changes to Known.

Vocabulary has no application/FFI command yet. Persisted status strings are
defined explicitly below, independently of Rust enum ordinals.

## Global vocabulary and status history (migration 0006)
`VocabularyId` is a distinct positive i64 newtype. `VocabularyEntry` carries ID,
exact Surface, status and registration Unix milliseconds. SQLite allocates entry
and event IDs with AUTOINCREMENT; later registration after deletion gets a fresh
entry ID. No reading or dictionary form participates in surface identity.

`vocabulary.surface` has BINARY uniqueness, without normalization or trimming;
inflections and composed/decomposed spellings remain distinct, while the same
written spelling across documents/readings shares an entry. Persist statuses as
`new`, `learning`, `familiar`, `known`, `ignored`, with constrained nonnegative
registration timestamps. Analyzer dictionary-form links remain future work.

`vocabulary_status_events` stores an allocated ID, semantics version 1, vocabulary
ID, nullable old status, new status, cause and nonnegative Unix milliseconds.
Registration has NULL→New and cause `registration`; actual manual changes have
different non-NULL old/new statuses and cause `manual`. Date/ID and entry/ID indexes
support future aggregation. Events are immutable through repository APIs; there
is no history-edit API. Their vocabulary FK uses ON DELETE CASCADE as required
for future confirmed vocabulary deletion. No document FK exists, so document
deletion preserves global entries and history. No production delete API exists.

`register_vocabulary` commits a supplied batch and its registration events together;
duplicate surfaces preserve their original date/status and emit no extra events.
`vocabulary_entry` is read-only. `set_vocabulary_status` commits current state and
dated old/new event together, returns None for absent IDs and emits nothing on an
unchanged selection. Transactions serialize concurrent writes; failure preserves
prior entries/history. Invalid timestamps/stored values error without repair.

These APIs are repository prerequisites, not analyzer/import or reading-completion
integration. Document creation and registration must share a future import
transaction. Completion-driven New→Known events need completion provenance and
must share the occurrences/time/position transaction, not call manual selection.
Existing document imports are not rescanned by this migration or by lookup.

## Analysis anchors and reversible boundaries (migration 0007)
`AnalysisId` and `OverrideId` are distinct positive i64 identities, allocated by
AUTOINCREMENT. No analysis token refers to a vocabulary ID: vocabulary deletion
must not erase source analysis or cause lookup to recreate a deleted entry.

`ChapterAnalysis` carries exact engine/dictionary revision identifiers, A/B/C mode
(default B), and ordered `AnalyzedToken`s. Each token has a source/chapter UTF-8
anchor, exact original surface, analyzer-supplied Japanese classification and optional
reading/dictionary form. Dictionary forms never replace the surface identity;
unresolved forms stay absent. Validation rejects empty/overlapping/out-of-order
spans, wrong ownership, split scalars, normalized/substituted surfaces, blank
metadata and omitted non-whitespace text. Whitespace gaps and empty analyses of
empty/whitespace text are allowed. The pure core module does not tokenize or
classify text. The separate `yomoku-analysis` adapter supplies real Sudachi output. Its profile pins engine
commit + adapter-v1/embedded-defaults and Full 20261015 + dictionary SHA-256.
Classification uses original Japanese script and analyzer POS, including OOV rare
kanji. OOV readings/dictionary forms are absent; original surfaces never become
normalized identity. Normalization-generated empty original spans are omitted,
and the complete result must still pass canonical coverage/nonoverlap validation.
Real A/B/C output survives repository saves and reopen without a schema change.
Import/registration composition and production analysis persistence remain pending.

`chapter_analyses` stores semantics version 1, chapter ownership, engine/dictionary
revision and explicit `a`/`b`/`c` mode. Its unique chapter/revisions/mode key identifies
one immutable output: identical retries return its ID; changed output under the
same key errors. The revision identity must include the adapter/configuration and
exact dictionary artifact when the real analyzer is integrated. `analyzed_tokens`
stores contiguous token indices, byte spans, exact surfaces, Japanese flags and
nullable readings/dictionary forms. Load revalidates indices and canonical anchors.
Analysis references live chapter text with cascading deletion of snapshots/tokens;
historical completion occurrences must retain their own anchors and vocabulary IDs.

`SegmentationOverride` stores a selected exact Surface/TextAnchor and a nonempty
ordered partition into exact Surface parts. One part is a merge; multiple parts
are a split. Concatenation must equal the selected original bytes. The pure helper
projects checked replacement anchors for the selected occurrence. No user-supplied
reading/lemma is accepted; manual edits do not alter analyzer metadata or history.

`segmentation_overrides` and ordered `segmentation_override_parts` store semantics
version 1, retained source/chapter provenance, the original span/surface, scope and
active flag. Scope defaults to AllOccurrences (`all`), keyed by exact BINARY surface;
ThisOccurrence (`this`) is keyed by source/chapter/start/end. Partial unique indexes
permit one active edit per exact key. Saving the same partition is a no-op (a global
rule retains its original provenance even when saved from another source). A changed
partition atomically deactivates the prior edit and inserts a fresh ID/parts.
Reversal disables only the supplied active ID, never resurrecting a superseded edit.
Missing/already inactive IDs are no-ops. Statements and commit failure roll back
replacement/reversal together. No override API edits text, statuses or past events.

Original selected bytes and source/chapter metadata survive content deletion, so
global edits remain inspectable/reversible. Load validates their shape/ownership
and, while canonical text exists, their exact source span. No production deletion
API is introduced. Existing documents are not analyzed/rescanned by migration.
These methods are repository prerequisites only: real analyzer output, atomic
import registration, matching rules across occurrences, overlapping rule precedence,
and segmentation UI/application remain unimplemented. No override is applied to
the current reader or treated as a completed-reading occurrence.

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
are not a page-length metric. Current paste progress uses the byte-offset ratio
described below.
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

## Derived logical pages and reading position
`pagination::PageLength` has only 250/500/750/1000/1500, default 500. The user
clarified that all non-whitespace Unicode scalars count, including punctuation
and Latin text; UTF-8 bytes and UTF-16 units do not. No normalization is applied.
LogicalPages borrows ordered canonical chapters and derives TextAnchor ranges
that cover every byte exactly once. No range crosses a supplied chapter; headings
never infer one. Chapter IDs must be unique and belong to one source. Empty or
whitespace-only chapters have one page; an empty chapter list has none.

ReadingPosition contains source ID, chapter ID and exact byte offset, separately
from a page number or page start. Reflow returns the unchanged position and its
new page index. A position at an internal page boundary selects the next page;
chapter EOF selects that chapter's final page. Unknown ownership, out-of-range
offsets and split scalars are rejected. ReadingPosition has generated Serde/Facet
codecs and is persisted by migration 0003 below. Vertical scroll, encounters and
settings are not stored here; reader reflow uses migration 0004 below.
Paragraph/sentence heuristics are recorded in DECISIONS.md.

ReaderPage is a transient projection of the logical page containing the saved
ReadingPosition, at the persisted global preset (default 500): document ID/title, exact page text,
TextAnchor, unchanged ReadingPosition, and one-based page number/total. Unopened
documents start at byte 0. No page number or vertical scroll is stored. Kotlin
renders this projection without calculating boundaries or slicing Unicode text.

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
document ID, live title, one `ChapterText` and optional `reading_position`;
mutating a snapshot does not edit
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
vocabulary registration and import timestamps remain separate unchecked tasks. No reading encounters are created here.

## Current position repository (migration 0003)
`reading_positions` has document ID as PK/FK (cascade only this document's progress
on deletion), chapter ID FK and a nonnegative UTF-8 byte offset. No row means
unopened, projected as 0%. Source ownership derives from the live document.
`save_reading_position` validates supplied source/chapter ownership, scalar
boundaries and EOF inside an immediate transaction before upsert. It returns the
new snapshot only after commit. Invalid input and insert/update/commit failure
preserve the previous row; missing documents return None. Load validates stored
positions without repair. No completion/history/time/status mutation occurs.

LibraryDocument adds whole `progress_percent`: floor(offset * 100 / total canonical
UTF-8 bytes), checked for bounds with overflow-safe u128 arithmetic. Listing uses
SQLite BLOB lengths (including text after NUL), without fetching every document's
text or paginating the Library. It checks chapter ownership and offset bounds;
full scalar-boundary validation happens when loading/saving a document. Current
progress can decrease and is independent of the future finished checkmark.
Only paste's single text unit is implemented; multi-chapter totals await imports.

SaveReadingPosition is a typed position-only protocol hook for an active reader;
there is no user-facing seek control. It uses the existing busy guard/FIFO worker.
The displayed page and Library summary update after commit; failures keep the
previous snapshot. Leaving during a save does not cancel the write or reopen the
reader when its callback arrives. Next/Previous/swipes and atomic forward
completion must be wired in their own implementation step.

## Reader settings repository (migration 0004)
`reader_settings` is a STRICT singleton table (`id = 1`) with `page_length`
constrained to 250/500/750/1000/1500. Migration seeds 500 and leaves canonical
text/current positions untouched. Store the character count, never a Rust enum
ordinal. PageLength has same-build Serde/Facet codecs for the typed FFI.

`page_length` validates the stored value; `save_page_length` reads/validates the
existing row and updates inside an immediate transaction, returning after commit.
Missing/invalid rows return InvalidSettings without repair; write/commit failures
leave the prior setting intact. Runtime maps this to a typed error. No page number,
range or document-specific setting is persisted.

Library and document loads include the stored preset. Core exposes an optional
page_length until a successful load, so Settings cannot submit an assumed default
after a failed startup. SetPageLength requires loaded settings and the Settings
route; unchanged selections and submissions while busy are ignored. The selected
preset and reader stay at their committed values until success. Core retains the
active immutable reader snapshot independently of the public LoadPaste snapshot,
then reprojects using the reader's exact ReadingPosition. Reflow never writes
current position, Library progress, encounters or completion. Late settings saves
update the global preset without reopening closed reader/Settings routes.

## Page completion repository foundation (migration 0005)
`CompletionCommandId` is another positive i64 identity. `completion_commands`
issues it with AUTOINCREMENT before a forward intent is submitted. Repeated
callbacks/retries must reuse one ID and its frozen payload (including timestamp);
a deliberate reread uses a fresh ID. Issued but unused IDs are not reading events
and are never automatically replayed. IDs survive reopen and cannot be reused.

Pure `PageCompletion` carries the document, exact completed TextAnchor, page-length
snapshot, exact departing ReadingPosition and nonnegative Unix milliseconds.
`next_position` validates ownership/scalar boundaries, the current exact position
and equality with the derived logical page. For the implemented single-unit paste,
the destination is the page's end (the next page start or final EOF).

`reading_completions` stores one row per command ID: semantics version 1,
historical document ID, retained source/chapter FKs, start/end/departing byte
offsets, page-length character count and completion timestamp. Its date/ID index
supports future history queries. Document ID deliberately has no live-document FK;
test-only deletion of content/documents preserves events and provenance. No
production deletion API or change to deletion semantics is introduced.

`record_page_completion` uses an immediate transaction. An existing ID must match
every payload field and returns AlreadyRecorded **without writing position**,
even after reflow, backward movement or deletion. A conflicting payload errors.
For new events it requires an issued ID, the current preset and a valid departing
page/position, then inserts history and updates current position in one commit.
Statement/commit failure rolls both back; retry reuses the same intent. Position-only
saves never insert history. Concurrent duplicate submissions record once.

This is repository-only infrastructure, with no app event/runtime/FFI/Android
forward action yet. Before exposing it, extend the same transaction with actual
Japanese occurrences, New→Known/status events and accumulated active time. Do not
use absent occurrence/time data as completed-reading statistics. Final EOF alone
does not implement finished-at-least-once, Finish Book or reopen-at-page-1 behavior.

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

ContentRoute (Main/Paste/Reader) overlays top-level navigation; Settings overlays
that route. OpenPaste opens the form; OpenReader loads on the existing Rust worker
and opens only if its requested document is still pending. CloseDocument or a
destination change clears that pending request, so late load results cannot
reopen the reader. LoadPaste remains a snapshot-only repository command.
Create success returns to Library and adds its title without opening a reader;
failure leaves the form/draft for retry. The unsaved text draft is Compose form
state, retained through Settings/recreation and cleared when leaving the form.

RefreshLibrary/CreatePaste/LoadPaste/SaveReadingPosition/SetPageLength use the pure
StorageOperation protocol.
StorageFinished is skipped by Serde and Facet, so it cannot be dispatched over
FFI. Model/ViewModel add LibraryDocument (ID/title/current percentage), an optional loaded snapshot,
storage_busy and typed document_error. Concurrent storage submissions are ignored
while busy; navigation remains available. A successful create adds its summary
without opening a reader; LoadPaste reads exact content without changing progress,
while OpenReader adds the saved-position page projection described above.
Listing is allocation order until the planned Library sorting task. Failed opens
can retry. No path or SQLite diagnostic string is exposed in application state.
The bridge constructor takes the path separately; Android uses filesDir/yomoku.sqlite3.
Submitted writes finish even if the session closes; shutdown does not block UI.

## Required future schema constraints
- Future import repositories must extend the identity/immutable-text and durable
  chapter/byte-offset contract above to multiple chapters.
- Add analyzer dictionary-form links (nullable) to global vocabulary and wire
  registration/status repositories into typed application/import effects.
  Reading never participates in identity.
- Extend migration-0005 completion events with occurrence rows preserving repeated
  tokens and active time in the same transaction; reread uses a new command ID.
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
