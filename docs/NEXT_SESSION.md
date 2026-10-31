# Resume here
Updated: 2026-10-31. Read-only Android Anki provider prerequisite verified.
Commit guidance: explanatory bodies; merges include provenance. Keep verification
results in session/progress docs and reports, out of commit message bodies.

## Current slice
- Unconnected `AnkiProviderReader`: fresh enabled/exported provider/spec >=2 and
  permission checks; deck IDs/names/filtered flags; worker-only ContentResolver.
- Exact-first-field bound candidates across models/decks, verified again locally.
  Recognize core escaping and syntax-free plain surfaces; no arbitrary HTML/entity
  decoding, trimming or Unicode normalization. Embedded NUL fails closed.
- Null/failed/malformed/partial/close-failed reads never yield empty success;
  successful empty reconciliation stays Unresolved. Long IDs/multiple matches stay
  exact. Resolve supported browser links in the current provider package only;
  return intent without launching. Query cost on large collections unmeasured.
- Manifest declares narrow provider visibility and database permission. No permission
  prompt, app/FFI call site, settings/UI action, model creation or note insertion.
  Kotlin result types are shell-local prerequisites, not a serialized protocol.
- Pure retained core export remains in-memory only. Durable pending/uncertain
  recovery and no mutating replay on restart are required before enabling export.
- Evidence: ANKIDROID_API.md (AnkiDroid 2.25.3). No API dependency/minimum app version
  adopted, no AnkiDroid installation/real collection access/note export performed.
- Schema 0020. Nostr starter configuration empty; ask Q188 before forwarding.

## Next tasks (ordered)
1. Continue Android model-layout validation and mutating provider adapter contract:
   never overwrite user models by name, recheck duplicates immediately before
   insertion; null/exception/transport loss after submission must be Uncertain.
   Preserve frozen core payload/deck and read-only reconciliation; extend fake-
   provider cases. No completion fence permits insertion after empty reconciliation.
2. Add app-owned retained export state, typed Event/FFI/provider bridge and durable
   pending/uncertain recovery across process death before enabling Add to Anki/
   default-deck settings/existing-note action. Reject stale shell selections and
   never replay mutation on restart. Test a disposable real AnkiDroid collection
   before compatibility claims; verify actual browser/return behavior and query cost.
3. Resume Android native-phase/100k Select All Compose and observer-free optimized
   profiling, plus accessibility/process-death matrix. Preserve full IDs/equality/
   replay guards. Heavy-history/diverse-book/full-load/exact peaks remain. Physical
   arm64 is user-deferred; emulator/host results do not establish hardware viability.

## Verification / blockers
- PASS: 12 new in-process fake-provider tests + 3 CoreSession regressions (15 total,
  0.129 s), API-36 x86_64 isolated 8 GiB AVD; debug/test APKs, strict Android lint,
  repository Kotlin formatting and diff check. JNI/bindings for both ABIs and debug
  Sudachi probe rebuilt by Gradle; assets reused. No independent Full-loader run.
- Commands: DEVELOPMENT.md final read-only Android Anki section. Logs:
  /tmp/yomoku-anki-read-{android,tests,format}.log. Kotlin fixture inference and lint
  findings corrected. Sandbox Gradle/ADB sockets needed approved execution.
  Default 6 GiB AVD installation failed for storage; switched to documented ignored
  .local/statistics-ui-avd 8 GiB image, without clearing app data. No open blocker.
- Both task-started AVD runs stopped. No Rust-source/schema/dependency/protocol
  change; host Rust workspace/Clippy/Rustfmt, real Anki provider, physical device,
  broad UI/lifecycle, Full-loader and performance checks were not run this slice.
