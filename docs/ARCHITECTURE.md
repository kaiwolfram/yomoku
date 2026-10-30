# Architecture

## Implemented
`yomoku-core` is a pure Crux 0.20.0 application. It owns Library-default
navigation, persisted page-length/font-size/line-spacing Settings, storage busy/error state, Library
summaries, an optional ordered-document snapshot and Main/Paste/Reader content routes.
OpenPaste/CloseDocument and SelectDestination/OpenSettings/CloseSettings
remain synchronous. RefreshLibrary/CreatePaste/LoadPaste/SaveReadingPosition/
SetPageLength/SetReaderTypography emit StorageOperation; only one storage command is accepted at a time, while navigation
stays available.
OpenReader has a separate storage operation that atomically resets finished books
to page 1, retaining the permanent marker and pending durations. It tracks the
requested document until its callback; leaving the route cancels opening without
cancelling submitted I/O, including an accepted reset. LoadPaste stays read-only.
Successful paste, EPUB, TXT or SRT import returns to Library without opening the reader.
StorageFinished is internal: both Serde and Facet skip it, so hosts cannot inject
storage results. No SQL, paths, threads or platform I/O enter core.

`SearchLibraryTitles` synchronously updates a session-local query while Library
is visible, including during an outstanding refresh/import. The model retains
all Library summaries; `ViewModel.documents` projects matching titles in the
selected sort order, alongside the literal query and unfiltered document count.
Matching uses locale-independent Unicode lowercasing and literal substrings;
content is never searched. Refresh/import callbacks update the full collection
before projection. Search neither dispatches storage nor clears errors, and
survives Settings/tab/reader navigation and retained Activity recreation. A fresh
native session starts with an empty query. Compose binds the field/clear action
to typed events and distinguishes an empty Library from zero matching titles.

`RequestDocumentRename` opens an editor for a visible Library ID. Pure core owns
its literal draft and separate retry error; edits/Cancel are synchronous and
retained across Activity recreation. `SaveDocumentTitle` freezes its ID/title on
the existing typed storage FIFO, rejects doubles during I/O and reconciles only a
matching result. SQLite changes only the live title in one immediate transaction,
without dictionary/content reads or other writes. Same-title retries write nothing.
Success reprojects the full Library under the retained query/sort and updates any
matching import summary. Word distributions stay valid. Navigation clears the
editor; accepted I/O completes. Android gates rename during imports/unresolved
saves, disables blank submissions and offers explicit Save/Retry/Cancel.

`SortLibrary` synchronously selects Last read (default), Title or Import date,
including during an outstanding storage operation. The selection lives in the
owning core session alongside search and resets to Last read in a new session.
Pure `library::LibrarySort` filters/case-folds titles once before sorting; ties
use descending document IDs. Import date uses committed import allocation order
(newest first), independent of device wall time. Last read puts known recency
first, newest first, then unknown/unopened books by newest import. Title uses
ascending locale-independent Unicode lowercase text without normalization.

Migration 0019 adds a nullable document read sequence and a singleton clock.
Successful explicit reader entry increments it in the same immediate transaction
as snapshot validation and any finished-book position reset. Legacy recency stays
unknown rather than being inferred from position/completion events. Ordinary
loads, movements, reflow, lookups, imports (including duplicates) and sort/search
never advance recency. Snapshots/Library summaries carry it through generated
Kotlin; position-only core updates preserve it. Reopen restores ordering without
loading a dictionary. Clock/write/validation/commit failures roll back entry and
recency together; exhaustion fails closed. Android renders the current sort in
an accessible menu and uses the existing stable document keys.

Library word bars use a separate `LoadLibraryWordDistributions` read through the
existing FIFO/shared busy gate. Core retains compact per-document five-status
counts and a separate explicit-retry error. Android requests missing counts only
while Library is composed, pending saves are drained and no file batch is active;
search/sort/recreation reuse them. Imports, global-status observations, completion,
list refresh and
return to Library invalidate the snapshot. One SQLite statement deduplicates
exact Japanese surfaces across every chapter's first stored B before joining
live vocabulary. No analyzer, source/assets, registration, writes or token cache
is involved. Deleted entries are absent; Missing B and zero counts differ.
The legacy repository retains B membership; production selects the effective
repository below, matching reader and completion eligibility.
Android draws the five proportional colors and exposes exact accessible counts.

`Storage::effective_library_word_distributions` is the separate storage read for
the coordinated effective-reader switch. One deferred transaction covers live
documents, canonical chapters, pinned B/same-profile A/C, winning rules and global
statuses. `EffectiveReaderChapter::eligible_surfaces` borrows surfaces using the
same eligibility predicate as page tokens/completions; no full-page token copies
or guessed manual metadata. Deduplicate exact spellings across the document,
then use indexed live vocabulary reads. A transaction-local map shares each
referenced spelling's validated status/absence across documents in that one
snapshot; it is discarded on success/failure and every refresh reads afresh.
It never loads source archives, image
assets or dictionaries, registers entries or writes. Resolve one chapter at a
time; host measurements and their corpus limits are in
EFFECTIVE_READER_PERFORMANCE.md. The production effective app routes the existing
Library event to native-only `LoadEffectiveLibraryWordDistributions` on that same
FIFO, returning the existing compact result. Shared cache/visibility/busy/time/
mutation gates, separate error/explicit Retry, rename drafts and frozen deletion
prompts are preserved. NativeBridge selects Model::with_effective_reader; the
legacy B repository remains independently testable. Neither operation can load dictionaries.

`yomoku-import` is a separate pure content-extraction boundary. Its EPUB parser
reads Stored/Deflate ZIP entries from a borrowed byte slice, without filesystem
extraction/network access. Container/OPF metadata selects the first package and
spine order; each nonempty XHTML/SVG spine item stays a separate chapter. XHTML
ignores CSS/scripts, collapses ordinary whitespace, retains paragraphs/pre/breaks,
and projects headings/emphasis/links/ruby and image placeholders as canonical
UTF-8 ranges. Only website links are tagged external; internal/other links stay
inert. Referenced image/cover bytes are retained once, without decoding/caching.
Parser budgets bound source/read expansion/entries/nodes; a nonrecursive XML
preflight caps element nesting at 64 before the recursive tree parser. Reject
custom entity declarations, invalid numeric references and encrypted used content;
unused obfuscated fonts/CSS need not be opened. See DATA_MODEL.md and DECISIONS.md
for the extraction contract and current compatibility limits.
EPUB's repository adapter now saves original-file SHA-256/bytes, metadata/filename,
ordered canonical chapters, ruby/formatting/link/image spans and separate image
bytes in one immediate transaction. Each chapter gets an original validated Mode B
snapshot and globally unique registration through the shared analyzed-chapter
composition; the whole file rolls back if any chapter fails or cancellation wins.
Migration 0014 preserves existing source IDs/high-water marks/file identities.
Read-only multi-chapter loading validates digests, counts, indexes, ownership and
UTF-8 anchors without parsing or analyzing. Library progress sums earlier chapter
lengths plus the current chapter offset against total book bytes.
The pure `ReaderDocument` now retains every canonical chapter in source order.
`DocumentReaderSnapshot` adds the saved preset, global typography and each chapter's original Mode B
identity. Generic read/open/position repositories use the authoritative saved-content
loader; ordinary reads never parse/analyze or reset a finished book. Explicit
opening atomically resets a finished book to its first chapter. Logical page
projections count all pages while keeping anchors and time chapter-local.
Existing completion/time/token/selected-word repositories now use the whole-document
loader. Forward completion moves to the next page/chapter or the last chapter's
EOF. Only the last page of the entire document marks it finished; transaction,
replay, original-token ownership and New-only status semantics remain unchanged.
The loader currently verifies all EPUB content/assets on every reader operation;
host and opt-in Android JNI/FIFO measurements are in EFFECTIVE_READER_PERFORMANCE.md.
Optimized Android, peak memory and diverse large-Library costs remain unmeasured.
Reader snapshots omit source bytes/image
payloads; keyed chapter formatting now travels with canonical text.
Pure `ReaderChapterFormatting` validates owned chapter keys, scalar ranges, heading
levels and HTTP(S) targets. `ReaderPage` clips these spans to page-local UTF-8
ranges without changing text, token anchors, pagination or saved positions.
The storage adapter omits inert internal/other links from the display projection.
Compose converts scalar boundaries strictly to UTF-16, renders heading/emphasis/
strong styles and sends website links through the external URI handler. Website
tap areas take precedence over dictionary taps while preserving vocabulary
backgrounds; the remaining word fragments retain the full token identity.
Malformed nested source links are split into disjoint click targets in source-span
order. CSS, internal navigation and TOC jumps remain absent.

Reader text uses the bundled, unmodified Noto Serif JP variable font (version
2.003), pinned by source revision and SHA-256 in `third_party/noto-serif-jp/`.
Compose `readerFontFamily` maps regular/bold to explicit 400/700 weight axes of
one offline resource; EPUB emphasis synthesizes italic (no italic axis). The
shared ReaderTextBlock applies it to plain/tokenized pages and every text fragment
around inline images. Missing glyphs retain Android fallback. Material UI and
image-decoder SVG text keep their existing fonts. Font metrics affect visual line
wrapping/scroll height; core logical page boundaries, canonical text/UTF-8 ranges
and saved positions are unchanged. Settings exposes complete offline attribution
and SIL OFL 1.1 in the existing retained credit dialog.

Global `ReaderTypography` carries finite font-size/line-spacing preferences and
a default-off furigana boolean through typed Settings events, the FIFO storage
protocol and generated Kotlin.
Migration 0015 adds constrained numeric columns to the singleton `reader_settings`;
default 16sp/150% preserves the former 16sp/24sp body text. Library startup and
consistent document snapshots restore committed values. Core rejects unknown,
unchanged, busy or pending-save changes; failures retain the committed preference
until an explicit retry. Compose applies the selected font size and proportional
line height to all reader fragments, preserving system font scaling, Material
letter spacing, status colors and relative EPUB headings. It never repaginates,
changes text/position/history, or changes image asset keys/sizing rules. Settings controls stay
disabled until a successful load and while saves are pending. Migration 0016
persists the constrained furigana boolean in the same atomic preference update;
font/spacing controls preserve it. The typed preference loads with Library and
reader snapshots. Pure core projects `ViewModel.reader_furigana` when enabled:
sorted page-local UTF-8 ruby, preferring source bases over every overlapping
original Mode B token and using available whole analyzer readings elsewhere.
Vocabulary status/absence never filters readings. Full document/page/analysis,
token ownership, clipped range and canonical surface validation rejects stale or
malformed analyzer snapshots as a whole while retaining independent source ruby.
Projection performs no I/O, does not change text/positions/history and follows
existing token refresh/reflow/departure invalidation. Readings stay literal and
complete across page boundaries; no proportional reading split is inferred.
Settings exposes a labeled, whole-row furigana switch with the same commit/retry
and busy gates. Compose validates UTF-8 ranges before converting to UTF-16 and
clipping to text fragments around images. ReaderRubyText keeps the canonical
AnnotatedString as the sole semantic/gesture text and paints half-size readings
above the bases. Bottom-aligned, untrimmed line height reserves measured reading
space plus the larger of chosen spacing and natural styled font metrics. Complete
readings repeat at visual line/image splits; long readings compress horizontally
to their base width, preventing adjacent ruby collisions without changing wrapping.
Layout/paint caches follow text, typography, density, theme and projection changes.
The disabled/empty path retains the ordinary Text layout.

