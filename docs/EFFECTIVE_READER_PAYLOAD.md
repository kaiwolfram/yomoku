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
