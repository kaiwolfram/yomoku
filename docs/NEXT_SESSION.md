# Resume here
Updated: 2026-10-31. App-owned Anki recovery-observation commits verified.
Commit guidance: explanatory bodies; merges include provenance. Keep verification
results in session/progress docs and reports, out of commit message bodies.

## Current slice
- Initial RefreshLibrary schedules native restore before List on the existing
  FIFO via AnkiStartupLayer. Constructors stay lazy; reader/import busy/errors/
  checkpoints stay independent. Repeated refresh never retries recovery failure.
- Model owns Unrestored/Loading/Failed/Restored plus native read/commit attempts.
  Only successful empty BOTH journals satisfies startup; restored obligations
  remain closed during commit. No recovery shell projection or provider dispatch.
- Native CommitAnkiRecovery carries the exact restored pair captured at provider
  read dispatch + typed note/model observation. App validates target/current pair;
  worker rechecks BOTH under one immediate transaction, reconciles one target,
  validates resulting pair, commits, then returns. Wrong results/false empty pairs,
  stale/duplicate callbacks, changed payloads/known IDs/new obligations cannot open
  the gate. Snapshot mismatch never reinterprets evidence for a newer record.
- Failed/lost commit response keeps old app obligation + separate persistence
  error. Reject further commits until explicit native RetryAnkiRecovery restores
  BOTH journals read-only; do not replay observation/provider work automatically.
  A committed-but-lost result is recovered by restore; rollback keeps obligation.
- Core owns recovery IDs/data/model observations and expected transition checks;
  storage reexports old paths. No schema/dependency/public codec change. Provider
  observation validation and callback provenance still need typed read dispatch.
- AnkiExportSession remains pure/in-memory, unowned by app/Event/FFI/UI. Preserve
  original/effective current-owner stale-shell selection guards and freeze content.
- Note/model payloads freeze v1 fields/surface/deck/model/full layout. Both begin
  methods gate BOTH journals in an immediate transaction. Commit BEFORE one live
  invocation; only new preparation creates private nonserializable callback tokens.
  Recovery IDs/data never reconstruct them. Startup-empty alone permits no write.
- Note existing IDs establish existence, not placement. Model evidence requires
  exact frozen full layout + original ID when known. Failed/empty/partial/wrong
  reads retain uncertainty. Terminal payload/results survive local data deletion.
  No external mutation replay/repair/rename/delete/cleanup or automatic retries.
- Android worker-only provider reader/model creator/note inserter stay isolated.
  Fresh host/spec>=2/permission; full layout/nonfiltered frozen deck; last-read
  cross-model/deck duplicates; explicit ordinal-zero placement/readback. Never
  alter model default deck/existing notes. Post-submission failures uncertain;
  creation can leave partial models. No real-provider claim; query cost unmeasured.
- API evidence: ANKIDROID_API.md, pinned 2.25.3; no Anki dependency/minimum adopted.
  Nostr config empty; ask Q188 before forwarding. Physical arm64 user-deferred.

## Next tasks (ordered)
1. Own live export/current selections in app; wire typed provider read/preflight
   callbacks (capture exact recovery pair at read dispatch) and durable note/model
   begin/outcome gates through FFI before one live invocation. Reject stale shell
   selections, freeze fields/deck/layout and never replay mutations on recovery.
   Test failed/lost begin/outcome commits, process death and stale callbacks.
2. Add compact recovery/error/explicit Retry projection, configured deck, Add to
   Anki/existing-note actions after dispatch gates. Handle permission, absent/
   filtered deck and incompatible/uncertain model. Validate disposable real
   collection/browser/return/query cost before compatibility claims.
3. Resume Android native-phase/100k Select All Compose and observer-free optimized
   profiling plus accessibility/process-death matrix. Preserve full IDs/equality/
   replay guards. Heavy-history/diverse-book/full-load/exact peaks remain.

## Verification / blockers
- PASS: 18 new host cases; 63 focused Anki cases; 909 workspace tests/17 opt-in
  fixtures ignored; build, strict Clippy, Rustfmt/diff. Kotlin generation identical.
  Commands: DEVELOPMENT.md final section. Logs: /tmp/yomoku-anki-reconcile-*.log.
- Fixed two initial test-only unused-command warnings; final checks clean. No
  open blocker. No Android APK/lint/emulator, host Kotlin/JNI JVM, real provider/
  collection/browser, independent Full-loader, physical arm64, broader lifecycle
  or performance check this slice. Native events/results remain unencodable.
