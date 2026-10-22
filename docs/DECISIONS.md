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

## 2026-10-22 — Android CLI emulator and executed bridge test
Use installed Android CLI `emulator create medium_phone` to download Emulator
37.2.12 and API 36 Google Play x86_64 image revision 7 and create the AVD. No
Android Studio or separate avdmanager installation is needed. Verified commands
against CLI help and official Android emulator/acceleration documentation:
- https://developer.android.com/tools/avdmanager
- https://developer.android.com/studio/run/emulator-commandline
- https://developer.android.com/studio/run/emulator-acceleration

Correct the environment diagnosis: `/dev/kvm` is hidden in the sandbox but exists
on the host; escalated `emulator -accel-check` reports KVM usable. SDK, AVD, ADB
and socket access require escalation here. Do not install virtualization packages
based solely on a sandbox-only device check.

Default startup segfaults in RenderThread/SwiftShader `libGLESv2.so`, captured
with GDB. Explicit `-gpu swangle`, no audio/cameras/snapshots, KVM and two cores
boot successfully. Persisting similar AVD settings did not make CLI headless
startup reliable; retain `tools/start-android-emulator.sh` with the verified flags.
It runs in the foreground, does not wipe data and accepts an optional AVD name.
Android CLI still creates/manages the AVD; the SDK executable supplies startup
flags the current CLI does not expose. Further default-renderer debugging is not
required for local development.

The image displayed a host-debugging consent prompt. Accepted persistent trust
through the emulator's authenticated control API using the SDK's installed proto
and temporary Python gRPC tooling under ignored `.local/`. Only keyboard input
worked for this prompt. No auth token/key or generated tooling enters Git. A
subsequent launcher cold boot preserved authorization. One CoreSession Android
test passes offline after fetching AGP's UTP dependencies once; APK installation,
Activity cold launch and the visible Rust-backed Library heading also pass.

## 2026-10-22 — Rust-owned navigation and retained Android session
Keep the four top-level destinations in Rust and add typed OpenSettings /
CloseSettings commands plus a view flag. Settings preserves the selected tab;
Back closes Settings, and selecting a destination closes Settings. These are
transient routes: a fresh process starts in Library. Settings and the four
screens remain empty placeholders until their respective vertical slices.
No Nostr starter configuration or settings values are invented.

Use the already resolved AndroidX Lifecycle **2.9.4** ViewModel as an explicit
app dependency. CoreSessionOwner retains only the native CoreSession and closes
it from onCleared; it holds no independent navigation/domain state, Activity or
Context. Activity recreation obtains the same owner without re-dispatching an
initial destination. This preserves the Rust model across rotation but does not
serialize navigation for process death. Rust remains the application state owner.

Use existing Material 3 NavigationBar / NavigationRail and TopAppBar. Choose a
rail at available Compose width >=600dp, otherwise a bottom bar; resize and
multi-window use actual available width rather than a device/tablet flag. The
rail scrolls on short windows and consumes Scaffold insets. Settings is a toolbar
flow with Back and no main navigation. Reader navigation hiding will be wired
when a real reader route exists; no synthetic reader command is introduced.
Navigation icons are small original vector resources with adjacent text labels.

Add only Compose ui-test-junit4 to instrumentation, using the existing Compose
BOM **2026.03.01** (resolves **1.10.6**); createAndroidComposeRule uses MainActivity,
so no test-manifest dependency is necessary. Verify actual JNI-backed tab /
Settings rendering, system Back, Activity recreation and final session closure.

Primary API/version evidence reviewed:
- https://developer.android.com/topic/libraries/architecture/viewmodel
  (Activity-scoped retention and onCleared cleanup).
- https://developer.android.com/jetpack/androidx/releases/lifecycle#2.9.4
  (retain the version already resolved by Activity, without upgrading the stack).
- https://developer.android.com/develop/ui/compose/layouts/adaptive/use-window-size-classes
  (600dp compact/medium boundary, dynamic available window width).
- https://developer.android.com/develop/ui/compose/components/navigation-bar
- https://developer.android.com/develop/ui/compose/components/navigation-rail
- https://developer.android.com/develop/ui/compose/testing
  (ui-test-junit4, createAndroidComposeRule and semantics assertions).

