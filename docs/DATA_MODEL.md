# Data model

## EPUB canonical content and atomic repository (migration 0014)

`yomoku-import::epub::EpubDocument` has an optional metadata title, ordered
`EpubChapter` values, a container-path → media-type/original-image-bytes map and
optional cover path. The repository adapter supplies the filename title fallback.
Select the first OPF rootfile from container.xml. Follow spine order, including
`linear="no"` auxiliary content because internal navigation is unavailable; never
infer chapters from headings. Omit empty text-only chapters; an empty book fails.

Each chapter stores its container path and immutable canonical UTF-8 text. Ordinary
XHTML ASCII whitespace collapses to spaces, block boundaries to a blank line,
`br` to LF; `pre` preserves whitespace. Ignore CSS/scripts/head/hidden content.
Heading levels, emphasis/strong and links are half-open chapter-local byte spans.
HTTP(S) website links are marked external; internal/other schemes retain an inert
span and original target. Ruby retains only base text in canonical content, with
primary `rt` readings in sorted `SourceRuby` ranges; omit fallback parentheses and
secondary `rtc` annotations. Segmented ruby joins primary readings over the base.

Each local image occupies a U+FFFC placeholder (three UTF-8 bytes) with an exact
range, resolved container path and separate alt text. Alt/ruby readings are never
analyzed as body text. Preserve repeated placements, storing bytes once per path.
SVG image wrappers resolve `href`/`xlink:href`; SVG spine items become image-only
chapters. EPUB 3 cover-image and EPUB 2 cover metadata resolve to the image map.
Image dimensions/bitmaps are transient Android presentation data; no dimensions or
cover cache are persisted. Raster and bounded static SVG images render at source
positions through the Android loader.

Stored/Deflate ZIP reading is memory-only. Resolve relative/percent-encoded paths
once, reject traversal outside the container, aliases/duplicate entries, symlinks,
broken references, damaged consumed bytes and encrypted used resources. Unused
CSS/fonts need not exist or decode, including obfuscated fonts. XML is strict
UTF-8/UTF-16 with validated declarations, numeric references and nesting preflight;
external DOCTYPE declarations never fetch. Custom entity definitions and unresolved
named entities fail. Current compatibility limits include remote images and
unsupported spine media/fallback chains; these fail rather than omit a chapter.
Parser budgets are operational limits, not peak-memory/performance measurements.

`import_analyzed_epub[_cancellable]` uses the bounded default parser and one
immediate transaction. Shared `imported_files` identity hashes original container
bytes, independent of canonical text/paste identity. Duplicates reference the first
live document before parsing/analysis, including an earlier TXT/SRT interpretation.
`find_imported_file_id` reads just that identity; `find_imported_file` atomically
adds a Library reference validated against the complete saved document.
SHA matches require exact byte equality. Different bytes with equal canonical
content remain new documents. Title uses metadata when present, otherwise the
literal display filename; retain both filename and optional original metadata title.

Migration 0014 rebuilds `sources` to widen kinds to EPUB and `imported_files` to
allow `text_encoding = 'epub'` (a container marker, not a text decoder choice).
Existing IDs, file hashes, foreign keys and source sequence high-water marks survive
atomically; failure restores the old schema/data. No existing content is reparsed.
Allocate ordered provenance chapters plus live canonical text. `epub_documents`
stores filename/metadata title/cover path and chapter/asset counts. `epub_chapters`
stores each live chapter's container path, text digest and ruby/span/placement counts.
`source_ruby` reuses canonical chapter-local UTF-8 ranges. `epub_spans` stores ordered
heading/emphasis/strong/external/inert link spans, including level/target. Overlapping
formatting is preserved. `epub_assets` stores one media type/original BLOB/digest per
(DocumentId, container path); this composite identity is shared by cover and repeated
placements. `epub_inline_images` stores ordered placeholder ranges/alt/path with
chapter/document and asset ownership FKs. No bitmap decoding/dimensions/cache yet.

The ready analyzer runs once per canonical chapter inside that same transaction.
Each original Mode B snapshot and all unique Japanese surface registrations commit
together, preserving existing statuses and creating no encounters/time/position.
Cancellation checkpoints surround parsing, assets, chapters and analysis; use the
existing final commit gate outside FIFO/off main. Cancellation or any later-chapter,
statement or COMMIT failure rolls back the entire file, including allocated IDs.
A winning commit and earlier files remain. No persistent import queue is added.

`load_epub_document` atomically reads all saved chapters/assets/source bytes with
their durable identities, without ZIP parsing or analysis. Verify digests, expected
counts, contiguous indexes, UTF-8 anchors, image placeholders/references and position
ownership. Corruption fails without repair. Library progress sums preceding chapter
bytes plus the current chapter offset over total canonical book bytes; backward
movement can decrease it. Live deletion cascades EPUB metadata/assets; deleting
chapter text drops source ruby, while provenance/analyses/global vocabulary survive.
Reimport allocates fresh IDs/position/finish state. Production deletion remains planned.

Ordered-chapter reader repositories now support the saved EPUB content; see below.
EPUB runtime/FFI imports and mixed Android selection are connected.
Shared file preflight and TXT/SRT/EPUB Existing results now return `LibraryDocument`, preserving the first
interpretation's title, whole-book progress and finish marker without a content
payload. Identity and complete document validation occur in the same read/import
transaction; inconsistent saved content fails without writes or analysis. Core
Created/Existing results also carry Library references. Private StorageFinished
changes did not alter exported event/request/view schemas. The subsequent ordered
reader migration changes ViewModel.loaded_document to ReaderDocument. Current app
imports accept paste and mixed EPUB/TXT/SRT selections.
The typed `EpubImport` transports original bytes/display name without text encoding.
Its FIFO adapter validates a bounded container before Full, then invokes the atomic
whole-book repository with a fresh published cancellation attempt. Container errors
use `InvalidEpub`; storage/engine errors keep their existing categories. Created
projects the committed ordered document to a Library reference without reloading
or auto-opening; all three file types share the direct cancellation/commit gate.

## Ordered-chapter reader repositories (no migration)

Pure `ReaderDocument` carries live title/finish/position plus every `ChapterText` in
source order. Validate a nonempty chapter list, unique chapter IDs, common source,
UTF-8 offset bounds and saved-position ownership. `ReaderPage::open_document` and
`at_document` project one chapter-bounded page with a whole-document page counter.
`library_summary` sums preceding chapter bytes and current offset over total bytes.
`PageCompletion::next_document_position` validates the exact departing position/page,
then advances to the next page's chapter/start or the final chapter EOF.

`load_document_reader_snapshot` atomically returns `DocumentReaderSnapshot` with
the document, global preset and ordered `ReaderChapterAnalysis` identities. Each
identity explicitly includes its chapter ID and original Mode B or Missing; no
analyzer, source parsing, token rescan or registration runs. The text projection
omits source bytes/image payloads, while its EPUB loader still verifies all saved
content/digests/annotations/assets. This repeats on each reader repository operation;
large-book performance has not been measured or optimized.

`ReaderDocument.formatting` carries sparse keyed `ReaderChapterFormatting` entries.
Each chapter key is owned and unique; `ReaderTextSpan` has a nonempty scalar-aligned
chapter-local `TextRange` and Heading(1–6), Emphasis, Strong or ExternalLink(HTTP(S))
style. Nested formatting is valid. Stored inert links stay in SQLite and are omitted
from this display projection. `ReaderPage.formatting` clips intersections into
page-local UTF-8 ranges, preserving the canonical text and all durable identities.
Plain/single-text documents have no formatting entries. No migration or source
rewrite is required. Source ruby uses the separate keyed projection below.

