# Resume here
Updated: 2026-10-31. Isolated Android Anki model creation verified.
Commit guidance: explanatory bodies; merges include provenance. Keep verification
results in session/progress docs and reports, out of commit message bodies.

## Current slice
- Unconnected `AnkiProviderReader`: fresh provider/spec >=2/permission gates,
  worker-only deck/model discovery, exact-surface duplicates and reconciliation.
  `modelMatches` reads a positive signed-64 ID's metadata and actual template rows:
  exact name/four ordered fields/CSS, normal type, sort field zero, one ordinal-zero
  forward card with exact name/question/answer and empty browser overrides.
  Expected layout comes from future core bridge; no production HTML/CSS copies.
  Same-name discovery never implies compatibility or selects/overwrites a model.
- Separate unconnected `AnkiProviderModelCreator` refuses exact-name conflicts,
  inserts a normal four-field/one-card model, validates a fresh returned URI/ID,
  updates only its ordinal-zero template and requires full layout readback.
  Refresh host/spec/permission between calls. Update counts measure properties,
  not rows. Any failure after submission is Uncertain with a known new ID retained;
  no retry/repair/delete. A partial/orphan model may remain; observations are
  non-atomic. Missing/incompatible layout is false; malformed/failed reads fail.
  Default model deck is not a layout constraint: explicit frozen-deck placement
  remains required. No note insertion or live app/FFI/UI call site exists.
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
1. Implement note insertion adapter contract: revalidate layout/deck and recheck
   exact duplicates immediately before insertion; place cards explicitly in the
   frozen configured deck.
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
- PASS: 11 new fake-provider cases, 21 prior provider cases and three CoreSession
  regressions (35 total, 0.217 s), API-36 x86_64 isolated 8 GiB AVD; debug/test APKs,
  strict Android lint, repository Kotlin formatting and diff check. Native/bindings/
  assets reused. Commands: DEVELOPMENT.md final Anki creation section. Logs:
  /tmp/yomoku-anki-create-{android,tests,format,format-check,emulator}.log.
- Sandbox ADB socket attempt failed; approved Gradle/ADB/emulator execution passed.
  No app data cleared. Task-owned emulator stopped. No open blocker.
- No Rust/schema/dependency/protocol change. Host Rust/Clippy/Rustfmt, independent
  Full-loader, real Anki provider, physical device, broader UI/lifecycle and
  performance checks were not run this slice.
