# AnkiDroid integration evidence

Reviewed 2026-10-31 for PRODUCT_SPEC.md §6. Primary-source inspection governs the
adapter below; no AnkiDroid installation, permission grant, real collection
access or export has run yet.
The official [latest release](https://github.com/ankidroid/Anki-Android/releases/tag/v2.25.3)
resolved to **2.25.3**, commit prefix `d9bf072`. Links below use that release tag.
The older [API guide](https://github.com/ankidroid/Anki-Android/wiki/AnkiDroid-API)
still advertises `api-v1.1.0`; do not infer that it packages current Kotlin APIs.
No dependency or minimum supported AnkiDroid app version is adopted in this step.

## Supported surface

The release [AddContentApi](https://github.com/ankidroid/Anki-Android/blob/v2.25.3/api/src/main/java/com/ichi2/anki/api/AddContentApi.kt)
exposes:

| Need | Actual API / limit |
| --- | --- |
| Availability | `getAnkiDroidPackageName(context)` resolves the provider; `apiHostSpecVersion` reads provider metadata. |
| Decks | `deckList`, `getDeckName`, `addNewDeck`; IDs are `Long`. |
| Models | `modelList`, `getFieldList`, `addNewCustomModel`; custom fields, card templates, CSS, default deck and sort-field index. |
| Add | `addNote(modelId, deckId, fields, tags)` returns a nullable note ID; performs no duplicate check. |
| Duplicates | `findDuplicateNotes(modelId, key)` checks the first field within that model. Spec 2 filters checksum candidates and then compares actual field keys. |
| Existing note | `getNote(noteId)` reads fields. This wrapper provides no open-editor method. |

Spec 1 documents custom-model persistence/sort-field limitations. Plan the adapter
around **provider spec 2 or newer**, checking capability on use. This is a proposed
compatibility gate, not a claim that every historical spec-2 release was tested.

The release [provider contract](https://github.com/ankidroid/Anki-Android/blob/v2.25.3/api/src/main/java/com/ichi2/anki/FlashCardsContract.kt)
defines authority `com.ichi2.anki.flashcards`, permission
`com.ichi2.anki.permission.READ_WRITE_DATABASE`, `notes`, `notes_v2`, `models`,
deck/card URIs and note `_id`, `mid`, `flds`, `csum` columns. Fields are joined by
U+001F; a literal separator must not create an extra field. `notes_v2` accepts
SQL selection rather than browser search syntax. Its
[implementation](https://github.com/ankidroid/Anki-Android/blob/v2.25.3/AnkiDroid/src/main/java/com/ichi2/anki/provider/CardContentProvider.kt)
passes selection arguments through to SQLite. Keep all such queries in the
Android adapter, through ContentResolver; never open AnkiDroid's files directly.

Android [package visibility](https://developer.android.com/training/package-visibility/declaring)
requires a suitable manifest query; use the known provider/package, not broad
package discovery. The release
[manifest](https://github.com/ankidroid/Anki-Android/blob/v2.25.3/AnkiDroid/src/main/AndroidManifest.xml)
declares database access as dangerous. Request it in context and check it for
each operation, handling denial/revocation while preserving local reading, per
the [Android permission workflow](https://developer.android.com/training/permissions/requesting).

## Feasible exact-surface duplicates and opening

The following are implementation conclusions from those supported queries, not
an additional API guarantee:

- Cross-model lookup is feasible through `notes_v2` without a model restriction.
  Read IDs/model/fields and compare the **entire first field** against the exact
  written surface representation. This can find externally created plain-text
  first-field vocabulary notes as well as Yomoku notes. Deck must not scope this
  lookup. Use bound arguments and a small projection; measure query cost before
  putting it on popup entry. Do not scan every field as though any occurrence of
  the word proved note identity.
- Recognize Yomoku's escaped first-field representation. For external HTML,
  formatting/entities require a deliberate, conservative decoding policy and
  fixtures. Combined word/reading fields, unknown field layouts, non-first-field
  vocabulary, arbitrary HTML and media are **not guaranteed** matches. Never
  normalize kana/Unicode, trim, case-fold or substitute dictionary forms to claim
  an exact duplicate. Checksum/search results alone do not establish equality.
- A failed, null or unavailable query is not evidence of no duplicate. Expose
  Retry; do not silently proceed to insert. Recheck immediately before adding.
  Query and insertion are separate provider calls; the reviewed interface has no
  atomic cross-application check-and-add guarantee. A lost insertion response
  requires reconciliation before Retry; never delete a discovered external note.
- The exported browser link is
  `anki://x-callback-url/browser?search=<URI-encoded search>`; its
  [handler](https://github.com/ankidroid/Anki-Android/blob/v2.25.3/AnkiDroid/src/main/java/com/ichi2/anki/IntentHandler.kt)
  passes the `search` query parameter to the browser. An exact
  [note-ID search](https://docs.ankiweb.net/manual/searching#object-ids)
  (`nid:<note-id>`) is the candidate existing-note action, subject to installed-version
  resolution and an integration test. The browser Activity itself is unexported;
  do not invent an explicit editor Activity/extra contract. The handler may first
  route an unconfigured collection to setup; return-to-reader behavior needs testing.

## Implemented preparation and continuation

`yomoku-core::anki::AnkiNote` now prepares four ordered, escaped HTML fields and
retains the raw `Surface` independently. Original `Yomoku v1` templates produce
one word-front card; the back contains reading, ordered English meanings and the
literal supplied sentence with only the selected UTF-8 occurrence highlighted.
HTML, bracket media syntax and U+001F are escaped. No audio or source formatting
is added. Empty reading/meanings remain empty. Control characters retained in the
raw identity do not imply that an HTML renderer can display them losslessly.
Template field substitution and `FrontSide` follow the
[Anki template contract](https://docs.ankiweb.net/manual/templates/fields).

This is a pure content boundary. Original/effective selection constructors now
validate the owner's current page/selection against dictionary details and
canonical text, extract a full chapter-local sentence and project licensed
readings/English senses or the permitted custom fallback. They preserve complete
cross-page words, exact surface identity and one occurrence highlight. See
ARCHITECTURE.md and DECISIONS.md for sentence/definition projection rules.
Callers must supply their actual current selection. `AnkiExportSession` now freezes
the prepared note, selection and external deck ID across typed requests/results.
Fresh attempts plus owning allocation identity reject stale/repeated/other-session
callbacks. Only known pre-insert failures allow a mutating Retry. Uncertain insert
outcomes require read-only reconciliation; an empty/failed read never establishes
safe insertion, and clear/replacement cannot erase the obligation. Verified exact
existing-note IDs resolve it. This is a pure in-memory prerequisite, not app wiring;
no Event, FFI type, settings, schema, dependency or UI action is added. Preparation
and export have no mutable vocabulary/history input or effect.

The read-only Android prerequisite is now `AnkiProviderReader`. It checks current
provider metadata/spec >=2, enabled/exported state and permission before every
operation, enumerates decks without selecting/creating one, queries exact-surface
duplicates and reconciles read-only. Shell-local results translate to the typed core recovery protocol. The manifest
declares only the known provider visibility query and database permission; no
permission prompt or automatic provider read is enabled. Explicit recovery reads
now dispatch through CoreSession; live dispatch requires a committed native permit,
and visible actions remain gated.

Duplicate candidates are the exact Yomoku escaped first field and, when it has no
HTML/entity/media/delimiter syntax, the literal plain surface. Bound `flds = ?`
or `instr(flds, <candidate plus U+001F>) = 1` filters match the entire first field,
including one-field notes; returned fields are compared again in Kotlin. No
model/deck/checksum restriction, Unicode normalization or arbitrary HTML decoding
is used. Embedded NUL input fails closed because lossless Android SQLite string
transport is not established. This query may scan notes: collection-size cost is
unmeasured and it is not dispatched on popup entry. Null/malformed/partial/failed
reads never produce a successful empty observation. Even a successful empty read
maps to unresolved reconciliation, preserving the core's no-insert obligation.

The reviewed provider's `addDeckToCursor` puts a Boolean into `deck_dyn`, despite
the contract's documented integer 0/1. Accept exact `true`/`false` string cursor
representations as well as integer 0/1, rejecting other forms. Existing-note links
use the current provider package, URI-encoded `nid:<positive signed-64 ID>` and
installed-handler resolution. The adapter returns the resolved intent without
launching an Activity. Resolution is not proof of actual browser/return behavior.
Android's supported [ContentResolver.wrap(ContentProvider)](https://developer.android.com/reference/android/content/ContentResolver#wrap(android.content.ContentProvider))
allows in-process fake-provider tests on minSdk 29; these use real Android SQLite
for bound query semantics and injected permission/metadata/resolution observations.
They do not establish real Binder/provider/runtime compatibility.

The read-only reader also enumerates model IDs/names and validates a specific
model ID against caller-supplied core layout data. The release contract and
provider expose `models/<id>` metadata and `models/<id>/templates` rows. Require
the exact name, ordered four fields, CSS, normal type, first-field sorting and
one forward template with exact name/question/answer and empty browser overrides.
Use zero-based `ord`, not synthetic template `_id`. Discovery preserves same-name
models and does not choose one; incompatible/missing models return false without
mutation. Failed/malformed reads remain failures. Recheck host/permission between
queries. Default model deck is deliberately not a layout constraint: the future
adapter must place cards in the frozen configured deck explicitly. Validation is
an observation, not an atomic reuse guarantee, and must run again before mutation.
No production template constants are copied into Kotlin; the future typed bridge
must supply Rust's layout. The reader never creates, repairs or overwrites models.

The separate worker-only `AnkiProviderModelCreator` follows the release wrapper's
two-stage creation: insert metadata/four fields/one dummy template, then update
only that new ID's ordinal-zero template. Discovery refuses exact-name conflicts
without overwriting or renaming user models. Validate the returned content URI,
positive signed-64 ID and absence from preflight discovery before updating. Send
normal type/sort field zero and empty browser overrides; do not set a default deck
or select one. The reviewed provider counts updated template **properties**, so
a positive count requires full metadata/template readback before success. Refresh
host/spec/permission between calls and after readback. Null/exception/invalid
response after submission, failed updates, mismatches and failed readback all
return uncertainty, preserving a validated new ID when known. No retry, repair or
delete is attempted: a partial/orphan model may remain. Concurrent collection edits
are not fenced. Fake-provider coverage does not establish real-provider behavior.

The worker-only `AnkiProviderNoteInserter` revalidates layout, an existing
nonfiltered frozen deck and exact duplicates before submission. The reviewed
[`addNoteForContentValues`](https://github.com/ankidroid/Anki-Android/blob/v2.25.3/api/src/main/java/com/ichi2/anki/api/AddContentApi.kt)
inserts a note, then moves its cards: there is no deck column for single-note
insertion. Validate the returned canonical URI and exact note ID/model/fields,
require one ordinal-zero card, update only its `deck_id` and verify note/card/deck
readback. The
[provider](https://github.com/ankidroid/Anki-Android/blob/v2.25.3/AnkiDroid/src/main/java/com/ichi2/anki/provider/CardContentProvider.kt)
initially uses the model default deck; the adapter never changes that default.
Any failure after insertion submission is uncertain, retaining the payload/deck
and validated returned ID when available. No retry/repair/delete occurs; a note
may remain in the default deck. Read-only exact-surface reconciliation establishes
existence, not placement success. The typed live dispatcher now invokes this
adapter only for a committed native permit. Visible Add UI remains unfinished; fake
coverage does not establish installed-provider compatibility or an atomic check-and-add.

App-owned state, durable pending/uncertain journals and typed live dispatch are now
implemented; configured-deck/permission settings and authenticated live actions
remain. Settings now projects recovery obligations/errors with explicit read-only
checks and Retry, remaining hidden on a clean startup. A process restart must not erase insert
uncertainty or replay an old mutating request. No supported provider completion
fence for safe retry after empty reconciliation has been established. Verify model
layout before reuse; never overwrite an existing user model merely by name.
Cover missing provider, unsupported spec, permission denial, unconfigured/locked
collection, missing deck/model, duplicate ambiguity, failure after insertion,
open-link resolution and unchanged Yomoku status. Test a disposable real AnkiDroid
collection before advertising supported app versions or complete export behavior.


Typed recovery dispatch now uses these existing queries through
`AnkiRecoveryReader`, with the frozen surface/layout supplied by Rust. Unknown
model IDs require exactly one full match; known IDs cannot be substituted.
Failures during later validation discard earlier matches. Crux retains callback
provenance and both journals natively until the SQLite evidence commit. Rechecked
the pinned contract and Android ContentResolver documentation for this wiring;
no new provider API, dependency/minimum-version or real-provider claim is adopted.

The native app now owns frozen-export preflight handles and validates complete
typed deck/model/duplicate observations before selecting an existing-note result,
model reuse or model creation. Typed preflight now dispatches through
`AnkiPreflightReader` using only the already reviewed queries. Every exact-name
candidate receives full layout validation; any failed read discards all earlier
evidence. Durable live begin/outcome gates and typed invocation are now implemented;
visible export remains unfinished.
No additional API, dependency or compatibility claim is adopted.

The live dispatcher uses the same reviewed insert/update/query APIs. Typed
`DispatchAnkiLive` consumes a Ready permit once and keeps its owner in Crux's
response closure. CoreSession invokes it off-main with session/close gates and
returns classified outcomes through the native durable outcome transaction.
Model creation refreshes the frozen deck/exact duplicates immediately before
submission; the adapter still refreshes host/permission and name conflicts.
Unexpected exceptions or invalid response IDs remain uncertain. Neither startup
restoration nor a standalone dispatch event can prepare/begin a mutation. No new
provider API, minimum version, permission flow or real-provider claim is adopted.

The shell can now request current-selection preparation and correlate begin/clear
with a volatile preparation generation (begin also matches the presented phase).
It supplies no note fields, native handles or journal callbacks. This reuses the
provider operations above without changing their API assumptions. CoreSession
now dispatches a rendered Ready generation once, with close/phase/owner rechecks;
Settings consumes the frozen export projection through explicit typed controls.
Deck/permission/dictionary Add/browser UI and disposable real-provider validation
remain unfinished. No new provider operation or compatibility claim is adopted.
