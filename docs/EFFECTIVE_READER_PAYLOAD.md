# Effective reader payload review

Measured 2026-10-27 with the pinned offline host toolchain:

```sh
cargo run -p yomoku-storage --example measure_effective_reader_payload --locked --offline
```

The reproducible synthetic fixture repeats `日本語。`, stores one Japanese B token
and one punctuation token per repeat, and resolves without overrides or alternates.
It projects the first page at the default 500-character preset. The existing
pagination rules produce the page sizes below; the fixture does not force a split.
The key includes complete resolved chapter boundaries and metadata/provenance.
Vocabulary is absent. All sizes below are bytes of compact `serde_json` output,
the representation already used for durable effective reservations.

| Chapter characters | Segments | Source UTF-8 | Page characters | Frozen key JSON | Page tokens JSON |
| ---: | ---: | ---: | ---: | ---: | ---: |
| 1,000 | 500 | 3,000 | 1,000 | 139,145 | 85,519 |
| 10,000 | 5,000 | 30,000 | 500 | 1,409,312 | 42,519 |
| 100,000 | 50,000 | 300,000 | 500 | 14,290,979 | 42,519 |

The full key grows with chapter length even when the visible page stays the same.
At 100,000 characters it is about 47.6 times the canonical UTF-8 source size.
Copying this representation into every rendered state, selection and callback
would multiply that cost. The narrow typed effective bridge prerequisite below
keeps full resolution out of shell values.
Full frozen provenance must still authenticate selections and durable completions.

These are synthetic JSON size measurements, not generated Kotlin/JNI wire sizes,
real-book memory measurements or latency benchmarks. The initial size review did not
introduce a bridge representation or change storage/completion semantics. Real EPUB
cost, peak allocation and device timings remain
unmeasured. Re-run the example when effective domain fields change.

## Compact identity and retained-page prerequisite

`effective_session::EffectiveReaderSession` is a pure core state component, ready
for the coordinated live-reader switch. One instance belongs to one core session
for its whole lifetime. It retains at most one full effective key and a compact
visible page, with no global registry, runtime cache, I/O or persistent handles.
`accept` moves the full resolution from the validated storage snapshot into the
key; it does not clone the chapter. Rendering exposes only `EffectiveReaderPage`.
The retained key now uses immutable `Arc` ownership. `freeze_page`/`freeze_selection`
issue private-field Rust-only command values after compact identity/visibility checks.
Accepted commands and retry clones share that allocation across invalidation instead
of cloning the chapter. The native `effective_storage` protocol routes token/word/
custom/preparation/completion commands through the existing SQLite FIFO/dictionary.
Both nested protocol variants have no Serde shell codec and remain skipped by Facet
generation. The worker still revalidates complete provenance through storage; errors
use the existing typed channel. No unfinished reservation is replayed automatically.

`EffectivePageIdentity` carries a monotonically increasing `u64` revision and the
existing document/page/original-B identity. `EffectiveReaderSelection` adds a
whole-chapter effective segment index. These are scoped to the owning native/core
session; Android must discard them when replacing/closing that session, as it does
existing callbacks. They are not durable IDs or authorization credentials.

- `begin_load` invalidates the current page and issues a fresh Rust-only request
  before dispatch. Retries and same-page refreshes also get a new revision.
- `accept` requires the exact outstanding request and matching document/page/B
  identity. A stale, duplicate or mismatched result cannot replace a newer page.
  Missing B accepts only an empty plain projection and has no selectable key.
- `fail` consumes only its outstanding request; retry remains explicit. Movement,
  reflow, exit, refresh and override edits call `invalidate` before asynchronous
  work. Invalidation drops the key and pending request, retaining the counter.
  Exhaustion fails closed; it never wraps or reuses an identity.
- `resolve` checks the entire compact identity. `resolve_selection` additionally
  requires an eligible visible segment. Punctuation/off-page indices fail; clipped
  crossings retain their full occurrence identity. Storage must still revalidate
  the full frozen provenance in its transaction before word/custom/edit commands.
- An accepted pending completion/custom command separately owns its frozen full
  key across checkpoint drains and exact retries. Invalidation retires UI callbacks
  without discarding an accepted command. Durable effective completion reservations
  keep their existing full provenance and write/compare semantics. Unfinished
  reservations never auto-replay after restart.

`EffectivePageToken` contains visible anchor/range/surface/eligibility, exact reading/
base and optional live vocabulary; metadata-source and rule provenance stay in Rust.
Unresolved manual parts keep eligibility without borrowing readings/base forms.
Live status changes do not change full provenance; an explicit refresh still retires
old UI callbacks. Source-preferred furigana must come from the existing pure effective
projection during the coordinated switch, not from guessed token metadata.

The example now also measures compact JSON at revision 1, absent vocabulary:

| Chapter characters | Compact page JSON | Identity JSON | Selection JSON |
| ---: | ---: | ---: | ---: |
| 1,000 | 66,233 | 136 | 163 |
| 10,000 | 32,983 | 136 | 163 |
| 100,000 | 32,983 | 136 | 163 |