`ReaderDocument.source_ruby` carries sparse, unique owned `ReaderChapterRuby`
keys for TXT/EPUB. Each ordered, nonoverlapping `ReaderRuby` has a nonempty
scalar-aligned chapter-local range and complete nonempty source reading.
`ReaderPage.source_ruby` clips intersecting bases into page-local UTF-8 ranges,
retaining the full reading for a crossing base; readings cannot be divided by
base byte/character counts. Canonical text, saved offsets, logical pages and
word/link identities stay unchanged. Paste/SRT and chapters without ruby have
empty projections. TXT source, EPUB content and TXT reader loads share the same
read-only ordered-index/range/reading validator. Source readings survive missing
analysis or deleted vocabulary and are independent of all five statuses. Generated
Kotlin transports both keyed chapter and clipped page forms. No schema/parser
change. The enabled source-preferred projection and Android layout are described
below.

`ReaderDocument.images` carries sparse, unique owned `ReaderChapterImages` keys.
Each source-ordered `ReaderInlineImage` holds one exact U+FFFC `TextRange`, literal
alt text and a `ReaderImageAsset` (canonical container-relative path + media type).
The enclosing document ID scopes asset identity. Repeated placements may share
one asset; its media type must agree throughout a document. Paths cannot contain
empty/dot/parent components, backslashes, colons or controls; media types have a
nonempty image/ subtype without whitespace/controls. Paths stay opaque and are
never resolved against the platform filesystem or fetched as URLs.

The repository resolves each reference against the already verified saved asset
map. `ReaderPage.images` selects whole placeholders for the current chapter/page
and translates their ranges into page-local UTF-8. Text, pagination, positions,
token anchors and completion semantics remain unchanged; images do not add word
occurrences. Plain documents have no image references. No original source/image
BLOB, bitmap or dimensions enter reader snapshots.

`load_epub_cover_reference(document_id)` projects the existing nullable
`epub_documents.cover_path` and joined asset media type/SHA-256/byte length in a
read transaction. It copies no source/chapter/image BLOB and guards the digest
copy at exactly 32 bytes. Live EPUB ownership/import metadata and declared cover
reference syntax, existence and nonempty <=32 MiB size must validate; damaged
declared metadata fails. No declaration, non-EPUB and missing/deleted IDs yield
None. The stored digest is an advertised cache identity; it is not rehashed here.
New cached pixels must use the verified asset read below. No migration, write or
core payload change. Android keys a transient 4 MiB/64-entry thumbnail LRU by
media type plus SHA-256 after resolving live metadata. Each miss uses the verified
asset read and compares its bytes with that frozen digest off main before decoding
within 16,384 pixels. Published thumbnails retain source geometry, share content
across document IDs/paths and survive Activity recreation through CoreSession.
No thumbnail bytes/negative results are persisted; process restart rebuilds the
cache. Eviction drops references without recycling published pixels.

`load_epub_image_asset(document_id, ReaderImageAsset)` reads one exact asset key
in a read transaction, requiring a live EPUB document, EPUB metadata and imported-
file kind. It validates the opaque key, expected image media type and selected
asset SHA-256. SQL returns no BLOB above the 32 MiB import-entry budget, before
copying into Rust/JNI. Missing/deleted/non-EPUB documents and missing keys yield
None; damaged live metadata, size/media/digest mismatch yield InvalidDocument.
This narrow read intentionally does not verify/copy source bytes, chapter text
or unrelated assets; ordinary reader loads retain their full integrity checks.
No cache, write, reanalysis, history or progress change occurs. Reimport's new
document ID prevents old requests from addressing the replacement. Runtime
enqueues reads on the existing FIFO and delivers one-shot typed JNI callbacks
outside core state. Android now performs bounds-first sampled raster decoding and
source-position rendering outside persisted/core data. Per-page shared bitmaps and
dimensions are transient. The full-screen viewer shares those bitmaps and stores
only its placement selection, zoom and normalized source focus in Compose saved
instance state for configuration changes. It makes no progress/history/vocabulary
writes or additional asset reads; page changes/close dispose viewer state.
Static SVG assets now use the same verified bytes/pixel-bounded Android decoder,
with XML/reference preflight and no resource I/O. Fractional SVG source dimensions
are presentation-only, never stored or used for text positions. Embedded base64
PNG/JPEG and nested self-contained SVG resources use transient bounded bitmaps,
private resolver keys and presentation-only viewport wrappers; original saved
asset bytes remain unchanged. Nested documents share graph/pixel bounds while
retaining independent IDs/styles. Additional resource loading, complex paint
and embedded-image stylesheets are outside v1 scope; unsupported images keep
their existing fallback (user clarification 2026-10-27). Library covers use the same
bounded decoder with a smaller pixel budget; all cache/bitmap data stays outside
SQLite/core state.
No schema migration is needed.

`open_document_reader_snapshot` alone resets finished books to the first chapter's
start. Snapshot/analysis/settings validation and reset share one immediate
transaction; any failure rolls back the reset. `save_document_reading_position`
accepts any owned chapter/scalar boundary and creates no reading history.

Existing time/prepare/record/token/selected-word repositories use this same complete
document loader. Pending durations remain keyed by chapter and exact offset; page
time reads/consumption never reach another chapter, including at chapter EOF.
Completion uses the departing chapter's analysis/occurrences and advances across
the boundary atomically. Only completion of the whole book's final page sets the
permanent finish marker. Replay checks still precede live-content validation;
rereads allocate fresh IDs. Image-only final chapters complete with zero new words.

The application/FIFO/typed bridge now consume these ordered repositories.
StorageResult.Loaded carries DocumentReaderSnapshot; PositionSaved/PageCompleted
carry ReaderDocument. ViewModel.loaded_document serializes every chapter in source
order, including the exact chapter-local position. Android renders ReaderPage as
before. Core validates exactly one original-analysis entry per owned chapter;
missing/duplicate/foreign keys fail. The active session retains this keyed map,
selecting the current chapter's identity after each movement. A plain LoadPaste
never replaces the opened reader's identities. Previous uses all chapter pages;
reflow preserves exact chapter/offset, and only whole-book final completion returns
to Library/celebrates. No chapter truncation, parsing, Full load or registration.

Legacy PastedDocument/ReaderSnapshot and their single-text read/open/position APIs
remain for repository callers and still reject EPUB. Their historical protocol
names LoadPaste/OpenReader remain stable; only the loaded-document view field's
serialized shape changes. No database migration or parser change. Formatting,
image metadata and source ruby now travel in reader payloads; image bytes remain
outside them. Android selection accepts EPUB/TXT/SRT.

## SRT canonical content and atomic file repository (migration 0013)

`yomoku-import::srt::SrtDocument` contains one canonical UTF-8 text string and
ordered nonempty half-open byte ranges for its cues. Normalize CRLF/CR to LF,
retain line breaks and dialogue whitespace within cues, and join cues with exactly
one blank line outside their ranges. A leading Unicode BOM is removed. Counters
immediately before recognizable time ranges and the ranges themselves are omitted;
missing counters/separators, nonsequential counters and short/dot timestamps recover.
Do not infer chapters from cues. Common HTML formatting is stripped, `<br>` becomes
a hard break and basic/numeric entities decode once after tag removal. Speakers,
numeric dialogue, unknown markup/entities and unrecognized damaged timing stay
visible. Empty canonical text is rejected. No times or reading events are created.

