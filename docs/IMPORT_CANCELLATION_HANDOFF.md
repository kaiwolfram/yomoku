# Issue 3: import stream cancellation handoff

## Scope and evidence

Prepared from the investigation on 2026-11-15, at revision `5df5a6d`.
This handoff records the third numbered issue. No application fix has been made.
Read [AGENTS.md](../AGENTS.md), [PRODUCT_SPEC.md](PRODUCT_SPEC.md) section 3.3,
and [DEVELOPMENT.md](DEVELOPMENT.md). Follow the repository's preservation,
focused-verification and automatic-commit instructions for implementation.

`FileImports.cancel` increments `readGeneration`, interrupts/retires the reader
executor and permits a fresh import. However, acquisition is one `readBytes()`
call, with the next generation check only after EOF. If a provider ignores
interruption, an obsolete attempt keeps requesting more blocks and growing its
buffer after the blocked read returns. The old stream can remain active alongside
a new import even though its eventual result will be ignored.

The investigation compiled unchanged production `FileImports.kt` against small
host stubs. A fake resolver returned an `InputStream` whose bulk reads used
`Semaphore.acquireUninterruptibly()`. After one 1 KiB read, the probe cancelled
the import, then released more reads. Its output was:

```text
cancel returned: active=false, read_calls=1
read_calls_after_cancel=7 (expected no more bulk reads once the blocked read returns)
```

The seven calls include the final EOF read. The important observation is that
the old attempt kept issuing reads after cancellation. A call already blocked
inside a provider may be uninterruptible; the fix should stop at the next
available checkpoint, not promise immediate interruption of arbitrary providers.
The host probe did not verify Android provider or service behavior.

Temporary artifacts were `/tmp/yomoku-audit/kotlin/Probe.kt`, neighboring stubs,
and `/tmp/yomoku-audit/import-probe.jar`. They are optional local aids only;
the reproduction above must remain sufficient if `/tmp` is cleaned.

## Relevant code and existing coverage

- [FileImports.kt](../android/app/src/main/java/app/yomoku/FileImports.kt):
  `start`, generation checks, `cancel`, `settleCancellation`, `complete`, and
  executor/stream ownership.
- [CoreSession.kt](../android/app/src/main/java/app/yomoku/CoreSession.kt):
  `cancelFileImport` and the native-handle close gate.
- [imports.rs](../crates/yomoku-runtime/src/imports.rs):
  `FileImportControl` and `AnalyzerLifecycle::cancel_file_import`.
- [import_cancellation.rs](../crates/yomoku-storage/src/import_cancellation.rs):
  cancellation serialized with the per-file COMMIT gate.
- [MixedFilePickerTest.kt](../android/app/src/androidTest/java/app/yomoku/MixedFilePickerTest.kt),
  [ImportNotificationsTest.kt](../android/app/src/androidTest/java/app/yomoku/ImportNotificationsTest.kt),
  and [CoreSessionTest.kt](../android/app/src/androidTest/java/app/yomoku/CoreSessionTest.kt):
  related import, service and lifecycle coverage. Locate and reuse the existing
  controllable provider fixtures before adding a new test mechanism.

## Proposed follow-up

1. Replace the opaque read-to-EOF operation with bounded chunk acquisition and
   generation/cancellation checks before and after each provider read. Stop
   requesting blocks once the attempt is obsolete. Coordinate the same helper
   with the memory fix rather than duplicating read loops.
2. Ensure every exit closes attempt-owned streams and discards private buffers
   or staging resources. Cancellation must not block the main thread on a provider
   read or close. Consider platform cancellation support where useful, without
   depending on every provider honoring it.
3. Preserve the existing separation between source acquisition and accepted
   native work. Before native submission, an obsolete read must never dispatch
   its import event. After submission, use the native cancellation/COMMIT gate:
   a successful COMMIT wins and must still produce the committed outcome.
4. Keep stale filename/error/result callbacks isolated from a fresh attempt.
   Retiring a blocked executor must continue to allow a new import to proceed;
   solving this by waiting for the obsolete provider would restore an old stall.

## Acceptance and verification

- Test cancellation while querying metadata, while opening/reading a stream,
  between chunks, before EOF, and before native submission. Use latches or
  semaphores rather than sleeps to establish ordering.
- A provider ignoring interrupts should be allowed to finish its outstanding
  call, after which no further read is requested and resources are released.
- Start a second import while the first provider remains blocked. After releasing
  the first, assert that only the second attempt can publish current progress or
  dispatch a new import; verify no stale terminal result overwrites it.
- Retain native cancellation-before-COMMIT and COMMIT-wins tests, exact encoding
  retry behavior, activity recreation, session close, and notification cleanup.
- Run relevant Kotlin formatting/instrumentation and native cancellation tests.
  Run Kotlin/JNI verification if the bridge changes. A host-stub result alone
  must not be reported as Android lifecycle/provider verification.

## Coordination

[Issue 1](IMPORT_MEMORY_HANDOFF.md) owns memory bounds and rejection behavior;
this handoff owns cancellation checkpoints, resource cleanup and stale-attempt
isolation. Either can introduce the shared acquisition helper.
[KISS/DRY cleanup](KISS_DRY_HANDOFF.md) owns broader sealed workflow states;
keep the bug fix independent of that larger refactor where practical.
