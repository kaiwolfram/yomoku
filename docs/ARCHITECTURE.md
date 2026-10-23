# Architecture

## Implemented
`yomoku-core` is a pure Crux 0.20.0 application. It owns Library-default
navigation, persisted page-length Settings, storage busy/error state, Library
summaries, an optional loaded paste snapshot and Main/Paste/Reader content routes.
OpenPaste/CloseDocument and SelectDestination/OpenSettings/CloseSettings
remain synchronous. RefreshLibrary/CreatePaste/LoadPaste/SaveReadingPosition/
SetPageLength emit StorageOperation; only one storage command is accepted at a time, while navigation
stays available.
OpenReader uses the same load operation but tracks the requested document until
its callback; leaving the route cancels opening without cancelling submitted I/O.
Successful paste creation returns to Library without opening the reader.
StorageFinished is internal: both Serde and Facet skip it, so hosts cannot inject
storage results. No SQL, paths, threads or platform I/O enter core.

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

`yomoku-analysis` is a separate Rust I/O adapter using Sudachi.rs v0.7.0 at
`5de0410b6edfd428fc510cc9763440772e9619d5` and Full 20261015 (V1). Its constructor
checks the exact dictionary SHA-256 before giving owned immutable bytes to Sudachi.
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
write SQLite. Nine explicit Full-data fixtures include A/B/C SQLite round trips;
they run with tools/test-sudachi.sh, separately from ordinary data-free checks.
Dictionary preparation verifies the archive, data and complete notices under
ignored .local/sudachi. Gradle's bundleSudachi task verifies already prepared data
and stages the dictionary plus complete LEGAL/LICENSE under generated Android
assets; it never downloads data. Settings reads and displays both full notices
off the main thread in a selectable, scrollable dialog. The dictionary remains
compressed in the APK; no private-file copy or eager analyzer load occurs.
This crate has no app/runtime/FFI entry point yet. A standalone native example
and CLI driver measure the actual adapter on Android without installing the app:
three debug and three release processes pass offline A/B/C/Unicode/long-input
validation on API 36 x86_64. Release initialization takes 1.03–1.20 s, with about
318 MiB anonymous resident memory after initialization and 334 MiB peak RSS.
See SUDACHI_MEASUREMENTS.md for exact methodology and limitations. The driver
verifies pinned source/data, temporarily disables radios, restores their original
settings and removes its own device files. Both probe ABIs link; physical arm64,
private asset extraction, mapped storage and combined app/lifecycle memory remain
unmeasured. These measurements do not settle production storage viability.
Unpatched upstream cross-target checking failed in plugin/loader.rs:
make_system_specific_name originally had no Android cfg implementation. The root
Cargo patch now selects a dependency-only vendored snapshot at that exact commit,
with Android added to the existing Linux/FreeBSD `.so` cfg. The complete patch and
all original file hashes live under third_party/sudachi; tools/verify-sudachi-source.py
reverses the patch in a temporary copy and verifies every original byte. Both
Android targets compile. The profile includes +android-loader-v1; segmentation
and embedded configuration are unchanged. App integration remains pending.
Owned dictionary storage alone requires about 315 MiB. Compare verified immutable
mapped storage before integration; keep one on-demand worker-owned analyzer and
avoid eager Library initialization or per-page reloads.

Pure `pagination` derives chapter-local TextAnchor pages from ordered immutable
ChapterText snapshots. Five presets count non-whitespace Unicode scalars; LF/CRLF
paragraph boundaries and following-sentence fallback are independent of display
size. Supplied chapters force boundaries; headings never infer chapters. A separate
ReadingPosition retains the exact UTF-8 offset through reflow, with ownership,
scalar-boundary and EOF validation. ReaderPage projects the page containing the
saved position at the persisted global preset (default 500), preserving the exact
offset separately from its page anchor/counter. These typed values cross FFI. The position-only
save hook updates the reader and Library percentage after commit; late saves
cannot reopen a closed reader. Page controls/swipes and atomic forward completion
remain next. Settings reflow keeps the exact position and Library percentage;
only committed values change the preset/page. An independent active-document
snapshot prevents repository loads from changing the text used for reader reflow.