The parser takes decoded text. `import_analyzed_srt` and its cancellable variant
reuse strict UTF-8/Shift-JIS decoding (`txt::decode`), including BOM handling,
explicit selection for uncertain bytes and rejection without replacement. They
never apply Aozora rules to subtitle dialogue. Filename supplies the literal title.
One immediate transaction checks shared original-file SHA-256 identity, then saves
original bytes/encoding, one canonical chapter, cue ranges, Mode B analysis and
unique Japanese registrations. No encounters, reading time or position are added.
Exact duplicates (including a previously imported TXT with the same bytes) reference
the first live interpretation before decoding/analyzing, preserving title/progress/
statuses. Equal canonical text with different original bytes remains a new file;
paste identity is separate. `find_imported_file` is the shared read-only preflight;
the existing `find_imported_txt` delegates to it.

Migration 0013 widens source kinds to paste/TXT/SRT using the existing source-table
rebuild protocol, preserving IDs, foreign keys and the source sequence high-water
mark. `subtitle_cues` has a chapter-text FK with delete cascade, contiguous ordered
cue indexes and nonempty start/end UTF-8 ranges. `load_srt_source` reads original
bytes/encoding/cues atomically without parsing or analysis. It validates the digest,
scalar boundaries, exact blank separators, full canonical coverage and nonempty
cues; corruption fails without repair. Live deletion drops file identity and cue
rows while provenance/global statuses survive. Reimport creates fresh progress.

A fresh shared `ImportCancellation` uses the same before/after-analysis checks and
final commit gate as TXT. Cancellation rolls back the entire current file and IDs;
a winning commit and earlier files stay. Statement/commit/analysis failures also
roll back. The typed FIFO/JNI adapter now carries `SrtImport` and SRT-specific
encoding/empty outcomes. It preflights exact duplicates and decoding before Full,
then invokes the authoritative cancellable repository. Shared `cancelFileImport()`
bypasses FIFO for EPUB/TXT/SRT; `cancelTxtImport()` remains a compatibility alias.
The retained Android mixed picker routes SRT through this typed adapter, including
format-specific encoding choices and terminal errors.

## TXT canonical content and atomic file repository (migration 0012)

`yomoku-import::txt::TxtDocument` contains the selected UTF-8/Shift-JIS encoding,
one canonical text string and ordered nonoverlapping `SourceRuby` records. Each
ruby stores a reading and a half-open UTF-8 byte range into the canonical base text,
after markup removal and before analysis/ID allocation. Hard LF/CRLF/CR and Unicode
scalars are preserved; ruby readings do not enter analyzed text. Common layout
directives do not create chapter boundaries. Unsupported notation stays visible.
`import_analyzed_txt` hashes original bytes before its immediate transaction, then
references the live exact match without parsing or analysis, or commits filename
title, original bytes/selected encoding, canonical text/ruby, original Mode B and
unique Japanese registrations together. It returns Created/Existing only after
commit; failures discard this file and preserve earlier imports. Duplicate titles,
selection changes or equal canonical text do not change original-byte identity.
An exact byte comparison rejects a false digest match. Paste identity is separate.
`find_imported_txt` is a read-only transaction using the same verified lookup; the
worker uses it to skip Full on duplicates. A miss never reserves an ID or changes
the authoritative immediate import transaction; concurrent creators still deduplicate.

The optional repository entry point `import_analyzed_txt_cancellable` uses the same
transaction with an externally shared per-file cancellation handle. `Ok(None)`
means rollback of the whole current file, including source/chapter IDs and sequence
updates; earlier committed content, statuses and history remain intact. Cancellation
checkpoints surround parsing/analysis and precede ruby writes. Final cancellation
check and COMMIT share a mutex, so cancellation cannot return and then allow a new
commit to begin. A successful commit already holding the gate remains completed.
Cancellation does not undo completed files or persist resume state. Production
runtime/JNI shares a fresh handle before each accepted FIFO import and projects
rollback as `StorageResult::ImportCancelled`, without an error/summary or new
durable state. Android Cancel clears unsubmitted URIs, ignores stale provider data
and awaits both the direct native return and current result. It retains completed
outcomes in the in-process summary; no queue or cancellation state is persisted.
No schema change.

Migration 0012 rebuilds `sources` to permit `kind IN ('paste', 'txt')`, preserving
all IDs, provenance and the autoincrement high-water mark. The migration runner
uses SQLite's documented create/copy/drop/rename procedure with foreign keys off
outside BEGIN, checks `foreign_key_check` before committing pending migrations,
and restores the original enforcement setting on success or rollback. Normal
Storage connections always enable foreign keys. Existing source/chapter/history
references remain intact; migration never analyzes or rewrites existing text.

`imported_files` stores live document PK/FK (delete cascade), unique 32-byte SHA-256,
original BLOB and `utf8`/`shift_jis` encoding. `source_ruby` stores chapter-text FK
(delete cascade), ordered ruby index, canonical start/end UTF-8 offsets and reading.
Each file has one canonical chapter. `load_txt_source` reads bytes/encoding/ruby in
one transaction, verifies digest, ordered nonoverlapping scalar ranges and readings,
and rejects corruption without repair or reanalysis. Historical single-text
snapshot APIs accept paste/TXT/SRT; application/bridge readers now use ReaderDocument
and the generic ordered repositories above, including source-ruby projections.
Ruby rendering uses the same canonical text. Typed per-file runtime/JNI imports
and the retained Android mixed-format picker, encoding choices, progress, Cancel
and summary are
connected. The Android foreground
lease/notifications persist no queue, URI, current-file bytes, progress or summary.
Process death preserves only committed per-file data; a fresh helper starts empty
and never automatically resumes unfinished selections. No schema change.
Deleting live content removes file identity/ruby;
retained source/chapter/history and global vocabulary survive, and reimport starts
with fresh IDs, position and finish state. Production deletion is still planned.

## Immutable word/name dictionary index

Android bundles the independently pinned index and complete source/legal bundle;
`third_party/dictionaries/android.lock.json` pins build-manifest SHA-256 and index
size/SHA-256. Lazy worker installation verifies the same independently compiled-in
index identity before returning a private read-only content-addressed path. Cached
bytes are reverified; repair uses a fresh inode. This adds no user migration or
mutable database state. NativeBridge passes this path lazily through the existing
FIFO to a separately held read-only dictionary connection.

The independently prepared JMdict English/JMnedict asset uses schema version 1
(`application_id = 0x594f4449`), separate from all user-database migrations.
`entries(source, entry_id, payload)` retains ordered English senses, readings,
restrictions, inherited POS and name groups in canonical JSON. Source-scoped IDs
prevent collisions between dictionaries. `forms(surface, source, entry_id)` has
a BINARY composite primary key for all exact written/reading candidates; no
normalization or fuzzy matching changes spelling identity. `metadata` stores the
locked source provenance. Full field definitions are in
`third_party/dictionaries/README.md`. These tables do not refer to vocabulary IDs,
statuses, positions or encounter history. The Rust dictionary repository opens
only existing read-only assets and rejects unsupported format identity. It returns
typed candidates separately for words/names, ordered by source-scoped entry ID,
retaining surface/base identity and per-form provenance without duplicate entries.
Complete payloads preserve original order/metadata; applicable reading/sense views
use indices into those payloads. Written queries select all permitted readings;
kana queries select every permitted spelling pair. re_nokanji/kana-only readings
have no spelling pair; both stagk and stagr must be satisfied for each displayed
sense. Empty applicable views authorize no displayed senses. No analyzer reading,
normalization or fuzzy fallback narrows/broadens matches. Index access never changes
vocabulary, encounters or position. The Android publisher verifies bytes before
the FIFO opens the index; format identity checks alone do not verify the checksum.

The build bundle preserves original compressed snapshots, a source/date/checksum
lock, complete CC BY-SA attribution/legal documentation and a manifest of artifact
hashes/tool versions. New source versions publish to fresh paths without replacing
an asset that could be open. No mutable user schema or backup format changes.

