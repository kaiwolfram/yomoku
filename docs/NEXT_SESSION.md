# Resume here
Updated: 2026-10-31. Durable isolated Anki model-creation recovery verified.
Commit guidance: explanatory bodies; merges include provenance. Keep verification
results in session/progress docs and reports, out of commit message bodies.

## Current slice
- `AnkiExportSession` remains pure/in-memory, unowned by app/Event/FFI/UI. Prepare
  exact current original/effective context/definitions without changing history.
  Preserve current-owner stale-shell guards when wiring live actions.
- `AnkiSubmission` freezes exact surface/four fields/deck/model/full layout.
  `AnkiModelCreation` freezes full layout independently; both validate version-1
  JSON and reject unknown/corrupt payloads instead of using current constants.
- Schemas 0021 note submissions and 0022 model creations are durable journals,
  not mutation queues. Both begin methods check BOTH journals in one immediate
  transaction, blocking overlapping note/model writes across connections.
  Commit BEFORE one live invocation. Only new preparation returns nonserializable
  live callback tokens; recovery returns distinct data/IDs for read-only reads.
- Startup must restore BOTH journals; errors are never empty. Pending/uncertain
  restore identically, with optional known returned IDs. Failed outcome commits
  keep the obligation. Never replay provider mutations from recovery.
- Note existing IDs establish existence, not deck placement. Model reconciliation
  requires exact frozen full layout and the original ID when known; a compatible
  different ID cannot hide a known partial model. Failed/empty/partial/mismatched
  reads retain uncertainty. No replay/repair/rename/delete/cleanup. Terminal rows
  retain payload/results; no reader/vocab FK can erase obligations. Stale callbacks
  cannot release newer work. No app/runtime/FFI/provider call site is enabled.
- Android worker-only provider reader/model creator/note inserter remain isolated.
  Refresh host/spec >=2/permission; validate full layout/nonfiltered deck and exact
  cross-model/deck duplicates immediately before insert. Explicit ordinal-zero
  card placement/readback; never change model default deck or existing notes.
  Every post-submission failure uncertain; model creation may leave partial models.
- No real-provider compatibility claim. Evidence: ANKIDROID_API.md, pinned 2.25.3;
  no Anki dependency/minimum version adopted. Observations non-atomic; query cost
  unmeasured. Nostr config empty; ask Q188 before forwarding.

## Next tasks (ordered)
1. Own retained export/recovery in app with typed Event/FFI/provider dispatch.
   Gate on startup restore of both journals and pending/outcome commits; use the
   journals before one live note/model invocation and recover read-only without
   replay. Preserve frozen payload/deck; reject stale shell selections. Test
   restore/commit failures, process death and duplicate/stale callbacks.
2. Wire configured-deck settings/Add to Anki/existing-note action after gates.
   Handle permission, missing/filtered deck, conflicting/incompatible model and
   uncertain creation. Test a disposable real collection before compatibility
   claims; verify actual browser/return behavior and duplicate query cost.
3. Resume Android native-phase/100k Select All Compose and observer-free optimized
   profiling plus accessibility/process-death matrix. Preserve full IDs/equality/
   replay guards. Heavy-history/diverse-book/full-load/exact peaks remain. Physical
   arm64 is user-deferred; emulator/host results do not establish hardware viability.

## Verification / blockers
- PASS: 11 new model cases; 25 focused Anki cases; 876 workspace tests/17 opt-in
  fixtures ignored; workspace build, strict Clippy, Rustfmt and diff. Exact commands
  in DEVELOPMENT.md final model-journal section. Logs:
  /tmp/yomoku-anki-model-journal-{focused,workspace,build,clippy}.log.
- Corrected one old multiline schema expectation and boxed a large readback enum
  variant after initial checks. No escalation/open blocker. No dependency,
  Event/ViewModel/FFI protocol or Kotlin source change.
- No Android APK/lint/emulator, independent Full-loader, real collection/browser,
  physical device, broader lifecycle or performance check this slice.
