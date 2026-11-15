# Issue 1: large-import memory handoff

## Scope and evidence

Prepared from the investigation on 2026-11-15, at revision `5df5a6d`.
This handoff records the first numbered issue. No application fix has been made.
Read [AGENTS.md](../AGENTS.md), [PRODUCT_SPEC.md](PRODUCT_SPEC.md) sections 2
and 3.3, and the applicable commands in [DEVELOPMENT.md](DEVELOPMENT.md).
Preserve existing work and commit each completed, verified implementation step
according to the repository instructions.

`FileImports.start` uses `openInputStream(uri)?.use { it.readBytes() }`, without
a byte budget. The source is allocated in Kotlin before Rust can apply EPUB's
parser limits. The subsequent typed event serialization creates additional
whole-file representations, so checking only the eventual native parser input
does not bound the acquisition/transport cost.

The investigation compiled unchanged production `FileImports.kt` against small
Android, Compose and core host stubs. A fake resolver generated a 64 MiB stream
incrementally; the stream did not itself allocate a 64 MiB fixture. With
`java -Xmx32m`, the recorded result was:

```text
stream_bytes=67108864 max_heap=33554432 worker_failure=OutOfMemoryError import_active=true result=null
```

The probe installed an uncaught-exception observer to inspect the failure. Its
result establishes an unbounded allocation and a missing terminal outcome in
the host harness; it is not an Android-device crash measurement. The generated
bytes were not a valid EPUB: the failure happened before native parsing.

Temporary artifacts were `/tmp/yomoku-audit/kotlin/OomProbe.kt`, the neighboring
stubs, and `/tmp/yomoku-audit/import-probe.jar`. They are optional local aids,
not committed fixtures or durable dependencies. Reconstruct the probe from the
description if they have disappeared.

## Relevant code and existing coverage

- [FileImports.kt](../android/app/src/main/java/app/yomoku/FileImports.kt):
  URI reading, `FileImportBytes`, retained input for encoding retries, and
  terminal import results.
- [CoreSession.kt](../android/app/src/main/java/app/yomoku/CoreSession.kt):
  `dispatch`/`send` serialize import events on the main thread.
- [imports.rs](../crates/yomoku-core/src/imports.rs) and
  [epub.rs](../crates/yomoku-import/src/epub.rs): typed inputs and existing
  `EpubLimits`, including the 256 MiB source-byte ceiling at review time.
- [FileImportBytesTest.kt](../android/app/src/androidTest/java/app/yomoku/FileImportBytesTest.kt):
  an existing 18 MiB exact-byte serialization/retry case. Preserve this coverage;
  it does not establish bounded stream acquisition.
- [MixedFilePickerTest.kt](../android/app/src/androidTest/java/app/yomoku/MixedFilePickerTest.kt)
  and [ImportNotificationsTest.kt](../android/app/src/androidTest/java/app/yomoku/ImportNotificationsTest.kt):
  existing import UI and notification behavior.

## Proposed follow-up

1. Reproduce the failure with a generated stream and inspect peak acquisition
   and serialization memory separately. Include providers with unknown or
   inaccurate reported lengths; metadata alone must not authorize allocation.
2. Establish a bounded acquisition strategy that checks actual bytes before
   growing buffers. Account for source retention during encoding retries and
   copies across the Kotlin/native boundary. Do not treat catching
   `OutOfMemoryError` as the primary solution.
3. Reuse existing resource limits where applicable. TXT/SRT have no equivalent
   source-byte admission limit in this path. A new user-visible rejection limit
   is a product decision, not an arbitrary number to introduce silently. If a
   file-backed or streaming FFI design is needed, assess and explain the contract
   and architecture implications before adopting a major change.
4. Return a concise terminal error for rejected inputs, release attempt-owned
   resources, and let another import start normally. Retain exact source bytes,
   SHA-256 duplicate identity, strict encoding choices and atomic native imports.

## Acceptance and verification

- Exercise known-length and unknown-length streams, incorrect metadata, the
  admission boundary, and an input exceeding the chosen resource strategy.
  Assert bounded reads/allocation and a terminal result before native import.
- Preserve successful EPUB/TXT/SRT imports, exact duplicates, encoding selection
  and retry, notification behavior, and cancellation/COMMIT ordering.
- Use deterministic resource/accounting assertions where possible; do not add a
  test that deliberately crashes the normal instrumentation process with OOM.
- Run affected Kotlin formatting and focused instrumentation. If transport or
  FFI changes, also regenerate codecs and run Kotlin/JNI checks and the required
  Android builds from DEVELOPMENT.md. Report device memory measurements separately
  from host probes.

## Coordination

[Issue 3](IMPORT_CANCELLATION_HANDOFF.md) changes the same read loop. Share one
bounded, cancellation-aware acquisition helper rather than adding two loops.
Either issue can establish that helper; the later change should extend it.
This handoff owns memory admission and terminal failure behavior.
[KISS/DRY cleanup](KISS_DRY_HANDOFF.md) owns broader workflow-state changes.