## Custom dictionary persistence (migration 0017)

`custom_dictionary_entries` is a separate STRICT/WITHOUT ROWID user-state table,
keyed by exact BINARY `surface`, with `semantics_version = 1`, `english_meaning`
and `reading`. Both fields must be nonblank; the pure `CustomDictionaryEntry`
retains literal Unicode/whitespace without language detection or normalization.
SQL rejects empty byte values and readers revalidate stored values/version.
The table contains no built-in payloads, analyzer mapping or status/history data.
It has no document/vocabulary FK: one custom definition belongs to the spelling
and does not require a live registration or a particular originating document.
Only explicit custom-entry deletion removes it through this repository.

`load_reader_custom_entry`, `save_reader_custom_entry` and
`delete_reader_custom_entry` take the original `ReaderTokenKey` plus token index,
reuse selected-word ownership/whole-chapter validation, and derive surface/base
from stored original Mode B. Every operation queries the separate read-only index:
any JMdict/JMnedict surface/base candidate returns `BuiltInMatch`; dictionary errors
never count as missing matches. No custom base-form fallback is applied. Missing
documents return None/false; stale/non-Japanese/out-of-page selections fail.

Reads use one consistent user-state transaction. Immediate save/delete transactions
validate existing custom values and commit atomically; identical saves and repeated
deletes do no writes. Statement/deferred-COMMIT failure preserves the prior entry;
corrupt values fail without repair. Vocabulary status/deletion does not influence
custom identity, and these operations never register words, rewrite analysis,
change position/time or record encounters/status events. Source/analyzer furigana
remains unchanged. `load_reader_word` also reads this exact-surface fallback in its
existing transaction when both immutable candidate sections are empty. Invalid
custom values fail the whole lookup; a later built-in match suppresses retained
custom data. The typed core/FIFO/FFI carries `ReaderCustomEntry` and explicit
save/delete selection commands, with a separate retryable mutation error. This
transport adds no schema or persisted UI state. Android Add/Edit/Delete and explicit
save/delete retries use these commands; only committed definitions persist, while
selection-scoped drafts are discarded with the reader word panel.

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

`create_analyzed_paste` composes document creation, analysis and registration in
one repository transaction (see below); production paste imports use its
deduplicating wrapper on the native FIFO worker.
Migration 0010 adds completion-driven New→Known events with command provenance
in the occurrences/time/position transaction, without calling manual selection.
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
commit + android-loader-v1 source patch + adapter-v1/embedded-defaults and
Full 20261015 + dictionary SHA-256. The loader patch does not change tokenization;
its explicit profile identity keeps patched snapshots distinguishable.
Classification uses original Japanese script and analyzer POS, including OOV rare
kanji. OOV readings/dictionary forms are absent; original surfaces never become
normalized identity. Normalization-generated empty original spans are omitted,
and the complete result must still pass canonical coverage/nonoverlap validation.
Real A/B/C output survives repository saves and reopen without a schema change.
Production paste imports atomically persist Mode B analysis and registration
through the verified-descriptor native worker. Existing pastes are not rescanned.

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
Real analyzer output and atomic paste registration are implemented separately.
The pure `segmentation` module now derives effective boundaries without modifying
these snapshots. `segmentation_from_snapshot` creates an exact partition from
A/B/C output only when the engine/dictionary profile matches original B and both
ends align with complete, adjacent Japanese tokens in both snapshots. Existing
manual partitions use the same exact-source anchors/parts representation.

`resolve_segmentation` matches global rules against complete, adjacent Japanese
original B tokens. It never replaces a substring within a different B word, crosses
whitespace gaps/non-Japanese tokens or normalizes spelling. Local rules apply only
to their exact source/chapter/range and must cover the same eligible B spans.
Local rules take precedence over overlapping globals; within a scope the newest
OverrideId takes precedence. Repeated self-overlapping matches of one global rule
choose the leftmost nonoverlapping matches. A losing overlapping rule stays active
and may reappear after the winner is reversed; superseded same-key rules stay
inactive. Resolution is independent of page length and input rule order.

`ResolvedSegment` carries exact surface/anchor, the applied OverrideId when present,
and an original token index only for an exact unchanged B token span. Changed
boundaries supply no invented reading or dictionary form. `ChapterSegmentation`
retains the pinned original AnalysisId. `Storage::load_chapter_segmentation` reads
the original B plus active global/relevant local rules in one deferred transaction;
missing B returns None, invalid canonical content/rules fail without repair.
Origin-deleted globals still resolve from retained provenance and exact parts.

`EffectiveChapterSegmentation` additionally retains the original profile and wraps
each boundary with optional `SegmentMetadata`: source AnalysisId, A/B/C mode,
whole-chapter token index, Japanese classification, reading and dictionary form.
The pure resolver matches exact source spans, preferring original B, then A, then
C; input order does not affect the result. Repeated spellings retain their own
occurrence's metadata. A known token with no reading/lemma is distinct from an
unresolved manual boundary, which has no metadata (including classification).
Never infer metadata by joining or truncating adjacent analyzer tokens.
`load_effective_chapter_segmentation` reads original B, stored A/C with exactly the
same engine/dictionary profile and active rules in one deferred transaction. Entire
eligible snapshots are validated; corrupt matching output fails without partial
results/repair. Missing B stays missing; later profiles are excluded. This adds no
table or migration and creates no alternate snapshots automatically.

`EffectiveReaderChapter` privately retains validated resolution and borrows canonical
chapter text. Its page tokens carry whole effective-chapter segment indices, exact
surfaces/anchors, page-local UTF-8 display ranges, optional exact metadata and explicit
reading eligibility. Analyzer Japanese tokens are eligible; exact non-Japanese tokens
are excluded. An unresolved manual partition inherits eligibility from the Japanese B
span that made the edit valid (`ManualJapaneseSpan`), without gaining analyzer
classification, reading or lemma. Visibility includes crossings on both pages;
completion projection assigns each whole occurrence only to its start page.

`load_effective_reader_tokens` reads document ownership, resolution and live global
statuses together. It returns the complete effective segmentation alongside tokens;
distinct visible surfaces use indexed vocabulary lookups, sharing results for repeats.
New split/merged spellings and deleted entries stay absent until an explicit future
registration path. A missing live document returns None; missing original B returns
a snapshot with absent segmentation and no tokens. Malformed pages/snapshots fail.
Query-only/reopen tests preserve database bytes and stored row values.

`EffectiveReaderTokenKey` freezes document/page and complete effective segmentation,
including original B, applied rule IDs and metadata provenance. It excludes live
vocabulary/status/count/custom-entry data; Missing B has no key. The separate
`load_effective_reader_word(dictionary, key, segment_index)` transaction validates
current resolution equality before selecting a visible eligible segment, including
crossing-page fragments. Caller-provided surfaces/base forms never drive lookup.
Whole-chapter validation still rejects off-page corruption. Changed resolution is
InvalidSelection; deleted document is None. No revision table or migration is needed.

Selected details share the original reader's indexed lifetime exact-surface count,
complete applicable word/name candidates and exact-surface custom fallback. Built-in
matches suppress custom entries. Only exact analyzer metadata provides a base form;
manual spans use surface-only lookup without guessed metadata. Deleted/unregistered
vocabulary stays absent with count zero. Query-only bytes/reopen, literal Unicode,
all presets, reversal/replacement, metadata arrival and retry tests cover this path.