`yomoku-runtime` composes Core → HandleEffectLayer<StorageEffects> → MapEffectLayer.
A named FIFO worker owns one lazily opened SQLite connection per bridge. Opening,
migration and create/load/list/position/settings saves execute on that worker.
Failed opens can retry; database failures become typed DocumentError state without internal strings.
Only Render reaches ShellEffect. Dropping the bridge closes the work channel;
submitted work finishes atomically and the worker exits without joining on the UI
thread. Crux discards results when the session has gone away. There is no new
async executor or dependency version.

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
fresh ID; position-only saves never add history. These repository methods have no
app event/runtime/FFI entry point yet: real encounters, New→Known events and active
time must join the transaction before forward controls are exposed.
Migration 0006 adds global exact-surface entries and versioned, dated registration/
manual-status events. Registration preserves existing statuses; lookup and unchanged
selections write no history. Immediate transactions commit registration batches or
status/event pairs together. These repository APIs accept analyzer-supplied surfaces
and explicit timestamps; no SQL or clock enters core. Vocabulary-history FKs cascade
only on vocabulary deletion; document deletion preserves entries/history. Import,
analyzer, completion and UI integration remain pending.
Migration 0007 persists immutable chapter analysis/token snapshots and reversible
segmentation overrides. Identical snapshot retries reuse IDs; conflicting output
under the same revision/mode key errors. Override replacement/deactivation commits
atomically, keeps base output intact and preserves global rules after source content
deletion. No production document edit/rename/delete API exists. Sixty-one storage
tests cover migrations, reopen, rollback, Unicode, concurrency, ID allocation,
settings, vocabulary, analysis anchors and completion history. New analysis/override
repositories have no runtime/FFI/UI operations; synthetic fixtures validate storage,
not Sudachi behavior. Cross-occurrence matching and overlapping edit resolution
belong to the later segmentation application task.

`yomoku-ffi::YomokuBridge` wraps the router in Crux's Bincode bridge.
BoltFFI 0.30.1 NativeBridge takes a database path and owned EffectsListener,
exposes update/resolve/view and reports worker creation failure as WorkerUnavailable.
Callbacks carry asynchronous Render batches; document errors are view values.
Core-only effects stay private. Facet generates Event/ViewModel/Requests and
document codecs in app.yomoku.shared; BoltFFI generates the native package and JNI C.
Native wrappers require explicit close; no global singleton exists.

One hundred and nineteen data-free Rust tests cover analysis classification/
checksum, domain, storage and bridge behavior, including an actual SQLite write
lock proving dispatch/view/navigation remain responsive.
The host Kotlin runner uses real JNI with -Xcheck:jni to verify navigation,
Unicode, async worker callbacks, typed failures, reopen persistence, isolation,
listener cleanup, 100 normal lifecycles and closing during I/O. Seven additional
bidirectional Rust/Kotlin fixtures exercise Unicode anchors and full-width IDs.

`android/` is a Kotlin/Compose shell (min SDK 29) built with Gradle CLI/JDK 17.
Gradle preBuild generates payloads/JNI and packages combined libraries for
arm64-v8a/x86_64 using NDK r30/API 29. CoreSession projects Render snapshots on
Android's main thread and ignores callbacks after close. CoreSessionOwner is an
Activity-scoped AndroidViewModel: it supplies Application.filesDir/yomoku.sqlite3,
retains the native session across recreation, and closes it onCleared. It retains
no Activity or independent domain state. Each new session requests Library data;
the database and page-length setting are first loaded on the Rust worker. A fresh
process starts Library.

Compose shows Library titles/current percentage, a multiline paste form and a
centered, scrollable reader column capped at 720dp. Its ScrollState is transient
and keyed by document/page anchor, resetting on page changes/reopen/recreation.
Settings exposes five radio choices, disabled while unknown/busy, and offline
Sudachi licenses/notices. Dialog visibility survives Activity recreation.
Nostr/Statistics/Vocabulary remain placeholders. Busy/error/retry state comes from
Rust; the unsaved form draft and notice dialog visibility use rememberSaveable.
The draft is retained through Settings/recreation.
Navigation uses a bottom bar below 600dp and a scrollable rail above; Settings,
paste and reader routes hide it. Toolbar/system Back return through Rust routes.
Nine instrumentation tests pass on API 36 x86_64 phone; the two new asset/notices
tests also pass on tablet landscape, both runs in airplane mode. The seven prior
tests previously passed on phone and tablet landscape,
including paste/draft/errors, recreation/Back, saved offsets, bidirectional
percentages, scroll resets, Settings reflow and persisted preset reopening.
Automatic Android backup/transfer is excluded. Arm64 execution is unverified.

## Required boundaries and next work
Paste creation is exposed through Android, but complete import semantics remain
unchecked: deduplication, vocabulary registration and timestamps. Page movement/
swipes requiring atomic forward completion/history and real Japanese
analysis/vocabulary repositories remain next. Add analysis/import crates only with
actual behavior; Sudachi Full defaults to B, JMdict/JMnedict remain offline data, and future Nostr I/O stays in Rust. Configuration is empty.
