# Data model

The release schema version is **2**. The SQL lives in
[0001_completion_receipts_initial_schema.sql](../crates/yomoku-storage/migrations/0001_completion_receipts_initial_schema.sql)
and [0002_library_read_override.sql](../crates/yomoku-storage/migrations/0002_library_read_override.sql).
It has **25 application tables** plus SQLite's internal `sqlite_sequence` table.
Databases with the `0001_completion_receipts_initial_schema` journal upgrade in
place. Earlier development journals remain unsupported.

| Area | Tables |
| --- | --- |
| Content | `sources`, `chapters`, `documents`, `chapter_texts`, `paste_content_hashes`, `imported_files`, `epub_documents`, `epub_chapters`, `epub_spans`, `epub_assets`, `epub_inline_images` |
| Analysis and vocabulary | `chapter_analyses`, `analyzed_tokens`, `vocabulary`, `vocabulary_status_events` |
| Progress and history | `reading_positions`, `library_read_clock`, `completion_commands`, `reading_completions`, `completion_analyses`, `reading_occurrences` |
| Settings and schema | `reader_settings`, `anki_settings`, `statistics_settings`, `schema_migrations` |

## Initialization and compatibility

The small migration runner opens an immediate transaction, reads the journal and
applies missing migrations in order. Concurrent opens serialize. Foreign keys are
enabled on every repository connection. Unknown, future and historical development
journals are errors. The runner does not reset incompatible databases, delete user
files or backfill development data.

## Content and analysis

A live document owns one source and ordered canonical chapter text. File imports
retain the input SHA-256 for duplicate detection and discard the input bytes.
TXT/SRT retain the selected encoding and a digest of the canonical UTF-8 text.
EPUB retains ordered spine chapters, formatting, and extracted image assets.
Pastes use a digest of their exact text. Duplicate imports preserve existing
title, status and progress.
Deletion removes live content while preserving supported historical provenance.
Only individually confirmed document deletion is exposed in the app. Vocabulary
rows open word sheets with individual status controls; there is no bulk action.

SRT cue parsing is transient: canonical text preserves blank-line separators.
EPUB stores heading/emphasis/strong and HTTP(S) link spans. Internal and unsupported
links contribute visible text without annotation rows. Recognized EPUB/Aozora reading
annotations are discarded while preserving their base text; no ruby metadata is stored.

Offsets are UTF-8 byte positions at scalar boundaries, never UTF-16 indices.
Analysis headers retain engine/dictionary profile and A/B/C mode; indexed tokens
retain exact surface, anchor, classification, reading and dictionary form. The
production reader uses C. A/B support dictionary components. Reading never silently
replaces stored analysis with a newer dictionary. Bundled dictionaries live outside
the user database and remain read-only.

Vocabulary uses exact surface identity. Imports register previously unseen Japanese
surfaces without changing existing status. Explicit status history supports the
five statuses. Deleting vocabulary removes its events/occurrences without causing
existing books to register it again. There are no custom dictionary records.

## Library read status

`documents.read_override` is nullable. The effective Library status uses the
override when present and otherwise inherits `documents.finished_at_least_once`.
Migration 2 leaves the override NULL for existing books. Manual marking updates
only the override on the live document. The permanent finish marker still
governs whether opening the reader resets the position to page one.

## Completion

`prepare_effective_page_completion` validates the rendered page and allocates an ID
in `completion_commands` with a 32-byte SHA-256 fingerprint covering the page
command, Mode C analysis provenance and accepted occurrences. The frozen command
lives in the native session.

`record_effective_page_completion` verifies the pending fingerprint and clears it
in the transaction that writes `reading_completions`, `completion_analyses`,
occurrences, New-to-Known transitions, and progress. On the final page it also sets
the finish marker and clears the Library read override. A retry of
a committed command matches its ID, page, departing position, page length,
timestamp and analysis provenance against the receipt. A replay preserves later
manual read changes and also works after content deletion. A deliberate reread
uses a new command ID and counts again. Statistics and Vocabulary query
committed history directly.
Previous/reflow preserve exact position without recording completion. No reading
durations or pending-time state are persisted.

## Settings

The singleton reader settings default to 250 characters, 16 sp font, 150% line
spacing. The 250-character seed applies only to fresh databases; saved
preferences, including 500 and legacy 1,500, survive reopening without migration
or automatic reflow. Page length is independent of typography. Library read
recency uses a persisted monotonic sequence.

The entire Anki database state is:

```sql
CREATE TABLE anki_settings (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    default_deck_id INTEGER CHECK (default_deck_id > 0)
) STRICT;
INSERT INTO anki_settings VALUES (1, NULL);
```

The saved deck is a preference, not proof of availability in the current collection.
Prepared notes, duplicate results and active provider ownership are transient.
New AnkiDroid note insertions include `yomoku` in the provider’s space-separated
`tags` value; existing duplicate notes are never retagged.
There are no submission/model-creation journals, export history or startup replay.

## Statistics preference

The initial schema includes `statistics_settings`, a required singleton (`id = 1`) with a
validated timeframe: `last_week`, `last_month`, `last_six_months`, `last_year`, or
`all_time`. New databases default to `last_week`. Native typed
read/save effects share the SQLite FIFO; one preference request runs at a time,
coalescing later selections even after navigation. Statistics Retry retries failed
preference operations. Missing or invalid settings are errors and are never repaired.

Statistics maps the initial schema’s `last_six_months` preference slot to the rolling
three-month `LastQuarter` interval. The slot and schema stay unchanged so saved
selections survive without a database migration.