`ReaderDocument.images` carries keyed `ReaderChapterImages` with source-ordered
placeholder ranges, alt text and document-scoped `ReaderImageAsset` paths/media
types. Core validates unique owned chapter keys, exact U+FFFC scalar ranges,
nonoverlapping source order, canonical container keys and consistent media types
for shared assets. `ReaderPage.images` selects whole placeholders and translates
their ranges into page-local UTF-8 without changing text or position. Storage
resolves metadata from the fully verified saved asset map; image BLOBs stay out
of core/ViewModel/JNI snapshots. Paths are opaque container keys, never platform
filenames or URLs. `NativeBridge.loadEpubImageAsset` accepts a positive document
ID plus the path/expected media type and a one-shot typed listener. It enqueues
a read on the existing SQLite FIFO without a core event, Render or dictionary
load. Image bytes never enter core state. The repository checks live EPUB
ownership/metadata, the selected BLOB's SHA-256 and expected media type in one
read transaction. SQL gates its copy at 32 MiB (the import per-entry budget).
Unlike ordinary reader loads, this narrow read does not copy/rehash original
source bytes or unrelated assets. Missing keys/deleted documents, corruption and
unavailable storage remain distinct responses; invalid request identities fail
before enqueue. Reads do not repair or write anything.

`NativeBridge.load_epub_cover_reference` uses a separate one-shot
`EpubCoverListener` on the same FIFO. It returns the stored cover's opaque path,
media type and 32-byte SHA-256 without copying source/chapter/image BLOBs or
loading dictionaries. The read checks live EPUB metadata, declared-asset presence,
path/media/digest shape and the 32 MiB asset limit. Absent/deleted/non-EPUB IDs and
books without covers return `on_no_cover`; damaged live cover metadata returns an
error. The digest advertises a thumbnail cache key; it does not verify
pixels. A cache miss must fetch bytes through the verified asset API before
decoding/publication.

CoreSession also owns a main-thread EpubCoverLoader and a 4 MiB/64-entry memory
LRU keyed by exact media type plus SHA-256. Lazy Library rows resolve live cover
metadata on every new display request, including hits, so missing/deleted/non-EPUB
documents cannot reuse an old document-ID entry. Misses use the same serial image
pipeline as reader pages, with <=16,384 pixels (64 KiB ARGB) per cover and an
additional off-main SHA-256 check against the frozen reference before decoding.
This closes a metadata/asset-read race; new bytes never populate an old digest key.
Only successful decodes enter the LRU. Content can share pixels across document
IDs/paths; there is no negative cache or disk cache. Recreation retains the LRU;
process restart rebuilds it from verified saved assets. Eviction/close drop
references without recycling published bitmaps, which Compose may still hold.
The retained-cache budget excludes visible/evicted images awaiting GC.

Library covers fit a 48x64dp frame at their source ratio. Loading, absence and
failure use a decorative generic document icon; title/progress/finish and the
whole-row OpenReader action remain available. Errors do not become core storage
errors or mutate progress/history. Disposing a row removes queued work, cancels
image delivery and drains accepted metadata callbacks; returning to Library or
redisplaying a disposed row retries failures. Cover metadata and image work share
the existing native FIFO and load no dictionaries. No new core/FFI/schema API.

The per-request listener runs once on the FIFO worker and drops after delivery;
accepted reads drain even after bridge closure. Callbacks must remain nonblocking.
Android's retained CoreSession owns an EpubImageLoader with one decode executor.
Its pipeline allows one encoded asset read/decode at a time across page changes;
callbacks only hand off work, leaving the Rust FIFO free for persistence. Each
page decodes distinct assets once and shares bitmaps across repeated placements.
The 4,194,304-pixel page budget is divided among distinct assets (16 MiB ARGB),
with a 2048-pixel sampled edge limit. BitmapFactory reads bounds first, rejects
sources above 32768 pixels per edge/100 megapixels and power-of-two samples before
allocation; actual allocation is checked too. Dimensions retain the source ratio.
Verified `image/svg+xml` assets select AndroidSVG 1.4 on that same executor.
Before library parsing, a streaming XML preflight caps input at 2 MiB, 4096
elements, 64 nested/reference levels and 32768 expanded nodes. It rejects DTDs,
processing instructions, active/unsupported elements, patterns/masks/markers,
duplicate IDs, missing/nonlocal references and recursive/explosive reference
graphs. Color/class SVG styles are supported for vector-only documents;
resource-valued rules must be inline so their owner participates in the graph.
Entities are disabled. A private in-memory resolver exposes only predecoded images;
its font/CSS methods return null and it performs no resource I/O.

Embedded `<image>` resources accept base64 PNG/JPEG with matching decoded media,
or self-contained base64 SVG, with valid source bounds and explicit width/height.
One preparation context deduplicates up to 64 exact URI identities across all
nested documents. Child SVGs retain separate IDs/styles, undergo the same preflight
and geometry checks, and render before their parents using already-decoded private
keys. The entire graph shares <=2 MiB of root plus decoded child XML, <=4096 nodes
including generated wrappers and <=32768 expanded nodes, with <=8 image-resource
nesting levels and <=64 element/reference levels per document. Repeated images
reuse rendered child pixels instead of multiplying child vector expansions.
Bounds-only raster inspection and complete graph preparation precede allocation.
Preflight rewrites their references to generated private keys so AndroidSVG never
sees a data URI (its built-in decoder bypasses sampling). Original image viewports
become group/SVG/image wrappers, preserving source aspect ratio even when sampling
rounds a raster to 1x1; wrapper nodes count toward expanded-node/depth limits.
Transforms, inline styles, local uses and clipping remain on the existing graph.
The combined temporary allocation is <= the asset's output pixel budget (in
addition to that output); each temporary raster/vector bitmap obeys the 2048-edge
limit. All temporary bitmaps recycle on success/failure, and the resolver
deregisters in `finally`.
A process-wide SVG decode lock covers every session because the upstream resolver
and entity settings are static. Relative/local-fragment/file/network references,
image children, ambiguous hrefs and SVG stylesheets accompanying embedded images
retain alt/Retry. The last restriction avoids changing selector matching when
image viewports are rewritten. Basic SVG support is sufficient for v1; advanced
resource/style support is outside scope (user clarification 2026-10-27).
Validated intrinsic dimensions or the viewBox determine the source ratio, with
the raster source-dimension limits; Canvas renders directly into a bitmap within
the same edge/page-pixel bounds. Fractional source sizes stay separate from
integer bitmap sizes, through inline layout and viewer geometry. Bitmap budgets
bound retained page pixels, not temporary renderer layers or total process heap.
Disposed page requests release their delivery closures, remove queued work and
suppress late decode/delivery. Close cancels the queue and shuts down the executor;
accepted native callbacks still drain. Only unpublished discarded bitmaps recycle
explicitly; Compose/RenderThread may retain already published images until GC.
Compose replaces verified whole U+FFFC scalars with images between slices of the
existing AnnotatedString, retaining clipped styles and full dictionary identities.
Inline annotated text disables minimum-target expansion locally, preventing short
website links from capturing nearby inert text or other vocabulary glyphs. Link
semantics remain available for accessibility; other controls retain normal targets.
Normal images fill the text column at source ratio; extremely tall images shrink
within an 8192-physical-pixel frame while retaining ratio. Canonical text/anchors,
pagination, time and counts do not change. Unsupported/corrupt raster or SVG bytes
show source alt text and an explicit Retry without changing reader storage errors.
Platform BitmapFactory raster formats and bounded static SVGs are supported.
Tapping a decoded image opens
the source-agnostic Compose ImageViewer, reusing the same bounded bitmap without
another asset read/decode or a larger allocation. A separate full-window Dialog
isolates pinch/pan from reader completion swipes and routes Back to modal dismissal.
Image taps inside source website links open the viewer; text links retain their
browser behavior. Controls respect safe drawing insets; the canvas clips drawing
at source ratio. Zoom is bounded to 1–8 times fit; pan clamps each oversized axis
to its edges, centering axes smaller than the viewport.

Compose saves the selected placement plus zoom and normalized source focus through
Activity recreation/rotation, holding the modal while page assets reload. The
document/page anchor and selected placement key that state; changing page anchors,
leaving the reader or closing the viewer disposes it. Closing/reopening starts at
fit. Canvas/button/key input in the separate window resets reader idle accounting.
The shared viewer accepts an ImageBitmap, source dimensions and loading/error/retry
callbacks, with no EPUB, storage, networking or core dependency. Future displayed
Nostr images can use it. Advanced SVG resources are outside v1 scope; source ruby
transport, source-preferred analyzer fallback and switch/layout are connected.
Settings ships AndroidSVG attribution and its full license offline.

Shared file preflight and TXT/SRT/EPUB Existing results now carry `LibraryDocument`
references (ID, title, whole-book progress, permanent finish marker). Exact-byte
identity and complete saved-document validation share one read transaction; the
import transaction repeats that authoritative lookup before parsing or analysis.
TXT/SRT can reference an existing EPUB without truncating chapters, loading Full,
resetting a finished book or modifying its first interpretation. Core Created/Existing
results also carry only Library references; imports never need reader content.
Application/FIFO readers now use these ordered-document repositories for loads,
explicit opens, position saves and completion reloads. The generated ViewModel's
loaded_document is ReaderDocument; Android still renders one ReaderPage at a time.
Core validates one keyed original-analysis state per chapter, retains those identities
for the reader session and selects the visible chapter's identity on movement.
Previous uses whole-document page order; reflow preserves exact chapter/offset.
Only the last chapter's final completion returns to Library/celebrates. Invalid
content fails without truncating to chapter zero or loading Full.
`Event::ImportEpub(EpubImport)` carries original bytes and display name through the
same reader-save gates/FIFO/JNI as TXT/SRT, without a text-encoding choice. Shared
preflight references any earlier format before name/container checks or Full loading.
New containers pass the bounded parser before loading the worker-owned analyzer;
the atomic repository repeats the duplicate lookup/parse, analyzing every chapter.
Invalid/unsupported/over-budget containers produce `InvalidEpub`; engine/storage
failures retain their existing errors. Created projects the committed whole book
into a Library summary; no reader auto-opens. Direct per-file cancellation reaches
queued/load/chapter work and uses the existing commit gate. Background release,
engine-failure eviction and later-file retries share the existing analyzer cache.
Android mixed selection, reader text formatting/website links and raster/static
SVG images with the shared viewer and Library cover thumbnails are connected;
source-preferred furigana and its Settings switch/layout are connected; advanced
SVG resources are outside v1 scope.

Android URI reads own a private `ByteArray`. `FileImportBytes` exposes it as the
existing read-only `List<UByte>` required by generated typed events without
retaining one boxed object per byte. Encoding choices and exact retries share that
buffer; callers must not mutate it. Serialization still writes the identical
full-byte event. This avoids the demonstrated Java-heap failure on a 17.9 MB EPUB;
it does not introduce streaming, a new size limit or a changed native import API.

Its SRT parser
accepts decoded text and returns canonical LF text plus ordered UTF-8 cue ranges.
Blank lines separate cues; missing counters/separators and short/dot timestamps
recover without merging cues into prose. Strip identifiable timing/counter lines
and common HTML formatting, preserving speakers, numeric dialogue, unknown markup
and damaged metadata. Decode entities once after tag removal. All cues form one
text unit before analysis/pagination. Its repository adapter now uses the shared
strict UTF-8/Shift-JIS decoder without Aozora transformation. `import_analyzed_srt[_cancellable]` saves original bytes,
filename, canonical chapter/cues, original Mode B and unique registrations in one
immediate transaction. Shared original-byte identity references an existing TXT/SRT
before decoding/analysis; duplicates preserve their original interpretation.
Migration 0013 widens source kinds and adds cascading cue ranges. Read-only source
loading validates the digest, contiguous indexes, scalar ranges and exact separators.
Cancellation uses the existing per-file commit gate, discarding partial rows/IDs
while preserving earlier files. No reading events or time arise from import.
The ready-analyzer/no-I/O/provisional-ID contract matches TXT; see DATA_MODEL.md.
`Event::ImportSrt(SrtImport)` now carries original bytes/display name/optional
encoding through the same reader-save gates and FIFO/JNI worker as TXT. Its
read-only shared-file preflight bypasses Full for duplicates; new input is decoded
and parsed before loading Full, then checked again in the authoritative transaction.
SRT-specific errors distinguish uncertain/invalid/unsupported/undecodable/empty
input; malformed timing with retained dialogue remains valid. All file formats reuse
one worker-owned analyzer cache and one published per-file cancellation registry.
Android mixed-format selection uses the retained FileImports helper described below.