## 2026-10-22 — Stable identities and Unicode text anchors
Use distinct positive signed 64-bit DocumentId/SourceId/ChapterId values. They
fit SQLite INTEGER and Kotlin Long exactly without new ID dependencies. Scope
them to one database (backup is replacement, not merge). Future repositories
must use AUTOINCREMENT and preserve high-water marks so deleted committed IDs
cannot be reassigned; allocation itself remains unimplemented. Source identity
is historical provenance, separate from the live document and duplicate hash.
Chapter identity survives page-length changes; non-EPUB text has one text unit.

Use chapter-local UTF-8 byte offsets into immutable canonical extracted text,
with half-open ranges. Check numeric bounds at Rust construction/deserialization;
check ownership, ordering and scalar boundaries against actual text at use.
Do not normalize text or use page numbers/UTF-16 indices as durable anchors.
Provide pure explicit UTF-16 conversions for Kotlin/Compose; reject surrogate
interiors. Grapheme selection and Japanese page-character counting are separate
future reader concerns. ChapterText is a snapshot, not an editing command.

Register these types with the existing Facet generator. Use a test-only Cargo
example plus Kotlin fixtures to exchange real Crux Bincode payloads in both
directions, including independently computed Kotlin anchors, supplementary
Unicode and IDs above 2^53. This verifies the payload boundary without adding
synthetic production events or native methods. Live document JNI and async
callbacks stay unchecked until document effects exist. Reuse pinned Serde as a
dev dependency for the example; no dependency versions were added or upgraded.

Primary references reviewed:
- https://doc.rust-lang.org/std/primitive.str.html#method.is_char_boundary
- https://doc.rust-lang.org/std/primitive.str.html#method.encode_utf16
- https://kotlinlang.org/docs/characters.html
- https://www.sqlite.org/autoinc.html

## 2026-10-22 — Immutable paste repository
Implement the next host-testable persistence slice with the existing pinned
rusqlite/core dependencies; no dependency versions or application architecture
change. Storage depends on core's pure document values, never the reverse.
`PasteText` rejects whitespace-only input without trimming valid content. The
literal first LF/CRLF-delimited line supplies the title, preserving spaces and
allowing an empty first line rather than inventing a fallback. Canonical text
retains line endings, NUL and Unicode exactly; no normalization or truncation.

Append migration 0002 with separate sources (kind/original title), chapter
identities/order, Library documents (live title) and canonical chapter text.
Use positive AUTOINCREMENT identities and bound parameters. Paste is one text
unit at index zero; a heading never creates inferred chapters. The provenance
tables are separate so future deletion can remove live content while retaining
historical anchors. Do not expose rename/deletion or change deletion semantics
in this step; raw SQL fixtures test separation and nonreuse only.

`Storage::create_paste` performs all inserts under one immediate transaction and
returns a snapshot only after commit. `load_paste` uses a read transaction and
distinguishes absent documents from inconsistent stored content. Neither method
edits canonical text. Both are synchronous Rust I/O for future worker effects;
they are not yet Crux events, FFI exports or Android import UI. Keep duplicate
detection, atomic vocabulary registration, timestamps and reader progress with
their planned tasks rather than claiming a complete product import pipeline.

Verified insert/commit/migration failure rollback, exact Unicode and reopen,
concurrent writes, independent i64 allocations including values above 2^53 and
i64::MAX, committed-ID nonreuse after deletion/reopen, and safe ID exhaustion.
The existing migration-history corruption fixture needed updates scoped to one
row after adding a second migration; the full suite then passed. No change to
migration 0001 or the reject-without-repair policy.

## 2026-10-22 — Real storage effects and private Android database
Use Crux 0.20.0's existing EffectMiddleware/EffectResolver/HandleEffectLayer for
StorageOperation before the exhaustive ShellEffect mapping. Verify APIs against
the pinned published crate's src/middleware/{mod,effect_handling}.rs and
src/command/mod.rs (primary source); upstream documents the same layer at
https://docs.rs/crux_core/0.20.0/crux_core/middleware/trait.EffectMiddleware.html .
Resolution must happen asynchronously; a std::thread FIFO channel worker owns a
lazy Storage connection. This needs no async executor or new dependency version.
Add the existing pinned rusqlite as an FFI test dependency solely for a real
write-lock fixture proving dispatch/view/navigation stay responsive.

NativeBridge requires a database path and starts the worker without opening a
file. Thread creation failure is WorkerUnavailable; SQL/open failures become
DocumentError view values without paths/SQL/error strings. Failed opens remain
retryable and never reset data. Android's Activity-scoped owner uses the existing
AndroidViewModel Application factory to supply filesDir/yomoku.sqlite3, retains
no Activity, and requests Library once per session. All database work is Rust
worker I/O; callback Render requests are posted onto Android's main thread.

