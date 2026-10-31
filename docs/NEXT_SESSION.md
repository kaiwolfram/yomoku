# Resume here
Updated: 2026-10-31. Read-only Android Anki model validation verified.
Commit guidance: explanatory bodies; merges include provenance. Keep verification
results in session/progress docs and reports, out of commit message bodies.

## Current slice
- Unconnected `AnkiProviderReader`: fresh provider/spec >=2/permission gates,
  worker-only deck/model discovery, exact-surface duplicates and reconciliation.
- `modelMatches` reads a positive signed-64 ID's metadata and actual template rows:
  exact name/four ordered fields/CSS, normal type, sort field zero, one ordinal-zero
  forward card with exact name/question/answer and empty browser overrides.
  Expected layout comes from future core bridge; no production HTML/CSS copies.
  Same-name model discovery never implies compatibility or selects/overwrites one.
- Missing/incompatible layout returns false; malformed/null/partial/close-failed
  reads fail. Recheck host/permission between queries; observations are non-atomic.
  Default model deck is not a layout constraint: explicit frozen-deck placement
  remains required. No model creation/update/delete or note insertion exists.
- Duplicate lookup recognizes core escaping/syntax-free plain first fields across
  models/decks, without arbitrary HTML decoding/normalization. Embedded NUL fails
  closed. Empty reconciliation stays Unresolved; query costs remain unmeasured.
- Pure retained export remains in-memory only. Durable pending/uncertain recovery
  and no mutating replay on restart are required before enabling app/FFI/UI calls.
  Manifest permission/visibility declarations do not request permission.
- Evidence: ANKIDROID_API.md, AnkiDroid 2.25.3. No API dependency/minimum supported
  version, real collection access or provider compatibility claim. Schema 0020.
  Nostr starter configuration empty; ask Q188 before forwarding.

## Next tasks (ordered)
1. Implement mutating provider adapter contract: create models without overwriting
   user models, revalidate layout/deck and recheck exact duplicates immediately
   before note insertion; place cards explicitly in the frozen configured deck.
   Null/exception/transport loss after submission must be Uncertain. Preserve
   payload/deck and read-only reconciliation; extend fake-provider cases. No
   completion fence permits insertion after empty reconciliation.
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
- PASS: nine new fake-provider cases, 12 prior provider cases and three CoreSession
  regressions (24 total, 0.162 s), API-36 x86_64 isolated 8 GiB AVD; debug/test APKs,
  strict Android lint, repository Kotlin formatting and diff check. Native/bindings/
  assets reused. Commands: DEVELOPMENT.md final Anki model section. Logs:
  /tmp/yomoku-anki-model-{android,tests,format,format-check,emulator}.log.
- Sandbox Gradle/ADB sockets required approved execution. One APK install attempted
  before boot finished failed; installation after boot succeeded. No app data
  cleared. Task-owned emulator stopped. No open blocker.
- No Rust/schema/dependency/protocol change. Host Rust/Clippy/Rustfmt, independent
  Full-loader, real Anki provider, physical device, broader UI/lifecycle and
  performance checks were not run this slice.