`load_effective_reader_custom_entry`, `save_effective_reader_custom_entry` and
`delete_effective_reader_custom_entry` use the same frozen key and effective segment
index. Shared selection validation re-resolves the complete chapter in the operation's
transaction. Reads stay query-only compatible; saves/deletes use an immediate
transaction so resolution cannot change between validation and mutation. Exact
metadata supplies the only allowed base form; unresolved manual spans use surface
alone. Either word or name matches reject all three operations, including base-only
matches. Dictionary failures never grant fallback permission. Stale selections fail;
deleted documents return None/false. Shared original/effective mutation helpers keep
literal fields, corrupt-entry rejection, write-free identical retries and atomic
rollback semantics. Entries remain global by exact spelling, independently of live
or deleted vocabulary; these APIs never register it or rewrite readings/history.
No migration is added.

These remain separate domain/repository APIs: live reader token/word/furigana and
completion commands still use original B. Read projections never register vocabulary,
reset position or rewrite history. Effective completion storage is described below;
override-time registration, core/FIFO/FFI integration and segmentation UI remain pending.

## Frozen effective completions (migration 0018)

`EffectivePageCompletion` is an opaque pure-domain value constructed by a validated
`EffectiveReaderChapter`. It contains the ordinary page completion, complete effective
selection key/provenance and start-owned eligible occurrences, with no live vocabulary
IDs/statuses. Exact analyzer metadata retains its source; manual unresolved spans
have no invented reading/lemma. Only the page owning a token's start counts it.

`prepare_effective_page_completion(intent, key)` holds an immediate transaction:
check live document/page/position/preset, original B and complete resolved key; read
the drained durable page time; reserve a command ID; store its exact serialized
payload in `effective_completion_commands(command_id, semantics_version=1, payload)`.
Preparation neither registers surfaces nor records history/consumes time/moves position.
The JSON format is versioned independently from the internal bridge and is currently
write/compare only: no process-restart replay or automatic recovery of unused commands.

`record_effective_page_completion` first compares the entire command with that durable
reservation, before replay or live-content checks. Exact successful retries return
AlreadyRecorded even after deletion/reflow/reversal; legacy APIs reject effective IDs.
Before a first commit the ordinary live position/preset/time checks still apply.
Later rule/metadata changes cannot reinterpret the reserved occurrences. Register only
this page's distinct eligible spellings, preserving existing statuses and re-registering
deleted ones as New; record every repetition, New-only Known events, time consumption,
position and finish atomically. Failures retain the original reservation for exact retry.

The existing `completion_analyses` header retains the original B basis/profile. Presence
of the versioned effective reservation distinguishes its `reading_occurrences.token_index`
as an effective segment index; original commands retain analyzer indices. The payload
retains rule IDs and exact per-segment metadata provenance without live analysis/rule/
document FKs. Historical rows are never rewritten. Vocabulary deletion still cascades
its actual occurrence/status rows; the reservation is identity/provenance, not a counter.
Migration 0018 creates an empty additive table and leaves all previous histories intact.
Full-chapter payload size/cost is unmeasured and must be reviewed during live integration.

## Read-only reader token/status projection

`load_reader_tokens(document_id, page)` reads live immutable text, the original
Mode B identity and visible Japanese tokens with current exact-surface vocabulary
statuses in one deferred read transaction. None means no live document. Missing
analysis returns `ReaderAnalysis::Missing` and no tokens; it never substitutes A/C,
another chapter or a later B snapshot. Existing analysis is fully revalidated;
corrupt original output/statuses error without fallback or repair. The supplied
span must belong to the document, be nonempty and use valid UTF-8 boundaries. It
does not need to match persisted position or today's preset, so a reflowed caller
can request its exact displayed span without changing progress.

`ReaderTokenSnapshot` echoes document/page/analysis for stale-result checks.
Each `ReaderToken` retains the original token index/full chapter anchor, full
surface, optional reading/dictionary form and a clipped `display_range` relative
to page text in UTF-8 bytes. A boundary-crossing token appears on both visible
pages under the same identity; completion still counts it only on its start page.
Whitespace gaps and non-Japanese tokens remain in page text without selectable
spans. Repetitions stay separate tokens while sharing one global vocabulary ID.

`ReaderVocabulary` contains ID/status, or stays absent for deleted/unregistered
surfaces. Lookup never implicitly registers New, applies segmentation overrides,
records encounters/status events, resets finished-book position or saves time.
The transient `ReaderTokenKey` carries document/page/analysis in the explicit
Crux/FIFO/FFI read request. Core requires the pending request, result and current
reader identities to match before exposing `reader_tokens`; it clears cached
tokens on departure, position/completion results or page-span reflow. This adds
event/view payloads, not persisted columns or migrations.
Statuses come from one exact-surface join for the visible page, not per-token
lookups. This correctness-first implementation validates/loads the whole chapter
analysis per request; large-document lookup performance remains unmeasured. No
schema change or new dependency is required.

`load_reader_word(dictionary, key, token_index)` reuses that validation inside a
single user-state read transaction and returns the selected full token/status,
echoed key, lifetime occurrence count and exact surface/base dictionary candidates.
The request cannot supply arbitrary lookup text or a vocabulary ID. It must match
the original analysis and a Japanese token intersecting the supplied page. Missing
document is None; missing/stale analysis or an invalid index is InvalidSelection.
Dictionary errors remain distinct from empty successful matches, without partial
results or mutation. ReaderWordSelection carries the key/index through the typed
Crux/FIFO/JNI request; ReaderWordDetails and concrete word/name candidate payloads
are transient view values. A result must match the pending/current selection and
full loaded token identity before attachment; live status may differ and refreshes
all repetitions. Dismissal, Settings, navigation, token refresh, position/completion
and reflow invalidate selection. Separate lookup errors survive timer saves until
explicit retry or dismissal. Both success/failure drain queued final time. No schema
or persisted selection is added. Compose renders only matching selected details,
uses applicable reading/sense indices without broadening restrictions, and keeps
word/name sections separate. Closing/recreation discard the transient selection.

Lifetime counts use `COUNT(*) WHERE vocabulary_id = ?` through the existing
`reading_occurrences_by_entry` covering index. They include every completed
occurrence/repetition/reread across documents, retain history after document
deletion, exclude registrations/manual choices/pending time and never aggregate
the linked base form. Vocabulary deletion cascades those occurrences, so an
absent entry has zero; actual later registration starts a new ID/count. Full
crossing-token identity is retained under reflow, without multiplying encounters
for its display fragments. The immutable dictionary connection stays separate.

`ReaderStatusChange` carries the current token key, existing VocabularyId, one of
five statuses and a nonnegative Unix millisecond timestamp through core/FIFO/FFI.
It is accepted only for vocabulary present in the loaded current snapshot. The
existing `set_vocabulary_status` transaction changes global status and appends a
manual event only when the value differs; progress/time/encounters remain untouched.
The worker returns the committed ID/status directly, avoiding a fallible post-commit
read. Core updates every visible repetition only for the matching pending/current
key. A deleted ID returns None and clears those references; it is never recreated.
Failures keep the prior snapshot and allow an explicit repeat choice; same-status
replays add no event. Navigation discards transient results/errors but does not
cancel accepted writes. Failed choices still drain queued final time saves.

Android converts page-local byte spans to UTF-16 only at exact scalar boundaries,
validating echoed identity, ordering, full anchor lengths and clipped surface text.
Full crossing-token metadata survives display clipping. Link activation opens only
transient UI; no status command is sent until a direct choice. System-theme status
backgrounds follow the product palette, with Known/absent entries unhighlighted.
The panel reads current committed statuses and disables absent vocabulary rather
than recreating it. No schema, persisted UI state or dictionary data is added.

## Document identities and text anchors
`document.rs` defines distinct `DocumentId`, `SourceId` and `ChapterId` newtypes.
Each is an integer in `1..=i64::MAX`; Rust constructors and Serde decoding reject
zero/negative values. Generated Kotlin represents each as a separate class with
a `Long` value. These identities are database-local, never derived from titles,
chapter order, page numbers, file paths or content hashes.

