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

## 2026-10-23 — Durable current position before page completion
Implement the independently testable persistence foundation of the next reader
slice using existing dependencies. Append migration 0003 rather than altering
published schemas. Keep one optional document/chapter/UTF-8 offset row; source
ownership derives from documents and is validated along with scalar boundaries
on save/load. Use an immediate transaction for validation/upsert and return only
after commit. Failed or corrupt positions never trigger repair or position reset.
The document FK cascades only progress, matching specified future deletion;
source/chapter provenance remains. No production delete command is introduced.

Use current UTF-8 byte offset / canonical UTF-8 byte length for paste percentage,
floored to whole percent. This matches the durable offset unit, includes original
whitespace/NUL, never counts UTF-16 units and uses u128 arithmetic to avoid
overflow. Library SQL obtains BLOB lengths without fetching all book text; core
owns the checked percentage calculation. Full scalar validation occurs on load
and save; listing checks ownership and numeric bounds. Future chapter importers
must sum preceding canonical chapter lengths rather than use page numbers.

Expose a typed SaveReadingPosition hook scoped to the active reader, using the
existing busy guard/FIFO worker. Keep the last committed page and summary visible
until success, and allow writes to finish after leaving without reopening the
route. This is position-only infrastructure, with no user-facing seek UI and no
completion/status/encounter side effects. Do not map Next/swipes to it alone:
forward navigation still needs atomic completion IDs/history and vocabulary work.
Reopen projects the containing page while preserving its exact saved offset.

Use transient Compose remember { ScrollState(0) }, keyed by document/page anchor,
instead of rememberScrollState's saved instance state. Page changes, route reopen
and Activity recreation start at the top. No vertical offset enters Rust/SQLite.
No dependencies, Nostr configuration or product completion behavior change.

## 2026-10-23 — Persisted global page length and reader reflow
Continue the independent Settings task while forward navigation awaits atomic
completion/history and real analysis/vocabulary repositories. Append migration
0004 with a constrained singleton, default 500, storing character counts rather
than enum ordinals. No new dependencies, destructive semantics or Nostr changes.
Missing/corrupt settings are errors, never repaired by selecting a new value.
Use the existing FIFO worker, immediate transactions and commit-before-render
policy; failure leaves the selected preset/page unchanged and permits retry.

Library/document loads carry the stored preset, including on fresh processes and
direct reader opens. Core exposes no selected preset until a successful load;
Compose disables unknown/busy settings and can retry initial loading. A typed
SetPageLength event handles only the five presets. The active immutable reader
snapshot is retained independently of repository LoadPaste snapshots so reflow
cannot use another document's text. Reproject its exact current ReadingPosition;
never replace it with the new page's start or write progress during reflow.
Returning from Settings remounts the transient reader scroll at the top. Late
commits after leaving update the global value without reopening any route.

## 2026-10-23 — Repository completion IDs and atomic history foundation
Append migration 0005 using existing pinned dependencies. Allocate positive i64
command IDs in SQLite before submission, rather than introducing random IDs or
process-local counters. An unused allocation is not a completion and is not resumed
automatically. Freeze the command's page, exact departing position, page preset and
timestamp; retries reuse that payload/ID, while rereads obtain a fresh ID.

Pure core validates the logical page and derives its destination; SQLite checks
the current preset and performs history insert plus position upsert under one
immediate transaction. Check replay before live-document validation so duplicate
callbacks remain harmless after reflow, progress changes and future deletion.
Replay must never upsert position: a late callback could otherwise undo Previous.
Reject payload conflicts and stale positions instead of silently accepting them.

Store dated, versioned page anchors with retained source/chapter provenance and
a historical document ID without a live-document FK. No data-destructive API,
dependency version or Nostr configuration changes. Command timestamps are explicit
Unix milliseconds supplied by the future I/O caller; core reads no clock.

Keep this foundation repository-only. Do not route forward navigation through it
until actual analyzed occurrences, New→Known/status history and active time join
the same transaction. Final-page EOF storage does not claim finished-book behavior.
Ten new repository tests cover idempotency, concurrency, rereads, Previous-only
saves, rollback/retry, Unicode/ownership/staleness, migration upgrades/failure,
deletion provenance, reopen and command-ID exhaustion.

## 2026-10-23 — Global vocabulary repositories and dated status history
Complete the independent phase-3 persistence prerequisite before adopting the
analyzer/dictionary. Append migration 0006 with existing dependencies. Store exact
surface strings under SQLite BINARY uniqueness, positive nonreused VocabularyId
values and explicit status strings, never Rust enum ordinals. Registration time
is a caller-supplied nonnegative Unix millisecond value; core reads no clock.

Register a batch under one immediate transaction, emitting one version-1 New
registration event per newly created surface. Preserve every existing status and
registration date. Read-only lookup does not register missing entries. Manual
selection commits the current status and dated old/new event together; selecting
the current status emits no event. An absent ID returns None instead of recreating
it. Statement or deferred-commit failures roll back both state and history.

Keep the methods repository-only until actual analysis and typed application
effects exist. Standalone registration is not an atomic document import: future
import must compose registration with content/anchors in the same transaction.
Dictionary-form links await analyzer output; reading occurrences, completion-driven
New→Known events and active time still must join completion atomically. These
repositories must not be used to approximate those operations with separate commits.

Vocabulary events reference their entry with ON DELETE CASCADE, implementing the
specified history-erasure schema for future confirmed vocabulary deletion. There
is no document FK or delete API. Test-only deletes verify document independence,
entry history cleanup and fresh registration IDs without introducing destructive
user actions. Eleven new tests cover identity/inflections/Unicode, all statuses,
lookup/no-ops, explicit history, concurrency, migrations, reopen and rollback.

## 2026-10-23 — Source-anchored analysis and reversible boundary schema
Complete the independent anchor/schema prerequisite before adopting an analyzer
revision or populating imports. Keep `analysis` in pure core for validation only;
actual tokenization will live in `yomoku-analysis`. No dependency/version was added.

