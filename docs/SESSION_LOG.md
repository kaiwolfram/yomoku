# Session log

## 2026-10-22 — Automatic step commits
- User authorized automatic local commits after each completed, verified step.
  Recorded the workflow in AGENTS.md and NEXT_SESSION.md, including progress
  docs, preserving unrelated work, and reporting the commit hash or blocker.
- Documentation-only change; verified with `git diff --check`.

## 2026-10-22 — Foundation
- Inspected blank repository; preserved original handoff. Added AGENTS.md,
  planning/resume/decision/architecture/data/development docs, Cargo workspace,
  .gitignore and pure vocabulary policy with integration tests.
- Build/test/fmt/clippy passed offline; five tests. No Android build attempted.
- Corrected initial environment assessment after user pointed out `android` CLI;
  verified help/info/templates/SDK packages with approved escalation. Templates
  downloaded by CLI; no project/platform/build tools installed yet.
- Next: Crux API/typed command, runtime/FFI, Android CLI Compose scaffold.

## 2026-10-22 — Pure Crux application
- Started clean; preserved vocabulary code. Verified published Crux 0.20.0
  source and current Kotlin bridge direction before adding dependencies.
- Added app model/event/view, Library-default destination selection and render
  command; four host app tests. Updated crate manifest/lockfile and continuity,
  architecture, data-model and decision docs.
- Build/test/clippy passed offline (nine tests); fmt initially reported one
  assertion layout, fixed with cargo fmt and the repeated check passed.
- Sandbox DNS prevented initial crate lookup; approved cargo info/fetch worked.
  Offline build initially lacked bincode; dependency fetch resolved it.
- Next: Rust EffectRouter/FFI error round trip, pinned Kotlin codegen, Android.
  No Android or cross-language checks run; no new product blockers.

## 2026-10-22 — Runtime router and Rust bridge facade
- Started clean; preserved core/domain implementation. Added runtime shell effect
  mapping and FFI facade with update/resolve/view, async callback plumbing and
  typed errors. Added four host bridge integration tests; updated workspace/lock
  and plan, resume, architecture, data model, decisions and development docs.
- Build/test/fmt/clippy PASS offline; thirteen tests. Optional typegen feature
  compilation PASS. Initial compile lacked effect(typegen); corrected after macro
  source inspection. Offline optional dependency resolution failed; approved fetch
  succeeded. No native/Kotlin/Android checks run.
- Next: verified BoltFFI/native exports and generated Kotlin tests, then Android
  scaffold and ABI packaging. No new product blockers; Q188 remains deferred.

## 2026-10-22 — Kotlin payload generation
- Started clean. Added FFI generate-kotlin binary, optional Facet metadata on core
  payloads and shell effects, feature wiring/manifests and refreshed Cargo.lock.
  Updated plan, resume, development, decisions, architecture and data model.
- Crux Serde generator lacks Kotlin; switched optional generation to Facet after
  published API inspection. Missing offline generator dependencies were fetched
  successfully with approved cargo fetch.
- Default workspace build, all-feature tests (13), fmt and all-target/all-feature
  clippy PASS. Kotlin generation twice and recursive output diff PASS.
- Kotlin compiler/Gradle/BoltFFI not installed on PATH; no Kotlin compilation,
  native round trip or Android build run. Native bridge checklist remains open.
- Next: BoltFFI bindings and generated Kotlin tests, Android scaffold, ABIs.

## 2026-10-22 — Native bridge exports
- Started clean. Added pinned BoltFFI 0.30.1, native facade and EffectsListener,
  typed error export and static/shared crate types; refreshed Cargo.lock.
- Added two host tests for payload/error forwarding, independent instances,
  cross-thread use and owned listener release. Updated plan, resume, architecture,
  development and decisions docs; no persisted data model changed.
- Workspace build, all-feature tests (15), fmt check and all-target/all-feature
  clippy PASS. Kotlin payload generation twice and diff PASS. Shared-library
  constructor/method/callback/release symbols verified with nm.
- No Kotlin/JNI execution or Android build run. CLI/compiler setup and actual
  cross-language tests remain next; asynchronous callback delivery needs a real
  asynchronous effect. No new product blockers.

## 2026-10-22 — Kotlin/JNI execution
- Started clean. Installed matching BoltFFI CLI 0.30.1 and official Kotlin 2.4.21
  compiler locally; downloaded official OpenJDK 25 JNI headers because the host
  has only the runtime. Tools/downloads remain ignored; no system changes.
- Added native generation config, Kotlin bridge integration tests and reusable
  offline runner. Updated plan, resume, architecture, data model, development
  and decisions docs. Native code remains generated; no product behavior added.
- Kotlin/JNI tests PASS with strict C/Kotlin compilation and JVM JNI checks:
  navigation, codec round trips, typed errors, instance isolation, calls after
  close and 100 lifecycles with listener registry cleanup.
- Initial offline CLI install lacked the package; approved download/install
  succeeded. Kotlin download initially failed sandbox DNS; approved retry worked.
  Initial runner link failed because BoltFFI metadata expansion replaces native
  exports; reproduced and fixed by rebuilding Rust after generation.
- Rust workspace build, all-feature tests (15), fmt and all-target/all-feature
  clippy PASS. Both generators repeated with identical output; shell syntax and
  git whitespace checks PASS. Dedicated Kotlin lint/format tooling remains for
  Gradle setup. No Android or asynchronous callback checks run.
