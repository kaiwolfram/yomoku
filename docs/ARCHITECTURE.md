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
the FIFO; it is separate from read-only reader opening. SavedReadingTime is a pure
protocol value validated against SQLite; only its anchored duration reaches the
shell view. Core freezes the checkpoint plus prior revision until success, blocks
other storage commands while unresolved and retains retries even after reader exit.
Late callbacks cannot restore closed-reader state. Reflow/same-offset saves preserve
time; moving to a different offset requires a fresh restore. No SQL, clock or
analyzer enters core. Automatic final checkpoints, completion sequencing and
platform clock/lifecycle/interactions/periodic scheduling remain separate work.

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
cannot reopen a closed reader. Page controls/swipes and atomic forward completion
remain next. Settings reflow keeps the exact position and Library percentage;
only committed values change the preset/page. An independent active-document
snapshot prevents repository loads from changing the text used for reader reflow.
Opening now loads content/position, the preset and the original stored Mode B
AnalysisId in one read transaction. The earliest Mode B snapshot is pinned,
independent of the installed engine/dictionary or later snapshots. Only its
header is read; completion still validates tokens and anchors transactionally.
ReaderAnalysis::Missing explicitly identifies legacy content with no Mode B base;
it remains readable without analyzer initialization, rescan or vocabulary writes.
The optional view field is absent outside an opened reader. Settings/position
saves/plain loads preserve its identity; reader exit/replacement clears it, and
cancelled loads cannot attach it. Corrupt metadata returns a typed error without
fallback/repair. No forward/completion action or legacy-completion policy yet.

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
work and drops both analyzer and loader. Eight data-free tests use a deliberately
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
fresh ID; position-only saves never add history. These repository methods have no
app event/runtime/FFI entry point yet. Migration 0010 supplies the analyzed
transaction below; platform checkpoints/forward controls remain open. Migration 0009 adds
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
deletion. No production document edit/rename/delete API exists. One hundred and two storage
tests cover migrations, reopen, rollback, Unicode, concurrency, ID allocation,
settings, vocabulary, analysis anchors and completion history. Only base-analysis identity reaches runtime/FFI today; token/override
repositories have no UI operations; synthetic fixtures validate storage,
not Sudachi behavior. Cross-occurrence matching and overlapping edit resolution
belong to the later segmentation application task.

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

One hundred and ninety-one data-free Rust tests cover analysis classification/
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
process starts Library with no dictionary load. Activity onStart permits reuse;
onStop queues release unless isChangingConfigurations, preserving the analyzer
through retained recreation. Accepted imports finish atomically before release;
hidden imports release before resolving. Final native close drains accepted work
and drops its analyzer/provider without joining Android's UI thread.

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
Nineteen instrumentation tests pass on API 36 x86_64 phone (including production
analyzed imports/lifecycle, eight private-extraction tests and the debug-only
in-app analyzer probe). The two
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
lifecycle are connected. TXT/EPUB/SRT parsing, metadata and multi-file import
semantics remain unchecked. Legacy imports are not automatically rescanned.
Explicit core/worker/FFI pending-time checkpoint transport is verified; automatic
final-checkpoint/completion sequencing, Android monotonic lifecycle/interactions
and page movement/swipes remain next. The analyzed transaction is
repository-only; effective segmentation edits and statistics aggregation remain
separate tasks. Add analysis/import crates only with
actual behavior; Sudachi Full defaults to B, JMdict/JMnedict remain offline data, and future Nostr I/O stays in Rust. Configuration is empty.
