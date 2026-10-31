# Resume here
Updated: 2026-10-31. App-owned Anki startup recovery verified.
Commit guidance: explanatory bodies; merges include provenance. Keep verification
results in session/progress docs and reports, out of commit message bodies.

## Current slice
- All runtime routers wrap `AnkiStartupLayer`: initial RefreshLibrary schedules
  native recovery before List on the existing FIFO. Constructors stay lazy;
  invalid paste still opens no DB. Core-only fixtures must request restore.
- Model owns Unrestored/Loading/Failed/Restored recovery with native attempt IDs.
  Only a successful empty pair satisfies startup; neither obligation nor errors
  enable preparation. Repeated refresh never retries failure. Native-only Retry
  exists; visible error/Retry projection remains unwired. Reader busy/errors/
  checkpoints remain independent. Recovery itself emits no shell effects.
- `anki_recovery_snapshot` reads BOTH journals in one deferred read transaction;
  errors never return a partial/empty pair. IDs/recovery data now live in core,
  reexported at old storage paths. No schema or generated codec change.
- `AnkiExportSession` is still pure/in-memory and unowned by app/Event/FFI/UI.
  Preserve original/effective current-owner stale-shell selection guards.
- Note/model payloads freeze actual version-1 fields/surface/deck/model/layout.
  Both begin methods gate BOTH journals under one immediate transaction. Commit
  BEFORE one live invocation. Only new preparation returns nonserializable live
  callback tokens; recovery IDs/data cannot reconstruct them. A startup-empty
  gate alone never authorizes a write; DB preparation must recheck both journals.
- Pending/uncertain recover identically with optional known returned IDs. Failed
  outcome commits retain obligations. No mutation replay/repair/rename/delete/
  cleanup. Terminal payload/results survive local reader/vocabulary deletion.
  Note existing IDs prove existence, not deck placement. Model readback requires
  frozen full layout and original ID when known; empty/partial/mismatched reads
  retain uncertainty. Stale callbacks cannot release newer work.
- Android provider reader/model creator/note inserter remain isolated/unconnected.
  Fresh host/spec>=2/permission, full layout/nonfiltered frozen deck and exact
  cross-model/deck duplicates gate inserts. Explicit ordinal-zero card placement/
  readback; no model-default/existing-note changes. Post-submission failure is
  uncertain; model creation may leave partial models. No real-provider claim.
- API evidence: ANKIDROID_API.md, pinned 2.25.3; no Anki dependency/minimum adopted.
  Observations non-atomic; query cost unmeasured. Nostr config empty; Q188 before
  forwarding. Physical arm64 remains user-deferred.

## Next tasks (ordered)
1. Own live export in app and wire typed begin/outcome/reconciliation commits and
   provider dispatch through FFI. Require startup gate + durable preparation
   before one live note/model invocation. Restore only read-only reconciliation;
   preserve frozen fields/deck/layout and current-owner selection checks. Test
   failed commits, lost responses/process death and stale/duplicate callbacks.
2. Add compact recovery/error/explicit Retry projection, configured-deck settings,
   Add to Anki and existing-note action after dispatch gates. Handle permission,
   absent/filtered deck and incompatible/uncertain model. Test a disposable real
   collection/browser/return and duplicate cost before compatibility claims.
3. Resume Android native-phase/100k Select All Compose and observer-free optimized
   profiling plus accessibility/process-death matrix. Preserve full IDs/equality/
   replay guards. Heavy-history/diverse-book/full-load/exact peaks remain.

## Verification / blockers
- PASS: 15 new host cases; 44 focused Anki cases; 891 workspace tests/17 opt-in
  fixtures ignored; workspace build, strict Clippy, Rustfmt and diff. Generated
  Kotlin output matches existing output byte-for-byte. Commands: DEVELOPMENT.md
  final startup-recovery section. Logs: /tmp/yomoku-anki-startup-*.log.
- Fixed an asynchronous test's missing wait and an accidental duplicate enum
  variant during relocation; final checks pass. No dependency/schema/public
  codec/Kotlin/UI change or open blocker. No Android APK/lint/emulator, JNI JVM,
  real provider/collection/browser, independent Full-loader, physical arm64,
  broader lifecycle or performance check this slice.