- Next: Android Compose/Gradle setup, native ABI packaging and device/emulator
  command → view test. No new product blockers.

## 2026-10-22 — Product specification consolidation
- Started clean. Consolidated the original handoff into docs/PRODUCT_SPEC.md and
  removed the superseded root document. All eight product/architecture sections
  and unresolved inputs are preserved verbatim apart from heading numbers.
- Removed obsolete startup instructions and duplicate roadmap; retained all
  twelve cross-feature acceptance cases in IMPLEMENTATION_PLAN.md and clarified
  the session workflow in AGENTS.md. Updated active references, decisions and
  resume notes; no product behavior or implementation task status changed.
- Requirement-preservation comparison, local documentation-link/reference checks
  and git diff --check PASS. Application tests/build/lint were not rerun for this
  documentation-only change; prior successful checks remain recorded.
- Next implementation tasks remain Android Compose/Gradle setup, native ABI
  packaging and responsive navigation. No new blockers or product decisions.

## 2026-10-22 — Android Compose build foundation
- Started clean. Generated `android/` with installed CLI/template; added pinned
  Gradle wrapper, Compose app/min SDK 29, full local JDK 17 and SDK/build tools.
  Verified dependency/toolchain APIs against primary documentation and official
  release metadata. Downloads and keys stay ignored; JDK archive SHA-256 checked.
- Removed template repository/navigation/ViewModel/example tests. Initial shell
  shows Library with system theme/insets; Rust remains the application owner.
  Disabled automatic backup/transfer per product spec. No persistence added.
- Added ktfmt 0.64 runner for Android Kotlin/Gradle scripts. Updated plan, resume,
  architecture, development and decisions. NDK pin/install is explicitly retained
  with native packaging, since this Kotlin-only step does not need an NDK.
- Debug APK build and strict lint PASS offline after fixing template icon folder
  `mipmap-anydpi-v26` → `mipmap-anydpi`. APK min/target/compile metadata checked.
  Kotlin formatting, shell syntax and git diff whitespace PASS. Rust build,
  all-feature tests (15), fmt, clippy and Kotlin/JNI suite PASS.
- Initial downloads failed sandbox networking; approved retries succeeded.
  Offline Gradle also needs escalation for local sockets. SDK XML v4 and unstripped
  AndroidX native graphics notices did not block build/lint. Java 25 ktfmt warns
  about upstream Unsafe usage; JDK 17 formatting is clean.
- No Android unit/instrumentation/device tests run. No NDK/emulator installed by
  this step; no /dev/kvm. Next: native ABI packaging and command → view device test,
  responsive navigation, then migration-safe storage. No new product blocker.

## 2026-10-22 — Android native packaging and shell connection
- Started clean. Verified official NDK/Rust/AGP/test APIs; installed pinned NDK
  r30 30.0.16248370 and both Rust Android targets. Added offline native-build
  script and Gradle preBuild integration for generated Kotlin/codecs and combined
  Rust/JNI arm64-v8a/x86_64 libraries. Generated linker files stay ignored too.
- Added CoreSession main-thread render projection/handle lifecycle and connected
  the Library heading to Rust. Added Android command → core → view/isolation/
  close instrumentation test; compiled its APK. No persistence/navigation UI yet.
- App/test APK assembly and strict Android lint PASS offline. Packaged native
  ELF architectures/exports, 16 KB LOAD alignment and APK ZIP alignment PASS.
  Host JNI suite, workspace build, 15 Rust tests, fmt/clippy, Kotlin formatting,
  shell syntax and git whitespace checks PASS.
- Fixed AGP 9 source registration (use main.kotlin.directories, only generated
  app/com folders, excluding Crux's standalone .kts). Initial offline test
  dependency resolution failed; approved online retry populated caches.
- ADB socket access required escalation; device listing succeeded but was empty.
  No emulator/system image or /dev/kvm. Android test execution/app launch remain
  unverified and unchecked. Native build uses debug Rust; release tooling later.
- Next: run instrumentation on available Android, responsive navigation with
  configuration-change retention, then SQLite migrations/reopen tests.

## 2026-10-22 — SQLite initialization and numbered migrations
- Started clean. Device/emulator execution remains unavailable; took the independent
  phase-1 storage task, prioritizing persistence. Added yomoku-storage, pinned
  rusqlite 0.40.2/bundled SQLite after primary API/source verification, and refreshed
  workspace/lockfile. No core, runtime, FFI or Android behavior changed.
- Added private connection ownership, per-open foreign keys, bounded lock wait,
  numbered journal initialization and atomic migration validation/application.
  Reject newer/inconsistent/unrecognized databases without data reset or repair.
  Updated plan, resume, architecture, data model, decisions and development docs.
- Workspace build, 26 Rust tests (11 storage), fmt/clippy and host Kotlin/JNI PASS.
  NDK r30/API 29 storage cross-builds PASS for arm64 and x86_64, including SQLite.
  Initial crate metadata lookup failed sandbox DNS; approved retrieval succeeded.
- Android APK assembly/lint/device tests were not rerun for this isolated storage
  crate; no device/emulator or /dev/kvm. Document CRUD/persistence in the app and
  async JNI execution remain unimplemented and unchecked.
- Next: available-device instrumentation, responsive navigation/session retention,
  stable document IDs/Unicode offsets followed by storage effects/repositories.
