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