The generated Bincode shell codec uses **60 bytes** for an available-B identity
and **68 bytes** for a selection (56/64 with Missing B). Rust→generated Kotlin→Rust
fixtures verify the available-B sizes, maximum revision/index, exact large IDs,
supplementary/combining/NUL text, resolved/unresolved metadata and deleted vocabulary.
These measure serialized values, not JNI execution time, allocation or device memory.
Full resolution and storage revalidation costs remain; real-book/device measurements
are still pending. Compact schemas and native FIFO operations are tested independently;
no live event/view or completion/token/word/custom/furigana path has switched from B.

The retained session now also owns word/custom lifecycle and exposes standalone
`EffectiveReaderWordState`/`EffectiveWordDetails` codecs. Details carry one compact
visible token, compact selection, count, candidates and optional custom entry;
no full key is included. A native `EffectiveWordRequest` retains the shared frozen
key/action and a fresh checked callback attempt per dispatch/retry. Native word
results authenticate complete key/selection/visible fields before compaction;
same-surface tokens receive live vocabulary observations, including deletion.
Dismissal/invalidation clears presentation while accepted callbacks/retries retain
their keys. Failed custom commands preserve committed fields and require explicit
retry or Cancel; in-flight writes cannot be abandoned by this component. The new
Kotlin round trip verifies maximum unsigned counts, unresolved manual metadata,
literal supplementary/combining/NUL custom fields and retry/error state. It is
still a prerequisite schema/component, not a live app or JNI performance claim.

The retained session also owns completion acceptance, preparation and commit retries.
Acceptance freezes the displayed full key/intent before final-time drain and retires
shell callbacks. Native `EffectiveCompletionRequest` attempts are fresh on every
dispatch; accepted keys and durable reservations share their existing allocations.
The app must drain both checkpoint slots before dispatch and explicitly retry failures.
No completion Cancel or restart replay is introduced: a failed follow-up read can
hide a successful commit, so the exact reservation must return AlreadyRecorded.
Seven pure cases and a real FIFO failure/reversal/reopen/lost-response case verify
this lifecycle. No new shell schema or payload measurement; the live switch remains.


Manual status choices also have retained native ownership. `EffectiveStatusRequest`
shares the accepted selection key and freezes the vocabulary ID, literal surface,
choice and timestamp; each dispatch/retry gets a fresh checked native attempt.
`SetStatus`/`StatusChanged` reuse the existing native-only FIFO envelope. Storage
validates full resolution, visibility and the live exact-surface ID in the same
immediate transaction as manual status/history. Missing entries stay absent;
replacement IDs and stale/reversed partitions fail closed. Successful results
update repeated visible spellings and selected-word details only for the retained
revision. Pending choices gate word/custom/completion until success or explicit
failed-request Cancel/new choice. No shell schema or payload-size change. Include
status with token/word/custom/completion/furigana in the coordinated live switch.

Guarded boundary prompts now have retained native callback/request ownership too.
`EffectiveOverrideRequest` dispatches context reads, stored A/B/C choices, literal
manual partitions and winning-rule reversals through the same FIFO. A request
shares its full key/context with exact retries and uses a fresh checked callback
attempt. Writes invalidate visible callbacks immediately and mutually gate word/
custom/status/completion obligations. Dismissed reads cannot restore a prompt.
Stale contexts are discarded and require explicit page/context reload; a lost
successful write cannot re-register deleted vocabulary on retry. This introduces
no shell schema or new payload measurement. Compact prompt presentation and live
Android controls remain pending with the coordinated reader switch.

Explicit on-demand A/C generation now shares the retained override lifecycle and
existing import analyzer cache. Requests retain mode/context across fresh retry
attempts. Stored same-profile snapshots bypass Full, including unaligned choices.
Missing modes analyze the whole canonical chapter outside the transaction; storage
checks exact profile/mode/anchors and revalidates the key/rule revision at commit.
New metadata can retire a full key even when boundaries stay identical. Acceptance
therefore invalidates callbacks, success requires explicit reload, and a lost
success never silently rebases a stale prompt. No shell schema/size change or live
reader adoption. Real-book/JNI/device cost remains unmeasured.

Compact override prompt/action schemas now accompany the native lifecycle.
`EffectiveReaderOverrideState` contains only selected-span text/stored partitions,
winning-rule display/reversal facts and pending/error/reload state. Full contexts,
chapter resolutions, rule revisions and frozen retry payloads remain native.
Each open gets a fresh prompt revision even on an unchanged page/span; subsequent
actions authenticate it. Accepted write retry/Cancel retains that identity across
invalidation, while callback attempts remain fresh.

The available-B prompt identity encodes to **68 bytes**. Five presentation fixtures
and all eight action variants round-trip through generated Rust/Kotlin codecs,
including maximum revisions/IDs/timestamps, both scopes, A/B/C, literal
supplementary/combining/NUL parts and distinct storage/invalid/stale failures.
These are prototype standalone schemas, not live Event/ViewModel integration or
JNI/device performance measurements. The live reader still uses original B.

