# Architectural decisions

## 2026-10-22 — Android native packaging
Pin NDK r30 **30.0.16248370**, the current official LTS, and install Rust
`aarch64-linux-android` / `x86_64-linux-android` standard libraries for the
existing stable toolchain. Package only arm64-v8a and x86_64 initially, for a
64-bit device and emulator respectively. The API level stays 29. Primary evidence:
- https://developer.android.com/ndk/downloads (r30 version and LTS status).
- https://doc.rust-lang.org/rustc/platform-support/android.html (Android targets
  and the supported LTS NDK policy).
- https://developer.android.com/ndk/guides/other_build_systems (ABI triples,
  API-suffixed Clang and Linux toolchain layout).
- https://developer.android.com/guide/practices/page-sizes (16 KB ELF/ZIP alignment).
- Published BoltFFI CLI 0.30.1 `src/config/targets/kotlin.rs`,
  `src/toolchain/android.rs`, `src/pack/android/{mod,link}.rs`.

Use the pinned BoltFFI `pack android` command to build Rust static archives,
generate Kotlin/C and link one `libyomoku_jni.so` per ABI. Keep all generated
sources, headers and libraries under ignored target directories. Gradle preBuild
runs the checked-in offline script, tracks Rust sources/manifests and generated
outputs, and packages those libraries and Kotlin codecs. Tool installation remains
an explicit workstation setup step. The script verifies the selected NDK revision
because BoltFFI otherwise falls back to another installed NDK if the pin is absent.
Do not run bridge generation/build commands concurrently against shared output.
This development step uses Rust's debug profile; release optimization is deferred
to the planned release tooling.

AGP 9 built-in Kotlin uses `android.sourceSets.main.kotlin.directories`; adding
Kotlin files only to the Java directories does not compile them. Register only
the generated `app` and `com` source folders: Crux also emits a standalone
`build.gradle.kts`, which must not be compiled as an Android Kotlin source.
Reference: https://developer.android.com/build/migrate-to-built-in-kotlin and
the AGP 9.0 `AndroidSourceSet` API documentation.

`CoreSession` owns one native handle on Android's main thread and exposes only a
Compose render snapshot from Crux. It dispatches generated typed events, handles
Render requests and closes the handle with the Activity. It queues future native
callbacks onto the main thread and ignores callbacks after close. Async errors
currently fail visibly via the typed exception; add recoverable error UI alongside
the first real async effect. No new domain state, SQL or platform I/O enters core.
Activity recreation currently creates a fresh core; retain the core across
configuration changes when adding interactive navigation.

Pin AndroidX test runner **1.7.0** and JUnit extension **1.3.0**, verified at
https://developer.android.com/jetpack/androidx/releases/test and
https://developer.android.com/training/testing/instrumented-tests/androidx-test-libraries/runner.
The instrumentation test exercises command → JNI → core → render snapshot,
independent sessions and handle closure. Building its APK does not establish
Android execution; a connected device/emulator is required to check off that task.

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
by [the product specification](PRODUCT_SPEC.md). Text offset encoding remains
undecided until FFI/anchor design.

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
already specified in [the product specification](PRODUCT_SPEC.md). Every selection
requests a render, including reselection, so a shell can request its initial view.
This is transient state;
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

## 2026-10-22 — Executed Kotlin/JNI boundary
Install the matching BoltFFI CLI 0.30.1 under ignored `.local/`, using its locked
published dependency graph. Adopt standalone Kotlin compiler 2.4.21 for the host
test, targeting JVM 17 bytecode and running on available OpenJDK 25.0.4.1. This
does not select Android Gradle/Compose/JDK versions. Primary evidence:
- https://www.boltffi.dev/docs/getting-started
- https://kotlinlang.org/docs/command-line.html
- https://github.com/JetBrains/kotlin/releases/tag/v2.4.21
- Published `boltffi_cli-0.30.1/src/config/targets/kotlin.rs` and
  `src/commands/generate/bindings.rs` in the Cargo registry.
- Official OpenJDK `jdk-25+36` JNI headers, downloaded unmodified into ignored
  `.local/jni-25/` because the installed Java runtime has no development headers.

Keep native bindings in `app.yomoku.nativebridge`, separate from Crux's
`app.yomoku.shared` payloads; load `yomoku_jni` through the system library path.
Generate JNI C rather than maintaining a handwritten JNI bridge. Host verification
links generated C against the Rust shared library with relative runtime lookup.
Generated C/Kotlin must compile without warnings; the JVM runs with JNI checks.

Native bindings require explicit close. Verify listener release by inspecting the
pinned generator's callback registry, avoiding nondeterministic GC assertions or
test-only product exports. Revisit that inspection when upgrading the generator.
The tests cover real payloads/errors/lifecycles; asynchronous delivery remains
pending a real async effect. InvalidEffectOutput is not reachable with Render-only
effects, so no artificial effect is introduced to force that error.

Observed and reproduced: `boltffi generate kotlin` leaves a host library without
the native bridge exports after metadata expansion. Rebuild the native library
after generation. The checked-in runner enforces this order; sharing the target
directory with concurrent generation/build is unsupported. Both generators
produce identical repeated output. Tool binaries/generated files stay ignored.

## 2026-10-22 — Product specification consolidation
Move the original handoff's product scope, architecture, behavior, exclusions,
UX details and unresolved inputs into [PRODUCT_SPEC.md](PRODUCT_SPEC.md), now
the authoritative requirements document. Preserve all eight requirement sections
verbatim apart from heading numbers, including Q188 and empty starter-data policy.
The original document remains available in Git history.

