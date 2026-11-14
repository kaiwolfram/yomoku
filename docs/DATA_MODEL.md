# Data model

The authoritative SQL is [0001_base_text_initial_schema.sql](../crates/yomoku-storage/migrations/0001_base_text_initial_schema.sql).
The release schema version is **1**, journal name `0001_base_text_initial_schema`.
It has **26 application tables** plus SQLite's internal `sqlite_sequence` table.
Earlier development schemas and journals are unsupported, including the previous
schema-1 journal. They are rejected without migration or modification.

| Area | Tables |
| --- | --- |
| Content | `sources`, `chapters`, `documents`, `chapter_texts`, `paste_content_hashes`, `imported_files`, `epub_documents`, `epub_chapters`, `epub_spans`, `epub_assets`, `epub_inline_images` |
| Analysis and vocabulary | `chapter_analyses`, `analyzed_tokens`, `vocabulary`, `vocabulary_status_events` |
| Progress and history | `reading_positions`, `library_read_clock`, `completion_commands`, `effective_completion_commands`, `reading_completions`, `completion_analyses`, `reading_occurrences` |
| Settings and schema | `reader_settings`, `anki_settings`, `statistics_settings`, `schema_migrations` |

## Initialization and compatibility

The small migration runner opens an immediate transaction, reads the journal and
applies the initial schema once. Concurrent opens serialize. Foreign keys are
enabled on every repository connection. Unknown, future and historical development
journals are errors. The runner does not reset incompatible databases, delete user
files or backfill development data. Additional upgrades can add migrations.
Backup candidates must match the exact trusted schema and journal before adoption.

## Content and analysis

A live document owns one source and ordered chapter text, with original imported
bytes and canonical text/annotations/images retained. TXT/SRT retain explicit saved
encoding; EPUB retains asset metadata/bytes and ordered spine chapters. Paste
SHA-256 is saved in the same transaction as the paste; imported-file identities use
original bytes. Duplicate imports preserve existing title, status and progress.
Deletion removes live content while preserving supported historical provenance.
Only individually confirmed document deletion is exposed in the app. Vocabulary
rows open word sheets with individual status controls; there is no bulk action.

SRT cue parsing is transient: canonical text preserves blank-line separators.
EPUB stores heading/emphasis/strong and HTTP(S) link spans. Internal and unsupported
links contribute visible text without annotation rows. Original bytes, saved
encoding/hashes, images and history remain durable. Recognized EPUB/Aozora reading
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

## Completion

`prepare_effective_page_completion` validates the rendered key and allocates retry
identity in `completion_commands` within the reservation transaction.
`record_effective_page_completion` is the sole completion writer. A version-2
`effective_completion_commands` payload freezes the page command, analysis ID,
profile, mode C and counted occurrences. It excludes whole-chapter text, token
segmentation and unrelated chapter content. `completion_analyses` records actual
C provenance. Receipt/history semantics remain version 1; these version numbers
are distinct from the database schema version.

One transaction applies occurrence counts, New-to-Known transitions, completion
history and next position/finish marker. Retry of the
same command is idempotent, including after content deletion. A deliberate reread
uses a new command and counts again.
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

## Backup boundary

A compact snapshot retains all the tables and ID sequences above. It excludes
bundled dictionaries and all transient provider/session state. Incompatible
pre-release databases/backups are rejected without clearing them. Restoring never
replays Anki writes and has no Anki recovery gate. Full manual encrypted restore
is unfinished; [RESTORE_VALIDATION.md](RESTORE_VALIDATION.md) states its limits.

## Statistics preference

The initial schema includes `statistics_settings`, a required singleton (`id = 1`) with a
validated timeframe: `last_week`, `last_month`, `last_six_months`, `last_year`, or
`all_time`. New databases default to `last_week`. Native typed
read/save effects share the SQLite FIFO; one preference request runs at a time,
coalescing later selections even after navigation. Statistics Retry retries failed
preference operations. Missing or invalid settings are errors and are never repaired.
Backup validation requires the exact initial schema and all settings singletons.

Statistics maps the initial schema’s `last_six_months` preference slot to the rolling
three-month `LastQuarter` interval. The slot and schema stay unchanged so saved
selections survive without a database migration.
