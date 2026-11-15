# Issue 2: reader EPUB I/O handoff

## Scope and evidence

Prepared from the investigation on 2026-11-15, at revision `5df5a6d`.
This handoff records the second numbered issue. No application fix has been made.
Read [AGENTS.md](../AGENTS.md), [PRODUCT_SPEC.md](PRODUCT_SPEC.md) sections 2
and 3, and [ARCHITECTURE.md](ARCHITECTURE.md) before changing reader/storage
boundaries. Use [DEVELOPMENT.md](DEVELOPMENT.md) for verification commands and
follow the repository's automatic-commit policy for verified implementation steps.

`load_effective_reader_tokens_in` calls `load_reader_document_in` even for one
small page. The EPUB branch calls `load_epub_in`, which reads and hashes the
original archive, loads and hashes every image asset, and loads all chapters and
annotations. The reader projection later discards the binary payloads. Selected
word validation also reaches this path through `selected_effective_token_in`.

A temporary Rust program imported two synthetic EPUBs with identical tiny text,
one referenced valid PNG each, and a deterministic test Mode C analyzer. Only
image size differed substantially. It then averaged five calls to the production
`Storage::load_effective_reader_tokens` API per EPUB. Desktop debug results were:

| Fixture | Archive size | Average token load |
| --- | --- | --- |
| Small PNG | 1,784 bytes | 0.83 ms |
| Approximately 16 MiB PNG | 16,787,085 bytes | 490.30 ms |

These are local diagnostic measurements, not release or Android-device latency
claims. They demonstrate that text-token loading scales with unrelated binary
payload size. Temporary artifacts were `/tmp/yomoku-audit/src/main.rs` and
`/tmp/yomoku-audit/make_epubs.py`; recreate fixtures if those files are absent.
No book or image fixture was added to Git.

## Relevant code and existing coverage

- [reader_tokens.rs](../crates/yomoku-storage/src/reader_tokens.rs):
  `load_effective_reader_tokens_in`.
- [reader_documents.rs](../crates/yomoku-storage/src/reader_documents.rs):
  whole-document snapshot/open/position APIs and EPUB projection.
- [epub.rs](../crates/yomoku-storage/src/epub.rs): `load_epub_in` reads
  `imported_files.original_bytes` and `epub_assets.bytes` and verifies digests.
- [reader_words.rs](../crates/yomoku-storage/src/reader_words.rs) and
  [effective.rs](../crates/yomoku-runtime/src/storage/effective.rs):
  selected-word validation, lazy A/B analysis, and completion follow-up reads.
- [reader_documents tests](../crates/yomoku-storage/src/tests/reader_documents.rs)
  include corruption checks for original bytes, assets and annotations.
- [status cache tests](../crates/yomoku-storage/src/tests/reader_words/status_cache.rs)
  cover validation reuse and invalidation across local/external database changes.

## Proposed follow-up

1. Trace page, word, position and completion operations separately. Establish
   which queries allocate archive/image payloads and how often. Keep the first
   fix focused on removing those payloads from reader operations; broader state
   and segmentation simplification belongs to the separate KISS/DRY handoff.
2. Introduce or reuse a dedicated reader query/projection for canonical text,
   ownership, annotations and asset metadata. Page/word operations should retrieve
   the needed chapter without materializing the original EPUB or all images.
   Whole-document pagination may still need ordered chapter text; do not make
   that a reason to read all binary content.
3. Preserve integrity guarantees deliberately. Existing readers validate stored
   data and existing tests expect corruption rejection. Do not just delete digest
   checks or weaken tests to improve timing. Consider where validation can safely
   be reused, keyed to immutable content and appropriate database revisions.
   If detection timing or behavior would change, explain that tradeoff before
   changing the contract. Asset loading must still verify the requested bytes.
4. Keep SQLite and asset I/O in storage/runtime. Preserve compact FFI identities,
   lazy dictionary loading, immutable stored analyses, and exact retry commands.

## Acceptance and verification

- Add a meaningful regression that detects reads/materialization of archive and
  image BLOBs during repeated page/word operations. Existing test work counters
  may be extended, but should measure the costly boundary rather than mirror a
  new helper's implementation. Prefer this over a machine-specific timing limit.
- Compare small and large assets with identical text, including an image in a
  different chapter. Record release measurements as well as debug diagnostics
  when evaluating the actual gain.
- Retain text/annotation/ownership corruption checks and appropriate binary
  integrity checks. Cover local and external changes if validation is cached.
- Verify lookup and status changes, crossing-token identity, Next/Previous,
  completion retries and duplicate prevention, finished-book reopening, EPUB
  images/covers, and TXT/SRT/paste behavior.
- Run focused storage/runtime/core/FFI reader tests, then applicable rustfmt and
  Clippy. Use Kotlin/JNI and Android checks if the bridge changes. Trial an
  image-heavy EPUB on a disposable emulator/device; report what was actually run.

## Coordination

This handoff owns the demonstrated binary-I/O performance defect.
[KISS/DRY cleanup](KISS_DRY_HANDOFF.md) owns simplification of the effective
segmentation representation and state ownership. Keep that refactor separate
unless a small part is necessary to make this fix reviewable. Preserve unrelated
import behavior covered by [issue 1](IMPORT_MEMORY_HANDOFF.md) and
[issue 3](IMPORT_CANCELLATION_HANDOFF.md).