TXT strict UTF-8/Shift-JIS decoding returns explicit uncertainty before any
canonicalization; a selected encoding never silently falls back or replaces errors.
The Aozora parser produces one canonical text unit plus ordered source-ruby UTF-8
ranges. It hides common typographic directives, retains unknown/semantic/gaiji
notations and preserves every hard break without chapter inference. The parser does
not allocate durable IDs or perform I/O/analysis/SQL. Storage now consumes it in
`import_analyzed_txt`: hash original bytes, lock one immediate transaction, reference
an exact live file or parse/allocate/store original bytes, filename title, canonical
text/ruby, Mode B analysis and unique registrations together. An existing file skips
decoding/analysis and preserves its chosen encoding/statuses/progress. The analyzer
callback obeys the same ready-worker/no-I/O/provisional-ID contract as paste. Shared
legacy single-text repository APIs retain PastedDocument; the application/bridge
uses ReaderDocument for every format;
source ruby also retains its separate original-source repository. TXT source,
EPUB content and TXT reader loads share ordered-index/range/reading validation.
ReaderDocument now transports sparse keyed ReaderChapterRuby and ReaderPage clips
source bases into page-local UTF-8 ranges. Complete readings remain unchanged even
for crossing bases; no byte/character-to-reading alignment is inferred. Chapter
keys, scalar boundaries, nonempty readings and ordered nonoverlapping bases are
validated in pure core. No analysis/vocabulary lookup is needed; Missing analysis,
deleted vocabulary and all five statuses preserve source readings. Generated JNI
carries both forms. The default-off setting, source-preferred analyzer fallback
and visible ruby rendering share the canonical word/link tap targets.
`import_analyzed_txt_cancellable` adds a shared per-file `ImportCancellation` handle.
Checkpoints before/after parsing, ruby writes and analysis can discard provisional
rows; the last check and COMMIT share a mutex. Cancellation winning that gate rolls
back the entire file including IDs; a successful commit winning it remains completed.
The result is `Some(Created/Existing)` for completed work, `None` for cancellation,
or the usual typed error. Cancellation is cooperative, without interrupting active
decoder/analyzer/SQLite calls. The caller must request it outside the import FIFO
and off the UI thread because a COMMIT already in progress holds the gate. Use one
fresh handle per attempt; no persistent queue or cross-file cancellation state.
Production runtime now uses this cancellable entry point. The storage middleware
publishes a fresh attempt synchronously before FIFO submission, then clears that
exact attempt before resolving its result. `AnalyzerLifecycle::cancel_file_import`
and native `cancelFileImport()` clone the current handle under a short registry lock
and request cancellation directly, bypassing FIFO and core/bridge update locks.
The registry lock is never held during COMMIT, cancellation or callbacks. Weak
lifecycle ownership permits cancelling accepted work while a closed router drains
without keeping its worker alive. Idle calls do not poison future attempts.
The existing `cancel_txt_import`/`cancelTxtImport()` delegate to this shared control
for compatibility with older callers. Android uses the shared file control.
`StorageResult::ImportCancelled` clears core busy state without an error or summary;
Created/Existing and typed failures keep their existing behavior. Cancellation
does not forcibly stop a dictionary loader already running. Android invokes the
native method on a dedicated cancellation executor, outside the URI reader and
FIFO, and awaits both its return and the import outcome before settling a batch.
CoreSession keeps the native handle alive through queued/running cancellation,
including owner teardown, and shuts down that executor after draining close.
`Event::ImportTxt(TxtImport)` carries original bytes, display name and optional
UTF-8/Shift-JIS choice through generated Kotlin/JNI to the same FIFO worker.
A read-only hash/exact-byte preflight returns existing snapshots without Full;
new input is validated before loading Full outside the write transaction. The
repository repeats parsing and its authoritative duplicate lookup atomically.
Typed errors distinguish encoding choice, invalid selected encoding, unsupported
BOM, undecodable input and empty text. Successful Created/Existing use the same
Library summary without auto-opening. Busy/pending saves gate submissions; imports
invalidate selected words and Android pauses the reader clock before dispatch.
Android's retained `FileImports` sequences mixed EPUB/TXT/SRT picker URIs and
consumes committed per-file outcomes. `OpenMultipleDocuments(*/*)` supplies read
grants, including providers with generic SRT MIME types. One platform executor
queries literal display names and reads original bytes; case-insensitive extensions
select the typed ImportEpub/ImportTxt/ImportSrt event. Unsupported extensions produce
a per-file result before reading bytes. Only current-file bytes are retained.
TXT/SRT uncertain encoding offers UTF-8/Shift-JIS or Skip with format-specific
errors; EPUB never offers a text-encoding choice. Errors never abort later files.
Modal encoding/import state and scrollable batch summaries survive recreation,
returning to Library without opening a reader. Current filename and File X of N
remain visible during reads/imports/encoding choices. Batch progress is terminal
outcomes / selected URIs (including errors, duplicates and skips); prompts/retries
do not advance it. Display names publish before potentially blocked provider reads.
Cancel immediately clears unsubmitted URIs and encoding input, disables repeated
Cancel/encoding actions and shows Cancelling while native work settles. A winning
Created/Existing result is copied into the retained summary; cancelled partial
work adds no outcome. Earlier terminal results remain. Every provider name/error/
byte callback checks a read generation invalidated on Cancel/close. Cancel retires
the reader executor so a provider ignoring interruption cannot block a fresh batch;
its late data is discarded. Progress continues to count only terminal results.
No URI queue/results/progress are persisted;
final owner teardown stops unsubmitted work while core drains accepted transactions.
`ImportNotifications` observes only filename/current/processed/total and encoding/
cancellation state. A nonexported `dataSync` foreground service holds a process-only
lease while the existing helper/FIFO imports. A local binder delivers updates and
timeout cancellation; the service receives no URI, bytes, SQL or persistent queue.
Start the service for a user-started batch or encoding choice; never restart it on
background updates, boot or process launch. `START_NOT_STICKY` and no saved queue
prevent unfinished-batch auto-resume. Android still controls process survival.
Release the lease during encoding selection and post a separate input-needed
notification; resume only after explicit selection/Skip. Progress notifications
show actual processed files, including failures/duplicates/skips. Completion and
cancellation use a separate notification ID, retaining the same processed/total
semantics. Tapping returns to the retained app without opening a reader. A denied
notification permission never blocks import; the modal offers an explicit enable
action. Manifest permissions are POST_NOTIFICATIONS, FOREGROUND_SERVICE and
FOREGROUND_SERVICE_DATA_SYNC; no new dependency/schema. If Android rejects a service
start, continue subject to ordinary process limits without retrying it automatically.
Timeout stops foreground execution/service immediately, then uses existing off-main
cancellation/draining; a winning commit stays. Normal owner teardown removes the
lease and unfinished progress notice. EPUB/SRT share this helper and lease.

Pure document values define positive i64 identities, immutable chapter snapshots,
UTF-8 byte anchors, checked UTF-16 conversions and literal first-line paste titles.
Vocabulary values define exact-surface identity, five statuses, registration,
persisted entry snapshots with stable IDs and New-only completion policy.
Pure `analysis` validates immutable token snapshots against exact canonical text:
engine/dictionary revisions, A/B/C mode (default B), ordered nonoverlapping UTF-8
anchors, original surfaces and optional analyzer reading/dictionary form. Only
whitespace may be omitted between tokens. Separate global/local boundary edits
partition a selected exact span; they cannot edit lemmas or source text.
Application vocabulary commands remain future work.

Pure `reader_tokens_on_page` projects Japanese Mode B occurrences intersecting a
nonempty page span. Tokens retain full chapter anchors, exact surfaces, readings,
dictionary forms and original indices; a separate page-local UTF-8 display range
clips crossing tokens for rendering. Display overlap does not change completion's
start-based occurrence ownership. `ReaderTokenSnapshot` echoes document/page/base
analysis identity for asynchronous result validation. Its optional
`ReaderVocabulary` carries the current global ID and one of five statuses.
The separate `load_reader_tokens` repository reads original Mode B plus current
exact-surface statuses in one read transaction, using one visible-token status
join. It reuses full stored-analysis validation; no analyzer, override application,
registration, history, position or finish-marker mutation occurs. Legacy analysis
absence is explicit; deleted/unregistered vocabulary stays absent.

Explicit LoadReaderTokens(ReaderTokenKey) flows through Crux's serialized storage
gate, the existing FIFO worker and generated Kotlin/JNI. Core accepts only the
visible document/page/base-analysis key outside Settings, then attaches results
only when both the pending and current keys match. reader_tokens is transient:
refresh clears it, as do navigation, position/completion callbacks and reflow to
a different page span. Cancelled results/errors cannot reopen or contaminate a
departed reader. Legacy Missing is a successful empty snapshot. Reads never need
an analyzer, and failures can be retried by submitting the same current key.
This nonmoving read allows final time checkpoints to queue; even a failed read
drains the waiting checkpoint before deferred departure.

SetReaderStatus(ReaderStatusChange) shares the same gate/FIFO. Core accepts a valid
timestamp and existing vocabulary ID from the current loaded page outside Settings.
The worker calls the existing atomic status/history repository; no analyzer or
post-commit reload is needed. The echoed choice/result must match pending/current
identity before updating all visible occurrences of that ID. A deleted ID returns
absence and clears its visible references without registration. Same-status choices
are repository no-ops. Failure preserves the old snapshot for explicit retry by
choosing again; no background replay obligation is created. Accepted writes finish
after navigation, while cancelled results/errors cannot attach to the departed page.
Success and failure both drain queued final clock checkpoints.

Compose requests tokens when the visible key has no snapshot and the storage/save
gates are clear. A failed read stays explicit until Retry; Missing stays empty.
An attempted-read guard survives shared error clearing by queued clock saves.
ReaderWords checks echoed identity, ordered ranges, exact clipped surface text and
UTF-8 scalar boundaries before mapping them to UTF-16. Inline LinkAnnotation links
preserve the original text layout and expose keyboard/accessibility activation;
only Japanese token spans are linked. Full crossing-token identity opens from
either fragment. An invalid projection falls back to plain page text.
Product light/dark status backgrounds follow the system theme; Known and absent
vocabulary have no background. A phone bottom sheet or wide-screen side panel
shows stored analyzer metadata and one row of five direct choices. Selection follows
committed snapshot changes, including every repetition; pending storage/time/completion
disables choices. Failure keeps the panel and prior status for explicit repeat.
Snapshot invalidation, Settings, navigation and Activity recreation dismiss it.
Definitions and encounter counts remain separate future dictionary work.
The sheet/popup has its own Android window, so its pointer/key input explicitly calls
the existing CoreSession interaction observer. Reading continues while it is open;
interactions extend the same idle deadline. Annotated links are remembered across
timer-only recompositions, without caching old status or page identities.

`tools/prepare-dictionaries.py` independently compiles pinned JMdict English and
JMnedict XML into an immutable, versioned SQLite asset using only Python stdlib.
It preserves ordered senses/readings, restriction metadata, inherited POS and
separate name groups as JSON, with a BINARY exact-form index and full provenance.
Compressed inputs/notices are hashed before parsing; staged transactional builds
publish by directory rename without replacing existing output. The bundle retains
sources, complete attribution/licences/docs and reproducibility hashes/tool versions.
See `third_party/dictionaries/README.md` for the index contract/update procedure.
This tooling is separate from mutable user SQLite. Gradle bundles the index, all
notices and reproducible inputs through `tools/bundle-dictionaries.py`, verifying
the pinned build manifest and every staged file before immutable publication.
`yomoku-storage::dictionary::Dictionary` opens an existing
published asset with read-only SQLite flags (no create, URI options or migrations),
checks application/schema identity and performs bound BINARY indexed lookups.
It unions exact surface/base candidates, preserving the original surface, both
query-form provenances and source-scoped IDs in separate word/name collections.
Typed payloads and pure spelling/reading/sense selection live in core; no I/O does.
All upstream metadata stays intact. Applicable views reference ordered payload
indices and enforce re_restr/re_nokanji plus both stagk/stagr restrictions. They
keep every licensed reading rather than filtering by one analyzer reading.
Malformed/unknown fields and inconsistent references fail the whole lookup.
This connection is separate from mutable user Storage; reads never register or
record encounters. Android's lazy `wordDictionaryFiles` reuses the worker-only
`VerifiedDictionaryFiles` publisher with a separate no-backup directory and
content-addressed SQLite filename. `verifiedPath()` checks pinned size/SHA-256
before supplying a read-only path; Rust open checks format identity, not a checksum.
Repair atomically replaces inodes without modifying held readers. Library/Settings
never extract the index. Settings exposes complete offline credits/legal/project
documents through the shared notice dialog; HTML documents render as text, without
web requests. Visibility survives recreation. NativeBridge's DictionaryProvider
also exposes verified_word_dictionary_path (empty means failure), invoked only
on the existing FIFO worker's first selected-word request. Rust opens the separate
read-only connection there and reuses it until worker teardown. Open failures
remain retryable; dictionary query failures retire the cached connection so an
explicit retry can receive a repaired immutable publication. Imports and Library/
reader/token opens do not load this index; word lookup never loads an analyzer.