Upstream evidence inspected on 2026-10-23:
- [Morpheme API](https://worksapplications.github.io/sudachi.rs/rust/sudachi/analysis/morpheme/struct.Morpheme.html):
  begin/end use original-text byte offsets and surface uses original text, separately
  from dictionary/normalized form and reading. This fits existing UTF-8 anchors.
- [Tokenizer API](https://worksapplications.github.io/sudachi.rs/rust/sudachi/analysis/stateful_tokenizer/struct.StatefulTokenizer.html)
  exposes explicit Mode, reset/do_tokenize and result extraction.
  [Dictionary API](https://worksapplications.github.io/sudachi.rs/rust/sudachi/dic/dictionary/struct.JapaneseDictionary.html)
  exposes configuration/storage construction. Hosted pages report mixed engine
  versions; verify/test source at an exact pinned revision before adoption.
- [Sudachi manifest](https://github.com/WorksApplications/sudachi.rs/blob/develop/Cargo.toml)
  currently reports develop version 0.7.1-a1/Apache-2.0.
  [Upstream README](https://github.com/WorksApplications/sudachi.rs/blob/develop/README.md)
  warns that 0.7 patch APIs may change and crates.io registration remains unchecked.
- [SudachiDict format notes](https://github.com/WorksApplications/SudachiDict/blob/develop/README.md)
  distinguish Full (including proper nouns) and require V1 for engine 0.7, V0 for
  0.6; release assets switch to V1 from v202610. An exact compatible engine/Full
  artifact, checksum, offline packaging and device measurements remain unselected.
- [Dictionary LEGAL](https://github.com/WorksApplications/SudachiDict/blob/develop/LEGAL)
  specifies Apache-2.0 plus UniDic and NEologd notices. Preserve the chosen artifact's
  complete license/legal notices when bundling. No dictionary is bundled yet.

Store exact engine/adapter/configuration and dictionary artifact revisions per
immutable analysis snapshot, with explicit A/B/C mode (default B). Do not substitute
normalized forms for source surfaces. Validate exact source ownership, scalar byte
boundaries, ordered nonoverlap and coverage of every non-whitespace byte. Permit
whitespace gaps so adapters need not manufacture whitespace morphemes. Persist
optional analyzer readings/dictionary forms per token without introducing manual
lemma corrections or making vocabulary IDs part of source-token identity.

Migration 0007 adds base snapshots/tokens and separate reversible boundary edits.
A selected exact span is partitioned into one (merge) or multiple (split) surfaces;
its parts must reproduce every source byte. Global scope defaults to AllOccurrences,
keyed by exact surface; local scope uses source/chapter/range. Replacing an exact
active key deactivates the old edit and allocates a fresh ID in one transaction.
Identical saves reuse the active ID. Reversing disables only that ID and leaves
superseded edits inactive, base analysis unchanged and past encounters untouched.
Keep source metadata/original selected bytes so global edits survive deletion of
their originating document. Live analysis cascades with chapter text, independently
of vocabulary deletion. No production document deletion behavior changes.

These are repository primitives, not an override application policy or UI. Do not
choose overlap precedence or arbitrary substring matching here: cross-occurrence
matching must be tested with the real analyzer before exposing segmentation edits.
Existing imports are not rescanned. Atomic import/vocabulary composition and real
completion occurrence snapshots remain separate unchecked work. Fifteen new core/
storage tests use explicitly synthetic adapter fixtures; they verify anchors,
reversal, immutable output, concurrency, migrations, corruption and rollback,
and do not claim verified Sudachi segmentation.


## 2026-10-23 — Pinned real Sudachi adapter and Full host fixtures
Adopt the specified Sudachi.rs engine at stable v0.7.0, exact commit
`5de0410b6edfd428fc510cc9763440772e9619d5`, with Full 20261015 (V1).
Verified primary source at that commit: [manifest](https://github.com/WorksApplications/sudachi.rs/blob/5de0410b6edfd428fc510cc9763440772e9619d5/sudachi/Cargo.toml),
[configuration](https://github.com/WorksApplications/sudachi.rs/blob/5de0410b6edfd428fc510cc9763440772e9619d5/sudachi/src/config.rs),
[morpheme API](https://github.com/WorksApplications/sudachi.rs/blob/5de0410b6edfd428fc510cc9763440772e9619d5/sudachi/src/analysis/morpheme.rs)
and [sentence detector](https://github.com/WorksApplications/sudachi.rs/blob/5de0410b6edfd428fc510cc9763440772e9619d5/sudachi/src/sentence_detector.rs).
Use StatefulTokenizer/MorphemeList and Config::new_embedded; no external plugins,
user dictionaries or filesystem configuration overrides enter this adapter.
Use SentenceDetector's signed boundaries directly: negative values identify
provisional splits, needed for long chapters exceeding the engine's u16 input
limit. Preserve chapter-global byte coordinates across these analysis chunks;
these boundaries do not become reader pages or inferred chapters.

[Official Full release](https://github.com/WorksApplications/SudachiDict/releases/tag/v20261015)
metadata reports archive SHA-256
`b38318ca26a444cd713a5a377a353308ef38c733c64cc0be905122527516c793`;
downloaded bytes match. The 330,506,339-byte system_full.dic hashes to
`7c797531dea76f06b629fad39cb3a5bdb3a9e54c67ef3d9a869e61418426a7d1`.
Keep the large data outside Git. Explicit Python preparation verifies archive,
data and both complete notice files, writes only named ZIP members to a temporary
directory, then publishes the prepared directory. Existing resources are verified,
never overwritten. Complete unmodified archive LEGAL/LICENSE are also retained in
third_party/sudachi. Offline preparation and invalid-archive preservation pass.
Android packaging/attribution UI remain unchecked.

Constructor verification uses RustCrypto [sha2 0.11.1](https://docs.rs/sha2/0.11.1/sha2/)
(Digest/Sha256 API, MIT/Apache-2.0). The verified owned Vec moves into Sudachi
Storage::Owned, so hashing and parsing use exactly the same immutable bytes without
unsafe file mapping. This is a host-tested foundation, not an Android memory
claim; it holds roughly 315 MiB before engine structures. Measure/revisit storage
before device integration. Pin regex 1.12.4, already in the lock graph; its
[Unicode script API](https://docs.rs/regex/1.12.4/regex/#unicode) supports source
Hiragana/Katakana/Han detection. Combine that with analyzer POS to exclude known
symbols, and permit OOV Japanese script because the fallback OOV plugin labels
rare supplementary kanji as symbols. Never fabricate OOV readings/lemmas.

Keep analyzer surfaces separate from normalized/dictionary forms. Skip empty
original spans generated during normalization and whitespace-only tokens; require
full core coverage/nonoverlap validation before returning any analysis. Default B,
A/C alternatives and exact immutable profile identities are explicit. Retain raw
Sudachi inflection boundaries (e.g. 食べ + た); manual conjugation merges and override
application remain separate work. No import/status/completion behavior changes.
Nine real Full tests cover these contracts, proper names including kanji-only
names, punctuation-ending names, normalization, NUL/emoji/rare kanji, whitespace,
72 KB unbroken input and persistent A/B/C SQLite snapshots after reopen. These
large-data tests are explicitly ignored in ordinary workspace runs; run them via
tools/test-sudachi.sh, which fails if data is absent rather than silently skipping.

Cross-target checking exposed an upstream blocker at this exact engine revision:
plugin/loader.rs defines make_system_specific_name for Linux/FreeBSD/Windows/macOS/
Apple embedded targets, but not Android (E0425). Both Android target checks fail.
Do not introduce fake target cfg flags, modify the shared Cargo cache or claim
Android support. Keep this crate outside runtime/FFI dependencies until a pinned
upstream fix or documented source patch is available. Existing app builds remain
independent; host integration is verified. This does not change the required
Sudachi architecture or adopt an alternate engine.

## 2026-10-23 — Pinned Sudachi Android loader source patch
The [upstream loader](https://github.com/WorksApplications/sudachi.rs/blob/5de0410b6edfd428fc510cc9763440772e9619d5/sudachi/src/plugin/loader.rs)
omits Android from make_system_specific_name. The inspected upstream develop
loader still omits it. Preserve v0.7.0 and all dependency versions rather than
changing engine APIs. Apply a root Cargo source patch to a checked-in,
dependency-only copy of this exact commit. Add Android to the existing
Linux/FreeBSD `.so` cfg; Yomoku's embedded-only configuration remains unchanged.
No analysis code, segmentation defaults, plugin selection or dictionary changes.

Keep all crate src files and upstream embedded resources/README/LICENSE with
their original Apache-2.0 notices. Reduce the nested workspace to this library
and omit upstream dev/bench targets/dependencies and their external fixtures.
The application tests exercise the dependency; the snapshot is not a standalone
upstream test checkout. Mark each modified file. Retain a complete three-file
patch plus original upstream SHA-256 hashes for every included file.
tools/verify-sudachi-source.py reverses the patch in a temporary copy and verifies
both exact file membership and every upstream hash. tools/test-sudachi.sh runs
that verification before its real-data fixtures. No shared cache mutation,
build-time network download or target cfg impersonation is needed.

Include +android-loader-v1 in the engine profile so persisted snapshots identify
the patched build without reusing the old profile key. Existing immutable
snapshots stay valid; no migration or rescan is performed. Both Android target
checks/library builds and the host Full-data fixtures pass. This removes the
compile blocker, not the device performance/bundling gate. Runtime, FFI and UI
still have no analyzer entry point; Full's owned-byte memory cost remains to be
measured on device before app integration.

## 2026-10-23 — Explicit Android Full assets and complete notices

Split packaging/attribution from the device analyzer measurements so each is
independently verifiable. Keep the pinned dictionary outside Git; Android builds
now require explicitly prepared Full data. Gradle's bundleSudachi task calls the
existing preparation tool in a separate --android-assets mode that never enters
the download path. Verify all source SHA-256 values before touching existing
outputs, stage/copy/reverify all three files, then publish generated assets.
Missing/corrupt inputs fail; no silent fallback or dictionary substitution.
No dependencies, engine versions or storage semantics change.

Use normal APK asset compression for this packaging step. The app does not copy
or load the 330,506,339-byte dictionary at startup. Evaluate the actual analyzer
loading/storage strategy with device measurements before runtime integration.
Settings offers a scrollable, selectable dialog with attribution and both entire
notice files, loaded on Dispatchers.IO. Its open state survives recreation; this
local presentation state does not enter Crux. Installed-asset checks use streaming
hashes to avoid retaining the full dictionary in the test/app Java heap.

## 2026-10-23 — Standalone Android Full measurements before app integration

Keep the measurement harness outside runtime/FFI/APKs. A Rust example executes the
real pinned adapter with owned bytes; a Python CLI verifies source/data, links
with the existing pinned NDK and runs fresh Android processes. It records actual
initialization/analysis times and kernel RSS/HWM/anonymous/file/swap snapshots.
No new library, version, schema or product behavior is introduced. The driver uses
unique temporary device paths, preserves failed output, checks offline radio
settings, and restores/verifies the originals even after probe failure.

Three release and three debug API 36 x86_64 runs pass the real A/B/C and Unicode
fixtures offline. Both Android probe targets link; only x86_64 executes. Do not
call fresh processes cold-storage tests: the filesystem cache is not flushed.
APK decompression, app memory/lifecycle and physical arm64 remain unmeasured.
Exact per-run results and methodology live in SUDACHI_MEASUREMENTS.md.

Release initialization is 1.03–1.20 s, versus debug 12.31–12.69 s. Owned storage
holds about 318 MiB anonymous memory after initialization, with roughly 334 MiB
release peak RSS through long analysis. That measured anonymous allocation
justifies comparing Sudachi's existing file-backed storage before app integration;
the final production storage strategy remains unchecked. Keep owned loading as a
correctness baseline. Future extraction must publish verified versioned private
files without mutating bytes held by a live analyzer. Load on demand off the main
thread and reuse one worker-owned analyzer, rather than reloading on every page
or eagerly when Library starts. Do not skip dictionary integrity checks to make
the unoptimized debug initialization appear faster.

## 2026-10-23 — Verified mapped Full comparison with an explicit lifetime contract

Keep `Analyzer::open_full` as the safe owned-byte baseline. Add an explicitly
unsafe `open_full_mapped(&File)` for controlled immutable files. Hash the mapping
itself before passing that same storage to Sudachi; never verify one path and
reopen another, or skip the pinned checksum. Sudachi owns the mapping until the
analyzer drops, even after the original file handle closes. Loading strategy does
not change analysis/profile identity or any persisted token semantics.

Use the already locked transitive memmap2 **0.9.12** directly, pinned exactly;
no graph versions change. Verified its `Mmap::map` and lifetime/safety API in the
local registry source at this exact version, alongside Sudachi's pinned
`dic/storage.rs` and `dic/dictionary.rs`. The
[primary API documentation](https://docs.rs/memmap2/0.9.12/memmap2/struct.Mmap.html)
requires callers to prevent all backing-file mutation/truncation while mapped.
A read-only descriptor, checksum or chmod alone is not that guarantee. Keep this
precondition visible in the unsafe API; arbitrary source paths use owned loading.
Production private-file publication/lifecycle enforcement remains a separate task.

The measurement driver owns a unique private directory, finishes copying before
making the dictionary read-only, never rewrites it during execution, and removes
it only after all probes exit. `--storage both` runs three fresh owned/mapped
pairs by default, in that order, without flushing filesystem caches. Preserve
raw storage-labelled results and fail on missing validation/storage markers.
Android radio changes are asynchronous: poll verified settings for at most ten
seconds before measuring and after restoring them, rather than accepting a race
or proceeding online. Every run still restores original settings in `finally`.

Host tests run every Full fixture with each loader and compare complete A/B/C
outputs, including profiles/readings/anchors and 72 KB input. Source verification,
engine/data pins and runtime/FFI boundaries stay unchanged. Mapping reduces the
anonymous allocation in Android measurements, but checksum verification touches
all mapped pages, so initial total RSS remains similar. See SUDACHI_MEASUREMENTS.md;
do not equate file-backed residency with demonstrated low-memory behavior or
finished app viability. Keep on-demand worker reuse as an integration requirement.

## 2026-10-23 — On-demand verified private dictionary publication

Implement the private-file part of the loading gate independently of analyzer
integration. Kotlin owns APK asset extraction/platform file operations; core,
SQLite, runtime and FFI remain unchanged. The helper has no startup caller and
rejects main-thread opens. Use `Context.noBackupFilesDir/sudachi`, a SHA-256-named
Full file, a 64 KiB streaming buffer and the existing pinned size/checksum. No new
dependencies or versions. Storage demand is approximately 315 MiB in addition to
the compressed APK, with another staging copy needed when repairing an old file.

Serialize publication with a JVM monitor plus a persistent file-channel lock
across processes; JVM file locks alone throw on overlapping same-process locks.
Under that lock, discard only this helper's abandoned staging files, verify an
existing descriptor/permissions or extract to a separate temporary inode. Sync
the complete file, verify its on-disk bytes, mark it mode 400, sync again, rename
atomically and sync the directory. Return that same verified read-only descriptor
at offset zero, with explicit caller ownership. Cache reuse still hashes every
byte but does not decompress the APK again. Failures propagate and can retry.

Never open a published inode for writing/truncation, even during corruption
recovery. Replacement changes the directory entry while existing descriptors and
mappings keep their original inode. A failed copy/rename preserves the prior
publication; an unclean process exit may leave only disposable staging bytes or
a complete publication that the next open rechecks. Read-only permissions do not
stop a future same-UID writer: all future cleanup/update code must honor this
directory's ownership and immutable-inode rule. Native integration must consume
the verified descriptor and retain the mapped constructor's own checksum check.
No production mapping, analyzer reuse, lifecycle/low-memory or physical-arm64
claim follows from this helper alone.

API evidence: Android's [Os reference](https://developer.android.com/reference/android/system/Os)
for fstat/fchmod/fsync/rename/open, [private no-backup directory](https://developer.android.com/reference/android/content/Context#getNoBackupFilesDir()),
and Java 17 [FileChannel locks](https://docs.oracle.com/en/java/javase/17/docs/api/java.base/java/nio/channels/FileChannel.html#lock()).
Checked the installed API-36 android.jar signatures too; Android's public
OsConstants lacks O_DIRECTORY, so directory sync uses an O_RDONLY handle.

## 2026-10-23 — Debug-only in-app Full descriptor/reuse measurement

Extend the loading gate with a separate `yomoku-analysis` cdylib example and a
debug-source-set JNI facade, leaving the production BoltFFI/runtime dependency
graph unchanged. No new dependencies, versions, schemas or reader behavior.
Build the probe with release Rust even in the debug Compose APK, using existing
NDK r30/API 29 and sequential native tasks. The release app excludes the probe.

Use JNI primitives only for descriptor, generation, fixture code and error/token
count. Fixed A/B/C/Unicode/page/long-input fixtures avoid introducing a parallel
application API. Duplicate the verified open descriptor with BorrowedFd into an
owned File, never reopen its pathname; retain the mapped constructor's checksum.
Kotlin holds its descriptor open until nativeOpen returns. The test-private
publisher owns immutable inodes until analyzer drop. Thread-local Rust state
enforces worker affinity without moving Sudachi between threads or retaining
Java references. Generation IDs distinguish reopened analyzers and reject
stale or foreign-worker calls; catch panics before returning to Java.

A test-only ViewModel retains a lazy single executor across real Activity
recreation/stop/resume, then closes native state on that executor when cleared.
No production Activity starts or owns this worker. Instrumentation records
Debug.MemoryInfo PSS/private-dirty and /proc/self/status RSS/HWM/anonymous/file/
swap; /proc/self/maps checks actual mapping removal before test-directory cleanup.
Timed validation calls include the executor/JNI round trip. Do not equate these
instrumented debug-app measurements with release app/physical-device viability,
kernel memory pressure, cold-storage behavior or production worker integration.

API evidence: [JNI naming, native arguments and primitive mappings](https://docs.oracle.com/en/java/javase/17/docs/specs/jni/design.html),
and Rust 1.99.1 [BorrowedFd lifetime and exact-description duplication](https://doc.rust-lang.org/std/os/fd/struct.BorrowedFd.html).
The boundary never dereferences its unused JNI pointers. Production event/view
payloads continue to use the existing generated, typed BoltFFI bridge.

## 2026-10-23 — External Full process-death recovery probe

Use a separate debug-only Activity and Python/ADB driver to terminate an app with
a live mapped analyzer. Instrumentation cannot kill its own process and continue
asserting recovery, so an external driver launches both MainActivity and the probe
in the same process, waits for validated/mapped state and force-stops the package.
Confirm all package processes exit before restart or removal of test-private files.
The recovered process must have a different PID, reverify through both publisher
and native mapping, pass the real fixtures and preserve publication inode/mtime/
size/read-only permissions. A normal onDestroy queues nativeClose and publishes
a marker; this marker must be absent in the killed-process experiment.

Protect the exported debug entry point with the system DUMP permission (ADB shell
can launch it); accept only fixed phases and a 32-hex run token, never arbitrary
paths or native input. Production startup/runtime/FFI remain independent. The
Activity and manifest entry live only in the debug source set alongside the
existing measurement JNI facade; release APKs must exclude them.

This measures force-stop recovery with a complete publication, not LMKD pressure,
interrupted extraction, power failure or physical-device viability. Leave the
broader loading/lifecycle gate unchecked. No library/version changes.
API evidence: Android's [ADB activity-manager commands](https://developer.android.com/tools/adb#am)
and [DUMP permission](https://developer.android.com/reference/android/Manifest.permission#DUMP).


## 2026-10-24 — Atomic analyzed-paste repository composition

Compose the existing paste, analysis and vocabulary repositories through private
transaction helpers, keeping their standalone commands intact. Add
`Storage::create_analyzed_paste` with an analyzer callback rather than a storage
dependency on Sudachi. One immediate transaction allocates canonical identities,
passes their exact chapter text to the callback, validates/stores the default
Mode B snapshot, and registers all unique Japanese original surfaces and dated
New events. Existing statuses/dates/IDs remain unchanged. Never register analyzer
lemmas, normalized text, non-Japanese tokens or reading encounters during import.

Analysis runs inside the transaction so anchors use the actual allocated IDs,
without provisional-ID rewriting or partially committed documents. This holds
the SQLite write lock during tokenization: the worker caller must preload/reuse
its analyzer first, and the callback must not access the database or expose IDs
before success. Preserve typed analyzer errors in `PasteImportError<E>` without
exposing SQL/engine dependencies to core or changing the production FFI schema.
All analyzer/validation/write/commit failures roll back the entire new import.

This is an independently verified repository prerequisite while the Full loading/
lifecycle gate remains open. Keep the existing Android text-only paste effect
until that gate and production import wiring are ready. Exact-hash deduplication,
other import formats, dictionary-form aggregates and completion/time are separate
unchecked work. No new dependencies, schema changes or destructive semantics.

## 2026-10-24 — Exact-byte paste identity before analysis

Add `Storage::import_analyzed_paste` as the deduplicating repository entry point,
keeping raw creation primitives for existing callers and transaction fixtures.
For pastes, hash every canonical UTF-8 byte without normalizing line endings,
Unicode, NUL or whitespace. File importers must hash original file bytes in a
separate domain when implemented; identical extracted text is not file identity.
Reuse the already locked sha2 0.11.1 as a direct storage dependency, with no
version/new-package changes. Verify its one-shot `Sha256::digest` API against
[primary crate documentation](https://docs.rs/sha2/0.11.1/sha2/).

Migration 0008 stores a 32-byte BLOB digest per live document with a nonunique
digest/document index. Preserve legacy duplicates and reference the lowest live
document ID deterministically, rather than merging entries or changing progress.
Opening creates the empty index only. On import, lazily hash unindexed pastes one
text at a time within the same immediate transaction as duplicate lookup and
conditional analyzed creation. This can hold the write lock across a large legacy
Library; it avoids startup scans, partial indexes and schema-time custom SQL.

Return an explicit Existing versus Created outcome. Existing skips the analyzer
and vocabulary registration entirely, including entries later deleted by the user;
return its current title/progress and validate exact text against the digest match.
Concurrent submissions serialize before checking identity, so one analyzes/creates.
Commit new text, analysis, vocabulary, hash and any legacy hash backfill together;
all failures roll back. Hashes cascade only with their live document, allowing
fresh IDs/progress after deletion while history/provenance/vocabulary remain.
SQLite's [foreign-key action documentation](https://www.sqlite.org/foreignkeys.html#fk_actions)
governs this existing cascade mechanism. No document deletion API is introduced.

This remains repository-only work while actual Full reclamation and production
worker ownership are unresolved. Android still uses text-only raw creation.
Production import integration must adopt the deduplicating entry point after the
gate; file import formats, summary/cancellation and completion/time remain open.

## 2026-10-24 — Pure active-time accounting before completion integration
Continue the independent phase-2 active-time prerequisite while Full pressure and
hardware measurements remain open. Add a pure domain accumulator with existing
dependencies, without attaching an incomplete timer to the production reader.
The caller supplies monotonic milliseconds; elapsed time is capped five minutes
after the last interaction. Opening/resuming starts an idle window; duplicate
resume notifications do not extend it. Any foreground reader/dictionary
interaction resets idle, including after timeout, without backfilling idle time.
Background/close settles the interval and pauses; interactions while paused do
not restart it. No periodic callback is needed for accurate idle capping.

Checkpoint settles elapsed time without pausing, resetting idle or draining time.
Use nonnegative i64 milliseconds for the pending duration, compatible with future
SQLite integer persistence. Restore only that duration, always paused, never the
monotonic clock epoch/running state: time away must not count across reopen/reboot.
Reject clock regression/overflow before mutation rather than clamping corrupt
values. Use timestamp differences to avoid idle-deadline overflow.

Thirteen domain tests verify these rules, including simulated restart/reboot with
a fresh clock epoch. No schema, FFI payload, app route or Android timer is added.
Durable page attribution/reflow, lifecycle/interactions and completion integration
stay unchecked. Freeze time with the future completion command ID and commit it
with occurrences/New→Known/position; checkpoints are never reading statistics.

## 2026-10-24 — Defer physical arm64 testing to the user's usable-app trial
The user will test their physical device once the app is usable. Physical arm64
validation is a later user-owned task, not a prerequisite for runtime integration
or continued emulator development. Do not wait for hardware access or claim that
emulator execution proves hardware viability. Earlier references grouping arm64
with the Full integration gate are superseded by this decision.

Continue emulator pressure/reclamation work and choose production worker
lifecycle/reuse, then integrate analyzed imports and atomic reader completion.
Keep recording the actual ABI/device/build and limits of each verification run.

## 2026-10-24 — Full mapping reclamation and production lifetime

Two real anonymous-allocation pressure runs on API-36 x86_64 reclaim 68,408 and
146,792 KiB of Full dictionary residency while its mapping stays open. The same
PID/generation/worker then passes A/B/C, Unicode/OOV/NUL, page and long-input
fixtures. Kernel file scan/steal counters increase; no cache drop, madvise or
synthetic trim signal is used. PSI is denied by device policy and recorded as
unavailable; do not infer stall latency. Exact results: SUDACHI_MEASUREMENTS.md.
The bounded shell helper uses the already pinned NDK, allocates 32 MiB steps,
fills all bytes with deterministic pseudorandom data to resist zram compression,
and releases on EOF/quit/process exit with a 180-second alarm fallback. Driver
bounds are at most 70% of guest RAM and stop after >=64 MiB measured reclaim.
This is enough evidence to proceed with emulator integration, not a device-wide
LMKD survival or physical-arm64 claim. The user owns the later hardware trial.

Choose the verified immutable private-file mapping for production. Keep the safe
owned loader for uncontrolled paths/tests. Integrate one lazy analyzer into the
existing Rust FIFO EffectRouter worker, preload before opening the analyzed-import
transaction, and reuse through foreground imports/configuration changes. Do not
load Full on Library startup, move Sudachi across threads or reopen a pathname in
place of the verified descriptor. Initialization failure leaves no cached analyzer
and can retry. Publisher repair must continue preserving live inodes.

Treat the analyzer as a disposable cache: release on that worker when UI becomes
hidden/backgrounded and on final session shutdown. Queue release after in-flight
atomic work; backgrounding must not interrupt/roll back an accepted import merely
to trim memory. Subsequent work lazily re-verifies/reopens. Configuration changes
keep the retained session/cache; do not interpret every Activity stop as process
background. No new timer/executor/library or global analyzer singleton is needed.
Production lifecycle/descriptor delivery remains an implementation/test task,
but the loading decision and emulator measurement gate are now resolved.

Primary references: Android's
[ComponentCallbacks2 contract](https://developer.android.com/reference/android/content/ComponentCallbacks2)
identifies UI-hidden/background as cache release opportunities; running low-memory
levels are no longer delivered since API 34. The kernel's
[PSI documentation](https://docs.kernel.org/accounting/psi.html) distinguishes
measured stalls from ordinary residency/counter snapshots. Neither synthetic
callbacks nor unavailable PSI establish real pressure/reclaim on their own.

## 2026-10-24 — Analyze imports on the existing FIFO worker

Implement the Rust worker part first with an opt-in router accepting a Send
loader factory. The ready analyzer need not be Send: create, use and drop it only
on the worker. The platform's later loader must publish/verify and pass the same
immutable private descriptor, then map it before the repository opens its write
transaction. No path or SQL enters core. Existing dependency versions are reused;
analysis/rusqlite are only added as runtime test dependencies in this step.

Reuse a healthy analyzer after SQL rollback, but discard one returning an engine
error or invalid output so retry starts fresh. Library, settings and blank input
do not initialize it. Runtime dates registration using UNIX wall time; reading
time keeps its separate monotonic clock. A cold duplicate currently preloads the
analyzer as required by the repository contract, then skips tokenization/rescan.

Queue foreground/hidden policy on the same FIFO. Hidden imports finish atomically
and release their analyzer before resolving; background signals arriving during
an import release after the accepted work. Foreground only permits caching.
Use a weak lifecycle sender so a retained platform control cannot keep the worker
or its callback/factory alive after close. Duplicate policy signals are harmless.

Add an internal Existing result and upsert the core Library row by document ID;
both import outcomes return to Library without opening the reader. Keep the
current native constructor on its existing text-only router for this independently
testable step; production analyzed-import work remains unchecked until descriptor
delivery, lifecycle and visible Created/Existing summaries are verified on Android.

## 2026-10-24 — Transient single-paste import summary

Implement the summary independently of native analyzer delivery. A committed
Created/Existing result projects a typed ImportSummary (document ID, current title,
outcome) into core view state. Android displays Imported/Already in Library above
the Library list with an explicit Close action; it never opens the reader.
No timer or modal blocks subsequent work. Keep this transient result through
Settings, Library refresh and retained Activity recreation; clear on dismissal,
tab selection, opening a reader/paste form or accepting another import. A fresh
session has no summary. Failures remain in the paste form with the draft intact.

These are routine presentation choices under PRODUCT_SPEC.md §3.3's required
summary/Library return. No new persistence, dependency or import semantics. The
native shell still uses text-only creation, so duplicate summaries are exercised
through the analyzed worker and core; Android duplicate delivery remains part of
the production integration task. Do not mark that task complete for this UI slice.


## 2026-10-24 — Production analyzed imports and descriptor ownership

Require DictionaryProvider in NativeBridge; select the analyzed router exclusively.
Keep the text-only Rust constructors for low-level storage fixtures, never as an
Android fallback. Add the existing analysis crate to FFI without new versions.
Library/settings/reopen/blank paste remain lazy; a cold duplicate may preload Full
but never tokenizes/rescans the existing document or recreates deleted vocabulary.

The publisher runs synchronously on the existing FIFO worker. Kotlin verifies the
private immutable publication, duplicates that exact read-only descriptor using
ParcelFileDescriptor.dup, then detachFd transfers ownership to Rust. Return FD+1
as an i64 so zero (the generator's callback failure default) cannot accidentally
consume stdin. Rust owns/closes the FD on load success/failure and rehashes the
mapping itself. No pathname reopens and no Activity/session captures in the provider.
Never mutate published inodes, including while accepted work drains after close.
Host-only JNI fixtures use JDK reflection to duplicate the prepared immutable Full
file; module-opening flags stay in the test runner, not the Android application.

Use Activity onStart to permit reuse and onStop to queue background release, except
isChangingConfigurations retains the warm worker/analyzer across recreation.
CoreSessionOwner still closes only on final onCleared. No new executor, third-party dependency version,
global analyzer singleton or eager Library initialization. SQL rollback retains
healthy cached analysis; provider/engine failures remain retryable typed view errors.

Primary API references: Android documents descriptor duplication and detach/close
ownership in [ParcelFileDescriptor](https://developer.android.com/reference/android/os/ParcelFileDescriptor),
and stopped visibility/configuration-change reporting in
[Activity](https://developer.android.com/reference/android/app/Activity).
Debug native Full initialization exceeds the previous 10-second host timeout;
JNI and the real Full worker fixture allow 60 seconds; data-free worker waits
remain 10 seconds and Android storage waits allow 30 seconds.
These are bounded test waits, not performance guarantees or altered UI behavior.


Full-suite verification also reproduced the existing notices dialog stall. Temporary
diagnostics found its tagged text still Loading after 10 seconds, and tracing
showed the producer could resume on an I/O thread with the UI test effect context.
Explicitly publish loaded notices on Dispatchers.Main.immediate after the off-main
asset read. Preserve the original full-content/recreation assertions and timeout;
remove diagnostic logging. This is a platform state-publication fix, without new
product behavior. Android's [Compose synchronization guidance](https://developer.android.com/develop/ui/compose/testing/synchronization)
explains the separate test clock and external asynchronous work; its
[effect guidance](https://developer.android.com/develop/ui/compose/side-effects#producestate)
describes composition-scoped producers.


## 2026-10-24 — Durable time attribution and completion handoff

Keep pending durations at the exact chapter-local UTF-8 reading offset where the
accumulator began, scoped to the live document. Checkpoint before moving position;
backward navigation retains the departing offset's time without history. Reflow
keeps all offsets/durations intact and assigns each to its containing logical page.
Do not split durations proportionally or key them by mutable page numbers/presets.
A page boundary belongs to the following page; EOF belongs to the final page, as
in pagination. Restore only the destination offset's duration; freeze the sum of
all offsets belonging to the departing page when constructing completion.

Use migration 0009 and existing rusqlite APIs/dependencies. A loaded opaque
SavedReadingTime token reserves a zero row and carries its duration/revision.
Absolute nondecreasing checkpoints validate the current position, increment the
revision and permit an exact immediate retry. Older/conflicting saves fail instead
of overwriting time. Retain zero rows after consumption; advance by two revisions
to distinguish completion from even a zero-duration checkpoint's expected retry.
This prevents stale checkpoints from resurrecting consumed time on reread.

Extend the frozen PageCompletion with PendingReadingTime. In the same immediate
transaction validate its durable total, insert history.active_time_ms, consume only
the departing page's buckets and persist destination position. Duplicate completion
IDs compare duration too, then return without draining later time or moving progress.
Migration gives historical repository-only events zero time. Deletion cascades
pending buckets but retains completion time/provenance; no deletion API is added.

This is the repository slice of PRODUCT_SPEC.md §§3.1/4: partial pages create no
history or status changes, and only completed pages contribute time. The pure core
still owns no clock or SQL. App/worker/FFI sequencing, periodic checkpoints, Android
monotonic lifecycle/interactions and actual close/reboot verification remain open.
Before exposing forward controls, add occurrences and New→Known to this transaction.
No new dependency version, platform behavior or architecture change.

## 2026-10-24 — Analyzed occurrences and promotions in atomic completion

Add migration 0010 and `record_analyzed_page_completion`, freezing an existing
immutable AnalysisId with PageCompletion's command ID/page/timestamp/duration.
Use the existing immediate transaction for analysis validation, every Japanese
occurrence, absent-surface registration, New-only promotion/status event, pending
time consumption and position. The pure core selects occurrences and owns the
New→Known policy; SQLite and analyzer loading stay outside core.

Assign each entire token to the logical page containing its starting byte offset.
This preserves exact anchors and counts a boundary-crossing word once; a token
starting at a boundary belongs to the following page. Persist original analysis
indices (including non-Japanese gaps), surfaces, readings and nullable dictionary
forms. No normalization, text slicing into new words or segmentation-rule precedence
is introduced. Effective override resolution remains its separate unchecked task.

Keep a historical completion_analyses header without a live-analysis FK, preserving
engine/dictionary/mode identity after document deletion. Its presence distinguishes
analyzed completions, including non-Japanese pages, from legacy repository events
with absent encounter data. Do not retrofit legacy rows with inferred statistics.
Replay validates both payload and analysis identity before live-content lookup;
neither legacy→analyzed nor analyzed→legacy substitution is allowed under one ID.
Recorded reports newly_known; AlreadyRecorded carries no repeated toast or effects.

Extend the existing status timeline with cause completion and a required command
FK for New→Known only. Rebuild the constrained table while preserving rows, IDs,
indexes and deleted-event allocation high-water marks. Vocabulary deletion cascades
its occurrence/status rows; document deletion retains all historical provenance.
On actual later page completion, an absent vocabulary surface registers New then
Known in this transaction. Only departing-page surfaces participate; no existing
document or other page is rescanned merely because vocabulary was deleted.

No new dependency or product deletion API. App/worker/FFI completion/checkpoint
ordering, platform clocks/lifecycle/interactions, controls, Finish Book and
statistics aggregation remain open. Legacy `record_page_completion` remains a
repository foundation, never a production forward action.


## 2026-10-24 — Restore the reader's stored base analysis identity

Load paste content, exact position, global page preset and Mode B AnalysisId in
one read transaction on the existing FIFO worker. Select the earliest persisted
Mode B snapshot for that chapter: this is the original import base today, and
later engine/dictionary snapshots must not silently replace it. Do not select A/C
or another chapter, compare against the currently installed engine, initialize
Sudachi or rescan/register vocabulary while opening. Full token validation stays
inside analyzed completion; opening reads only the stored analysis header.

Represent absent Mode B analysis as ReaderAnalysis::Missing, distinct from a
reader that is not open. Legacy pastes remain readable without inferred analysis
or statistics. Corrupt original headers/settings return the existing typed error,
without falling back to a later snapshot or repairing data. Freeze the selected
identity in core through position saves, settings reflow and unrelated LoadPaste
results; clear it on reader exit/replacement. Expose it in the generated view
protocol for the upcoming completion intents. No migration, dependency, product
feature or selected-override behavior is added. Timer checkpoints, forward intents
and the eventual legacy-completion policy remain unfinished.

## 2026-10-24 — Explicit pending-time checkpoint intents

Move SavedReadingTime's pure anchor/duration/revision value into core's active_time
protocol (reexported by storage), retaining SQLite validation and revision semantics.
Expose RestoreReadingTime separately from OpenReader so opening stays read-only;
the future platform timer requests the exact offset's accumulator when ready.
The view carries only document/position/duration, never a revision or clock epoch.
Negative milliseconds fail Rust Serde decoding; foreign anchors and decreasing
checkpoints are ignored in core before I/O.

Freeze the absolute checkpoint and previous revision when accepted. On failure,
RetryReadingTimeCheckpoint resubmits that same payload; other storage operations
remain blocked until it succeeds. Reader exit clears visible timer state but keeps
this retry obligation. Accepted writes finish on the existing FIFO; late callbacks
cannot reopen/attach time to a closed reader. Same-offset position saves and reflow
preserve the token, while a different offset needs a fresh restore. Checkpoints
create no history/status/encounter events, and no analyzer is loaded.

This is transport/state preparation, not automatic active-time tracking. The next
slice must sequence/coalesce checkpoints while busy, flush before movement/close,
freeze analyzed completion IDs/time across retries, and then connect Android's
monotonic clock/lifecycle/interactions/periodic saves. No dependency, migration,
legacy completion policy or new product behavior is introduced.

## 2026-10-24 — Coalesce checkpoints before reader departure

While an absolute time save is unresolved, retain its exact payload/prior revision
for retry and coalesce subsequent valid same-anchor updates into one greatest-duration
slot. Equal, decreasing and foreign updates do not replace it or create another
write. On success, dispatch the coalesced value with the returned revision before
any accepted reader departure. Updates arriving during that chained save can
coalesce again; failures require the existing explicit retry and retain all state.

Hold one CloseDocument, OpenPaste, tab selection or position-only save until the
checkpoint drain succeeds. Keep the first requested departure; subsequent duplicate
or competing callbacks do not replace its destination. This extends the session's
existing one-command policy with a bounded time slot and one departure, without a
general event queue. Reader/time/analysis stay attached while saving or retrying;
only successful draining permits departure. Same-offset movement preserves the
token, and changed-offset movement still requires a fresh restore.

This is explicit transport sequencing, not automatic timer production. A platform
caller must restore first, submit its final duration before requesting departure,
pause accounting and wait for the drain before destroying the native session.
Other unrelated busy operations retain their existing ignore policy. Native close
still drains work already submitted to FIFO, not unsubmitted coalesced core state.
Android clock/lifecycle/periodic saves/teardown, analyzed completion and its frozen
IDs/timestamp/page total remain open. No schema, dependency or public codec change;
checkpoints still create no completion/encounter/status history.


## 2026-10-24 — Freeze analyzed completion after checkpoint drain

Accept CompleteReaderPage only for the visible document/page/exact position with a
nonnegative caller timestamp. Capture the current preset and original Mode B identity
immediately. Duplicates cannot replace that intent or generate another command;
MissingAnalysis leaves legacy content readable without history or automatic rescan.
An accepted completion waits for all coalesced checkpoints before preparing its total.
The FIFO reserves a command ID and reads the entire departing page's durable buckets
in one immediate transaction; this includes earlier offsets brought together by reflow.
Preparation itself creates no history, consumption, status or position changes.

Core freezes that prepared payload across all transaction retries. It blocks other
storage and holds one first-wins departure until success. Once preparation starts,
further timer updates are ignored: the platform must pause accounting and send its
final checkpoint before requesting completion. A reread gets a new reservation.
Completion returns persisted content for progress reconciliation; if that read fails
after commit, retry the exact command and accept AlreadyRecorded without another
notice or position write. Only newly Recorded positive counts expose a dismissible
command-keyed CompletionNotice; route exit clears it. Counts use u64 in the typed
protocol. Consumed timer tokens retire even at final EOF. A fresh forward at EOF
is ignored; permanent finished state/Finish Book remain a separate step.

No schema or external dependency version changes. The FFI test adds a workspace-only
storage dev-dependency to seed analyzed snapshots without loading a dictionary.
No Android timer, navigation controls, gesture, toast renderer or teardown flush is
claimed by this slice; Android must later await both time and completion obligations.

## 2026-10-24 — Drive the Rust timer with typed monotonic observations

Keep active-time accounting in the existing pure Rust accumulator. Add an anchored
ObserveReaderClock sample (document/exact position, signed monotonic milliseconds,
Resume/Interact/Checkpoint/Pause) to the generated event protocol. Restore remains
explicit and read-only reader opening stays unchanged. The host starts a clock only
after restore and supplies observations; timestamps never enter storage or the view.
Checkpoint/Pause save increased absolute durations. Resume and Interact only update
hidden accounting. Negative/regressing/foreign samples are ignored, and an accepted
departure/completion freezes further clock observations. Settings only accepts Pause;
the host must pause before opening it. Reflow/same-offset writes preserve the clock;
movement/completion/exit retire it. An explicit greater saved duration resets it paused.

Reuse the existing greatest-duration slot when nonmoving storage is already busy.
Drain it on success before completion/departure; retain it for explicit retry after
failure. The save-pending flag covers this waiting slot. A new absolute update may
supersede an unsubmitted slot, but retries of submitted writes retain their exact
payload/revision and any newer queued duration. Reject late producers behind an
already accepted movement/import: saving that old offset after the move is invalid.
This retains the bounded queue and the existing FIFO; no schema/dependency changes.

Android production of monotonic/lifecycle/interaction/periodic/final observations,
restore scheduling and native teardown acknowledgment remain the next slice. Core,
blocked-SQLite FFI and host Kotlin/JNI tests establish transport/accounting only;
actual Android close/reboot recovery remains unverified.

## 2026-10-24 — Retained Android reader clock and draining close

Keep Kotlin responsible for observation scheduling, with all duration/idle/revision
rules in Rust. Use SystemClock.elapsedRealtime; Activity onResume/onPause gates
accounting separately from analyzer start/stop, including recreation. Restore only
when an active reader is ready and the FIFO is free. Start Resume after its callback;
do not count restore latency, settings, background or EOF. Use a 15-second Handler
checkpoint without resetting idle; onUserInteraction plus touch movement/generic
scroll supplies Interact. Typed reader actions submit Pause before their intent.
Reflow preserves the offset; movement restores independently. No dependency changes.

Foreground restore/save failures expose Retry in reader/Settings and pending saves
disable preset changes. onCleared requests close, immediately rejects new dispatch,
then retains callbacks until storage_busy and both pending flags clear. After a
failure with no UI left, retry the core's frozen obligation every second. Persistent
failure keeps it alive while the process survives; never silently drop a queued
duration to release the handle. Abrupt process death can still lose uncommitted time
since the latest checkpoint. No clock timestamp is persisted and no OS guarantee is
added. This uses existing typed effects/FIFO, without a general Kotlin event queue.

Primary Android API references checked before implementation:
- https://developer.android.com/reference/android/os/SystemClock
- https://developer.android.com/reference/android/app/Activity#onUserInteraction()
- https://developer.android.com/guide/components/activities/activity-lifecycle

## 2026-10-24 — Reader controls, swipes and completion toast

Keep forward navigation on CompleteReaderPage and its existing frozen transaction;
both controls and gestures go through CoreSession so Pause precedes the intent.
Add PreviousReaderPage(document, page, exact position), validating the visible
snapshot in pure core before deriving the preceding logical page. Feed that target
into the existing position-only checkpoint-draining path. First page, foreign/stale
callbacks, Settings and unresolved completion ignore Previous; duplicate deliveries
cannot walk backward again. No SQL or pagination rules move into Kotlin.

Place Previous/counter/Next below page content in the existing centered 720dp column.
Use axis-specific horizontal drag recognition alongside vertical scrolling. Accumulate
48dp net horizontal movement and dispatch once on release; cancel/short/vertical
gestures do not navigate. Left completes forward without a bottom requirement;
right only saves position. Pending storage/time/completion disables the actions.
EOF disables further forward actions until finished-book behavior is implemented.

Render the core's positive, command-keyed CompletionNotice with Android's short
text Toast. Consume the notice synchronously in a keyed Compose effect so normal
recomposition/Activity recreation cannot replay it. Zero counts remain silent.
No dependency or schema changes; the generated event codec adds anchored Previous.

Primary API references checked:
- https://developer.android.com/develop/ui/compose/touch-input/pointer-input/drag-composables
- https://developer.android.com/develop/ui/compose/side-effects
- https://developer.android.com/guide/topics/ui/notifiers/toasts

## 2026-10-24 — Durable finish marker before reader finish UI

Store finished-at-least-once as a constrained boolean on the live documents row,
default false. The existing completion transaction sets it when a validated page
ends at the single-chapter paste's exact EOF. Do not derive it from current position
or reset it on backward movement/reflow. A fresh final-page reread still records
history; the permanent boolean stays true. Deletion removes the live marker and
reimport starts false while immutable history survives. No new destructive API.

Migration 0011 backfills only matching live paste/source/chapter EOF completion
history, using UTF-8 BLOB length (including text after NUL). Position-only EOF is
insufficient. Keep this slice storage-only with an Option<bool> getter; snapshots,
Library checkmark, reopen-at-page-1, Finish Book and confetti remain next. Future
multi-chapter imports must explicitly validate the final chapter before marking a
document finished. No dependencies or public event/view codec changes.

Track SQL files alongside Rust sources in both Gradle native build tasks so a
migration-only edit invalidates packaged native output.

## 2026-10-24 — Finish checkmarks and explicit finished-book reopening

Carry the permanent boolean in PastedDocument, LibraryDocument and ReaderPage so
imports, snapshots, position saves and completion reloads preserve the same state
through generated Kotlin/JNI. Render a labeled vector checkmark in Library and
reader, independent of the current percentage. No schema/dependency change.

Separate StorageOperation::OpenReader from LoadPaste. Its repository method uses
one immediate transaction to read a consistent document/settings/Mode B snapshot
and persist chapter-start position for finished books. Unfinished books keep their
exact offset, including position-only EOF. Reset neither the finish marker nor
pending-time buckets/history. Avoid a redundant position write if already at start.
Failures roll back and can be retried by reopening; a navigation cancellation does
not cancel accepted I/O. Core updates the existing Library summary after callback.

Keep load_paste/load_reader_snapshot read-only, especially the worker's completion
reload: final-page completion must leave the current reader at EOF. ReaderPage::open
only projects the supplied exact position; it cannot infer a reopen reset. Retained
Activity recreation likewise preserves the current position. Historical migration
fixtures now seed old position/time/history rows directly because current snapshot
queries require migration 0011. Finish Book and confetti remain a separate slice.

The paste-reader regression test scrolls the Library container by document key
before asserting a row, since offscreen lazy children may not be composed.
API verified against the primary [Compose test reference](https://developer.android.com/reference/kotlin/androidx/compose/ui/test/SemanticsNodeInteraction).

## 2026-10-24 — Finish Book and command-keyed confetti

Label the final page's forward control Finish Book; a left swipe takes the same
completion path. Keep the existing anchored intent, Pause/checkpoint drain,
frozen command and atomic SQLite transaction. Failure keeps the reader and Retry;
success reconciles the Library at EOF and returns there, retiring the clock.
Determine finality from the frozen page's source/chapter/end against immutable
paste content, never from the permanent flag (already true on rereads). Future
multi-chapter imports must extend this final-document test.

Expose book_completion_notice: Option<CompletionCommandId> only for newly Recorded
final-page results, including zero newly Known. AlreadyRecorded remains silent.
Keep the positive-only vocabulary toast independent and preserve it on the automatic
Library return. A keyed acknowledgment ignores stale command IDs. Compose consumes
before starting a local 1.4-second Canvas/Animatable confetti overlay; ordinary
recomposition does not restart it, recreation discards the animation and cannot
replay a consumed command. No schema or dependency changes.

Primary Compose APIs checked:
- https://developer.android.com/develop/ui/compose/animation/value-based
- https://developer.android.com/develop/ui/compose/graphics/draw/overview
- https://developer.android.com/develop/ui/compose/side-effects

## 2026-10-24 — Read-only reader tokens before word-tap transport

Implement a pure page projection and a separate SQLite read API as the next
small phase-3 prerequisite. Reuse the original stored Mode B selection and full
analysis validation. Read live text, analysis and current statuses under one
deferred transaction, with one exact-surface status join for visible Japanese
tokens. Preserve explicit Missing and absent vocabulary; lookup must not rescan,
register, reset finished-book progress or create history. Base-only output remains
consistent with current completion; override resolution remains its own task.

Expose full token index/anchor/surface and analyzer reading/dictionary form, plus
a page-local UTF-8 display range clipped to the page. A token crossing a page
boundary is selectable from either visible fragment with the same full identity;
this leaves start-based completion ownership unchanged. Keep byte offsets explicit;
the future Compose layer must convert validated page-local boundaries to UTF-16.
Echo document/page/analysis identity for later stale-result rejection. Five-status
values gain Serde/Facet support, but no event/view codec or Kotlin payload changes.

Do not couple page projection to persisted position/preset: the supplied valid,
nonempty source span can be the currently displayed reflowed page. Validate source
ownership and scalar boundaries even when analysis is Missing. Missing documents
return None. Current implementation loads/validates the whole stored chapter per
request before the visible-page join; optimize only with measured evidence, retaining
the immutable-validation/read-snapshot guarantees. No schema/dependency changes.
Crux effects/FIFO/FFI/Compose taps and manual status events remain the next slice.

## 2026-10-24 — Explicit typed token reads through the existing worker

Add ReaderTokenKey(document/page/original analysis) to the pure protocol and an
explicit LoadReaderTokens event/storage operation. Reuse the session's single
storage_busy gate and FIFO; do not add a second connection or eager analyzer work.
Reject requests outside the current reader key or while Settings is open. Return
the repository's unchanged snapshot and require pending/result/current keys to
match before exposing transient reader_tokens in the view. Missing analysis is
a successful empty snapshot, not a trigger for analysis. Accepted I/O can finish
after navigation, but its result or error cannot attach to a departed reader.

Clear cached tokens on explicit refresh, navigation, position/completion results,
and reflow to a different page span. Treat a read as nonmoving so a final time
checkpoint can queue behind it. A failed read has no durable retry obligation:
drain a waiting checkpoint even on failure, preserving the existing deferred-exit
ordering. Lookup errors retry by requesting the current key again. No schema or
dependency changes. This independently testable prerequisite deliberately leaves
manual status events and Compose taps for the next implementation slice.

## 2026-10-24 — Manual reader status choices reuse the atomic repository

Use an explicit ReaderStatusChange carrying page/base identity, visible existing
VocabularyId, target status and timestamp. Validate against the loaded token
snapshot outside Settings and reuse the single storage gate/FIFO. Return only the
committed optional ID/status plus echoed choice: the existing repository commits
global status/history atomically and needs no follow-up read. Update every visible
matching ID after success; absent/deleted IDs clear references without registration.
Do not optimistically change the snapshot or add reading history/toasts. Existing
same-status no-op semantics make an explicit repeat choice safe. A failed atomic
choice has no frozen automatic-retry obligation; preserve the snapshot so the user
can choose again. Accepted writes finish after navigation, but stale callbacks/errors
cannot attach to a departed reader. Treat the write as nonmoving and drain queued
final clock checkpoints on either success or failure. No schema/dependency changes;
Compose token taps/status controls remain a separate independently testable step.

## 2026-10-24 — Compose word interaction over the existing typed snapshot

Request the current key from Compose after existing storage/time/completion gates
clear. Retry token-read errors explicitly, avoiding an automatic failure loop.
Remember an attempted read until success/key replacement; a queued clock save can
clear the shared document error and must not cause automatic replay.
Validate scalar boundaries, echoed identity, ordered/clipped spans and exact
surface text before converting page UTF-8 offsets to JVM UTF-16 indices. A malformed
projection renders plain text. Preserve the full token identity when a page shows
only a fragment. Use inline LinkAnnotation.Clickable with explicit product colors
and no underline, preserving text flow and keyboard/accessibility activation.
Remember annotations across timer-only renders; status changes rebuild them.

Open a transient bottom sheet on phones and a side panel on wide displays, with
surface, stored analyzer reading/dictionary form and one row of five direct status
choices. Selection reflects committed Rust state;
busy/pending saves disable controls. Failures retain the prior value for an explicit
repeat choice; absent vocabulary stays disabled and unregistered. Opening, closing
and recreation never send status commands. Dictionary definitions, names and
encounter counts await their planned offline-data work; show no invented matches.
Use LocalWindowInfo's actual container size for the 600dp breakpoint and bounded
scrollable panel height, following Compose lint's window/insets requirements.

Sheet/popup input uses a separate window and bypasses Activity.onUserInteraction, so
forward its content and container pointer/key interactions to the clock observer. Keep
the clock active while the panel is open, with the same five-minute idle cap.
No dependency, schema or FFI change. Primary Compose APIs checked:

- https://developer.android.com/develop/ui/compose/text/user-interactions
- https://developer.android.com/reference/kotlin/androidx/compose/ui/text/LinkInteractionListener
- https://developer.android.com/develop/ui/compose/components/bottom-sheets
- https://developer.android.com/reference/kotlin/androidx/compose/ui/window/PopupProperties
- https://android.googlesource.com/platform/frameworks/support/+/12a8e82373811d8811921d9bc331653ceb2320ee/compose/ui/ui-text/src/commonMain/kotlin/androidx/compose/ui/text/LinkAnnotation.kt

## 2026-10-25 — Pinned independent offline word/name index

Compile the official standard JMdict English (`JMdict_e.gz`, not basic/NG) and
JMnedict XML snapshots created 2026-10-25 using Python stdlib, without new app or
Rust dependencies. Source SHA-256/counts and notice hashes live in
`third_party/dictionaries/sources.lock.json`. Current EDRDG terms specify CC BY-SA
4.0 for data/derivatives. Retain full legal code, current licence statement,
project documentation, attribution, modifications and update procedure alongside
every index. The app integration must expose these notices in Settings before
shipping data. Code remains MIT. Primary evidence and local copies are linked
from `third_party/dictionaries/README.md`; the old wiki entrypoint now returns a
closure notice, so use the official complete static project page instead.

Choose a separate immutable SQLite asset with source-scoped entry IDs and ordered
JSON payloads, plus an indexed BINARY exact-written/reading-form table. This
reuses the eventual storage boundary without moving SQL into pure core or mixing
dictionary data with mutable user state. Expand upstream entity labels and make
POS inheritance explicit; retain spelling/reading/sense restrictions and all
English glosses/name groups, including overlapping word/name entries. Compiler
candidate inspection deliberately leaves runtime surface/base matching and
restriction filtering for the next slice; no guessed definitions or status writes.

Copy and verify compressed inputs before parsing. Validate format/date/counts,
foreign keys and integrity; stage transactionally and publish a fresh directory
by rename. Never replace published output. Preserve original compressed inputs
for offline reproduction because upstream download URLs roll daily. Default
preparation has no network; --download is explicit and rejects changed bytes.
Record Python/SQLite/compiler versions and artifact hashes; guarantee binary
reproducibility with identical inputs/compiler/toolchain. Two complete builds
match byte-for-byte on Python 3.14.8/SQLite 3.51.2. Result: 279,109,632-byte index;
no Android extraction, asset/APK size or lookup-performance claim. App bundling,
typed Rust lookup, encounter count and panel integration remain separate work.

## 2026-10-25 — Typed read-only exact-form dictionary candidates

Keep dictionary I/O in a separate `yomoku-storage::dictionary::Dictionary`, never
in core or the mutable user database. Open existing paths with explicit read-only
SQLite flags, without URI interpretation, creation or migrations; check index
application/schema identity and bind exact BINARY surface/base queries. The
installer remains responsible for verifying checksums and immutable publication;
this API does not claim to verify asset bytes. No Android asset installation yet.

Return the union of every exact surface and optional Sudachi base candidate,
deduplicated by source/positive ID, keeping the exact vocabulary surface and both
query-form provenances. Separate word/name lists preserve overlapping source IDs.
Retain complete ordered typed payloads and supply applicable indices into them.
Pure core rules pair written forms with all licensed readings, kana forms with all
licensed spelling pairs, and satisfy both sense spelling/reading restrictions.
re_nokanji/kana-only matches have no spelling pair. Do not filter with Sudachi's
single reading: one written spelling intentionally shares all appropriate readings.
Raw candidate payloads are not authorization to display restricted senses. A pair
with no applicable senses is omitted from the display view; source data stays intact.

Deserialize strictly, reject missing/unknown structures, non-English translations,
bad references or mismatched indexed forms, and fail the complete lookup without
partial output or repair. No registration, status, position or encounter writes.
Use existing pinned rusqlite 0.40.2 and add the already locked serde_json 1.0.154 as
a direct storage dependency; no version changes or new packages. Primary API
evidence: locally installed published crate source/docs (`Connection::open_with_flags`,
`pragma_query_value`, `prepare`, bound `query`, `is_readonly`, `serde_json::from_str`)
and the pinned source XML's embedded DTD comments for re_restr/re_nokanji/stagk/stagr.
`prepare_cached` is feature-gated out by the existing rusqlite configuration; use
ordinary preparation rather than enabling features. Compact POS and FIFO/FFI/UI
remain the next integration; no public bridge schema changes in this step.

## 2026-10-25 — Verified Android word/name assets and offline credits

Pin the reproducible index/build manifest separately in `android.lock.json`.
Gradle bundles only explicitly prepared local data, verifies the exact manifest,
compiler and every staged file, then publishes an immutable asset directory.
Retain the original compressed sources, compiler, build/source manifests and all
five legal/project documents in the APK. Android's asset merger was observed to
expand `.gz` inputs and remove their suffix, breaking exact-byte provenance;
package those files as `.gz.bin`, restoring the original name only when recovering
source data. Installed-asset tests hash every file against the original manifest.

Reuse `VerifiedDictionaryFiles` with a separate private no-backup directory and
content-addressed `.sqlite3` filename. Require worker access, verify pinned
size/SHA-256 even for cached bytes, stage/fsync/chmod/rename atomically and preserve
held readers during repair. `verifiedPath()` exposes only verified immutable
read-only publication, for the later Rust FIFO provider. No eager extraction in
Library/Settings; no mutable user schema or FFI change and no dependency addition.

Share the existing offline notice dialog between Sudachi and word/name credits.
Keep exact original notice bytes bundled; render HTML project documents as plain
text using Android `Html.fromHtml`, with no WebView or remote resource loading.
All text is selectable/scrollable and visibility survives Activity recreation.
Primary platform APIs checked against Android AssetManager.open and Html.fromHtml:

- https://developer.android.com/reference/android/content/res/AssetManager
- https://developer.android.com/reference/android/text/Html

## 2026-10-25 — Selected reader word and lifetime count repository

Build the read-only repository composition before FIFO/FFI/panel integration.
Reuse the original Mode B/page projection through `load_reader_tokens_in`, taking
a page/base key and original token index rather than trusting caller-supplied
surface/base/vocabulary values. Resolve the selected full token and live status,
then count completed occurrences for its current exact-surface vocabulary ID in
the same deferred transaction. The existing entry index is covering; no migration
or new dependency is needed. Dictionary lookup uses its separate immutable
read-only connection and preserves all metadata/applicable views/source sections.

Echo key/full token in `ReaderWordDetails` so later asynchronous callers can
discard departed selections. Missing document returns None; stale base or a
nonvisible/non-Japanese index is InvalidSelection. Distinguish dictionary and user
storage errors from empty matches and return no partial details on either failure.
Lookup never loads an analyzer, registers vocabulary, changes status, counts a
partial page or updates position/time. Preserve counts after document deletion;
vocabulary deletion erases them and absent entries remain absent during lookup.
This step adds no public shell schema/UI behavior. Whole-chapter validation remains
the existing correctness-first approach; performance remains unmeasured.

## 2026-10-25 — Lazy selected-word FIFO and typed bridge

Extend the existing trusted DictionaryProvider with a worker-only verified index
path callback (empty string means unavailable); Android uses wordDictionaryFiles
with the existing pinned hash/size publication contract. Share the provider via
Arc with the Full descriptor loader. Keep path handling/SQLite in runtime/storage;
core receives only pure values. Open lazily on LoadReaderWord, reuse the separate
immutable connection, and retire it on dictionary query failure so explicit retry
can reopen repaired publication. No extra worker, migration or dependency.

Move details/candidate/form-match values to core and derive Serde/Facet. The pinned
Kotlin generator rejects two parameterizations of generic Candidate/FormMatch;
use concrete WordCandidate/NameCandidate and corresponding form types, preserving
all upstream metadata and applicable indices. Box only internal StorageResult's
large details variant to keep Event's size bounded; Bincode values are unchanged.

Track selection separately from the pending operation; dismissal/Settings/page
invalidation retires it while the callback still drains final time on success or
failure. Match full token identity except live vocabulary, which refreshes every
visible repetition. Keep word errors separate from timer/save errors and require
explicit retry. Successful manual choices update selected status; deleted entries
clear status/count without registration. Land this independently testable transport
before Compose requests/rendering; existing panel UI behavior remains unchanged.

## 2026-10-25 — Reader dictionary panel integration

Reuse the existing metadata/status phone sheet and wide panel. Keep the tapped
token index local to Compose and request LoadReaderWord once storage/time gates
clear. DismissReaderWord on close/disposal retires the core selection, including
Activity recreation; tolerate only this dismissal after CoreSession teardown starts.
Core invalidation dismisses the local panel even when its page key is unchanged.
Errors remain distinct from successful empty matches and require explicit retry.

Render every applicable JMdict sense immediately, unioning only its licensed
spelling/reading pairs across surface/base matches in source order. Show all
readings and compact common grammatical labels, preserving unknown English labels.
Keep gloss qualifiers, domain/dialect/miscellaneous labels and sense notes. JMnedict
readings/categories/translations get their own section; show the committed lifetime
exact-surface count above both sources. No new dependency, storage schema, lookup
policy or custom-definition behavior is introduced.

## 2026-10-25 — Pure TXT decoding and Aozora canonicalization

Add the planned `yomoku-import` crate before storage/Android file wiring. Pin
`encoding_rs` 0.8.35 for strict Shift-JIS (WHATWG/Windows-31J-compatible) decoding.
Verified the [versioned Encoding API](https://docs.rs/encoding_rs/0.8.35/encoding_rs/struct.Encoding.html)
on 2026-10-25: `decode_without_bom_handling_and_without_replacement` returns
`Option<Cow<str>>` and treats malformed bytes as fatal. UTF-8 uses `str::from_utf8`.
The crate license is `(Apache-2.0 OR MIT) AND BSD-3-Clause`; retain its notices
when it enters distributed Android binaries. It adds only itself to the lockfile;
`cfg-if` was already present. No statistical detector: a UTF-8 BOM resolves
automatic detection; otherwise compare both strict decodes,
select a sole valid encoding or equivalent output, and require explicit selection
when both are valid but disagree. Unsupported UTF-16/32 BOMs and invalid bytes
fail without replacement text; a manual choice stays strict. Identical ASCII
chooses UTF-8. This is validity-based detection of the two supported encodings,
not a guarantee about arbitrary bytes from other encodings.

Follow the [official Aozora annotation list](https://www.aozora.gr.jp/annotation/)
for common explicit/implicit kanji ruby and typographic directives. Preserve ruby
readings separately with canonical UTF-8 ranges; analyze only base text later.
Hide a conservative set of layout/emphasis/heading directives, retaining their
surrounding hard breaks. Preserve unknown/malformed notation and gaiji descriptions
instead of guessing missing characters. Do not infer chapters, normalize Unicode,
rewrite CR/LF, extract title/author from body text or strip publication footers.
One canonical text unit will receive its IDs in the future atomic file import.
File-byte SHA-256, ruby persistence/rendering, filename titles and picker/manual
selection UI remain separate wiring steps; no current paste semantics change.

## 2026-10-25 — Atomic TXT file repository and source migration

Keep pure decoding in yomoku-import and SQLite composition in yomoku-storage.
Use a separate unique original-file SHA-256 domain, retaining the exact source BLOB
and chosen encoding; equal canonical text, BOM/encoding/markup changes and pasted
text remain distinct sources. A duplicate references its first committed canonical
interpretation regardless of later encoding selection, with no rescan, registration,
rename, position reset or finished reset. Filename is the caller-supplied literal
Android display name, including extension; reject blank names for new files.
Original bytes, canonical text/ruby, Mode B and registrations commit together;
source ruby is separate from analysis and awaits typed reader/rendering work.
Reuse current single-text snapshot/completion machinery and PastedDocument's wire
shape instead of a bridge-wide rename; EPUB multi-chapter work remains separate.

Migration 0012 widens sources.kind via the documented SQLite
[table rebuild procedure](https://www.sqlite.org/lang_altertable.html#making_other_kinds_of_table_schema_changes).
Verified the primary documentation on 2026-10-25: foreign_keys must be disabled
outside BEGIN, rebuild through a new table rather than renaming the old parent,
run foreign_key_check before COMMIT and restore enforcement. Preserve sources'
AUTOINCREMENT high-water mark, including deleted IDs and the absent sequence in a
fresh database. Migration failure restores original schema/data/history; normal
connections retain FK enforcement. No writable_schema shortcut, new package/version,
platform I/O in core, production deletion or Android import behavior is introduced.

## 2026-10-25 — Typed per-file TXT worker imports

Add `TxtImport` (original bytes, literal display name, optional UTF-8/Shift-JIS)
to the existing generated Crux event/effect protocol. Append event/error variants
to retain current discriminants. Map strict parser errors to explicit typed
DocumentError variants, retaining the bridge's simple enum shape. Reuse committed
Created/Existing and Library summaries; no automatic reader open or batch policy.
Core keeps existing busy/pending-save gates and invalidates word selection; Android
pauses the reader clock before TXT dispatch like paste.

Keep Full publication/loading outside SQLite's write transaction. A shared read-only
original-byte/digest lookup skips decoding and analyzer loading for live duplicates,
including hidden/fresh workers or an unavailable provider. Validate new-file name
and parsing before loading Full, then let the repository repeat its authoritative
lookup/parse and atomic import. This deliberately duplicates new-file parsing
rather than caching provisional IDs or loading assets inside a write transaction;
large-file performance remains unmeasured. No new dependency/version or migration.
Existing worker cache/failure/background/close policies apply unchanged. Bundle
encoding_rs 0.8.35's complete upstream copyright and three license texts under
Android `assets/licenses/encoding_rs/` now that TXT is reachable via production JNI.
The Android picker, batch progress/cancel/background notifications, EPUB/SRT and
source-ruby reader transport/rendering remain independent plan tasks.

## 2026-10-25 — Android multi-TXT picker and encoding handoff

Use the existing AndroidX `OpenMultipleDocuments` contract with `text/plain`,
query `OpenableColumns.DISPLAY_NAME` and read original bytes through
`ContentResolver.openInputStream` on one platform executor. Follow the
[Android Storage Access Framework guide](https://developer.android.com/training/data-storage/shared/documents-files)
and [AndroidX contract source](https://android.googlesource.com/platform/frameworks/support/+/7d9e2f29cffbcd522c7b0ce4d34788cf3d73213e/activity/activity/src/main/java/androidx/activity/result/contract/ActivityResultContracts.kt),
reviewed this session. No storage permission, persistent URI grant, package/version
or migration is needed. This slice accepts case-insensitive `.txt` display names;
other extensions get a per-file result until their parsers are connected.

Retain only URI sequencing, current bytes/encoding prompt and batch result copies
in CoreSessionOwner's platform helper. The core remains authoritative for busy/save
gates, atomic per-file commits, duplicates, errors and Library routes. Observe its
render outcomes via a main-thread callback, posting the next action to avoid
recursive core updates. Read one file at a time; never keep the whole batch's bytes.
Uncertain encoding offers UTF-8, Shift-JIS or Skip file. A selected encoding's strict
error keeps the same bytes available for explicit selection. Failed/skipped files
do not abort later files. Show a scrollable per-file summary in Library; consume
intermediate single-file summaries and clear their transient errors afterward.
Duplicate rows name the existing document title as well as the selected filename.

Use a modal import/encoding dialog for this bounded slice; retain it and results
through recreation without retaining an Activity. Queue/bytes/results are not
persisted or automatically resumed after process death. Final owner teardown drops
unsubmitted work and stops URI reads; CoreSession still drains any accepted atomic
Rust import. Progress counts, cancellation with current-transaction rollback,
background service/notifications, EPUB/SRT and reader ruby remain separate tasks.
Existing in-process background worker behavior supplies no process-survival guarantee.
Large-file heap use and provider-read latency are unmeasured.

## 2026-10-25 — File-count TXT batch progress

Keep the selected-file total in the existing retained Android helper and derive
File X of N from terminal results plus the current file. The determinate bar uses
processed files / total selected files: Created, Existing, errors and Skip each
advance once; encoding prompts and retries do not advance. This measures batch
outcomes, not elapsed time, bytes or analyzer progress. Clear counts on summary
dismissal and reset for each accepted batch. Publish the queried display name on
main before reading original bytes, with the URI's last segment as a temporary
fallback. No I/O, transaction, permission or process-survival semantics change.

Use the existing pinned Material3 determinate `LinearProgressIndicator` lambda
API, checked against the [official Compose progress guide](https://developer.android.com/develop/ui/compose/components/progress).
No dependency/version adoption. A test-only gated pipe provider verifies the name
and zero progress while a real ContentResolver read blocks, including recreation;
the mixed-outcome nine-file fixture verifies counted outcomes and retained choices.
Transaction-aware cancellation, permitted background notifications and actual
process-death acceptance remain separate unchecked work.

## 2026-10-25 — Cooperative TXT repository cancellation

Add cancellation first at the atomic repository boundary. Share a fresh
`ImportCancellation` per file attempt; do not change core events, JNI or existing
noncancellable runtime behavior in this storage slice. A cancelled result is
`Ok(None)`, keeping cancellation separate from parser/analyzer/storage errors and
committed Created/Existing outcomes. Check after analysis before storing its tokens
and registrations; retain the common validated analysis-persistence helper for
paste/TXT. An active decoder/analyzer/SQLite call is not forcibly interrupted.

Serialize the final cancellation check and COMMIT with a per-file mutex. An unlocked
check followed by COMMIT leaves a race that could publish after cancellation returns.
If cancellation wins, drop the transaction and all provisional rows/sequence updates;
if COMMIT wins and succeeds, retain that completed file. Commit failure still rolls
back and releases the gate. The future runtime control path must share the handle
before submission, bypass the import FIFO, and invoke cancellation off Android's
main thread because it can wait for an in-flight commit. No connection, SQL, thread
or cancellation token enters the pure core. No dependency/version or migration.

Host tests use channels rather than timing sleeps to gate analysis and commit,
verify earlier statuses/content after reopen and assert sequence rollback. Android
Cancel/remaining-URI disposal/summary, native lifecycle races, permitted background
notifications and process-death acceptance remain unchecked follow-up work.

## 2026-10-25 — Direct runtime/JNI TXT cancellation

Publish a fresh per-attempt repository cancellation handle synchronously in storage
middleware before enqueueing an accepted TXT effect. Core's existing single-busy
gate prevents rejected duplicate callbacks from replacing it. Keep only a weak
control reference in AnalyzerLifecycle and expose `NativeBridge.cancelTxtImport()`
as a direct call outside FIFO. Clone the handle under a short registry mutex, then
release that mutex before entering its potentially blocking commit gate. Clear
only the matching attempt before resolving its outcome, so stale handles cannot
cancel a later file or hold up callbacks. Accepted work draining after router drop
still owns its control until the worker exits; the lifecycle handle does not keep
the worker or listener alive.

Use a distinct internal `StorageResult::ImportCancelled` for rollback: clear busy
without an error, success summary, reader movement or fabricated document. Preserve
Created/Existing when a file completed and typed failures on actual errors. Check
queued/preflight work before Full and use repository checkpoints around analysis
and COMMIT. An already running dictionary load is cooperative too; it may finish,
but cancellation prevents its later analysis/transaction from starting. No new
event, dependency, schema, durable queue or forced thread interruption.

The Android batch helper must stop submission immediately on Cancel, call this
method on a separate executor (never main or the blocked URI-reader executor),
and await both cancellation return and the current core outcome before settling
the summary or starting another batch. Preserve a successful commit even when
Cancel was pressed, discard unsubmitted URIs and guard late provider callbacks
with a batch/read generation. Android control wiring, notifications and actual
process-death acceptance remain subsequent slices.

## 2026-10-25 — Android TXT batch cancellation and native lifetime

Keep Cancel sequencing in the retained `TxtImports` helper. Clear the remaining
URI queue and current encoding input immediately; invalidate provider callbacks
with a volatile read generation on Cancel and close. Retire the URI executor on
Cancel so a provider ignoring thread interruption cannot hold up a fresh batch.
The old provider may still return, but its name/error/bytes cannot mutate the new
batch or reach Rust. No URI grants, queue, progress or cancellation are persisted.

Run direct native cancellation on a separate CoreSession-owned executor. While
submitted work is outstanding, keep the modal Cancelling state until BOTH native
return and current-file result arrive, in either order. Disable repeated Cancel
and encoding actions; reject another batch until settlement. Preserve a winning
Created/Existing commit and every earlier terminal result; cancelled partial and
unsubmitted files add no result. Refresh Library after settlement, without opening
a reader or manufacturing a storage error. Processed progress remains completed
outcomes / original selection count, so cancellation need not reach 100%.

Include queued/running cancellation in CoreSession's draining-close obligations:
a storage callback can arrive before its direct cancellation call returns. Keep
the native handle alive until both finish, then shut down the cancellation
executor. A closed helper ignores the return and does not dispatch another event.
No Rust production, wire, schema, permission or dependency change. Notifications
and actual process-death acceptance remain separate work.

The host runtime fixture must observe its factory's channel disconnection before
asserting a retained weak lifecycle handle reports closure. Receiving the last
result can precede the callback returning and dropping its middleware references;
an immediate closure assertion was a test race. Use the existing bounded drain
signal rather than changing production teardown or adding a sleep.

## 2026-10-25 — Permitted TXT background execution and notifications

Use the platform `dataSync` foreground service type for user-initiated imports/
local file processing, verified against Android's [service types documentation](https://developer.android.com/develop/background-work/services/fgs/service-types#data-sync)
and [foreground launch rules](https://developer.android.com/develop/background-work/services/fgs/launch).
Use existing Android APIs/minSdk 29/targetSdk 36; add no library/version. Declare
FOREGROUND_SERVICE, FOREGROUND_SERVICE_DATA_SYNC and POST_NOTIFICATIONS. Per the
[notification permission documentation](https://developer.android.com/develop/ui/compose/notifications/notification-permission),
notification denial does not prevent foreground service startup. Offer the runtime
permission only on explicit Enable import notifications in the batch modal; do not
block selection/import or repeatedly prompt automatically.

Keep all work in the existing retained helper/FIFO. A local binder carries only
current filename/number and processed/selected counts to the nonexported service;
its interrupt callback uses the existing Cancel path. Service intents carry no
work/URIs. Start only for user-started batches or explicit encoding decisions,
return START_NOT_STICKY, stop with the task and store no queue or resume token.
Release the service while waiting for encoding and use a separate input-needed
notification, so waiting for the user consumes no foreground execution budget.
Service rejection falls back to ordinary OS process limits and permitted progress
notifications, with no automatic restart. The UI explains possible process loss
and retained completed files without promising background survival.

Progress is terminal outcomes / original selection size; do not call errors,
duplicates or skips newly imported documents. Completion/cancellation notifications
retain these counts and use a separate ID so foreground teardown cannot remove
them. Input-needed notifications also need their own ID because service removal
can complete asynchronously. Tapping reuses the existing Activity without opening
a reader; in-process summary remains in the helper. No summary is reconstructed
after process death.

Implement [Service.onTimeout](https://developer.android.com/develop/background-work/services/fgs/timeout)
by stopping foreground execution/service immediately, then requesting cooperative
cancellation. Never wait for native/SQLite arbitration before satisfying Android's
deadline. Existing native lifetime/commit rules preserve a winning commit. Owner
teardown drops unsubmitted work and removes progress while accepted native work
drains. Do not add a boot receiver, WorkManager queue or service redelivery.

The debug-only DUMP-protected recovery Activity uses an isolated SQLite database
and the test APK's gated provider. The host runner sends HOME then same-UID SIGKILL
with one committed, one blocked and one remaining file; fresh-process checks require
the committed document and an empty helper/no service. Exercise both notification
permission states and restore the previous grant. This demonstrates abrupt process
death, without claiming low-memory-killer, device/release or cache-flush coverage.

## 2026-10-25 — Pure SRT extraction before file-import adapters

Keep SRT extraction in `yomoku-import`, taking decoded `&str` and producing one
canonical text unit plus ordered nonempty UTF-8 cue ranges. SRT byte decoding/
encoding policy and original-file persistence belong to its future import adapter;
do not send SRT through TXT's Aozora parser or change any wire/schema. Callers must
validate supported file extensions; this is not WebVTT/ASS support. No dependency.

Normalize CRLF/CR to LF and remove only a leading BOM. Preserve dialogue whitespace,
speakers and line breaks inside cues; join cues with one blank line outside cue
ranges. Blank lines or recognized time ranges delimit cues, including absent blank
separators/counters. Remove an ASCII numeric counter only immediately before a
recognized time range, preserving numeric dialogue. Recognize two/three-field
colon timestamps with optional comma/dot fractions and trailing positioning fields;
do not require chronological/range-valid times or parse durations. Unrecognized
damaged metadata stays visible rather than consuming possible dialogue.

Strip common HTML subtitle formatting with a nonrecursive scan, mapping `<br>` to
a hard break. Preserve unknown/speaker tags, malformed brackets and brace notation.
Decode basic named/numeric entities once after stripping, retaining unknown, invalid
and control references; escaped dialogue cannot become a stripped tag. Empty cues
are omitted; an entirely empty result is an error. Cues are paragraphs, never
chapters, and their ranges precede analysis/pagination. All five existing page
presets reconstruct the canonical text exactly in fixtures.

## 2026-10-25 — Pure EPUB extraction and bounded XML preflight

Keep EPUB extraction in `yomoku-import`, accepting original borrowed bytes and
returning canonical chapters/format ranges plus immutable referenced image bytes.
No ZIP filesystem extraction, network, SQL, analyzer, durable IDs or Android change.
Use the first declared OPF rendition and spine order, including auxiliary items
so their text is reachable without internal-link/TOC navigation. Empty text items
are omitted. Headings never split chapters. Filename title fallback remains at
the adapter boundary. See DATA_MODEL.md for whitespace/ruby/image range rules.

Pin `zip` 8.6.0 with default features off and only deflate-flate2, plus `flate2`
1.1.9 with its pure Rust backend; no native compression, crypto or extra codecs.
Pin `roxmltree` 0.21.1 for namespace-aware immutable XML trees and `xmlparser`
0.13.6 for a nonrecursive lexical preflight. Verified the published
[ZIP read/metadata API](https://docs.rs/zip/8.6.0/zip/read/struct.ZipFile.html),
[compression features](https://docs.rs/zip/latest/zip/),
[XML parsing options](https://docs.rs/roxmltree/0.21.1/roxmltree/struct.ParsingOptions.html)
and [token ends](https://docs.rs/xmlparser/0.13.6/xmlparser/enum.ElementEnd.html),
then checked the exact downloaded sources/Cargo feature graph before use. These
are MIT / MIT-or-Apache dependencies, locked with nine newly required packages.
API choice follows the [EPUB package/spine specification](https://www.w3.org/TR/epub-33/#sec-spine-elem).
Both Android targets check. Retain complete notices when making EPUB reachable
through the production import adapter, as done for encoding_rs in the TXT slice.

Ordinary XHTML whitespace uses a deferred separator: later text appends never
trim or insert before existing range anchors. Source ruby readings/alt text stay
outside canonical analyzed text. U+FFFC marks each inline image's exact position
and ensures image-only chapters can paginate; image dimensions/ratio/decoding
remain with the renderer. Preserve source bytes once per referenced image/cover.
HTTP(S) website links are external; retain all other targets as inert spans.
Ignore CSS even when its unused manifest URL is remote; never fetch it.

Initial adversarial nesting exposed roxmltree tokenizer recursion before a
post-parse depth check could run (500 nested spans overflowed the host test stack).
Preflight tokens before tree parsing, cap element depth at 64 even if callers ask
for more, and reject entity declarations that could inject nesting after preflight.
External DOCTYPE declarations without resolved entities are accepted without a
resolver. Validate numeric XML references explicitly: roxmltree replaces an
invalid surrogate reference with U+FFFD rather than failing. Final fixtures test
10,000 nested spans fail cleanly, near-limit valid content/CDATA/comments, invalid
references and external entities. Source/read/entry/node budgets are configurable;
these are not measured device memory guarantees. Unsupported remote images,
unresolved named/custom entities and spine fallback chains currently fail clearly.

Reject duplicate raw ZIP names before trusting zip-rs's last-wins name index:
scan its central-directory location for raw entry names, then normalize/validate
each indexed path and reject aliases. All resource reads stay by validated index
with CRC/size checks and per-entry/cumulative expansion bounds. Encryption metadata
blocks only resources actually used; unused obfuscated fonts do not reject a book.
Strict UTF-8/BOM or declared UTF-16 decoding never replaces malformed source bytes.
No migration or wire/schema change. Repository/runtime and UI adapters remain open.


## 2026-10-25 — Atomic SRT repository before worker/picker expansion

Reuse TXT's strict UTF-8/Shift-JIS decoder directly for subtitle files; expose
`txt::decode` without invoking Aozora canonicalization. Conservative detection,
manual selection and unsupported-BOM/error semantics match TXT. Keep decoder
errors typed as `SrtImportError::Decode(TxtError)` at this internal repository
boundary; a future runtime adapter maps them to format-appropriate wire outcomes.
No new dependency, encoding guess, lossy fallback or Android behavior is added.

Use the existing unique original-byte SHA-256 index across TXT/SRT. Identical bytes
selected under another filename/extension reference the first committed live
interpretation, without reparsing, reanalysis or changes to statuses/progress.
Factor the read-only lookup into `find_imported_file`; keep `find_imported_txt` as
a delegating compatibility entry point. Paste stays in its separate identity table.

Migration 0013 follows the existing source-table rebuild, widens source kinds to
SRT and adds ordered canonical cue ranges with chapter-text deletion cascade.
Source loading checks full cue coverage/separators/scalar boundaries and original
byte digest without re-decoding. Import uses the same immediate transaction and
shared per-file cancellation/COMMIT gate as TXT; all analysis/registration/cue rows
and provisional IDs roll back together. One canonical chapter fits the existing
reader repositories; no subtitle timestamps are used for reading time. Worker,
FFI, picker/summary and EPUB persistence remain separate continuation steps.


## 2026-10-25 — Typed SRT worker and shared file cancellation

Add `SrtImport`/`ImportSrt` alongside TXT, preserving existing enum variant order.
Map shared decoder failures to SRT-specific wire errors; keep empty canonical cues
separate from encoding failures. Preflight original-file identity before checking
new metadata/decoding or loading Full, so even a different extension/encoding/blank
new name cannot reinterpret an existing live file. Decode/parse new input before
loading Full and drop preflight text before analysis; the repository repeats its
authoritative lookup/parse inside the transaction. Reuse the FIFO analyzer/cache,
ready-worker callback contract and Library summaries without encounters/time.

Generalize the one accepted import's cancellation registry for TXT/SRT. Publish a
fresh handle before enqueue; clear that exact handle before callbacks. Expose
`cancel_file_import`/`cancelFileImport()` outside FIFO, retaining the existing TXT
method as a delegating compatibility entry point for the current Android helper.
Call off main and await both return/outcome before another file. No new dependency,
schema, batch behavior or picker capability. Android dispatch handles reader-clock
pause and typed error resources now; mixed-format selection remains separate.

## 2026-10-26 — Atomic EPUB repository before multi-chapter transport

Keep the bounded pure parser as the only EPUB canonicalizer. Save its ordered
spine chapters without flattening them, retaining metadata title/filename fallback,
source ruby, overlapping formatting/link ranges, image placeholders/alt and original
image bytes. Reuse source/chapter IDs and source_ruby; asset identity is the live
DocumentId plus normalized container path, shared by cover/repeated placements.
Normalized SQLite rows retain parser output without serializing its Rust layout.
Saved counts/digests detect missing chapters/annotations/assets and damaged content
on read; read-only loading never reparses the ZIP or repairs stored content.

Migration 0014 follows the established source-table rebuild and widens the existing
original-file encoding discriminator with an EPUB container marker. Preserve old
file hashes/bytes/encodings, IDs, foreign keys and source high-water marks. One
immediate transaction creates all chapters/assets, each original validated Mode B
and unique registrations. Factor analyzed-chapter registration out of the existing
single-text composition rather than manufacture single-text EPUB snapshots. Reuse
the cooperative per-file cancellation/COMMIT gate. Later-chapter failures discard
earlier provisional chapters/registrations/IDs; committed prior files stay intact.

Add a format-independent file-ID preflight and return Existing(DocumentId) from
the EPUB repository. This references an earlier TXT/SRT interpretation without
decoding/analysis or a misleading single-chapter payload. The historical single-text
preflight/results and reader APIs still reject EPUB. Generalize their transport,
reader completion/time/tokens and whole-book finish semantics before connecting
EPUB FIFO/JNI or Android selection. Library percentage now sums all canonical
chapter bytes with preceding-chapter offsets and rejects incomplete/foreign chapters.

No new production dependency/version or wire/UI behavior. Shared test archives
reuse the already pinned zip 8.6.0 as a dev dependency in storage/analysis; no new
lockfile packages. No permission, queue, rendering or data-destructive semantic change.


## 2026-10-26 — Whole-document reader repositories before bridge migration

Introduce pure `ReaderDocument` with ordered canonical chapters, exact saved
position and live title/finish state. `DocumentReaderSnapshot` pairs it with the
persisted preset and explicit chapter-keyed original Mode B identities. Keep the
historical `PastedDocument`/`ReaderSnapshot` APIs and wire payloads intact until
application, FIFO and Kotlin adapters can migrate together. The generic snapshot
is a text projection; source ruby/formatting/images remain in EPUB persistence
for the later reader rendering slice.

Reuse the authoritative EPUB loader inside the generic transaction rather than
create another partial or less strict content loader. It validates all saved
source/assets/annotations for every reader operation. Large-book performance is
unmeasured; optimize only with evidence while retaining integrity checks. Single-
text documents must have exactly one chapter; corrupt extras never truncate.

Generalize the existing time/completion/token/selected-word repositories through
that loader. Chapter-end forward completion goes to the next chapter's start;
only completion of the last logical page of the whole book sets its marker.
Pending durations and analysis identities remain chapter-local; preserve command
replay, New-only promotion, occurrence identity and transaction rollback. Ordinary
loads never reset a finished book. Only explicit open atomically resets to the
first chapter; later analysis validation or COMMIT failure rolls back the reset.

No migration, new dependency/version, data-destructive semantic change, application
capability or publishing step. EPUB/import-reference/application/worker/typed JNI
migration remains the next slice; Android selection stays paste/TXT.


## 2026-10-26 — Format-independent import references before reader transport

Use `LibraryDocument` for TXT/SRT/EPUB Existing outcomes and core Created/Existing
results. Imports return only the ID/title/progress/finish information used by the
Library and summary; reader content is a separate load. This avoids carrying or
truncating a multi-chapter book merely to reference a duplicate. Keep legacy
single-text reader payloads until their application/FIFO migration; EPUB selection
is still disabled. StorageFinished is private/skipped by Serde and Facet, so this
internal protocol change does not change exported event/request/view schemas.

Use shared exact-byte preflight for both TXT/SRT workers. Atomically load/validate
the complete saved document and project its Library reference with the hash lookup,
then repeat the authoritative lookup inside each file import transaction. Return
an existing interpretation before filename/encoding checks, ZIP parsing or Full
loading, preserving renamed titles, later-chapter position and permanent finish
state. Ordinary duplicate reads never reset a finished book. Corrupt saved EPUBs
fail closed without repair/reanalysis; later independent files can still import.
The complete-content integrity loader also checks assets; cost remains unmeasured.

No migration, new production dependency/version, permission or destructive semantic
change. Runtime tests reuse the existing pinned ZIP dev dependency/shared readable
EPUB archive builder; no new lockfile package or download.

## 2026-10-26 — Ordered reader payloads across core, FIFO and typed bridge

Migrate application loads/explicit opens/position saves/completion reloads to the
existing ordered-document repositories. StorageResult.Loaded uses
DocumentReaderSnapshot; position/completion results and ViewModel.loaded_document
use ReaderDocument. Keep the historical LoadPaste event name and single-text
repository APIs for existing callers; the app no longer consumes PastedDocument.
Regenerate Kotlin codecs with the changed view shape in the same build. No database
migration, production dependency/version, parser, permission or destructive change.

Validate exactly one explicit original-analysis state for every owned chapter.
Retain the accepted reader's keyed map across its session, selecting the visible
chapter's identity on movement rather than reusing chapter zero's analysis. Plain
loads cannot replace these frozen identities. Previous traverses whole-document
page order; global counters/progress and exact chapter-local reflow use ReaderDocument.
Only completion of the last chapter's final page returns to Library/celebrates.
Time, vocabulary and completion transactions retain their existing repositories.

Keep reader transport a canonical-text projection: source formatting/ruby/assets
stay in EPUB persistence for the rendering slice. Image-only chapters retain the
parser's U+FFFC placeholder and complete without Japanese occurrences. Do not
special-case them as empty text. Android picker stays paste/TXT until EPUB FIFO
imports and mixed-file behavior are verified. Complete saved-content validation
still reads EPUB assets on every reader operation; performance remains unmeasured.


## 2026-10-26 — EPUB per-file FIFO and typed JNI imports

Add `EpubImport` (original bytes, literal display name) and `ImportEpub` to the
existing pure event/storage protocols. EPUB XML/container encoding is independent
of TXT/SRT's optional text encoding; expose one `InvalidEpub` container error for
invalid, unsupported, empty or over-budget archives. Keep the bounded parser's
specific diagnostics inside the import boundary. No new production dependency,
version, migration, permissions or data-destructive behavior.

Use shared read-only exact-byte preflight before filename/container validation or
Full loading, including duplicates whose first interpretation was TXT/SRT. The
repository repeats lookup/parse under its whole-book transaction. Validate/drop a
bounded parsed projection before loading Full; this follows TXT/SRT preflight and
avoids dictionary extraction for invalid containers. The extra parse has unmeasured
large-book cost. Reuse one worker-owned analyzer across chapters/files; evict after
engine/invalid-analysis failure and release while backgrounded. Return a Library
summary from the committed ordered document, without a second database read or
automatically opening the reader.

Publish EPUB's cancellation attempt synchronously before enqueue, reusing the
existing direct cancel registry and repository commit gate. A later chapter's
failure/cancellation rolls back earlier chapters/assets/registrations/provisional
IDs, while committed earlier files stay. No persisted queue or automatic resume.
Android recognizes the new event for clock pausing and the error for rendering;
the mixed-format picker and source formatting/ruby/images remain separate slices.


## 2026-10-26 — Mixed Android file batches

Generalize the existing retained TXT platform helper to FileImports, keeping one
URI reader, one current-file input and the existing per-file Rust transaction/FIFO.
Use OpenMultipleDocuments with */* so SRT providers with text/plain or generic
binary MIME types remain selectable. Route the literal display-name extension
case-insensitively to EPUB/TXT/SRT; reject other extensions before reading bytes.
MIME does not choose interpretation. No content sniffing or excluded format is added.

TXT and SRT use their existing distinct encoding errors and retain the selected
encoding on retries. EPUB carries bytes/name only and never offers text encoding.
Keep ordered per-file outcomes (Created/Existing/error/Skip) in the shared summary
and continue after each failure. Preserve Library return without reader auto-open.

Reuse the notification observer/foreground lease and generation-based URI disposal
for every format. Call shared native cancelFileImport off main and outside FIFO;
await both the native return and current result so a winning COMMIT is retained.
No persisted queue, restart behavior, dependency, permission, schema or destructive
semantics change. EPUB reader formatting/ruby/images remain a separate slice.

## 2026-10-26 — Reader text formatting and website links

Extend ReaderDocument with keyed chapter formatting and ReaderPage with clipped
page-local spans. Carry headings/emphasis/strong and HTTP(S) website targets from
the existing validated EPUB repository through generated payloads. Keep core pure;
no EPUB parser types, SQL, CSS or platform calls enter it. Validate chapter ownership,
unique formatting keys, scalar ranges, heading levels and website targets. Leave
source ruby and image bytes/references for the next independently testable slices.
No migration, dependency/version or data-destructive semantics change.

Compose uses relative heading sizes/bold and emphasis/strong styles over unchanged
canonical text. Use the existing Compose LinkAnnotation APIs, verified against the
[official Url reference](https://developer.android.com/reference/kotlin/androidx/compose/ui/text/LinkAnnotation.Url).
Website ranges own taps inside their source anchors, keeping vocabulary backgrounds;
dictionary clicks on remaining token fragments keep the complete original token
identity. This resolves overlapping source links and Japanese token click targets.
Nested source website spans become disjoint targets in stored order. Internal/other
source links acquire no navigation or special styling; Japanese words within them
retain ordinary dictionary taps. No TOC/seek behavior is introduced.

Normalize website URI schemes before the external handler; an unavailable browser
does not crash the reader. Invalid source formatting at the display boundary falls
back to canonical text with ordinary valid word interactions. Font/furigana settings,
image transport/rendering, cover cache and shared viewer remain separate tasks.

## 2026-10-26 — Reader image reference transport

Carry source image placements as sparse keyed ReaderChapterImages, each with an
exact canonical U+FFFC range, alt text and ReaderImageAsset path/media type. The
enclosing document ID plus immutable container-relative path identifies an asset;
repeated placements share that identity. Resolve metadata from the authoritative
saved EPUB asset map. Do not copy image BLOBs into core or generated reader state;
bounded off-main asset retrieval/decoding remains the next implementation slice.

Validate chapter ownership/key uniqueness, placeholder/scalar boundaries, ordered
nonoverlap, normalized container keys and consistent media types for shared paths.
Treat paths as opaque keys, never filenames/URLs. Page projection selects whole
placeholders and rebases their UTF-8 ranges without changing canonical text,
pagination, exact positions or vocabulary/completion/time behavior. Plain documents
keep empty image metadata. No migration, dependency/version, permission or data-
destructive semantics change. Rendering/ratio, covers and shared viewer remain open.

## 2026-10-26 — Bounded EPUB asset reads outside reader state

Add a narrow `load_epub_image_asset` repository and native request using the
existing SQLite FIFO. Keep image bytes out of pure core events/ViewModel: a
document ID plus validated opaque path/expected media type identifies one asset;
a per-request typed listener delivers the bytes or a categorized error off main.
No second database connection, parser pass, analyzer load or Render is required.

Read live EPUB ownership/metadata and the selected asset in one read transaction.
SQL gates the BLOB copy at 32 MiB, matching the current import-entry budget; verify
the exact expected media type and saved SHA-256 before delivery. Do not copy or
rehash the full original container or unrelated images for this narrow operation.
Normal reader/document loads retain their full integrity checks. Missing/deleted/
non-EPUB documents or missing keys are distinct from corruption/unavailable I/O;
invalid native identities fail before enqueue. Reads never repair or write data.

Retain each listener only through its single worker response. Accepted requests
drain after native close; a weak worker handle alone cannot keep the FIFO alive.
Future Android callers must offload bounded decoding from the FIFO callback and
discard results for disposed requests. This step adds no production Android image
caller, decoder, cache, rendering, dimensions or viewer. No new dependency,
migration, permission or data-destructive behavior.


## 2026-10-26 — Bounded Android EPUB raster presentation

Use the existing Android BitmapFactory API rather than adding a dependency for
this raster slice. Follow Android's [bounds-first sampled decoding guidance](https://developer.android.com/topic/performance/graphics/load-bitmap)
and [BitmapFactory.Options contract](https://developer.android.com/reference/android/graphics/BitmapFactory.Options):
read dimensions without allocating the bitmap, use power-of-two inSampleSize,
disable density scaling and request ARGB_8888. Bound source edges to 32768 and
source pixels to 100 million before decoding. Bound sampled edges to 2048 and
actual allocated pixels to each asset's share of a 4,194,304-pixel current-page
budget (16 MiB ARGB); check actual allocation afterward and fall back on failure.
This bounds retained pixels without an unbounded cache on image-heavy pages.
Published bitmap reclamation remains GC/RenderThread-owned, so this is not a total
process-heap cap. A single CoreSession-owned executor serializes the complete
read/decode pipeline, including across rapidly disposed pages, to avoid accumulating
encoded BLOBs or concurrent decoders. One-shot FIFO callbacks only submit work.

Share one decoded image per distinct current-page asset. DisposableEffect cancels
queued reads and delivery closures on movement/reflow/settings/recreation; accepted
native reads drain but disposed results never update UI. Explicit Retry reloads the
current page's assets. Session close shuts down the decoder; only unpublished stale
bitmaps recycle explicitly. Do not touch progress, vocabulary, time or core errors.

Split the existing AnnotatedString at strictly validated U+FFFC scalar ranges and
render full-column images between its slices. Existing token closures retain full
identities after clipping. Keep source width/height separate from sampled bitmap
rounding and draw at the source ratio. Pathological tall images shrink within an
8192-physical-pixel frame, preserving ratio and safe Compose constraints. Alt text
remains available on images and on loading/failure placeholders. Static platform
raster decoding is the delivered slice; unsupported codecs, SVGs and corrupt data
show a retryable fallback. SVG support, cover cache and shared viewer stay unchecked;
no dependency/version, schema, permission or destructive-semantics change.


The image/link regression exposed default Compose minimum touch-target expansion:
tapping the adjacent inert `note` after a short `site` link also launched the
browser. Scope a delegated LocalViewConfiguration with zero minimum touch-target
size to reader AnnotatedString Text blocks only, keeping exact source link/word
hit areas and accessibility link semantics. Buttons, status choices and other
controls retain their usual targets. Android documents the [default expanded
click targets](https://developer.android.com/develop/ui/compose/accessibility/api-defaults#minimum-touch-target-sizes).
A real browser-monitor test taps both link and adjacent inert text around an image;
word-span/dictionary regressions cover the narrower inline text hit areas.

## 2026-10-26 — Shared full-screen image viewer

Deliver the independent planned image viewer slice before SVG/cover work. Reuse
the existing Compose foundation/UI APIs without a dependency/version change.
Use a separate full-window [Dialog](https://developer.android.com/develop/ui/compose/components/dialog)
and [detectTransformGestures](https://developer.android.com/develop/ui/compose/touch-input/pointer-input/multi-touch)
for pinch zoom/drag, with accessible Zoom in/Zoom out/Reset/Close controls and safe
drawing insets. Keep images upright. Bound zoom to 1–8 times fit and clamp panning
so no enlarged axis can move beyond its edges; center smaller axes. Use original
source dimensions for the ratio, independently of decoded sampling rounding.

Keep the component source-agnostic: callers supply ImageBitmap/source dimensions,
description and loading/retry callbacks. EPUB uses the already decoded current-page
bitmap; opening/zooming adds no asset read, decode, cache or larger pixel allocation.
Fine detail remains limited to the page's bounded sampled resolution. A future
Nostr caller can use the same viewer without changing reader/core/storage types.

Save selection/zoom/normalized source focus in Compose instance state through
configuration changes, keeping a loading/error modal if assets must reload. Key
state by document/page anchor/placement; close or page change discards it. Do not
persist reader scroll or viewer state to SQLite. Source image taps always open
the viewer, including inside website anchors; neighboring text links keep their
existing behavior. The modal owns system Back and gesture input, preventing pans
from reaching reader completion swipes. Observe its pointer/key/control input for
the existing five-minute reader idle policy because Activity does not see dialog
input. No schema, permission, core/FFI or destructive-semantics change.

## 2026-10-26 — Bounded static SVG decoding

Use the unmodified `com.caverock:androidsvg-aar:1.4` library, pinned after reviewing
the [upstream release/API documentation](https://bigbadaboom.github.io/androidsvg/download.html)
and published 1.4 source archive. It adds a roughly 200 KiB AAR, no transitive
dependencies/native code, and Apache-2.0 attribution/license shipped offline in
Settings. This is a routine renderer dependency within the planned SVG slice;
it adds no new platform, image framework or browser. Provenance is recorded in
`third_party/androidsvg/README.md`.

Select SVG only from the already verified exact media type and keep parsing and
software Canvas rendering on the existing serial decode executor. Reuse page
sharing, disposal/cancellation, Retry, pixel budgets and the viewer. Validate
dimensions/viewBox and render directly into a <=2048-edge bitmap within the
asset's share of the 16 MiB page budget. Preserve fractional source dimensions in
Compose `Size`, independently of bitmap rounding, including viewer fit/zoom/pan.

Preflight untrusted XML before AndroidSVG constructs its tree: <=2 MiB, <=4096
elements, <=64 nested/reference levels and <=32768 expanded nodes, rejecting
duplicate/missing/local-reference cycles and exponential `<use>` expansion.
Reject DTDs/processing instructions and disable internal entities (enabled by
default upstream). Install no external resource resolver. In SVG styles support
color/class rules and inline local paint/clip references; reject resource-valued
stylesheet rules whose owning node cannot be established for the reference graph.
Handle the renderer's permissive unclosed `url(` syntax too.

Do not pass compound `<image>` assets to this renderer: its embedded data-URI
rasters bypass BitmapFactory sampling. Keep these, active/unsupported elements,
invalid/over-budget inputs and rendering failures on the existing alt/Retry
fallback. Compound local/data-URI image support remains an explicit continuation,
alongside cover caching. SVG anchors are inert bitmap pixels; EPUB text links
retain browser behavior, internal links remain inert, and image taps open the
shared viewer. Patterns/masks/markers also retain fallback: inherited paint on
their rendered descendants can recurse through resources outside the explicit
reference graph. A bounded resource renderer must handle those before enabling them.
Pixel bounds are not a total heap/CPU guarantee; temporary Canvas
layers and library structures remain bounded by input/structure rather than a
measured process cap. No core/FFI/schema/permission/destructive-semantics change.

## 2026-10-26 — Metadata-only EPUB cover references

Deliver the independent cover lookup before Android thumbnail caching/display.
Keep the existing core/Library payloads unchanged: a one-shot typed native callback
uses the SQLite FIFO for document-scoped path/media/SHA-256 metadata. Do not load
the complete EPUB snapshot or source/chapter/image BLOBs merely to identify a
cover. Validate live EPUB ownership/import marker, declared asset, reference
syntax, 32-byte digest shape and nonempty <=32 MiB image length in one read
transaction; guard the digest copy in SQL too. No schema/dependency change.

Missing/deleted/non-EPUB documents and books with no declared cover share the
normal no-cover outcome; damaged declared cover metadata is an error, never a
repair. Invalid native IDs fail synchronously before enqueue. Accepted reads
drain after close and release their one-shot listener without Render, dictionaries
or core/history writes. The advertised digest supports a later content-addressed
thumbnail key across recreation and avoids relying on document IDs after database
replacement. It is not content verification: new cached pixels must come through
the existing digest-verified asset read and bounded decoder. Cache implementation,
eviction and Library presentation remain the next slice.

## 2026-10-26 — Library cover thumbnails

Use the existing metadata-only FIFO/JNI lookup plus verified asset pipeline for
lazy Library rows, without expanding core/Library payloads or loading dictionaries.
Resolve live ownership and digest before every new display request, including cache
hits. Key an Android memory LRU by exact media type plus SHA-256, sharing successful
pixels across paths/documents while avoiding stale document-ID keys after database
replacement. Cache misses additionally hash the returned verified asset bytes
against the frozen reference on the decode executor; metadata and asset reads are
separate transactions, so a changed asset must never publish under the old key.

Keep <=16,384 decoded pixels (64 KiB ARGB) per cover, the existing source/edge/SVG
limits and one shared serial encoded-read/decode pipeline across reader and covers.
The LRU retains <=4 MiB and <=64 entries; no new image framework/dependency or disk
cache. This routine cache policy satisfies the cover-cache slice while avoiding
another persisted representation; process restart rebuilds thumbnails from saved
assets. CoreSession retains it across Activity recreation. Visible/evicted bitmaps
awaiting GC and temporary decoder allocations are outside its retained-byte cap.
Never recycle published pixels during eviction/close.

Fit covers at source ratio in a 48x64dp frame, retaining compact title/progress/
finish rows. Use one decorative document icon for loading, missing/no-cover and
unsupported/damaged images, without hiding the title or interfering with the row's
OpenReader action. Do not add a cover viewer or mutate core storage errors.
Failures are not cached and retry when the row is redisplayed or Library reentered.
Disposal removes queued requests, suppresses late image delivery and drains
accepted metadata callbacks. No schema, native API, permissions or destructive
semantics change. Remaining compound SVG resources stay a separate unfinished task.


## 2026-10-27 — Bounded embedded SVG rasters

Continue the unfinished SVG task with self-contained base64 PNG/JPEG resources;
relative EPUB asset traversal, nested vectors, complex paint and selector-aware
resource CSS remain separate work. Recheck the pinned AndroidSVG 1.4 source JAR
against its recorded SHA-256 and upstream resolver documentation. Its
`checkForImageDataURL` runs before `resolveImage` and decodes without sampling, so
never pass original data-URI image hrefs through to the renderer.

During existing streaming preflight, validate a single unqualified/xlink href,
explicit width/height, exact PNG/JPEG media, source dimensions and bounds-only
codec detection. Cap unique exact URI identities at 64 within the existing 2 MiB
input cap; decode each distinct source once and share it for repeated placements
and `<use>` instances. Rewrite hrefs to generated private keys and install a
resolver that returns only those bitmaps. Do not install font/CSS/file/network
resolvers. Each temporary raster gets its equal share of one additional asset
pixel budget, within the existing edge/source limits; final output retains its
normal budget. Recycle temporary pixels and deregister the resolver on every
exit. This is a temporary decode cap, not a whole-process heap guarantee.

The renderer infers raster aspect ratio from sampled bitmap dimensions. Wrap each
image viewport in a group/SVG/image sequence using original source dimensions and
an inner stretched raster, preserving original fit/slice/none behavior even at
1x1 sampling. Preserve placement, transforms, IDs, inline styling and clipping;
count the generated wrappers in reference depth/expanded-node costs. Reject image
children and ambiguous/foreign hrefs. Keep stylesheet-styled embedded-image SVGs
on fallback because changing the element hierarchy can change CSS selector
matching; vector-only color/class CSS remains supported. Other unsupported
resources continue to show alt/Retry instead of silently missing content.

Serialize all SVG decode operations with a process-wide lock: each CoreSession
has its own executor, while AndroidSVG exposes a process-global resolver/entity
setting. This prevents one session from clearing another session's resolver or
recycling its temporary rasters. No new dependency, schema, core/FFI, permission,
source-asset mutation or destructive semantics.

## 2026-10-27 — Self-contained nested SVG image resources

Continue the compound-image task with exact `data:image/svg+xml;base64,` children,
using the pinned AndroidSVG 1.4 dependency. Rechecked the recorded source-JAR
SHA-256 and its image rendering/resolver path against the
[upstream resolver API](https://bigbadaboom.github.io/androidsvg/doc/com/caverock/androidsvg/SVGExternalFileResolver.html).
The resolver supplies bitmaps, so prepare nested vectors off main, then rasterize
each unique child once before its parent. Retain independent child IDs/styles;
outer styles do not enter an image document. Reuse the existing source-ratio
viewport wrappers, sampled page/viewer/cover pipeline and process-wide lock.
No original data URI reaches the renderer's unbounded built-in decoder.

Use one preparation context for all descendants: <=64 distinct exact URI resources
across SVG/PNG/JPEG, <=8 image-resource nesting levels, <=2 MiB total root/decoded
child XML, <=4096 generated/original nodes and <=32768 expanded nodes. Retain <=64
nested/reference levels per document. Count each unique child vector expansion
once, because repeated placements reuse its raster. Explicitly require a single
closed root: both platform pull parsing and upstream SVG parsing can accept a
truncated root at EOF. Malformed children reject the whole asset through alt/Retry.

Flatten children before parents and split one additional output-pixel budget
among ALL unique temporary images, including intermediate SVG bitmaps. No child
gets a new budget. Retain source limits and <=2048 sampled edges; recycle every
temporary bitmap on success/failure and deregister the private resolver in finally.
Child rasterization retains the existing sampled-detail limitation and does not
establish a total heap/CPU cap. Relative/local/remote image resources, patterns,
masks, markers and selector-aware/resource CSS remain separate unfinished work.
No dependency/version, core/FFI/schema, permission, persisted setting, source-asset
mutation or destructive-semantics change.

## 2026-10-27 — Basic SVG support is sufficient for v1

User clarified that complex SVG styles are unnecessary and basic support is more
than enough. Accept the existing verified bounded renderer and self-contained
PNG/JPEG/SVG images as sufficient for v1. Remove the remaining advanced SVG task
from the active plan, including additional resource loading, patterns/masks/markers
and selector-aware/resource stylesheet rules. Preserve existing behavior and
unsupported alt/Retry/generic-cover fallbacks; EPUB page CSS remains ignored.
This supersedes earlier notes describing those extensions as unfinished v1 work.
Fonts, line spacing and default-off furigana are next. Documentation-only scope
change; no implementation removal, dependency, persistence or runtime changes.


## 2026-10-27 — Bundle Noto Serif JP before typography controls

Implement the independently testable font/attribution part of the existing reader
typography task first. Use the unmodified Google Fonts Noto Serif JP variable
TrueType file, internal version 2.003, at immutable Google Fonts revision
`8b0a1d0f5983c89bc2b93f1b5fb55f9e252744b5` (13,574,352 bytes). Record font,
OFL and metadata SHA-256 plus original source revision in
`third_party/noto-serif-jp/README.md`; retain upstream METADATA.pb. Bundle the
complete SIL OFL 1.1 and both the font's Adobe copyright and upstream Google
license notice. Settings uses the existing offline, recreation-retained credits
dialog. No runtime downloads, new library, or font processing tool is required.

The existing Compose BOM 2026.03.01 resolves ui-text 1.10.6. Verified
[resource Font/FontFamily and variable-font APIs](https://developer.android.com/develop/ui/compose/text/fonts)
against official Android documentation before use. Explicit weight-axis values
400/700 give regular and real bold from one file; Compose synthesizes italic for
EPUB emphasis because this font has no italic/slant axis. API-29 minimum already
exceeds variable-font support's API-26 minimum. Keep Android glyph fallback for
characters outside the font (including emoji). No additional family selector.

Apply the family only in the shared reader text component, including plain
fallback and all fragments surrounding inline images. Preserve inherited body
size/line height, status colors, formatting/link annotations, exact text and core
logical pages/positions. Material controls, dictionary panels and SVG asset text
retain their existing typography. This slice adds no setting/schema/core/FFI
payload or destructive behavior. Persisted global font-size/line-spacing controls
and default-off source/analyzer furigana remain unchecked continuation work.


## 2026-10-27 — Persist global reader font size and line spacing

Use finite 14/16/18/20/24sp sizes and 125/150/175/200% line heights, with
16sp/150% defaults matching the existing Material bodyLarge 16sp/24sp metrics.
These are routine control ranges for the specified adjustable reader typography,
not another page-length setting. Android system font scaling still applies.
Store semantic integer values in migration 0015, validate without repair, and
save the pair atomically through the existing core/FIFO/JNI Settings protocol.
Controls submit explicit steps and display committed values; failed saves can
repeat and no value is accepted before loading or while time/completion saves
remain pending. No new dependencies or destructive semantics.

Apply the global pair in the shared reader text component for plain text and
all fragments around images. Retain Material letter spacing, Noto Serif JP,
relative heading sizes, emphasis, system theme/status colors and annotated taps.
Visual wrapping/height can change; core logical pages, exact reading offsets,
original analysis, image sizing/decoding and history do not change. Dictionary
panels and Material controls retain their own typography. Source/analyzer
furigana remains the next independent slice.


## 2026-10-27 — Transport source ruby before the furigana switch/rendering

Continue the furigana task with a bounded core/storage/JNI slice: carry existing
TXT/EPUB source readings through keyed ReaderChapterRuby in ReaderDocument and
clipped ReaderRuby in ReaderPage. Do not parse source files or load Full to expose
readings. Share the existing read-only ordered-index/range/reading loader between
TXT source, EPUB content and TXT reader repositories; preserve their transaction
boundaries. Pure core rejects foreign/duplicate chapter keys, invalid scalar
ranges, empty bases/readings and unordered/overlapping source bases.

A logical page may split a source ruby base. Clip its displayed UTF-8 range and
retain the complete supplied reading on every intersecting page, just as token
fragments retain whole-token readings. Japanese compounds do not provide a safe
proportional mapping from base characters/bytes to reading characters. Preserve
the complete original chapter annotation in ReaderDocument; never move logical
page boundaries or saved reading anchors to fit ruby. No source normalization,
vocabulary registration/status/history change, dependency or schema migration.
This is transport/projection only; keep the default-off switch, source-preferred
analyzer fallback, Compose layout and word/link tap verification unchecked.

## 2026-10-27 — Persist the furigana preference before exposing its switch

Extend the existing global ReaderTypography value with a default-false boolean.
Migration 0016 stores semantic 0/1 in the STRICT settings singleton with a CHECK
constraint. Validate exact integers on read; do not let SQLite's permissive bool
conversion silently accept corrupt values. Reuse the existing atomic typography
save and core/FIFO/generated JNI protocol, including loaded-settings, busy and
pending time/completion gates. Font/spacing controls copy the loaded value and
preserve the furigana flag. Library and consistent reader snapshots restore it.

This is a bounded persistence step of the open furigana task. Do not expose a
switch until source-preferred analyzer fallback and Compose rendering give it
an effect. Preference changes leave canonical text/ranges, logical pages,
positions, original analyses/tokens, time and vocabulary/history untouched.
No dependency, permission or destructive-semantics change.

## 2026-10-27 — Resolve source-preferred furigana in pure core

Continue the open furigana task with an independently testable projection before
Compose layout. `ReaderPage::furigana` merges existing source ruby with available
original Mode B token readings. `ViewModel.reader_furigana` publishes sorted
page-local `ReaderRuby` only when the committed global preference is enabled.
Source ruby is independent of token availability; analyzer fallback uses only the
existing explicit page-token snapshot, with no new effects, Full load or scan.

Each source base owns all overlapping analyzer tokens. Suppress a whole analyzer
reading on any source overlap rather than inventing how part of a compound's
reading maps to uncovered characters. Use analyzer readings for other tokens,
including kana bases; retain literal readings (no katakana/hiragana conversion).
Crossing page bases keep complete readings. Do not filter by vocabulary status or
presence: Known, Ignored and deleted entries retain available readings.

Validate full document/page/base identity, owned ordered token anchors/indices,
exact scalar-aligned clipped ranges and canonical surface fragments before using
any fallback. A stale or malformed analyzer snapshot contributes no readings;
valid source ruby remains usable. Existing refresh/reflow/movement/exit rules
invalidate the derived fallback automatically. Preference commits only affect
presentation metadata, never canonical text/anchors/history/time. No schema,
dependency, I/O protocol or destructive-semantics change. Keep the visible switch
and Compose layout/tap/theme verification unchecked for the next slice.

## 2026-10-27 — Furigana switch and canonical-text ruby paint

Expose the existing persisted default-off preference as a labeled whole-row
Settings switch, using the same commit/retry and busy gates as typography.
Use the enabled core projection directly; no Android reading inference or status
filter. Validate ordered scalar-aligned UTF-8 ranges again at the UI boundary,
then clip the converted UTF-16 ranges around verified image placeholders.

Keep Compose Text and its canonical AnnotatedString unchanged. Paint readings
above its measured base boxes rather than inserting reading characters or inline
replacement objects. This retains source formatting, link precedence, full token
identities, accessibility text and exact logical anchors. Use half-size Noto
readings in the current content color. Reserve their measured height above every
line, on top of the larger of selected line spacing and natural styled metrics;
bottom alignment and no first-line trimming prevent heading/first-line clipping.
Compress long readings horizontally to the base width to avoid adjacent overlap.
At visual line/image splits, repeat the complete reading, following the existing
page-clipping decision: Japanese compounds have no reliable proportional mapping.
No logical boundary moves or partial reading is inferred.

Use existing pinned Compose APIs only; no new dependency, schema or I/O. Primary
API references: AndroidX [LineHeightStyle](https://github.com/androidx/androidx/blob/androidx-main/compose/ui/ui-text/src/commonMain/kotlin/androidx/compose/ui/text/style/LineHeightStyle.kt)
and [TextLayoutResult](https://github.com/androidx/androidx/blob/androidx-main/compose/ui/ui-text/src/commonMain/kotlin/androidx/compose/ui/text/TextLayoutResult.kt).

## 2026-10-27 — Remove CI from scope

User clarification: CI checks are not needed. Remove the GitHub Actions task from
the implementation checklist and next-session priorities, and update the product
specification accordingly. Continue running relevant local tests, lint and builds.
Next implementation slice: custom meanings/readings for missing dictionary matches.


## 2026-10-27 — Missing-match custom dictionary persistence

Start the custom-definition task with a repository-only slice. Keep user meanings
and readings separate from the immutable JMdict/JMnedict asset and original
analyzer output. Migration 0017 stores one versioned entry per exact BINARY
spelling, independent of vocabulary registration and document lifetime. Do not
normalize Unicode, reuse a custom base-form definition for an inflected surface,
or change analyzer readings/mappings, source/analyzer furigana, statuses or history.
Both English meaning and reading are required nonblank fields; preserve literal
input and use labels rather than guessing language or restricting kana scripts.

Require the validated original reader token key/index for all three repository
operations. Factor the existing selected-word token resolver so custom operations
share its owned chapter/page/analysis/Japanese-token gates. Derive surface/base
from stored Mode B; requery the separately opened read-only dictionary on every
operation. Any word/name surface/base candidate blocks custom access, matching
the existing panel's empty-candidate policy. A dictionary failure is never proof
of absence. Existing custom data stays retained if a later dictionary gains a
match, but cannot override that match. No custom text is written to built-in tables.

Use immediate transactions for save/delete, validate existing custom values before
mutation, and skip identical saves/repeated deletes. Corruption fails without
implicit repair. Missing documents return None/false and deleted vocabulary stays
absent. Custom entries have no source/vocabulary FK; status/history deletion cannot
accidentally delete a user-owned dictionary definition. Explicit custom deletion
only affects that table. No existing destructive operation changes.

No new dependency/version or public bridge schema. Runtime/FIFO/core selection
lifecycle, typed custom result transport and Android editor/retry controls remain
the next slice; keep their implementation task unchecked.


## 2026-10-27 — Custom dictionary core/FIFO/FFI transport

Include the exact-surface custom fallback in `ReaderWordDetails`, using the same
user-state read transaction as selected tokens/status/counts, only after immutable
word/name lookup succeeds with no candidates. Keep a string-only `ReaderCustomEntry`
transport separate from the validated domain entry and original analyzer metadata.
No extra automatic read, analyzer load or custom base-form fallback is introduced.

Require the currently loaded missing-match selection for explicit save/delete events;
reuse shared busy/pending-save gates and recheck built-in surface/base matches in
storage for every mutation. Return the committed entry (or absence after deletion)
with the selection key. Attach only while pending/current selection and exact surface
still match. Dismissing or leaving the reader discards callbacks, but accepted atomic
writes may finish. This follows existing selected-word/status command lifecycle.

Retain the previous committed entry on failure and expose a separate custom mutation
error so clock/status saves cannot erase it. Retry only on another explicit save/delete;
identical repository retries already do no writes. Both outcomes drain waiting final
time checkpoints/departures. Reuse lazy dictionary connection/error retirement and
the existing FIFO/Render-only shell; no dependency/schema/architecture or destructive
semantics change. Android transport tests are part of this slice; visible editor and
delete/retry controls are the next independently verifiable slice.

## 2026-10-27 — Android custom dictionary editor

Show Add meaning and reading only after a successful lookup with both built-in
sections empty. Display committed custom reading/meaning separately from original
analyzer metadata, with Edit entry and Delete entry actions. Both input fields are
required nonblank; retain literal Unicode without trimming or language guessing.
Cancel discards the draft. Use the existing phone sheet/wide panel and typed events;
no new dependency, dialog or persistence mechanism is needed.

Scope draft/edit/retry state to the full original-token selection. Status/clock
updates preserve it; dismissal, movement, refresh and Activity recreation discard
selection/drafts following existing panel behavior. Committed data reloads on the
next tap. Disable fields and mutations while shared storage/time/completion gates
are pending. Input callbacks reset reader idle time, including IME edits.

Keep committed data and the draft on failure. Show a specific save/delete error
with an explicit Retry action holding the exact failed command, even if fields
have since changed; Save entry can submit the new draft instead. Only committed
success closes the editor. Clock saves/recomposition never submit mutations.
Deletion affects only the selected spelling's custom definition, using the existing
explicit atomic delete semantics; no vocabulary/history or built-in changes.

## 2026-10-27 — Effective segmentation boundary resolution

Continue migration 0007 with a pure resolver and one read-only repository API.
Keep this result separate from `ChapterAnalysis`: manual boundaries are not new
Sudachi output and cannot fabricate a reading, Japanese classification or lemma.
Resolved segments retain exact anchors/surfaces and applied rule IDs. An original
token index is available only when the effective span exactly equals that B token;
future consumers must obtain analyzer metadata from validated snapshots.

Routine matching policy: global rules apply only across complete adjacent Japanese
Mode B tokens, with literal source bytes. Avoid arbitrary substring edits within
other B words or across punctuation/whitespace. This occurrence uses the exact
source/chapter/range and the same B eligibility. Local rules win overlaps regardless
of age; newer OverrideIds win within one scope. A self-overlapping global rule
uses leftmost nonoverlapping matches. Apply winners once against immutable B,
independent of the caller's rule order and page length. Losing overlapping rules
stay active, so reversal can reveal them; reversal never reactivates a superseded
same-key edit. These rules are covered with synthetic boundary cases and the
pinned real Full dictionary before any UI exposure.

The A/B/C helper selects complete eligible spans from whole-chapter snapshots with
exactly matching engine/dictionary profiles; crossing tokens and changed profiles
are rejected. Manual split/merge continues using existing exact partitions. Reader
selection will need to expose complete original-B spans for these operations.

Storage loads original pinned B plus globals/relevant locals in one transaction,
including globals retained after originating content deletion. Missing B stays
missing; malformed snapshots/rules fail without partial projection or repair.
No schema/dependency/bridge change or automatic rescan is required. This step
does not yet change reader tokens, custom dictionary gating, furigana, vocabulary
registration or completion behavior. Effective selection/metadata/registration and
frozen completion identities must be integrated together before exposing controls;
past completed-reading occurrences must remain unchanged.

## 2026-10-27 — Effective segmentation metadata provenance

Add a separate effective chapter value and read-only repository API before changing
live reader/completion behavior. Each effective boundary optionally carries exact
analyzer metadata with its AnalysisId, split mode and whole-chapter token index.
Readings, dictionary forms and Japanese classification come only from a validated
token with exactly the same canonical source span; never concatenate neighboring
readings or copy a neighboring lemma. Missing metadata means unresolved, including
classification; it is distinct from a token classified as non-Japanese or a known
analyzer token with absent reading/lemma. Manual conjugation merges remain valid
boundaries even when no snapshot supplies metadata.

Routine deterministic preference is original B, then same-profile A, then C. This
preserves original metadata for unchanged spans and favors fine-grained A output
when both alternates match a changed span. Match by exact occurrence anchor rather
than surface alone so repeated spellings keep contextual readings. Validate entire
supplied snapshots, unique IDs/modes and original-profile ownership before returning
any result. Storage loads only existing A/C snapshots from the pinned original
engine/dictionary profile in the same transaction as original B and active rules.
Later profiles are not metadata fallbacks; missing B stays missing. No analyzer,
registration, history, bridge, schema or dependency changes are needed for this
prerequisite. Live effective eligibility/selection/registration and frozen future
completion identities remain the next integration step.

## 2026-10-27 — Effective reader eligibility and page projection

Keep reading eligibility separate from analyzer metadata. An exact matching token's
Japanese classification governs eligibility. When a manual split/merge has no exact
analyzer token, its parts inherit eligibility from the complete contiguous Japanese
original-B span required by override resolution. Label this `ManualJapaneseSpan`;
leave analyzer classification, reading and dictionary form unresolved. This keeps
manual conjugated expressions selectable/countable without guessing metadata or
classifying source characters heuristically. Exact non-Japanese output is excluded.

Use an opaque validated `EffectiveReaderChapter` that borrows canonical text and
owns resolution. Preserve complete effective-chapter indices (distinct from analyzer
indices), anchors/surfaces and exact metadata. Clip only display ranges; visible
crossings appear on both pages, and future occurrence projection owns each start
once. Retain repetitions independently of shared exact-surface vocabulary identities.

Expose a separate read-only storage page/status snapshot using the same resolution
transaction helper as chapter segmentation. Cache each distinct visible spelling's
indexed lookup within that transaction; missing/newly partitioned/deleted entries
stay absent. Retain full effective provenance in the snapshot. No dependency,
migration, public bridge or live-reader change is needed for this slice. Live word
lookup/registration and frozen future completions must be integrated before controls
are exposed; existing analysis, reading history and completion semantics stay intact.

## 2026-10-27 — Effective selected-word identity and read-only details

Freeze the complete resolved chapter/provenance plus document/page for effective
selected-word requests. Compare with a fresh validated resolution in the same read
transaction before using its effective segment index. Original AnalysisId alone
cannot distinguish a split/merge, replacement/reversal or newly available exact
metadata. Include applied rule IDs so reapplying the same boundaries does not
revive a stale selection. Exclude live vocabulary, statuses, counts and custom
entries from the key. Missing B cannot produce a selectable key. This internal
value is not a new public bridge payload; serialization/transport and large-chapter
costs remain to be addressed with the live integration.

Share original/effective exact-surface lifetime counting, dictionary candidate and
missing-match custom fallback composition. Resolve surface/base from freshly
validated storage, never caller text. Use a base only from exact analyzer metadata;
manual eligible spans remain surface-only, including custom-entry gates. Read-only
lookups never register new or deleted vocabulary or change completed history.
Keep registration, effective custom mutation commands and frozen future-completion
identities for later slices before switching live reader/furigana paths together.

The Full analysis integration fixture uses a separate empty immutable word/name
index, avoiding another downloaded-data requirement. Link the already pinned,
already used rusqlite 0.40.2 as an analysis dev-dependency to create this fixture;
no library version, production dependency, schema or public bridge change.

## 2026-10-27 — Effective custom-entry CRUD with transaction-local selection

Continue segmentation integration with a bounded storage slice: add effective
custom-entry load/save/delete using the existing complete resolved key and effective
segment index. Extract selected-effective-token validation from word lookup so both
paths reject stale rule/metadata provenance identically. Read loads use a read
transaction; mutations acquire an immediate transaction before resolving identity,
checking built-in matches and changing the custom row. This prevents a concurrent
override change between selection validation and mutation without a new revision
table or schema migration.

Share original/effective built-in gates and custom row helpers. Resolve exact
surface/base from storage; manual unresolved spans use only their surface. Word or
name matches (including exact analyzer base-only matches) block load/save/delete;
dictionary errors do not permit a custom fallback. Keep literal Unicode fields,
invalid/corrupt-row rejection, write-free identical saves and repeated deletes,
atomic failure rollback and global spelling identity. Custom entries survive
vocabulary deletion and do not register absent/deleted spellings or supply analyzer
metadata. Retain live original-B behavior until effective registration, frozen
future completion identities and reader paths can be integrated together.


## 2026-10-27 — Durable frozen effective completion reservations

Add effective completion as a separate domain/storage prerequisite while the live
reader continues using B. Validate the full rendered effective key after final-time
drain and reserve one immutable command containing position/preset/time, boundaries,
rule/metadata provenance and start-owned occurrences. Persist exact versioned JSON
in additive migration 0018. Authenticate the entire payload before first commit or
replay; legacy completion APIs cannot consume an effective command ID. Later override
or metadata changes do not reinterpret an accepted reservation. Normal live page/
position/preset/time checks still govern its first commit; recorded retries precede
live-content checks and remain harmless after deletion/reflow/reversal.

Keep the original B/profile header in completion_analyses; the effective reservation
explicitly identifies the different segment-index semantics and exact metadata sources.
Share existing exact-surface registration and New-only transitions with original B.
Register only the departing effective page at actual completion, including deleted
spellings; do not register from projections/preparation or rescan other pages. Preserve
all repetitions and unresolved manual metadata, with no guessed readings/lemmas.
Override-time registration is a separate remaining integration task, not implicit here.

JSON uses existing pinned serde_json/Serde, with Serialize-only domain values; no
new dependency/version or public bridge schema. Reservations are write/compare only,
not a recovery queue. Retained commands can retry across storage reopen, but process
restart does not auto-replay unfinished reservations. Entire chapter payload costs
remain unmeasured; review before switching live reader/FFI paths. No old history is
backfilled or modified, and vocabulary deletion still removes actual encounter/status
rows through existing cascades. No destructive migration or architecture change.


## 2026-10-27 — Atomic override-time registration prerequisite

Complete the independent storage prerequisite before the larger live-reader switch.
Add `save_registered_segmentation_override(edit, timestamp)` alongside the existing
analysis-only save API, sharing the transaction-local override row helper. Require
live canonical text, complete contiguous Japanese original-B token boundaries and
valid whole-chapter snapshots/rules. Hold one immediate transaction across validation,
rule replacement and registration. Return the rule ID and distinct new-entry count.
No new dependency, migration, analyzer invocation or public bridge payload is needed.

Register only eligible effective segments owned by the newly saved rule within the
selected source occurrence. Exact analyzer classification takes priority; unresolved
manual parts inherit Japanese-span eligibility without guessed readings/lemmas. The
AllOccurrences rule still resolves elsewhere using existing precedence, but saving
it does not scan other occurrences/documents to register hidden/contextually different
parts. Existing local winners suppress hidden global parts at the selected occurrence.
Future explicit completions retain their existing page-only registration policy.

Preserve existing vocabulary/status/history and add dated New registration events
only for genuinely absent spellings. An identical active-rule retry reuses its ID
and performs no writes, even after vocabulary deletion or from another occurrence;
it must not turn a retry into a rescan. Reversal disables the rule without deleting
registered vocabulary, resurrecting superseded rules or altering past readings/time.
A deliberate fresh save after reversal may register an absent spelling again.

This internal repository prerequisite does not yet authenticate a rendered selection
or provide durable retry identity after intervening edits. Add those guards in the
future typed control workflow; do not expose the raw repository API directly to UI.
Live effective completion/token/furigana/word/custom integration and payload-cost
review remain separate, with the current original-B reader intact.

## 2026-10-27 — Effective furigana prerequisite and payload evidence

Project effective furigana from `EffectiveReaderChapter`'s private validated
resolution before the coordinated live-reader switch. Reuse one source-precedence
merge helper with the existing original-B projection. Validate chapter source ruby
in full, reject invalid/foreign/empty pages, clip visible bases and retain complete
literal readings. Only exact Japanese analyzer metadata supplies fallback readings;
unresolved manual spans never inherit old B readings. Vocabulary/status/custom data
does not participate. Source ruby remains usable on unresolved spans.

Add a reproducible synthetic size example using existing pinned serde_json and
record its results in EFFECTIVE_READER_PAYLOAD.md. At 100,000 characters, full-key
JSON is 14,290,979 bytes versus 42,519 bytes of 500-character page tokens. This
justifies addressing identity representation before transporting full chapters in
render/selection payloads. It does not measure actual JNI bytes, peak memory or
real-device latency. Bridge identity/lifecycle design stays pending; no new cache,
handle system, dependency, migration or product behavior is chosen by this slice.

## 2026-10-27 — Compact effective reader identities and one retained page

Use a pure core-owned `EffectiveReaderSession` component for the next live-reader
integration. Keep one full chapter key in Rust and expose a compact session revision
plus document/page/original-B identity. Selections add an effective segment index.
Move the validated storage resolution into the retained key instead of cloning it;
send only visible token fields to Kotlin. This follows the current core-owned reader
state and existing Rust effect routing, without an opaque runtime service, LRU,
hashing dependency or schema change. Full key equality remains the repository's
transactional authority; a compact revision is a stale-callback guard only.

Issue fresh revisions before reads, including same-page refresh/retry, and clear
current/pending state immediately on movement/reflow/exit/edit. Keep the counter for
the entire core lifetime; checked exhaustion fails closed. Results must echo the
exact Rust-issued attempt and match document/page/B. A failed/stale result cannot
consume another attempt; Missing B has no selection. Resolve selections only against
the retained visible eligible indices, preserving complete crossing anchors.

Scope identities to their existing native/core handle. Close/replacement drops all
UI callbacks; do not persist or carry revisions between sessions. Accepted commands
retain their own full frozen keys across drains/retries, independently of visible
page invalidation. Durable completion semantics and no restart replay remain intact.
Live status changes leave provenance intact; an explicit refresh retires callbacks.
Intervening-edit retry guards remain required for the future override controls.

Register prerequisite page/selection schemas explicitly with the Kotlin generator
and test them independently before modifying live app events/views. Generated-codec
round trips verify 60-byte available-B identities and 68-byte selections including
maximum unsigned counters and literal Unicode. The synthetic compact JSON page is
32,983 bytes at both 10k/100k chapter characters. These are serialization measurements;
real JNI timing/memory, live integration and physical-device validation remain pending.

## 2026-10-27 — Native effective commands on the existing FIFO

Introduce the effective storage protocol as a nested, Rust-only operation/result in
the existing storage capability. Consume it on the same FIFO, SQLite connection and
lazy word dictionary as original-B operations. Serde skips both nested variants;
Facet skips them and treats their payload fields as opaque. Full chapter keys and
durable completion payloads intentionally have no shell codec. No new worker/service,
public JNI entry point, dependency, migration or alternate persistence policy.

The pure retained-page session issues private-field accepted command values after
authenticating compact page/visible selection identities. Store the immutable key
in `Arc` and share it with accepted commands/retries, keeping chapter-sized copies
out of retry cloning. UI invalidation does not discard accepted work. Storage still
revalidates provenance for every word/custom/preparation transaction; only already
reserved completions survive later edits with their frozen meanings. Preparation
follows final time checkpoints in the same queue, and restart never auto-replays.

This is an independently verified transport slice before the coordinated live app
switch: current core events/views and Android still use original B. Native protocol
failures reuse `DocumentError`, pending command ownership remains with the core, and
unexpected effective results in the current live state machine fail closed.

## 2026-10-27 — Retained effective word/custom lifecycle and compact details

Keep selected-word and custom-command lifecycle inside the existing pure retained
reader session. A Rust-only callback request combines a checked monotonically
increasing attempt with an accepted shared-key selection and literal action.
Even exact-payload retries get a fresh attempt, preventing duplicate old callbacks
from consuming the retry. Reuse the existing native FIFO operations, typed errors
and repository built-in/stale-key checks; add no worker, dependency or migration.

Move validated native word details into a compact generated shell schema after
checking complete key, selection and visible token/metadata. Full resolution never
enters selected-word views. Counts/candidates/custom values remain live, unresolved
manual metadata stays absent, and a live vocabulary observation updates repeated
visible spellings. Successful empty matches permit custom controls; immutable word
or name matches and lookup failures do not. Keep literal fields, no optimistic
entry replacement, and no status/encounter/position writes from presentation.

Retain accepted custom payloads across dismissal/invalidation and explicit retries;
the old callback still drains but cannot restore the retired presentation. An
in-flight command gates mutations. After failure the committed entry remains;
explicit retry uses frozen values, and explicit Cancel may discard that retry
without writes. A new explicit load/save/delete can supersede a failed retry;
Retry itself never reads edited fields. Lookup failure releases its request and needs another explicit
load. The live app must apply the shared FIFO/time/completion gates and capture
these native request tokens when adopting this component. This verified slice
does not switch app events/views or Android from original B; completion/token/
furigana/word/custom still need their coordinated integration.

## 2026-10-28 — Retained effective completion ownership

Keep accepted completion lifecycle in the existing pure effective reader session.
Freeze the rendered partition and intent before final clock drain, immediately
invalidate UI callbacks, then dispatch preparation only after the app drains both
accepted and coalesced checkpoints. Word I/O/accepted custom retries gate acceptance;
pending completion gates new word/custom work. No clock, SQL or new worker in core.

Use a native checked attempt per dispatch/retry, authenticate preparation against
the entire frozen key/intent, and share the exact durable reservation across commit
retries. Failed preparation/commit retains explicit retry ownership. Do not expose
Cancel: atomic commit can succeed before its follow-up read fails, requiring exact
AlreadyRecorded replay. Only a consumed success returns position/time/notice inputs;
duplicate or stale callbacks cannot return another success. A new session does not
auto-replay reservations. No dependency, migration, shell codec or live reader change.
Coordinated core/FFI/Android adoption remains the next implementation slice.


## 2026-10-28 — Effective manual status validation and retained ownership

Include manual five-status choices in the coordinated effective-reader switch.
The original status worker accepts a vocabulary ID without effective provenance;
add a native effective command rather than translating an effective segment index
into an original-B selection. Reuse the existing manual status/history helper in
one immediate SQLite transaction with complete resolved-key, visible eligible
segment and live exact-surface ID validation. Same-status requests stay no-ops;
deleted vocabulary stays absent and an old ID cannot mutate its replacement.
No analyzer/dictionary load, registration, encounters, position or time changes.

The existing retained session owns the accepted status key/ID/choice/timestamp.
Checked fresh callback attempts protect explicit exact retries from old results.
Keep committed presentation until success, then update repeated visible spellings
and selected-word details only for the same retained revision. Invalidation retires
presentation without abandoning accepted I/O. Failed requests require explicit
retry, another explicit choice or Cancel; in-flight requests must drain. Pending
status gates word/custom/completion, and their accepted obligations gate status.
The live app must still apply its shared FIFO/time/lifecycle gates. No dependency,
migration, shell schema or current Android behavior change in this native slice.
## 2026-10-28 — Guard boundary prompts against intervening edits

Add a storage-only prompt context for the existing effective-reader boundary work.
It shares the native frozen full key through `Arc`, keeps canonical original-B
selection bounds and exposes only already stored, exact same-profile A/B/C choices
and winning rules. It has no shell codec. On-demand analysis, retained callback
ownership and the coordinated live reader switch remain separate pending work.

Compare the full key and an opaque rule revision inside the same immediate
transaction as save/registration or reversal. The revision is `(MAX(rule id),
inactive rule count)`: current repositories append immutable rules, deactivate
only, and preserve rows after document deletion. This detects split-then-reversal
even when full rendered provenance returns to B. Conservatively retire prompts
after edits elsewhere; status/count/time changes do not retire them. No migration
or persistent command registry is required; computing the revision scans rules.

Rollback preserves the before-image for explicit retry. A committed mutation makes
the old context stale, including a lost-success response; the future UI must offer
explicit reload and never automatically rebase/replay old fields. An already
identical active-rule save remains write-free and never re-registers deleted
vocabulary. Reversal accepts only an exact winning rule shown by that context.
These are repository guards, not implemented Android flows or new product controls.

## 2026-10-28 — Retained guarded boundary requests on the existing FIFO

Continue the independent guarded-edit prerequisite before attempting the broad
coordinated live-reader switch. Keep prompt context and accepted command ownership
in the existing pure effective session; dispatch through its existing native-only
storage envelope/FIFO. No second worker, shell chapter payload or durable edit queue.
Stored A/B/C choices and manual parts share save validation/registration. Freeze
scope, literal partition, timestamp and shared before-image; each explicit retry
gets a fresh checked callback attempt. Pending word/custom/status/completion work
mutually gates edits; the live app must additionally apply its clock/lifecycle gates.

Retire the rendered page on write acceptance. A storage failure keeps the frozen
request for explicit exact retry or failed-request Cancel; in-flight writes must
drain. Stale/invalid contexts discard retry ownership and require explicit reload,
without rebasing old fields. A successful write also requires reload; a lost success
therefore becomes stale on retry and cannot recreate deleted vocabulary. Cancel
after a failed write retires newer rendered state too because its response may hide
a commit. Dismissed reads drain without restoring presentation. Keep typed native
stale/invalid/storage failures outside the existing shell `DocumentError` schema.
No dependency, migration or live Android change; on-demand A/C and compact prompt
controls remain separate work after the coordinated reader switch.

## 2026-10-28 — Guarded on-demand same-profile A/C generation

Complete the independent analyzer prerequisite before the larger coordinated live
reader switch. Keep ordinary page/prompt reads read-only. Add an explicit native
generation action to retained override requests and reuse the existing FIFO/import
analyzer cache; no new worker, dependency, migration or public shell schema.

Storage first checks the shared rendered key and rule revision and reuses any
existing original-profile A/C snapshot, even when its boundaries cannot form a
prompt choice. Missing modes analyze the whole canonical chapter outside SQLite's
transaction, preserving contextual output and allowing competing connections to
write. Verify mode, exact original profile and all anchors, then validate the same
before-image again under the immediate transaction that stores immutable analysis.
Failed SQL/commit leaves no partial cache. Explicit retries freeze context, mode
and profile; deterministic output can be recomputed until stored. Conflicting
output under an existing immutable identity remains an error.

Extend the worker analyzer trait with an explicit profile-aware mode capability;
import-only closures reject it. The native Full adapter checks pinned engine and
dictionary identities before tokenizing. Bad output/engine failures discard cached
analyzer state; reuse and background release keep existing import policy.

New exact metadata can change effective identity without changing boundaries.
Generation therefore uses retained write gates, invalidates callbacks on acceptance
and requires explicit page/context reload on success. Failed requests permit exact
retry or Cancel; stale contexts never rebase, including after lost success. Cache
generation writes no overrides, vocabulary, encounters, progress or time; previously
reserved completions retain their frozen unresolved/resolved meanings after reopen.

## 2026-10-28 — Compact boundary prompt contracts and prompt revisions

Project selected-span choices and winning-rule display/reversal facts into a
standalone generated shell contract before Android controls. Keep the shared full
key, rule-table revision, original rule anchors and exact retry payload in Rust.
Reuse SplitMode/OverrideScope and the existing typed override failures; no new
dependency, migration or change to live reader behavior.

A page revision alone cannot reject a delayed action after dismiss/reopen of the
same span without reloading the page. Give each prompt a checked fresh revision
from the existing native attempt counter. Pair it with compact page identity;
authenticate Choose/manual Save/Generate/Reverse/Dismiss against that prompt.
Accepted writes retain its identity across invalidation, with Retry/Cancel scoped
to the pending obligation and fresh native attempts on dispatch. No persisted
handle/registry, automatic rebase or replay. Default scope remains All occurrences.

Keep the coordinated live token/status/word/custom/completion/furigana switch
unchecked. This slice supplies a testable prompt transport/handler contract; it
does not expose controls that could edit boundaries while completion still uses B.

## 2026-10-28 — Retained effective rendering adapters

Complete a testable rendering portion of the coordinated reader switch before
changing live commands. Project furigana directly from the retained effective
page's exact visible metadata, borrowing its key rather than cloning or resolving
the chapter again. Share page/surface/range validation and source-overlap policy
with the existing B projection. Keep validated source ruby on absent, invalid,
stale or Missing B fallbacks; unresolved manual parts get no guessed reading.

Extract Android's existing pure UTF-8/UTF-16 token geometry from Compose and reuse
it for compact effective tokens. Carry the full surface plus page revision and
effective segment selection through clipping; never substitute a B index. Reject
the whole projection for malformed Unicode/ranges, stale identity or metadata on
an unresolved manual token. Compile this production helper in the host Kotlin/JNI
test runner so boundary guards can be tested without an emulator. No dependency,
migration or new shell schema. Keep the live switch unchecked until token/status/
word/custom/completion/furigana all adopt the retained protocol together.

## 2026-10-28 — Common retained reader callback adapter

Before the coordinated live switch, give accepted effective requests one native
Crux dispatch/response adapter. Capture the original request in the callback;
never reconstruct ownership from returned tokens or mutable app selection. Route
ordinary FIFO failures as well as native effective outputs through each existing
retained lifecycle. Unexpected envelopes fail closed; stale/duplicate callbacks
return None and must not release a newer shared busy obligation. Invalid token
responses consume only the captured outstanding read and require an explicit fresh
revision. Retain write reservations/payloads for their existing exact retry policy.

Keep shared clock/FIFO/lifecycle gates in the app and leave retry/follow-up dispatch
explicit. No schema, migration, dependency, worker or automatic replay. Verify all
request families on the existing real FIFO before adopting them in live events.
When manual status observes deleted vocabulary, clear its selected-word encounter
count alongside the missing status, matching erased history and the existing B UI.

## 2026-10-28 — Library title search

Take the existing plan's title-search portion as a small independent slice;
leave the coordinated effective-reader switch as the next reader task. Filter
the core's retained Library summaries during view projection. Do not add a SQL
query or scan stored content. Match literal substrings after locale-independent
Unicode lowercasing on both title/query; retain whitespace, punctuation, combining
marks and supplementary characters without normalization or wildcard syntax.
Preserve the underlying collection/order, errors, import outcomes and progress.

Keep the literal query for the owning core session across tabs, Settings,
reader navigation and Activity recreation; reset on a new session. Ignore search
events while Library is hidden. Search/clear can render during an outstanding
storage operation without changing its busy ownership. Return the total Library
count separately so zero matches and no imports have different empty messages.
This changes ephemeral Event/ViewModel codecs only; no migration, dependency,
durable preference or sorting policy. Sort controls and five-status bars remain
unchecked in the plan.


## 2026-10-28 — Library sorting and durable read order

Continue the existing Library checklist with Last read (default), Title and
Import date controls as a small independent slice. Keep the coordinated effective
reader switch next in the reader plan. Retain the selected sort beside the title
query for the core session, including Activity recreation, navigation and refresh;
a fresh session defaults to Last read. Sort/search remain pure core projections
and may update presentation while storage is busy without taking busy ownership.

Define recency by successful explicit reader entry, including an immediate exit
without page completion. Advance an ordinal clock in that entry's existing SQLite
transaction, so wall-clock adjustments cannot reorder reading sessions and failed
opens cannot promote a book. Use a singleton high-water mark plus a nullable value
per live document (migration 0019), with checked exhaustion and no expanding event
log. Do not infer old recency from position or reading history; legacy values stay
unknown until explicitly opened. Position/time/completion reloads and duplicate
imports preserve order without advancing it.

Order known recency newest first, then unknown/unopened books newest import first.
Use existing autoincrement document IDs as exact committed import order for Import
date; no date is displayed, so do not invent/backfill wall timestamps. Title sorts
by locale-independent Unicode lowercasing without normalization, ascending, with
newest import as a deterministic tie-breaker. The Android menu names the current
choice. No dependency, destructive-data behavior or new reader interaction.

## 2026-10-28 — Library distinct-surface status bars

Continue the existing Library checklist with stacked New/Learning/Familiar/Known/
Ignored bars. Count exact registered Japanese surfaces once per whole document,
deduplicating repetitions and chapters before joining current global vocabulary.
Use the first stored B per chapter, matching the live reader; later profiles and
stored A/C do not substitute. The coordinated effective-reader switch must also
switch this membership projection to resolved partitions before exposing edits.
Do not silently give the Library a different segmentation from the live reader.

Keep this a separate lazy read on the existing FIFO so title/progress listing,
import summaries and duplicate preflight do not acquire a token scan. Read every
distribution in one SQLite statement/snapshot without source text/assets, Full,
registration or writes. Deleted vocabulary contributes to no status and is never
recreated. A missing chapter B yields an unavailable distribution; an analyzed
document with zero live Japanese entries has an empty neutral bar.

Retain the compact counts/errors in core, invalidate after imports, status
observations/completion, refresh and return to Library, and request them only
while Library is visible and saves are drained. Search/sort/recreation reuse the
snapshot. Defer automatic reads and disable their Retry during an active file
batch, so scans cannot take the FIFO between provider reads and import submission.
A failed read has its own explicit Retry and cannot clear import errors.
Use the existing status palette plus green for Known, and expose all five exact
counts to accessibility. No schema/dependency or persistent cache. Large-library
query cost is unmeasured; include it in the existing performance work.

## 2026-10-28 — Effective Library membership prerequisite

Implement the pending Library membership part of the coordinated effective-reader
switch as a separate read-only storage API. Keep the live Library on original B
until the reader switches too. Reuse validated chapter resolution and one shared
eligibility predicate for Library surfaces, reader tokens and completion, so
unresolved manual parts and exact non-Japanese metadata cannot drift apart.
Borrow eligible surfaces without allocating full-chapter page tokens; deduplicate
literal spellings across chapters before reading current indexed vocabulary.

Use one deferred SQLite transaction across live documents, chapter text, stored
analyses/rules and statuses. Resolve one chapter at a time; do not load/verify
original file or image BLOBs or load Full. Missing B makes a document unavailable;
corrupt available data fails closed. Never register missing/deleted entries or
write caches, history, position or recency. Global rules remain valid after their
origin document is deleted. Existing shared resolution does require canonical
text and stored-analysis reads; include that cost in the planned measurements.
No migration, dependency, public wire-schema or new product behavior.

## 2026-10-28 — Confirmed single-document deletion

Continue the existing Library deletion task with one independently testable action;
leave rename and bulk/Select All unchecked. Store the confirmation target and its
separate retry error in pure core so Activity recreation keeps the exact title/ID.
Only a visible Library row can open the prompt; confirmation must match its ID.
Cancel is read-only, and in-flight confirmation rejects duplicate submissions/Cancel.
Navigation retires presentation; an accepted callback still reconciles only its ID.
Keep title query/sort and unrelated errors. Defer deletion controls during a file
batch using the existing Library action gate, alongside automatic status scans.

Use the existing SQLite FIFO and one immediate transaction to delete all canonical
chapter text for that live document, then its Library row. Existing cascade rules
remove live analysis/ruby/cues/EPUB resources/original bytes/dedup/position/partial
time; source/chapter provenance, original title, completed-reading commands/time/
occurrences, global vocabulary/statuses/custom entries and overrides survive.
This implements the specified document-deletion semantics without a new migration.
Missing IDs are idempotent no-ops for explicit lost-response retries. Do not load
content/dictionaries before deleting or reset allocation/recency high-water marks.
Normal reimport creates fresh IDs, zero progress, no finish/recency/partial time;
existing vocabulary/history remain intact. No dependency or architecture change.

## 2026-10-28 — Library rename

Implement rename as the next small independent Library slice while the coordinated
effective-reader switch remains pending. Keep the ID/literal draft/separate error
in core so Activity recreation preserves edits and failed saves. Open only from a
visible row; Save must match its ID. Freeze accepted values until the FIFO returns,
reject double submissions/edits/Cancel during that write, and allow explicit retry
or editing/Cancel after failure. Navigation retires the editor, while the accepted
callback can still update only its exact ID/title. Keep search/sort and word counts.

Reject blank/whitespace-only rename submissions without trimming, normalizing or
truncating accepted titles. This is editor validation; preserve existing imported
titles, including empty first lines. Store only `documents.title` in one immediate
transaction. Retain historical original titles, all source/content/assets/analysis,
progress/finish/recency/time, vocabulary/history and dedup keys. A missing document
reports NotFound and is never recreated. Same-title retries skip UPDATE, so explicit
lost-response retries are write-free. Update matching import summaries and retire
cached loaded documents after success. Disable Android actions during file batches
using the existing Library gate. No dependency or migration; bulk deletion stays
separate and unchecked.

## 2026-10-28 — Confirmed Library bulk deletion

Implement the remaining Library deletion slice independently of the coordinated
effective-reader switch. Reuse single-document cleanup in one immediate transaction
for the entire frozen selection; never chain independently committed deletes.
Reuse bound statements to support selections beyond a SQL parameter-list limit.
Keep absent/duplicate IDs idempotent and retain the existing history/vocabulary/
provenance/global-rule semantics. A failure on a later document or deferred COMMIT
must restore earlier deletions too. No migration, dependency or new deletion policy.

Keep selection and confirmation in pure core across Activity recreation. Long press
starts selection, taps toggle visible rows, and Select All adds all current title
matches, including rows not composed on screen. Preserve selected hidden rows across
search/sort; display the complete selected count and scrollable frozen title list in
confirmation. Do not include unselected/unmatched rows merely because they exist in
the Library. Cancel of the prompt returns to selection; Cancel/Back exits selection.
Navigation/accepted content operations clear presentation, while accepted callbacks
still reconcile only their exact ID vector. Read-only Library status scans preserve
selection and confirmation. Freeze targets through failures/explicit retries and
disable selection edits/Cancel/double submission during accepted writes. Use the
existing file-batch and reader-save UI gates. Reimports use fresh IDs, so a repeated
old selection cannot delete them.

## 2026-10-28 — Effective resolution performance before live adoption

Take the planned real-book/Library performance slice independently of the coordinated
reader switch. Use an opt-in host example over supplied TXT/EPUB files with a new
scratch database, explicit Full analysis and distinct B-only/stored-A/B/C/global-edit
phases. Keep inputs/corpus out of Git and verify that timed reads preserve database
bytes. Record host/setup/iteration details and keep JNI/device/EPUB/large-Library
claims separate from the measured one-novel TXT fixture.

Remove the measured extra metadata tree by reusing exact original-B indices and
binary-searching sorted validated A then C only for changed spans. Continue to
validate entire snapshots and require both anchor ends; preserve original provenance,
classification, unresolved metadata, command keys and serialized reservations exactly.
Do not add a cache, schema/payload change, guessed metadata or live-reader adoption
as part of this optimization. A full-scan oracle across 4,096 Unicode partitions and
existing storage/real-Full cases guard the unchanged semantics. Remaining hundreds
of milliseconds of repository work need separate measurement before a cache design.

## 2026-10-28 — EPUB Library scaling and transaction-local status sharing

Continue the pending host EPUB/Library performance slice independently of live
reader adoption. Use the unchanged IDPF Kusamakura sample and an explicit opt-in
`--epub-copies` harness option. Change only ZIP comments for distinct source hashes;
keep package members/text/ruby/assets identical and import every copy through the
production repository. Label the result a repeated-book scaling fixture, never a
diverse corpus or Android measurement. Stream database hashes to avoid a second
database-sized allocation; keep the disposable database under configurable TMPDIR.
No new dependency, migration or import/dedup behavior.

Share exact-surface vocabulary observations between documents only inside the
existing effective Library read transaction. A referenced spelling has one global
status/absence in that pinned snapshot, so repeated books do not need repeated
indexed lookups/entry validation. Retain the existing per-document surface set and
full per-chapter analysis/rule validation, including corrupt later chapters and
Missing B. Discard the map on commit/failure; the next scan must observe external
status changes, deletion/re-registration and repaired data afresh. Do not preload
unreferenced lifetime vocabulary, persist a cache, weaken provenance or switch the
live B Library independently of the reader. The transient map consumes memory
proportional to referenced distinct spellings; peak allocations remain unmeasured.

## 2026-10-28 — Independently verified Android boundary editor

Implement the compact Android boundary dialog as an independent part of the
remaining override controls while the coordinated live-reader/Library switch
stays pending. Render the existing typed prompt and emit only its scoped actions;
do not add a second native session, worker, analysis path, migration or dependency.
The owner must provide the retained native session's identity and shared FIFO/time
gates. Opening/selecting the original-B span and forwarding these actions through
live Event/ViewModel remain part of that switch.

Initialize manual boundaries from a valid stored B choice, otherwise the whole
literal selected span. Toggle cuts at Unicode scalar boundaries, including
combining characters; join removes cuts without editing source text. Validate
nonblank parts against Rust's Unicode White_Space rules, which differ from JVM
isBlank. Stored A/B/C choices save explicitly; absent A/C request generation.
Show winning-rule surface/parts/scope before explicit reversal. Rust owns exact
pending writes: Android emits Retry/Cancel with the accepted prompt identity and
requires explicit reload for retired contexts. No automatic retries or writes.

Keep the saveable cuts/scope above Dialog's separate view save-state registry.
Restore only for the same retained native session, full compact prompt identity
and literal surface; replacing/restarting a session discards old drafts even if
revision numbers repeat. Pending write payloads never enter saved Android state.
All pointer/key/scroll interaction in the dialog reports reader activity.


## 2026-10-29 — Stage effective page/completion application integration

Integrate the retained session into the actual application gates in an independently
testable slice. Use a native `Model::with_effective_reader()` constructor; keep
production Model::default until remaining word/status/custom/override, Library and
Android consumers can switch together. This is an implementation staging point,
not a product setting. Reject legacy word/status/custom/completion events in the
opt-in model rather than interpreting B token indices as effective segments.

Reuse LoadReaderTokens and RetryReaderCompletion. Add compact page presentation
and an explicitly revision-anchored CompleteEffectiveReaderPage event. Freeze
before final-time drain; prepare after both timer slots clear. Capture each native
request in its callback and retain it as the app's FIFO owner. A dismissed token
read can lose its page lifecycle while still owning that FIFO slot, so release the
matching app request and drain queued time even when finish_request returns None.
Never release another request's busy state. Move full results rather than cloning
chapter keys; box the native callback request to bound Event variant size.

Share original/effective position/finish/notice application. Preparation success
chains commit; failures require explicit retry. AlreadyRecorded is silent, final
new success returns Library with the existing celebration, and no restart replay
is added. No dependency, schema, new worker, data semantics or dictionary load.

## 2026-10-29 — Stage compact application word/status/custom consumers

Continue the native effective-model staging slice with typed compact word lookup,
manual status and custom CRUD events/views. Reuse the retained session and existing
FIFO adapter; keep production on B until override/Library/Android migrate together.
Lookup retains no reading writes, custom CRUD changes only literal dictionary
fallback fields, and storage still validates full provenance/built-in protection.

Expose explicit payload-free Retry/Cancel events for accepted failed status/custom
writes, as for existing timer/completion retries. Rust retains exact selections,
values and timestamps; each dispatch gets a fresh callback attempt. Preserve pending
custom errors even after dismissal so the remaining obligation stays reviewable.
Defer movement/exit while writes are pending, including failures, and block refresh,
reflow and other storage work except timer saves. Checkpoints keep coalescing during
word/status/custom I/O and drain even after failure; departure waits until both time
slots and writes clear. Cancel never abandons in-flight work. Keep compact status
facts separately visible after a word panel closes. No new setting, dependency,
migration, worker, automatic retry/replay or production activation.

## 2026-10-29 — Stage compact application override consumers

Wrap the existing `EffectiveOverrideAction` in one app event and expose its compact
retained state directly. Authenticate Open against the current page revision and
subsequent actions against the retained prompt identity. Keep full contexts,
intervening-rule revisions and exact retry payloads native. Continue production B
until effective Library and Android consumers can switch together.

Include save/reverse/on-demand generation in the shared accepted-write gate;
generation can change metadata and therefore needs the same invalidation/reload
policy. Reads may dismiss immediately while retaining their app FIFO obligation.
Writes retire the page before dispatch, survive dismissal/failure and defer
movement/import/reflow until success or explicit Cancel. Allow clock checkpoints
to coalesce during override I/O and drain both slots before deferred movement.
Retry/Cancel authenticate the frozen prompt even after page retirement, with a
fresh attempt on Retry. Success/stale outcomes require explicit page/context
reload; never rebase or automatically replay. Invalidate Library distributions on
override outcomes. No dependency, schema, worker, history or product-setting change.


## 2026-10-29 — Stage effective Library membership through the app FIFO

Let the native effective-model constructor choose the existing effective Library
repository through a skipped `LoadEffectiveLibraryWordDistributions` operation.
Use the same FIFO, compact result, retained cache and error/explicit Retry path as
original B. Keep all visibility, busy, pending-write/time gates and preserve
literal rename drafts, selection and frozen deletion prompts during reads.
Do not serialize this native staging choice or add a user mode. Production remains
original B until Android switches with the reader. No dictionary, dependency,
schema, worker, automatic retry or data semantics change.

## 2026-10-29 — Stage compact Android word consumers before activation

Keep production on original B until Android navigation/completion and original-B
span selection/editor are ready for the coordinated switch. Add an independent
`EffectiveReaderWords` component accepting the compact app ViewModel and an
explicit retained-native-session key. Share only annotation styling, dictionary
presentation and the phone/wide word-panel frame with the original reader; never
convert effective indices into original-B selections.

Scope selections and literal custom drafts to the native session and page revision.
Authenticate retained link callbacks against the current compact selection before
dispatch. Preserve unresolved metadata and full crossing surfaces, render invalid
projections as plain text with explicit reload, and load/retry through existing
compact events and shared FIFO/time gates. Keep native pending status/custom
Retry/Cancel reachable after panel dismissal or page retirement; Android never
reconstructs accepted write payloads. Custom validation uses Rust Unicode
White_Space and strict scalars while retaining the literal submitted fields.
No new library, wire/schema change, user setting or production activation.

## 2026-10-29 — Stage effective Android navigation with retained handle ownership

Give each Android `CoreSession` a fresh UUID string, retained with the native
handle across Activity recreation and passed explicitly through the app/reader.
Never save or restore it independently of the handle. Use it for compact word
selection/drafts, scroll reset and stale navigation callback guards.

Choose the effective renderer from the native optional word-state projection,
which remains present when its token page is missing/retired. Next/Finish require
a validated available effective page and dispatch its compact revision/base plus
the exact departing position. Previous keeps its existing position-only event;
it remains usable when token loading fails. Gate movement on storage/time/
completion and accepted effective mutations. Use current gesture callbacks so
same-page revision changes cannot leave swipes targeting an old partition.
Sample/pause the Android clock before effective completion, using the existing
native checkpoint drain and exact completion Retry path. No new Kotlin history,
time or retry payload ownership. Production remains B until original-B span
selection/editor and core/FFI/Android activation are ready together. No dependency,
wire/durable schema or product-setting change.

## 2026-10-29 — Original-B edit span selection in the staged reader

Expose only visible Japanese original-B anchors, surfaces and clipped display
ranges alongside the compact effective page. Project them while original B is
already loaded in the page's read transaction; keep effective boundaries/metadata
and full native provenance unchanged. Do not infer B edges from edited segments
or perform an extra whole-chapter analysis/read for Android selection.

Use an explicit boundary action to select one original token or two endpoints of
a contiguous Japanese span. Both crossing edge tokens remain whole. Reject gaps,
malformed Unicode/ranges and Missing B. Save only presentation endpoint indices,
scoped to the retained native session and complete compact page identity. The
prepared editor owns scope/cut drafts; native prompt identities, exact accepted
writes/retries and transaction guards remain authoritative. Opening/submitting
uses shared storage/time/completion/mutation gates; failed accepted commands stay
reachable after page retirement, and reload is explicit.

Keep production on B for this independently verified staging step. The following
activation step must switch the production native model and Android consumers
together and verify actual JNI/SQLite reader, edits, Library and completion flow.
No dependency, SQLite migration, historical rewrite or destructive semantic change.

## 2026-10-29 — Serialized effective-reader activation candidate

Provide a Rust-only effective factory at the EffectRouter and YomokuBridge layers
before switching NativeBridge. Existing effective app/FIFO tests bypassed the
Bincode boundary, while production JNI fixtures still send B indices and legacy
completion events. Exercise the actual serialized events/callbacks/views against
SQLite using the same dictionary loaders, FIFO and analyzer lifecycle; keep the
full retained requests inside native code.

Use deterministic synthetic analysis and a small SQLite dictionary in the new
FFI regressions to make partition provenance, scope, exact retries, literal Unicode,
read-only loads and frozen completion outcomes independently reproducible.
Production remains B; this factory is the concrete constructor for the subsequent
coordinated NativeBridge/Android activation and fixture migration. No additional
BoltFFI export, shell setting, dependency, wire schema or durable migration.

## 2026-10-29 — Production effective reader activation

Select `YomokuBridge::with_effective_reader` in NativeBridge's existing constructor.
Keep the existing verified Full/word providers, FIFO, lifecycle and generated wire
contracts. Android already selects compact consumers from native word-state
presence; migrate JNI/device fixtures to current page revisions and effective
segment selections instead of converting them to original-B indices. Original B
stays immutable and supplies the editor's contiguous Japanese anchor spans.

Successful override writes/generation retire the native prompt and rendered page.
Use the reader's existing explicit reload affordance, labeled Reload for that
retired page; read failures retain Retry word lookup. Pending writes still expose
native exact Retry/Cancel. Do not preserve a stale editable prompt after success,
automatically reload/rebase, or guess metadata for manual segments. No dependency,
wire/durable migration, history rewrite or destructive semantic change.

Production-device completion/reopen verification exposed a reload marker carried
from an earlier edited page into a newly opened reader. Clear that presentation
requirement on normal page invalidation and explicit page load; override outcomes
set it again after retiring their before-image. Keep pending accepted writes,
errors and retry payloads intact. Closing/reopening the same book must permit the
normal initial read without requiring a second manual reload.

A normal position save can keep the same logical page anchor while retiring its
native token revision. Scope Android's one-attempt token-read guard to base,
exact reading position and page-length preset. Reload normally when those change;
keep explicit Retry after failures in the same context and explicit Reload after
boundary edits. Clock saves, error clearing and typography alone must not replay
a failed read. A compact component regression covers same-anchor position/preset
changes; the real typography/position fixture exercises production invalidation.

## 2026-10-29 — Bound interactive layouts on long reader pages

Debugger samples of the long-page failure found main-thread work in Compose's
`TextLinkScope.applyAnnotators`/`replaceStyle` and Android span geometry for each
word link, in addition to the previously observed HWUI wait. Move the fixed word
foreground/background/decoration into ordinary spans. Give clickable word links
explicit empty `TextLinkStyles`: null lets Material inject website styling and
reintroduce the annotation work. Keep real link nodes for taps/accessibility.

For text fragments over 1,024 UTF-16 units, measure at the actual column width,
then slice after a soft visual line ending once the current block reaches 512
units. This bounds repeated link geometry within long paragraphs. Never insert
characters or split a scalar; annotated slicing preserves selection callbacks,
source styling and crossing-token identity. Bidirectional text, direction controls
and explicit paragraph styles keep their complete layout; do not force paragraph
direction on a partial range, which can introduce an extra visual paragraph.
Clip/repeat whole ruby readings using the existing rule. Skip
hard-break split points to avoid adding an empty visual line. Logical pagination,
position, completion/time/history and native wire/storage contracts do not change.

Navigation/typography fixtures must wait for Library's automatic distribution
load before direct import/open commands, including after closing Settings and
returning from a reader. A transient idle snapshot can precede that Compose load;
the existing busy gate correctly rejects commands sent during the subsequent read.

Primary implementation references: AndroidX
[TextLinkScope](https://github.com/androidx/androidx/blob/androidx-main/compose/foundation/foundation/src/commonMain/kotlin/androidx/compose/foundation/text/TextLinkScope.kt)
and Material [Text](https://github.com/androidx/androidx/blob/androidx-main/compose/material3/material3/src/commonMain/kotlin/androidx/compose/material3/Text.kt).
No dependency/version or data-destructive semantic change.

## 2026-10-29 — Measure Android effective reads and retain primitive import bytes

Use an opt-in instrumentation harness through the production CoreSession/native
FIFO/codecs, with one disposable database per public corpus and no Activity or
reader clock. Time dispatch through the idle callback, and report synchronous
dispatch separately; that column does not measure the later callback's main-thread
decode cost. Keep setup observations separate from warmed read distributions and
record the native build profile. Verify fresh page revisions, read-only database
hashes, absent reading history/time and scratch-handle cleanup. Image measurements
use the existing asset/decode queue and aggregate page budget without drawing.
Do not infer optimized-build, physical-device, frame-time or peak-memory results.

The 17,940,673-byte real EPUB exhausted the emulator's 192 MiB Java heap while
eagerly constructing `List<UByte>`, before native import. Replace that conversion
in the actual URI picker and measurement setup with `FileImportBytes`, a read-only
list view owning the private primitive buffer. The serializer may box a byte
transiently; the pending file no longer retains an object plus list slot per byte.
Encoding selections/retries retain the same exact buffer; no caller mutates it.
Keep the existing typed event/wire format, original-byte identity, atomic import,
cancellation and error policy. This is not a streaming import or a universal heap
bound for arbitrarily large files. No dependency/version/schema/size-limit change.

Picker/notification fixtures must wait for automatic Library word counts before
direct start or picker-button actions. A transient idle view before that scan is
not readiness. During an active import batch, counts intentionally remain absent.

## 2026-10-29 — Phase-5 historical event contract audit

Retain the existing schema-0019 versioned tables: completion/status/analysis headers
and effective JSON reservations each have explicit v1 semantics. Occurrences inherit
the analyzed command contract; reservation presence distinguishes effective indices
from original analyzer indices. Repository APIs append history and expose no edits.
Do not rebuild tables or backfill legacy encounters merely to begin aggregation.
Source kind/original title and chapter order remain independently joinable after
live document deletion. Unused reservations never count as completed reading.
Audit all history-bearing upgrade starting points, late migration rollback/retry,
allocation high-water marks and real effective payload retention with executable
fixtures. No dependency, durable schema, wire or destructive semantic change.

## 2026-10-29 — Read-only lifetime occurrence/time aggregation

Use a pure totals value and a single SQLite statement in the separate storage
crate. Select complete analyzed events (including zero-Japanese pages) for both
occurrence and time totals, following the existing no-inferred-legacy-statistics
contract. Count surviving occurrence rows rather than vocabulary surfaces, analyzer
tokens or effective reservation JSON. Sum completed durations once per command,
independently of repetitions, so vocabulary deletion recalculates word counts
without erasing page time. Pending time and unused reservations contribute zero.
One statement supplies a consistent snapshot without caches, registration, Full
loading, content reads or a new worker. Reject unsupported included header versions
and negative durations; exact signed-integer SUM overflow returns a storage error.
Keep shell serialization/effects, timeframe/status projections and Compose charts
for their own slices. No dependency, migration or data-destructive change.

## 2026-10-29 — Five-status history replay and causal chart dates

Implement the independently testable five-status portion of the phase-5 timeline
before Known dictionary-form counts. The latter require durable analyzer links
for registered-but-unread surfaces, surviving document deletion; do not infer them
from completed encounters or silently count only live analyses.

Replay surviving v1 registration/manual/completion transitions in immutable event
ID order through the pure core. Validate per-entry registration/old→new chains;
storage validates causes and completion provenance in the same read statement.
Use exact signed deltas grouped by timestamp, then absolute unsigned counts for
each of the five statuses, reusing `WordStatusCounts`. Do not derive history from
current cached statuses, content, JSON or dictionary loads. All simultaneous
changes coalesce into one point. Vocabulary-history cascades erase its entire
contribution; document deletion leaves the projection intact.

Existing write contracts permit nonnegative wall-clock dates without requiring
monotonicity, and frozen completion dates may precede committed status changes.
For chart projection only, take the maximum of each entry's recorded event date
and preceding projected event date. This preserves its causal order, nonnegative
counts and final actual status; it leaves unrelated entries' dates and all stored
events unchanged. Sorting raw wall dates could otherwise move an unregistered
entry or apply an obsolete transition last. Core validates before mutation and
storage returns no partial points on invalid history. Retain only entry state and
per-date deltas; do not copy the complete event stream or introduce a cache.
No dependency/version/schema/wire or data-destructive semantic change.

## 2026-10-30 — Vocabulary-owned durable analyzer links

Append migration 0020 rather than deriving Known dictionary-form statistics from
live documents or completed reading alone. Retain distinct analyzer-resolved
written forms under the current vocabulary ID, with cascade on vocabulary deletion
and no document FK. Exact spelling, including Unicode distinctions, deduplicates
observations across readings; preserve distinct resolved forms across contexts
instead of silently picking an arbitrary analysis. Null/unresolved and manual
metadata add no association and never remove prior evidence.

New snapshots link existing entries only. Imports, selected registered boundary
edits and actual original/effective completions link their own validated metadata
after registration, atomically. Raw surface registration, read projections,
reservations and identical snapshot/rule retries do not restore erased evidence
by scanning old content. New evidence may link a newly registered generation.

Upgrade backfills surviving live Japanese snapshots and historical occurrences
for existing vocabulary; it never analyzes text or manufactures missing forms.
Unread metadata deleted before upgrade cannot be recovered. Older snapshots lack
vocabulary-generation identity, so one-time recovery can only use their exact
surface join to currently registered entries. Schema-20 reopen performs no such
rescan. Keep the count/timeline projection as the next independently testable
slice; no dependency, wire, core I/O or destructive semantic change.

## 2026-10-30 — Known dictionary-form timeline from vocabulary links

Extend each pure status-timeline point with distinct Known dictionary-form counts,
using the existing migration-0020 links and the same validated status history.
Supply all links before replay, deduplicate exact spellings and exclude unresolved
surfaces. Any Known linked surface keeps the form Known; all retained automatic
context resolutions participate, with no reading identity or Unicode normalization.
Links have no acquisition timestamp: apply the current generation's retained
evidence throughout its replay. New evidence can refine earlier projected counts;
do not fabricate dated link events or rescan content to recover erased evidence.

Accumulate signed reference-count changes per form and projected date, then replay
them chronologically. Computing the distinct total in global event-ID order would
misdate a shared form when unrelated entries have different wall dates. Preserve
existing per-entry causal clamping and coalesce simultaneous handoffs before
returning absolute counts. Keep entry/link state and per-date deltas, without a
complete event copy. One SQL UNION streams links before events in one snapshot,
avoiding an event-by-link expansion or separate potentially inconsistent reads.
Document deletion retains links/history; vocabulary cascades recalculate both
lines and fresh raw registration has no inferred links. No dependency, migration,
wire or data-destructive semantic change; timeframe buckets and UI remain next.


## 2026-10-30 — Statistics calendar ranges and lifetime bucket values

Keep calendar projection pure and caller-clock driven, with no new dependencies,
SQL/platform I/O in core, schema change or persisted counters. The initial policy
is UTC calendar dates: subtract one/six/twelve calendar months from the supplied
as-of date, clamp the day to the target month's end and begin at midnight. Use
Monday-start weeks for six-month/year views, calendar months for all time and
days for the last month. Clip partial edge intervals and include the supplied
instant using an end-exclusive cutoff one millisecond later. This records the
routine boundary choices left unspecified by PRODUCT_SPEC.md; device timezone/
DST behavior is not introduced. Last year is the default.

Carry the lifetime state strictly before the visible range as an explicit opening
value, then emit absolute end-of-interval values, including empty intervals.
Occurrence/time origins never reset under filtering; five statuses and Known
surface/dictionary-form counts remain absolute. All time starts at the earliest
visible reading or registration/change month, including unread registrations;
empty/future-only history uses the current month. Ranges never precede epoch.
Reject calendar dates outside 1970–9999 without changing durable timestamps.
This bounds all-time allocation and calendar arithmetic independently of storage.

One deferred SQLite read transaction supplies both validated status/link replay
and date-grouped analyzed completions. Count actual surviving occurrence rows per
command independently of time, so repetitions/rereads count and zero-word page
time survives. Preserve the lifetime SQL's signed-integer total limit even across
date groups; pure core additionally checks unsigned accumulation. No frozen JSON
replay, inferred legacy counts, content rescans or cache writes. Errors release
the transaction without partial output. Add only native projection values/API;
worker, Crux/FFI state and Compose chart rendering stay in the next slice.


## 2026-10-30 — Native statistics transport and retained queries

Use the existing FIFO worker and its typed storage errors for calendar statistics,
with no new library, schema or I/O in core. Keep the storage request, boxed result
and request-attempt callback native. Generate only explicit timeframe/as-of shell
events and retained presentation values. Default to LastYear. Retry uses the exact
failed cutoff; identical lazy loads are idempotent, including after failure, to
avoid a render-driven retry loop. A new cutoff is an explicit fresh read.

Capture an incrementing attempt ID per accepted read so late callbacks cannot
release a newer owner, even for an identical query. Coalesce timeframe/as-of
changes behind the current read, clear mismatched data, discard stale successes
and errors, and dispatch the latest query only when visible. Reject requests
behind unrelated busy work without changing presentation; the UI retries its lazy
load when idle. Keep Settings/recreation state, invalidate on reentry and relevant
mutations, and preserve unrelated document errors. Compose will capture wall time
once per entry/selection rather than each render. Do not add chart or drilldown
behavior in this transport slice. Generated schemas are a same-build boundary;
regenerate Kotlin/native together, with generation sequential to JNI execution.

## 2026-10-30 — Statistics charts use exact retained bucket values

Use Compose Canvas and existing Material controls, without adopting a chart
library. Render the three specified graphs as an overview with tappable drilldowns
and a lazy list of exact interval-end values, including the lifetime opening row.
Show UTC range/grouping explicitly. Keep lifetime origins and absolute statuses;
step segments represent bucket closing samples rather than interpolated event
dates. Use elapsed timestamps for x spacing and the full visible series extrema
for y bounds. Constant lines sit at the plot midpoint; labels retain their exact
value. Keep ULong through projection/labels and subtract before converting drawing
distances to floating point. Reuse status colors and distinguish the Known
dictionary-form series with a dashed tertiary stroke. Provide textual legends,
accessibility summaries and exact rows for interpreting coincident lines.

Keep drilldown choice in session-keyed saveable shell state; all statistics data,
timeframe, error and Retry ownership remain in Rust. Capture now for the first or
invalidated query, reuse a retained query's cutoff through busy/render/Settings/
recreation transitions, and capture again on an explicit timeframe selection.
The screen exists only while visible and resubmits its lazy load when storage is
idle. Explicit Retry carries no newly sampled time. No periodic background refresh,
time totals UI, migration, dependency or new durable behavior is added.

## 2026-10-30 — Read-only Vocabulary browser repository

Start the browser with pure query/page/row values and one native read transaction
per page. Reuse the BINARY unique surface index: match a literal lower prefix
bound and its exclusive Unicode-scalar successor, skipping surrogates and handling
trailing U+10FFFF. Empty prefix matches all. Preserve case, normalization,
whitespace, embedded NUL and wildcard characters exactly; do not use LIKE or
search analyzer readings/forms. Status choices form a union intersected with the
prefix; default all five, explicit empty set none, duplicates ignored.

Use registration timestamp descending then stable ID descending for Newest.
Frequency is lifetime completed-reading occurrence count descending, including
repetitions and rereads. LastSeen is the maximum recorded completion timestamp,
not registration, manual status change, dictionary lookup or commit order; unseen
entries follow seen ones. Frequency/date ties use the Newest order. Alphabetical
uses exact BINARY surface ascending without locale or reading-based collation.
These are routine ordering choices within the specified four sorts.

Return matching count and at most 100 rows from the same deferred read snapshot,
with an optional next offset. Reject offsets above SQLite's signed integer range.
Offsets require an unchanged query/dataset: future retained state must restart
from zero after mutations and reject stale responses. Existing occurrence indexes
supply counts/dates with correlated probes; frequency/last-seen sorts may evaluate
all matching histories and require a temporary sort. No performance claim for
large histories, new counter cache or index migration. Reads never load content,
analysis, dictionaries or reservation JSON, register erased entries, or write.
Worker/core/FFI/Compose integration remains the next independently verified slice.


## 2026-10-30 — Retained Vocabulary paging transport

Use the existing native FIFO and one storage slot for read-only Vocabulary pages.
Expose LoadVocabulary, SetVocabularyQuery, LoadMoreVocabulary and RetryVocabulary,
plus session-retained query/count/rows/next-offset/loading/error presentation.
The default query has an empty literal prefix, all statuses and Newest sort.
Canonicalize only the status set to fixed enum order; preserve prefix bytes.
None count means unattempted/invalidated, Some(0) means successfully empty.

Load the first 100 rows lazily and append only on an explicit load-more event.
Keep successful rows/count during an append or failed append. Retry replays the
failed query/offset exactly; automatic load and load-more never retry errors.
Clear old rows on a changed query and coalesce selections behind the current read.
Capture monotonic attempt IDs, reject late/duplicate callbacks, and mark an active
read obsolete on every query change/reentry, including A -> B -> A. An obsolete
callback releases only its own slot and starts the latest page zero only if visible.
Settings preserves query/pages/error and defers coalesced follow-ups until the lazy
load resumes. Unrelated busy work rejects browser input without replacing state.

Retain query choices across navigation; restart pages on reentry (external writes)
and imports, status changes, completions and effective edits/observations. Validate
page offset/size/next-offset, membership and duplicate IDs before appending. A
changed matching count between pages restarts at zero. No cross-page snapshot or
protection against unseen external changes with unchanged counts is claimed; a
reentry refresh remains necessary. Keep native requests/results/callbacks out of
the shell codec. Generate query/state/row values with exact u64/ULong counts and
i64/Long dates/IDs. No migration, dependency, clock, SQL in core or dictionary
rescan. Compose browser/infinite scroll and bulk commands are separate slices.

## 2026-10-30 — Vocabulary Compose controls and demand paging

Use existing Compose Material controls and a LazyColumn, with no new library.
Render written surface, current status and exact completed encounter count per
row; keep ULong for count labels. Send literal prefix edits without trimming or
normalization, toggle statuses independently (including none), and dispatch all
four sorts through the retained core query. Keep controls in the scrolling list
for small-window accessibility. Preserve query/page ownership in Rust.

Lazy-load only an unattempted error-free query while visible and idle. Observe
the last loaded row with snapshotFlow to request the next native 100-row page;
pause while any storage read runs and resume if that row is still visible.
Never page or retry automatically from error state; retain successful rows and
offer explicit Retry for both initial and append errors. Permit query coalescing
during Vocabulary reads, disable input during unrelated work. Keep list position
in session/query-keyed saveable shell state hoisted across Settings; reset on
query changes. Do not add row actions or bulk mutation behavior in this slice.

## 2026-10-30 — Frozen Vocabulary repository selections

Capture all matching IDs with the browser's existing indexed literal prefix/status
predicate in one SELECT. Return ascending IDs independently of presentation sort
and loaded pages. Keep query capture separate from mutation so the forthcoming
retained confirmation and Retry can use exact IDs, including unloaded matches,
without admitting later matching imports or new generations of erased spellings.

Apply bulk status changes with the existing validated per-entry helper inside one
immediate transaction. Preserve registration dates and old events; append a manual
event only for an actual transition, at the supplied frozen date. Missing/repeated
IDs and same-status retries are no-ops. This follows existing single-status retry
semantics and does not introduce a persistent command receipt.

Delete only frozen IDs in one immediate transaction, relying on established
cascades for events/occurrences/dictionary-form links. Preserve completion/time/
provenance and live content/analysis. Use bound per-ID statements to avoid parameter
limits. Empty/absent/repeated IDs are harmless; all statement/cascade/commit failures
roll back the whole selection. Existing AUTOINCREMENT allocation prevents stale
IDs from touching replacements. No migration, library, rescan, counter repair,
automatic registration or shell action is added by this repository slice.
