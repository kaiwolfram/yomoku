# Restore candidate validation

This inventories current **schema 1** validation. It does not authorize activation.
See [BACKUP_FORMAT.md](BACKUP_FORMAT.md) and [PRODUCT_SPEC.md §7](PRODUCT_SPEC.md#7-secure-manual-backup-and-restore).
Development databases/backups are unsupported; the exact initial schema/journal
must match `0001_simplified_initial_schema`, including the Statistics preference
singleton. Previous schema-1 journals,
unknown/extra schema objects and incompatible versions fail unchanged.

## Implemented checks

| Area | Checks after authentication, before any replacement |
| --- | --- |
| Container | Complete frame/final-tag/EOF authentication, exact manifest length/schema; private owned staging |
| SQLite | Header/page/length limits, hardened no-CREATE connection, exact trusted schema objects/journal, integrity/CHECK/index/FK validation |
| Controls | Required reader/deck/recency/statistics singletons, recency ordering, trusted AUTOINCREMENT sequence domains and surviving issued-ID lower bounds |
| Vocabulary | Typed exact UTF-8 surface/status/timestamp domains without normalization or dictionary lookups |
| Compact completion | Strict version-2 payload shape, page command, C analysis/profile/mode, ordered counted occurrence domains, live text/page/analysis and completed-receipt consistency |
| Content | Source/document/chapter ownership, contiguous chapter ordering, format/text shape, UTF-8 titles/nonblank live text and representable totals |
| Positions | Ownership and scalar boundaries/range including EOF |
| Analysis | Profile/mode, contiguous indices, range/coverage/scalar/surface/classification/reading/form domains; no analyzer rerun |
| Import identity | Exact canonical-paste/original-file SHA-256, encoding/format ownership, required file/EPUB headers |
| TXT/SRT/ruby | Pure saved-encoding derivation matches canonical text and TXT ruby; SRT cue parsing remains transient; ruby indices/ranges/readings obey existing contracts |

Compact reservations contain no whole-chapter segmentation. They retain C provenance
and counted occurrences; surviving text/analysis validate the frozen page without
reinterpreting history. Deleted content/vocabulary can remove corresponding live
rows; unfinished reservations are never replayed. Typed decoders reject unknown,
duplicate or missing fields. Candidate validation never imports content, allocates
IDs, repairs data, contacts AnkiDroid or mutates the live database.

## Remaining release work

EPUB metadata/assets/chapter hashes/counts/spans/images and their ownership/link/
placeholder relations still require complete semantic validation. Completion and
status-event chains, historical cross-group relations and permanent finish-marker
consistency also require further review/hostile-data coverage. Schema checks and
local domains alone are not a restore-ready result.

`SchemaCheckedBackup` remains opaque and exposes no activation authority. App secret
handoff, confirmation correlation, reader mutation/worker drain, mutation gating, private
crash cleanup, Android descriptors/UI, atomic activation/sidecars and interrupted
replacement recovery remain unfinished. Cancellation polls between synchronous
operations/rows; SQL/JSON/parsing/sync are not interruptible and there is no total
resource budget. Real-device resource trials remain required.

There are no custom entries or Anki journals to validate and no Anki recovery gate.
A saved default deck is merely a preference; fresh provider checks are required
on later explicit use. Restore never replays external writes.
