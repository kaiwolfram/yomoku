# Architectural decisions

## 2026-10-22 — First independently testable implementation
The repository contained only the handoff and no commits. Start a dependency-free
Cargo workspace with `yomoku-core` domain value types/policies. This is the first
piece of the required pure Crux core, not an alternative framework. Crux model,
effects and FFI remain to be implemented after checking current APIs. Avoid empty
runtime/storage/FFI scaffolds until their first behavior can be tested.

Pull the small phase-3 vocabulary rules forward to establish a tested host
baseline while Android SDK setup remains outstanding. Registration returns an
explicit create/existing decision; it does not simulate persistence or encounters.
Completion status policy is pure; transaction/idempotency/event execution is not
implemented yet. No database schema or serialized enum discriminants are fixed.

Surface preserves the exact supplied Unicode spelling without trimming or
normalization; blank values are rejected. Analyzer output is responsible for
Japanese-token eligibility. Reading and lemma are not identity fields, as required
by the handoff. Text offset encoding remains undecided until FFI/anchor design.

## 2026-10-22 — CLI environment
Use the installed stable toolchain binaries directly in this sandbox because the
rustup launcher attempts read-only self-maintenance. No toolchain repair is needed
for current host checks. Android CLI 1.0.16625370 is installed and operational with
sandbox escalation; it reports `/home/kai/Android/Sdk`. Only `build/templates`
0.1.1 is installed. Its empty-activity template supports Compose/AGP 9. Inspect
actual generated dependency versions before establishing the Android baseline.
No Android Studio dependency is introduced.

## Deferred product inputs
- Curated Nostr npubs/relay URLs: not supplied; empty configuration is permitted.
- Q188: user must choose queued retry or best effort before Citrine forwarding.
- Neither question blocks local reader work. Reaction retry is already required.