`Storage::load_reader_word(dictionary, key, token_index)` composes the existing
page-token validation/status projection with an indexed lifetime occurrence count
in one deferred user-state read transaction. It selects only a visible original
Mode B Japanese token and derives surface/base queries from that stored token.
`ReaderWordDetails` echoes the page/base key, full token, count and separate typed
word/name candidates with applicable views. Missing document returns None;
stale/missing analysis or invalid token selection errors. Dictionary failures
return a distinct typed error without partial details; empty matches are success.
Counts use the live exact-surface vocabulary ID, keeping inflections separate,
including repetitions/rereads/history from deleted documents and excluding
registration/pending time. Vocabulary deletion removes its counts; lookup never
re-registers it. No large-document performance claim is made.

Pure ReaderWordSelection(key, token_index) and ReaderWordDetails plus concrete
WordCandidate/NameCandidate and per-form match types are serializable/Facet values.
LoadReaderWord is accepted only for a token in the current loaded snapshot, outside
Settings and the shared busy/pending-save gates. Result attachment checks pending,
selected and current keys plus full token identity (allowing live vocabulary changes).
Live status/absence refreshes every visible repetition. Manual status results also
update selected details; deleted vocabulary clears its count without registration.
DismissReaderWord, Settings, navigation, token refresh, movement/completion and reflow
retire the selection. Late results/errors cannot restore it. reader_word_error is
separate from document_error, so clock saves cannot erase a failed lookup; retry is
explicit. Success and failure both drain waiting final time saves. The internal
StorageResult boxes details to bound the Event enum size; this does not change wire
encoding. All reads/callbacks stay on the existing worker and only Render reaches
Android. Compose requests details after busy/pending-save gates clear, once per
selection; failed requests require explicit retry. Its phone sheet/wide panel
renders the lifetime count, every applicable expanded sense with its licensed
readings/compact grammatical labels and a separate names section. Selection stays
local to the composition; closing/disposal dispatches DismissReaderWord, including
Activity recreation. Session teardown tolerates this dismissal after closing starts.
Core invalidation also dismisses the local panel, even at an unchanged page anchor.

Pure `active_time` accounts pending reading milliseconds using caller-supplied
monotonic timestamps. Resume/interaction arms a five-minute idle window; dictionary
interactions count, while background/reader-close pause. Checkpoints neither reset
idle nor drain time. Only a validated nonnegative signed duration can be restored;
restoration starts paused with no old clock epoch. Migration 0009's separate SQLite
repository saves only durations at exact text offsets, with revision tokens for
stale/duplicate checkpoints. Reflow assigns each offset to its containing page;
completion consumes that page's frozen total with history/position in one commit.
Core now exposes explicit RestoreReadingTime, CheckpointReadingTime and
RetryReadingTimeCheckpoint intents. Restore reserves the exact offset's token on
the FIFO; it is separate from reader snapshot loading. SavedReadingTime is a pure
protocol value validated against SQLite; only its anchored duration reaches the
shell view. Core freezes the checkpoint plus prior revision until success, blocks
other storage commands while unresolved. Newer same-anchor absolute updates during
that save coalesce to the greatest duration in one slot, without replacing the
frozen retry payload. Successful saves chain the coalesced update using the returned
revision. CloseDocument, OpenPaste, tab selection and position-only movement wait
for all accepted checkpoints; the first requested departure wins over duplicate or
competing callbacks. Failure retains both the queue and departure for explicit retry.
Cancelled restores cannot attach to closed readers. Reflow/same-offset saves preserve
time; moving to a different offset requires a fresh restore. No SQL, clock or
analyzer enters core. CompleteReaderPage captures the visible document/page/position,
preset, stored AnalysisId and caller-supplied timestamp, rejecting stale/duplicate
callbacks. It waits for every accepted checkpoint, then the FIFO atomically reserves
a command ID and freezes the whole page's durable duration. Core chains the prepared
analyzed transaction and retains each stage unchanged for RetryReaderCompletion.
Other storage and departures wait until success; the first deferred departure wins
for ordinary pages. Successful final-page completion returns to Library.
Only newly Recorded results with newly_known > 0 publish a dismissible command-keyed
CompletionNotice. AlreadyRecorded and zero results publish none. Successful results
refresh reader/Library progress from persisted content and retire consumed timer tokens.
Finality is the frozen page ending at immutable paste EOF, independent of the permanent
marker. Final success returns to Library; newly Recorded outcomes also publish a
separate command-keyed book_completion_notice, including zero newly Known. A keyed
acknowledgment consumes it before a local 1.4-second Compose confetti animation;
recreation cannot replay a consumed notice. Final return preserves the positive toast.
The worker's post-commit read can fail; retry uses the same command, reconciles the
persisted position and suppresses duplicate notices. No analyzer is loaded for completion.
Android supplies automatic final checkpoints and platform observations as described below.

ObserveReaderClock now drives the existing pure accumulator with anchored
Resume/Interact/Checkpoint/Pause samples after explicit RestoreReadingTime.
Only Checkpoint/Pause submit increased absolute durations; timestamps never reach
storage or the view. Missing/foreign tokens, negative/regressing samples, settings
interactions and samples after accepted departure/completion are ignored. Reflow
and same-offset writes preserve the clock; movement, completion and exit retire it.
An externally supplied greater saved duration resets the accumulator paused.
Final checkpoints may also wait in the existing maximum-duration slot behind
unrelated nonmoving storage (list, read, settings or same-offset position saves).
Success drains the slot before accepted completion/departure; unrelated failure
retains it for RetryReadingTimeCheckpoint. Late samples after an already accepted
position change/import cannot queue a checkpoint for the departing offset. The
pending flag includes waiting saves; this adds no general event queue.

`yomoku-analysis` is a separate Rust I/O adapter using Sudachi.rs v0.7.0 at
`5de0410b6edfd428fc510cc9763440772e9619d5` and Full 20261015 (V1). Its constructor
checks the exact dictionary SHA-256 before giving owned immutable bytes or a
read-only file mapping to Sudachi. The mapped constructor is explicitly unsafe:
callers must prevent backing-file mutation/truncation through analyzer drop.
It hashes the mapping itself and transfers that same storage into the dictionary;
the original file handle can close. The safe owned constructor remains available.
Pinned upstream embedded defaults supply normalization/OOV plugins and resources;
no external config or dynamic plugins are accepted. Default B and explicit A/C
produce validated ChapterAnalysis values, with original surfaces/byte spans and
resolved reading/dictionary forms kept separate. Japanese classification combines
original Hiragana/Katakana/Han script with Sudachi POS; OOV rare kanji remain
Japanese even when the fallback plugin labels them symbols. OOV forms/readings
remain absent. Punctuation, Latin, numbers and emoji stay anchored without being
vocabulary. SentenceDetector boundaries (including negative provisional limits)
keep long chapters below the engine input limit without changing canonical text.
The adapter does not merge conjugations, apply overrides, register vocabulary or
write SQLite. Eleven explicit Full-data fixtures include A/B/C SQLite round trips
and full owned/mapped output parity; tools/test-sudachi.sh runs all fixtures
with each loader, separately from ordinary data-free checks. Do not rewrite the
prepared dictionary while mapped fixtures are running.
Dictionary preparation verifies the archive, data and complete notices under
ignored .local/sudachi. Gradle's bundleSudachi task verifies already prepared data
and stages the dictionary plus complete LEGAL/LICENSE under generated Android
assets; it never downloads data. Settings reads and displays both full notices
off the main thread in a selectable, scrollable dialog, publishing the loaded
state explicitly on Android’s main dispatcher. The dictionary remains
compressed in the APK; no private-file copy or eager analyzer load occurs at startup.
An on-demand Kotlin platform helper can extract to noBackupFilesDir/sudachi, using
the pinned size/SHA-256 and a bounded streaming buffer. It rejects main-thread
opens and serializes publication across threads/processes. It fsyncs, verifies,
makes staging bytes read-only, atomically renames and syncs the directory before
returning the same verified descriptor at offset zero. Published inodes are never
opened for writes/truncation; repair preserves held readers/mappings. Cache hits
rehash bytes without APK decompression; abandoned staging/failures can retry.
The native import worker now calls the helper through DictionaryProvider, never
at startup. Eight extraction
instrumentation tests pass, including compressed Full and failed-rename recovery; two API-36
x86_64 runs take 2.273–3.273 s to extract/verify, 0.632–0.689 s to reverify. No analyzer
or combined-memory/lifecycle claim follows. See SUDACHI_MEASUREMENTS.md.
Production NativeBridge imports use the adapter below. A standalone native
example and CLI driver also measure it on Android without installing the app:
three debug and three release processes pass offline A/B/C/Unicode/long-input
validation on API 36 x86_64. Release initialization takes 1.03–1.20 s, with about
318 MiB anonymous resident memory after initialization and 334 MiB peak RSS.
See SUDACHI_MEASUREMENTS.md for exact methodology and limitations. The driver
verifies pinned source/data, temporarily disables radios, restores their original
settings and removes its own device files. Both probe ABIs link; physical arm64
remains unmeasured. Private extraction is measured separately below the standalone
results in SUDACHI_MEASUREMENTS.md.
The owned/mapped comparison reduces release initialized anonymous memory from
about 318 MiB to 2.5 MiB; total RSS remains similar after the complete checksum
read. File-backed residency alone does not demonstrate reclamation under pressure.
The later bounded pressure measurements below settle the emulator loading gate.
Unpatched upstream cross-target checking failed in plugin/loader.rs:
make_system_specific_name originally had no Android cfg implementation. The root
Cargo patch now selects a dependency-only vendored snapshot at that exact commit,
with Android added to the existing Linux/FreeBSD `.so` cfg. The complete patch and
all original file hashes live under third_party/sudachi; tools/verify-sudachi-source.py
reverses the patch in a temporary copy and verifies every original byte. Both
Android targets compile. The profile includes +android-loader-v1; segmentation
and embedded configuration are unchanged. Production uses this mapped loader below.
Owned dictionary bytes alone require about 315 MiB. Mapped storage avoids that
anonymous copy. The private extraction helper enforces immutable publication;
production consumes only verified immutable private descriptors. Keep one
on-demand worker-owned analyzer and avoid eager Library initialization or per-page
reloads. Physical arm64 testing is deferred to the user's usable-app trial.