- Document identifies one live Library instance and its mutable progress/title.
  Deletion/reimport must allocate a fresh identity with fresh progress/checkmark.
- Source identifies retained historical provenance for that import. It survives
  document deletion; paste duplicate keys reference live documents (migration 0008).
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
persisted content. Application/FFI snapshots now use the ordered ReaderDocument.

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
This text-only allocation primitive is retained for low-level storage fixtures;
NativeBridge requires analyzed/deduplicating imports below. File-import metadata
remains unchecked. No reading encounters are created here.

`Storage::create_analyzed_paste` is a separate worker-only repository entry point.
An immediate transaction allocates document/source/chapter identities and exact
text, then supplies that canonical `ChapterText` to a caller-provided analyzer.
The caller must preload/reuse the analyzer before entering this method; the
callback holds the SQLite write transaction and must not do database work or
publish provisional identities. Validate complete source coverage/anchors and
require default Mode B, then store the immutable analysis and register every
distinct token surface marked Japanese. Non-Japanese tokens remain in the
snapshot but create no vocabulary. Use exact surfaces, including OOV Japanese;
do not register lemmas or normalize source text.

Text, tokens, new vocabulary and dated registration events commit together.
Existing vocabulary IDs/statuses/dates/history remain unchanged. The return value
contains the committed document, analysis ID and count of newly registered
surfaces. `PasteImportError<E>` preserves the caller's typed analyzer failure
separately from storage/validation failures. Callback, validation, statement and
commit failures roll back every import row; retry starts a new transaction.
There are no completion commands, positions, encounters or status promotions.
No schema change, automatic rescan, deduplication or production runtime/FFI
operation is introduced by this raw composition. The deduplicating wrapper below
must be used by production imports before they are considered complete.

## Exact paste identity (migration 0008)
`paste_content_hashes` stores a live document ID (PK/FK with ON DELETE CASCADE)
and a 32-byte SHA-256 BLOB, indexed by `(sha256, document_id)`. This table's identity
domain is exact pasted canonical UTF-8 bytes, including line endings, whitespace,
NUL and original Unicode spelling; neither title nor normalized/analyzed text is
hashed. TXT uses original bytes and the separate migration-0012 file identity
domain, never extracted text as an exact-file duplicate key.

The digest index is deliberately nonunique: migration preserves all preexisting
duplicate Library entries. Opening adds only the empty table/index, without reading
all text, registering vocabulary or changing document IDs/progress. Raw
`create_paste`/`create_analyzed_paste` remain allocation primitives without hashing.

`import_analyzed_paste` hashes the incoming bytes before acquiring an immediate
transaction. It lazily hashes unindexed live pastes, one text at a time, then
returns `PasteImportOutcome::Existing` for the lowest matching document ID. Load
validates the document/position, and exact byte equality protects against a corrupt
digest or collision. Duplicates invoke no analyzer and change no document, analysis,
position, vocabulary or history; a deleted vocabulary entry is not recreated.
Legacy hash backfill is the only possible write on this branch; fully indexed
duplicates perform no writes. Backfill may hold the write lock across a large
legacy Library and is not performed on app startup.

For a new document it composes the raw Mode B import and hash insertion in the
same transaction, returning `Created(AnalyzedPaste)` only after commit. The
callback follows the same ready/reused analyzer and provisional-ID constraints
as raw creation. Concurrent imports serialize lookup and allocation, so only
one analyzes/creates. Indexing/analyzer/validation/write/commit errors roll back
the entire transaction, including provisional legacy hashes; earlier committed
imports remain intact. Deleting a document removes its hash while retaining
historical source/chapter provenance and vocabulary; reimport starts at 0% with
new IDs. The production analyzed-import runtime router calls this repository on
its FIFO worker with a preloaded analyzer and a wall-clock registration timestamp.
It maps the committed Created/Existing result to an internal storage result;
core upserts one Library row by document ID without changing persisted state or
opening a reader. Both results project a transient typed import summary; its
document ID/title/outcome live only in core view state. NativeBridge/Android
require this path with lazy verified-descriptor loading and queued cache lifecycle.
Both outcomes are delivered to the Android Library summary. No schema,
production deletion or file import is added. Finished-at-least-once remains separate.

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
progress can decrease and is independent of the persisted finished-at-least-once marker.
EPUB totals include every ordered chapter; single-text sources retain one chapter.

SaveReadingPosition is a typed position-only protocol hook for an active reader;
there is no user-facing seek control. It uses the existing busy guard/FIFO worker.
The displayed page and Library summary update after commit; failures keep the
previous snapshot. Leaving during a save does not cancel the write or reopen the
reader when its callback arrives. Android Previous/right-swipe supplies an anchored
PreviousReaderPage; pure core validates the visible page and derives the preceding
page start before position-only saving. Next/left-swipe uses analyzed completion.

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

## Reader typography (migrations 0015–0016)
Migration 0015 adds `font_size` (14/16/18/20/24, default 16) and
`line_spacing` (125/150/175/200 percent of font size, default 150) to the STRICT
singleton `reader_settings` table. Values are semantic numbers, never enum
ordinals. The migration leaves page length, document content/position, original
analysis, vocabulary and history unchanged; failed ALTERs roll back together.

Migration 0016 adds `furigana`, an INTEGER constrained to 0/1, default 0 (off).
Existing font size, line spacing, page length, content and reading state are
preserved. Invalid boolean integers fail validation without repair.

`ReaderTypography` contains finite font/spacing enums and a furigana boolean.
`reader_typography` validates all three stored values; `save_reader_typography`
validates the existing row/page preset, updates all three fields in one immediate
transaction, and returns only after commit. Missing/corrupt settings fail without
silent replacement.
Library startup loads typography alongside page length; every ordered-document
snapshot includes it from the same read transaction as content/position/analysis.
Legacy single-paste `ReaderSnapshot` remains unchanged; the app uses the ordered
snapshot. `SetReaderTypography`/`SaveReaderTypography`/`ReaderTypographySaved`
carry these preferences through core/FIFO/generated Kotlin. The optional ViewModel
value is absent until a successful load; failed saves retain the previous values and
can be repeated. Typography never changes logical pagination or anchors and
produces no position, encounter, vocabulary or completion writes.
`ViewModel.reader_furigana` is a transient, derived `Vec<ReaderRuby>`, empty when
disabled or no reader is open. Enabled projection retains validated page source
ruby and appends original Mode B token readings only for tokens whose clipped
base has no source overlap. Any source overlap suppresses that whole analyzer
reading: no mapping of part of a compound reading to uncovered characters is
invented. Keep literal readings (including analyzer katakana), sorted by base.
All statuses and absent/deleted vocabulary receive the same readings. Tokens
without readings add nothing; source bases do not depend on tokens or analysis.
Require exact document/page/analysis identity, owned ordered token anchors/indices,
scalar-aligned exact clipped ranges and matching canonical surface fragments.
Reject an invalid analyzer snapshot as a whole, retaining valid source ruby.
Refresh failures, Missing analysis and pending loads therefore retain source
readings. Page moves/reflow/departure follow existing snapshot invalidation; late
callbacks cannot restore departed readings. The projection does not request I/O,
reanalysis or writes, and never changes text, logical pages, saved offsets,
token/link identities or reading/vocabulary/history state. Generated Kotlin
transports the derived field. The visible Settings switch updates only the existing
preference. Compose validates ordered UTF-8 ranges, converts them to UTF-16, clips
around image fragments and paints complete readings above each intersected visual
line. Paint-only readings never enter annotated text or accessibility/tap offsets;
no proportional reading split, schema or persistence change is introduced.

