# Restore candidate validation inventory

Governing requirements: [PRODUCT_SPEC.md §7](PRODUCT_SPEC.md#7-secure-manual-backup-and-restore)
and [BACKUP_FORMAT.md](BACKUP_FORMAT.md). This inventories schema-24 durable state
and the continuation points for semantic checks. It does **not** authorize
confirmation or replacement. Unchecked groups need implementation and hostile-data
coverage using the existing repository/domain contracts before a restore-ready
type can exist. Do not invent stricter product rules while filling these gaps.

- [x] Authenticate every stream frame/Last/EOF/exact manifest length before SQL.
- [x] Validate file/header, exact bundled schema objects and migration journal,
  integrity/CHECKs and foreign keys; migrate supported schemas privately/recheck.
- [x] Validate the control records below before and after private migration.
- [x] Validate vocabulary/custom entry domains before and after private migration;
  status-event/history relations remain a separate unchecked group.
- [x] Decode detached effective records and validate internal domains/projections,
  historical issued-ID bounds and matching receipts before/after migration.
  Surviving text/analysis relations remain in the unchecked groups below.
- [ ] Validate every remaining group below, including cross-group relations and
  legacy/deleted-content cases; only then design an opaque restore-ready result.

## Implemented control records

| Tables | Checks beyond schema/integrity/FKs |
| --- | --- |
| `reader_settings`, `anki_settings` | Exactly one required id=1 row when the table exists; existing CHECKs/STRICT enforce persisted preference types/domains. Missing settings are errors, never repaired. |
| `library_read_clock`, `documents.last_read_sequence` | Required clock row; retained nonnull ordinals are positive, distinct and no greater than the clock. Gaps/deleted-document ordinals and exhausted clocks remain valid. |
| `sqlite_sequence` | Only trusted reference AUTOINCREMENT table names; at most one integer nonnegative counter per table, at least its largest surviving id; a populated table requires its counter. Empty tables may have no counter or retain a deleted-id high-water mark, including i64::MAX. |
| `anki_submissions` | Every row, including terminal states: positive journal id, existing versioned `AnkiSubmission` decoder (exact surface/escaped first field, positive external IDs, complete valid frozen layout). Existing-note IDs decode as a nonempty list of distinct positive i64 values. SQL CHECKs enforce state/known-ID/nullability combinations. A retained known note need not occur in subsequent existing-note evidence. |
| `anki_model_creations` | Every row: positive journal id and existing versioned `AnkiModelCreation` decoder. SQL CHECKs enforce state/ID combinations. When both known and reconciled model IDs exist, they agree. |

No control check rewrites records, canonicalizes historical layouts, compares
them to today's model constants, resolves Anki recovery or creates mutation
permits. Domain decoders reject unknown versions/fields, invalid layouts and
malformed JSON. Diagnostics never echo candidate payloads. All supported schema
prefixes receive the applicable checks before migration can mask bad input.

Counter checks establish consistency with surviving table IDs, not authenticity
of erased history. A maliciously reduced but otherwise plausible high-water mark
cannot be reconstructed after deletion. Surviving historical document/analysis
references inside the groups below still need checking against counters.
[SQLite documents the allocation semantics and writable sequence contents](https://www.sqlite.org/autoinc.html).

## Implemented saved entry domains

| Tables | Checks beyond schema/integrity/FKs |
| --- | --- |
| `vocabulary` | Existing repository entry decoder validates positive typed ID, exact nonblank UTF-8 Surface, all five status spellings and nonnegative i64 registration date. No Japanese detection or normalization. |
| `custom_dictionary_entries` | Version 1, exact nonblank UTF-8 Surface and existing `CustomDictionaryEntry` constructor for nonblank meaning/reading. No required vocabulary registration, live document or built-in dictionary lookup. |

Apply these checks to each supported prefix where its tables exist, then again
after private migration. Preserve literal Unicode/whitespace/NULs permitted by
the domain constructors, inflected spellings, deleted registrations and custom
entries without registered vocabulary. Clock rollback remains accepted; do not
introduce timestamp monotonicity. No candidate record is rewritten or repaired.
Poll cancellation between rows and use fixed diagnostics for malformed data.
Registration/transition chains, final status and completion/date ownership still
require the history checks below; entry success alone grants no restore authority.

## Implemented detached effective records

Version-1 `effective_completion_commands` uses private typed wire records with
required fields, explicit nullable fields, rejected unknown/duplicate fields and
existing core identity/offset/page-length/duration decoders. These records cannot
become live commands. Check command/key/page/position agreement, nonnegative dates,
nonblank profile/reading/dictionary-form domains, ordered nonoverlapping exact-byte
segment lengths, source/chapter ownership, original-B versus alternate-A/C metadata
identity/index consistency, and exact start-owned occurrence projection including
clipping/eligibility/metadata and absent vocabulary snapshots. Check scalar boundaries
where page/position offsets intersect retained segment surfaces.

Document/original/metadata-analysis/override references must fit their surviving
allocation counters even after document deletion. Match any completed receipt's
entire page/time payload, B analysis/profile header and every surviving occurrence's
index/anchor/surface/metadata/vocabulary spelling. Missing encounters remain valid
after vocabulary erasure/re-registration; an unfinished reservation needs no receipt
and is never replayed. Frozen rules need not be active or match today's resolution.

Apply to every supported schema since 18 before and after private migration. Preserve
literal metadata, clock rollback, full signed-64 identities/durations, deleted documents,
crossing tokens and pages without start-owned occurrences. Cancellation polls between
records/segments/occurrences, not inside decoding or SQL. Saved segmentation can omit
whitespace gaps and deleted documents lose their original analyses: detached success
cannot establish original coverage or reconstruct missing bytes. Checking surviving
canonical text, logical page derivation and original/alternate analysis associations
remains required below; so do the other history/status chains. No restore authority.

## Required remaining groups

| Tables | Required semantic review/checks using existing contracts |
| --- | --- |
| `sources`, `chapters`, `documents`, `chapter_texts` | Text/identifier types and domain values; document/source/chapter ownership; contiguous chapter order; required live text/position/format records. Retained deleted-source provenance is valid without live text/documents. Original titles and renamed titles remain distinct. |
| `reading_positions`, `pending_reading_time` | Document/source/chapter ownership, valid UTF-8 byte boundaries/range, accepted EOF/legacy positions and duration/revision domains; preserve zero-duration revision tombstones. |
| `paste_content_hashes`, `imported_files` | Hashes versus exact canonical/original bytes, source-format/encoding/metadata ownership and record presence; preserve exact original bytes. Validation must not reimport or allocate new identities. |
| `source_ruby`, `subtitle_cues` | Contiguous indices, ordered UTF-8 ranges/reading domains and relation to source text and format. |
| `epub_documents`, `epub_assets`, `epub_chapters` | Counts, ownership, canonical paths/media/cover references, asset/text SHA-256 and existing size limits. No extraction or external URL resolution. |
| `epub_spans`, `epub_inline_images` | Contiguous indices, UTF-8 anchors, kind/target/link policy and image placeholder/path/asset relations through the existing domain contracts. |
| `chapter_analyses`, `analyzed_tokens` | Version/identity/revision domains, contiguous indices, complete ordered original analysis coverage, exact anchored surfaces/metadata and Japanese flags. Preserve explicit legacy analysis absence; no analyzer rerun. |
| `segmentation_overrides`, `segmentation_override_parts` | Exact anchored surfaces, scope/provenance, ordered parts reconstructing the selection, domain split/merge constraints and active-rule consistency. Deleted originating text must not invalidate retained global rules. |
| `completion_commands`, `reading_completions`, `completion_analyses`, `reading_occurrences` | Historical ownership, immutable ranges/analysis identities and occurrence membership/surfaces, page/time semantics and surviving counter lower bounds. Account for vocabulary deletion removing encounters and document deletion removing live text/analysis. Legacy unanalyzed completions remain valid. |
| `effective_completion_commands` | Detached typed payload/internal projection/issued bounds/receipt checks implemented above. Still check surviving canonical text, logical pages and original/alternate analysis associations with the text/analysis groups. Preserve deleted-content records and old frozen rules without current override/vocabulary requirements. Never replay reservations. |
| `vocabulary_status_events` | Registration/transition chain and final retained status, cause/completion ownership/date relations using existing event-order rules. Vocabulary deletion legitimately removes its events/occurrences. |
| `documents.finished_at_least_once` | Permanent marker/history consistency under existing completion and legacy migration semantics; backward position/reflow do not clear it. |

Migration 0023 removes `vocabulary_dictionary_forms`; it is not a current durable
table. Earlier candidates must still match their exact migration prefix before
the trusted removal. No user dictionary/analyzer asset copies, routes, caches,
callbacks, worker permits or reader clock epochs belong in candidate state.

## Operational boundary still required

Semantic success must remain separate from session-local confirmation, current
installation Anki recovery, checkpoint drain/mutation gating, secret transport,
private crash cleanup, descriptor/UI lifecycle, atomic activation and interrupted
replacement recovery. Poll cancellation between records and synchronous calls;
SQL/JSON/sync are not interruptible here and total resource use is not bounded by
the per-record checks. Real-device resource trials remain required before shipping.
