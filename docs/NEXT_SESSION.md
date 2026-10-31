# Resume here
Updated: 2026-10-31. Native Anki recovery read callback ownership verified.
Commit guidance: explanatory bodies; merges include provenance. Keep verification
results in session/progress docs and reports, out of commit message bodies.

## Current slice
- Native BeginAnkiRecoveryRead(Note|Model) retains one immutable handle with the
  exact restored pair. Capture it BEFORE provider read dispatch; callback must
  return that handle, matching target and unchanged pair. Identity is per shared
  allocation, not journal IDs/contents; another owner/attempt cannot recreate it.
- Foreign/stale/duplicate callbacks cannot consume newer reads. Invalid matching
  callbacks record separate read error. Accepted evidence consumes handle and
  enters existing FIFO CommitAnkiRecovery with the captured pair. Direct native
  evidence commits cannot compete with an active read.
- Explicit Retry can invalidate a lost read and restore BOTH journals read-only.
  Late callbacks cannot consume a fresh read. Pending journal commits cannot be
  abandoned; failed/lost commit responses require restore + fresh observation.
- This is native lifecycle ownership ONLY: begin emits NO effects/provider work.
  Native accessor is for future coordinator, not shell polling. No shell codec,
  ViewModel projection, Android/FFI call site or provider mutation is enabled.
- Native PrepareAnkiExport authenticates current visible loaded original/effective
  word and canonical document/page/analysis; reader busy/checkpoint/completion/
  departure/mutation gates apply. BOTH journals must be restored empty with no
  pending/failed commit. Immutable selection/exact fields/surface/deck/full layout
  survive dictionary/deck/popup/navigation changes. Clear drops only unsubmitted
  intent. New process has neither prepared intent nor read handles.
- Initial Library refresh schedules native restore before List on existing FIFO.
  Reader/import busy/errors stay independent. Repeated refresh never retries.
- Recovery commit worker compares BOTH journals in one immediate transaction,
  reconciles one target, validates pair and commits before returning. Stale/wrong
  results cannot open gate; errors retain app obligation. Never replay evidence.
- Both journal begin methods gate BOTH journals under an immediate transaction.
  Commit BEFORE one live invocation; only newly committed preparation produces
  private live tokens. Recovery IDs/data/startup-empty cannot authorize writes.
- Note IDs establish existence, not deck placement. Model evidence requires exact
  frozen full layout + original ID when known. Failed/empty/partial/wrong reads
  retain uncertainty. Terminal data survive deletion. No mutation replay/repair/
  rename/delete/cleanup or automatic retries. Isolated AnkiExportSession stays
  unowned; do not use CheckAndAdd to bypass durable gates.
- Isolated Android worker provider adapters have no app call site or real-provider
  claim. API evidence: ANKIDROID_API.md, pinned 2.25.3; no Anki dependency/minimum
  adopted. Nostr config empty; ask Q188 before forwarding. Physical arm64 deferred.

## Next tasks (ordered)
1. Connect native recovery handles and prepared intent to typed Android/FFI
   read-only dispatch/preflight. Capture handle before read; validate supported
   full observations, permission/provider/deck/model failures and lost callbacks.
   Then route durable note/model begin/outcome commits before one live invocation.
   Test process death, stale callbacks and failed/lost begin/outcome commits.
2. Add compact recovery/error/explicit Retry projection, configured deck, Add to
   Anki/existing-note actions after dispatch gates. Handle absent/filtered deck and
   incompatible/uncertain model. Validate disposable real collection/browser/
   return/query cost before compatibility claims.
3. Resume Android native-phase/100k Select All Compose and observer-free optimized
   profiling plus accessibility/process-death matrix. Preserve full IDs/equality/
   replay guards. Heavy-history/diverse-book/full-load/exact peaks remain.

## Verification / blockers
- PASS: 13 new cases; 50 focused core/runtime/FFI Anki cases; 932 workspace tests/
  17 opt-in fixtures ignored; build, strict Clippy, Rustfmt/diff. Kotlin generation
  identical. Commands: DEVELOPMENT.md final section. Logs: /tmp/yomoku-anki-read-*.
- No open blocker. No Android APK/lint/emulator, host Kotlin/JNI JVM, real provider/
  collection/browser, independent Full-loader, physical arm64, broader lifecycle
  or performance check this slice. Native read/preparation remain unencodable.