Retire its outdated first-session instructions and duplicate implementation
roadmap. AGENTS.md owns the session workflow; IMPLEMENTATION_PLAN.md owns ordered
tasks and the twelve retained cross-feature acceptance cases. NEXT_SESSION.md
owns current progress and commands; architecture/data-model docs describe actual
implementation. Routine technical decisions here do not override confirmed
product requirements. Update active references to the new specification.

## 2026-10-22 — Android Compose build foundation
Generate `android/` using the installed Android CLI 1.0.16625370 and its
empty-activity template bundle 0.1.1. Retain Gradle 9.1.0 with the template's
SHA-256-pinned distribution, AGP 9.0.1, SDK/target 36, min SDK 29, Compose BOM
2026.03.01 and Activity Compose 1.13.0. Pin build tools to 36.0.0. Use JDK 17
for both the Gradle JVM and compiler toolchain; the sandbox has a verified local
Temurin 17.0.21+9 archive because its system Java installation is runtime-only.
Normal Fedora development can use `java-17-openjdk-devel`. Do not require Studio
or automatic JDK resolver plugins. Native code is not part of this slice; select
and verify an NDK when adding Android ABIs rather than installing unused tooling.

Use Kotlin/Compose compiler 2.4.21, already verified for the host codecs and in
Kotlin's supported Gradle 9.1/AGP 9.0 compatibility range. AGP's built-in Kotlin
remains enabled; applying the Compose compiler plugin supplies the matching Kotlin
version without a second Android Kotlin plugin. The template's Kotlin 2.3.20
compatibility table only explicitly lists AGP through 9.0.0.

Remove template data repository, Kotlin navigation/ViewModel state, serialization
plugin, unused dependencies and sample greeting tests. They do not implement
Yomoku behavior and would duplicate the planned Rust application boundary. The
initial Kotlin shell shows only a Library heading with system theme/window insets;
Rust-backed navigation/ABI packaging and device tests remain separate steps.
`app.yomoku` is a provisional development namespace/application ID. Disable
Android automatic backup and exclude all backup/transfer domains to honor the
manual encrypted backup requirement before adding persisted data.

Use ktfmt 0.64 CLI (Apache-2.0) in Kotlin language style for Android source and
Gradle scripts via `tools/format-kotlin.sh`; Android lint and Kotlin warnings fail
on warnings. Suppress only lint's dependency-upgrade reminders since these pinned
versions are reviewed explicitly. Downloaded tools and build products stay ignored.

Primary evidence reviewed before adoption:
- https://developer.android.com/build/releases/agp-9-0-0-release-notes
  (Gradle 9.1, build tools 36.0.0, JDK 17 and built-in Kotlin).
- https://kotlinlang.org/docs/gradle-configure-project.html
  (2.4.21 compatibility table).
- https://developer.android.com/develop/ui/compose/setup-compose-dependencies-and-compiler
  (Compose compiler version must match Kotlin; Compose BOM dependencies).
- https://developer.android.com/develop/ui/compose/bom/bom-mapping
- https://developer.android.com/jetpack/androidx/releases/activity#1.13.0
  (browser fetch timed out; official release HTML retrieved with curl).
- https://adoptium.net/installation/archives and official Adoptium release API
  metadata (JDK archive SHA-256 verified before extraction).
- https://github.com/Kotlin/ktfmt and published Maven Central 0.64 CLI help
  (JDK 11+, Kotlin style, dry-run and failure-on-change options).

## 2026-10-22 — SQLite initialization and migration boundary
Take the independent phase-1 storage step while Android execution is blocked;
persistence has priority over navigation styling. Pin **rusqlite 0.40.2** (MIT),
with default features disabled and only `bundled`. Cargo.lock pins
libsqlite3-sys 0.38.2, whose published amalgamation is SQLite 3.53.2. Bundling
keeps SQLite consistent across host and Android without a system SQLite package
or runtime dependency on Android's private SQLite libraries. No async framework
is needed for initialization; the future EffectRouter must run this synchronous
I/O off Android's main thread. Do not add a storage dependency to core or expose
connections/SQL through FFI.

Use an ordered migration journal (version and name) as the sole version source,
avoiding a second version in `PRAGMA user_version`. The first numbered migration
creates that journal; domain tables wait for stable IDs/text offsets and actual
commands. Keep applied SQL immutable and append new migrations. Validate journal
prefixes and acquire an immediate transaction before reading history, applying
all pending SQL and journal rows atomically. Reject newer/inconsistent/unknown
databases; never silently recreate or downgrade. Keep SQLite's default rollback
journal/durability settings for now; enable foreign keys on every connection and
set a five-second busy timeout. WAL/concurrency policy belongs with real effects.

Primary evidence checked before adoption:
- https://docs.rs/rusqlite/0.40.2/rusqlite/struct.Connection.html
  (`open`, `busy_timeout`, `pragma_update`, `transaction_with_behavior`).
- https://docs.rs/rusqlite/0.40.2/rusqlite/struct.Transaction.html
  (explicit commit; rollback on drop).
- https://github.com/rusqlite/rusqlite (bundled SQLite build).
- https://www.sqlite.org/lang_transaction.html (immediate write locking).
- Published rusqlite 0.40.2 Cargo.toml/source and libsqlite3-sys 0.38.2
  Cargo.toml/sqlite3.h in Cargo's registry cache (features, versions, licenses).

Eleven on-disk host tests cover initialization/reopen, ordered upgrade fixtures,
Unicode content/path persistence, failed fresh/existing migrations, newer and
inconsistent journals, unrelated/invalid files, foreign keys and concurrent opens.
These do not claim document CRUD, Android persistence or async JNI execution.
