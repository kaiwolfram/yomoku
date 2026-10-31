# Resume here
Updated: 2026-10-31. First MVP excludes Nostr; Feed is a static placeholder.
Commit guidance: explanatory bodies; merges include provenance. Keep verification
results in session/progress docs and reports, out of commit message bodies.

## User checkpoints
- Announce when no plan steps remain; ask when product opinions/clarifications
  are needed. Recommend physical testing when it is more useful than more code.
- User reports the app works reasonably well on their physical phone. Many small
  polish issues can wait; continue the implementation plan next session.
- Detailed physical validation remains unchecked; phone feedback is acceptable.
- Feed replaces Nostr in navigation and shows only `Coming Soon! 🌱`. Nostr tasks,
  identity and Q188 are deferred beyond the first MVP (PRODUCT_SPEC.md §5).

## Physical test APK
- `target/releases/yomoku-0.1.0-0374365-release.apk` (~242 MiB), Android 10+,
  arm64/x86_64; optimized Rust + release app, locally development-signed.
- PASS: assembleRelease/lintRelease, signature, ZIP/ELF alignment, CRC/assets/probe
  exclusion. User reports acceptable phone use; no agent-run hardware checks.
  Rebuild commands: DEVELOPMENT.md “Local optimized release APK” section.

## Current slice
- Feed UI/resources and navigation regression updated; docs reflect reduced scope.
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
  adopted. Nostr is outside the first MVP; ask Q188 only if forwarding is resumed.

## Next tasks (ordered)
1. Connect native recovery handles and prepared intent to typed Android/FFI
   read-only dispatch/preflight. Capture handle before read; validate full
   observations and provider/permission/deck/model failures. Route durable begin/
   outcome commits before one live invocation; test death/stale/lost callbacks.
2. Add recovery/error/explicit Retry projection, deck settings, Add to Anki and
   existing-note actions after gates. Validate disposable real collection/browser/
   return/query cost. Performance/accessibility matrix remains in the plan.

## Verification / blockers
- PASS: Feed debug/test APKs, strict lintDebug, Kotlin format/diff; NavigationTest
  2/2 on phone and 2/2 on tablet-sized API-36 x86_64 emulator. Display reset.
  Commands: DEVELOPMENT.md final Feed section; logs: /tmp/yomoku-feed-*.
- Initial emulator install lacked space; trim-caches resolved it, no data reset.
- No open blocker or remaining work for Feed request. Rust/JNI, release and phone
  trials not rerun; prior native verification is in DEVELOPMENT.md native-read.
- Next value: typed Anki reads/preflight before another physical trial; real
  provider/collection/browser, arm64 and broader performance matrix remain open.