The separate `android_app_probe` cdylib example is packaged only in debug APKs.
Its primitive JNI boundary duplicates the verified descriptor directly, rehashes
the mapping through the real adapter, and keeps one analyzer in thread-local
storage. Positive generation IDs reject stale/foreign-worker calls; errors stay
numeric and panics cannot unwind into Java. No Java object, Activity or JNI
interface pointer is retained. A/B/C and fixed Unicode/page/long-input fixtures
are the only analysis requests; no application/import effects are exposed.
Instrumentation owns an isolated no-backup directory and a lazy executor retained
by a test ViewModel. It verifies reuse over Activity recreation/stop/resume,
close/unmapping on that worker, and fresh-worker reopen. Test files are removed
only after all worker mappings drop. This measures Compose/JVM/native PSS/RSS
together; the test owner/executor is not a production analyzer architecture.
An ADB-only debug Activity (protected by the system DUMP permission) adds an
external process-death experiment. Its driver force-stops the app while Full is
mapped, confirms all app processes exit without graceful close, and launches a
new process against the same test-private publication. Two x86_64 pairs preserve
the inode/mtime/read-only permissions and pass real analysis after re-verification.
Cleanup runs only after confirmed process exit. This is force-stop recovery,
not an OS low-memory-killer or interrupted-extraction simulation.
The same debug Activity now accepts a pressure phase: after real analysis it
holds its mapping until the host requests fixture revalidation on the same worker.
An emulator-only NDK shell helper allocates bounded incompressible anonymous
memory; the driver samples dictionary-specific smaps residency and kernel reclaim
counters without cache dropping, madvise or synthetic trim signals. Two runs
reclaim 67/143 MiB and preserve process/generation and fixture correctness.
This closes the emulator loading gate; no LMKD-kill/hardware claim follows.
Production lazily loads one immutable mapped analyzer on the
existing FIFO worker, reuse it through configuration changes/foreground work,
and release its idle cache on UI-hidden/background and final session close.
Background release waits for in-flight atomic imports; later work reopens through
the verified publisher. No Android/SQL I/O enters core and no new executor is
needed. NativeBridge and Android now select this worker and supply verified descriptors
and Activity lifecycle signals. Physical arm64 testing is deferred
to the user's usable-app trial; see SUDACHI_MEASUREMENTS.md.
Release APKs exclude the probe Activity/facades and the probe library.

Pure `pagination` derives chapter-local TextAnchor pages from ordered immutable
ChapterText snapshots. Five presets count non-whitespace Unicode scalars; LF/CRLF
paragraph boundaries and following-sentence fallback are independent of display
size. Supplied chapters force boundaries; headings never infer chapters. A separate
ReadingPosition retains the exact UTF-8 offset through reflow, with ownership,
scalar-boundary and EOF validation. ReaderPage projects the page containing the
saved position at the persisted global preset (default 500), preserving the exact
offset separately from its page anchor/counter. These typed values cross FFI. The position-only
save hook updates the reader and Library percentage after commit; late saves
cannot reopen a closed reader. Explicit atomic forward intents are connected;
Android Next/Previous/swipes use the retained dispatch path, pausing the clock
before intent capture. Anchored PreviousReaderPage derives the preceding page in
Rust and uses position-only persistence after checkpoint drain. Forward uses the
existing frozen analyzed completion. Both directions reset scroll; controls and
the counter sit below content. Settings reflow keeps the exact position and Library percentage;
only committed values change the preset/page. An independent active-document
snapshot prevents repository loads from changing the text used for reader reflow.
Reader snapshots load content/position, permanent finish state, the preset and the
original stored Mode B AnalysisId together. Plain loads use a read transaction;
explicit reader entry uses an immediate transaction for its finished-book reset.
The earliest Mode B snapshot is pinned,
independent of the installed engine/dictionary or later snapshots. Only its
header is read; completion still validates tokens and anchors transactionally.
ReaderAnalysis::Missing explicitly identifies legacy content with no Mode B base;
it remains readable without analyzer initialization, rescan or vocabulary writes.
The optional view field is absent outside an opened reader. Settings/position
saves/plain loads preserve its identity; reader exit/replacement clears it, and
cancelled loads cannot attach it. Corrupt metadata returns a typed error without
fallback/repair. Forward intents return MissingAnalysis for legacy content, with
no fallback, rescan or history. Legacy analysis recovery remains undecided.

`yomoku-runtime` composes Core → HandleEffectLayer<StorageEffects> → MapEffectLayer.
A named FIFO worker owns one lazily opened SQLite connection per bridge. Opening,
migration and create/load/list/position/settings saves execute on that worker.
Failed opens can retry; database failures become typed DocumentError state without internal strings.
Only Render reaches ShellEffect. Dropping the bridge closes the work channel;
submitted work finishes atomically and the worker exits without joining on the UI
thread. Crux discards results when the session has gone away. There is no new
async executor or dependency version.

`effect_router_with_analyzed_imports` opts into the deduplicating repository path
on that same worker. A Send factory loads/verifies a ready `PasteAnalyzer` before
the write transaction; the analyzer itself need not be Send and is created,
reused and dropped on the worker. Library/settings/blank-paste paths never load it.
Runtime supplies wall-clock registration timestamps, distinct from active time.
Created/Existing results return only committed documents. Core upserts their
Library summary by ID, preserves saved progress, returns to Library and never
opens a reader. Core also projects a transient typed ImportSummary (document ID,
title, Created/Existing) with a dismiss event. Android renders the confirmation
above the Library list; Settings/refresh/retained recreation preserve it, while
dismissal, tab selection and new import/reader flows clear it. Fresh sessions do
not restore it. Analyzer/load/invalid-output failures can retry; engine/invalid
output discards the cache, while SQL rollback keeps a healthy analyzer reusable.
The weak `AnalyzerLifecycle` handle queues foreground/background policy alongside
storage work without extending worker lifetime. Hidden/background releases after
earlier atomic work, and imports accepted while hidden release before resolving.
Foreground permits lazy reuse without initialization. Final close drains accepted
work and drops both analyzer and loader. Thirteen data-free tests use a deliberately
non-Send analyzer; an explicit real Full fixture tests both loaders and lazy reopen.
NativeBridge requires this router. The original text-only `effect_router` and
`YomokuBridge::new` remain low-level Rust storage fixtures, never Android fallbacks.

`yomoku-storage` owns rusqlite 0.40.2/bundled SQLite, migration history and SQL.
Migration 0002 separates source/chapter provenance, Library documents and canonical
text. create_paste commits all four inserts atomically; load_paste reads a
consistent snapshot. Migration 0003 adds current document/chapter/byte offsets;
saves validate ownership/scalar boundaries under an immediate transaction. Listing
projects titles/current percentage in allocation order using canonical BLOB lengths
without reading all text. Migration 0004 stores the global page-length preset
in a constrained singleton. Immediate settings transactions validate existing
values and preserve them on failure; missing/corrupt rows return typed errors.
Migration 0005 adds durable completion command IDs and dated page history. Pure
PageCompletion validates the exact departing page/position; storage records history
and destination position atomically. Duplicate IDs require identical payloads and
never rewrite progress, including after reflow/reopen/deletion. Rereads allocate a
fresh ID; position-only saves never add history. Analyzed completion supplies the
typed core/runtime/FFI path. Migration 0010 supplies the analyzed transaction below;
Android checkpoints and forward controls are connected. Migration 0009 adds
active_time_ms to history and pending_reading_time buckets, scoped to live documents.
Checkpoint loads reserve a zero row/revision; absolute saves validate current
position and reject stale tokens. Completion validates the frozen page total,
zeros its buckets and advances their revisions atomically. Replays compare time
too and never drain later reread time. Only durations cross process/reboot epochs.
Migration 0006 adds global exact-surface entries and versioned, dated registration/
manual-status events. Registration preserves existing statuses; lookup and unchanged
selections write no history. Immediate transactions commit registration batches or
status/event pairs together. These repository APIs accept analyzer-supplied surfaces
and explicit timestamps; no SQL or clock enters core. Vocabulary-history FKs cascade
only on vocabulary deletion; document deletion preserves entries/history. Paste
imports now register analyzed surfaces atomically; vocabulary UI remains pending.
Migration 0017 adds a separate exact-surface custom dictionary table, independent
of vocabulary/source lifetime. Pure `CustomDictionaryEntry` validates nonblank
meaning/reading; selected-token storage operations reuse original Mode B ownership
validation and recheck surface/base against the read-only word/name index before
read/create/edit/delete. No built-in, analyzer, reader position/time or history
mutation occurs. Immediate writes roll back atomically; identical retries do no
writes and corruption fails without repair. Selected-word reads now include an
optional typed `ReaderCustomEntry` from the same read transaction, only after both
built-in sections are empty. The transport contains literal surface/meaning/reading
strings; validated domain/storage values remain distinct from wire payloads.
`SaveReaderCustomEntry`/`DeleteReaderCustomEntry` run on the existing FIFO via typed
core events/effects. Both core and repository require the current missing-match
selection; repository rechecks original-token ownership and built-ins on each write.
Callbacks attach only to the same pending/current selection and matching surface.
Dismissal/navigation discard late results/errors without cancelling accepted commits.
Failed mutations retain the committed entry and expose `reader_custom_entry_error`,
independent of lookup/time errors; explicit submission retries, with no automatic
write loop or frozen retry obligation. Success/failure drain queued clock saves and
session teardown uses existing busy gates. Generated Kotlin/JNI and Android session
transport exercise CRUD/reopen. Compose shows committed custom meaning/reading
and Add/Edit/Delete only for successful empty built-in matches, inside the existing
phone sheet/wide panel. Selection-keyed drafts preserve literal input across
status/clock updates; dismissal/recreation follows the existing selection reset.
Shared save gates disable mutations; separate save/delete errors retain draft and
committed data with explicit exact-command retries. IME edits reset reader idle.
Migration 0010 adds completion_analyses historical identities, every Japanese
reading_occurrence and completion-linked New→Known status events in the same
time/history/position transaction. Pure analysis selects tokens by their start
offset, retaining whole anchors across page boundaries. Storage validates a frozen
AnalysisId against canonical text, registers only absent departing-page surfaces,
preserves other statuses and returns newly_known only on a new success. Replays
check command payload/analysis identity before live content, so deletion/reflow
cannot repeat effects. Document deletion retains occurrences/analysis provenance;
vocabulary deletion cascades only its occurrence/status rows. Legacy completions
remain explicitly without analyzed history. Status-table migration preserves old
rows, IDs and allocation high-water marks. No new dependency or executor is added.
Migration 0007 persists immutable chapter analysis/token snapshots and reversible
segmentation overrides. Identical snapshot retries reuse IDs; conflicting output
under the same revision/mode key errors. Override replacement/deactivation commits
atomically, keeps base output intact and preserves global rules after source content
deletion. Confirmed single-document deletion now runs on the same FIFO: one immediate
transaction removes live chapter text then the Library row, cascading content/assets/
analyses/dedup/position/partial time while retaining historical provenance, vocabulary,
custom entries and overrides. No dictionary load or rescan occurs. Core retains the
visible target across Activity recreation; matching explicit confirmation is required,
Cancel has no effect and failures allow explicit Retry. Android disables delete during
file batches and unresolved saves. Success reconciles the full Library and invalidates
status counts without changing query/sort. Bulk deletion shares the same cleanup:
`DeleteDocuments`/`DocumentsDeleted` carry a frozen ID vector over the existing typed
FIFO/JNI, and one transaction covers the entire selection. A failure on a later
document or at COMMIT restores earlier deletions. Single deletion delegates to that
repository path. Pure core retains selection, confirmation titles/IDs and explicit
retry errors; no selection or SQL I/O lives in Compose. Long press starts selection,
row taps toggle, Select All adds every title matching the current query across the
full Library, and selection persists across filtering/sorting/recreation. The UI
shows the total selected count and all confirmation titles, including hidden rows.
Cancel of the prompt returns to selection; Cancel/Back or navigation retires it.
File-batch/save gates also cover bulk actions. Rename is described above. Storage
tests cover migrations, reopen, rollback, Unicode, concurrency, ID allocation,
settings, vocabulary, analysis anchors and completion history. Base-analysis identity and anchored completion intents reach core/FFI; token/override
repositories have no segmentation UI operations.