Accept one storage command per session at a time; expose busy/error state for
future UI controls and ignore overlapping storage submissions. Navigation stays
available. Internal StorageFinished is skipped by Serde/Facet; forged completion
payloads fail before mutation. List minimal ID/title summaries in allocation order
until Library sort/progress tasks exist. A successful create adds the summary;
load returns immutable text without opening a reader or adding reading history.

Closing a native session stops accepting work, lets submitted atomic operations
finish and drops late results. Do not join a potentially locked SQLite worker on
the UI thread. Host JNI verifies eventual callback registry release even when
closing during I/O. Crux's weak-reference shutdown diagnostic is benign here.
Paste UI, import dedup/registration, reader progress, pagination and rename/delete
remain separate tasks. No migration, product deletion semantics, Nostr config or
dependency version changed.

## 2026-10-22 — Pure logical pages and exact-offset reflow
Implement the next three phase-2 pure-domain tasks together, without adding
dependencies, SQL, app routes or FFI types. The user explicitly clarified that
all non-whitespace Unicode characters count toward the five page presets,
including punctuation and Latin text. Count Rust Unicode scalars: supplementary
kanji/emoji count once; combining marks remain separate scalars. Keep the exact
canonical text, including whitespace and line endings, in the page ranges.

Routine boundary choices: each LF-delimited hard line ends a paragraph (CRLF is
preserved). Prefer the first paragraph end at/after the threshold. Define an
extremely long continuation as a paragraph end exceeding twice the preset from
the page start; scan that paragraph for the first following sentence at/after
the threshold, then repeat for the remaining text. Japanese 。！？ and ASCII !?
end sentences; ASCII periods need whitespace, EOF or a punctuation/closing-quote
tail, avoiding decimal splits. Keep adjacent closing quotes/brackets and terminal
punctuation with the sentence. These are lightweight boundary heuristics, not a
linguistic parser. If no sentence break exists, retain the long text through the
paragraph end; never invent a mid-sentence hard cut. Trailing whitespace extends
the last page rather than producing an extra blank page.

Ordered chapter snapshots are the only source of chapter boundaries. Reject
mixed-source/duplicate chapter identities; never infer chapters from headings.
An empty chapter keeps one empty range for a stable EOF position, while no
chapters yields no pages. Image-only extraction/rendering remains future work.
LogicalPages borrows immutable snapshots and exposes derived TextAnchor ranges.
ReadingPosition retains source/chapter/exact byte offset independently. Reflow
returns that same position plus its new containing page, without snapping to
page start. Boundary offsets select the following page, except chapter EOF
selects that chapter's last page. Reject invalid ownership and scalar offsets.

Fourteen tests verify thresholds/long-paragraph policy, exact text coverage,
LF/CRLF, mixed scripts, supplementary/combining Unicode, all 25 reflow preset
pairs, chapter boundaries, EOF and invalid positions. This is a host-domain slice;
reader commands, global settings, progress persistence and Android remain next.

## 2026-10-22 — Paste UI and first-page reader route
Continue phase 2 with an independently testable Android slice: paste → committed
Library title → explicit first-page reader opening. Split Library current progress
from the paste/open checklist item so progress stays unchecked until its durable
position command exists. Page controls, swipes, reflow Settings and completion
remain the following slice; opening currently starts at page 1.

Core owns ContentRoute (Main/Paste/Reader), Settings overlay and OpenReader's
pending document identity. LoadPaste remains a snapshot command for repository
clients. OpenReader uses the same FIFO storage load, then derives ReaderPage from
pure LogicalPages at the default 500 preset. Typed page text, source anchor,
separate ReadingPosition and counter cross generated FFI; Compose performs no
pagination or Unicode slicing. Leaving the reader clears the pending identity,
so late successful loads cannot reopen it. Storage failures remain typed/retryable.

A successful create returns to Library without opening the reader. The existing
per-session busy guard and disabled controls prevent overlapping submissions;
this does not replace future exact-content deduplication/import registration.
Keep unsaved draft text in Compose rememberSaveable through Settings/recreation;
clear it on leaving the form. Library rows show exact persisted titles; percentage
awaits durable progress. Reader uses one centered, vertically scrollable column
with a 720dp maximum width and content-bottom page counter. Main navigation hides
for paste/reader/Settings; toolbar and system Back use Rust events. No dependencies,
migrations, data-destructive semantics or Nostr configuration changed.
