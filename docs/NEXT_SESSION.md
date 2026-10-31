# Resume here
Updated: 2026-10-31. App-owned frozen Anki export preparation verified.
Commit guidance: explanatory bodies; merges include provenance. Keep verification
results in session/progress docs and reports, out of commit message bodies.

## Current slice
- Native PrepareAnkiExport authenticates the app's current visible loaded word,
  canonical document/page/analysis and original/effective owner. No fallback from
  legacy shell selection to effective reader. Reader busy/checkpoint/completion/
  departure and effective mutation gates apply; BOTH journals must be restored
  empty with no outstanding/failed recovery commit.
- Model retains one immutable AnkiExport: selection, exact fields/surface, deck,
  full v1 model layout. Duplicate preparation/dictionary/deck changes/dismissal/
  navigation cannot replace it. ClearPreparedAnkiExport drops only unsubmitted
  intent, never journals/commits. New process has no prepared intent.
- Preparation emits NO effects/provider work/live tokens; native events have no
  shell codec/ViewModel projection. AnkiExportSession stays isolated/unowned;
  shared constructors now freeze its layout too. Do not dispatch CheckAndAdd
  through it until typed preflight + durable begin/outcome gates exist.
- Initial RefreshLibrary schedules native restore before List on the existing
  FIFO via AnkiStartupLayer. Constructors stay lazy; reader/import busy/errors/
  checkpoints stay independent. Repeated refresh never retries recovery failure.
- CommitAnkiRecovery carries the exact pair captured at provider read dispatch
  plus typed observation. App validates target/current pair; worker rechecks BOTH
  under one immediate transaction, reconciles one target, validates pair, commits.
  Wrong results/stale callbacks/changed snapshots cannot open the gate. Failed/
  lost commit retains old app obligation and separate persistence error; explicit
  Retry restores BOTH journals read-only. Never replay observation/provider work.
- Core owns recovery data/IDs/full-layout matching; storage reexports old paths.
  No schema/dependency/public codec change. Snapshot/observation validation and
  callback provenance still need typed provider read dispatch.
- Both journal begin methods gate BOTH journals in an immediate transaction.
  Commit BEFORE one live invocation; only new preparation creates private live
  callback tokens. Recovery data/IDs and startup-empty cannot authorize a write.
- Note IDs establish existence, not placement. Model evidence requires exact
  frozen full layout + original ID when known. Failed/empty/partial/wrong reads
  retain uncertainty. Terminal payload/results survive local data deletion.
  No mutation replay/repair/rename/delete/cleanup or automatic retries.
- Isolated Android worker provider reader/model creator/note inserter: fresh
  host/spec>=2/permission, full layout/nonfiltered frozen deck, last-read cross-
  model/deck duplicates, ordinal-zero placement/readback. No app call site or
  real-provider claim; query cost unmeasured. API evidence: ANKIDROID_API.md,
  pinned 2.25.3; no Anki dependency/minimum adopted. Nostr config empty; ask Q188
  before forwarding. Physical arm64 remains user-deferred.

## Next tasks (ordered)
1. Connect app-owned prepared intent to typed read-only provider preflight and
   recovery callbacks (capture exact pair at read dispatch), then durable note/
   model begin/outcome gates through FFI before one live invocation. Retain exact
   payloads/selection guards; never replay mutations on restart. Test failed/lost
   begin/outcome commits, process death, stale callbacks and clearing live state.
2. Add compact recovery/error/explicit Retry projection, configured deck, Add to
   Anki/existing-note actions after dispatch gates. Handle permission, absent/
   filtered deck and incompatible/uncertain model. Validate disposable real
   collection/browser/return/query cost before compatibility claims.
3. Resume Android native-phase/100k Select All Compose and observer-free optimized
   profiling plus accessibility/process-death matrix. Preserve full IDs/equality/
   replay guards. Heavy-history/diverse-book/full-load/exact peaks remain.

## Verification / blockers
- PASS: 10 new cases; 29 focused core/FFI Anki cases; 919 workspace tests/17 opt-in
  fixtures ignored; build, strict Clippy, Rustfmt/diff. Kotlin generation identical.
  Commands: DEVELOPMENT.md final section. Logs: /tmp/yomoku-anki-preparation-*.log.
- Fixed initial incomplete analyzer fixture; final checks clean. No open blocker.
  No Android APK/lint/emulator, host Kotlin/JNI JVM, real provider/collection/
  browser, independent Full-loader, physical arm64, broader lifecycle or
  performance check this slice. Native preparation remains unencodable.