## Retained rendering adapters

`EffectiveReaderSession::furigana(page, analysis)` borrows the current compact
page's readings and shares the original renderer's strict text/range validation
and source-precedence policy. It never clones the retained full key or resolves
the chapter again. Missing/stale/invalid fallback data retains validated source
ruby; unresolved manual parts have no guessed B/custom reading.

Android's `effectiveDisplayTokens` accepts the expected compact identity and page,
rejects stale revisions/foreign bases and malformed whole projections, and returns
UTF-16 ranges paired with full tokens and `EffectiveReaderSelection`. Crossings
retain the same chapter segment/full surface, contextual metadata stays per
occurrence, and unresolved manual parts stay selectable without metadata. Its
geometry is shared with the existing B renderer and compiled by the host Kotlin
test runner. No wire-schema/size change or live reader switch is included.

## Native callback dispatch

The separate [host performance review](EFFECTIVE_READER_PERFORMANCE.md) now measures
a real novel's chapter resolution, Library scans and retained-page acceptance,
including whole-chapter A/C setup. It preserves all payload/provenance fields while
removing the temporary metadata tree. JNI/device and real EPUB costs remain pending.

`EffectiveReaderRequest` wraps the accepted token/word/custom/status/completion/
override attempts. Its `command` method sends the existing storage effect and
captures the originating request in a native Crux callback. `finish_request`
routes both `StorageResult::Failed` and effective results to that exact lifecycle.
Unexpected envelopes fail closed; a stale/duplicate callback returns None and
cannot release newer app busy state. Token result validation also consumes only
the captured read on failure, requiring a fresh explicit load revision.

Requests/progress have no shell codec and keep the existing shared full keys and
retry reservations. Completion progress never dispatches a commit or retries
automatically. The app must still enforce shared FIFO/time/lifecycle gates before
acceptance and drain both final checkpoint slots before dispatching preparation.
The real FIFO tests use this adapter for token, word/custom and completion flows;
status/deletion and override save/reload/reversal also run through it. Live app
adoption and device measurements remain pending.

## Android boundary editor component

`ReaderBoundaryEditor` now renders the existing compact prompt and emits its typed
Choose/Save/Generate/Reverse/Retry/Cancel/Dismiss actions in isolated Compose tests.
Manual partitions toggle Unicode scalar boundaries without changing the literal
surface. Draft cuts/scope restore only for the same retained native session and
complete prompt identity/text; pending write payloads stay in Rust. Explicit
reload handles retired contexts. No wire-schema/payload-size change or live
Event/ViewModel/reader adoption is included. The parent still must open validated
original-B spans and enforce shared FIFO/time gates during the coordinated switch.


## Staged application page/completion integration

The native `Model::with_effective_reader()` now exercises retained page loads,
source-preferred furigana and compact-identity completion through real app events,
shared FIFO/time gates and native callbacks. ViewModel adds optional
`effective_reader_tokens`; CompleteEffectiveReaderPage contains only identity,
position and timestamp. Rust/Kotlin fixtures exchange the event and a populated
application view with maximum revision/IDs and literal Unicode. Existing standalone
identity/selection sizes remain 60/68 bytes. Full callback results move into the
session without a chapter clone; accepted commands retain shared keys as before.

Production FFI still constructs Model::default, and Android/Library still use B.
Remaining Library membership and Android consumers must migrate before activating
this constructor. This is not a user-visible mode.
No durable schema/payload change, dictionary load or JNI/device performance claim.

The staged application now also routes compact word/status/custom events and
projects optional `effective_reader_word`/`effective_reader_status`. Word details
contain only the selected visible token, dictionary candidates/count/custom entry
and retry/error facts. Status projects pending/busy/error. Neither view sends a
chapter key. Generated Kotlin fixtures exchange all eight new event variants and
both populated view fields with maximum IDs/revisions, unresolved metadata and
literal supplementary/combining/NUL text. Retry/Cancel have no payload; retained
native commands supply exact values, with a fresh native callback attempt.
The 60/68-byte identity/selection encoding is unchanged. This changes same-build
application codecs only; no durable data schema or new device/performance claim.

The staged app also wraps all 15 compact override actions in
`Event::EffectiveReaderOverride` and exposes optional `effective_reader_override`.
Generated Kotlin fixtures exchange all action events and five populated app views
(ready prompt, busy, failed write, invalid selection and stale/reload). The prompt
retains only selected-span choices and winning-rule facts; full contexts/rule
revisions/frozen retries stay native. Shared FIFO/time gates include override
writes; success/stale results require explicit reload. Production remains B until
the Library and Android consumers switch together. No durable schema change.


The staged app now routes the existing Library event to native-only
`LoadEffectiveLibraryWordDistributions` on the same FIFO and reuses the existing
compact result, cache and error fields. Serde/Facet skip the new operation;
generated Kotlin and application wire schemas are unchanged. Production still
uses B pending Android activation. No new device/performance claim.