## Reader analysis restoration (no schema change)
`load_reader_snapshot` reads canonical paste content/current position, persisted
page length and the earliest stored Mode B analysis header in one transaction.
The historical `ReaderSnapshot` returns these together; the FIFO now uses the
ordered `DocumentReaderSnapshot` and one identity per chapter. `ReaderAnalysis` is
Available(AnalysisId) or Missing; no Mode B base means legacy absence, never an
A/C or foreign-chapter substitution. Later profiles do not replace the original
base. Invalid metadata/settings fail without fallback/repair. No token scan,
analyzer load or vocabulary registration is performed. Explicit reader entry uses
`open_reader_snapshot` and may reset finished-book position as described below;
`load_reader_snapshot` remains read-only.

Core attaches this identity only to an accepted OpenReader load. Its optional
view field is absent outside the opened reader, follows the current chapter's
frozen identity across position saves, remains fixed on reflow/plain LoadPaste,
and clears on reader exit/replacement. Late
cancelled loads cannot restore it. Generated Kotlin codecs carry the exact i64
identity and explicit missing variant. Completion will validate all snapshot tokens
inside its transaction; Missing does not authorize a text-only completion path.
Timer checkpoints and anchored forward intents are connected below. MissingAnalysis
blocks legacy forward completion without rescan; recovery policy remains open.

## Page completion repository foundation (migration 0005)
`CompletionCommandId` is another positive i64 identity. `completion_commands`
issues it with AUTOINCREMENT before a forward intent is submitted. Repeated
callbacks/retries must reuse one ID and its frozen payload (including timestamp);
a deliberate reread uses a fresh ID. Issued but unused IDs are not reading events
and are never automatically replayed. IDs survive reopen and cannot be reused.

Pure `PageCompletion` carries the document, exact completed TextAnchor, page-length
snapshot, exact departing ReadingPosition, nonnegative Unix milliseconds and a
validated frozen PendingReadingTime total (migration 0009).
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
Migration 0009 also validates and consumes the page's frozen pending duration in
that transaction (details below). Statement/commit failure rolls everything back;
retry reuses the same intent. Position-only
saves never insert history. Concurrent duplicate submissions record once.

This legacy entry point is repository-only infrastructure. Migration 0010 adds the
analyzed entry point used by core/runtime/FFI/Android forward actions below. Do not
use absent occurrence data as completed-reading statistics. Migration 0011 adds
the permanent finish marker; explicit reopening now starts at page 1. Finish Book
uses the analyzed completion path and returns to Library after final-time drain.

## Permanent document finish marker (migration 0011)
`documents.finished_at_least_once` is a non-null integer boolean, constrained to
0/1 and defaulting to 0. Newly recorded completion of the final logical page sets
it to 1 in the same immediate transaction as history, occurrences, statuses, time
and position. Completion validates the whole document's final chapter and its
canonical UTF-8 byte length; earlier chapter endings never set this marker.

The marker never follows position backward or changes with page-length settings,
partial-time checkpoints or duplicate callbacks. `finished_at_least_once` reads
`Option<bool>` for a document (None means absent), without changing position. It
also appears on ReaderDocument, LibraryDocument and ReaderPage, including generated
Kotlin snapshots. Android renders an accessible checkmark in Library and reader.
Each deliberate final-page reread still gets its own completion history.

`StorageOperation::OpenReader` calls `open_document_reader_snapshot` on the FIFO. One
immediate transaction reads the document, resets a finished document's exact
position to the first chapter's start, and returns its marker/settings/analysis identities.
The reset updates Library percentage after commit and preserves all pending-time
buckets and history. Unfinished documents keep their exact position, including
position-only EOF. Already-at-start finished opens avoid redundant writes.
Statement/commit failures roll back and return no new snapshot. Accepted resets
finish even if navigation cancels the pending reader. Ordinary LoadPaste,
load_reader_snapshot and post-completion load_paste stay read-only and preserve
EOF/current offsets. Retained Activity recreation does not dispatch OpenReader.

The migration backfills live pastes with recorded EOF history, matching document,
source and chapter IDs and exact byte length (including embedded NUL). Current EOF
position alone is insufficient; a finished book moved backward remains finished.
Migration failure rolls back the column, backfill and journal together. Deletion
removes the marker with its live document; reimport defaults to false and 0%
while old history/provenance remain. No production deletion API is added here.

## Analyzed page completion (migration 0010)
`record_analyzed_page_completion` takes a frozen PageCompletion and AnalysisId.
It shares the immediate transaction and all page/time/position validation above.
The immutable stored analysis must validate against this document's canonical
chapter. Missing, foreign or corrupt snapshots fail without partial effects.
The caller must retain the analysis ID with the command across callbacks/retries.

`completion_analyses` marks analyzed completions, including zero-Japanese pages,
with semantics version 1, historical AnalysisId, exact engine/dictionary revisions
and split mode. It references the completion, without a live-analysis FK, so text
deletion preserves the snapshot identity. Legacy completion rows remain unchanged
and have no such marker; migrations do not invent missing encounters/promotions.
Retries compare the stored analysis ID before loading live content; analyzed and
legacy commands cannot be substituted for one another under an existing ID.

The pure `japanese_tokens_on_page` helper assigns each Japanese token to the page
containing its start, retaining its entire original span if a boundary crosses it.
A token starting at a page boundary belongs to the following page. Repetitions
stay separate; punctuation/non-Japanese tokens never count. This selects the
frozen base analysis only; segmentation-override resolution remains future work.

`reading_occurrences` stores command/original token index, VocabularyId, full byte
range, exact surface and optional reading/dictionary form. Command provenance
retains source/chapter/date through reading_completions. It has no live text/token
FK; document deletion preserves encounters. Its vocabulary FK cascades deletion,
erasing only that entry's encounters. The entry/command index supports future
lifetime aggregation; aggregate/UI APIs are not implemented here.

Unique departing-page surfaces register only if absent (e.g. actual reading after
vocabulary deletion); untouched pages are not rescanned. The pure New-only policy
updates current status and inserts one dated event per changed entry, with cause
`completion`, New→Known and a required completion_command_id FK. Registration/manual
events retain their original constraints and NULL completion provenance. The table
rebuild preserves existing IDs and the AUTOINCREMENT high-water mark, including an
empty history after vocabulary deletion. A unique command/entry index prevents
duplicate completion promotions. Learning/Familiar/Known/Ignored remain unchanged.

Occurrences, registrations/promotions, pending-time consumption, history and current
position commit together. The outcome is Recorded { newly_known } or AlreadyRecorded;
only a new success can supply an N>0 toast. Duplicate delivery never repeats
status changes, recreates deleted vocabulary, consumes later reread time or moves
progress. Rereading uses a new command ID and adds every occurrence again.
Statement/deferred-commit/migration failure rolls back; no platform/FFI action or
production deletion API is introduced by this repository slice.

## Pending active-time values and repository (migration 0009)
`active_time::PendingReadingTime` validates nonnegative signed 64-bit milliseconds.
`ActiveReadingTime` accepts caller-supplied monotonic u64 milliseconds; it owns no
clock or I/O. Each foreground interval counts until five minutes after the last
interaction. Interactions restart counting after idle without counting the gap;
dictionary interactions use the same path. Background/reader close pauses, and
opening/resuming starts a new idle window. Duplicate resume signals do not extend
an existing idle window. Checkpoints account elapsed time without resetting idle
or draining the pending total. Clock regression and overflow leave state unchanged.

Only the accumulated duration may cross a process/reboot boundary: restoring it
starts paused without saved monotonic timestamps or inferred time away. Tests
simulate SQLite checkpoint/reopen with fresh clock epochs. Android now supplies
monotonic samples through the retained driver; only accumulated durations persist.

