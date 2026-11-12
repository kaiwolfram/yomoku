# Architecture

The [product specification](PRODUCT_SPEC.md) defines behavior. Yomoku has one
production reader and a versioned release schema.

## Ownership

| Component | Responsibility |
| --- | --- |
| Kotlin/Compose | Rendering, lifecycle, file descriptors, permissions, AnkiDroid ContentResolver and browser intents |
| `yomoku-core` | Pure Crux model/events, pagination, dictionary projection, vocabulary rules and in-memory operation ownership |
| `yomoku-runtime` | EffectRouter, lazy analyzer/dictionary loaders and one FIFO SQLite/analysis worker |
| `yomoku-storage` | SQLite repositories, atomic imports/completions, schema runner and private backup candidates |
| `yomoku-analysis` / `yomoku-import` | Sudachi adapter and pure EPUB/TXT/SRT parsing |
| `yomoku-ffi` | Narrow typed Crux/BoltFFI interface and generated Kotlin codecs |
| `yomoku-backup` | Bounded authenticated snapshot streams, separate from app restore orchestration |

Core has no SQL or platform I/O. `YomokuBridge::new` and `effect_router` require
both dictionary loaders; imports always analyze atomically. There is no text-only
fallback or alternative reader constructor. Full and word dictionaries load lazily
on the worker. Android verifies immutable bundled assets before providing them.
Lifecycle background events release Full after queued work, without eager reload.

## Production reader

`Model::default()` owns an `EffectiveReaderSession`. Stored Mode C analysis is
the reader's segmentation. For new imports the version-2 adapter joins conjugated
verb/adjective endings and allowlisted non-independent helper chains before storage;
existing C snapshots remain immutable. Raw A/B components can still be generated
under the original version-1 profile with the unchanged pinned engine/dictionary.
A/B analyses remain available for dictionary components;
they do not select another reader. The native session retains chapter data and
validates each page. The shell receives a compact page and revision identity;
selections contain that identity and a segment index. Stale callbacks cannot attach
to a later page or session. UTF-8 anchors remain canonical; Compose validates and
converts only displayed ranges to UTF-16, including crossing tokens and ruby.

Dictionary lookup uses bundled JMdict/JMnedict only. Empty results show the
existing no-match state. There is no custom-entry editor, fallback or export.
Status changes commit through the worker and update repeated surfaces globally.
Shared text styling, links, images and ruby rendering remain independent of word
selection. Whole-document payloads are not part of the live view.

Completion freezes identity and reserves a command ID. The
stored version-2 reservation contains the page command, Mode C analysis identity/
profile and counted occurrences; it omits whole-chapter segmentation. Retry uses
the same reservation. Completion atomically commits history, status changes
and position; deletion-safe receipt lookup prevents duplicate counting. Explicit
rereads use new commands and count again. Previous moves position without counting.
Reading durations are not collected or persisted.

The worker uses whole-document snapshot/open/position operations and effective
page/word lookups. Session furigana combines source-preferred ruby with validated
visible tokens.

| Operation | Production API |
| --- | --- |
| Snapshot / explicit entry | `load_document_reader_snapshot` / `open_document_reader_snapshot` |
| Position | `save_document_reading_position` |
| Page / selected word | `load_effective_reader_tokens` / `load_effective_reader_word` |
| Completion | `prepare_effective_page_completion` / `record_effective_page_completion` |
| Furigana | `EffectiveReaderSession::furigana` |

## AnkiDroid

Settings persists only the default deck. Prepared fields, current duplicate queries,
setup observations and active operation ownership live in memory. A typed provider
effect consumes a Ready invocation once; attempt/allocation identity rejects repeated
taps and stale callbacks. No SQLite begin/finish transaction surrounds external writes.
Queries refresh on dictionary lookup, return to the app and immediately before
insertion. Read-only observations can refresh from a validated selection during
reader I/O; explicit insertion still requires the reader mutation gates. Failed/incomplete queries remain errors. Every write revalidates provider
permission, deck and model. Existing-note buttons use fresh verified note IDs.

An interruption can leave a partial external write. A later explicit retry starts
with fresh queries; startup never replays a write. There is no durable export log,
reconciliation protocol or cross-process exactly-once promise. Compatible existing
models can be reused; incompatible/partial same-name models are never overwritten.
See [ANKIDROID_API.md](ANKIDROID_API.md) for provider details.

## Persistence and builds

The [data model](DATA_MODEL.md) contains 27 application tables. The initial schema
includes the Statistics preference. A transactional migration runner/journal remains
for future releases; historical development files are rejected without clearing them. Bundled dictionaries are separate read-only assets.

Gradle debug/release variants select matching Rust profiles and separate generated
outputs. Native generation tasks serialize and use a cross-process lock because
BoltFFI metadata expansion shares Cargo artifacts. Release includes both supported
ABIs and excludes debug probes. See [DEVELOPMENT.md](DEVELOPMENT.md).

Encrypted backup primitives and private validation exist. App secret handoff,
confirmation, crash cleanup and atomic replacement remain separate release work;
there is no Anki recovery prerequisite. See [BACKUP_FORMAT.md](BACKUP_FORMAT.md).
