# Resume here
Updated: 2026-10-31. Durable isolated Anki note-submission journal verified.
Commit guidance: explanatory bodies; merges include provenance. Keep verification
results in session/progress docs and reports, out of commit message bodies.

## Current slice
- `AnkiExportSession` still pure/in-memory, unowned by app/Event/FFI/UI. Current
  original/effective selections prepare exact context/definitions without changing
  vocab/history. Keep current-owner stale-shell guards when wiring live actions.
- New `AnkiSubmission` freezes raw surface/four fields, positive external model/deck
  IDs and complete actual model layout strings. Version-1 serde validates identity,
  provider delimiter/NUL/layout/ID constraints; unknown/corrupt data fails closed.
- Schema 0021 `anki_submissions`: immediate transactions, one unresolved slot,
  nonreused committed IDs, frozen payloads and optional known note ID. Commit BEFORE
  one live provider invocation. Only preparation returns a nonserializable live
  callback token; recovery returns separate data/ID for read-only reconciliation.
  Never invoke/replay from recovery or treat restore/commit failure as empty.
- Pending/uncertain recovery is conservative, including interrupted placement.
  Empty/failed reads keep obligation/known ID. Existing IDs establish existence,
  not successful deck placement. Duplicate/old callbacks cannot release newer work.
  Terminal rows retain content/results; no delete/pruning API. No reader/vocab FK
  can erase an obligation. No app/runtime/FFI/provider call site is enabled yet.
- Android isolated worker-only reader/model creator/note inserter remain unconnected.
  Refresh host/spec >=2/permission; validate full layout/nonfiltered deck, exact
  cross-model/deck duplicates immediately before insert. Explicit ordinal-zero
  card placement/readback; never change model default deck or existing notes.
  Every post-submission failure uncertain; no retry/repair/delete. Model creation
  may leave partial models and still needs separate durable ownership/recovery.
- No real-provider compatibility claim. Evidence: ANKIDROID_API.md, pinned 2.25.3;
  no Anki API dependency/minimum supported version adopted. Observations non-atomic;
  duplicate query cost unmeasured. Nostr config empty; ask Q188 before forwarding.

## Next tasks (ordered)
1. Own retained export/recovery in app with typed Event/FFI/provider dispatch.
   Gate on startup restore and pending/outcome commits; use the journal before one
   live note invocation and recover read-only without replay. Add durable model-
   creation handling before enabling creation. Preserve exact payload/deck, reject
   stale shell selections; test restore/commit failures, process death and callbacks.
2. Wire configured-deck settings/Add to Anki/existing-note action after those gates.
   Handle permission, missing/filtered deck, conflicting/incompatible model and
   uncertain creation. Test a disposable real collection before compatibility
   claims; verify actual browser/return behavior and duplicate query cost.
3. Resume Android native-phase/100k Select All Compose and observer-free optimized
   profiling plus accessibility/process-death matrix. Preserve full IDs/equality/
   replay guards. Heavy-history/diverse-book/full-load/exact peaks remain. Physical
   arm64 is user-deferred; emulator/host results do not establish hardware viability.

## Verification / blockers
- PASS: 14 new host cases; 865 workspace tests/17 opt-in fixtures ignored; workspace
  build, strict Clippy, Rustfmt and diff. Commands: DEVELOPMENT.md final journal
  section. Logs: /tmp/yomoku-anki-journal-{focused,workspace,build,clippy}.log.
- Initial helper-name compile error and four historical migration-fixture failures
  corrected; final checks pass. No escalation or open blocker. Core/storage only;
  no dependency, Event/ViewModel/FFI protocol or Kotlin change.
- No Android APK/lint/emulator, independent Full-loader, real Anki collection/
  browser, physical device, broader lifecycle or performance check this slice.
