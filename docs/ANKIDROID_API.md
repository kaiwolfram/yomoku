# AnkiDroid integration evidence

Reviewed 2026-10-31 for PRODUCT_SPEC.md §6. Source inspection only; no provider
installation, permission grant, collection access or export has run yet.
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

This is a pure content boundary: callers still need to resolve the canonical
sentence, applicable dictionary/custom data and a current original/effective
selection. No Event, FFI type, settings, schema, dependency or UI action is added.
Preparing a note has no mutable vocabulary/history input or effect.

Next implement typed retained export commands/results and an Android provider
adapter with a fake-provider contract suite before live wiring. Verify model
layout before reuse; never overwrite an existing user model merely by name.
Cover missing provider, unsupported spec, permission denial, unconfigured/locked
collection, missing deck/model, duplicate ambiguity, failure after insertion,
open-link resolution and unchanged Yomoku status. Test a disposable real AnkiDroid
collection before advertising supported app versions or complete export behavior.