`pending_reading_time` keys a live document, chapter and exact UTF-8 byte offset
to nonnegative milliseconds and a positive signed revision. It cascades on live
document deletion. Attribution uses the exact saved reading offset where the
accumulator began; a page-size change keeps that offset and duration intact.
Backward navigation checkpoints the departing anchor before changing position
and leaves its time pending. Restore only the destination anchor's accumulator;
do not seed it with the whole page total. That total may include other anchors.

`load_pending_reading_time` validates ownership/scalar boundaries and reserves a
zero row if absent. `SavedReadingTime` is a pure internal protocol token containing
the anchor, previous duration and revision. `save_pending_reading_time` saves an
absolute nondecreasing duration only at the current position. A successful save
increments revision; an exact immediate retry returns the same result without
writing. Conflicting/older tokens error. Only duration, never monotonic timestamps
or idle deadlines, is persisted. Checkpoints never create reading/status events.

`pending_page_time` sums anchors in a derived logical page with checked signed
arithmetic. Boundaries belong to the following page, except EOF belongs to the
final page, matching pagination. Reflow neither duplicates nor proportionally
splits durations: completion credits each anchor once to its containing page.
The worker must checkpoint first, then freeze this total with the command ID and
timestamp, and stop writing that departing accumulator while completion resolves.

`record_page_completion` compares the frozen total against durable buckets inside
its immediate transaction. It inserts `active_time_ms`, zeroes only that page's
buckets, advances their revisions and writes destination position in one commit.
Consumption skips one revision so even a zero-duration checkpoint retry cannot
mistake completion for its own successful write. Zero rows remain until document
deletion to reject stale checkpoints after reread (no revision reset/ABA). A new
accumulator must load a fresh token after completion. Failures roll everything
back. Duplicate completion IDs compare the stored duration and never consume
new reread time or move position, even after content deletion. Historical v1 rows
receive zero active_time_ms on upgrade; history is retained on document deletion.

Core/worker/FFI now expose explicit RestoreReadingTime and anchored absolute
CheckpointReadingTime intents. The shell receives only ReadingTimeUpdate
(document/position/duration), without the revision. Core retains the previous
SavedReadingTime and frozen checkpoint across failures; RetryReadingTimeCheckpoint
resubmits them unchanged. Newer updates during an unresolved checkpoint coalesce
to the maximum same-anchor duration. Each successful save chains the latest queued
value with the newly returned revision. Reader exit/position-only movement waits
for this drain; one deferred departure is retained unchanged through failures.
Other storage commands remain blocked until success. Wrong document/offset and decreasing durations are
ignored; negative durations fail Serde decoding at FFI. A move clears the restored
token; reflow and same-offset saves preserve it. A cancelled restore or late save
cannot attach time to a closed reader. Snapshot loading does not reserve time:
explicit restore reserves the zero row only when the active Android driver needs it.

ObserveReaderClock accepts a document/exact-position identity, nonnegative signed
monotonic milliseconds and Resume/Interact/Checkpoint/Pause action. It drives the
pure accumulator only after restore; Checkpoint/Pause submit increased absolute
durations through the same transport. Regressions, foreign identities and samples
after departure/completion acceptance are ignored. Reflow/same-offset writes keep
the accumulator; movement/completion/exit retire its epoch. No timestamp is saved.
The existing maximum-duration slot can now also wait behind unrelated nonmoving
storage. Success drains it before departure/completion; failure keeps it visible
as reading_time_save_pending and RetryReadingTimeCheckpoint can submit it. Newer
absolute updates can supersede an unsubmitted slot without chaining an older value.
Samples behind already accepted movement/import are rejected: the platform must
pause and checkpoint before submitting those operations.

The platform must restore, send Resume when ready, send interactions/periodic
checkpoints and Pause before settings/background/departure/completion, then await
both pending flags and storage_busy before destroying the native session. Core
does not queue arbitrary busy operations or read a clock itself.
Android CoreSession supplies elapsedRealtime samples after successful restore,
15-second Handler checkpoints, interactions and final Pause observations before
settings/background/departure/completion. It waits for storage, both pending flags
and any direct file cancellation call before native close, retaining exact failed
save obligations for retry. Cancellation state remains in process only. No platform
clock timestamps enter SQLite. Checkpoint operations create no completion, encounter
or status events. Analyzed completion intents are connected below; post identity
remains open.

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
while OpenReader adds the page projection and finished-book reset described above.
ImportSummary is separate transient confirmation state containing the committed
document ID/title and Created/Existing outcome. DismissImportSummary clears it;
Settings and refresh preserve it, while tab selection, opening paste/reader or
accepting a new import clear it. It is never persisted or restored in a new core.
Listing is allocation order until the planned Library sorting task. Failed opens
can retry. No path or SQLite diagnostic string is exposed in application state.
The bridge constructor takes the path separately; Android uses filesDir/yomoku.sqlite3.
Submitted writes finish even if the session closes; shutdown does not block UI.

## Analyzed completion intent transport
CompleteReaderPage includes the visible document/page/exact position and a nonnegative
caller-supplied Unix timestamp. Core pins the preset and stored AnalysisId at acceptance,
rejects stale page callbacks and already-departed EOF, and drains accepted checkpoints.
MissingAnalysis is explicit: readable legacy documents gain no inferred encounters.
prepare_page_completion validates the current position/page/preset, reserves one command
ID and reads all durable buckets belonging to that page in one immediate transaction.
No history, status, time consumption or position change occurs in preparation. Core
then retains the exact AnalyzedPageCompletion (ID, timestamp, page total and analysis)
through failures and RetryReaderCompletion. Other storage mutations are blocked;
reader exits/position-only moves wait, retaining the first requested departure.
record_analyzed_page_completion performs the existing migration-0010 atomic write.
The worker then loads the persisted document for reader/Library reconciliation. A
failed post-commit load retains the same command: replay returns AlreadyRecorded,
never rewrites position/time/history and produces no duplicate notification. A new
Recorded outcome alone may expose CompletionNotice(command_id, newly_known), only
when positive; dismissal, new completion or reader exit clears this transient state.
Completion retires the reader's consumed time token; another anchor/reread restores
its own durable duration. Rereads reserve a new command. No new schema or dependency
version; Android automatically supplies clock/final saves and drains pending core
obligations before native teardown. Android controls/swipes use this completion
path; the toast consumes its transient command-keyed notice once. Migration 0011
adds the permanent finish marker; its checkmarks and reopening behavior are wired.
Final-page Finish Book/swipe success returns to Library after reconciliation and
clock drain. A separate transient book_completion_notice contains the newly Recorded
command ID, including zero-newly-Known rereads; AlreadyRecorded exposes none. It is
independent of the permanent marker and positive-only toast. Compose acknowledges
the matching ID before a local 1.4-second confetti animation, so recreation cannot
replay a consumed command. Neither animation nor its notice is persisted.

## Required future schema constraints
- Future import repositories must extend the identity/immutable-text and durable
  chapter/byte-offset contract above to multiple chapters.
- Add analyzer dictionary-form links (nullable) to global vocabulary and wire
  registration/status repositories into typed application/import effects.
  Reading never participates in identity.
- Document deletion removes content/progress but preserves historical provenance,
  encounters and global vocabulary. Reimport creates fresh document progress.
- Vocabulary deletion removes its encounter/status history, recalculates statistics
  and does not trigger rescans. Future registration starts New.
- Derive statistics from complete analyzed events and their occurrence/status rows;
  legacy events without analysis markers have no inferred encounter data.

Refine schema with executable migration/transaction tests before claiming storage
or accurate historical statistics are complete.
