# AnkiDroid provider contract

Current behavior follows [PRODUCT_SPEC.md §6](PRODUCT_SPEC.md#6-ankidroid-integration).
The provider research below pins AnkiDroid **2.25.3**; no minimum supported app
version is implied by that research. The opt-in installed-provider trial passed
against 2.25.3 on an API 36 x86_64 emulator, covering setup, Add, external note
changes, return refresh, browser launch and restart without replay. Other app
versions and physical arm64 runtime behavior remain unverified.

## Provider and exact-headword queries

Kotlin uses ContentResolver, never AnkiDroid's database files. Check provider
availability/enabled/exported state, API spec **>= 2** and
`com.ichi2.anki.permission.READ_WRITE_DATABASE` before operations. Package
visibility declares the known provider. Settings owns permission requests/deck
selection/model setup; the dictionary sheet contains entry actions only.

The reviewed [provider contract](https://github.com/ankidroid/Anki-Android/blob/v2.25.3/api/src/main/java/com/ichi2/anki/FlashCardsContract.kt)
and [implementation](https://github.com/ankidroid/Anki-Android/blob/v2.25.3/AnkiDroid/src/main/java/com/ichi2/anki/provider/CardContentProvider.kt)
support bound `notes_v2` field queries across models/decks. Match the exact escaped
Yomoku first field and a safe literal plain-headword candidate, then compare every
returned first field in Kotlin. No fuzzy/normalized/arbitrary-HTML interpretation.
NUL input fails closed. Null, malformed, partial and failed reads are errors, never
absence. Query on lookup, app return and immediately before insertion, so notes
created/deleted externally change the buttons. Existing-note navigation resolves
verified positive IDs to AnkiDroid's browser; it is not replayed after pause/restart.

## Note preparation and model compatibility

Pure Rust prepares four escaped fields: headword, reading, definitions and sentence.
The front is the dictionary headword; the source sentence highlights the retained
source occurrence, including inflections/components. Bundled dictionary candidates
supply licensed readings/English senses. There is no custom-entry export or audio.
Export leaves vocabulary status/history unchanged.

The core supplies the exact **Yomoku v2** layout. Validate name, ordered fields,
CSS, normal model type, first-field sorting, one ordinal-zero forward template
and empty browser overrides. A model's default deck is not layout authority;
insert into the frozen selected regular deck. Same-name incompatible/partial
models remain conflicts. Never overwrite, repair or delete them automatically.

The reviewed [AddContentApi](https://github.com/ankidroid/Anki-Android/blob/v2.25.3/api/src/main/java/com/ichi2/anki/api/AddContentApi.kt)
creates metadata and then updates the new template. The adapter validates the
returned new ID and fully reads back layout. Note insertion initially uses the
model default deck, then moves/verifies the one forward card in the selected deck.
Permission, deck, layout and duplicates are checked again before writes. Partial
model/note creation can remain after an error; no automatic cleanup/replay occurs.

## Ownership and interruption

Only `anki_settings.default_deck_id` persists. Prepared fields, query observations
and the active request live in memory. Typed provider effects consume a Ready
invocation once. Fresh attempt/allocation identity rejects repeated taps and stale
callbacks, including after selection/session changes. There are no SQLite
begin/finish steps, export journals, recovery UI or startup reconciliation.

After interruption, an explicit action performs fresh queries. A successful empty
query can allow a new explicit insertion; it does not prove an earlier external
operation never completed. There is no exactly-once guarantee across process death.
Failed/incomplete observations remain errors.

## Verification boundary

Host core/JNI and Android fake-provider tests cover external note changes, query
failures, exact-headword semantics, permission/deck/layout checks, repeated taps,
stale callbacks and restart without replay. Android uses
[ContentResolver.wrap](https://developer.android.com/reference/android/content/ContentResolver#wrap(android.content.ContentProvider))
for in-process provider fixtures. These do not prove real Binder/collection/browser
compatibility across versions/devices. See [the opt-in disposable-collection
trial](DEVELOPMENT.md#android-trials). Broader permission-dialog, partial-failure
and physical-device trials remain useful before release.