Pure `yomoku-core::segmentation` derives effective whole-chapter boundaries from
original Mode B and active edits. A/B/C choices partition a complete Japanese span
from a validated same-profile snapshot; manual edits share the exact partition
representation. Global rules match complete adjacent B tokens; local source/range
rules take precedence, then newer IDs within each scope. Resolution preserves
original token indices only for exact unchanged spans and never invents analyzer
metadata. `Storage::load_chapter_segmentation` reads original B and the active rules
in one read transaction, validates retained provenance and returns a separate typed
result without SQL in core or an analyzer dependency in storage.
`resolve_effective_segmentation` and `load_effective_chapter_segmentation` additionally
attach exact-span analyzer metadata from original B, then same-profile A, then C.
Metadata retains AnalysisId/mode/token index and original classification/reading/
dictionary form. Arbitrary manual spans remain explicitly unresolved when no exact
token exists; metadata is never guessed from neighboring tokens or another spelling
occurrence. The repository validates complete eligible snapshots in the same
read-only transaction as rules; later profiles are excluded. Domain/storage fixtures
plus real Full owned/mapped fixtures cover scope, metadata provenance, reversibility
and unchanged snapshots/history.
Metadata resolution reuses the boundary's exact original-B token index. Changed
spans binary-search validated sorted A, then C, requiring both ends to match; no
additional chapter-sized metadata tree is built. Complete snapshot validation,
contextual provenance and B/A/C priority remain unchanged. The opt-in
`measure_effective_reader` host example measures real TXT/EPUB inputs in a fresh
scratch database; see EFFECTIVE_READER_PERFORMANCE.md for the real-novel baseline
and remaining SQLite/device costs. It introduces no app cache or new payload.
`effective_reader::EffectiveReaderChapter` borrows canonical text and privately owns
validated resolution. It projects eligible whole occurrences with distinct effective
segment indices and page-local UTF-8 display ranges. Exact analyzer classification
governs resolved tokens; unresolved manual partitions inherit reading eligibility
from the editable Japanese B span, explicitly separate from analyzer metadata.
Visible crossings appear on both pages; future completion projections own starts.
`EffectiveReaderChapter::furigana_on_page` projects source-preferred ruby from that
same private validated resolution. It shares the original reader's source-overlap
policy, clips bases while retaining complete readings, and uses only exact Japanese
analyzer metadata as fallback. Unresolved manual spans never borrow old B readings.
Whole source-ruby validation and page ownership checks precede output; no live
vocabulary, dictionary, SQL, registration or platform I/O participates.
`EffectiveReaderSession::furigana` now supplies the corresponding retained-page
projection for live integration. It borrows compact visible metadata without
cloning/re-resolving the full chapter key. Original and effective paths share
strict page/surface/range validation and source-overlap policy; stale, pending,
failed, Missing B or invalid fallback data leaves validated source ruby usable.
Android's pure `ReaderTokenRanges.kt` shares the existing B renderer's UTF-8 to
UTF-16 geometry with `effectiveDisplayTokens`. Effective display tokens keep the
compact page revision/segment selection and full crossing surface, and reject the
whole projection on stale identity, malformed Unicode/ranges or invented manual
metadata. The host Kotlin bridge runner compiles/tests this production helper.
These adapters are ready; live Event/ViewModel/Compose adoption remains coordinated
with completion/status/word/custom commands below.
`Storage::load_effective_reader_tokens` composes the same resolution loader with
live indexed exact-surface status reads in one read-only transaction. Repeated
surfaces share a lookup; missing/deleted entries stay absent. Results retain the
complete effective segmentation/provenance; no analyzer or registration is invoked.
`EffectiveReaderTokenKey` freezes document/page and complete resolved boundaries/
metadata/provenance, independently of live vocabulary. Missing B has no selectable
key. `Storage::load_effective_reader_word` re-resolves and compares that identity in
one read-only user-state transaction before accepting an effective segment index.
It shares exact-surface counts, word/name candidate lookup and missing-match custom
entry loading with the original-B API. Only exact analyzer metadata supplies a base
form; manual spans remain unresolved. Reversal/replacement/metadata changes reject
stale selections, while status/count/custom-entry changes stay live. This internal
domain/repository key supports serialized completion reservations and remains
native behind compact page/selection identities.
Effective custom-entry CRUD shares that transaction-local selection validator.
Save/delete hold an immediate SQLite transaction across validation, dictionary
gating and mutation; read-only loads use a normal read transaction. Both original
and effective APIs share exact-surface built-in protection and custom row helpers.
Manual spans never inherit an original token's base form. No registration, analyzer
metadata, historical event, schema or bridge changes are made by these operations.
Production reader token/word/furigana/completion paths use these effective APIs.
A separate effective completion repository now prepares an opaque pure-domain
`EffectivePageCompletion` after checkpoint drain, validating the full rendered key
and reserving exact page/time/occurrence provenance in migration 0018. Commit authenticates
that reservation before replay checks; override changes cannot reinterpret it. It uses
the common position/time/finish transaction and shares original-B exact-surface
registration/New-only transitions. Effective occurrence indices have explicit versioned
reservation provenance; prior history remains untouched. No analyzer or SQL enters core.
The internal JSON reservation is write/compare only; unused IDs never auto-complete.
A separate registered-override save now shares transaction-local rule persistence
with the original analysis-only save. One immediate transaction validates the selected
Japanese B span/full effective snapshots, replaces its rule and registers only that
rule's eligible winning parts at the selected occurrence. Existing statuses remain;
identical active-rule retries write nothing, including after vocabulary deletion.
Reversal preserves registration/history. Other occurrences/documents are not rescanned
for registration, and no analyzer/platform I/O enters core. No schema/dependency/bridge
change. Rendered-selection and intervening-edit retry guards remain prerequisites for
future typed override controls. Live core/FIFO/FFI integration and Android controls
remain next. The [synthetic payload review](EFFECTIVE_READER_PAYLOAD.md) measures
14.3 MB of full-key JSON for a 100,000-character chapter versus 42.5 KB of visible
500-character page tokens. The pure `effective_session::EffectiveReaderSession`
prerequisite retains one full key in core while exposing compact typed page/selection
identities and visible token fields. Each load/refresh gets a fresh session revision;
invalidation drops pending/current state without resetting the counter. Exact request,
document/page/original-B checks reject stale results; selection additionally requires
a visible eligible segment. Missing B remains plain and unselectable. Full provenance
still authenticates repository transactions and separately retained accepted commands.
Prototype generated Kotlin codecs verify 60-byte available-B identities and 68-byte
selections; no full chapter enters those values. These schemas/state methods are not
yet wired into live core/FFI/Android events/views. The linked payload review defines
lifecycle/integration obligations; actual device/real-book performance remains pending.

`effective_storage` now defines the Rust-only protocol for that switch. The nested
`StorageOperation::Effective`/`StorageResult::Effective` variants are Serde-skipped
and Facet-skipped (their fields are opaque), so neither full keys nor durable frozen
completions can acquire a shell codec. The existing `StorageEffects` consumes them on
its single FIFO/connection; no additional worker, runtime registry or middleware layer
is introduced. Token reads echo their exact Rust-issued request and verify original B.
Word/custom operations reuse the existing lazy dictionary/error/reopen policy and
transaction-local effective selection validation. Preparation and commit use migration
0018 repositories; completion reloads the document after its atomic commit, as the
original-B path does. Failures use the existing typed `DocumentError` channel.

The retained page now owns its full key in an immutable `Arc`. `freeze_page` and
`freeze_selection` authenticate compact callbacks and issue Rust-only accepted command
values with private fields; retry clones share the same allocation rather than copying
whole chapters. Invalidation retires future callbacks while accepted commands retain
their original key through time drains and retries. Prepared completions similarly
share one frozen reservation. Storage still revalidates every selection/preparation;
a reserved completion keeps its exact semantics after reversal/deletion. Neither worker
startup nor opening the database replays unfinished reservations. The app's live state
machine has not switched protocols yet; unexpected effective results fail closed.

`EffectiveReaderSession` also owns retained selected-word/custom-entry state. It
issues Rust-only `EffectiveWordRequest` callback tokens with fresh checked attempt
numbers for every dispatch, including retries of the same frozen write. Native
word results authenticate full key, visible token/metadata and exact selection
before moving candidates/count/custom data into `EffectiveWordDetails`. That
shell schema contains only one compact token and selection; no chapter provenance.
Live vocabulary observations update every same-surface visible token. Missing
matches permit literal custom edits; word/name matches and failed lookups do not.
Accepted saves/deletes retain their exact key and values until success or an
explicit action after failure. In-flight writes cannot be cancelled through this
state API; dismissal/invalidation retires presentation while callbacks still drain.
New explicit loads/edits may supersede a failed retry without retargeting old callbacks.
Failures preserve the committed entry. Old attempts cannot consume a newer retry,
and late results cannot resurrect a dismissed/refreshed page. Load failures require
another explicit load. The owning app must still apply its shared storage/time/
completion gates before dispatch and wire callback tokens to the native result.
Standalone Kotlin codecs and actual FIFO tests verify this component; the live
app has not adopted it yet. There is no new worker, persistent retry or schema.

The same retained session now owns completion from acceptance through final-time
drain, reservation and commit. `begin_completion` freezes the rendered key/intent
and invalidates shell callbacks immediately. Word I/O, accepted custom retries and
pending status choices must drain first. Pending completion blocks new word/custom/
status work. The owning app must drain accepted AND coalesced checkpoints before
`dispatch_completion`; core
does not read clocks or storage. Each native `EffectiveCompletionRequest` gets a
fresh checked attempt, even on exact retries. Preparation authenticates the returned
key/intent; commit retries share the same reservation allocation across invalidation
and worker reopen. Stale/duplicate/wrong-stage results cannot consume newer work.
Failures retain their obligation for explicit retry; there is no completion Cancel
because commit may have succeeded before a follow-up read failed. Only one consumed
success returns its frozen command and Recorded/AlreadyRecorded outcome to the app
for position/time/notice handling. No unfinished command auto-replays on restart.
Actual FIFO tests verify checkpoint ordering, rollback and a lost-success response.
This component is still separate from live app events/views and Android.

Manual status commands also use the native effective protocol. `SetStatus` carries
an accepted shared-key selection plus the vocabulary ID, choice and timestamp.
Storage validates the complete resolution and visible exact-surface ID in the same
immediate transaction as the shared manual status/history helper. No analyzer or
word dictionary loads; missing vocabulary stays absent and replacement IDs are
rejected. The retained session issues fresh native `EffectiveStatusRequest`
attempts and preserves frozen values for explicit retries across invalidation.
Only the matching retained revision receives successful live status/deletion
updates, including repeated spellings and selected-word details. Failure preserves
committed presentation. A new explicit choice or Cancel can release a failed
request; in-flight writes must drain. Status pending gates word/custom/completion;
accepted word/custom/completion obligations gate status. This completes the native
status path needed by the coordinated live switch, without adding shell schemas.

`Storage::create_analyzed_paste` composes the existing private transaction helpers
for document creation, validated Mode B analysis and exact-surface registration.
A caller-provided callback receives allocated canonical chapter identities; the
caller must load/reuse its analyzer before entering the immediate transaction.
Storage has no analyzer dependency, and core remains free of SQL/platform I/O.
Document/text, immutable tokens and new Japanese vocabulary/dated events commit
together; existing statuses remain intact. Analyzer/validation/write/commit errors
roll back the entire import. Typed analyzer errors remain separate from storage
errors. Import creates no position/completion/encounters and does no deduplication.
Six synthetic repository tests and a real Full fixture per loader verify this
composition, including concurrent registration and exact Unicode reopen. Production NativeBridge/Android use
the deduplicating wrapper below on the same worker.

Migration 0008 adds a live-document SHA-256 index for exact pasted UTF-8 bytes.
`Storage::import_analyzed_paste` uses one immediate transaction to index legacy
pastes lazily, reference the oldest live exact match, or compose a new analyzed
import and its hash. Concurrent identical submissions analyze/create only once.
Duplicate results preserve title, progress, analysis and vocabulary without
rescanning or invoking the analyzer. Existing duplicate Library entries remain;
document deletion cascades only its hash, so reimport allocates fresh identities.
Every failure rolls back hashes and new import rows together. Nine host tests
cover migration/retry, concurrency, byte identity, preservation and rollback;
the real Full composition fixture also verifies deduplication after reopen.
Raw creation methods remain allocation primitives. Production imports must use
the deduplicating entry point; file identity must hash original
file bytes separately when format importers exist.

