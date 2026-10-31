# Resume here
Updated: 2026-10-31. Isolated Android Anki note insertion verified.
Commit guidance: explanatory bodies; merges include provenance. Keep verification
results in session/progress docs and reports, out of commit message bodies.

## Current slice
- Unconnected worker-only Anki provider boundaries: `AnkiProviderReader` freshly
  gates host/spec >=2/permission, discovers decks/models, validates full layout,
  looks up exact first-field duplicates across models/decks and reconciles reads.
  Rust owns layout/content; Kotlin comparison data is not production HTML/CSS.
- `AnkiProviderModelCreator` refuses exact-name conflicts and configures only a
  validated new returned model ID, requiring full readback. Any post-submission
  failure is uncertain with a known ID retained. No retry/repair/delete; partial
  models may remain. Default model deck is not a layout constraint.
- New `AnkiProviderNoteInserter` freezes raw surface/four fields/model/layout/deck,
  validates exact escaped first-field identity and rejects NUL/delimiter ambiguity.
  Revalidate full layout and existing nonfiltered frozen deck, then exact duplicates
  as the last collection read before submission. Return all matches without writes.
  Validate canonical returned URI/positive signed-64 ID and exact note content,
  require one ordinal-zero card, explicitly update its deck and require note/card/
  deck readback. Insert alone uses the model default deck; never change that default.
- Refresh host/spec/permission between calls. Every failure after submission is
  Uncertain, preserving request/deck/known returned ID; no retry/repair/delete.
  A note may remain in the model default deck. Surface reconciliation establishes
  existence, not placement success. Empty/failed reads never permit insertion.
  Observations are non-atomic; duplicate query cost remains unmeasured.
- Pure retained core export remains in-memory only; no app/Event/FFI/UI call site.
  Before live calls the durable owner must record pending submission, invoke once
  and recover read-only uncertainty across process death without mutation replay.
  Manifest declarations do not request permission. No real-provider compatibility
  claim. Evidence: ANKIDROID_API.md, pinned AnkiDroid 2.25.3; no API dependency or
  minimum supported version adopted. Schema 0020. Nostr starter config empty;
  ask Q188 before forwarding.

## Next tasks (ordered)
1. Add durable pending/uncertain export recovery and app-owned retained state with
   a typed Event/FFI/provider bridge. Preserve exact payload/deck on restart; never
   replay mutations. Reject stale shell selections and test interrupted insertion/
   placement, duplicate/repeated callbacks, empty reconciliation and restore errors.
2. Wire configured-deck settings/Add to Anki/existing-note action only after that
   recovery prerequisite. Handle permission, missing/filtered deck, incompatible/
   conflicting model and uncertain model creation. Test a disposable real AnkiDroid
   collection before compatibility claims; verify browser/return and query cost.
3. Resume Android native-phase/100k Select All Compose and observer-free optimized
   profiling, plus accessibility/process-death matrix. Preserve full IDs/equality/
   replay guards. Heavy-history/diverse-book/full-load/exact peaks remain. Physical
   arm64 is user-deferred; emulator/host results do not establish hardware viability.

## Verification / blockers
- PASS: 16 new fake-provider cases plus 32 prior provider cases and three CoreSession
  regressions (51 total, 0.304 s), API-36 x86_64 isolated 8 GiB AVD; debug/test APKs,
  strict Android lint, repository Kotlin formatting and diff. Native/bindings/
  probe/assets reused. Commands: DEVELOPMENT.md final Anki insertion section.
  Logs: /tmp/yomoku-anki-insert-{android,tests,install,format,format-check,emulator}.log.
- Sandbox Gradle wildcard-IP and ADB socket attempts failed; approved offline
  Gradle/ADB/emulator execution passed. No app data cleared. Task-owned emulator
  stopped. No open blocker.
- No Rust/schema/dependency/protocol change. Host Rust/Clippy/Rustfmt, independent
  Full-loader, real Anki provider, physical device, broader UI/lifecycle and
  performance checks were not run this slice.
