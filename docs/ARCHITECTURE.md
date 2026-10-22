# Architecture

## Implemented
`yomoku-core` is a pure Crux 0.20.0 application. It owns Library-default
navigation, Settings visibility, storage busy/error state, Library summaries and
an optional loaded paste snapshot. SelectDestination/OpenSettings/CloseSettings
remain synchronous. RefreshLibrary/CreatePaste/LoadPaste emit StorageOperation;
only one storage command is accepted at a time, while navigation stays available.
StorageFinished is internal: both Serde and Facet skip it, so hosts cannot inject
storage results. No SQL, paths, threads or platform I/O enter core.

Pure document values define positive i64 identities, immutable chapter snapshots,
UTF-8 byte anchors, checked UTF-16 conversions and literal first-line paste titles.
Vocabulary values define exact-surface identity, five statuses, registration and
New-only completion policy; tokenization/persistence remains future work.

`yomoku-runtime` composes Core → HandleEffectLayer<StorageEffects> → MapEffectLayer.
A named FIFO worker owns one lazily opened SQLite connection per bridge. Opening,
migration and create/load/list execute on that worker. Failed opens can retry;
database failures become typed DocumentError view state without internal strings.
Only Render reaches ShellEffect. Dropping the bridge closes the work channel;
submitted work finishes atomically and the worker exits without joining on the UI
thread. Crux discards results when the session has gone away. There is no new
async executor or dependency version.

`yomoku-storage` owns rusqlite 0.40.2/bundled SQLite, migration history and SQL.
Migration 0002 separates source/chapter provenance, Library documents and canonical
text. create_paste commits all four inserts atomically; load_paste reads a
consistent snapshot; list_documents returns IDs/titles in allocation order until
Library sorting is added. No edit/rename/delete API exists. Twenty-one storage
tests cover migrations, reopen, rollback, Unicode, concurrency and ID allocation.

`yomoku-ffi::YomokuBridge` wraps the router in Crux's Bincode bridge.
BoltFFI 0.30.1 NativeBridge takes a database path and owned EffectsListener,
exposes update/resolve/view and reports worker creation failure as WorkerUnavailable.
Callbacks carry asynchronous Render batches; document errors are view values.
Core-only effects stay private. Facet generates Event/ViewModel/Requests and
document codecs in app.yomoku.shared; BoltFFI generates the native package and JNI C.
Native wrappers require explicit close; no global singleton exists.

Forty-nine Rust tests cover domain, storage and bridge behavior, including an
actual SQLite write lock proving dispatch/view/navigation remain responsive.
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
the database is first opened on the Rust worker. A fresh process starts Library.

Compose currently shows four placeholders and toolbar Settings with system theme
and insets. Navigation uses a bottom bar below 600dp and a scrollable rail above;
Settings hides it. Four tests pass on API 36 x86_64: JNI/navigation, Settings/Back,
recreation/cleanup, private-file persistence/failure recovery. Arm64 execution is
unverified. Automatic Android backup/transfer is excluded.

## Required boundaries and next work
Paste creation is repository/runtime plumbing, not the complete product import
pipeline or an exposed UI flow. Deduplication, vocabulary registration, progress,
pagination, paste UI/Library rendering and the reader route remain unchecked.
Hide navigation when the real reader route is added. Add analysis/import crates
only with actual behavior; Sudachi Full defaults to B, JMdict/JMnedict remain
offline data, and future Nostr I/O stays in Rust. Configuration is empty.