`yomoku-ffi::YomokuBridge` wraps the router in Crux's Bincode bridge.
`YomokuBridge::with_effective_reader` and
`effect_router_with_effective_reader` construct the production reader using
`Model::with_effective_reader`, the existing dictionary loaders, FIFO and analyzer
lifecycle. NativeBridge selects this factory without a shell setting, extra
BoltFFI export or new wire schema. Serialized Rust tests cover retained requests
and exact retry guards; production Kotlin/JNI and Android fixtures use compact
page revisions and effective segment selections for words/status/custom/completion.
Original B remains immutable as the default and editor anchor basis. A/C snapshots
and saved rules are reused without Full on reopen; only explicit missing A/C
generation requests analysis. Full keys and accepted retry payloads stay native.

BoltFFI 0.30.1 NativeBridge takes a database path, owned EffectsListener and
required DictionaryProvider. It exposes update/resolve/view and set_foreground,
and reports worker creation failure as WorkerUnavailable. DictionaryProvider is
called lazily on the existing FIFO worker and transfers an owned read-only FD
plus one (zero means unavailable, including generated callback failure). Android
verifies/extracts through the immutable private publisher, duplicates/detaches the
same descriptor, and closes the Java stream. Rust consumes/closes the transferred
FD on success/failure and rehashes the actual mapping; it never reopens a pathname.
Published inodes remain immutable through draining accepted work after close.
No Activity or analyzer crosses the worker boundary and no extra executor is used.
Callbacks carry asynchronous Render batches; document errors are view values.
Core-only effects stay private. Facet generates Event/ViewModel/Requests and
document codecs in app.yomoku.shared; BoltFFI generates the native package and JNI C.
Native wrappers require explicit close; no global singleton exists.

One hundred and ninety-five data-free Rust tests cover analysis classification/
checksum, domain, storage and bridge behavior, including an actual SQLite write
lock proving dispatch/view/navigation remain responsive.
The host Kotlin runner uses real JNI with -Xcheck:jni to verify navigation,
Unicode, async worker callbacks, typed failures, reopen persistence, isolation,
listener cleanup, 100 normal lifecycles and closing during I/O. Seven additional
bidirectional Rust/Kotlin fixtures exercise Unicode anchors and full-width IDs.

`android/` is a Kotlin/Compose shell (min SDK 29) built with Gradle CLI/JDK 17.
Gradle preBuild generates payloads/JNI and packages combined libraries for
arm64-v8a/x86_64 using NDK r30/API 29. CoreSession projects Render snapshots on
Android's main thread and ignores callbacks after native close. CoreSessionOwner is an
Activity-scoped AndroidViewModel: it supplies Application.filesDir/yomoku.sqlite3,
retains the native session across recreation, and requests draining close onCleared.
It retains no Activity or independent domain state; its file platform helper retains
only the in-process URI queue, current encoding input and copied per-file outcomes.
Each new session requests Library data;
the database and page-length setting are first loaded on the Rust worker. A fresh
process starts Library with no dictionary load. Activity onStart permits reuse;
onStop queues release unless isChangingConfigurations, preserving the analyzer
through retained recreation. Accepted imports finish atomically before release;
hidden imports release before resolving. Final native close drains accepted work
and drops its analyzer/provider without joining Android's UI thread.

CoreSession drives the pure clock with SystemClock.elapsedRealtime samples. Activity
onResume/onPause controls accounting independently of analyzer start/stop, including
configuration pauses. After the visible reader's busy work finishes, restore reserves
its exact-offset token; only its successful callback starts Resume. Handler checkpoints
run every 15 seconds without resetting idle. Activity interaction signals include
touch movement and generic scroll, with typed UI events sampling Pause before settings,
movement, completion or exit. Fresh offsets restore a new epoch; EOF starts none.
Restore failures require visible Retry. Failed checkpoint/completion obligations also
offer Retry in reader/Settings; unresolved saves disable preset changes.

Close rejects new UI dispatch immediately, pauses once and keeps callbacks alive until
storage_busy, both save-pending flags and direct file cancellation clear. Only then
does it release the native handle. If a save fails after onCleared, a one-second
Handler retry resubmits the exact core payload; a persistent failure retains the session while the process
survives. This never joins the UI thread or fabricates a successful save. Android can
still kill a process before a final callback; only committed durations survive, and
abrupt death can lose time since the latest periodic checkpoint. No OS guarantee is made.

Compose shows Library titles/current percentage, a multiline paste form and a
centered, scrollable reader column capped at 720dp. Its ScrollState is transient
and keyed by document/page anchor, resetting on page changes/reopen/recreation.
Below content, Previous/counter/Next share a row. Axis-specific horizontal swipes
dispatch once after 48dp net movement, alongside ordinary vertical scrolling.
An app-level keyed Compose effect consumes positive CompletionNotice and displays
a short Android Toast once; zero stays silent and recreation does not replay it.
Settings exposes five radio choices, disabled while unknown/busy, and offline
Sudachi licenses/notices and JMdict/JMnedict credits. Dialog visibility survives Activity recreation.
Nostr/Statistics/Vocabulary remain placeholders. Busy/error/retry state comes from
Rust; the unsaved form draft and notice dialog visibility use rememberSaveable.
The draft is retained through Settings/recreation.
Navigation uses a bottom bar below 600dp and a scrollable rail above; Settings,
paste and reader routes hide it. Toolbar/system Back return through Rust routes.
Android instrumentation covers production analyzed imports/lifecycle, reader-time
accounting/drain/recovery, navigation/rereads/duplicate callbacks/completion failure
and toast consumption, eight private-extraction tests and the debug-only
in-app analyzer probe on API 36 x86_64 phone. Exact current runs are in
DEVELOPMENT.md. The two
asset/notices tests previously passed on phone and tablet landscape in airplane
mode. The seven prior
tests previously passed on phone and tablet landscape,
including paste/draft/errors, recreation/Back, saved offsets, bidirectional
percentages, scroll resets, Settings reflow and persisted preset reopening.
Automatic Android backup/transfer is excluded. Arm64 execution is unverified.

## Required boundaries and next work
Production paste imports now atomically persist Mode B analyses and register all
unique Japanese surfaces without encounters, deduplicate exact bytes and deliver
Created/Existing summaries. Native descriptor delivery and retained/background
lifecycle are connected. TXT parsing, atomic storage and the Android multiple-file
picker/encoding/summary path are connected. SRT extraction and atomic analyzed
repository/FIFO/typed JNI imports are verified. EPUB persistence and ordered-chapter
reader/application/FIFO/typed JNI imports are verified. Android mixed-format
batches use the shared retained picker/encoding/progress/Cancel/summary helper;
reader text formatting/website links and raster/static SVG images/shared viewer
are connected, including Library covers and source-preferred furigana with its
Settings switch/layout; advanced SVG resources are outside v1 scope.
All file background notifications use the platform-only
foreground lease described above. Legacy imports
are not automatically rescanned.
Explicit core/worker/FFI pending-time checkpoints coalesce during unresolved saves
and drain before reader movement/exit or analyzed completion. Completion preparation/
exact retries now run through core/FIFO/FFI. Android monotonic lifecycle/interactions
and native drain are connected. Page movement/swipes and positive-count toast
rendering are connected. Migration 0011 persists a permanent finish boolean with
final-page completion, independent of position, and backfills live pastes from EOF
history. Library/document/reader projections carry it through generated FFI;
Android shows an accessible checkmark independent of the current percentage.
Explicit reopening persists page 1, while completion reloads and retained Activity
recreation preserve the active position. Final-page Finish Book/swipe completion
returns to Library and celebrates each newly Recorded command, including rereads.
Effective segmentation edits are connected as described below. Read-only lifetime
aggregation and vocabulary timelines are implemented; buckets, transport and UI remain
separate tasks. Add analysis/import crates only with
actual behavior; Sudachi Full defaults to B, JMdict/JMnedict remain offline data, and future Nostr I/O stays in Rust. Configuration is empty.
## Guarded effective boundary repositories

`yomoku-storage::effective_overrides` adds a native-only read/choice/save/reverse
slice. `yomoku-core::effective_overrides` owns the I/O-free snapshot types.
`EffectiveOverrideContext` shares a supplied frozen effective key through
`Arc`; context/retry clones do not duplicate the chapter. One read transaction
checks rendered provenance and exact contiguous Japanese original-B bounds whose
edge tokens intersect the page, then reads existing same-profile A/B/C partitions
and winning rules. Crossing tokens remain whole; unavailable or unaligned alternate
choices are omitted. This API never invokes the analyzer or registers vocabulary.

Save and reversal revalidate both the full key and a rule-table revision in one
immediate transaction. Save reuses the transaction-local registered-override helper,
preserving winning-parts-only registration and same-rule no-ops. Reversal is limited
to a winning rule returned by the prompt. The revision observes every supported
append/deactivation, catching an intervening split/reversal even if the key returns
to its original B value. Rule changes elsewhere conservatively invalidate prompts;
live vocabulary and time changes do not. Failed transactions can retry unchanged
contexts; committed mutations require explicit reload, including lost responses.

`effective_session::overrides` now retains prompt presentation and accepted native
read/save/reverse requests. Stored choices and manual partitions share the same
save path; core validates literal partition shape, while storage revalidates full
provenance and rule revision transactionally. Checked fresh callback attempts reject
duplicates and previous retries. Dismissed/invalidated reads drain without restoring
prompts. Accepted writes share an immutable context through `Arc` across invalidation
and explicit exact retries; in-flight work cannot Cancel. Write acceptance retires
the rendered page immediately. Successful writes and stale results require explicit
page/context reload. A lost success retried against its old before-image returns
stale, without re-registering deleted vocabulary. Failed-write Cancel also retires
any newer page, since an unknown response can hide a committed edit.

Native `LoadOverrideContext`/`SaveOverride`/`ReverseOverride` effects use the existing
FIFO/SQLite connection without analyzer or word-dictionary loading. Typed native
failures distinguish storage errors from invalid selections/stale contexts; no
shell error enum changes. Pending override I/O/failed writes mutually gate retained
word/custom/status/completion requests. The live app must also apply its time/FIFO/
lifecycle gates. Contexts are database/session scoped and must be discarded on
replacement/restore. Full contexts and requests have no shell codec. Compact
prototype contracts are described below; live Android controls remain pending
alongside the coordinated reader switch.

Explicit `GenerateOverrideAnalysis` uses the same FIFO-owned analyzer cache as
imports. Its retained request freezes mode plus the shared context and gets a fresh
attempt on retry. Storage validates the before-image in a read transaction and
reuses existing same-profile A/C snapshots without loading Full. For a missing mode,
it releases the transaction before whole-chapter analysis, then rechecks the key
and rule revision under an immediate transaction before storing immutable output.
The requested mode, original engine/dictionary profile and all canonical anchors
must match. Competing edits reject the computed result; no automatic rebase occurs.
The production native adapter supports pinned-profile modes; import-only analyzer
closures reject alternate requests. Failures discard a bad analyzer, while normal
reuse/background release follows the existing import lifecycle.

Generation creates no rule, vocabulary, encounter, time or progress writes. New
exact metadata can nevertheless change effective identity, so acceptance retires
page callbacks and success requires explicit page/context reload. Failed attempts
retain their before-image for explicit retry/Cancel; a lost success that changes
metadata becomes stale. Stored but unaligned alternates remain absent from prompt
choices and are never regenerated. No new migration or public shell codec.

`EffectiveReaderOverrideState` now projects only the selected literal span, stored
A/B/C partitions, winning-rule IDs/scopes/parts, default All occurrences and typed
pending/error/reload facts. Full chapter resolution, rule revision, originating
rule anchors and retry payloads stay native. `EffectiveOverrideAction` supplies
Open/Choose/manual Save/Generate/Reverse/Retry/Cancel/Dismiss. Its pure handler
authenticates prompt identity before delegating to the existing retained lifecycle;
the live app must still enforce shared time/FIFO/lifecycle gates.

