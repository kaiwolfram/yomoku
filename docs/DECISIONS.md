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

## 2026-10-22 — Crux application API and bridge direction
Verified crates.io with `cargo info crux_core` and inspected published 0.20.0
source (`src/lib.rs`, `src/core/mod.rs`, `Cargo.toml`). Pin `crux_core = 0.20.0`
(Apache-2.0, declared Rust minimum 1.90); use `App::update` returning `Command`,
`Core::process_event`, and the built-in render operation. Serde 1.0.228 with
derive supplies typed event/view serialization; its published local source is
available for API inspection. Cargo.lock pins transitive dependencies.

Primary references:
- https://docs.rs/crate/crux_core/0.20.0/source/src/lib.rs
- https://docs.rs/crate/crux_core/0.20.0/source/Cargo.toml
- https://redbadger.github.io/crux/part-1/shell/android/index.html
- https://docs.rs/serde/1.0.228/serde/

The current upstream Android guide uses BoltFFI for native Kotlin bindings and
Crux-generated typed data. Prefer that supported approach for the next bridge
slice; verify/pin the generator version and run a Kotlin round trip then. UniFFI
compatibility remains available in Crux but is not selected. Neither binding
generator is added in this slice. The serialized Crux bridge uses `update`,
`resolve`, and `view` with output buffers; old tutorial `process_event` bridge
examples must not be copied. Rust EffectRouter must consume Rust I/O effects
before they reach Android; only render/platform operations belong in Kotlin.

First application behavior is top-level destination selection (Library default),
already specified in the handoff. Every selection requests a render, including
reselection, so a shell can request its initial view. This is transient state;
it does not imply persistence or Nostr fetching. Settings/reader navigation will
be added with their actual flows, not guessed here.

## 2026-10-22 — Rust routing and serialized host boundary
Inspected published Crux 0.20.0 source in the Cargo registry: middleware Layer,
MapEffectLayer and Bridge, BincodeFfiFormat, ResolveRegistry, and crux_macros
0.10.1 effect implementation. Primary references:
- https://docs.rs/crux_core/0.20.0/crux_core/middleware/trait.Layer.html
- https://docs.rs/crux_core/0.20.0/crux_core/middleware/struct.Bridge.html
- https://docs.rs/crux_core/0.20.0/crux_core/bridge/struct.BincodeFfiFormat.html
- https://docs.rs/crux_macros/0.10.1/crux_macros/attr.effect.html
The web source fetch failed; local published source supplied the API evidence.

Use the upstream middleware bridge and serialization, with a separate shell effect
enum and exhaustive mapping. No hypothetical I/O operations are added just to test
routing. Future storage/network handlers belong below the shell mapping; their
async callbacks must be tested when introduced. The current callback cannot fire
from navigation alone. Use effect(typegen) to generate the Serde FFI enum; optional
runtime typegen forwards to Crux's generator feature (compilation verified).
Generator dependencies were fetched and locked; Kotlin generation/BoltFFI remain
next-session work. This choice does not preclude the newer facet generator.

Keep the host API to update/resolve/view, per-instance state, and categorized errors
without payload/error-string leakage. Tests verify invalid IDs return errors
rather than panic (the registry source is more current than some bridge comments).
No custom codec, unsafe native exports or pretend cross-language test is added.
Bincode's upstream trailing-byte acceptance is retained for generated-code parity.

## 2026-10-22 — Kotlin payload generator
Inspected published Crux 0.20.0 `src/type_generation/{serde,facet}.rs`, effect
macro implementation, and the current upstream Android guide:
- https://docs.rs/crux_core/0.20.0/crux_core/type_generation/facet/index.html
- https://redbadger.github.io/crux/part-1/shell/android/index.html
- https://www.boltffi.dev/docs/getting-started

Correction to the previous slice: the Serde type generator supports Java but not
Kotlin. Use the existing Crux Facet backend, with Facet exactly 0.46.5 (matching
Crux) and facet_generate 0.19.1 locked in Cargo.lock. Optional `facet_typegen`
replaces the unused Serde `typegen` feature. Serde still handles runtime Bincode;
Facet describes the same types for code generation. Enum repr(C) supplies Facet
layout metadata, not an exported C ABI or a persisted encoding contract.

The explicit generator registers ShellEffect's exported requests plus Event and
ViewModel, rather than the core's effect enum, so future Rust I/O does not become
shell work accidentally. Generated files go into a dedicated ignored output
folder; sources and lockfile are authoritative. `app.yomoku.shared` is a routine
internal namespace choice, not a publishing/application-ID decision.

Generation and deterministic output comparison passed; host bridge tests also
pass with Facet enabled. This does not establish Kotlin serialization compatibility
or a native ABI. BoltFFI errors/callbacks and compiler setup are a separate next
slice; no BoltFFI version has been adopted based on untested docs examples.

## 2026-10-22 — BoltFFI native facade
Pin BoltFFI to 0.30.1, a published release whose APIs were inspected, rather than
adopting a newer release solely from the registry's availability notice. Cargo.lock
also pins its companion crates to 0.30.1. Matching CLI installation remains next.
Primary API evidence:
- https://docs.rs/crate/boltffi/0.30.1
- https://www.boltffi.dev/docs/classes
- https://www.boltffi.dev/docs/errors
- https://www.boltffi.dev/docs/callbacks

Use an exported impl on NativeBridge, an error attribute on FfiError and an exported
Send + Sync listener trait. Keep the existing host facade usable from Rust without
requiring a foreign callback. Separate listener success/error methods avoid a
Result-valued callback argument; synchronous methods retain Result errors.
Build rlib for tests and staticlib/cdylib for native packaging. The wrapper adds
no state, lock or I/O beyond the existing Crux bridge. Owned callback lifetime is
tested on host; generated foreign handle lifetime still requires Kotlin tests.
Do not add a synthetic product event just to trigger asynchronous callbacks.
The original broad bridge checklist item is split into verified host exports and
remaining Kotlin/JNI work, without claiming a cross-language round trip.