Each opened prompt receives a checked fresh revision alongside its compact page
identity. Dismiss/reopen of the same span on the same retained page therefore
rejects old actions. Accepted writes retain that identity across invalidation and
exact retries; new callback attempts remain separate. Retry/Cancel authenticate
the accepted obligation, and in-flight writes still drain. Generated Kotlin
round trips exercise these standalone schemas; live Event/ViewModel adoption is
pending. No migration, dependency or durable prompt registry.

`ReaderBoundaryEditor` is an independently tested Android renderer for that compact
state/action contract. It offers stored A/B/C or explicit missing-A/C generation,
All/This scope, scalar-boundary cut toggles/join, literal partition previews and
winning-rule reversal. `ReaderBoundaryText` rejects malformed UTF-16 and blank
parts using Rust's whitespace policy. No source editing or inferred readings.
Saveable cuts/scope belong above Dialog and authenticate the native session key,
entire prompt identity and literal text at restoration. The parent must keep the
session key across rotation and replace it with the native session.

Pending writes expose only native Retry/Cancel; busy writes cannot be dismissed.
Retired/invalid prompts offer explicit reload, and dictionary-clock interactions
include dialog scrolling/keys. The owner supplies existing time/FIFO gates and
reload/close callbacks. The component has no native bridge/session of its own and
is not exposed in the current B reader: original-B span selection, live app events
and the coordinated reader/Library switch remain pending.

## Native effective reader callbacks

`effective_session::EffectiveReaderRequest` groups accepted token, word/custom,
status, completion and override attempts. `command` captures the originating
request in a native Crux callback while dispatching the existing storage effect.
`EffectiveReaderSession::finish_request` routes standard FIFO failures and native
results back to the matching retained lifecycle. Malformed envelopes fail closed;
stale/duplicate attempts return None, so the app must leave newer busy state alone.
For tokens, both the captured and returned request must match before acceptance;
invalid responses consume only their own read, requiring explicit fresh loading.

The wrapper has no Serde/Facet shell representation and adds no worker, I/O,
registry or persisted state. Progress distinguishes consumed token failures and
completion/override outcomes; word/status errors remain in their existing retained
state. It never dispatches retries or completion follow-ups automatically. Shared
time/FIFO/lifecycle gates still belong to the app; final checkpoint slots must drain
before preparation dispatch. Selected-word status observations of deleted entries
also clear their erased encounter count. The native FIFO regression harness uses
this adapter; live Event/ViewModel/Android adoption remains pending.


## Production application effective pages and completion

`Model::with_effective_reader()` is the production native integration constructor.
It is not a user preference or shell toggle. The model routes existing
`LoadReaderTokens` through one lifetime retained session and projects `effective_reader_tokens`. Source-preferred furigana
borrows that compact page. Legacy word/status/custom/completion events fail closed
in this model. Library membership uses the effective repository through the same
model and FIFO.

`CompleteEffectiveReaderPage` authenticates the compact displayed revision and
exact reading position. It freezes the full partition before both accepted and
coalesced final-time slots drain, then dispatches retained preparation and commit
through the existing FIFO. One captured native request owns the app busy slot.
Its callback moves the storage result into `finish_request`, without cloning a
chapter; boxed native request ownership keeps shell Event size bounded. Dismissed
reads release only their own slot and still drain queued time. Duplicate/foreign
callbacks cannot clear newer work. Departure/reflow/refresh invalidates callbacks
before I/O, including departures deferred behind a timer save.

Preparation success dispatches commit; failures require explicit exact retry.
Accepted completions gate movement/settings writes/clock resumption, survive
failed checkpoints, and drain before deferred departure. Original/effective
completions share position, final-page return, positive-only word notice and
new-success-only celebration handling. AlreadyRecorded stays silent. No automatic
restart replay, worker, dictionary access, persistent registry or schema change.

The app also accepts `LoadEffectiveReaderWord`,
`SetEffectiveReaderStatus` and effective custom-entry save/delete events. Each
selection contains only page revision/base and effective segment index; full keys
and literal retry payloads stay in the retained session. Optional
`effective_reader_word` and `effective_reader_status` project compact details,
errors, busy state and pending write obligations. Legacy selection events still
fail closed. Android uses the compact selection events in production.

Word/status/custom requests use the same captured FIFO callback and allow clock
checkpoints to coalesce while busy. Failed writes retain exact native commands;
explicit Retry gets a fresh attempt, and Cancel releases only failed work. Pending
writes defer departure and block page refresh/reflow/import/other selections;
clock checkpoints still drain on failure. Dismissal/settings hide word details but
preserve pending custom errors and writes until success/Cancel. Deferred movement
waits for both time slots and writes. Word reads/status results refresh repeated
surface observations and invalidate Library bars, including vocabulary deletion.
Custom CRUD never changes registration/status/history. Built-in protection and
stale provenance checks remain transaction-local in storage. Only word/custom
lookup loads the lazy word/name index; no new analyzer access or worker is added.

The app routes `EffectiveReaderOverride(EffectiveOverrideAction)` and
projects `effective_reader_override` using the existing compact prompt/state.
Open authenticates the current page revision; subsequent actions authenticate the
retained prompt. Stored A/B/C choices, literal manual parts, explicit A/C generation
and winning-rule reversal use the same captured FIFO callback. Reads may be
dismissed without releasing their I/O slot. Accepted edits/generation invalidate
the page immediately; storage failures retain exact Retry/Cancel obligations and
defer movement/reflow/import until resolved. Clock saves coalesce during override
I/O and both time slots drain before deferred departure. Success/stale outcomes
require an explicit page/context reload; no automatic rebase/replay is added.
Override outcomes invalidate Library distribution caches. Full contexts/rule
revisions remain native; existing transaction-local storage guards still apply.

## Production Android effective word consumers

`EffectiveReaderWords` accepts the compact app ViewModel, explicit retained native
session key, existing dispatch/interaction callbacks and image loader. It uses
`effectiveDisplayTokens` for strict UTF-8/UTF-16 projection, revision/segment
selection and complete crossing surfaces. Shared annotation styling preserves
source formatting/link ownership, status backgrounds, image positions and ruby.
Shared dictionary and phone/wide panel presentation accept display fields only;
effective selections are never converted to original-B word identities.

Vocabulary colors and decoration are ordinary static spans. Clickable word links
carry explicit empty `TextLinkStyles`, preventing Material from adding website
styles and avoiding Compose's per-link annotation rewriting. Long text fragments
(over 1,024 UTF-16 units) are measured once at the actual column width and rendered
in smaller interactive layouts, split at a soft visual line end after 512 units.
Canonical text, logical pages, native identities and source offsets stay intact.
Annotated slices retain their original callbacks; crossing tokens/readings keep
their whole identity/reading. Bidirectional text, direction controls and explicit
paragraph styles retain the complete layout to preserve embedding context.
Images keep their existing source order. No scroll position is persisted.

Word lookup/status/custom events use the compact native app lifecycle. Reads wait
for shared storage/time/completion/mutation gates; failures need explicit Retry.
Attempt token reads once per base identity; page retirement needs explicit reload.
Scope the override reload marker to its retired page, allowing later navigation.
Panel dismissal retires presentation, while pending native status/custom writes
keep reachable payload-free Retry/Cancel controls. Session replacement/page
revision changes discard local selections/drafts; retained link callbacks cannot
reopen a selection from an older revision. Custom fields preserve literal Unicode
and use Rust whitespace validation without changing unresolved analyzer metadata.
The app/reader now passes `CoreSession.sessionKey`, a UUID retained with the native
handle across Activity recreation and renewed for each replacement. The optional
effective word-state projection selects this renderer, including when tokens are
missing/retired. NativeBridge constructs the effective model. Next/Finish validate the
effective projection and carry its compact identity and exact departing position;
Previous uses the position-only event and remains available after token failure.
Storage/time/completion and pending effective mutations gate movement. Navigation
callbacks reject retired revisions/sessions, and swipes observe current identities
even when page geometry stays unchanged. Effective completion samples/pauses the
platform clock before dispatch through the existing native drain/Retry lifecycle.
`EffectiveReaderBoundaries` exposes the editor in the production reader.
The atomic page read projects `original_boundaries` while original B is already
loaded, without another analyzer or analysis read. Each page-only span retains its
whole anchor/surface and clipped display range, independently of effective edits.
Android validates strict UTF-8/UTF-16 projection and lets users select one B token
or two endpoints of a contiguous Japanese range. Gaps, malformed data and Missing B
fail closed. Selection is scoped to the native session/page revision; accepted
prompt/write/retry ownership remains native. Shared time/FIFO/mutation gates apply
to opening and submitting the editor; pending failures remain reachable after page
retirement. Reload explicitly retires the old prompt and requests a new page.
The production core/FFI/Android path uses these controls. The ephemeral page
wire schema adds only these visible spans; no durable schema/dependency change.

Effective override reload requirements belong only to the retired page. Normal
page invalidation, close/reopen and explicit token loads clear the presentation
marker; override success/stale/Cancel outcomes set it after retiring their
before-image. Accepted writes/errors/retry payloads remain owned independently.
This permits normal initial token reads on a newly opened reader, including the
same book after effective completion. Successful edits retire the prompt; the
reader exposes an explicit Reload action rather than silently reloading.

Android's one-attempt token-read guard includes the exact reading position and
page-length preset as well as the base page/analysis. A normal same-anchor position
save or reflow can retire native tokens and must trigger a new read. Error clearing,
clock saves and typography do not change that ownership or replay a failed read;
boundary retirement continues to require explicit Reload.

## Read-only lifetime statistics

`yomoku-core::statistics::LifetimeReadingTotals` is a pure occurrence/time value,
without shell serialization or storage effects. The separate SQLite repository
exposes `Storage::lifetime_reading_totals(&self)`: one statement reads a consistent
snapshot of completed analyzed commands and their surviving occurrence rows.
Time is summed independently per command, preserving zero-word pages without
multiplying time by repeated words. It reads no live content/analysis, dictionary
or effective JSON and writes no caches or history. Legacy events without analysis
markers, pending checkpoints and unused reservations are excluded. Document
deletion preserves totals; vocabulary-history cascades reduce occurrences while
retaining completed time. Header-version/duration checks and exact-integer overflow
errors fail without repair. Runtime/FFI/Compose statistics integration remains
future work; the Statistics tab is a placeholder.

`StatusTimelineBuilder` in the pure core streams surviving registration and
status changes in durable event-ID order, validates each entry's causal chain and
accumulates date deltas. `Storage::vocabulary_status_timeline(&self)` supplies the
links and events from one SQLite read statement, validating v1 semantics, causes and any
completion provenance headers. It reads neither live content/analysis nor JSON,
dictionary data or cached counts. The result has absolute five-status counts after
all changes at each timestamp, plus distinct Known dictionary-form counts, without
timeframe filtering or date bins. Memory holds entry/link state, status deltas per
date and Known reference-count deltas per affected form/date, rather than a copy
of every event. Document deletion preserves the input;
vocabulary history cascades recalculate it without rescans.

For a regressing wall clock or delayed frozen completion, clamp only that entry's
projected date to its preceding event's projected date. This retains causal state,
nonnegative counts and the final committed status without changing durable dates
or shifting unrelated entries. Shared-form reference counts replay in projected
date order, counting a spelling while any linked surface is Known; simultaneous
handoffs coalesce. Exact spelling identifies forms, unresolved surfaces add none
and readings do not affect identity. Links have no observation date and apply
throughout their vocabulary generation's history. Timeframe buckets and UI
transport remain separate tasks.

Migration 0020 retains resolved analyzer dictionary-form spellings in a global
vocabulary-owned junction table, independent of live documents. New snapshots
link existing entries; analyzed imports, selected registered edits and actual
original/effective completions retain their own token metadata in the same write
transaction. Missing/manual metadata stays absent; raw registration and identical
snapshot retries never rescan existing content. Exact spelling deduplicates links
across readings/contexts; distinct resolved forms remain separate associations.
Document deletion retains unread links, while vocabulary deletion cascades them
with that generation's history. Upgrade recovers available live-snapshot and
completed-occurrence evidence without creating entries or altering events; unread
metadata deleted before upgrade cannot be recovered. No core I/O, dependency,
FFI or Kotlin change is required for this storage prerequisite.
