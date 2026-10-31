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

## 2026-10-22 — Android CLI emulator installation and Android execution
- User requested emulator installation through Android CLI. Created medium_phone;
  CLI downloaded Emulator 37.2.12 and API 36 Google Play x86_64 image revision 7.
  Host KVM is usable; earlier sandbox-only /dev/kvm checks hid the host device.
- Default startup crashed in SwiftShader GLES RenderThread (GDB confirmed).
  Explicit swangle/headless/no-audio/no-camera/KVM flags boot successfully;
  persisted AVD settings alone did not fix CLI startup. Added the tested foreground
  tools/start-android-emulator.sh launcher; SDK/AVD files stay outside Git.
- Accepted first-boot host debugging prompt through the authenticated emulator
  control API. Temporary gRPC tools/generated proto/screenshots remain ignored.
  Launcher cold restart PASS, persisted ADB authorization PASS, boot_completed=1.
- connectedDebugAndroidTest PASS offline: one CoreSession JNI/core/view/isolation/
  handle-close test. Initial offline run lacked AGP UTP dependencies; online run
  fetched them but found no online device, then authorized offline execution passed.
  Debug APK install, Activity cold launch and visible Library screenshot PASS.
- Launcher bash syntax and git whitespace PASS. Updated plan/resume/development/
  architecture/decisions; no application code changed or Rust/lint checks rerun.
  Emulator left running headlessly with Yomoku open. Arm64 execution, real async
  JNI and document storage effects remain unverified; default CLI boot uses fallback.
- Next: responsive navigation/session retention, stable IDs/Unicode offsets and
  storage repositories/effects, then CI.

## 2026-10-22 — Responsive navigation and retained Rust session
- Added Rust-owned Settings open/close state alongside four destinations; Back
  returns to the chosen tab. Fresh core starts Library; Settings is a placeholder.
- Added Material bottom bar / >=600dp rail, toolbar Settings/Back, empty screens,
  original vector icons and inset handling. Activity-scoped CoreSessionOwner
  retains the native handle across recreation and closes it on final destruction.
- New Rust navigation policy test and host JNI Settings codec checks PASS.
  Workspace build, 27 Rust tests, fmt/clippy, Kotlin formatting, host JNI and
  git whitespace PASS. App/test APK assembly and strict Android lint PASS.
- Three Android tests PASS at default phone and tablet portrait/landscape viewport
  overrides on API 36 x86_64. Verified native-session identity across recreation,
  final closure, tabs and toolbar/system Back. Restored emulator size/density;
  reinstalled/launched debug APK and visually checked phone bottom bar/tablet rail.
- Initial offline build lacked ui-test-junit4 1.10.6; fetched approved dependencies.
  Fixed generated Kotlin object syntax/import assumptions and a system Back test
  recomposition race. No remaining verification failures. UTP uninstalls app after
  tests; reinstall before manual launch. Screenshots stay ignored under .local/.
- Updated plan/architecture/decisions/development/resume. Split reader navigation
  hiding into an unchecked item pending a real reader route. No persistent schema,
  Nostr configuration or document CRUD behavior changed.
- Next: stable IDs/Unicode offsets, paste document storage/effects/FFI with real
  async JNI checks, then CI. No blocker for local reader development.

## 2026-10-22 — Stable document IDs and Unicode offsets
- Added pure positive i64 document/source/chapter IDs, exact chapter text snapshots,
  UTF-8 byte ranges/anchors and checked UTF-16 conversion. Invalid numeric IDs,
  split UTF-8/surrogate positions, reversed ranges and wrong ownership are rejected.
- Extended generated Kotlin payload types and host runner with seven bidirectional
  Rust/Kotlin Unicode fixtures, including supplementary kanji/emoji, combining/ZWJ
  sequences, empty text, CRLF/NUL and IDs above 2^53 through i64::MAX. Kotlin-created
  suffix anchors validate in Rust. Production native methods/events stay unchanged;
  real document JNI/async callbacks remain pending storage effects.
- PASS: locked/offline workspace build, 32 Rust tests (11 storage), fmt/clippy,
  host Kotlin/JNI + seven payload exchanges, Kotlin formatting, shell syntax and
  whitespace. Repeated Kotlin payload generation matches byte-for-byte. Android
  app/test APK assembly and strict lint PASS offline. Device tests not rerun;
  prior navigation/lifecycle tests remain the baseline. No new verification failure.
- Updated data model/allocation contracts, architecture, decisions, development,
  checklist and handoff. No new dependency versions, SQL migration or Nostr data.
- Next: immutable paste repositories/first-line titles, transaction rollback and
  ID nonreuse/reopen tests; runtime/FFI/private Android DB with async checks; CI.

## 2026-10-22 — Immutable paste persistence
- Added pure PasteText validation/literal first-line titles and PastedDocument
  snapshots. Canonical Unicode, hard breaks, spaces and NUL remain exact.
- Appended migration 0002: separate source/chapter provenance, live documents
  and chapter text with positive AUTOINCREMENT IDs. Typed create/load repository
  commands commit atomically and distinguish absent from inconsistent documents.
  SQL remains outside core; no content-edit, production rename or deletion API.
- PASS: locked/offline workspace build, 44 Rust tests (21 storage), fmt/clippy,
  host Kotlin/JNI + seven Unicode payload exchanges, Kotlin formatting and
  storage builds for aarch64/x86_64 Android. No APK/lint/device rerun this slice;
  existing navigation/device results remain the baseline.
- Ten new storage tests cover exact text/anchors/reopen, per-insert and deferred
  commit rollback, concurrent writes, full-width IDs/nonreuse/exhaustion, real
  migration failure preservation and title/provenance separation. Two new core
  tests cover titles and blank validation. Initial old corruption fixture failed
  on multi-row UNIQUE constraints; scoped updates to version 1, full rerun PASS.
- Updated plan/data model/architecture/decisions/development/handoff. No new
  dependency versions, Nostr data or product deletion semantics. Runtime/FFI and
  Android private DB wiring remain pending; current APK creates no database.
- Next: off-main-thread storage effects/events/FFI and real async JNI/device
  persistence/failure checks; logical pages/reflow; CI. No local-reader blocker.

## 2026-10-22 — Runtime storage effects and Android persistence
- Added pure create/load/list effects, Library summaries and busy/error snapshots;
  internal storage results cannot be injected over FFI. A FIFO runtime worker
  owns lazy SQLite open/migrations/commands. Only Render crosses back to Kotlin.
- Native constructor now accepts a DB path and reports worker creation failure;
  Android supplies Application.filesDir/yomoku.sqlite3 and posts async renders on
  main. Retained lifecycle/close behavior remains verified. UI is still placeholder.
- PASS: 49 Rust tests, workspace build, fmt/clippy, actual Kotlin/JNI async
  callbacks/errors/Unicode/reopen/close-during-I/O plus seven Unicode exchanges,
  Android APK/test assembly, strict lint, four API 36 x86_64 device tests, Kotlin
  formatting and whitespace. SQLite write-lock fixture proves dispatch and
  navigation remain responsive. Both packaged Android ABIs compile.
- Initial Kotlin assertions expected underscored enum names; corrected to generated
  EMPTYPASTE/NOTFOUND. Resolved Clippy receiver-type complexity with a type alias.
  Crux logs discarded weak-reference callbacks during intentional early close;
  JNI/lifetime checks pass. No remaining failure or local-reader blocker.
- Updated checklist, architecture/data model, decisions, development and handoff.
  No new dependency version or migration. No rename/delete, complete import,
  paste UI, reader, tokenizer, progress or Nostr behavior is claimed.
- Next: pure logical pages/chapter boundaries/reflow; Android paste/Library/reader
  slice; CI for verified commands.

## 2026-10-22 — Logical pages and exact-offset reflow
- Added pure core pagination with five presets/default 500, paragraph boundaries,
  long-paragraph following-sentence fallback and explicit chapter isolation.
  TXT headings never infer chapters; ranges cover canonical text exactly.
- User clarified mixed text counts all non-whitespace Unicode characters,
  including punctuation/Latin. Recorded scalar counting and routine paragraph,
  sentence, empty-chapter and EOF choices; updated the product source of truth.
- ReadingPosition retains source/chapter/exact UTF-8 offset through reflow,
  separate from the new page index. Invalid ownership, partial scalars and
  out-of-range offsets fail without changing the original layout.
- PASS: locked/offline workspace build and 63 Rust tests (14 new pagination),
  fmt/clippy, real Kotlin/JNI storage/lifecycle regression and seven Unicode
  exchanges, Kotlin formatting and whitespace. Removed an initial unused
  assignment warning. Existing intentional-close Crux diagnostics remain benign.
- Android APK/lint/device checks not rerun for this pure-domain slice; previous
  four device tests remain the baseline. No dependencies, migration or FFI schema
  changed. Reader app/Settings/Android wiring and progress persistence remain next.
- Updated plan, architecture/data model, decisions, development and handoff.
  No blocker. Next: Android paste/Library/reader opening and navigation hiding;
  page controls/current position persistence; CI.

## 2026-10-22 — Android paste and first-page reader opening
- Added Rust-owned Main/Paste/Reader routes and typed ReaderPage/ReadingPosition
  payloads; OpenReader derives the exact first default-500 page and counter.
  Leaving the route prevents late successful loads reopening the reader.
- Android now has a multiline paste form, exact-title Library list and centered
  scrollable first-page reader. Save returns to Library; opening is explicit.
  Busy/errors/retry and toolbar/system Back use core state. Main navigation hides
  while reading; unsaved drafts survive Settings, recreation and storage errors.
- PASS: 67 Rust tests (four new reader-flow tests), workspace build, fmt/clippy,
  real host Kotlin/JNI/Unicode/lifecycle, Kotlin formatting, Android app/test APK
  assembly, strict lint and all five API 36 x86_64 instrumentation tests on phone
  and tablet landscape. Restored emulator size/density after the second run.
- Corrected initial test constructor/offset API assumptions and the private-event
  rejection ordinal after public event additions. All reruns pass; existing
  intentional-close Crux and formatter warnings remain benign. No blocker.
- Updated plan, architecture/data model, decisions, development and handoff.
  Split current-progress work from the completed paste/open task; it stays
  unchecked. Page movement/swipes, durable position/percentage, page Settings,
  completion/history, dedup and vocabulary registration remain future work.
  No dependencies, migrations, Nostr configuration or destructive APIs changed.

## 2026-10-23 — Durable current reader position and Library percentage
- Started clean. Added migration 0003 and atomic position save/load with source/
  chapter ownership, UTF-8 scalar/EOF validation and insert/update/commit rollback.
  Existing version-2 text upgrades unchanged; invalid stored positions are rejected.
- Added typed core/runtime save hook; committed snapshots drive Library percentage
  and reader page projection while retaining the exact offset separately. Late
  saves update Library without reopening a closed route. No seek UI or forward
  completion is implemented; Next/Previous/swipes remain unchecked.
- Android shows current percentage and uses transient page-keyed vertical scroll.
  PASS: 75 Rust tests (eight new), build/fmt/clippy, host JNI save/reopen/errors/
  lifecycle, seven Unicode exchanges, Kotlin formatting, app/test APK assembly,
  strict lint and all six Android tests on API 36 x86_64 phone/tablet landscape.
  Restored viewport; emulator left running. Arm64 execution remains untested.
- Corrected initial JNI fixture offset inside a Japanese title scalar; complete
  rerun PASS. Existing intentional-close/formatter warnings remain benign.
- Updated checklist, architecture/data model, decisions, development and handoff.
  No dependencies, Nostr data or production deletion APIs changed. No blocker.
  Next: atomic completion/analysis/vocabulary prerequisites and page controls,
  Settings/reflow persistence, CI.

## 2026-10-23 — Persisted page-length Settings and exact-position reflow
- Started clean. Completed the independent Settings slice while forward navigation
  awaits atomic completion/analysis/vocabulary prerequisites. Migration 0004 stores
  the five global presets/default 500 without changing canonical text or positions.
- Added typed core/runtime settings saves and commit-driven reflow of the active
  reader at its exact UTF-8 offset. Unknown/busy selections are disabled; failed
  writes retain the prior preset/page and allow retry. Late results do not reopen
  routes. Android Settings has accessible radio choices and storage errors/retry.
- PASS: 81 Rust tests (six new; 30 storage), build/fmt/clippy, host Kotlin/JNI with
  settings/reopen, seven Unicode exchanges, Kotlin formatting, app/test APK builds,
  strict lint and all seven Android tests on API 36 x86_64 phone/tablet landscape.
  Restored viewport; emulator left running. Arm64 execution remains untested.
- Fixed initial PluralsCandidate lint error with plural resources; rerun PASS.
  Resumed after a server restart by inspecting saved work/build reports. Existing
  intentional-close Crux and toolchain warnings remain benign. No blocker.
- Updated plan, architecture/data model, decisions, development and handoff.
  No new dependencies, Nostr data or destructive APIs. Next: completion IDs/history,
  real Japanese analysis/vocabulary prerequisites for navigation, and CI.

## 2026-10-23 — Repository completion IDs and atomic page history
- Started clean. Added migration 0005 with database-issued command IDs and dated,
  versioned page anchors retaining source/chapter provenance after live deletion.
  Pure core validates the exact departing page/position; SQLite records history
  and advances current position in one transaction. No dependencies were added.
- Identical retries count once and never move progress, including after reopen,
  backward movement, reflow or deletion. Conflicting/stale payloads error. Rereads
  allocate fresh IDs; position-only saves add no history. Event/position/commit
  failures roll back together and permit retry with the same command.
- PASS: 91 Rust tests (ten new; 40 storage), workspace build/fmt/clippy, host JNI,
  seven Unicode exchanges, Kotlin formatting, app/test APK builds, strict lint and
  all seven API 36 x86_64 phone tests. Viewport stayed 1080x2400/420dpi; emulator
  left running. No tablet rerun; arm64 builds pass but execution is untested.
- Fixed clippy's initial test-vector initialization finding; rerun PASS. Existing
  intentional-close Crux and formatter/JDK warnings remain benign. No blocker.
- Updated checklist, architecture/data model, decisions, development and handoff.
  Completed only the repository foundation; core/runtime/FFI navigation wiring,
  encounters/New→Known/time and finished-book behavior remain unchecked. Next:
  real Japanese analysis/surface anchors/vocabulary prerequisites, then extend
  completion atomically and wire controls; CI remains independent follow-up.
  No Nostr data or production destructive APIs changed.

## 2026-10-23 — Global vocabulary repositories and status history
- Started clean. Completed the independent phase-3 persistence prerequisite with
  migration 0006, positive nonreused VocabularyIds, exact global spelling identity
  and versioned, dated registration/manual-status events. No dependencies added.
- Registration batches commit atomically, preserving every existing status/date.
  Lookup and unchanged selections add no events. Status/current-state and history
  updates commit together; statement and deferred-commit failures roll back.
  Concurrent registration/identical status selections record once. Document
  deletion preserves global vocabulary; test-only vocabulary deletes cascade
  only that entry's status history and later registration gets a fresh ID.
- PASS: 102 Rust tests (11 new; 51 storage), workspace build/fmt/clippy, host JNI,
  seven Unicode exchanges, Kotlin formatting, app/test APK builds, strict lint and
  all seven API 36 x86_64 medium_phone tests. No viewport changes; emulator left
  running. No tablet rerun; arm64 packaging passes, execution remains untested.
- Fixed the new migration test's initial nonexistent pagination API reference;
  full rerun PASS. Existing shutdown/formatter warnings remain benign. No blocker.
- Updated plan, architecture/data model, decisions, development and handoff.
  Repository methods have no application/FFI/UI entry point. Atomic document-import
  registration and analyzer dictionary-form links remain unchecked; completion
  still needs analyzed occurrences/New→Known/time before exposing forward controls.
  Next: verify Sudachi Full, token/source anchors/reversible override schema and
  atomic import registration, then completion wiring; CI remains independent.
  No Nostr data, production deletion or data-destructive semantics changed.

## 2026-10-23 — Analysis anchors and reversible boundary schema
- Started clean. Completed phase-3's independent anchor/schema prerequisite:
  pure core validates exact original token surfaces, source/chapter UTF-8 spans,
  optional analyzer readings/dictionary forms and versioned A/B/C snapshots
  (default B). Gaps may omit whitespace only. No tokenizer or dependency added.
- Migration 0007/repositories preserve immutable base snapshots and separate
  global/local split/merge partitions. Identical retries reuse IDs; replacement
  and reversal commit atomically without editing base output, text, vocabulary
  status or history. Reversal never resurrects superseded edits. Global rules
  retain provenance/reversibility after originating-content deletion.
- Inspected upstream Sudachi API/license/Full-format evidence; documented mixed
  hosted API versions and 0.7/V1 compatibility. An exact engine/Full artifact,
  checksum/notices, real tokenization fixtures and device measurement remain
  unchecked. New tests deliberately use synthetic adapter output.
- PASS: 117 Rust tests (15 new; 61 storage), workspace build/fmt/clippy, host JNI,
  seven Unicode exchanges, Kotlin formatting, both-ABI app/test builds, strict
  lint and all seven API 36 x86_64 phone tests. No tablet rerun; arm64 execution
  remains untested. No viewport changes; emulator left running.
- Fixed one old migration expectation from 6 to 7; full Rust rerun PASS. First
  connected-test attempt had no device; started the documented swangle/KVM AVD
  and reran successfully. Existing shutdown/formatter warnings remain benign.
- Updated plan, architecture/data model, decisions, development and handoff.
  No new app/FFI/UI hook. Cross-occurrence matching/overlap resolution, actual
  Sudachi integration and atomic import registration remain next, followed by
  occurrences/New→Known/time completion and controls. CI remains independent.
  No Nostr data, production destructive APIs or outstanding blocker.


## 2026-10-23 — Pinned Sudachi Full host adapter
- Started clean. Added separate yomoku-analysis using Sudachi.rs v0.7.0 pinned by
  commit and Full 20261015 V1 pinned by official archive SHA-256 and data hash.
  Explicit Python preparation retains complete LEGAL/Apache notices, verifies
  all bytes and never replaces existing resources. Large data remains ignored.
- Default B and explicit A/C produce validated exact original surfaces/UTF-8
  anchors with optional resolved readings/lemmas. Japanese script/POS filtering
  excludes non-Japanese/punctuation; OOV rare kanji remain Japanese without
  fabricated dictionary forms. Provisional sentence boundaries handle long
  chapters. No core SQL/I/O, schema, runtime/FFI/UI or import behavior changed.
- PASS: 119 data-free Rust tests plus nine separately invoked actual Full tests,
  locked/offline workspace build/fmt/clippy, host JNI/seven Unicode exchanges,
  Kotlin formatting, both-ABI existing app/test APK builds and strict lint.
  Offline/repeated preparation, corrupt-archive preservation and Python/shell
  syntax pass. No instrumentation/device measurement rerun; no emulator changes.
- Initial compile corrected sha2-0.11 hex formatting and surface dereferencing.
  Initial data fixtures corrected input byte length/expected Tokyo segmentation;
  rare-kanji fixture exposed fallback OOV symbol classification and is now fixed.
- BLOCKER: both analyzer Android target checks fail upstream plugin/loader.rs:92
  (E0425 make_system_specific_name undefined; platform cfg omits Android).
  Existing app remains buildable because it does not link the adapter. Do not
  claim analyzer device support. Next resolve with pinned fix/documented patch,
  bundle/measure Full (currently owned 315 MiB bytes before engine structures),
  then compose atomic analyzed import/registration and complete reading events.
- Updated checklist, architecture/data model, decisions, development and handoff.
  Bundling, automatic conjugation/override application, import/completion/UI stay
  unchecked. Nostr configuration and destructive semantics remain unchanged.

## 2026-10-23 — Sudachi Android compile unblock

- Started clean. Preserved the exact v0.7.0 engine commit and all dependency
  versions. Root Cargo source patch now selects a dependency-only vendored copy
  with Android added to the existing `.so` loader cfg. No runtime/FFI/UI changes.
- Retained upstream resources/source/license/README, complete three-file patch
  and every original file hash. Added offline source verification by reversing
  the patch in a temporary copy; explicit Full tests run it automatically.
  Engine profile includes +android-loader-v1; no migration or rescan.
- PASS: both Android target checks and library builds; workspace build and 119
  data-free Rust tests; nine real Full fixtures; fmt/clippy; host Kotlin/JNI and
  seven Unicode exchanges; Kotlin formatting; Python/shell syntax; all 138
  source hashes and rejection of altered/missing/extra files/divergent patch.
- No APK, instrumentation or device measurement rerun; analyzer still has no app
  entry point. Existing intentional-close Crux logs/JDK formatter warnings remain
  benign. No outstanding check failure or new blocker; emulator untouched.
- Initial staged checks found upstream trailing spaces and the general build-dir
  ignore hiding dic/build source. Scoped attributes preserve original bytes; a
  narrow ignore exception includes all 138 files, verified against staged bytes.
- Next: bundle Full with app-accessible complete notices and measure device
  startup/memory/tokenization/offline use; atomic analyzed import/registration;
  real completion occurrences/status/time before reader Next/Previous/swipes.

## 2026-10-23 — Android Full dictionary bundling and offline notices

- Started clean. Split the next packaging/measurement task into independently
  verifiable steps. Android Gradle now requires prepared pinned Full data and
  calls the Python tool's offline-only --android-assets mode. It verifies all
  inputs, stages/reverifies the dictionary and both complete notices, then
  packages generated assets. Missing/corrupt inputs preserve prior outputs.
- Settings displays attribution, complete LEGAL and Apache-2.0 license in a
  selectable, scrollable dialog, reading off the main thread and retaining its
  open state on Activity recreation. No analyzer/runtime/FFI entry point, eager
  dictionary loading, private-file copy, schema or dependency changes.
- PASS: app/test APK assembly, strict lint, APK entry SHA-256 and 16 KB ZIP
  alignment, Kotlin formatting, Python syntax, offline/repeated preparation and
  bundling, negative missing/corrupt-input checks, 119 data-free Rust tests,
  fmt/clippy, source integrity and nine real Full-data fixtures. Host JNI not
  rerun. Debug APK 184,525,421 bytes; dictionary entry compresses to 146,067,671.
- PASS: all nine API 36 x86_64 phone tests and the two new tests on tablet
  landscape in airplane mode, covering installed bytes and complete notices/
  recreation/dismissal. Restored phone 1080x2400/420dpi and airplane mode off;
  emulator left running. XML copied under ignored .local/test-reports/
  sudachi-bundling. Both native ABIs package; no arm64 execution.
- Tablet commands initially hit sandbox ADB/Gradle socket restrictions; reran
  with escalation successfully. No outstanding failures. Analyzer initialization,
  memory, tokenization and offline analysis still require device measurement
  before runtime integration, then atomic analyzed import/registration and real
  completion occurrences/status/time. Nostr/Q188 remain unchanged.

## 2026-10-23 — Standalone Android Full analyzer measurements

- Started clean. Added a Rust probe and Python CLI using the real pinned adapter
  outside app/runtime/FFI. Driver verifies source/data, links with NDK r30/API 29,
  records fresh-process timing/kernel memory and offline fixtures, restores and
  verifies prior radio settings, and removes its unique device directory.
- Three release + three debug API 36 x86_64 processes PASS: A/B/C golden results,
  mixed-script/OOV/normalization/Unicode/NUL anchors and 72 KB unbroken input.
  Release init 1.03–1.20 s; debug 12.31–12.69 s. Approximately 318 MiB anonymous
  RSS after init, 334–336 MiB peak RSS through long analysis. Exact per-run table,
  method and limitations are versioned in SUDACHI_MEASUREMENTS.md; raw reports
  under ignored .local/test-reports/sudachi-android/. No cold-storage/app claim.
- PASS: both Android release probe links, 119 data-free Rust tests, nine real Full
  host fixtures, 138-file source integrity, fmt, strict all-target/all-feature
  Clippy, Python syntax and final one-process auto-device/source-verification
  smoke. No Android APK/instrumentation/lint or host Kotlin/JNI rerun: no app,
  bridge, schema, dependency or product behavior changes. Physical arm64 untested.
- Initial sandbox ADB failure reran with escalation. Fixed missing target C
  compiler/archiver and probe-only assumption about a rare-kanji token adjacent
  to NUL; switched to existing verified host fixtures. All later checks pass.
  Failed local reports retained. Emulator phone viewport/radios restored and
  verified; app data untouched. No outstanding check failure.
- Next: compare verified immutable mapped storage/private asset extraction and
  combined app memory before runtime integration; atomic analyzed import/unique
  registration; real completion occurrences/status/time before reader movement.
  Nostr/Q188 remain unchanged.

## 2026-10-23 — Verified mapped Full loader and Android comparison

- Started clean. Added explicit unsafe mapped loader with the same checksum and
  profile as safe owned loading. Verifies the mapping actually held by Sudachi;
  callers must prevent backing-file changes through drop. Pins already locked
  memmap2 0.9.12 directly; no dependency graph versions change.
- All ten real Full fixtures pass with each loader, including complete A/B/C
  output parity/readings/anchors/long input after closing the mapped file handle.
  The data-free corrupt-input fixture rejects unpinned bytes with both loaders.
- Extended probe/driver with storage selection and fresh owned/mapped pairs,
  isolated read-only published files and labelled reports. Twelve offline API 36
  x86_64 processes pass (three pairs each in debug/release). Release mapped init
  0.932–0.944 s; anonymous RSS ~2.5 MiB versus ~318 MiB owned. Initial total RSS/
  peak stay similar after checksum reads all mapped pages. Exact per-run tables
  and limits in SUDACHI_MEASUREMENTS.md; no reclamation/app-viability claim.
- PASS: 119 data-free Rust tests, workspace build, source integrity, all twenty
  Full fixture executions, fmt/strict Clippy, Python/shell syntax, whitespace and
  both release Android probe links. No APK/Compose instrumentation, Android lint
  or host Kotlin/JNI rerun; no app/runtime/FFI/schema changes.
- First measurement stopped before analysis on asynchronous mobile-data state;
  bounded polling now verifies offline settings/restoration. Both complete reruns
  pass; original radios restored/verified, temporary device files removed. Emulator
  viewport/app data untouched; physical arm64 execution remains untested.
- Next: on-demand verified immutable private asset extraction/failure recovery,
  combined app/lifecycle memory before runtime integration; atomic analyzed import/
  unique registration; completion occurrences/status/time before reader movement.
  Nostr/Q188 remain unchanged.

## 2026-10-23 — On-demand private Full extraction and recovery

- Started clean. Added a Kotlin platform helper for worker-only, lazy extraction
  from the compressed APK to SHA-256-named no-backup private files. Streams with
  bounded buffers; verifies pinned size/on-disk bytes, syncs, publishes read-only
  bytes by atomic rename and returns the same verified descriptor at offset zero.
  JVM/process publication locks serialize callers. Repair/cleanup never writes
  or truncates a published inode, preserving existing readers/mappings.
- Eight new Android tests verify real Full extraction/cache reuse, concurrent
  requests, worker-only access, interrupted/corrupt assets, abandoned staging,
  failed rename/retry, descriptor permissions and held-reader/mapping preservation.
  Two instrumented x86_64 processes: extraction+verification 2.273–3.273 s;
  verified reopen 0.632–0.689 s. Exact results/limits in SUDACHI_MEASUREMENTS.md.
- PASS: both-ABI app/test APK assembly, strict Android lint, all seventeen phone
  tests, final eight focused tests after tightening the interrupted-stream
  fixture, Kotlin formatting and whitespace. Reports retained under ignored
  .local/test-reports/sudachi-extraction/. No tablet, Rust/host JNI or native probe
  rerun for this platform-only slice. No dependencies, schemas or FFI changes.
- Initial compile used non-public O_DIRECTORY; checked installed SDK signatures,
  switched to public O_RDONLY and verified directory sync on device. Final focused
  run first hit sandbox sockets, then a disconnected emulator; escalation and
  documented swangle restart resolved both. No remaining check failure.
- Test-private files removed; viewport/radios unchanged, emulator left running.
  No runtime/startup/analyzer caller yet. Combined app memory, analyzer worker
  reuse/lifecycle/low-memory and physical arm64 remain gated. No actual power-loss,
  ENOSPC or cross-process contention claim. Next: in-app descriptor/analyzer
  measurement harness; atomic analyzed import/registration; real completion.
  Nostr stays empty; Q188 remains before forwarding.

## 2026-10-23 — Debug-app Full descriptor, reuse and lifecycle measurements

- Started clean. Added a separate measurement-only JNI cdylib example and debug
  facade/build task for both ABIs, with release-optimized Rust and no new
  dependencies/versions. The existing runtime/BoltFFI/core and schemas stay
  unchanged. Unsigned release APK inspection excludes the facade and library.
- The probe duplicates the verified private descriptor directly, rehashes the
  mapping and keeps one thread-local analyzer. Positive generations reject stale/
  foreign-worker calls; load/validation/panic failures stay inside the boundary.
  Instrumentation validates actual Full A/B/C, mixed/OOV/normalization/Unicode/
  NUL, repeated pages and 72 KB input after Kotlin descriptors close.
- A test-only ViewModel retains a lazy worker across real Activity recreation/
  stop/resume with one initialization, closes on clearance and confirms actual
  mapping removal before cleanup. Fresh-worker reopen and invalid-load retry pass.
  Production analyzer lifecycle/reuse remains a decision after the loading gate.
- Two final API-36 x86_64 processes measure extraction+map/init 2.965–3.295 s and
  cached verify+map/init 1.974–3.306 s. Combined app PSS ~403–409 MiB initialized,
  ~89–98 MiB after close. Exact tables/method/limits in SUDACHI_MEASUREMENTS.md;
  no kernel memory-pressure/process-death/cold-storage/physical-arm64 claim.
- PASS: focused probe and all eighteen Android phone tests, strict lint, both-ABI
  debug/test and unsigned release APK builds, probe exclusion from release,
  16 KB ELF/ZIP alignment, pinned source integrity, workspace build/119 Rust
  tests, fmt/strict Clippy, Python syntax, Kotlin formatting and whitespace.
  No host JNI, host Full-fixture, tablet or standalone-probe rerun. Reports:
  .local/test-reports/sudachi-app/{focused-1,full-suite}/.
- First connected Gradle run needed socket escalation. Added map assertion first
  assumed Context and /proc used identical private-directory paths; unique test
  suffix matching fixes it and both final processes pass. No open check failure.
  All test mappings/workers close before private-file cleanup; viewport/radios
  unchanged, emulator left running. Next: pressure/process-death/arm64 gate,
  atomic analyzed import/registration, real completion before reader movement.
  Nostr remains empty; ask Q188 before forwarding.

## 2026-10-23 — Full app process-death recovery

- Started clean. Added a debug-only ADB Activity protected by system DUMP and an
  external Python driver; no new dependency, production startup/runtime/FFI or
  schema changes. MainActivity/Compose/CoreSession share the probe process.
- Two API-36 x86_64 force-stop/restart pairs kill a live verified Full mapping
  without graceful close. New PIDs reverify the same published inode/mtime/
  size/read-only mode and pass A/B/C/Unicode/page/72 KB analysis. Cached recovery
  takes 2.704–3.012 s; PSS remains about 419 MiB after checksum/validation. Exact
  results/limits are in SUDACHI_MEASUREMENTS.md. Pressure/LMKD and arm64 remain open.
- Cleanup confirms every app process exits before deleting only its unique probe
  directory; user data and viewport/radios stay unchanged, ordinary Library
  reopened. Failure diagnostics/report preservation and bounded waits included.
- PASS: two death/recovery pairs, all eighteen Android tests, strict lint,
  debug/test and unsigned release APK builds, manifest/DEX/native-library probe
  exclusion from release, Kotlin formatting, Python syntax and whitespace.
  Reports: .local/test-reports/sudachi-process-death/{20261023T194512308957Z.json,full-suite/}.
  No Rust/host JNI/host Full fixtures/standalone/tablet rerun for this platform
  measurement slice. Initial ADB sandbox failure resolved with escalation. A
  post-suite driver rerun found instrumentation had uninstalled the app; added
  early prerequisite diagnostics and reinstalled debug before final verification.
  Both final driver pairs pass: report 20261023T194946330811Z.json in that directory.
- Checked only the verified process-death subtask; broader Full gate stays open.
  Next: actual pressure/reclamation and production lifecycle decision, physical
  arm64, then atomic analyzed imports and completion semantics before navigation.
  Nostr stays empty; ask Q188 before forwarding.


## 2026-10-24 — Atomic analyzed-paste repository

- Started clean. Added `Storage::create_analyzed_paste`, composing private paste,
  analysis and registration helpers in one immediate transaction. Caller supplies
  a preloaded/reused analyzer callback against exact allocated chapter identities;
  typed analyzer diagnostics remain separate from storage/validation failures.
- Validated Mode B tokens and all unique Japanese original surfaces persist with
  text and dated New registrations. Existing statuses/IDs/dates/history survive;
  no encounters, position, completion or deduplication. No schema/dependency or
  production Android/runtime/FFI changes; broader import task remains unchecked.
- Six host storage tests cover repeated/Unicode/non-Japanese tokens, preservation
  of all statuses, concurrent imports, analyzer/invalid output, every write stage,
  deferred commit failure, retry/reopen and isolation from another connection.
  New real Full composition fixture preserves OOV/half-width surfaces and Known.
- PASS: workspace build, 125 data-free Rust tests (67 storage), fmt/strict Clippy,
  eleven Full fixtures per owned/mapped loader, pinned source integrity and host
  Kotlin/JNI plus seven Unicode exchanges. Initial Full fixture incorrectly
  assumed a half-width compound was a single B token; corrected the assertion to
  validate original spans across actual tokens; both full suites pass. Prior
  nonfatal JNI weak-reference shutdown logging remains. No open failure.
- No Android/Gradle/device/standalone/tablet rerun or emulator/user-file changes.
  Next: actual Full pressure/reclamation and production worker lifecycle/arm64
  gate, production atomic analyzed import plus dedup, then occurrence/time-aware
  completion before reader navigation. Nostr empty; ask Q188 before forwarding.

## 2026-10-24 — Exact-byte paste deduplication repository

- Started clean. Added migration 0008 and `Storage::import_analyzed_paste` with
  explicit Created/Existing outcomes. SHA-256 covers exact pasted UTF-8 bytes;
  reused pinned sha2 0.11.1 after checking primary API documentation.
- Duplicate imports reference the oldest live match, skip analysis/registration
  and preserve title/progress/status/history, including deleted vocabulary.
  Preserve legacy duplicates; lazily hash unindexed pastes under the same write
  transaction. Concurrent identical submissions analyze/create once. New content,
  tokens, vocabulary, hash and legacy backfill roll back together on any failure.
- Nine storage tests verify Unicode/NUL/line-ending identity, known SHA-256,
  concurrency, reopen, legacy upgrade/indexing, every write/deferred commit,
  analyzer/validation/indexing failure, corrupt match and deletion/reimport with
  retained history/vocabulary. Real Full import fixture now verifies dedup too.
- PASS: workspace build, 134 data-free Rust tests (76 storage), fmt/strict Clippy,
  eleven real Full fixtures per owned/mapped loader, pinned source integrity and
  host Kotlin/JNI plus seven Unicode exchanges. Prior nonfatal weak-reference
  shutdown logging remains; no open failure. No Android/Gradle/device/standalone/
  tablet rerun, emulator changes or production runtime/FFI/Android wiring.
- Checked only the paste repository subtask; file-byte identity/imports and app
  integration remain unchecked. Next: Full pressure/reclamation and worker
  lifecycle/arm64 gate, deduplicating analyzed import integration, then atomic
  occurrences/New→Known/time before navigation. Nostr empty; Q188 before forwarding.

## 2026-10-24 — Pure active-time accounting

- Started clean. Continued the independent reader-time prerequisite while the
  Full pressure/lifecycle/hardware gate remains open. Added pure `active_time`
  domain values and accumulator with caller-supplied monotonic milliseconds.
- Five-minute idle cap, foreground reader/dictionary interactions, background/
  close pause, duplicate lifecycle signals and duration-only paused restoration
  are covered. Checkpoints never reset idle or drain time; clock regression and
  overflow fail without mutation. No new dependencies, schema or FFI payloads.
- PASS: thirteen new domain tests; all 147 data-free workspace Rust tests (76
  storage), workspace build, fmt, strict Clippy and whitespace. Restore/reboot
  cases are simulated at the saved-duration boundary; no actual persistence,
  Android lifecycle or device reboot is claimed. Eleven Full-data tests remain
  ignored in the workspace run; no Full/Kotlin/JNI/Android rerun for this slice.
- Split the plan's pure-accounting subtask from the still-unchecked durable
  pending-time/platform/atomic-completion work. Updated architecture, data model,
  decisions, development commands and handoff. No emulator/user-file changes.
- Next: Full pressure and production worker lifecycle/arm64 gate, analyzed import
  wiring, then pending-page provenance/reflow and occurrence/status/time-aware
  atomic completion before reader controls. Nostr empty; Q188 before forwarding.

## 2026-10-24 — Physical-device validation deferred by user

- User will test their physical device once the app is usable. Removed physical
  arm64 from the runtime-integration gate and tracked it as a later user-owned
  validation task. Emulator pressure/reclamation and production worker lifecycle
  remain the next integration prerequisites; no hardware pass is claimed.
- Updated plan, handoff, architecture, measurement scope and decision record.
  Documentation only; whitespace and consistency checks pass. No build/tests or
  device actions rerun; working tree was clean at start.

## 2026-10-24 — Full emulator reclamation gate

- Started clean. Added bounded NDK shell pressure helper and emulator-only Python
  driver; extended only the debug Activity to revalidate on its original worker.
  No production runtime/core/schema/dependency changes. Pressure uses anonymous
  pseudorandom bytes, not trim signals/cache drop/madvise. Bounds/EOF/watchdog and
  process-exit-before-file-removal cleanup keep the experiment controlled.
- Two API-36 x86_64 runs reclaim 68,408/146,792 KiB of dictionary residency at
  640/896 MiB helper allocation. File scan/steal counters increase; same PID and
  analyzer generation pass A/B/C, Unicode/OOV/NUL, page and long-input fixtures
  after release. PSI denied by device policy; no LMKD/hardware/stall-time claim.
- Resolved the emulator loading gate. Chose lazy immutable mapping on the existing
  FIFO Rust worker, foreground/configuration reuse and idle cache release on
  UI-hidden/background/final session close after accepted atomic work. Production
  descriptor delivery/lifecycle/import wiring is next, not implemented here.
- PASS: strict helper compile, Python syntax/bound rejection, Kotlin formatting,
  both-ABI debug/test APK assembly, strict lint, final 18/18 Android tests and two
  actual force-stop/recovery regression pairs; whitespace. No host Rust/Full/JNI,
  unsigned release/tablet/physical-arm64 rerun. No radio/viewport changes.
- Initial pressure attempt lost ADB transport, excluded; original app PID survived.
  Reconnected and confirmed exit before removing exact owned paths. Two successful
  runs clean their files/return to Library. Initial Android suite had one existing
  notices timeout (17/18); focused 2/2 and full 18/18 reruns pass unchanged. Reports:
  .local/test-reports/sudachi-{memory-pressure,pressure,process-death}/. Socket
  restrictions resolved with escalation; no open verification blocker.
- Updated plan, architecture, measurements, decisions, commands and handoff.
  Next: atomic analyzed/deduplicating imports in production, then persisted page
  time/occurrences/New→Known completion before navigation. Arm64 is the user's
  later usable-app trial; Nostr empty and Q188 still required before forwarding.

## 2026-10-24 — Analyzed-import FIFO worker

- Started clean. Added opt-in analyzed-import router, worker-local analyzer factory/
  cache and weak lifecycle control using the existing FIFO; no new executor,
  dependency version, schema, production JNI or Android behavior.
- Created/Existing results contain committed documents; core upserts a Library row
  by ID, preserves current progress and never opens the reader. Runtime dates
  registrations; no active time, encounters or status promotions are created.
- Analyzer loading is lazy, ready before the write transaction and retryable.
  Engine/invalid output drops cache; SQL rollback permits healthy reuse. Hidden
  imports finish then release; queued background/close cannot interrupt accepted
  atomic work. Retained lifecycle handles cannot retain a closed worker/factory.
- PASS: workspace build, 154 data-free Rust tests, fmt, strict Clippy; 11 analysis
  + one runtime Full fixture with each owned/mapped loader; 138-file source check;
  host Kotlin/JNI, seven Unicode exchanges, shell syntax and whitespace. Seven
  new runtime tests cover non-Send worker affinity, dedup/status/progress/reopen,
  all writes/deferred commit rollback/retry and background/close ordering.
- Previous nonfatal JNI weak-reference shutdown messages persist; all assertions
  pass. No open verification blocker. No Android/device/probe/tablet/arm64 rerun.
- Updated plan, decisions, architecture/data model, development and handoff.
  Next: connect the new router via typed native verified-descriptor delivery,
  retained/background Android lifecycle and visible Created/Existing summary.
  Production imports stay unchecked; NativeBridge still uses text-only creation.

## 2026-10-24 — Single-paste import summary

- Started clean. Added typed transient ImportSummary/ImportOutcome and dismiss
  event in core, plus Android Library confirmation and localized messages.
  Created/Existing show their committed title without opening the reader.
- Settings/refresh/retained recreation preserve confirmation; dismissal, tab
  selection and new paste/reader/import flows clear it. Failed imports preserve
  draft/error with no success summary. Fresh sessions do not restore confirmation.
- PASS: 156 data-free Rust tests, workspace build, fmt, strict Clippy; host
  Kotlin/JNI and seven Unicode exchanges; Kotlin format, both-ABI debug/test APKs,
  strict Android lint and three API-36 x86_64 paste-reader tests. Reports:
  `.local/test-reports/import-summary/paste-reader/`.
- Initial FFI test used a stale event number; derive from final public event.
  Fixed Kotlin smart-cast warning; final reruns pass. Connected Gradle required
  socket escalation. Previous nonfatal JNI shutdown logging persists.
- Updated plan, decisions, architecture/data model, development and handoff.
  Production analyzer/native descriptor/lifecycle and duplicate delivery remain
  unchecked. No schema/dependency changes, Full fixtures, full Android suite,
  release/probe/tablet or physical-arm64 rerun. No open verification blocker.


## 2026-10-24 — Production analyzed paste imports

- Started clean. NativeBridge requires DictionaryProvider and always selects the
  analyzed/deduplicating router. Android transfers owned FD+1; Rust closes it and
  maps/rechecks the same immutable publication on the existing FIFO worker.
- Activity recreation retains warm analysis; background queues release, hidden
  imports finish/release and final close drains accepted work. No eager Library
  load/new executor/schema/version change. Native builds verify pinned Sudachi.
- Host real Full JNI tests cover load failure/retry, exact duplicates/progress,
  summaries, cache reuse/release/reopen and both callback registries' cleanup.
  New data-free native test forbids text-only fallback. New device test verifies
  actual Mode B/vocabulary transactions, SQL rollback/retry, native worker TID,
  preserved duplicate status/history/progress, hidden release and close/drain.
  Paste UI tests cover visible duplicates and actual recreation/background mapping.
- PASS: build, 157 data-free Rust tests, fmt/strict Clippy, eleven analysis + one
  worker Full fixture per loader, pinned 138-file source check, host Kotlin/JNI +
  seven Unicode exchanges, Kotlin format/shell syntax, debug/test APKs and lint.
- Final Android full 19/19 and strict lint PASS. Initial full runs 18/19 and focused
  notices stalled at Loading. Trace showed producer resume on I/O under UI tests;
  explicitly publish on Main.immediate. Original assertions/10s timeout retained;
  focused 2/2 and full 19/19 pass. Temporary diagnostics removed; all reports retained
  under .local/test-reports/analyzed-import/. No open failure.
- Real Full worker/JNI old 10s timeout fixed with 60s only for those loads; data-free
  waits unchanged, Android storage 30s. JNI thread-object assumptions replaced by
  native TID on device. Previous benign Crux shutdown logging persists. Emulator
  disconnected/restarted with swangle, no data reset; Gradle/ADB escalation used.
- Updated plan, architecture/data model, decisions, development and handoff. Next:
  pending-page time/reflow attribution, then atomic completion with occurrences,
  New→Known/time and Android clock/navigation. No standalone/pressure/death probe,
  unsigned release, tablet or arm64 rerun; arm64 remains the later user trial.


## 2026-10-24 — Persist pending time and hand it off atomically

- Started clean. Migration 0009 adds durable duration/revision buckets at exact
  UTF-8 reading offsets plus historical completion.active_time_ms (legacy zero).
  Backward navigation retains pending time; reflow assigns stable anchors once
  to containing pages. No timestamps from the monotonic clock cross restarts.
- Opaque checkpoint tokens permit exact retries and reject stale/conflicting
  saves. Completion freezes/validates a page total, consumes only its buckets,
  advances revisions and commits time/history/position together. Duplicate IDs
  cannot drain later reread time. Zero rows prevent stale-time resurrection;
  consumption skips a revision to distinguish even zero-time checkpoint retries.
- Twelve new storage tests cover all 25 presets, SQLite reopen/fresh clock epoch,
  stale/duplicate/concurrent delivery, rereads/EOF/deletion, overflow and migration/
  statement/commit rollback. Updated historical migration fixtures for schema 9.
- PASS: workspace build, 169 data-free Rust tests (88 storage), fmt/strict Clippy,
  host real-Full Kotlin/JNI + seven Unicode exchanges and whitespace checks.
  No open failure; prior benign Crux shutdown logs persist. No new dependency.
- Updated plan, decisions, architecture/data model, development and handoff.
  Next: occurrences/New→Known in the completion transaction, app/worker/FFI timer
  checkpoints, Android clock/lifecycle/interactions and forward controls/Finish.
  No Full standalone fixtures, Android build/lint/tests, release/probes/tablet or
  arm64 rerun; actual Android close/reboot recovery remains unchecked.

## 2026-10-24 — Repository README

- Started clean. Added a root README describing the app, current versus planned
  features, module boundaries, host checks, Android build/run prerequisites and
  licensing, with links to the detailed project documents.
- Verified local Markdown links/anchors, checked commands and version pins against
  tracked scripts/build files and development docs, and passed git diff --check.
  Documentation only; no Rust/Android/JNI checks rerun or new blocker.
- Updated the continuity checklist and handoff. Next reader tasks remain atomic
  occurrences/New→Known completion, then timer/lifecycle and navigation wiring.


## 2026-10-24 — Atomic analyzed completion occurrences and New→Known

- Started clean. Migration 0010 adds historical completion analysis identity,
  per-token Japanese occurrences with full anchors/readings/lemmas, and dated
  completion-linked New→Known events. The pure core assigns a whole token to
  the page containing its starting offset and preserves all other statuses.
- record_analyzed_page_completion validates the frozen snapshot and commits all
  occurrences, missing departing-page registrations, status changes, time buckets,
  history and position together. Replays compare payload/analysis ID before live
  content and repeat no effects; legacy rows cannot acquire encounters by replay.
- Document deletion retains history; vocabulary deletion cascades its encounter/
  status rows. Actual later completion recreates only absent departing-page entries.
  Status migration retains event IDs and allocation high-water marks even if empty.
- PASS: workspace build, 182 data-free Rust tests (99 storage), fmt/strict Clippy,
  eleven analysis plus one worker Full fixture per owned/mapped loader, 138-file
  source integrity, host real-Full Kotlin/JNI, seven Unicode exchanges and diff
  check. Added eleven storage and two pure tests; extended real Mode B composition.
  Initial test-tuple Clippy complexity fixed with an alias. No open failure;
  previous benign Crux JNI shutdown logging persists. No dependency change.
- Updated README, plan, data model, architecture, decisions, development and handoff.
  Next: core/FIFO/FFI checkpoints and analyzed completion intents, Android monotonic
  lifecycle/interactions, then navigation and Finish Book. No Android build/lint/
  tests, release/probes/tablet/arm64 rerun; actual close/reboot remains unverified.


## 2026-10-24 — Reader analysis identity restoration

- Started clean. Added a read-only reader snapshot with document/exact position,
  preset and original Mode B identity; wired it through core/FIFO worker/generated
  FFI. Earliest stored B stays fixed across settings/position/plain loads. Missing
  explicitly identifies legacy absence; no dictionary loading/rescan/registration.
  Reader exit/replacement clears identity; cancelled loads cannot attach it.
- Added three storage, one core and one worker test; extended FFI/JNI checks and
  two analysis-state codec exchanges (including max i64). Covers original versus
  later profiles, wrong modes/chapters, corrupt metadata/settings, read-only reopen,
  frozen state and actual JNI reopen with an unavailable dictionary provider.
- PASS: workspace build, 187 data-free tests (102 storage), fmt/strict Clippy,
  real-Full Kotlin/JNI + seven Unicode/two analysis exchanges, Kotlin format,
  debug/test APK builds, strict Android lint and diff check. Initial missing test
  import fixed. Prior benign Crux shutdown/JVM Unsafe/SDK XML warnings persist.
- Updated plan, decisions, architecture/data model, development and handoff.
  Next: time checkpoints and analyzed completion intents with frozen retries,
  Android clock/lifecycle/interactions, then controls/Finish Book. No migration,
  dependency or legacy forward policy. No instrumentation, standalone Full suite,
  release/probes/tablet/arm64 rerun. No local blocker.

## 2026-10-24 — Explicit pending-time checkpoint transport

- Started clean. Added typed RestoreReadingTime/CheckpointReadingTime/retry intents
  across core/FIFO worker/generated FFI. Moved the pure saved token to core; SQLite
  still owns validation/I/O. Opening stays read-only; explicit restore reserves the
  current offset's token. Only anchor/duration reaches the view, never a revision.
- Frozen absolute checkpoints retain prior revisions across failure/reader exit;
  unresolved saves block later storage commands. Wrong/decreasing updates ignore,
  negative FFI durations reject, late callbacks cannot attach to closed readers.
  Reflow/same-offset saves preserve time; a changed offset needs fresh restore.
- PASS: workspace build, 191 data-free Rust tests (102 storage), fmt/strict Clippy,
  host real-Full Kotlin/JNI with Unicode/analysis exchanges and negative-duration
  rejection, Kotlin format, both debug APK builds, strict Android lint and diff
  check. Added three core and one async worker/FFI tests; extended JNI/numeric
  checks. Initial expected-view/event-tag failures fixed; no open failure.
- Updated README, plan, decisions, architecture/data model, development and handoff.
  Next: coalesce/finalize checkpoints before movement/close and freeze analyzed
  completion intents; then Android clock/lifecycle/interactions/periodic saves and
  controls/Finish Book. No migration/dependency/legacy forward policy. No device
  instrumentation, standalone Full suite, release/probe execution/tablet/arm64
  rerun; actual Android timer/close/reboot remains unverified. No local blocker.

## 2026-10-24 — Coalesced checkpoints before reader departure

- Started clean. Core coalesces newer same-anchor absolute durations while a save
  is unresolved, preserving the frozen payload/revision for exact retry. Success
  chains the maximum duration with the returned token; equal/decreasing/foreign
  updates cannot overwrite it. One requested reader exit/position change waits
  for all accepted saves; duplicates retain the first departure. Failures retain
  both queued time and departure, with committed view state unchanged.
- Added three core tests and one blocked-SQLite async FFI test covering chained
  updates/retries, exit routes, duplicate destinations, nonblocking dispatch,
  precise revision counts and exact-offset reopen without reading history.
- PASS: workspace build, 195 data-free Rust tests (102 storage), fmt/strict Clippy,
  real-Full Kotlin/JNI + Unicode/analysis exchanges, debug/test APK builds, strict
  Android lint and diff check. Initial test revision expectation corrected; no
  open failure. Prior benign Crux shutdown logging persists. No schema, dependency
  or public-codec change; no Kotlin source changed.
- Updated plan, README, decisions, architecture/data model, development and handoff.
  Next: analyzed completion after checkpoint drain, then platform timer/final saves
  and native teardown acknowledgment, controls/Finish Book. Unrelated busy events
  still ignore; no automatic timer is connected. No device instrumentation,
  standalone Full fixture suite, release/probe/tablet/arm64 execution; actual
  Android close/reboot remains unverified. No local blocker.

## 2026-10-24 — Analyzed completion intents after checkpoint drain

- Started clean. Added anchored CompleteReaderPage/RetryReaderCompletion and
  transient positive CompletionNotice through core/FIFO/generated FFI. Capture
  page/position/preset/AnalysisId/timestamp before checkpoint drain, atomically
  prepare command ID + whole-page durable time, then commit analyzed completion.
  Failures retain the exact stage/payload; other storage and one first-wins
  departure wait. Success reconciles persisted position/progress and retires time.
  AlreadyRecorded suppresses duplicate notices; deliberate reread gets a new ID.
  MissingAnalysis blocks legacy completion without rescan or fallback.
- Added three core tests, one preparation repository test and a blocked-SQLite
  FFI test for checkpoint bursts, rollback, frozen identity/time, duplicate
  callbacks, 250→500 occurrences on reread, New-only promotion and reopened
  position. Real-Full Kotlin/JNI now completes/rereads without dictionary access.
- PASS: build, 200 data-free Rust tests (103 storage), fmt/strict Clippy, host
  Kotlin/JNI + Unicode/analysis exchanges, Kotlin format, both-ABI debug/test APKs,
  strict Android lint and diff check. Initial compile/Clippy/preview omissions
  fixed; Gradle socket denial passed with escalation. No open failure. Existing
  benign Crux/ktfmt/SDK warnings remain. No schema/external dependency version
  changes; one workspace storage dev-dependency seeds the FFI fixture.
- Updated README, plan, decisions, architecture/data model, development and handoff.
  Next: actual Android clock/lifecycle/interactions/final checkpoints and native
  teardown acknowledgment, then controls/swipes/toast/Finish Book. No automatic
  timer/UI forward action yet. No device instrumentation, standalone Full suite,
  release/probe/tablet/arm64 rerun; actual close/reboot remains unverified.
  No local blocker; Nostr configuration remains empty and Q188 remains open.

## 2026-10-24 — Typed core reader-clock driver

- Started clean. Added anchored ObserveReaderClock resume/interact/checkpoint/pause
  events, reusing the pure Rust accumulator after explicit restore. Only absolute
  durations reach SQLite. Idle/paused rules, reflow and fresh epochs remain in core.
- Final checkpoints also wait behind unrelated nonmoving storage in the existing
  maximum slot, drain before completion/departure and retain exact retries. Guard
  late producers behind accepted movement/import and superseded unsubmitted slots.
- PASS: build, 206 Rust tests (five new core, one blocked-SQLite FFI), fmt/strict
  Clippy, real-Full Kotlin/JNI clock/reopen checks plus Unicode/analysis exchanges,
  repeatable generation, Kotlin format, both-ABI debug/test APK builds, strict lint
  and diff check. Initial cleared-retry-slot regression fixed. Prior benign Crux/
  ktfmt warnings persist; no open failure, schema or dependency change.
- Updated README, plan, decisions, architecture/data model, development and handoff.
  Next: Android observation/restore/lifecycle/periodic/final-save producer and native
  teardown acknowledgment, then controls/swipes/toast/Finish Book. No device
  instrumentation, standalone Full suite, release/probe/tablet/arm64 rerun. Actual
  Android close/reboot remains unverified. No local blocker; Nostr empty/Q188 open.

## 2026-10-24 — Android reader clock and draining native close

- Started clean. Connected retained CoreSession to elapsedRealtime, Activity
  resume/pause/interactions, 15-second Handler checkpoints and final Pause before
  settings/movement/completion/exit. Restore waits for ready storage; same-offset
  writes preserve idle. Accounting stays in Rust; no schema/dependency/codec change.
- Close rejects UI dispatch, retains callbacks until busy and both pending flags
  clear, then releases native. Failed obligations after onCleared retry exactly;
  persistent failure retains the session while alive. Reader/Settings expose Retry
  and unresolved saves disable presets. No partial-page history/status changes.
- Added five JNI/SQLite driver tests, two Compose lifecycle/retry checks, a real
  clock close/reopen test and an emulator-only reboot runner. PASS: 206 Rust tests,
  debug/test APKs, full 26 Android tests then final 11 affected tests, strict lint,
  Kotlin format, Python parsing/help and diff checks. Actual process/reboot restored
  exact 259 ms then 268 ms; report/commands in DEVELOPMENT.md. Initial nonblocking
  Android SQLite lock fixture corrected; explicit failure/retry coverage passes.
- Updated README, plan, architecture/data model, decisions, development and handoff.
  Next: Next/Previous/swipes/positive toast, then finished-book behavior. No local
  blocker. No release/tablet/physical-arm64 or host Kotlin/JNI rerun. Emulator left
  running; no OS final-flush guarantee. Nostr configuration empty and Q188 open.

## 2026-10-24 — Android reader navigation and completion toast

- Started clean. Added content-bottom Previous/counter/Next and horizontal swipes
  through retained CoreSession dispatch/Pause. Forward uses frozen analyzed completion;
  new anchored PreviousReaderPage derives the previous page in pure Rust and drains
  final checkpoints before position-only persistence. Pending work disables actions.
- Added positive-count short Toast presentation with synchronous notice consumption.
  Duplicate rendered callbacks count once; deliberate rereads count again. Permanent
  finished/checkmark/Finish Book/confetti remain the next separate slice.
- PASS: 208 Rust tests (two new core), host Kotlin/JNI plus Unicode/analysis exchanges,
  strict Clippy/fmt/Kotlin format, both-ABI debug/test APK builds, strict Android lint
  and all 30 API-36 x86_64 Android tests (three new Compose/JNI/SQLite tests). Covers
  progress in both directions, scroll resets, gestures, duplicate input, SQLite
  failure/exact Retry and toast consumption/zero silence. Initial Kotlin warnings/
  missing import corrected; Gradle socket restriction passed with escalation.
- Updated README, plan, architecture/data model, decisions, development and handoff.
  No schema/dependency change or blocker; generated Event adds anchored Previous.
  No release/tablet/physical-arm64 or separate process/reboot recovery rerun.
  Nostr configuration remains empty; ask Q188 before forwarding.

## 2026-10-24 — Durable finished-at-least-once repository marker

- Started clean. Migration 0011 adds a constrained live-document finish boolean.
  Final-page completion commits it with history/occurrences/status/time/position;
  backward movement, reflow, partial-time saves and retries preserve it. EOF
  position alone never sets it. Deletion/reimport resets it while history stays.
- Backfill uses matching live paste EOF history and exact UTF-8 BLOB lengths.
  Added a read-only Option<bool> getter and six repository tests, extended final-
  page/concurrency/rollback coverage, and updated schema-version fixtures. Gradle
  now tracks embedded SQL in both native build tasks.
- PASS: 214 Rust tests (109 storage), strict Clippy/fmt, real-Full host Kotlin/JNI
  with Unicode/analysis exchanges, Kotlin format, both-ABI debug/test APK builds,
  strict Android lint and all eight ReaderNavigationTest/ReaderClockSessionTest
  regression tests on API 36 x86_64. Fixed one stale schema expectation; Gradle's
  sandbox lock/socket restriction required escalation. No open failure.
- Updated plan, handoff, architecture/data model, decisions, README and commands.
  Next: project marker into reader/Library, checkmark/reopen-at-page-1, then Finish
  Book/confetti. No dependency/codec changes, full Android suite, process/reboot,
  tablet or physical-arm64 rerun. Nostr empty; ask Q188 before forwarding.

## 2026-10-24 — Finish checkmarks and page-1 reopening

- Started clean. Projected permanent finish state through document/Library/reader
  and generated Kotlin/JNI; Android shows accessible checkmarks independent of %.
  Explicit OpenReader atomically persists page 1 for finished books, preserving
  history/pending durations. Plain/completion loads and retained recreation keep
  exact position. Unfinished books retain saved offsets. No schema/dependency change.
- PASS: 217 Rust tests (111 storage), strict Clippy/fmt/Kotlin/diff, real-Full host
  Kotlin/JNI and seven Unicode exchanges, reproducible generation, both-ABI debug/
  test APKs, strict Android lint and 14 distinct API-36 x86_64 tests across runs.
  Two new repository tests, one core test and one Compose test cover reset rollback/
  retry, rereads, marker projections, pending time, EOF reloads and recreation.
- Historical migration fixtures now seed old rows directly. Fixed initial Kotlin
  safe-call warnings and an existing paste-reader offscreen-row assumption; child
  scrolling also failed, then final LazyColumn key scrolling passed the focused
  test. Other 13 selected tests passed twice. Gradle required socket escalation.
- Updated progress/architecture/data docs. No open failure or local blocker.
  Next: Finish Book and per-completion confetti, including rereads/zero newly Known.
  No full suite, release APK, standalone Full suite, process/reboot, tablet or
  physical-arm64 rerun. Nostr stays empty; ask Q188 before forwarding.

## 2026-10-24 — Finish Book and per-command confetti

- Started clean. Final-page Finish Book/left swipe retains anchored completion and
  final checkpoint drain, then returns to Library only after success. Failures keep
  Retry. Separate transient command IDs celebrate every newly Recorded final page,
  including rereads/zero New; AlreadyRecorded stays silent. Positive toast survives
  automatic return. Compose consumes the key before a local 1.4s confetti overlay;
  recomposition/recreation cannot replay it. No schema/dependency change.
- PASS: 219 Rust tests (111 storage), strict Clippy/fmt/Kotlin/diff, real-Full host
  Kotlin/JNI + seven Unicode exchanges, reproducible generation, both-ABI debug/test
  APKs, strict Android lint and all 15 selected API-36 x86_64 tests (5m42s build/run).
  Two new core tests and one animation test; amended real JNI/SQLite finish/reread
  and final-clock/draining-close tests verify retry/dedup/recreation behavior.
- Fixed initial post-finish core expectation, strict Kotlin safe-call warning and
  stale host reread offset. Final checks pass; no open failure or local blocker.
  Updated plan/handoff/architecture/data/decision/development/README docs. Next:
  typed reader token/status projection and word taps; offline dictionary data/index.
  No full Android suite, standalone Full suite, release, process/reboot, tablet or
  physical-arm64 rerun. Nostr empty; Q188 still required before forwarding.

## 2026-10-24 — Read-only reader token/status foundation

- Started clean. Added pure Mode B page-token projection and separate SQLite
  load_reader_tokens snapshot with current exact-surface global ID/status. Retains
  original token indices/full anchors/surfaces/readings/forms; clipped page-local
  UTF-8 display spans allow crossing fragments without changing completion counts.
  Original B stays pinned; Missing/deleted vocabulary stays absent. No lookup writes,
  rescans, implicit New, override application, progress reset or history changes.
- PASS: workspace build, 229 data-free Rust tests (117 storage), strict Clippy/fmt/
  diff, 138-file Sudachi source verification and all 11 real-Full analysis fixtures
  plus one runtime fixture per owned/mapped loader. Four new pure + six repository
  tests cover Unicode/reflow/repetition/all statuses/global refresh/deletion/corrupt
  state; query-only SQLite + unchanged DB bytes verify read-only behavior. Extended
  real-Full import/completion fixture checks stored token metadata and current
  statuses before/after completion, preserving finished EOF position on lookup.
- Updated plan, handoff, architecture/data model, decisions, development and README.
  No schema/dependency/event/view codec change; no failure/local blocker. Foundation
  is not wired to runtime/FFI/Compose yet: next add typed effects/manual status
  changes and page-keyed stale-result rejection, then taps/five direct choices.
  Android/JNI/lint, process/reboot, release/tablet/physical-arm64 not rerun this slice.
  Nostr stays empty; ask Q188 before forwarding.

## 2026-10-24 — Typed read-only reader tokens through core/FIFO/FFI

- Started clean. Added explicit LoadReaderTokens(ReaderTokenKey), worker dispatch
  and transient reader_tokens in generated Kotlin/JNI. Core requires matching
  current/pending/result document/page/original-analysis keys, ignores cancelled
  results/errors and invalidates cached tokens on refresh/navigation/position/
  completion callback or changed reflow span. Missing remains empty; no analyzer
  load, registration, status/history/position mutation or schema change.
- Reads share the existing busy gate/FIFO and allow final checkpoints to queue.
  A failed read drains waiting time before deferred departure. Four new core tests
  cover stale identities/cancellation/retry/reflow/Missing and clock drain. One new
  asynchronous FFI test verifies Unicode metadata, all five live statuses and exact
  unchanged database bytes. Real-Full Kotlin/JNI verifies New-to-Known refresh and
  analyzer-free reopen. Android preview supplies the added optional view field.
- PASS: 234 Rust tests (117 storage), workspace build via JNI runner, strict Clippy/
  fmt/diff, seven Unicode exchanges, repeatable Kotlin generation, Kotlin formatting,
  both-ABI debug/test APK builds, strict Android lint and all 11 selected API-36
  x86_64 core/reader/clock tests (2m38s final run).
- Fixed a synthetic fixture missing an unselectable non-Japanese token. Gradle's
  sandbox lock/socket denial required escalation; the initial device run disconnected
  after four passes. Restarted the existing AVD with documented swangle flags without
  resetting data; complete rerun passes. No open failure/local blocker.
- Updated plan, handoff (under 80 lines), architecture/data/decisions/development
  and README. Next: typed manual status changes, then Compose taps/five choices;
  offline dictionary data/index. No standalone Full fixture suite, full Android
  suite, release/tablet/physical-arm64 or separate recovery rerun. Nostr/Q188 unchanged.

## 2026-10-24 — Typed manual reader status choices

- Started clean. Added ReaderStatusChange and SetReaderStatus through pure core,
  FIFO worker and generated Kotlin/JNI, reusing the atomic status/history repository.
  Only existing vocabulary in the loaded current page is accepted. Successful
  matching callbacks update every visible repetition; stale callbacks/errors ignore.
  Same-status choices add no history, deleted IDs never register, failed choices
  preserve the snapshot for explicit retry. Both outcomes drain final clock saves.
- PASS: 238 Rust tests (117 storage), strict Clippy/fmt/diff, real-Full Kotlin/JNI,
  seven Unicode exchanges, repeat generation, Kotlin formatting, both-ABI debug/test
  APK builds, strict Android lint and all 13 selected API-36 x86_64 tests (3m32s).
  Four new core tests and extended FFI/JNI tests verify five choices, repeated IDs,
  cancellation/mismatches, rollback/retry, deletion and clock/departure ordering.
  New Android test verifies all five statuses and global identity across two docs.
- Updated plan/handoff, architecture/data, decisions, development and README.
  No schema/dependency/View change. ADB/Gradle sandbox sockets required escalation;
  no open failure/local blocker. Next: Compose token loading, UTF-16 spans,
  highlights/taps/five direct controls, then offline dictionary data/index.
  Standalone Full/full Android suite, release, recovery/tablet/physical-arm64 not
  rerun. Nostr remains empty; Q188 remains required before forwarding.

## 2026-10-24 — Compose reader word highlights and direct choices

- Started clean. Connected gated token loading, validated UTF-8→UTF-16 spans,
  product light/dark highlights and inline Japanese links. A phone bottom sheet or
  wide side panel shows full surface/analyzer metadata and five direct choices.
  Opens leave vocabulary/history/position untouched; committed changes update
  repetitions. Deleted IDs stay absent, failed choices allow explicit repeat,
  pending saves disable choices, and invalidated/recreated UI dismisses the panel.
- Added per-key read-attempt guard so clock saves clearing errors cannot replay
  failed lookup. Sheet/popup content input extends the existing idle deadline.
  Definitions/names/encounter count remain next; no schema/dependency/FFI change.
- PASS: 238 Rust tests (117 storage), strict Clippy/Rust/Kotlin formatting/diff,
  both-ABI debug/test APK builds and strict Android lint. All 24 selected API-36
  x86_64 phone tests pass; seven word tests also pass wide landscape/dark (31 total
  successful executions). Saved XML under `.local/test-reports/reader-words/`.
- Fixed strict Kotlin/lint findings, an analyzer-segmentation fixture, sheet input
  forwarding and old uncomposed Library selectors. Recovered an AVD disconnect
  without reset; extended initial Full-import fixture wait to 60s after cold boot.
  Interrupted the stale failing APK run while applying fixes; final checks pass.
  Gradle/ADB sockets required escalation. Restored AVD size/density/light mode.
- Updated plan/handoff (79 lines), architecture/data/decisions/development and README.
  Unrelated release-checklist deletions appeared during work and remain unstaged.
  No open blocker. Host JNI/generation/standalone Full, release, separate reboot/
  process and physical arm64 not rerun. Nostr/Q188 unchanged.

## 2026-10-25 — Reproducible offline English word/name index

- Started clean; pinned official JMdict English/JMnedict 2026-10-25 snapshots,
  compressed checksums/counts and complete CC BY-SA 4.0 attribution/docs/legal code.
  Added a stdlib-only staged SQLite compiler retaining ordered senses/readings,
  restrictions, inherited POS, separate names and exact-form indexed candidates.
  Default offline operation; verified source archives are retained in the bundle.
  Published output is read-only and never replaced on changed/corrupt input.
- PASS: nine synthetic/full dictionary tests, all 962,639 entries, two complete
  byte-identical bundles, 238 Rust tests, strict Clippy/Rust/Kotlin formatting and
  diff checks. Fixed unclosed Python SQLite handles and used official static docs
  after the retired wiki redirected to a closure notice. Downloads needed network
  escalation. No open failure/blocker; no app schema/dependency/FFI/UI change.
- Updated plan, handoff, architecture/data, decisions, development and README.
  Next: immutable asset loading/typed Rust candidate lookup with restriction
  filtering, then definitions/names/lifetime count in the existing panel. Android,
  host JNI/generation, standalone Full/release/recovery/physical-arm64 not rerun.
  Nostr configuration remains empty; ask Q188 before forwarding.

## 2026-10-25 — Typed read-only Rust dictionary candidates

- Started clean; added core typed word/name payloads and pure exact-form rules for
  all licensed readings and combined spelling/reading sense restrictions. Added a
  separate read-only storage connection with format checks, bound indexed queries,
  source/base provenance, source-scoped deduplication and separate name results.
  Complete upstream metadata remains intact; display uses applicable index views.
  Invalid input fails without partial results, writes or repairs. Installer checksum
  verification/publication remains future work; no mutable database/FFI/UI change.
- PASS: 248 data-free Rust tests (ten new), strict Clippy/Rust fmt/diff, nine Python
  tests and explicit full-index Rust validation of all 962,639 payloads plus real
  inflected/kana/simultaneous-word-name/no-match lookups. Reused locked serde_json;
  no new packages. Fixed feature-gated preparation call and relative test data path.
- Updated plan/handoff (80 lines), architecture/data/decisions/development, index
  contract and README. No open blocker. Next: verified Android index/notices and
  Settings credits, then FIFO/FFI definitions/readings/compact POS/names/lifetime
  count in the existing panel. Android/JNI/generation/Kotlin/Full/release/recovery/
  physical-arm64 not rerun. Nostr remains empty; Q188 required before forwarding.

## 2026-10-25 — Verified Android word/name dictionary assets

- Started clean; added an offline Gradle bundler with pinned build/index identity,
  verified immutable staged output, exact compressed sources/compiler/manifests
  and complete legal/project notices. Added lazy worker-only private read-only
  SQLite publication via the existing checksum/recovery publisher; no user
  migration/dependency/FFI change. Library/Settings do not extract the index.
  Added complete selectable offline Settings credits with retained dialog state.
- PASS: 248 Rust tests, strict Clippy/Rust/Kotlin fmt/diff, four bundler + eight
  compiler tests, both-ABI debug/test APK builds, strict Android lint, 14 selected
  API-36 x86_64 phone tests + two wide/dark credits/recreation checks in airplane
  mode. Every installed dictionary/source/compiler/notice hash matches; real
  read-only index opens/東京 lookup/reverification pass. Reports and measurements:
  DEVELOPMENT.md and `.local/test-reports/dictionary-assets/`.
- Fixed lint's numeric-date warning and Android's automatic `.gz` expansion;
  `.gz.bin` preserves exact compressed source bytes. Preserved old generated output
  before fresh immutable publication. Restored AVD size/density/theme/network.
  Updated plan/handoff and architecture/data/index docs. Gradle/ADB/emulator needed
  escalation. No open blocker. Next: verified FIFO/FFI lookup/count provider and
  dictionary panel. Host JNI/Full/release/recovery/reader matrix/physical arm64
  not rerun. Nostr remains empty; ask Q188 before forwarding.

## 2026-10-25 — Selected reader word dictionary/count repository

- Started clean; composed validated original Mode B page/token selection, live
  status and indexed lifetime exact-surface count in one read transaction, with
  separate immutable dictionary word/name candidates and applicable views.
  Echoed identity supports later cancellation. Typed selection/storage/dictionary
  errors distinguish failure from no match; lookup never writes or re-registers.
- PASS: 256 data-free Rust tests (eight new), strict Clippy/Rust fmt/diff. Coverage
  includes repetitions/rereads/duplicate completions/cross-document counts, base
  isolation, deletion/re-registration, all-presets clipped Unicode, no match,
  failure/retry, indexed counting and query-only unchanged-byte checks.
- Updated plan/handoff, architecture/data/decisions/development. No migration,
  dependency or FFI/UI change; Android/JNI/Kotlin/generation/Python/real data/
  release/recovery/physical arm64 not rerun. No open blocker. Next: verified lazy
  FIFO/FFI selected-word transport, then panel senses/readings/POS/names/count.
  Nostr remains empty; ask Q188 before forwarding.

## 2026-10-25 — Lazy selected-word FIFO and typed JNI

- Started clean. Added worker-only verified word-index path callback to the existing
  provider/FIFO, lazy read-only connection reuse and failed-query retirement for
  repaired-index retry. Moved complete candidate/details values to pure core,
  using concrete word/name types for Kotlin generation. No new dependency/migration.
- Added explicit selection/dismiss events and transient details/errors; current,
  pending, selected and full token identity gate attachment. Live status refreshes
  all repetitions; manual choices update selected status/absence. Navigation,
  Settings and page invalidation cancel results; clock saves preserve lookup errors
  and both read outcomes drain final time. Compose panel remains unchanged.
- PASS: 262 Rust tests (six new), strict Clippy/Rust/Kotlin fmt/diff/shell syntax,
  real-Full/real-index Kotlin/JNI plus seven Unicode exchanges, reproducible payloads,
  both-ABI debug/test APK builds, strict Android lint and four API-36 x86_64 tests
  (one new dictionary transport test plus three document/bridge regressions).
  Full commands/coverage in DEVELOPMENT.md; saved Android reports under ignored
  .local/test-reports/reader-word-details/. AVD remains running.
- Fixed generator generic-name collision with concrete types and Clippy enum size
  with internal boxing. Concurrent BoltFFI Android/host runs caused a transient JNI
  link failure; sequential final rerun passes. Gradle/ADB sandbox sockets needed
  escalation. No open blocker; no tablet/dark/release/recovery/physical arm64 run.
- Updated README/index contract and architecture/data/decisions/plan/handoff.
  Next: connect current panel to details requests, then applicable readings/senses,
  compact POS, separate names/count, retry and recreation dismissal. Nostr remains
  empty; ask Q188 before forwarding.

## 2026-10-25 — Reader dictionary panel

- Recovered six uncommitted Android files from the interrupted session; preserved
  the implementation and completed its verification. Connected the existing phone
  sheet/wide panel to gated selected-word requests and dismissal. Render all
  applicable expanded senses/readings, compact POS, separate name categories/
  translations and lifetime exact-surface count; errors require explicit retry.
- Closing/disposal/recreation and core invalidation retire selection; late results
  cannot reopen it. Teardown tolerates Compose's final dismissal. No dependency,
  storage migration, Rust or wire change.
- PASS: debug/test APK builds, strict Android lint, Kotlin fmt/diff; 14 dictionary/
  span/session tests phone/light + wide/dark, plus 17 phone reader/navigation/clock/
  paste regressions. Six new tests cover applicable views/POS, simultaneous sources/
  counts/read-only opens, failure/no match, late close and both movement directions.
  Fixed the old mock selection echo and a new fixture's page-content assumption.
- Reports: .local/test-reports/reader-dictionary-panel/. Gradle/ADB sockets needed
  escalation; all final runs pass. Emulator restored to phone/light and left running.
  Host Rust/JNI/generation, Python/full scans, release, separate process/reboot and
  physical arm64 were not rerun. No open blocker. Updated plan/handoff and supporting
  docs; next: phase-4 TXT imports, then atomic integration. Nostr empty; ask Q188.

## 2026-10-25 — Pure TXT decoding and Aozora parser

- Started from a clean tree and continued phase 4. Added `yomoku-import` with
  strict UTF-8/Shift-JIS decoding, conservative both-valid uncertainty and explicit
  manual selection. Verified/pinned encoding_rs 0.8.35 against primary API docs;
  only one new locked package, no I/O/SQL/core or Android protocol changes.
- Canonical TXT preserves hard breaks/Unicode, stores source ruby separately as
  ordered canonical UTF-8 ranges, hides common typographic directives and preserves
  unknown/semantic/gaiji/malformed notation. No inferred chapters or body titles.
- PASS: all 275 data-free workspace Rust tests (13 new TXT), strict Clippy, Rust
  fmt and diff checks. Corrected one fixture's premature empty-text expectation;
  review also added nested malformed quote/note preservation. Final tests pass.
- Cargo unpacking needed approved cache-write escalation, using already cached
  bytes offline. No open blocker. Android/JNI/Kotlin/generation, real dictionaries,
  release/process/reboot and physical arm64 were not rerun for this pure parser.
- Updated plan, architecture/data docs, README, decisions, commands and handoff.
  Next: original-file SHA-256 plus atomic TXT text/ruby/Mode B/registration storage,
  then typed runtime/Android picker and encoding selection. Nostr empty; ask Q188.

## 2026-10-25 — Atomic TXT original-file repository

- Continued phase 4 from a clean tree. Added migration 0012, original-file SHA-256
  identity/source BLOB/encoding and canonical source-ruby tables. A new worker-only
  TXT repository atomically composes strict parsing, filename title, canonical text,
  ruby, original Mode B and unique registrations; exact duplicates skip decoding/
  analysis and preserve existing state. Paste hash semantics remain separate.
- Shared single-text snapshot/token/completion paths accept TXT with the unchanged
  historical bridge type. Added read-only source/ruby access; runtime/Android file
  and ruby transport/rendering remain future work. No new package/version.
- Rebuilt sources.kind using SQLite's documented transaction/FK-check procedure,
  preserving existing IDs, history and autoincrement high-water marks. Migration
  rollback restores schema/data and enforcement. Fixed a fresh-database sequence
  regression caught by the existing full-width-ID test and corrected encoding/FK
  fixture assumptions. No open failure/blocker.
- PASS: 287 data-free Rust tests, strict Clippy/fmt/diff; twelve new repository/
  migration tests and one real-Full TXT fixture. All twelve Full and one runtime
  fixture pass with both owned/mapped loaders. Targeted tests rerun after stronger
  query-only/history assertions. No escalation/download required.
- Updated plan/architecture/data/decisions/README/commands/handoff. Android/Kotlin/
  JNI/generation/Python/full-index/release/process/reboot/physical arm64 not rerun.
  Next: lazy FIFO/typed file import with encoding uncertainty, then Android picker/
  encoding choice and Library summary. Nostr stays empty; ask Q188 before forwarding.

## 2026-10-25 — Typed TXT FIFO/JNI imports

- Continued phase 4 from a clean tree. Added typed original bytes/display name/
  optional encoding to the core event/effect protocol, explicit strict TXT errors
  and lazy FIFO repository dispatch. Created/Existing reuse Library summaries
  without auto-opening; pending saves gate imports and Android pauses its clock.
- Added a shared read-only exact-file preflight: duplicates skip decoding/Full
  even after hidden release or reopen with an unavailable provider. New files
  validate before loading Full outside the write transaction, then repeat the
  authoritative atomic lookup/parse. No new package/version or migration.
- Five new worker fixtures and one core fixture verify encoding/retries, duplicate
  preservation, failed-file/write/deferred-commit isolation, analyzer failures,
  save gates and background/close draining. Extended storage query-only/corruption
  coverage; new real-Full Kotlin/JNI and Android session fixtures pass. Bundled
  encoding_rs upstream copyright/licenses and verified exact APK bytes.
- PASS: 293 data-free Rust tests, strict Clippy/Rust/Kotlin fmt/diff, reproducible
  payloads, real-Full/real-index host JNI and seven Unicode exchanges, both-ABI APK
  builds/strict lint, all five selected API-36 x86_64 import/session tests. Fixed
  intermediate Kotlin warnings; ADB/.git sandbox escalation and cold AVD start required.
  Reports: `.local/test-reports/txt-import-bridge/`; AVD remains running.
- Updated plan/architecture/data/decisions/README/commands/handoff. No open blocker.
  Standalone Full/Python/full-index/release app/process/reboot/tablet/dark/physical
  arm64 not rerun. Next: Android multi-file TXT picker/encoding choice and summary;
  progress/cancel/background notifications, EPUB/SRT and ruby rendering remain.
  Nostr empty; ask Q188 before forwarding.

## 2026-10-25 — Android multiple-TXT picker, encoding and per-file summary

- Added Library's multiple-TXT Android picker, off-main original-byte URI reads
  and retained sequential submission to the existing atomic Rust command. UTF-8/
  Shift-JIS/Skip choices reuse current bytes; one failed/skipped file never aborts
  later files. Duplicate results reference the existing title. Scrollable results
  return to Library without opening a reader and survive Activity recreation.
- Kept platform queue/results in CoreSessionOwner without retaining an Activity;
  final teardown drops unsubmitted work while native accepted imports drain.
  No persisted queue, dependency/version, FFI, schema or Rust change. Progress/
  cancellation/background notifications, EPUB/SRT and reader ruby remain unchecked.
- PASS: both-ABI debug/test builds, strict Android lint, Kotlin fmt/diff, five final
  picker/navigation/paste tests and five distinct import/session regressions on
  API-36 x86_64. New tests use actual contract callbacks and a test-APK-only pipe
  provider; nine files verify duplicates/failures/continuation, both persisted
  encoding choices/Skip, summary scrolling/retention/dismissal and picker cancel.
- Fixed intermediate test imports, intent filter, cache/provider runtime and
  duplicate selector fixtures; a cold-run native timeout passed offline. Gradle/
  ADB/AVD required escalation. Reports: `.local/test-reports/txt-picker/`.
  Restored airplane mode off; AVD remains running. No open blocker.
- Updated README/plan/architecture/decisions/development/handoff. Rust/JNI/Full/
  Python/index/release/process/reboot/tablet/arm64 not rerun this UI slice. Next:
  progress/current-file/X-of-N, transaction-aware cancellation/background notices,
  then EPUB/SRT. Nostr empty; ask Q188 before forwarding.


## 2026-10-25 — Retained TXT current-file and batch progress

- Added File X of N, queried filename before original-byte reads and a determinate
  processed-files bar to the retained import dialog, including encoding prompts.
  Terminal successes/duplicates/errors/skips advance once; retries do not. Counts
  reset on dismissal/new batches. No time/byte progress or persistence claim.
- PASS: both-ABI debug/test APK builds, strict Android lint, Kotlin formatting/diff
  and all five API-36 x86_64 picker/navigation tests. Added a gated pipe/recreation
  fixture and extended the mixed-outcome nine-file fixture with progress semantics.
  Reports: `.local/test-reports/txt-progress/phone/`.
- Fixed initial lint count wording; connected Gradle sockets required escalation.
  No new dependency, permission, Rust/wire/schema change or open blocker. AVD left
  running. Rust/JNI/Full/Python/index/tablet/release/process/arm64 not rerun.
- Updated README/architecture/decisions/plan/development/handoff. Next:
  transaction-aware Cancel retaining earlier commits, then permitted background
  notifications/process-death acceptance. EPUB/SRT remain next; Nostr empty/Q188.

## 2026-10-25 — Cooperative TXT repository cancellation

- Added shared per-file ImportCancellation and optional cancellable TXT import.
  Checkpoints roll back the current partial file; a mutex serializes cancellation
  with COMMIT. Completed imports remain intact. Split common validated analysis
  persistence so TXT checks cancellation after analysis before token/registration
  writes. Existing runtime/JNI/Android continue through the noncancellable wrapper.
- PASS: 299 Rust tests (148 storage, six new cancellation fixtures), strict Clippy,
  fmt/diff, twelve real-Full plus one worker fixture with each owned/mapped loader.
  Tests cover pre-start bypass, blocked-analysis/sequence rollback, durable earlier
  imports/statuses, fresh retry, duplicate/late-cancel isolation and commit races/
  deferred failure. Fixed the observer fixture's migration-lock ordering.
- Updated architecture/data model/decisions/plan/development/handoff. No new
  dependency, migration, escalation or blocker; Android/JNI/codegen/process-death/
  Python/index/release/reboot/tablet/arm64 not rerun. Next: runtime/JNI cancellation
  control outside FIFO and off main, Android Cancel/remaining-file disposal/summary,
  then background notifications and process-death acceptance. Nostr empty/Q188.

## 2026-10-25 — Direct TXT runtime/JNI cancellation

- Published a fresh cancellation handle before accepted FIFO submission; direct
  AnalyzerLifecycle/native cancelTxtImport bypasses the queue and core update lock.
  A distinct cancelled result clears busy without error/summary. Weak controls,
  matching-attempt cleanup and repository commit arbitration retain completed files
  and isolate fresh retries. Android must call off main and await return + outcome.
- PASS: 304 Rust tests (four new worker fixtures and one core fixture), strict
  Clippy/fmt/diff, real-Full/word-index host Kotlin/JNI including blocked-provider
  cancellation, seven Unicode exchanges, reproducible Crux/native generation,
  both-ABI app/test APK builds, strict Android lint and Kotlin formatting.
- Fixed a test lookup method name; Gradle wildcard-IP/socket failure needed approved
  escalation. No new dependency/schema/permission or open blocker. Android UI tests,
  standalone Full/Python/index/release/process/reboot/tablet/arm64 not rerun.
- Updated architecture/data model/decisions/plan/development/handoff. Next: Android
  Cancel/remaining-URI disposal/summary, then background notifications and actual
  process-death/no-resume acceptance. Nostr empty; Q188 before forwarding.

## 2026-10-25 — Android transaction-aware TXT batch Cancel

- Added Cancel/Cancelling to retained TXT progress/encoding dialogs. Stop URI
  submissions immediately, clear remaining/encoding input, ignore late provider
  callbacks by generation and retire blocked URI executors for fresh batches.
  Keep earlier results and a winning commit in the summary, without opening reader.
- CoreSession owns a separate cancellation executor and retains its native handle
  until both direct Cancel and current-file result finish, including teardown.
  Keep current filename/number fixed while cancellation settles; progress counts
  completed outcomes only. No persisted queue or new schema/permission/dependency.
- PASS: 304 Rust tests, strict Clippy/fmt/Kotlin fmt/diff, both-ABI app/test builds,
  strict Android lint and fourteen final API-36 x86_64 phone/light tests. Three new
  picker/JNI tests cover blocked reads/load, encoding/recreation/background, both
  callback orders, duplicate cancellation, commit retention, fresh retries and
  draining native close. Existing clock/navigation/picker regressions pass.
- Fixed the existing host teardown fixture's premature closure assertion with its
  channel-disconnect signal and a new ambiguous-input fixture with a UTF-8 BOM.
  Gradle/ADB/AVD needed sandbox escalation. Final reports in
  `.local/test-reports/txt-cancel/phone-final/`; no open failure/blocker.
- Updated plan/architecture/data/decisions/development/handoff. Next: permitted
  background notifications and actual process-death/no-auto-resume acceptance;
  EPUB/SRT and CI independent. Host JNI/Full/release/tablet/dark/physical arm64 not
  rerun. Airplane mode restored off; AVD running. Nostr empty/Q188 unchanged.

## 2026-10-25 — Permitted TXT background notifications and process death

- Added a platform-only nonexported dataSync foreground lease around the existing
  helper/FIFO, processed/current-file progress and completion/cancellation notices.
  Release the lease while encoding needs input; use a separate notification ID.
  Explicit notification enable action; denial never blocks import. Timeout stops
  the service immediately, then cooperatively cancels/drains. No persisted queue,
  restart or process-survival guarantee. No Rust/wire/schema/dependency changes.
- PASS: both-ABI debug/test builds, strict lint/Kotlin fmt/diff, ten distinct API-36
  x86_64 phone/light notification/picker/native-cancel/navigation tests across the
  final focused group and earlier regressions. Corrected two test races by waiting
  for Android notification publication/removal as well as helper/service state.
- Added a DUMP-protected debug-only isolated recovery Activity and host runner.
  Two actual HOME/SIGKILL/fresh-process pairs with notification permission granted/
  denied keep one committed file and discard the blocked/remaining selections;
  no graceful close, batch counts/summary, service or queue resumes. Permission
  restored and ordinary Library relaunched; normal Library never cleared by probe.
- Reports: `.local/test-reports/txt-background/notifications-final/` plus passing
  regressions in `removal-wait-failure/`; process report
  `.local/test-reports/txt-process-death/20261025T212646351293Z.json`. Gradle/AVD
  sockets/KVM needed escalation; no open failure/blocker. Timeout callback tested
  directly; no LMK/reboot/release/physical-arm64 claim. Emulator left running.
- Updated README/architecture/data/decisions/plan/development/handoff. Next: pure
  EPUB/SRT parsers, repository/runtime/mixed-picker wiring. CI independent. Rust/
  Clippy/host JNI/codegen/Full/index/tablet/dark not rerun; Nostr empty/Q188 unchanged.

## 2026-10-25 — Pure tolerant SRT extraction

- Added `yomoku-import::srt` for decoded text → canonical LF text and UTF-8 cue
  ranges. Strip recognizable counters/timing/common formatting; retain cue blocks,
  within-cue breaks, speakers, numeric dialogue and unknown/damaged content.
  Recover missing counters/separators and short/dot timing. Decode entities once
  after tag removal. No I/O, IDs, dependencies, schema or wire changes.
- PASS: 317 workspace Rust tests, strict Clippy/fmt/diff; thirteen new SRT tests
  plus existing TXT regressions cover malformed/large inputs, Unicode/ranges and
  all five pagination presets. Review tightened malformed numeric entity handling.
  No open failure/blocker. Android/JNI/codegen/explicit Full/index/device checks
  were not rerun for this parser-only slice.
- Updated README/plan/architecture/data/decisions/development/handoff. Next: EPUB
  parser, EPUB/SRT byte-decoding/original-file repository/runtime adapters, then
  mixed picker; reader ruby and CI independent. Nostr empty/Q188 unchanged.

## 2026-10-25 — Pure EPUB extraction

- Added `yomoku-import::epub`: in-memory ZIP → metadata/spine chapters, canonical
  UTF-8 text, headings/emphasis/ruby/link ranges and positioned image/cover bytes.
  Ignore CSS/scripts; retain auxiliary/image-only chapters and inert internal
  links. No I/O, IDs, schema, wire or production app import changes.
- PASS: 341 data-free Rust tests (24 new EPUB), strict Clippy/fmt/diff and both
  Android target checks. Fixtures cover Stored/Deflate/CRC/malformed archives,
  duplicate/alias/unsafe paths, encryption, UTF-8/UTF-16/ranges, ruby/images/links,
  whitespace, resource limits, large text and all five pagination presets.
- Fixed XML library recursion exposed by the first deep fixture: nonrecursive
  lexical preflight caps depth at 64 before tree parsing, rejects custom entity
  injection and invalid numeric references instead of silent replacement. Final
  10,000-depth/near-limit/CDATA/comment checks pass. Corrected initial API/borrow
  errors; no open failure/blocker. Nine pinned ZIP/Deflate/XML lock entries added.
- Updated README/plan/architecture/data/decisions/development/handoff. Next: atomic
  EPUB/SRT repository/runtime imports, mixed picker and EPUB/source-ruby rendering;
  CI independent. App still paste/TXT. Compatibility limits in DATA_MODEL.md.
  APK/lint/device/JNI/codegen/explicit Full/index/release/reboot/physical arm64
  not rerun for this parser slice. Nostr empty/Q188 unchanged.


## 2026-10-25 — Atomic analyzed SRT repository

- Added original-byte SRT imports with shared strict UTF-8/Shift-JIS decoding (no
  Aozora), filename title, one canonical chapter/cue ranges, Mode B and unique
  registrations in one cancellable transaction. No import encounters/time.
- Migration 0013 widens source kinds and adds cascading cue rows, preserving old
  IDs/foreign keys/source high-water. Shared TXT/SRT exact-file identity references
  the first interpretation; source loading validates digest/anchors without parsing.
- PASS: 353 data-free Rust tests, strict Clippy/fmt/diff, storage checks on both
  Android targets; thirteen real-Full analysis + one worker fixture per owned/mapped
  loader. Twelve new repository fixtures plus a real SRT import/completion/reopen
  fixture cover rollback, cancellation, concurrency, encoding, duplicates, migration
  and status/progress preservation. Corrected one synthetic expected count.
- Updated README/plan/architecture/data/decisions/development/handoff. Next: SRT
  worker/typed FFI, EPUB repository/runtime, mixed picker and reader assets/ruby;
  CI independent. App still paste/TXT. JNI/APK/lint/device/release/reboot/physical
  arm64 not rerun; no open blocker. Nostr empty/Q188 unchanged.


## 2026-10-25 — SRT FIFO imports and typed JNI

- Added SrtImport/ImportSrt, format-specific encoding/empty outcomes, lazy FIFO
  preflight/atomic import and Library summaries with the existing reader-save gates.
  Exact TXT/SRT duplicates preserve the first interpretation without loading Full.
- Generalized direct cancellation to cancelFileImport(); retained the TXT alias.
  One per-file registry covers queued/load/analysis/close races; committed files
  survive and cancelled work adds no summary/error. Android error resources and
  clock-pause dispatch compile; picker remains TXT-only.
- PASS: 365 data-free Rust tests, strict Clippy/Rust+Kotlin fmt/diff, real-Full
  Kotlin/JNI and Unicode exchanges, reproducible bindings, both-ABI debug/test APKs
  and strict Android lint. Ten new worker/two core fixtures; two new JNI scenarios.
  Corrected initial fixture table/range expectations and regenerated a stale native
  comment; final checks pass. Gradle local sockets required approved escalation.
- Updated README/plan/architecture/data/decisions/development/handoff. Next: EPUB
  atomic chapters/assets/Mode B + runtime, mixed picker/summary/Cancel, reader
  images/source ruby and CI. No new dependency/migration or open blocker.
  Android device/instrumentation, standalone Full/index/Python, release, process-
  death/reboot and physical arm64 not rerun. Nostr empty/Q188 unchanged.

## 2026-10-26 — Atomic EPUB persistence

- Added migration 0014 and EPUB repository: original SHA-256/bytes, metadata/title
  fallback, ordered canonical chapters/ruby/formatting/link/image spans, shared
  cover/image bytes, per-chapter original Mode B and unique registration together.
  Existing file-ID references bypass parsing/analysis. Shared cancellation/COMMIT
  arbitration rolls back a partial book; all prior committed files remain.
- Read-only reopen validates counts/digests/anchors/ownership without reparsing.
  Library progress now includes all chapters and validates complete chapter sets.
- PASS: 379 data-free Rust tests, strict Clippy/fmt/diff, fourteen real-Full analysis
  + one worker fixture per owned/mapped loader, existing real-Full Kotlin/JNI and
  seven Unicode exchanges, both Android storage target checks. Fourteen new
  repository tests and one explicit Full fixture. Boxed the large import result
  after Clippy feedback. No open failure/blocker or escalation/download.
- Updated README/plan/architecture/data/decisions/development/handoff. Next:
  multi-chapter reader/import-reference transport and EPUB FIFO/JNI, mixed picker,
  then reader assets/ruby. Historical single-text APIs deliberately reject EPUB;
  app remains paste/TXT. Migration preserves prior IDs/FKs/high-water/file identity.
  No new production dependency/version; test helpers reuse pinned zip. APK/Android
  lint/device/instrumentation, codegen reproducibility, standalone index/Python,
  release/reboot/process-death, tablet/dark and physical arm64 not rerun. Nostr/Q188 unchanged.


## 2026-10-26 — Ordered-chapter reader repositories

- Added pure ReaderDocument/DocumentReaderSnapshot, complete chapter-bound pages,
  global counters/progress and generic read/open/position repositories. Existing
  completion/time/token/selected-word repositories now accept complete books.
  Chapter EOF advances to the next chapter; only whole-book final completion
  finishes. Original B IDs, time buckets, replay and rollback remain chapter-safe.
- PASS: 393 data-free Rust tests, strict Clippy/fmt/diff, both Android storage
  targets, fourteen real-Full analysis + one worker fixture per owned/mapped loader,
  real-Full Kotlin/JNI and seven Unicode exchanges. Five new pure/nine repository
  fixtures plus expanded real EPUB coverage. Corrected uncertain-encoding fixture
  and consolidated shared test imports after Clippy feedback; final checks pass.
- Updated README/plan/architecture/data/decisions/development/handoff. No migration,
  new dependency, download/escalation or open blocker. Next: migrate application/
  worker/import-reference/typed JNI payloads, wire EPUB FIFO, mixed picker and assets.
  Android app remains paste/TXT. Complete EPUB integrity reads repeat per operation;
  performance unmeasured. APK/lint/device, codegen repeat, index/Python, release/
  reboot/process-death/tablet/dark and physical arm64 not rerun. Nostr/Q188 unchanged.


## 2026-10-26 — Format-independent import references

- File preflight and TXT/SRT/EPUB duplicate results now return Library references
  with complete saved-document validation in the identity transaction. Core import
  results also carry only Library state; exported event/request/view schemas stay
  unchanged. No single-chapter payload/truncation or finished-book reset on duplicate.
- PASS: 395 data-free Rust tests, Clippy/Rust+Kotlin fmt/diff, both Android storage
  targets, fourteen real-Full analysis + one worker fixture per owned/mapped loader,
  real-Full Kotlin/JNI and seven Unicode exchanges. Two new FIFO fixtures verify
  duplicate EPUB title/progress/finish without Full/decoding/writes/auto-opening,
  corrupt-content rejection and later-file continuation; expanded repository tests
  cover all format adapters/read-only/corruption. No new failure or blocker.
- Updated README/plan/architecture/data/decisions/development/handoff. No migration,
  production dependency/version, permission, download or escalation. Shared runtime
  test archives reuse the existing pinned ZIP dev dependency. Next: app/worker/
  typed JNI ordered-chapter readers and EPUB FIFO/import wiring, then mixed picker
  and reader assets. App stays paste/TXT; duplicate integrity-read cost unmeasured.
  APK/Android lint/device, codegen repeat, standalone index/Python, release/reboot/
  process-death/tablet/dark and physical arm64 not rerun. Nostr/Q188 unchanged.

## 2026-10-26 — Ordered application/FIFO/typed JNI readers

- Application loads/opens/position saves/completion reloads now use complete ordered
  ReaderDocument snapshots. Generated view payloads retain all chapters and exact
  chapter-local position; active core selects each chapter's frozen original Mode B.
  Previous/global counters/progress/reflow and whole-book-only finish are connected.
- PASS: 399 data-free Rust tests, strict Clippy/Rust+Kotlin fmt/diff, real-Full
  Kotlin/JNI, seven Unicode exchanges including a seven-chapter payload, repeatable
  Crux/native bindings, both-ABI APK builds/strict Android lint and 13 focused API-36
  x86_64 persistence/navigation/timer tests. Reports: .local/test-reports/ordered-reader/.
  One new core/three worker fixtures cover keyed identity validation, cross-chapter
  movement/reread/time/reflow/finish/reset, image-only completion and corrupt-reader
  isolation without Full. Corrected synthetic expectations/Kotlin safe-call warning;
  final checks pass. Existing benign Crux close diagnostics persist.
- Updated README/plan/architecture/data/decisions/development/handoff. No dependency,
  migration, permission or open blocker. AVD cold-started without data wipe; ADB
  socket/KVM required escalation. Emulator left running. Next: EPUB per-file FIFO/JNI
  import isolation/cancellation, mixed picker and reader assets. App stays paste/TXT;
  complete EPUB integrity-read cost remains unmeasured. Standalone Full/index/Python,
  release, process-death/reboot/tablet/dark and physical arm64 not rerun. Nostr/Q188 unchanged.


## 2026-10-26 — EPUB per-file FIFO and typed JNI imports

- Started clean. Added typed EPUB bytes/name event, pure storage intent and
  encoding-independent InvalidEpub error; Android pauses its reader clock for the
  event and renders the error. Mixed picker selection stays a separate task.
- Wired bounded pre-parse/shared exact-byte preflight to the existing lazy FIFO
  analyzer and atomic whole-book repository. All chapters share one transaction;
  Created projects a Library reference without another read or auto-opening.
  EPUB reuses the direct cancellation registry/commit gate and analyzer lifecycle.
- Added two core/eight worker fixtures, expanded all-format preflight coverage and
  a real-Full Kotlin/JNI EPUB suite. Verify full content/analyses/registration,
  unchanged statuses/no encounters, duplicates/renamed/later-chapter/finished state,
  invalid containers before Full, every-write/later-chapter/deferred-COMMIT rollback,
  queued/load/chapter/draining cancellation, prior-file preservation and retry.
- PASS: 409 data-free Rust tests, strict Clippy/Rust+Kotlin formatting/diff,
  real-Full Kotlin/JNI/seven Unicode exchanges/repeatable generated bindings, both-ABI APK/strict Android lint and
  eight API-36 x86_64 persistence/clock regressions. Reports/logs retained under
  .local/test-reports/epub-worker/. Exact commands/results: DEVELOPMENT.md end.
- Fixture fixes: refresh a fresh worker's Library, share its archive module for
  Clippy, use Kotlin unsigned counters and trim the expected-text file's final LF.
  No production parser change. Gradle's sandbox wildcard-IP lock-service failure
  required approved outside-sandbox execution. Final checks pass; no open blocker.
- No dependency/version/migration/new permission. Large-book double-parse/complete
  saved-content validation costs are unmeasured. Standalone Full/index/Python,
  release app/process-death/reboot/tablet/dark/physical arm64 were not rerun.
  Next: mixed EPUB/TXT/SRT picker/summary/Cancel/notifications, then reader assets.


## 2026-10-26 — Mixed Android EPUB/TXT/SRT batches

- Generalized retained FileImports/dialogs/progress/notification observer and wired
  wildcard multiple selection to case-insensitive literal filename routing. Reuse
  typed per-file events; SRT has its own encoding errors/choices, EPUB none.
- Reused direct off-main cancelFileImport and existing commit/outcome drain; no
  persisted queue, schema, dependency, permission or destructive behavior change.
- Added real mixed-picker fixture with complete EPUB duplicate references, invalid
  EPUB/SRT/excluded format isolation, encoding/Skip, exact stored formats/chapters,
  per-chapter Mode B, immediate registration/no encounters and no reader auto-open.
  Extended notifications and cancellation fixtures to SRT/EPUB; TXT regressions stay.
- PASS: 409 data-free Rust tests, Kotlin formatting, both-ABI debug/test APKs and
  strict lint; eleven distinct focused API-36 x86_64 tests pass. Initial mixed-test
  ambiguous text selector corrected; other ten passed unchanged, final mixed rerun
  passes. Reports: .local/test-reports/mixed-file-picker/{initial,final}/. Exact
  commands/limits: DEVELOPMENT.md end. Gradle device tests needed approved execution
  outside sandbox for its lock service. No open failure/blocker.
- Updated architecture/data/decisions/progress notes. Host JNI/standalone Full, Rust
  lint/fmt/codegen, index/Python, release/process-death/reboot/tablet/dark/physical
  arm64 were not rerun. Next: reader source formatting/ruby/images/links/viewer; CI
  independent. Large EPUB parse/integrity-read costs remain unmeasured.

## 2026-10-26 — EPUB reader text formatting and website links

- Added validated keyed chapter formatting and page-local scalar clipping to pure
  core; the repository now transports stored headings/emphasis/strong/website links
  through typed JNI to Compose without changing canonical text or saved anchors.
- Rendered styles and external browser links. Website tap areas preserve vocabulary
  highlighting; remaining token fragments retain full dictionary identity. Internal/
  other links stay inert; nested website targets remain disjoint. No CSS/TOC added.
- PASS: 412 Rust tests, strict Clippy/Rust+Kotlin formatting/diff, real-Full Kotlin/JNI,
  Unicode/formatting exchanges, repeatable Kotlin payloads, both-ABI debug/test APKs,
  strict Android lint and eleven distinct focused API-36 x86_64 tests. Reports:
  .local/test-reports/reader-formatting/. Exact commands/limits: DEVELOPMENT.md end.
- Fixed explicit Facet representation and lint's KTX URI helper. New UI test initially
  closed the reader through Activity Back; changed it to the dictionary sheet's Close
  button and readiness wait. Initial ten regressions and final single rerun pass.
- Updated architecture/data/progress/handoff. No migration/dependency/permission/
  destructive behavior change. No open blocker. Next: images/covers/shared viewer,
  then Noto/font/furigana/source-ruby rendering; CI independent. Large EPUB costs
  unmeasured; physical arm64 still deferred, other unrun checks in DEVELOPMENT.md.

## 2026-10-26 — EPUB reader image references

- Added pure keyed chapter image references and page-local placeholder projection;
  carry source order/alt/document-scoped asset paths/media through storage and typed
  JNI without image BLOBs. Validate ownership, scalar ranges, overlap, canonical
  keys and consistent shared metadata. Preserve text, positions and completion rules.
- Added four pure tests and one repository fixture; extended read-only/corruption,
  FIFO movement/reflow and actual EPUB/Unicode Kotlin-JNI exchanges. Android's
  synthetic ReaderPage fixture now supplies an empty image list.
- PASS: 417 data-free Rust tests, strict Clippy/Rust+Kotlin formatting/diff, real-Full
  JNI, repeatable Kotlin codecs, both-ABI debug/test APKs, strict lint and all 14
  focused API-36 x86_64 document/formatting/word-span/navigation regressions. Reports:
  .local/test-reports/reader-image-references/. Commands/limits: DEVELOPMENT.md end.
- Device Gradle's sandbox socket failed; approved outside-sandbox retry passes.
  Cold-started existing AVD without wiping data; it remains running. No open blocker,
  dependency/version, migration, permission or destructive-semantics change.
- Updated plan/architecture/data/decisions/handoff. Next: bounded off-main asset
  retrieval/decoding and source-position/ratio rendering, covers/shared viewer;
  Noto/font/furigana and CI remain next independent work. Large EPUB costs unmeasured;
  physical arm64 remains user-deferred. Other unrun checks: DEVELOPMENT.md end.

## 2026-10-26 — Bounded EPUB image asset retrieval

- Added read-only single-asset repository checks for live EPUB ownership, opaque
  key, expected media and SHA-256. SQL caps BLOB copies at the existing 32 MiB
  import-entry budget; source/unrelated BLOBs stay outside this narrow read.
- Added one-shot typed JNI asset callbacks on the existing Rust SQLite FIFO,
  outside pure core state. No dictionary/Render/history/position changes. Accepted
  reads drain after close; per-request listeners release after delivery/rejection.
- PASS: 425 data-free Rust tests, strict Clippy/Rust+Kotlin formatting/diff,
  real-Full host JNI/seven Unicode exchanges, repeatable native bindings, both-ABI
  debug/test APKs, strict lint and five API-36 x86_64 asset/document/formatting tests.
  Reports: .local/test-reports/epub-image-assets/; commands/limits: DEVELOPMENT.md end.
- Fixed only test assumptions about core renders after close, generated error
  names and Java Thread wrappers. Gradle required approved outside-sandbox socket
  access; final suites pass. Existing AVD remains running, without a wipe.
- Updated plan/architecture/data/decisions/README/handoff. No open blocker,
  dependency/version, schema, permission or destructive-semantics change. Next:
  bounded Android decoding/stale-result disposal/source-position rendering, covers
  and shared viewer; Noto/font/furigana and CI remain next. Large-book costs and
  physical arm64 remain unmeasured/deferred; other unrun checks in DEVELOPMENT.md.


## 2026-10-26 — Bounded inline EPUB raster images

- Added CoreSession-owned single-asset read/decode pipeline, bounds-first sampled
  BitmapFactory decoding, 16 MiB page pixel budget and shared current-page assets.
  Disposed/closed requests suppress late results; unpublished bitmaps recycle.
- Rendered verified U+FFFC source placements between existing annotated text slices,
  retaining source ratio, offsets, styles, word identities and website links.
  Added alt/loading/error/explicit retry; SVG remains a supported-data fallback.
- Six new image tests cover decoding/limits, Unicode placement, pipeline disposal,
  actual pixels/source order/ratio, links/words, reflow/recreation and image-only
  chapters. A stronger inert-text tap test caught Compose link target expansion;
  fixed with a reader-text-only minimum-target override and reran word regressions.
- PASS: both-ABI debug/test APKs, strict lint, Kotlin formatting/diff, initial 13
  and final 20 API-36 x86_64 tests (26 distinct). Reports/commands/failure details:
  .local/test-reports/epub-image-rendering/ and DEVELOPMENT.md end.
- Gradle/adb sandbox socket failures required approved outside-sandbox runs. Started
  existing AVD without wiping data; remains running. No open verification blocker,
  dependency/version, native API, schema, permission or destructive-semantics change.
- Updated plan/architecture/data/decisions/README/handoff. SVG rendering, cover cache,
  shared zoom/pan viewer, font/furigana and CI remain. No Rust/host JNI/release/
  process-death/tablet/dark rerun; physical arm64 remains user-deferred.

## 2026-10-26 — Shared full-screen image viewer

- Added a source-agnostic full-window Compose viewer and connected EPUB image taps,
  including images inside source website links. Reuses bounded page bitmaps with
  source-ratio drawing, centroid pinch zoom, clamped pan and accessible controls.
- Save placement/zoom/normalized source focus through rotation/recreation; loading
  stays modal, Back/Close dismiss, page changes dispose. Dialog pointer/key input
  resets reader idle accounting and cannot reach reader completion swipes.
- PASS: six new geometry/Compose cases on API-36 x86_64 phone/light and tablet/dark,
  including actual pixels/pinch/hardware Back/full-window sizing/rotation/chapter
  replacement. Eight existing image/formatting/navigation regressions also pass
  (14 distinct cases). Both-ABI debug/test APKs, strict lint and formatting/diff pass.
- Corrected test-only Back-dispatch and safe-inset measurement assumptions; final
  suites pass. Reports: .local/test-reports/image-viewer/; commands/limits/failures:
  DEVELOPMENT.md end. Gradle/ADB needed approved outside-sandbox sockets. Restored
  emulator size/density/light/free rotation; existing AVD remains running.
- Updated plan/architecture/data/decisions/README/handoff. SVG/cover cache remains
  next, then Noto/font/furigana and CI. No dependency/schema/FFI/permission change
  or open blocker. Zoom detail stays at sampled page resolution; unrun checks in
  DEVELOPMENT.md. Physical arm64 remains user-deferred.

## 2026-10-26 — Bounded static SVG images

- Added reviewed/pinned AndroidSVG 1.4 to the existing serial asset/decode pipeline,
  sharing page pixels, cancellation, Retry and the viewer. Streaming XML/reference
  preflight rejects excessive/recursive input and unbounded compound paint/image
  resources; no external resolver/entities. Fractional source geometry stays exact
  through inline layout and viewer fit/zoom/pan. Offline Settings ships full credits.
- PASS: both-ABI APKs, strict lint, Kotlin formatting/diff, 21 phone/light tests,
  final three SVG guard fixtures and 11 tablet/dark tests. Pixel/geometry/UTF-16/
  bounds/cycle fixtures and actual EPUB word/link/recreation/chapter/credits checks
  pass with existing raster/loader/viewer/navigation regressions. Reports:
  .local/test-reports/epub-svg/; commands/failures/limits in DEVELOPMENT.md end.
- Corrected UseKtx lint; restarted disconnected AVD without wipe and retried one
  pre-boot install failure. Final checks pass; original phone/light/free-rotation
  overrides restored. Approved sockets/downloads needed; no open blocker.
- Updated plan/architecture/data/decisions/README/handoff and renderer provenance.
  Remaining SVG compound resources/complex paint/stylesheet references and EPUB
  cover cache stay unchecked; font/furigana and CI follow. No core/FFI/schema/
  permission change. Total heap/large-book performance unmeasured; host/Rust/
  release/process-death checks not rerun, physical arm64 remains user-deferred.

## 2026-10-26 — Metadata-only EPUB cover references

- Added cover path/media/SHA-256 lookup in storage, existing FIFO and one-shot typed
  JNI; copies no source/chapter/image BLOBs and loads no dictionaries. Normal
  no-cover outcomes and damaged declared metadata are explicit; accepted callbacks
  drain after close. No core/schema/dependency/permission changes.
- PASS: 432 Rust tests (seven new), strict Clippy/formatting/diff, complete real-Full
  host JNI/Unicode exchanges, repeatable bindings, both-ABI debug/test APKs/strict
  lint and five API-36 x86_64 bridge/document/EPUB formatting regressions. Reports:
  `.local/test-reports/epub-cover-reference/`; commands/limits in DEVELOPMENT.md end.
- Fixed test import and corruption-fixture CHECK bypass; approved Gradle sockets
  needed outside sandbox. Final checks pass, no open blocker; existing AVD reused.
  Updated plan/architecture/data/decisions/README/handoff. Next: bounded thumbnail
  cache and Library display using metadata keys plus verified asset bytes. Remaining
  SVG resources/ruby stay open; physical arm64 remains user-deferred.


## 2026-10-26 — Library cover thumbnails

- Added lazy raster/static-SVG EPUB thumbnails and decorative generic fallback
  icons in compact Library rows. CoreSession retains a 4 MiB/64-entry content LRU
  across recreation; live metadata precedes hits, verified bytes plus frozen-digest
  comparison precede new decodes. Each cover uses <=64 KiB pixels on the shared
  serial reader decoder. No negative/disk cache or core/FFI/schema change.
- PASS: both-ABI APKs, strict lint/formatting/diff; six new cache/Compose cases on
  phone/light and tablet/dark, plus eight phone image/navigation regressions.
  Reports: .local/test-reports/epub-cover-thumbnails/; DEVELOPMENT.md end has exact
  commands, corrected test assertions, socket approval and emulator restart details.
- Restored phone/light/free rotation; no open blocker. Updated plan, architecture,
  data model, decisions, README and handoff. Remaining SVG compound resources stay
  unchecked; Noto/font/furigana and CI follow. Cache cap excludes visible/evicted
  bitmaps awaiting GC; process restart rebuilds. Rust/JNI/release/process-death and
  physical arm64 not rerun; arm64 remains user-deferred.


## 2026-10-27 — Bounded embedded SVG raster resources

- Added self-contained base64 PNG/JPEG decoding through the existing image/viewer/
  cover pipeline. Rewrote data URIs to private resolver keys; <=64 distinct sources
  share one additional asset pixel budget and recycle after render. Original-ratio
  viewport wrappers survive one-pixel sampling. A process-wide lock protects the
  upstream static resolver across CoreSessions; no resource I/O/source mutation.
- PASS: both-ABI APKs, strict lint/formatting/diff; 19 API-36 x86_64 cases each on
  phone/light and tablet/dark, including six new decoder/reader cases, cover pixels/
  recreation and existing image/navigation regressions. Reports:
  .local/test-reports/epub-svg-embedded/; DEVELOPMENT.md end has exact commands.
- Corrected a test pixel coordinate; final phone/tablet runs have zero failures or
  skipped cases. Approved source download/sockets needed; existing AVD restarted
  without wipe and restored to phone/light/free rotation. No open blocker.
- Updated plan, handoff, architecture/data, decisions, development and provenance.
  Remaining local/nested resources, complex paint and selector-aware CSS stay
  unchecked; stylesheet-styled embedded SVGs retain alt/Retry. Font/furigana and
  CI follow. No dependency/core/FFI/schema/permission changes. Rust/JNI/release/
  process-death not rerun; physical arm64 remains user-deferred.

## 2026-10-27 — Bounded self-contained nested SVG images

- Added base64 SVG children to the existing reader/viewer/cover decoder. Unique
  child vectors/rasters share aggregate XML/node/expansion/resource/pixel limits,
  render children before parents and retain independent IDs/styles. Intermediate
  bitmaps recycle after render under the existing global resolver lock. No new
  dependency, core/FFI/schema/permission or saved-asset changes.
- PASS: both-ABI APKs, strict lint/formatting/diff and all 25 API-36 x86_64 cases
  each on phone/light and tablet/dark. Six new decoder/real-EPUB cases plus expanded
  cover/concurrency fixtures check pixels, bounds, sharing, links, recreation and
  navigation. Reports: `.local/test-reports/epub-svg-nested/`; commands and limits
  in DEVELOPMENT.md end.
- Initial decoder run passed 12/13 and caught permissive acceptance of a truncated
  child root; explicit single/closed-root preflight fixes it. Approved ADB/Gradle
  sockets needed; existing AVD reused and restored to phone/light/free rotation.
  Final suites have zero failures/skips; no open blocker.
- Updated plan/handoff, architecture/data, decisions, development, README and
  renderer provenance. Relative/local/other nested image URI forms, complex paint
  and selector-aware CSS remain unchecked; fonts/furigana and CI follow. Rust/JNI/
  release/process-death not rerun; physical arm64 remains user-deferred.

## 2026-10-27 — Accept basic SVG scope

- Recorded the user's clarification that basic SVG support is sufficient for v1.
  Removed advanced SVG resources/styles from the active plan without marking
  them implemented; retained existing decoding and unsupported-image fallbacks.
- Updated product spec, plan, handoff, README, architecture/data and renderer
  scope notes; fonts/line spacing/default-off furigana are next.
- Documentation-only: git diff check passed; app tests/builds were not rerun.
  No blocker or runtime change; previous verified image results remain recorded.


## 2026-10-27 — Bundled Noto Serif JP reader font

- Bundled the unmodified pinned Noto Serif JP 2.003 variable font with complete
  offline Settings attribution/SIL OFL 1.1. Shared reader text blocks use explicit
  regular/bold axes and synthesized EPUB italic, including fragments around images.
  Recorded source revision, upstream metadata and SHA-256; no new dependency,
  core/FFI/schema/settings or saved-text/position change.
- PASS: both-ABI APKs, strict lint/formatting/diff, 23 distinct phone/light cases
  across the broad run and corrected three-case rerun, plus eight tablet/dark
  cases. Glyph outlines match direct platform font instances; Unicode/recreation,
  credits, EPUB formatting/images, word/link taps and navigation stay correct.
- Fixed two new test-fixture assumptions (variable default-weight metadata and
  compressed resource access); final tests pass. One disconnected-device attempt
  ran zero tests; restarted the existing swangle/KVM AVD without wiping data.
  Reports: `.local/test-reports/reader-font/`; commands in DEVELOPMENT.md end.
- Restored phone/light/free rotation; no open blocker. Updated plan/handoff,
  architecture, decisions, README and font provenance. Persisted size/line spacing
  and source/analyzer furigana remain unchecked. Rust/JNI/release/process-death
  checks not rerun; physical arm64 remains deferred.


## 2026-10-27 — Persist global reader font size and line spacing
- Started clean. Added pure finite typography preferences, migration 0015, atomic
  SQLite saves/validation, consistent reader snapshot restoration and typed
  core/FIFO/generated JNI transport. Settings publishes committed choices only.
- Compose applies size/proportional line height in every reader text fragment;
  Noto, system theme/status colors, logical pages/offsets, images and taps persist.
- PASS: eight new Rust cases; 440 workspace tests/strict Clippy/fmt; real-Full
  Kotlin/JNI all 20 pairs/reopen; repeatable codegen; both-ABI debug/test APKs,
  strict lint/Kotlin formatting/diff; 12 distinct phone + four tablet/dark cases.
- Fixed historical schema fixtures and two new Android fixture assumptions
  (normal reader departure; token identity versus list index). A preexisting
  worker lifecycle timing assertion passed isolation/final full rerun. Reports
  and exact commands: DEVELOPMENT.md end and .local/test-reports/reader-typography/.
- Updated README/architecture/data/decisions/plan/handoff. Restored emulator to
  phone/light/free rotation. No blocker; release/process-death/physical arm64
  not rerun. Next: default-off source/analyzer furigana; CI remains independent.


## 2026-10-27 — Source ruby reader transport and clipping
- Started clean. Added pure owned/keyed ReaderChapterRuby and clipped page-local
  ReaderRuby through reader snapshots/FIFO/generated JNI. TXT source/EPUB content/
  TXT reader loads share read-only ordered-index/range/reading validation.
- Preserve complete source readings across logical-page boundaries, canonical
  text/exact offsets and all vocabulary/analysis/history behavior. No schema,
  parser, dependency or visible reader changes.
- PASS: six new Rust cases; 446 workspace tests (16 opt-in ignored), strict
  Clippy/fmt, real-Full Kotlin/JNI/Unicode round trips/repeatable codegen,
  both-ABI APKs/lint/Kotlin formatting and nine API-36 phone cases.
- Fixed two missing imports in new Android assertions; Gradle required approved
  outside-sandbox local sockets. Reports/commands: DEVELOPMENT.md end and
  .local/test-reports/reader-ruby/. Existing phone/light AVD remains running.
- Updated README/architecture/data/decisions/plan/handoff. No blocker. Furigana
  switch/persistence, analyzer fallback and visible layout/taps/themes remain
  unchecked. Release/process-death/physical-arm64 not rerun; arm64 deferred.


## 2026-10-27 — Persist default-off furigana preference
- Started clean. Migration 0016 adds constrained 0/1 furigana to global typography;
  existing core/FIFO/generated JNI loads/saves it atomically with font/spacing.
  Keep the visible switch for the source-preferred analyzer/layout step.
- Three new Rust cases plus expanded fixtures cover migration/retry/preservation,
  all 40 combinations/reopen, corrupt flags and statement/COMMIT rollback, routing
  gates and unchanged reader state. Existing Android controls preserve the flag.
- PASS: 449 Rust tests (16 opt-in ignored), strict Clippy/Rustfmt/Kotlin formatting,
  real-Full host JNI/Unicode/repeatable codegen, both-ABI debug/test APKs/lint and
  both API-36 phone typography/image/word-tap cases (2m11s final Gradle run).
- Updated a remaining old schema-version assertion after the first Rust runs;
  final full workspace passes. Gradle required approved outside-sandbox sockets.
  Cold-started existing swangle/KVM AVD without wipe; phone/light remains running.
- Updated README/architecture/data/decisions/plan/handoff. Reports and commands:
  DEVELOPMENT.md end; .local/test-reports/reader-furigana-preference/. No blocker.
  Visible switch/source-preferred analyzer fallback/ruby rendering remain next.
  Tablet/dark/release/process-death/physical-arm64 not rerun; arm64 deferred.


## 2026-10-27 — Source-preferred furigana projection
- Started clean. Added pure ReaderPage::furigana and derived ViewModel.reader_furigana
  through generated JNI. Enabled projection prefers source bases over all overlapping
  original B token readings, retains literal complete readings and ignores statuses.
- Validate full keys/owned anchors/exact clipped ranges/canonical fragments; stale
  or malformed analyzer snapshots retain source ruby alone. Existing refresh/reflow/
  exit invalidation applies. No I/O/schema/dependency/text/history changes.
- PASS: four new Rust cases; 453 workspace tests (16 opt-in ignored), strict
  Clippy/Rustfmt/Kotlin formatting, full real-Full host JNI/repeat codegen,
  both-ABI debug/test APKs/lint and ten API-36 phone transport/span/typography cases
  (3m03s). TXT JNI checks source-before-tokens, source precedence, analyzer fallback,
  status independence and fresh-session dictionary-free restoration.
- Fixed old explicit ViewModel fixture/new reflow fixture and Clippy suggestions;
  Gradle/ADB sockets needed approved outside-sandbox execution. Existing phone AVD
  remains running, unchanged. Commands/results: DEVELOPMENT.md end and
  .local/test-reports/reader-furigana-projection/. No blocker.
- Updated README/architecture/data/decisions/plan/handoff. Next: visible switch and
  Compose ruby layout from reader_furigana, then anchor/word/link/theme checks.
  Tablet/dark/release/process-death/physical-arm64 not rerun; arm64 deferred.


## 2026-10-27 — Furigana switch and Compose layout
- Started clean. Exposed the persisted default-off Settings switch and wired the
  source-preferred projection into canonical-text ruby paint, including image fragments.
- Strict UTF-8/UTF-16 validation; half-size literal readings fit base width above
  measured styled lines. Visual/page/image splits keep complete readings. Canonical
  text, anchors, word/link identities, status semantics and I/O remain unchanged.
- Four new device cases cover projection/Unicode, wrapped heading pixels/bounds,
  font scaling/themes, switch failure/retry/recreation, all statuses, EPUB word/link/
  image taps and TXT page crossing/reflow. Existing typography assertion permits
  enabled headroom; disabled typography remains exact.
- PASS: 16 distinct phone cases, six tablet/dark cases (1m50s), both-ABI debug/test
  APKs, strict lint, Kotlin formatting/Rustfmt/diff. Final 11-case phone regression
  run: 2m42s. Full Rust tests/Clippy/host JNI/codegen unchanged and not rerun.
- Fixed test TextRange/activity setup and a paragraph-first pagination fixture;
  no production changes after initial build. Gradle sockets required approved
  outside-sandbox execution. Cold-started existing swangle/KVM AVD without wipe;
  restored phone/light after tablet. Reports/commands: DEVELOPMENT.md end and
  .local/test-reports/reader-furigana-layout/. No blocker.
- Updated README/architecture/data/decisions/plan/handoff. Next: CI, missing-match
  custom definitions/readings, then persisted segmentation overrides. Release,
  process-death and physical-arm64 not rerun; arm64 remains user-deferred.

## 2026-10-27 — Remove CI task at user request
- Removed CI/GitHub Actions from the implementation checklist and handoff; updated
  the product specification and recorded the scope decision. Local verification remains.
- Next: custom meanings/readings for missing dictionary matches, then segmentation overrides.
- Documentation only; verified the diff and active task lists. No tests run or blockers.


## 2026-10-27 — Custom dictionary persistence
- Started clean. Added pure validated custom meaning/reading values and migration
  0017 exact-surface storage, separate from built-in dictionaries and registrations.
- Selected-token CRUD reuses original Mode B/page ownership validation, rechecks
  immutable surface/base word/name matches, preserves literal Unicode and performs
  atomic/no-op retries. No analyzer, vocabulary/history, time or position changes.
- PASS: 11 new Rust cases; 464 workspace tests (16 opt-in ignored), strict Clippy/
  Rustfmt/Kotlin formatting, real-Full host JNI/repeat codegen, both-ABI debug/test
  APKs/lint and 11 phone regressions (5m52s). Migration fixtures updated to 17/18.
- Fixed two test fixture mistakes (chapter slice and distinct-surface count). Gradle
  sockets required approved outside-sandbox execution. Reports/commands:
  DEVELOPMENT.md end and .local/test-reports/custom-dictionary-storage/. No blocker.
- Updated plan/architecture/data/decisions/handoff. Runtime/FFI/editor remains open;
  next wire typed custom results/mutations with selection/retry gates, then Android
  missing-match editing/deletion. Existing phone AVD unchanged and running.
  No tablet/dark/release/process-death/physical-arm64 rerun; hardware deferred.


## 2026-10-27 — Custom dictionary core/FIFO/FFI transport
- Started clean. Same-transaction selected-word reads now include validated custom
  entries only for successful empty built-in matches. Added literal string DTO and
  explicit save/delete core events/effects on the existing FIFO/generated Kotlin/JNI.
- Current missing-match selection/shared save gates and repository built-in rechecks
  protect mutations; callbacks reject retired selections and foreign surfaces.
  Failures preserve committed entries with a separate explicit-retry error; both
  outcomes drain final clock saves/departures. No schema/dependency/reader semantics change.
- PASS: six new Rust cases, 470 workspace tests (16 opt-in ignored), strict Clippy/
  formatting, real-Full host JNI/repeat codegen, both-ABI debug/test APKs and lint.
  New Android transport verifies Unicode CRUD/retry/reopen and built-in protection.
- Thirteen distinct phone cases pass: nine before an AVD disconnect, then four
  continuation cases (2m24s) after restart. Initial post-boot APK install failed for
  storage; direct retry succeeded without cleanup or wipe. No code/test weakening.
  Gradle/ADB sockets required approved outside-sandbox execution; no downloads.
- Updated prior explicit ViewModel/projection fixtures and corrected new dictionary/
  no-op-callback fixtures before passing checks. Commands/failures: DEVELOPMENT.md
  end; preserved reports: .local/test-reports/custom-dictionary-transport/.
- Plan/architecture/data/decisions/handoff updated; next Android missing-match
  editor/edit/delete + explicit retries, then segmentation overrides. No blocker.
  AVD running at original phone size; no tablet/dark/release/process-death/physical
  arm64 rerun; hardware remains user-deferred.

## 2026-10-27 — Android custom dictionary editor
- Started clean. Added missing-match Add/Edit/Delete controls with required literal
  meaning/reading fields and Cancel in the existing phone sheet/wide popup. Built-in
  word/name matches and failed lookups never offer custom writes.
- Selection-keyed drafts/failed commands survive clock/status updates; shared save
  gates disable fields/mutations. Explicit exact-command Retry retains drafts and
  committed entries after failed save/delete. IME edits record reader interaction.
- PASS: both-ABI debug/test APKs, strict lint, Kotlin formatting/Rustfmt/diff and
  13 distinct API-36 phone cases across runs; all four focused tablet/dark cases
  pass (1m49s). Three new tests cover Unicode/validation/Cancel/CRUD/recreation,
  cross-occurrence/book reuse, built-ins/errors, pending gates, real SQLite failures
  and unchanged tokens/position/vocabulary/status/completion counts.
- Corrected two fixture errors: isolated editor lacked a scroll container; enhanced
  cross-book import skipped closing/draining reader time. Final single phone case
  passes (1m06s). All corrections were in test setup. Gradle local sockets required
  approved outside-sandbox execution; no downloads. Logs/reports/lint XML:
  .local/test-reports/custom-dictionary-editor/; exact commands: DEVELOPMENT.md end.
- Updated plan/architecture/data/decisions/handoff. Next effective A/B/C/manual
  overrides using existing anchors, then typed transport/UI. No blocker; AVD
  restored to original 1080x2400/420dpi/light and running. No Rust/core/FFI/schema/
  dependency changes; Rust suite/Clippy/host JNI/codegen, release, benchmarks,
  process-death and physical arm64 not rerun; hardware stays deferred.

## 2026-10-27 — Effective segmentation boundary resolver
- Started clean. Added a pure original-B resolver for migration-0007 manual/A/B/C
  partitions and same-profile A/B/C partition construction. Literal contiguous
  Japanese B token matching, local/newer-ID overlap precedence and reversal retain
  immutable snapshots; changed boundaries carry no invented analyzer metadata.
- Added one read-only SQLite chapter-segmentation API, with pinned B, retained
  global provenance, exact local scopes and consistent active-rule loading. Current
  reader/word/furigana/completion APIs remain original B; no schema/dependency or
  public bridge change. Updated architecture/data/decisions/plan and brief handoff.
- PASS: 13 new data-free cases, 483 workspace Rust tests (17 opt-in ignored), strict
  Clippy/Rustfmt, Kotlin formatting/diff, full real-Full host Kotlin/JNI and
  byte-identical repeat Kotlin generation.
  All 15 Full analysis + one runtime fixture pass for each owned/mapped loader;
  new real-data coverage exercises A/B/C/conjugation merge/local reversal/reopen.
  History tests compare complete persisted rows/values before/after edits/reversal.
- Fixed a nonexistent table name in the new test snapshot and Clippy's borrowed-
  singleton assertion request before passing checks. No blocker or new approval
  requirement. Commands/results: DEVELOPMENT.md end. Android APK/device/lint,
  release, performance, process-death and physical arm64 were not rerun; AVD untouched.
- Next effective token/word/metadata/registration and frozen future-completion
  identities, then core/FIFO/FFI/Android choices and controls. Keep past occurrences
  unchanged and All occurrences as the default scope.

## 2026-10-27 — Effective segmentation metadata provenance

- Added pure effective chapter/segment metadata and a read-only SQLite loader.
  Exact source spans prefer original B, then same-profile A/C, preserving each
  occurrence's AnalysisId/mode/token index, classification, reading and lemma.
  Arbitrary manual spans remain explicitly unresolved; no metadata is guessed.
- Five new data-free cases plus extended real Full owned/mapped coverage verify
  contextual repeated readings, profile/ID/mode validation, corrupt/missing data,
  irrelevant later profiles, global/local reversal, reopen, query-only bytes and
  unchanged original reader tokens and historical rows. No schema/dependency,
  public bridge, analyzer/registration or live completion changes.
- PASS: 488 workspace tests (17 ignored), strict Clippy/Rustfmt/Kotlin formatting/
  diff, 15 Full analysis + one runtime fixture for each owned/mapped loader,
  real-Full host Kotlin/JNI and byte-identical repeat Kotlin generation.
  Fixed a test closure's inferred unsized argument. An existing runtime teardown
  assertion failed once; isolated and full reruns pass without runtime changes.
- Updated plan, architecture/data model, decisions and 80-line handoff. Full
  commands/results/log paths: DEVELOPMENT.md end. No unresolved blocker; Android
  APK/device/lint, release, process-death and physical arm64 were not rerun; AVD
  untouched. Next: effective eligibility/token/word/registration plus frozen future
  completions, then typed core/FIFO/FFI and Android segmentation controls.

## 2026-10-27 — Effective reader eligibility and page/status projection

- Added a pure validated effective reader chapter with explicit analyzer versus
  manual Japanese-span eligibility, complete metadata/provenance, effective indices,
  UTF-8 page clipping and start-owned future-occurrence projection. Manual terms
  remain selectable/countable without fabricated analyzer classification or forms.
- Added a separate atomic read-only effective page/live-status repository, sharing
  chapter-resolution loading and indexed lookups for repeated spellings. Newly
  partitioned/deleted entries remain absent; original live reader/completion stays B.
- PASS: seven new data-free cases, 495 workspace Rust tests (17 ignored), strict
  Clippy/Rustfmt/Kotlin formatting/diff, real-Full host Kotlin/JNI, byte-identical
  repeat generation and 15 Full + one runtime fixture EACH owned/mapped. Fixtures
  cover exact classification, Unicode/repetition/all-preset crossings, ownership,
  corrupt off-page data, status/deletion/reversal/reopen and unchanged bytes/rows.
- Corrected test-only preset names/import and a crossing fixture that originally
  aligned with paragraph/sentence page boundaries. No production workaround,
  dependency/schema/bridge/Android changes or open blocker. Updated architecture,
  data model, decisions, plan and brief handoff; commands/logs: DEVELOPMENT.md end.
  APK/device/lint/release/process-death/physical arm64 not rerun; AVD untouched.
- Next: effective selected-word/registration and frozen future-completion identities,
  live reader/furigana integration, then typed commands and Android controls.

## 2026-10-27 — Effective selected-word lookup

- Added a frozen effective document/page/resolution key and a separate read-only
  selected-word repository. Fresh resolution/provenance must match before accepting
  the segment index; override replacement/reversal and metadata changes reject old
  selections. Live statuses/counts/custom entries do not invalidate the key.
- Shared original/effective dictionary/count/custom composition. Exact analyzer
  metadata alone supplies base forms; manual spans use surface-only lookup without
  guessed readings/lemmas. New/deleted vocabulary remains absent; no registration,
  history, schema, production dependency, bridge or live-reader behavior change.
- PASS: six new data-free cases, 501 workspace tests (17 ignored), strict Clippy/
  Rustfmt/Kotlin formatting/diff, repeat codegen and 15 Full + one runtime fixture
  EACH owned/mapped. Real Full fixtures now verify effective word details/reopen;
  already pinned rusqlite 0.40.2 is linked as an analysis test dependency only.
- Corrected one fixture assertion about retained base==surface dictionary-form
  payloads. Updated plan, architecture/data model, decisions and 80-line handoff.
  Commands/results/logs: DEVELOPMENT.md end. APK/device/lint/release/performance/
  process-death/physical arm64 not rerun; AVD untouched. No implementation blocker.
- Final host Kotlin/JNI passes selected-word/custom CRUD, Unicode/codecs,
  imports/cancellation, navigation and handle lifecycle regressions. Known Crux
  teardown/ktfmt JDK messages remain informational; no escalation/download needed.
- Next: effective registration/custom mutations and frozen future-completion
  identities, live token/furigana/word integration, then typed commands/UI controls.

## 2026-10-27 — Effective custom-entry CRUD

- Added effective custom-entry load/save/delete using the frozen resolved selection
  and shared transaction-local validation with selected-word reads. Immediate
  mutations keep resolution checks, built-in protection and writes atomic.
- Shared original/effective dictionary gates and custom row helpers. Exact metadata
  alone supplies base forms; manual terms remain surface-only. Literal fields,
  global spelling identity, absent/deleted vocabulary, write-free retries, corrupt
  state rejection and rollback are preserved. No registration/history/metadata,
  dependencies, migrations, bridge or live-reader changes.
- PASS: seven new cases, 508 workspace tests (17 ignored), strict Clippy/Rustfmt/
  Kotlin formatting/diff, repeat codegen and 15 Full + one runtime fixture EACH
  owned/mapped. Real Full now checks effective custom CRUD/reopen and stale writes.
  Host Kotlin/JNI selected-word/custom CRUD, imports/cancellation, Unicode/codecs,
  navigation and handle lifecycle regressions pass. No failing check or blocker.
- Updated plan, decisions, architecture/data docs, DEVELOPMENT.md commands/logs and
  80-line handoff. APK/device/lint/release/performance/process-death/physical arm64
  were not rerun; AVD untouched. No escalation/download needed.
- Next: effective registration and frozen future completions, then live reader/
  furigana/word integration and core/FIFO/FFI/Android segmentation controls.


## 2026-10-27 — Frozen effective completions

- Added opaque pure effective completion values and migration-0018 durable command
  reservations. Preparation validates full resolved identity and drained time; commit
  authenticates exact payloads and preserves frozen occurrences across rule changes.
- Effective completion shares original-B page-only registration/New-only status changes
  and atomic history/time/position/finish. Exact replay survives deletion/reopen; legacy
  calls reject effective IDs. Past histories and live B reader behavior stay intact.
- PASS: nine new storage cases; 517 workspace tests (17 ignored), strict Clippy,
  formatting/diff, 15 Full + one runtime fixture EACH owned/mapped. Tests cover stale
  keys, exact/unresolved metadata, Unicode/all presets, deleted vocabulary, concurrency,
  rereads, migration, statement/deferred-commit failures and exact retries.
- Updated plan/decisions/architecture/data model and handoff. DEVELOPMENT.md records
  commands, logs and corrected fixture failures. No dependency/version/public bridge
  changes. APK/device/lint/release/performance/process-death/physical arm64 not rerun.
- Next: live effective completion/token/furigana/word/custom integration, payload-cost
  review, atomic override-time registration and typed override controls/UI. No blocker.
- Final host Kotlin/JNI and byte-identical repeat codegen pass; seven cross-language
  fixtures and 100 handle lifecycles verify. Existing Crux teardown messages remain
  informational. No escalation/download needed.


## 2026-10-27 — Atomic segmentation-override registration

- Added registered-override save sharing transaction-local rule persistence with
  the original analysis-only API. Validate live Japanese B spans/full effective
  snapshots, replace the rule and register selected eligible winning parts atomically.
- Existing statuses/history survive; identical active-rule retries write nothing
  after deletion/reopen. Reversal retains registration/history. Other occurrences
  are not rescanned for registration; exact analyzer classification takes priority
  and unresolved manual spans acquire no guessed reading/lemma.
- PASS: seven new cases, 524 workspace Rust tests (17 ignored), strict Clippy,
  Rustfmt/diff, 15 Full + one runtime fixture EACH owned/mapped. Verified Unicode,
  concurrency, local precedence, stale/corrupt inputs, nonempty history/time/position/
  finish preservation and statement/deferred-COMMIT rollback. No failing check remains.
- Updated plan/decisions/architecture/data model, commands and 80-line handoff.
  No dependencies/schema/bridge/Android changes. Kotlin/JNI/codegen/APK/device/lint/
  release/performance/process-death/arm64 not rerun; no escalation/download needed.
- Next: live effective completion/token/word/custom/furigana integration and payload
  review, then typed override controls with rendered-key/intervening-edit retry guards
  and Android A/B/C/manual scope choices. Current live reader stays B; no blocker.

## 2026-10-27 — Effective furigana and payload review

- Added pure source-preferred furigana over the private validated effective chapter;
  original and effective projections share one overlap helper. Exact B/A/C readings
  retain occurrence provenance; unresolved manual parts never borrow old B readings.
- Five new tests verify source overlaps, reversal, repeated/literal Unicode readings,
  all-preset crossing bases/UTF-16 and invalid page/whole-source rejection. Extended
  real Full A/B/C fixtures verify the same projection and source priority.
- PASS: 529 workspace Rust tests (17 ignored), strict Clippy/Rustfmt/Kotlin formatting,
  15 Full analysis + one runtime fixture EACH owned/mapped (32 invocations).
- Added reproducible payload example and EFFECTIVE_READER_PAYLOAD.md: 100k synthetic
  characters produce 14.3 MB of complete-key JSON versus 42.5 KB of page tokens.
  Actual JNI representation, real-book memory and device timing remain unmeasured.
- Updated plan/decisions/architecture/data model and handoff. No dependency/version,
  schema or Android source changes. APK/device/lint/release/process-death/arm64 not
  rerun. Next: narrow effective bridge identity/lifecycle, coordinated live reader
  integration, then guarded override controls. Current live reader stays B.
- Final host Kotlin/JNI passes real-Full selected-word/custom CRUD, file imports/
  direct cancellation, navigation/codecs, seven Unicode fixtures and 100 handles.
  Repeat codegen is byte-identical. Existing Crux teardown messages are informational;
  no verification blocker remains.

## 2026-10-27 — Compact effective reader identity/lifecycle

- Added a pure core retained-page component with compact revision/document/page/B
  identity, Rust-only load attempts, immediate invalidation and visible effective
  selection guards. Full provenance stays in one Rust key; accepted commands can
  retain an independent frozen key across drains/retries. No persistent/global cache.
- Added compact page/selection schemas to standalone Kotlin generation and literal
  Unicode codec fixtures. Missing B stays unselectable; manual parts keep eligibility
  without guessed readings/base. Live app paths still use B; integration is next.
- PASS: eight new Rust cases, 537 workspace Rust tests (17 ignored), strict Clippy,
  Rustfmt/Kotlin formatting, 15 Full + one runtime EACH owned/mapped, repeat codegen.
- Synthetic compact page JSON stays 32,983 B at 10k/100k chapter characters versus
  full keys of 1.4/14.3 MB. Actual Bincode available-B identity/selection = 60/68 B.
  Cross-language fixtures check maximum unsigned values, literal Unicode and exact
  metadata. Initial Kotlin enum-name assertions were corrected before the rerun.
- Updated payload lifecycle obligations, architecture/decisions/plan/development
  and 80-line handoff. No dependencies/schema/Android source changes; APK/device/
  Android lint/release/performance/process-death/arm64 not rerun. No blocker.

## 2026-10-27 — Native effective-reader FIFO protocol

- Routed effective tokens/word/custom CRUD and completion preparation/commit through
  the existing Rust storage capability, FIFO/connection and lazy dictionary. Full
  keys/reservations have no shell codec; current live app remains original B.
- Added private-field page/selection commands sharing immutable retained keys via
  Arc. Accepted retries survive page invalidation without chapter-sized key clones;
  storage still revalidates current provenance before selection writes/preparation.
- PASS: seven new core/worker/FFI tests, 544 workspace Rust tests (17 ignored), strict
  Clippy/Rustfmt/Kotlin formatting/diff, 15 Full + one runtime EACH owned/mapped,
  real-Full Kotlin/JNI/seven Unicode/100 handles and identical repeat generation.
- Worker tests cover no analyzer loads, read-only bytes, unresolved manual metadata,
  live statuses, literal custom data, built-in/corrupt dictionary protection, explicit
  repair/retry, Missing B/deletion, stale keys, blocked final-time FIFO order, atomic
  rollback, reversal/reopen exact retry, no startup replay and deliberate rereads.
- Updated architecture/data/payload/decisions/plan/development/handoff. No dependency,
  schema or Android source change. APK/device/lint/release/performance/process-death/
  arm64 not rerun. Next: coordinated live core/FFI/Android switch, guarded override
  commands, then A/B/C/manual controls. No implementation blocker.

## 2026-10-27 — Retained effective word/custom lifecycle

- Added pure session-owned selected-word/custom state and compact generated details.
  Native requests retain shared keys and literal actions; fresh callback attempts
  reject stale/duplicate results, including callbacks from an earlier retry.
- Added exact save/delete retries, committed-entry preservation, explicit Cancel
  after failure, dismissal/invalidation guards and live status/deletion observation.
  Word/name matches and failed lookups cannot enable custom writes. No SQL in core.
- PASS: seven new pure cases plus one real FIFO/SQLite failure/retry case; 552
  workspace Rust tests (17 ignored), strict Clippy/Rustfmt/Kotlin formatting/diff,
  15 Full + one runtime EACH owned/mapped, repeat codegen and real-Full host JNI.
  Standalone Kotlin codec checks include literal Unicode/custom/error values and
  maximum unsigned counts. Fixed initial test-path/import and Kotlin warning errors.
- Updated architecture/data/payload/decisions/plan/development and 80-line handoff.
  No dependency/schema/live Android changes. Live reader remains original B;
  coordinated core/FFI/Android switch is next, followed by guarded override controls.
  APK/device/lint/release/performance/process-death/arm64 not rerun. No blocker.

## 2026-10-28 — Retained effective completion lifecycle

- Added pure ownership from accepted rendered key/intent through final-time drain,
  reservation and exact commit retries. Fresh callback attempts reject duplicates,
  stale/wrong-stage results and changed reservations. Word/custom obligations gate
  completion; pending completion gates word/custom work. No completion Cancel.
- Added seven pure cases plus one real FIFO/SQLite case for checkpoint ordering,
  preparation/commit rollback, reversal/reopen and a lost successful response.
  Replay preserves one completion/six occurrences/2345 ms without duplicate effects.
- PASS: 560 workspace Rust tests (17 ignored), strict Clippy/Rustfmt/Kotlin
  formatting, 15 Full + one runtime fixture EACH owned/mapped. Corrected initial
  enum/size/assertion issues; reran Full after an interrupted server restart.
- PASS: real-Full Kotlin/JNI imports/cancellation/word/custom, seven Unicode
  fixtures, 100 handle lifecycles, byte-identical repeat generation and diff check.
- Updated architecture/data/payload/decisions/plan/development/handoff. No dependency,
  schema or live Android changes. Live reader remains B; coordinated adoption is
  next. APK/device/lint/release/performance/process-death/arm64 not rerun. No blocker.


## 2026-10-28 — Effective manual status commands and retained lifecycle

- Found the remaining original-B-only manual status path while reviewing the live
  switch. Added native effective status operations on the existing FIFO, sharing
  the manual status/history helper under transaction-local full-key/visible-word/
  exact-surface-ID validation. Deleted entries stay absent; replacement IDs reject.
- Retained frozen choices/timestamps across invalidation and explicit exact retries
  with fresh callback attempts; success updates repeated spellings/selected details.
  Word/custom/completion obligations mutually gate. No dictionary loads or reading
  writes; same-status/lost-success retries add no duplicate history.
- PASS: six pure, four storage and one actual worker case; 571 workspace Rust tests
  (17 ignored), strict Clippy/Rustfmt/Kotlin formatting, 15 Full + one runtime fixture
  EACH owned/mapped, real-Full host Kotlin/JNI, seven Unicode fixtures, 100 handles,
  byte-identical repeat generation and diff check. No initial verification failures.
- Updated plan/handoff/architecture/data/decisions/payload/development. No dependency,
  migration, shell schema or live Android change. Next adopt token/status/word/custom/
  completion/furigana together. APK/device/lint/release/performance/process-death/
  physical arm64 not rerun; no implementation blocker.

## 2026-10-28 — Guarded effective boundary repositories

- Continued the segmentation task with a small storage slice before the live switch:
  read-only stored A/B/C choices/winning rules and atomic guarded save/register/reverse.
  Native snapshots live in pure core and share full keys via Arc; no shell codec.
- Added an intervening-rule revision to reject old prompts after split/reversal even
  when the rendered key returns to B. Concurrent connections cannot overwrite a
  committed winner. Failed transactions retry; successful mutations require explicit
  reload. Same-rule no-ops never recreate deleted vocabulary.
- PASS: 12 new SQLite cases; 583 workspace Rust tests (17 ignored), strict Clippy and
  Rustfmt. All 15 Full + one runtime fixture EACH owned/mapped; extended real A/B/C
  segmentation fixture also rerun after the pure type move. Host Kotlin/JNI passes
  real imports/cancellation/word/custom, seven Unicode fixtures and 100 handles.
- Corrected deletion/pending-time test fixtures and documentation EOF whitespace.
  Updated plan/handoff/architecture/data/decisions/development. No dependency,
  migration, shell schema or Android source change. Live reader still uses B.
  Next: coordinated live switch, retained guarded-edit adapters/on-demand A/C,
  then Android controls. Device/APK/lint/release/arm64 not rerun; no blocker.

## 2026-10-28 — Retained guarded override requests

- Added native retained prompt/stored-choice/manual-save/reverse requests through
  the existing FIFO. Frozen context/parts/scope/timestamp survive invalidation and
  explicit retries; fresh attempts reject duplicate callbacks. Dismissed reads
  drain without restoring prompts. Word/custom/status/completion obligations gate.
- Stale contexts and successful writes require explicit reload; a lost-success
  retry never recreates deleted vocabulary. Failed-write Cancel retires the page;
  in-flight work drains. No analyzer/dictionary loads, reading/time/position writes,
  shell schema, migration, dependency or live Android change.
- PASS: seven pure + two worker cases; 592 Rust tests (17 ignored), strict Clippy/
  Rustfmt/Kotlin formatting, 15 Full + one runtime fixture EACH owned/mapped, host
  Kotlin/JNI, repeat generation and diff check. Fixed test helper/shadowing/column
  errors before the passing reruns; existing middleware/JDK messages informational.
- Updated progress/architecture/data/decisions/payload/development. Next coordinated
  live reader switch, on-demand A/C, then compact Android override controls.
  APK/device/Android lint/release/performance/process-death/arm64 not rerun; no blocker.

## 2026-10-28 — Guarded on-demand A/C prerequisite

- Added explicit retained A/C generation through the existing FIFO/import analyzer
  cache and pinned-profile native adapter. Stored alternatives bypass Full; missing
  modes analyze the whole chapter outside SQLite and revalidate context at commit.
- Generation registers no vocabulary or reading data. New metadata requires page/
  context reload; fresh retry attempts retain mode/context, reject stale/lost-success
  rebasing and preserve previously reserved completion meanings after reopen.
- PASS: 13 new cases; 605 Rust tests (17 ignored), strict Clippy/format, 15 Full +
  one runtime fixture EACH owned/mapped, host Kotlin/JNI and repeat code generation.
  Fixed test compilation/format issues before passing reruns. Updated plan/handoff/
  architecture/data/decisions/payload/development; no dependency/migration/schema.
- Next: coordinated live reader switch, Android override controls, real-book/device
  measurements. Android builds/device/lint/release/arm64 not rerun; no blocker.

## 2026-10-28 — Compact boundary prompt contracts

- Added compact prompt/action schemas and a pure guarded handler over the retained
  override lifecycle. Full chapter/rule revision/retry payloads remain native.
  Fresh prompt identities reject actions after same-page/span dismiss/reopen;
  stored A/B/C, literal manual parts, both scopes/default All, winning reversal,
  exact retries/Cancel and typed stale/reload state are ready for shell adoption.
- PASS: six new pure cases; 611 Rust tests (17 ignored), strict Clippy/format,
  extended actual SQLite/FIFO rollback/reopen/lost-success/reversal case, five state
  and 15 action Kotlin fixtures, host JNI imports/cancellation/custom/100 handles,
  byte-identical repeat generation and diff check. Fixed initial Facet repr errors.
- Updated plan/handoff/architecture/data/decisions/payload/development. No dependency,
  migration or live Android change. Coordinated live reader switch stays unchecked;
  then wire compact override controls and measure real-book/device costs. Separate
  Full matrix/APK/device/lint/release/arm64 not rerun; no blocker for this slice.


## 2026-10-28 — Retained effective rendering adapters

- Added borrowed retained-page furigana using shared strict original/effective
  text/range/source validation. Extracted Android UTF-8/UTF-16 geometry and added
  compact effective tokens with full crossing surface/revision/segment selections;
  unresolved manual metadata stays absent. No chapter clone/re-resolution or I/O.
- PASS: three new pure cases; 614 Rust tests (17 ignored), strict Clippy/format,
  production-helper host Kotlin checks plus Full JNI import/custom/codec/lifecycle
  regressions, repeat codegen, debug app/test APKs and Android lint. All 10 focused
  phone emulator span/formatting/furigana/ruby-layout tests pass. Fixed initial
  fixture borrow error; Gradle required usual sandbox socket escalation.
- Updated plan/handoff/architecture/decisions/payload/development. No dependency,
  migration or wire-schema change. Live reader still uses B; next switch app/FFI/
  Android commands and render adapters together, then add compact override controls
  and measure real-book/device costs. Full matrix/release/arm64 not rerun; no blocker.

## 2026-10-28 — Native effective reader callback adapter

- Added a common Rust-only Crux request/callback adapter for all retained effective
  operations. Captured attempts route FIFO errors/native results to the matching
  lifecycle; bad envelopes fail closed, old callbacks cannot release newer work,
  and write reservations keep exact retry ownership. No automatic follow-ups.
- Fixed selected-word encounter counts remaining visible after status observed
  deleted vocabulary; status and erased count now agree, preserving candidates.
- PASS: eight new regressions, 622 Rust tests (17 ignored), strict Clippy/Rustfmt,
  actual worker rollback/reopen/final-time/custom/completion/override coverage,
  host Kotlin/JNI Full imports/cancellation/custom/Unicode/100 handle lifecycles,
  byte-identical repeat codegen and diff check. No failed checks.
- Updated plan/handoff/architecture/decisions/payload/development. No dependency,
  migration, wire-schema or Android source change. Live switch remains unchecked;
  next adopt the effective session and captured native events in app/FFI/Android
  together, then expose override controls. APK/device/lint/release/performance/
  process-death/Full-loader matrix/physical arm64 not rerun. No blocker.

## 2026-10-28 — Library title search

- Added core-owned title filtering and Android Search titles/Clear controls.
  Literal Unicode queries survive navigation/refresh/import/recreation; full
  summaries remain retained. No content search, storage effect or dictionary load.
- PASS: four new core cases; 626 Rust tests (17 ignored), strict Clippy/format,
  host Kotlin/JNI including unchanged DB bytes, repeat codegen, both-ABI debug
  app/test APKs and lint. Six phone regressions plus wide/dark search pass.
- Initial phone run exposed off-screen lazy-row targeting and transient cover
  pixel sampling in existing tests; stable-key scrolling/animation settling fix
  both. Gradle required usual sandbox socket escalation. Direct instrumentation
  found no test package after Gradle cleanup; the Gradle wide run passed.
- Updated README/plan/handoff/architecture/decisions/development. No dependency or
  migration; Event/ViewModel codecs now include search state. Effective-reader
  switch remains the next reader task; sorting/status bars remain unchecked.
  Release/performance/Full-loader matrix/physical arm64 not rerun; no blocker.


## 2026-10-28 — Library sorting and durable recency

Implemented Last read (default), Title and Import date controls over the existing
core-owned title search. Migration 0019 atomically records explicit-reader-entry
order; legacy/unopened values stay unknown and duplicate imports preserve recency.
Four pure/seven SQLite cases, 637 workspace Rust tests (17 ignored), strict lint/
format/codegen, real-Full JNI, both-ABI APKs/Android lint, seven phone regressions
and two wide/dark cases pass. Fixed old fixtures/generated enum/preview assumptions before passing reruns.
Architecture/data/decision docs updated. Effective-reader live switch stays next;
Library status bars remain unchecked. No blocker; physical
arm64, release and performance remain deferred. Commands: DEVELOPMENT.md end.

## 2026-10-28 — Library distinct-surface status bars

Added read-only whole-document five-status distributions and accessible Android
stacked bars. Exact registered Japanese spellings count once across repetitions/
chapters; first B matches the live reader, global statuses stay live, and deleted
entries remain absent. Core retains counts/explicit-retry errors; return/import/
status/completion refresh them. Defer scans during active file batches.

Four core/four SQLite cases, 645 Rust tests (17 ignored), strict Clippy/format,
real-Full host JNI, repeat codegen, both-ABI debug APKs/lint, eight focused phone
tests and one wide/dark case pass. Initial fixture field/EPUB-wrapper compile assumptions were corrected;
device polling now flushes Compose's lazy effect before awaiting native counts.
Batch cancellation/picker regressions verify the new scan guard. Documentation
and plan updated. Effective-reader switch remains next, including Library
membership; release/performance/physical arm64 remain deferred. Commands and
final device/environment details: DEVELOPMENT.md end.

## 2026-10-28 — Effective Library membership storage

Added a separate atomic read-only effective Library distribution API for the
coordinated reader switch. It shares reading/completion eligibility, resolves
pinned B/same-profile metadata/winning rules and counts each registered exact
surface once across chapters. Missing/deleted vocabulary stays absent; Missing B
and zero counts differ. Canonical-text reads skip source/image BLOBs and Full.
Live Library and reader continue matching original B until their joint adoption.

Seven new SQLite cases, 652 workspace Rust tests (17 ignored), strict Clippy/fmt/
diff and extended real Full A/B/C fixture with both loaders pass. Tests cover
split/merge/precedence/reversal, exact classification/Unicode, live statuses,
deleted-origin globals, corrupt data, query-only bytes and unchanged history.
Initial import/iterator-lifetime compilation errors were fixed before passing
reruns. Plan, architecture/data/decisions and handoff updated; no blocker.
Next switch live core/FFI/Android and the Library FIFO membership call together,
then wire compact override controls. Android/JNI/release/hardware/performance
not rerun; exact commands/results: DEVELOPMENT.md end.

## 2026-10-28 — Confirmed single-document deletion

Implemented the existing Library plan's single-delete slice end to end. Core retains
an exact visible confirmation target across recreation, gates duplicate submissions,
and provides Cancel/explicit Retry with separate errors. Typed FIFO/JNI invokes one
atomic SQLite transaction; all live content/assets/analysis/dedup/position/partial
time are removed while historical source/chapter identities, completion provenance/
time/occurrences, vocabulary/status/custom entries and reversible global rules stay.
Reimport starts with fresh IDs, zero progress and no finish/recency/partial time.
Android disables actions during file batches and unresolved saves; query/sort survive.

Eight new Rust cases; 660 workspace tests (17 ignored); all 294 storage cases after
converting original/effective completion replay and word-count simulations to real
deletion; strict Clippy/fmt, real-Full host JNI, byte-identical codegen, both-ABI APKs
and Android lint pass. Nine existing phone regressions pass; corrected deletion UI
case passes phone and wide/dark, including Cancel/recreation/forced rollback/Retry/
history/unrelated-row/fresh-reimport checks. Initial fixtures needed type/module/
position corrections and a Kotlin warning fix. The device test initially assumed
an empty title filter after deleting the only document; adding a survivor verified
the intended case. The emulator exited after the first suite; restart restored the
corrected runs. Display size/density/light mode restored. No remaining blocker.

Progress/architecture/data/decisions/handoff updated. Rename and bulk/Select All
remain unchecked; coordinated effective-reader/Library adoption remains first.
No migration/dependency. Release, separate Full-loader matrix, performance,
process-death and physical arm64 not rerun. Commands/results: DEVELOPMENT.md end.


## 2026-10-28 — Library rename

Implemented the next independent Library slice while coordinated effective-reader
adoption stays pending. Pure core retains literal ID/title/error drafts across
Activity recreation; Save/explicit Retry/Cancel use the existing typed FIFO/JNI.
Only the live Library title changes atomically; history/source titles/content/assets,
progress/finish/recency/time, vocabulary/counts and dedup identities survive.
Same-title retries write nothing; absent IDs fail without recreating documents.
Search/sort reproject after success and duplicates keep the renamed title.

Eight new Rust cases; 668 workspace tests (17 ignored), strict Clippy/fmt/Kotlin,
real-Full host JNI, repeat codegen, both-ABI APKs/lint and eleven focused phone tests
pass; the rename case also passes wide/dark, with display settings restored.
Initial fixture/constructor errors
were corrected before passing reruns; Gradle/ADB required local-socket escalation.
No dependency/migration. Progress/data/architecture/decisions/handoff updated;
effective-reader/Library adoption, override controls and bulk deletion remain next.
Release/process-death/physical arm64/performance were not rerun. Exact commands
and final device results: DEVELOPMENT.md end.

## 2026-10-28 — Confirmed Library bulk deletion

Completed the remaining Library deletion slice with long-press selection, row tap
toggles, Select All over every matching title (including offscreen rows), retained
hidden selections and a frozen scrollable confirmation list. Core owns selection/
confirmation/error state across Activity recreation, exact Retry, Cancel/Back and
shared busy/save gates. Existing FIFO/JNI executes one immediate transaction across
the entire selection; a later-document or deferred-COMMIT failure restores earlier
deletions. Single deletion shares that cleanup. History/vocabulary/provenance/rules
survive; empty/absent/repeated IDs are harmless and reimports get fresh progress.

Eight new Rust cases; 676 workspace tests (17 ignored), strict Clippy/fmt/Kotlin,
real-Full JNI, identical repeat codegen, both-ABI APKs and Android lint pass.
Eleven existing phone regressions pass in the initial suite; the new bulk case
passes after correcting its scroll/virtualization assumption, then passes wide/dark.
Twelve distinct phone cases total; original emulator display/theme restored.
Initial reread-time/safe-call/back-helper/plural-resource fixture/build errors were
corrected. Gradle/ADB/emulator needed local-socket/AVD escalation. No blocker.

Updated progress/handoff/decisions/data/architecture/README/development docs. No
dependency/migration. Next: coordinated effective-reader/Library adoption, Android
override controls and real-book/device measurements. Physical arm64 remains deferred;
release/process-death/separate Full-loader matrix/performance not rerun. Exact commands,
results and initial/corrected device-run distinction: DEVELOPMENT.md end.

## 2026-10-28 — Effective resolution performance and real-novel host baseline

Completed the planned host performance slice before the coordinated live switch.
Added an opt-in TXT/EPUB benchmark that imports into a disposable file-backed DB,
measures B-only/stored-A/B/C/global-split resolution/Library/page costs and separate
whole-chapter A/C setup, verifies unchanged DB bytes and preserves source files.
Aozora's real novel produces one 329,187-character chapter with 211,016 B tokens.
Removed the extra chapter-sized metadata tree: reuse exact B indices, binary-search
validated A then C only for changed spans, preserving every provenance field.
Pure B-only median improves 65→41 ms; full repository reads still take hundreds
of milliseconds. Detailed before/after figures and corpus hashes are in
EFFECTIVE_READER_PERFORMANCE.md; Android/JNI/real EPUB costs remain unmeasured.

677 Rust tests (17 ignored), strict Clippy/fmt/diff, pinned Sudachi verification,
real Full owned/mapped segmentation and unchanged synthetic payload sizes pass.
New exhaustive regression checks 4,096 Unicode B/A/C/manual partitions against a
full-scan metadata oracle. Multi-input/two-chapter EPUB/duplicate/even-iteration
benchmark smoke and invalid CLI checks pass, with scratch directories cleaned.
Initial download needed DNS/network escalation; exploratory overlapping-build
timings were discarded and distinct baseline/production executables verified.
No dependency/migration/wire change or blocker. Handoff/plan/architecture/decisions/
development updated; live effective reader/Library switch and override UI remain
next. APK/Android lint/JNI/device checks were not rerun for this host-only slice.

## 2026-10-28 — Real EPUB / repeated-book Library scaling

Continued the pending host measurement slice with the unchanged IDPF Kusamakura
EPUB (16 chapters, 96,296 canonical characters) and 100 distinct-byte copies
(1,600 chapters/6.23 million B tokens). The opt-in `--epub-copies` harness changes
only archive comments, imports through production dedup/registration, checks exact
document counts and streams database hashes. Inputs stay out of Git; timed reads
preserve database bytes and normal exit removes only the scratch database.

Effective Library storage now shares validated exact-surface status/absence across
books within one read transaction, retaining per-document distinct counts/full
chapter validation. Nothing persists between scans or after failure. The new
24-book case verifies all statuses, Unicode identities, external changes, deletion/
fresh-ID re-registration and corrupt later analysis/repair without read writes.
Sequential release before/after medians improve 6.41→4.37 seconds for B-only and
12.70→11.55 seconds for stored A/B/C on the repeated-book fixture. Single-book scans
do not improve; peak memory/device/diverse-corpus costs remain unmeasured. Corpus
hashes, full methods/results/limitations: EFFECTIVE_READER_PERFORMANCE.md.

678 workspace tests (17 ignored), the additional archive-copy example test, eight
focused storage cases, selected real Full owned/mapped fixture, strict Clippy/fmt/
diff and ten invalid CLI cases pass. Network escalation was needed for the corpus
download after sandbox DNS failure. No dependency/migration/wire change or blocker.
Updated plan/handoff/architecture/data/decisions/development. Next: coordinated live
reader/Library switch, Android override controls, then JNI/device/diverse-book costs.
APK/Android lint/JNI/codegen/device checks were not rerun for this host-only slice.

## 2026-10-28 — Independent Android boundary editor

Implemented ReaderBoundaryEditor against the existing compact state/action
contract: A/B/C choices/explicit generation, All/This scope, scalar cut toggles/
join without editing text, winning-rule scope/reversal and native Retry/Cancel.
Stale/invalid contexts require explicit reload. ReaderBoundaryText validates exact
Unicode partitions and matches Rust whitespace semantics. Session/prompt/text
guards keep saveable drafts scoped; accepted retry payloads remain native.
The component reports dialog interaction/scroll/keys to the reader clock hook.
No native/schema/dependency/wire change or live reader/Library adoption.

Eight isolated Compose cases pass on API-36 phone and tablet; long-span scrolling
uses 1.6 font scale and both themes. Host Kotlin partition checks and real-Full
JNI regressions, 53 retained-session Rust tests, APKs, strict lint/formatting/diff
pass. Fixed the initial unnecessary-!! compile warning, Dialog-registry restoration
loss and nullable-inner-saver rejection during session replacement; all corrected
before the final device runs. Gradle/ADB/emulator needed sandbox escalation;
original emulator display/theme restored. Exact commands/failures: DEVELOPMENT end.

Updated plan/handoff/architecture/data/decisions/payload/development. No blocker.
Next coordinate live core/FFI/Android reader/completion/Library membership, then
expose the editor with validated original-B span selection and retained events.
Full workspace/Clippy/separate Full-loader matrix, process death, release,
physical arm64 and performance measurements were not rerun for this UI slice.


## 2026-10-29 — Staged effective application pages and completion

Added a native integration constructor to the actual Yomoku app model. Its token
loads use retained effective keys/compact views and source-preferred furigana;
revision-anchored completion freezes the rendered partition before draining both
final-time slots, then prepares/commits through captured FIFO requests. Explicit
exact retries survive checkpoint/preparation/commit failures and rule reversal.
Dismissed reads still release their own FIFO/time obligations; stale callbacks
cannot release newer work. Shared original/effective position/finish/notices keep
AlreadyRecorded silent. Legacy word/status/custom/completion events reject in the
opt-in model. Production activation remains pending all remaining consumers.

Six pure app cases and one real app/SQLite/FIFO fixture pass; 685 workspace tests
(17 opt-in ignored), strict Clippy/fmt, real-Full host Kotlin/JNI/new codec fixtures,
18-file identical Kotlin regeneration, both-ABI APKs and Android lint pass.
All 11 API-36 x86_64 phone bridge/navigation/clock regressions pass; the emulator
started for this run was shut down afterward.
Resolved initial Facet/visibility compile errors, Clippy Event-size finding via
boxed native request, punctuation fixture expectation and deferred-read invalidation.
Gradle/ADB/emulator needed usual local-socket/AVD escalation. No schema/dependency
change or blocker. Updated plan/handoff/architecture/data/decisions/payload/development.
Next integrate app word/status/custom/override + effective Library, then activate
core/FFI/Android together and expose the prepared boundary editor. Physical arm64,
separate Full opt-in loader matrix, release/process death and effective device
performance were not rerun. Exact commands/logs/failures: DEVELOPMENT end.

## 2026-10-29 — Staged effective application word/status/custom consumers

Integrated compact selected-word, five-status and custom CRUD events/views into
the native effective app model through retained requests and the existing FIFO.
Reads preserve unresolved metadata and live deletion/counts; custom writes preserve
literal fields and built-in gates. Failed status/custom writes retain exact Retry/
Cancel obligations and pending custom errors after dismissal. Clock saves coalesce
and drain on failures; deferred movement waits for time and write resolution.
Stale/late callbacks cannot consume newer work. Production remains original B.

Seven new pure app and two real app/SQLite/FIFO cases pass; 694 workspace tests
(17 opt-in ignored), strict Clippy/fmt, real-Full host Kotlin/JNI, eight new event/
populated-view codec exchanges, 18-file identical regeneration, both-ABI APKs and
Android lint pass. Initial fixture compile issues were corrected. The first full
run hit an existing import lifecycle teardown assertion; isolated/full reruns pass.
Gradle needed the usual local-socket escalation. No dependency/schema change or
blocker. Updated plan/handoff/architecture/data/decisions/payload/development.
Next integrate app override consumers + effective Library, then activate core/FFI/
Android together. Device, separate Full opt-in matrix, release/process death,
physical arm64 and performance were not rerun. Exact commands/logs: DEVELOPMENT end.

## 2026-10-29 — Staged effective application override consumers

Connected compact override actions/state to the native effective app model and
existing FIFO. Current-page/prompt guards, stored choices/manual save, explicit
A/C generation and reversal use retained requests. Accepted writes retire the
page, keep exact Retry/Cancel obligations and defer movement; clock checkpoints
coalesce during I/O and both time slots drain before departure. Read dismissal
keeps FIFO ownership. Success/stale results require explicit reload and invalidate
Library bars. Production remains B pending effective Library/Android integration.

Four new pure app and one real app/SQLite/FIFO cases pass; all 699 workspace tests
(17 opt-in ignored), strict Clippy/fmt, real-Full host Kotlin/JNI, 15 new app event/
five populated-view codec exchanges, 18-file identical regeneration, both-ABI APKs
and Android lint pass. Initial fixture/compile assertions corrected; full rerun
passes. Gradle needed the usual local-socket escalation. No schema/dependency
change or blocker. Updated plan/handoff/architecture/data/decisions/payload/development.
Next add the effective Library operation, then activate core/FFI/Android together.
Device, separate Full opt-in loader matrix, release/process death, physical arm64
and performance were not rerun. Exact commands/logs: DEVELOPMENT end.


## 2026-10-29 — Staged effective application Library membership

Connected the existing Library event to the effective distribution repository via
one native-only operation on the existing FIFO. Reused compact results, retained
cache and separate explicit-Retry errors. Preserved visibility/busy/time/mutation
gates, literal rename drafts, selection and frozen deletion prompts. Production
Model::default still uses original B pending coordinated Android activation.

Five core Library cases now run against both models (one new), and two new real
app/SQLite/FIFO cases cover split membership, live status/deletion, reversal/reopen,
Missing/zero, corrupt-data failure/recovery and unchanged scan bytes. All 702 Rust
tests (17 opt-in ignored), strict Clippy/fmt, real-Full host Kotlin/JNI, Kotlin
formatting, unchanged generated Kotlin, both-ABI APKs and Android lint pass. Native operation has no shell codec.
Gradle required the usual local-socket sandbox escalation. No schema/dependency
change or blocker. Updated plan/handoff/architecture/data/decisions/payload/development.
Next activate core/FFI/Android together and expose original-B span selection/editor.
Device, separate Full opt-in matrix, release/process death, physical arm64 and
effective device/diverse-book performance were not rerun. Commands/logs: DEVELOPMENT end.

## 2026-10-29 — Staged Android effective word consumers

Added isolated compact effective word rendering, dictionary lookup, five statuses
and literal custom CRUD with native Retry/Cancel reachable after dismissal/page
retirement. Shared annotation/dictionary/phone-wide presentation with original B;
kept effective revisions/segments separate. Native-session replacement and stale
links retire local selections/drafts. Strict ranges, unresolved/crossing surfaces,
Rust whitespace, pending-write/time gates and explicit reload/later navigation
are covered. Production remains B pending coordinated activation.

PASS: 70 focused Rust, real-Full host JNI/codecs, Kotlin formatting/diff, both-ABI
APKs and Android lint. All 16 production word/span regressions and 11 new compact
phone cases pass (initial 23 + final 11); all 11 compact cases pass wide/dark.
Emulator restored/stopped. Initial Long fixture compile error corrected; usual
Gradle socket escalation passes. No blocker/schema/dependency change. Updated
plan/handoff/architecture/decisions/payload/development. Next connect the component
with a retained session key, Android navigation/completion and original-B span/
editor, then activate core/FFI/Android together. Broader Rust/Clippy, release/
process death, physical arm64 and effective performance not rerun/measured.

## 2026-10-29 — Staged Android effective navigation and retained session ownership

Connected compact word consumers to the actual app shell with a UUID retained by
CoreSession across Activity recreation. Effective Next/Finish/swipes carry the
validated compact page identity and exact position; Previous stays position-only.
Missing/retired/malformed pages never complete with B. Shared time/storage and
effective mutation gates block movement; gestures use current revisions and
replacement sessions retire selections. Android samples final time before the
effective completion intent. Production stays B until original-B span selection/
editor and coordinated activation are ready.

PASS: 70 focused Rust cases, real-Full host Kotlin/JNI/codecs, Kotlin formatting,
both-ABI APKs/lint and 36 distinct phone cases. The final 15-case phone run passes
all six clock cases (one new), four compact shell cases, native-key independence/
recreation and notice/celebration checks. All four shell cases also pass wide/dark.
Display settings restored; task-started emulator stopped.
Initial null-assertion warnings and a mistaken assumption that Compose button
semantics retain frozen callbacks were corrected. No schema/dependency change.

Long-page device checks remain blocked: ReaderWords movement times out before
navigation, reproducing with the previous ReaderScreen implementation; the next
ReaderNavigation case stalls waiting for Activity idle. JDB observes main blocked
in HardwareRenderer.syncAndDrawFrame. Fresh processes/Vulkan-disabled swangle
repeat; swiftshader crashes at boot. Baseline source was restored to the new
implementation and final APK/lint/phone checks pass. No speculative product fix.
Next add original-B span selection/editor and activate together; investigate the
long-page renderer and finish device/performance coverage. Commands/logs and
precise unverified cases: DEVELOPMENT end. Full Rust/Clippy, release/process death,
physical arm64 and effective performance were not rerun/measured.

## 2026-10-29 — Staged original-B span selection and boundary editor integration

Added page-only original-B edit spans to the atomic effective read/compact bridge,
without another analysis read or retained whole original chapter. Strict Kotlin
projection preserves full crossing Unicode anchors; endpoint selection rejects
Japanese-span gaps, Missing/malformed data and stale session/page ownership. The
staged shell now opens ReaderBoundaryEditor and routes typed modes/scope/manual/
reversal, native Retry/Cancel and explicit reload, with shared FIFO/time/mutation
gates and retained write errors. Production remains B; activation is next.

PASS: 704 Rust tests (17 ignored), expanded read-only SQLite/actual app FIFO checks,
strict Clippy/fmt/diff, complete real-Full host JNI/codecs, 17 byte-identical
generated Kotlin files, Kotlin formatting, both-ABI APKs and lint. All 28 phone
cases, five final guard phone reruns and 13 wide/dark cases pass without skips or
failures. Display restored and task-started emulator stopped. Updated architecture,
data/payload notes, decisions, plan and handoff. Synthetic page JSON includes B
edit spans (50,135 B at 500 characters, equal at 10k/100k chapter sizes); 60/68-byte
identity/selection and durable schema stay unchanged.

Corrected initial fixture/API/nullability errors. Concurrent host/Android BoltFFI
metadata generation caused one JNI symbol-load failure; sequential full host
retry passes. Gradle/AVD required sandbox escalation. Existing long-page HWUI
stall remains unresolved; physical arm64 and actual effective device/diverse/
image-heavy costs remain unverified. Commands/results/logs: DEVELOPMENT end.

## 2026-10-29 — Serialized effective-reader activation candidate

Added Rust-only runtime/FFI effective factories using the existing native
dictionary loaders, SQLite FIFO and analyzer lifecycle. Five new Bincode/SQLite
regressions exercise shell events, actual Render callbacks and compact views:
manual/global/local edits, A/C generation and stored A/B/C reuse, reversal/reopen,
effective Library membership, live statuses, literal Unicode custom Retry after
dismissal/reopen, stale selections, Missing B and legacy command rejection.
Completion retry preserves five occurrences and 321 ms after rule reversal;
furigana does not invent manual readings. Full retained keys stay native.

PASS: all 709 Rust tests (17 ignored), strict Clippy/Rustfmt/diff. Initial
Crux API/status iterator and Settings-gate fixture errors were corrected.
The complete existing real-Full host Kotlin/JNI suite and bidirectional Unicode
codecs pass (production B); existing teardown diagnostics remain nonfatal.
Updated architecture, decisions, plan and handoff. No dependency, wire/durable
schema or BoltFFI export change. NativeBridge/Android remain B; activation and
compact production JNI/device fixture migration remain unchecked. APK/device,
separate Full-loader matrix, physical arm64 and performance were not rerun.
Existing long-page HWUI stall remains unresolved. Details: DEVELOPMENT end.

## 2026-10-29 — Production effective reader and boundary controls

Activated NativeBridge's effective factory using the existing dictionary
providers/FIFO/cache/lifecycle. Migrated production JNI/device fixtures to compact
page revisions and segment selections. Real Full A/B/C generation/stored reuse,
default All/local/manual edits, reversal/reopen, effective Library, words/status/
custom/furigana and frozen completion now execute through the production path.
Success exposes explicit Reload. Fixed two issues found on device: a retired-page
reload marker surviving same-book reopening, and a token-read guard suppressing
normal same-anchor position/preset refreshes. Exact accepted retries stay native.

PASS: 710 Rust/17 opt-in ignored, strict Clippy/Rustfmt, full final real-Full host
JNI/codecs/100 handles, unchanged 17 generated Kotlin files, both-ABI APKs/Android
lint and formatting/diff. PASS: 57 distinct phone cases and three wide/dark cases;
final 19-case phone rerun passes all four initially failed regressions plus full
compact-word/production coverage. Corrected fixture layout/scroll/revision
assumptions; emulator System UI/install-space recovery is recorded in DEVELOPMENT.
Display/light restored, task-started emulator stopped. No dependency, wire/durable
schema, historical rewrite or destructive semantic change. Updated README,
architecture/data/decisions, checked activation and refreshed the 62-line handoff.

Next: investigate four pre-existing long-page HWUI stalls, measure actual effective
JNI/device/diverse/image-heavy costs, then phase-5 history aggregation. Separate
Full-loader matrix, release/process-death, physical arm64 and performance were not
rerun/measured. Nostr empty/Q188 remain unrelated pending inputs. See DEVELOPMENT
end for exact commands, failures, reruns and `/tmp/yomoku-activation-*` logs.

## 2026-10-29 — Long-page reader rendering and regression closure

Removed per-word dynamic style rewriting with static spans and explicit empty
link styles; bounded long LTR interactive layouts at measured soft line endings.
Canonical Unicode/wrapping/glyph positions/spacing/crossing selections remain
intact; bidi/paragraph-styled text retains its complete layout. Corrected native
test synchronization with automatic Library counts and effective token loads.

PASS: final 23 phone/light and nine wide/dark cases, including all four formerly
blocked long-page cases, two new geometry guards, palettes/status/Retry,
navigation/recreation/finish, typography, EPUB formatting/images/furigana.
Both-ABI APKs with unchanged native binaries, strict Android lint, Kotlin
formatting and diff pass. No Rust/JNI/dependency/wire/schema change; prior native
suites were not rerun. Initial failures/corrections and rebuildable dictionary
space recovery are recorded in DEVELOPMENT.md; reader DB/data were preserved.
Phone/light restored, debugger forward removed, task emulator stopped.

Next: actual effective JNI/device/diverse/image-heavy measurements, phase-5
immutable event/provenance audit, then lifetime occurrence/time aggregation.
Physical arm64 remains deferred; Nostr empty/Q188 do not block local work.

## 2026-10-29 — Android effective-reader measurements and large-file import heap fix

Added opt-in scratch CoreSession/JNI/FIFO/codec measurements for the two pinned
real books and a 12-asset image page, with five-sample distributions and raw CSV.
The real 17.9 MB EPUB exposed retained boxed UByte allocation exhausting Android's
192 MiB Java heap. Production URI imports now own a primitive buffer behind the
same typed list; bytes/encoding/retries/atomic identity and native APIs stay intact.

PASS: 18 MiB byte-identity/repeated-encoding regression, eleven other distinct
phone import cases, benchmark default skip and final isolated benchmark; both-ABI
APKs with unchanged native output, strict lint/formatting/diff. Corrected fixture
Library-read races and constant SRT bytes that correctly deduplicated on reruns.
Real EPUB import now succeeds. Debug page medians: TXT 903/2057 ms and EPUB
770/826 ms for B-only/largest-chapter A/B/C; image batch ~251–254 ms. No optimized,
frame-time or peak-memory claim. Reader data preserved during dictionary-only
installation-space recovery; scratch/input copies removed and emulator stopped.

Next: phase-5 immutable event/provenance audit, lifetime aggregation, then optimized
Android profiling/diverse-Library costs. Physical arm64 and Nostr empty/Q188 remain
separate deferred inputs. Exact commands, failures and results: DEVELOPMENT.md and
EFFECTIVE_READER_PERFORMANCE.md.

## 2026-10-29 — Phase-5 immutable history/provenance audit

Finalized the existing v1 event/provenance contracts without a new migration.
Added schema-5–19 upgrade fixtures preserving dated completions/statuses, repeated
occurrences, nullable metadata, deleted source/chapter joins and ID high-water
marks. Late failures roll back unchanged and retry; actual completed and unused
v18 effective reservations survive upgrade/deletion without replay.

PASS: 43 focused history tests, all 713 workspace Rust tests (17 opt-in ignored),
strict Clippy/Rustfmt/diff. No production/native-wire/Kotlin change or new
dependency; Android/JNI/Full-loader/release/arm64 were not rerun. No blocker.
Next: read-only lifetime occurrence/time aggregation, then status timeline.

## 2026-10-29 — Read-only lifetime occurrence/time totals

Added pure LifetimeReadingTotals and one-statement SQLite aggregation from complete
analyzed event rows. Repetitions/rereads count; time sums once per command,
including zero-Japanese pages. Imports/pending time/unused reservations/legacy
events are excluded. Document deletion preserves both totals; vocabulary history
cascades recalculate occurrences without erasing page time or recounting JSON.
Unknown versions, corrupt durations and integer overflow error without writes.

PASS: seven dedicated tests plus schema-5–19 upgrade/deletion totals; all 720
workspace Rust tests (17 opt-in ignored), workspace build, strict Clippy/Rustfmt/diff.
No failures/blockers, new dependency/migration/wire change or Android/JNI/Full-loader
rerun. Statistics tab remains a placeholder. Updated README, architecture/data/
decisions, checked the aggregation item and refreshed the handoff.
Next: status timeline and durable Known dictionary-form links, then timeframe buckets.

## 2026-10-29 — Five-status vocabulary history timeline

Added pure streaming status replay and a one-statement read-only repository
projection of registration/manual/completion history. Import registrations create
jumps; actual transitions move absolute counts between all five statuses, with
one point per timestamp. Per-entry causal projection handles regressing clocks
and frozen completion dates without rewriting raw events. Version/cause/provenance
and old→new chains validate without repair. Document deletion retains even unread
manual Known history; vocabulary erasure recalculates its full history, never
rescans, and later registration starts a fresh ID/New.

PASS: six new core/eight new SQLite tests; schema-5–19 timeline projections after
upgrade/deletion/reopen; all 734 workspace Rust tests (17 opt-in ignored), workspace
build, strict Clippy/Rustfmt/diff. No failures/blockers or dependency/migration/wire
change. Android/JNI/Full-loader/release/physical-arm64 were not rerun.

Split the combined timeline plan item to check off only the verified five-status
portion. Known dictionary-form counts remain unchecked, requiring durable links
for registered-but-unread entries after document deletion. Date buckets and UI
remain open; next session starts with that link/count prerequisite. Updated
README, architecture/data/decisions/development and the brief handoff.


## 2026-10-30 — Durable vocabulary dictionary-form links

Added migration 0020 and transactional analyzer links in vocabulary/analysis,
all analyzed imports, selected registered edits and original/effective completions.
Exact spellings survive document deletion, including unread registrations;
vocabulary erasure cascades them and later evidence belongs to a fresh ID.
Unresolved/manual metadata stays absent. Upgrade recovers available snapshot and
retained encounter evidence without rewriting history; already deleted unread
metadata is unavailable. No read/retry rescan recreates erased associations.

PASS: seven new storage tests, schema-5–20 history coverage, 741 workspace Rust
tests (17 opt-in ignored), build, strict Clippy/Rustfmt/diff. Initial API/fixture/
Clippy issues corrected; interrupted workspace test rerun completed. No blocker.
Updated progress/data/architecture/decisions/development and the brief handoff.
No dependency/FFI/Kotlin change; Android/JNI/Full-loader/release/arm64 not rerun.
Next: Known dictionary-form timeline counts from durable links, then timeframe bins
and typed UI transport. The count task remains unchecked.

## 2026-10-30 — Known dictionary-form history timeline

Extended pure timeline points and the read-only SQLite projection with distinct
Known dictionary-form counts from durable vocabulary links. Shared forms use
chronological reference counts, including simultaneous handoffs and regressing
wall dates. Exact spelling identifies forms; unresolved metadata adds none.
Retained undated context evidence applies throughout its vocabulary generation.
Document deletion preserves the projection; vocabulary erasure recalculates it
without rescans and fresh raw registration supplies no invented links.

PASS: five new core/six SQLite tests; schema-5–20 Known-form upgrade coverage;
752 workspace Rust tests (17 opt-in ignored), build, strict Clippy/Rustfmt/diff.
No failures or blockers, dependency/migration/wire/Kotlin change. Android/JNI,
Full-loader/release/process-death/physical-arm64 checks were not rerun.
Checked only the completed count task and updated README, architecture/data/
decisions/development and handoff. Next: timeframe buckets with lifetime cumulative
origin, then typed statistics transport and charts.


## 2026-10-30 — Statistics timeframe buckets

Implemented pure UTC daily/weekly/monthly buckets and one-snapshot read-only
SQLite aggregation for all four timeframes, defaulting to last year. Explicit
opening lifetime values preserve cumulative origins; empty intervals carry all
absolute status/Known-form lines. Calendar-month clamping, Monday weeks and
partial edge intervals have documented policy. Actual analyzed rereads count,
zero-word time survives, and deletion/duplicate/pending/legacy semantics remain.
No dependencies, migrations, wire or Kotlin changes; runtime maps the new error.

PASS: eight new core/seven SQLite cases; 767 workspace Rust tests (17 opt-in
ignored), build, strict Clippy/Rustfmt/diff. Initial exhaustive-error mapping and
unresolved-form fixture expectations corrected; no unresolved failures/blockers.
Updated plan, handoff, README and architecture/data/decisions/development docs.
Android/JNI/codegen/Full-loader/release/process-death/physical-arm64 not rerun.
Next: typed statistics transport with retained loading/error/Retry, then charts.


## 2026-10-30 — Retained statistics transport

Added native FIFO statistics requests and typed core/FFI timeframe/as-of/state
payloads, default year, exact Retry and request-ID stale callback protection.
Rapid selections coalesce, Settings retains state/resumes unattempted queries,
and imports/status/completions/effective edits plus tab reentry invalidate values.
Only compact bucket values and explicit shell intents cross FFI; no schema,
dependency, history semantics or Compose chart change.

PASS: 780 Rust tests (17 opt-in ignored), nine new core/three worker/one protocol
cases, real-Full full Kotlin/JNI suite and Rust/Kotlin/Rust exact-u64 fixtures,
repeat generation, build, strict Clippy/Rustfmt/Kotlin-format/diff. Initial corrupt
fixture/error expectations and Kotlin safe-call warning corrected; Settings edge
case fixed/tested. No unresolved blockers. Updated plan, README, architecture/data/
decisions/development and handoff. Android/device/release/performance/Full-loader
matrix not rerun. Next: Compose overview/drilldowns using retained statistics.


## 2026-10-30 — Statistics Compose overview and drilldowns

Rendered the three specified charts, all four timeframe controls, explicit Retry
and exact opening/interval drilldown rows from retained native values. Preserved
lifetime origins, unsigned counts, status colors, UTC ranges and actual x spacing;
added accessible legends/summaries and Settings/Back/recreation retention. No
new dependency, schema or Rust source change. Updated preview/reader fixtures for
the previous Statistics wire field and refreshed plan/handoff/architecture/data/
decisions/development/README docs.

PASS: both-ABI native regeneration, debug/test APKs, strict lint, changed Kotlin
format/diff; seven new plus six existing cases on phone/light and tablet/dark,
26 executions without failures/skips. Visual checks pass. Existing emulator's
insufficient installation space resolved with an isolated workspace image while
preserving Library data; original temporary reserve setting restored. Sandbox
Gradle and /tmp image allocation failures recovered. Display settings restored,
task emulators stopped; no remaining blocker. Rust/host-JNI/release/performance/
physical-arm64 checks not rerun. Next: Vocabulary indexed query/paging transport.


## 2026-10-30 — Read-only indexed Vocabulary browser repository

Added pure query/sort/row/page values and one-snapshot native prefix/status/count/
100-row paging repository with four sorts. Reused existing indexes and exact
completed-reading history without content/analyzer access, writes or migrations.
Recorded prefix, ordering and page-invalidation choices; updated plan, handoff,
architecture/data/development docs. Vocabulary UI/transport remain unchecked.

PASS: nine new SQLite cases; 789 workspace Rust tests, 17 opt-in ignored;
workspace build, strict Clippy/Rustfmt/diff. Verified query plans, all status
subsets, Unicode/literal prefixes, stable filtered paging, original/effective
rereads, deletion/reopen/read-only behavior, failure isolation and cleanup.
Corrected an initial missing test import; no remaining blocker. Android/JNI/
codegen/Full-loader matrix/release/performance/physical arm64 not rerun.
Next: retained Vocabulary worker/core/FFI paging, then Compose integration.


## 2026-10-30 — Retained native Vocabulary browser transport

Added retained literal query/status-set/sort, optional matching count, loaded rows,
next offset and loading/error state through native FIFO/core/generated FFI.
Implemented lazy 100-row append, frozen failed-page Retry, coalesced query changes,
stale attempt rejection including A -> B -> A, Settings deferral, reentry/mutation
invalidation and count-change restart. Preserved read-only storage and no analyzer/
dictionary loads. Updated Android view fixtures, architecture/data/decision docs,
plan and handoff; Compose browser and bulk actions remain unchecked.

PASS: eight core/three worker/one native-protocol cases; 801 workspace Rust tests
(17 opt-in ignored), strict Clippy, Rustfmt/diff, bridge workspace build, real-Full
Kotlin/JNI regressions and 100 handle lifecycles, exact unsigned/literal codecs,
identical repeated generation, both-ABI native regeneration, debug/test APKs,
strict Android lint and changed Kotlin formatting. Corrected initial fixture
schema name/unused mut/missing view initializer; sandbox Gradle socket failure
recovered with escalated offline run. No unresolved failure or blocker. Device,
separate Full-loader matrix, release/process-death/performance and physical arm64
not rerun. Next: Compose prefix/filter/sort/count/Retry/infinite scroll and
phone/tablet recreation, followed by global matching bulk operations.

## 2026-10-30 — Vocabulary Compose browser and infinite scroll

Connected retained native Vocabulary state to literal prefix input, five status
toggles, four sorts, exact matching count and lazy 100-row infinite scroll.
Added visible/idle loading, unrelated-work input gating and explicit initial/
append Retry without automatic error retries. Shared existing status labels;
hoisted session/query-keyed saveable scroll through Settings and recreation.
Updated progress, architecture, decisions and development/handoff docs. Bulk
selection/status/deletion remains unchecked; no schema/dependency/native change.

PASS: debug/test APKs, strict Android lint, changed Kotlin formatting and diff;
five new UI/native plus six navigation cases on phone/light (114.611 s) and
tablet/dark (82.566 s), 22 executions without failures/skips. Verified 205 rows/
three pages, unloaded search, filters/sorts, unsigned labels, errors/repair/Retry,
busy gates, actual Full import/manual-status/completion refresh, Settings/Back and
session/query/pages/scroll recreation. Visual checks pass; display restored and
isolated emulator stopped. Corrected initial test assumptions about offscreen
Retry and stale selection after reopening. Gradle/ADB socket sandbox failures
recovered with escalation; early install retried after boot. No open blocker.
Rust/JNI/codegen/Full-loader matrix/release/process-death/performance/physical arm64
not rerun; existing both-ABI native outputs were up-to-date. Next: global matching
bulk repositories and confirmed retained bulk UI.

## 2026-10-30 — Commit message guidance

Updated AGENTS.md to require commit bodies explaining changes and purpose, with
source branch and available issue/PR references for merges. Verified the wording
and `git diff --check`; documentation only, no application tests run. Next tasks
remain unchanged; no blockers.

## 2026-10-30 — Frozen Vocabulary bulk repositories

Added all-matching ID capture sharing browser prefix/status SQL, atomic frozen-ID
bulk status changes using existing immutable manual events, and permanent bulk
vocabulary deletion using existing history cascades. Retained page time/provenance/
live content and prevented stale IDs from targeting replacement generations.
Updated plan, architecture/data model, decisions, development commands and handoff.
Retained Select All/confirmation/FIFO/FFI/Compose wiring remains unchecked.

PASS: seven new SQLite cases; 808 workspace tests (17 opt-in ignored), strict
Clippy, workspace build, Rustfmt/diff. Verified 1,205 matches, all filters/sorts/
literal prefixes, read-only capture, immutable events/all statuses, concurrent and
frozen-ID retries, later-row/cascade/commit rollback, original/effective chart
recalculation, no live-book recreation and later fresh New registrations. Corrected
initial test API assumptions and one test-only Clippy warning. No open blocker.
No Android/JNI/codegen/Full-loader matrix/release/process-death/performance/physical
arm64 checks rerun. Next: retained selection and confirmed native bulk operations.


## 2026-10-30 — Retained native Vocabulary bulk transport

Implemented retained selection/global matching Select All and frozen confirmed
status/deletion through core/FIFO/generated FFI. Exact explicit retries preserve
IDs/status/date; prompts/capture failures gate unrelated work. Successful mutations
invalidate browser/statistics/Library/reader state. Updated Kotlin preview/test
fixtures and architecture/data/decisions/plan/handoff; Compose controls/Back and
phone/tablet recreation remain unchecked.

PASS: four new core/three worker/one protocol cases; 816 Rust tests (17 opt-in
ignored), strict Clippy/Rustfmt/diff, workspace build, complete real-Full host
Kotlin/JNI and exact large-ID/date codecs, repeatable generation/Kotlin formatting,
both-ABI debug/test APKs and Android lint. Verified 1,205 unloaded matches, hidden
selection, frozen Retry/Cancel/stale callbacks, later-row rollback/new generations,
immutable events/reopen and refreshed original/effective reader/Library/charts
without rescans. Corrected initial test field names; sandbox Gradle wildcard-IP
failure passed with approved socket access. No unresolved blocker or schema/library
change. Device/Full-loader matrix/release/process-death/performance/physical arm64
not rerun. Next: Compose selection/confirmation/Retry/Cancel/Back and device tests.

## 2026-10-30 — Compose Vocabulary selection and confirmed bulk actions

Connected tap/long-press row selection, global matching Select All, five-status
confirmation and permanent deletion to retained native commands. Kept exact
IDs/status/date on confirmation/Retry, exposed separate capture/write errors and
Retry/Cancel, gated unrelated controls/paging/navigation and consumed pending Back.
Selection actions stay above scrolling rows; ordinary Settings and Activity
recreation retain native selection/prompts/errors. Updated README, architecture/
data/decisions, verified plan, development commands and handoff.

PASS: debug/test APKs using existing both-ABI native libraries, strict Android
lint, Kotlin formatting/diff, and 16 focused cases on phone/light and landscape
tablet/dark (32 final executions). Five new cases verify all statuses/exact
commands, 205 unloaded matches/hidden selection, capture/update/delete failures,
atomic rollback/frozen-date Retry/later-match exclusion, Cancel/Back/recreation
and browser/chart/Library/effective-reader refresh without rescans; fresh import
gets a New ID. Corrected a test recomposition race and reset inherited display
overrides before the explicit matrix. Tablet host ADB wrapper returned 143 after
OK output; device logs confirm all 16 pass and runner exit 0. No open blocker.
Phone selection visual check passes; emulator display restored and emulator
stopped. Rust/Clippy/host JNI/codegen/Full-loader matrix/release/process-death/
performance/physical arm64 not rerun; no native/schema/dependency changes.
Next: profiling, AnkiDroid API feasibility and remaining lifecycle/accessibility.

## 2026-10-30 — Vocabulary history profiling and bounded Newest projections

Added a disposable, production-seeded host benchmark for all Vocabulary sorts,
selection, statistics, native FIFO callbacks and view/confirmation Bincode costs.
Measured 1k/10k/100k entries, 50k encounters and up to 1.08m status events in
release builds. Newest now selects registration-ordered candidates before history
probes; its 100k first-page median falls 108.177→30.456 ms and native callback
111.877→30.455 ms. No schema, dependency, protocol or product-behavior change.

PASS: 817 workspace tests (17 opt-in ignored), two harness cases, workspace build,
strict all-target/all-feature Clippy, Rustfmt/diff, 132 raw before/after metric rows
with seven samples each, zero-round/reread even-sample smoke and exact read-only
guards/cleanup. The new SQLite VM regression verifies unchanged first-page work
after offscreen original/effective encounters, correct last-page counts/date and
retained corruption rejection. Initial example enum spelling was corrected to
LastYear before verification; no remaining blocker. No Android/JNI/Kotlin/codegen,
Full-loader matrix, process-death or physical-arm64 rerun/claim.

Results/commands are in VOCABULARY_PERFORMANCE.md and DEVELOPMENT.md; raw CSV is
under docs/measurements. Statistics replay still takes ~1.04 s at 1.08m events;
100k selected IDs plus frozen prompt create a ~1.6 MB view. These remain measured
follow-ups, alongside diverse-book/peak-memory/Compose work. Updated architecture,
data/decisions/plan/handoff. Logs: /tmp/yomoku-vocabulary-performance-{tests,
example,build,clippy}.log and /tmp/yomoku-vocabulary-measure-{after-,}*.{csv,log}.

## 2026-10-30 — Profile and remove statistics history sorting

Separated SQL stepping, predecoded pure builder replay and complete repository
costs in the disposable host harness. The former ordered UNION sorts the full
status history twice; production now reads links then primary-key-ordered events
inside one deferred read transaction, retaining all validation and snapshot
semantics. No schema, dependency, core/wire or Android code change. Updated
architecture/data/decisions/plan, performance CSV/results, commands and handoff.

PASS: 24 focused statistics cases, 819 workspace tests (17 opt-in ignored), two
harness cases, workspace build, strict all-target/all-feature Clippy, Rustfmt/diff.
New cases verify zero sorts, concurrent erasure/new-generation isolation between
link/event reads and failure cleanup. Initial test compilation required internal
helper/SQL visibility; corrected, no open blocker. Four sequential release runs
at 10k/100k entries pass exact replay/bytes/cleanup; 104 seven-sample CSV rows
validate extrema/medians. Zero-round/reread even-sample smoke passes.

At 1.08m events, SQL steps fall 65.88m→39.96m and sorts 2→0; full year-statistics
median falls 944.157→542.853 ms (42.5%). At 108k events: 86.947→53.254 ms.
Residual decoding/replay and Frequency/LastSeen profiling remain, alongside
diverse-book/peak-memory/Compose work. No emulator/device/JNI/Kotlin/codegen,
Full-loader matrix/process-death/physical-arm64 rerun or performance claim.
See VOCABULARY_PERFORMANCE.md, DEVELOPMENT.md and measurements/statistics-host-
2026-10-30.csv. Logs: /tmp/yomoku-status-stream-*.log and
/tmp/yomoku-status-profile-{before,after}-*.{csv,log}.

## 2026-10-30 — Bound Frequency/LastSeen history projections

Shared the private production page SQL builder with opt-in diagnostics. Profiled
SQL stepping/VM work separately from complete repository reads, then selected
metric-sort candidates with only their required aggregate and deferred the other
summary/provenance probes. Reuse the selected metric and repeat outer ordering.
No schema/dependency/wire or product-behavior change. Updated architecture/data,
decisions/plan/handoff, performance results/raw CSV and CLI commands.

PASS: 12 browser cases, 821 workspace tests (17 ignored), two harness cases,
all-feature build, strict all-target/all-feature Clippy, Rustfmt/diff. New exact
257-row mixed-history paging/filter/tie coverage and offscreen VM-growth bound
pass; the latter fails the former SQL as expected (7,500 versus 600 Frequency
steps). All-sort corruption/query-failure cleanup passes. Initial diagnostics u64
SQL binding compile failure corrected to checked i64; no unresolved blocker.

Four sequential release runs (10k/100k before/after) pass exact replay/bytes/
integrity/FK/cleanup guards; all 120 seven-sample metric rows validate extrema and
median. Zero-history/two-sample smoke passes. At 100k, Frequency first-page median
108.717→64.419 ms and LastSeen 109.834→65.600 ms; last pages 155.940→103.059 ms and
155.573→107.287 ms. Deep offsets, status decoding/replay and device/Compose/memory
work remain. No Android/JNI/Kotlin/codegen/Full-loader/process-death/arm64 rerun.
See VOCABULARY_PERFORMANCE.md and DEVELOPMENT.md. Logs: /tmp/yomoku-browser-page-*
and /tmp/yomoku-browser-profile-*. No emulator started.

## 2026-10-30 — Statistics row decoding and temporary enum text
- Started clean; continued the ordered host-history performance follow-up. Added
  SQL-plus-decoding/provenance diagnostics using the exact private production
  decoder and enum parser, without new public API, dependency or schema changes.
- Borrow SQLite TEXT for old/new status and cause within each row, removing up
  to three temporary String allocations per event. Keep SQL, snapshots, required
  provenance checks, pure streaming replay and dictionary-form ownership intact.
- Two new regressions verify raw SQLite type/NULL gates and malformed UTF-8/
  nonliteral spellings through public timeline/calendar reads, unchanged bytes,
  snapshot cleanup and exact recovery after fixture repair. Initial BLOB writes
  were correctly rejected by STRICT tables; moved those cases to raw rows.
- PASS: 26 focused statistics cases, 823 workspace tests (17 opt-in ignored),
  three harness cases, all-feature build, strict Clippy, fmt/diff. Four release
  runs retain 124 checked raw metric rows plus a passing 22-metric zero-history/
  even-sample smoke. Replay/byte/FK/integrity/scratch-cleanup guards pass.
- At 100k entries/1.08m events, median SQL plus decode/validation falls
  596.927→532.782 ms; complete timeline 584.478→545.795 ms. Samples/hashes and
  host limits are in VOCABULARY_PERFORMANCE.md and measurements/statistics-
  decoding-host-2026-10-30.csv. No cached counters or core/wire behavior changes.
- No unresolved blocker. Android/JNI/Kotlin/codegen, Full-loader matrix, device/
  Compose/peak-memory/diverse-book/lifecycle/process-death/physical-arm64 not
  rerun or claimed. No emulator started. Broader hardening tasks stay unchecked.

## 2026-10-30 — Commit message verification guidance
- Added an AGENTS.md rule excluding test counts, verification summaries and
  build/lint/formatting results from commit message bodies; keep those details
  in session/progress docs and user-facing reports. Updated the handoff reminder.
- Reviewed the documentation diff and passed `git diff --check`. No code changes
  or runtime tests needed; implementation priorities and blockers are unchanged.


## 2026-10-30 — Vocabulary Compose and sampled process memory
- Started clean; continued the ordered Android performance follow-up. Added an
  opt-in instrumentation harness using production YomokuApp/CoreSession/JNI/FIFO
  with a disposable migrated/seeded SQLite file and forbidden analyzer access.
  Measures first/append paging, retained-row scrolling, global Select All and
  frozen confirmation; cancels without writes. No production/schema/wire change.
- Capture main-window FrameMetrics on a separate handler and sample used Java
  heap/native allocations/RSS about every 25 ms. Record observer/idle/debug,
  separate Dialog-window and sampled-high-water limits; no exact peak/PSS claim.
- PASS: both-ABI native rebuild/generated bindings, debug/test APKs, strict lint,
  repository Kotlin formatting/diff; 16 existing regressions plus default skip
  (17 reported; 205.770 s). Two isolated opt-in processes pass in 32.330/49.535 s:
  10k/100k entries, 1,000 loaded rows, five scenarios after a full warmup each.
- Validate all 50 action samples/110 raw frames, exact selected/frozen IDs,
  unchanged DB hashes, zero analyzer/reading effects, integrity/FK/closure and
  cleanup. No reported frame drops; all observed frames exceed fixed 16.667 ms.
  Raw CSV, hashes and methodology: VOCABULARY_ANDROID_PERFORMANCE.md.
- At 100k, Select All median action-to-idle is 865.486 ms; confirmation view
  ~1.65 MB. Largest measured-action samples: Java 43.619/native 28.574/RSS
  278.637 MiB at different times, not additive. Profile main callbacks and
  selection composition next; measurements do not attribute a single cause.
- Initial sandbox Gradle lock socket needed approved rerun; no source/test
  failure remains. Reset existing isolated AVD to phone/light; stopped the
  task-started emulator. Scratch files absent; existing dictionary caches kept.
  Logs: /tmp/yomoku-vocabulary-compose-{android,format,regressions,10000,100000}.log.
- No Rust host/Clippy, separate host JNI/codegen reproducibility/Full-loader,
  optimized/tablet/dark/continuous-fling/heavy-history/diverse-book performance,
  process-death or physical-arm64 rerun/claim. Broader hardening tasks stay open.

## 2026-10-30 — Bound Vocabulary Compose selection membership

- Replace the selection-sized hash set/structural remember key with binary search
  of core-sorted positive-i64 IDs per composed row. Full hidden/unloaded IDs,
  selected counts, frozen commands/dates and Retry/history semantics remain intact.
- Add exact/gapped/empty/single/large-ID and 100k logarithmic-read cases, plus
  Compose row-order/equal-size replacement coverage. PASS: 19 regression cases and
  one default assumption skip (20 reported, 228.889 s), debug/test APKs, strict
  lint, repository Kotlin formatting/diff. Native/binding/assets are up-to-date.
- Sequential 100k baseline/changed debug phone/light runs pass (51.327/50.541 s).
  Select All median idle 881.920→765.376 ms with overlapping ranges; confirmation
  and frame durations do not consistently improve. Avoid general speed/frame-rate
  or exact memory-savings claims; callback decoding/view equality remain targets.
- Validate 50 action rows/116 raw frames, positive sampled memory, payload sizes,
  zero drops, exact selected/frozen IDs, unchanged bytes, no analyzer/reading
  effects, integrity/FK/native closure and cleanup. Raw samples/hashes/limits:
  VOCABULARY_ANDROID_PERFORMANCE.md and vocabulary-selection-android CSVs.
- Initial sandbox Gradle local lock-socket failure required approved rerun; no
  open failure/blocker. Existing isolated AVD restored phone/light and stopped;
  ADB confirms no measurement scratch directories. Existing caches preserved.
  Logs: /tmp/yomoku-selection-{android,format-check,regressions,before-100000,
  after-100000}.log. No Rust/schema/dependency/wire/destructive change.
- No Rust host/Clippy, independent JNI/codegen/Full-loader, optimized/tablet/dark/
  continuous-fling/heavy-history/diverse-book/fully loaded device performance,
  exact peaks/PSS, process-death or physical-arm64 rerun/claim. Broader tasks stay
  open; next callback profiling, Anki API verification and lifecycle coverage.

## 2026-10-30 — Vocabulary Android snapshot phase profiling

- Started clean; continued the ordered callback profiling slice. Added an opt-in
  main-thread CoreSession render observer distinguishing native view retrieval,
  generated Kotlin decoding and state assignment, plus dispatch/callback source,
  payload and row/selected/frozen counts. Normal renders retain the original
  direct path without clocks/samples. No view/ID retention, equality-policy,
  sequence, threading, schema, dependency or wire change.
- Extend the disposable Vocabulary harness with deferred per-render output and
  full selected/frozen count guards. CoreSessionTest verifies observer install/
  removal alongside independent native handles and correct destinations.
- PASS: debug/test APKs, strict Android lint, Kotlin formatting/diff; 20 ordinary
  regressions plus default assumption skip (21 reported; 219.050 s). Native/
  bindings/assets reused outputs. Separate 10k/100k debug phone/light profiles
  pass (36.355/53.160 s), one warmup/five scenarios each, 1,000 retained rows.
- Validate 70 render rows/50 action rows/107 raw frames, exact selected/frozen
  IDs, unchanged hashes, zero analyzer/reading effects, integrity/FK/native
  closure and scratch cleanup. ADB confirms only the existing sudachi cache in
  no_backup. Task-started isolated emulator stopped; phone/light retained.
- At 100k Select All callback native/decode/publication medians are
  22.046/82.612/0.785 ms; confirmation 29.789/65.726/1.009 ms. Retain equality
  policy; all observed renders changed, so equal-snapshot cost stays unmeasured.
  Total idle residual is unattributed, not composition time. No before/after,
  generalized speed/frame or exact peak-memory claim. Raw CSV/hashes/limits:
  VOCABULARY_ANDROID_PERFORMANCE.md, snapshot phase section.
- Initial sandbox Gradle lock sockets needed approved rerun; initial nullable
  selection count compile error corrected. No unresolved failure/blocker. Logs:
  /tmp/yomoku-render-{android,format-check,regressions,10000,100000,validation}.log.
- Host Rust/Clippy, independent JNI/codegen/Full-loader, optimized/tablet/dark/
  continuous-fling/heavy-history/diverse-book/fully loaded, equal-snapshot,
  exact peak/PSS, process-death and physical arm64 not rerun or claimed. Continue
  native update/composition/decoding investigation, then Anki APIs/lifecycle.

## 2026-10-31 — Vocabulary request phases and equal snapshot replay

- Started clean; continued the ordered profiling task. Added opt-in event
  encoding/synchronous JNI update/request decoding measurements; callbacks leave
  unmeasured native intervals absent. Disabled diagnostics read no clocks or
  allocate records. No Rust/schema/dependency/wire/product-policy change.
- Added observer-gated main-thread native snapshot replay through production
  decode/publication. CoreSessionTest checks identity, no requests/notification,
  observer removal and thread guards. The disposable fixture replays exact full
  frozen confirmations; equality and product event guards remain unchanged.
- PASS: debug/test APKs, strict Android lint, Kotlin formatting/diff; 21 passing
  regressions plus default assumption skip (22 reported, 240.384 s). Native/
  bindings/assets reused Gradle outputs. Separate 10k/100k debug phone/light
  processes pass (38.092/60.011 s), 1,000 retained rows, warmup/five scenarios.
- Validate 80 render/70 request/60 action rows and 111 frames, exact IDs, unchanged
  bytes, zero analyzer/reading effects, integrity/FK/native closure/scratch cleanup.
  ADB confirms only sudachi cache remains; task-started isolated AVD stopped.
- At 100k Select All/confirmation synchronous native updates have medians
  0.298/0.335 ms; equal replay native/decode/publication 61.037/192.500/5.594 ms.
  Equal publication range 3.936–35.825 ms; all ten equal samples retain state,
  all 70 changed renders publish. Keep equality policy; async native callback
  work/composition remain unmeasured. No optimization/composition/peak claim.
- Raw CSV/hashes/method/limits: VOCABULARY_ANDROID_PERFORMANCE.md, final section.
  Logs: /tmp/yomoku-request-{android,format-check,regressions,10000,100000,
  validation}.log. Initial sandbox Gradle lock-socket access needed approved
  rerun; no source/test failure remains. Existing ignored caches preserved.
- Host Rust/Clippy, independent JNI/codegen/Full-loader, optimized/tablet/dark/
  continuous-fling/heavy-history/diverse-book/fully loaded, exact peaks/PSS,
  process-death and physical arm64 not rerun. Broader tasks stay unchecked.
  Continue controlled large-ID decoding/allocation and async callback/composition
  profiling, then Anki API verification and lifecycle matrix.

## 2026-10-31 — Controlled Vocabulary ID-list decoding comparisons

- Started clean; continued the ordered large-selection decoding/allocation task.
  Added test-only generated/preallocated/specialized fixed-width ID-list decoders
  with identical generated bytes and balanced forward/reverse order on the main
  thread. No production/runtime/generation/Rust/schema/dependency/wire change.
- PASS: debug/test APKs, strict Android lint, repository Kotlin formatting/diff;
  native/bindings/assets reused Gradle outputs and production APK hash unchanged.
  Six passing codec/selection/CoreSession cases plus default assumption skip
  (seven reported, 0.720 s). Exact empty/100k/gapped/>2^53/signed wire values,
  generated-byte round trips, truncations/bad lengths/trailers all pass.
- PASS: two opt-in processes per size (10k/1 0.671 s, 100k/1 2.156 s,
  100k/2 2.135 s, 10k/2 0.651 s), four warmup/eight balanced measured rounds each.
  All 128 raw rows validate positions/payloads/finite nonnegative counters and
  timings; every result matches all IDs/bytes and original input remains exact.
- 100k generated decode medians 25.502/23.098 ms versus capacity 25.341/23.795;
  buffered+capacity 3.303/3.029 ms, with long samples up to 46.911 ms. Specialized
  readers also omit generic/per-ID container calls; no production speed inference.
  Bytecode identifies per-read range/progression construction in generated 64-bit
  loops. Approximate process ART deltas lag/include unrelated work; zero samples
  and GC are retained in raw data. No exact allocation/peak/retained-memory claim.
- Results/hashes/limits: VOCABULARY_ANDROID_PERFORMANCE.md, controlled ID-list
  section; measurements/vocabulary-decoding-android-2026-10-31.csv. Logs:
  /tmp/yomoku-decoding-{android,format-check,regressions,10000-1,10000-2,100000-1,
  100000-2,validation}.log; bytecode -bytecode.txt. Initial sandbox Gradle socket
  startup needed approved rerun; no source/test failure or open blocker.
- Stopped the task-started isolated AVD; kept existing ignored caches/image.
- Host Rust/Clippy/JNI/independent generation, full Compose/native profiles,
  optimized/heavy-history/diverse-book/fully loaded/exact peak/lifecycle/physical
  arm64 checks not rerun. Broader tasks stay unchecked. Next: isolate/remove scalar
  loop progression allocations through reproducible generation, then full frozen
  snapshots; continue async callback/composition, Anki API and lifecycle tasks.


## 2026-10-31 — Generated scalar Kotlin decoder integration

- Started clean; continued the ordered scalar-allocation task. The Rust payload
  generator replaces only both pinned 64-bit stepped ranges with scalar loops,
  rejecting changed/missing/duplicated matches. All 17 files match repeat
  generation; only BinaryDeserializer differs from prior output. Bytecode removes
  production RangesKt calls while both legacy test loops retain until/step.
- Added shared host/Android legacy-oracle checks and opt-in generated-list/full
  synthetic frozen-ViewModel comparisons. Per-byte checks, signed/unsigned/float/
  128-bit values, errors/offsets, nested truncations/trailers, Unicode and exact
  empty/100k/gapped/>2^53 IDs/bytes remain correct. Generated list/container/wire,
  equality, product events, schema and dependencies stay unchanged.
- PASS: generator guard/targeted strict Clippy/Rustfmt, full real-Full host JNI/
  codecs/Unicode fixtures and latest shared JVM checks, reproducible generation,
  debug/test APKs, strict Android lint, repository/shared Kotlin formatting/diff.
  Eight focused cases plus two default skips pass (10 reported, 1.226 s); eight
  production Vocabulary/Statistics/effective-reader cases pass (249.438 s),
  including frozen retries, persistence, recreation, scope/Library/furigana.
- PASS: four separate opt-in processes (10k/1 1.441 s, 100k/1 3.433 s,
  100k/2 3.416 s, 10k/2 0.922 s), four warmup/eight balanced measured rounds per
  shape. All 128 raw rows validate sizes/positions/payload hashes/nonnegative
  counters/timings and every exact-ID/generated-byte/immutable-input guard.
  At 100k synthetic frozen-view decode medians legacy 55.207/53.924 ms improve
  to scalar 34.515/35.366 ms. No whole-app/callback/composition speed inference;
  classes/JIT/debug/GC/emulator matter. Lagged process ART allocation deltas
  include zeros/unrelated work and do not establish exact savings or peaks.
- Results/raw CSV/hashes/limits: VOCABULARY_ANDROID_PERFORMANCE.md, final section.
  Commands: DEVELOPMENT.md, Scalar Kotlin decoder integration. Logs:
  /tmp/yomoku-scalar-{generator-test,clippy,bridge,host,codegen,codegen-diff,
  android,format,regressions,production,10000-1,10000-2,100000-1,100000-2,
  validation}.log; -bytecode.txt and -legacy-bytecode.txt. Initial sandbox Gradle
  socket startup needed approved rerun; existing nonfatal host teardown messages
  persist. No source/test failure or open blocker. Stopped the task-started
  isolated AVD; kept the existing ignored caches/image.
- Full workspace Rust/Clippy, independent Full-loader matrix, full native/Compose/
  optimized/tablet/dark/fling/heavy-history/diverse-book/fully loaded/exact peaks,
  process-death/physical arm64 not rerun. Broad tasks remain unchecked. Next:
  measure actual callbacks/composition with the scalar decoder, then Anki API
  verification and remaining lifecycle/accessibility matrix.

## 2026-10-31 — Actual Vocabulary callbacks and Compose intervals

- Started clean; continued the ordered actual-path profiling task. Added opt-in
  volatile Kotlin callback arrival/handler queue/handler elapsed observation and
  test-only pinned Compose runtime pass observation with lifecycle detachment.
  Preserve callback order, structural equality, full selected/frozen IDs, event/
  replay guards and schema/dependency/wire. Record interval union, not nested sums;
  observed passes include inherited subcomposition and animations.
- PASS: debug/test APKs, strict Android lint, repository Kotlin formatting/diff;
  ten focused cases plus three default opt-in skips (13 reported, 11.216 s),
  17 Vocabulary UI/native persistence/Retry/recreation/navigation regressions
  (221.766 s). New cases verify callback main-thread delivery/removal/thread guard
  and composition change/equality/detachment.
- PASS: separate opt-in 10k/100k processes (37.639/54.632 s), one full warmup/five
  measured scenarios each. Validate 60 actions/80 renders/70 requests/30 callbacks/
  1,982 passes/108 frames, exact interval/count/frame/replay/full-ID guards,
  unchanged database bytes, zero analyzer/reading effects, integrity/FK/native
  close/scratch cleanup. ADB confirms scratch directories absent.
- 100k Select All median queue/handler/composition union: 54.153/71.460/192.414 ms.
  Equal replay retains the state instance and produces no requests/callbacks/
  composition passes. Debug/emulator/observer/heap/JIT effects and long samples
  prevent before/after speed or native-worker attribution; sampled memory is not
  exact peaks. Results/hashes/raw CSVs: VOCABULARY_ANDROID_PERFORMANCE.md; commands:
  DEVELOPMENT.md, Vocabulary callback queue and Compose passes.
- Logs: /tmp/yomoku-callback-{android,format,focused,regressions,10000,100000,
  validation}.log; validator /tmp/yomoku-validate-callback-profile.py. Initial
  sandbox Gradle local-socket startup needed approved rerun; no source/test
  failure or open blocker. Stopped task-started AVD; kept ignored image/caches.
- Native/bindings/assets reused build outputs. Host Rust/Clippy/JNI/codegen,
  independent Full-loader, native worker breakdown, optimized/tablet/dark/fling/
  heavy-history/diverse-book/full-load/exact peak/lifecycle/process-death/physical
  arm64 work not rerun. Broad tasks stay open. Next: native request/response and
  observer-free optimized profiling, Anki API verification, lifecycle matrix.

## 2026-10-31 — Native Vocabulary host worker diagnostics

- Started clean; continued the native profiling task with a default-off Rust
  storage-measurements feature over the existing effective-reader FIFO. Only
  Vocabulary page/ID reads allocate metadata/read clocks. No Android diagnostic
  API, dependency, schema, core, wire, equality or product behavior change.
- Separate pre-send/worker start, repository execution/cleanup and combined
  resolver/core/Render serialization/host callback durations. Deliver samples
  after callback on the worker; observers must not block/reenter. Keep host
  timings separate from Android callback/Compose intervals and device claims.
- PASS: workspace build, 826 tests/17 ignored data-dependent fixtures, three
  harness cases in each feature mode, strict all-feature workspace/feature-off
  harness Clippy, Rustfmt/diff. Two new FFI tests cover a blocked earlier FIFO
  callback, callback/sample ordering/thread, exact unloaded/frozen IDs, read-only
  failure and exact Retry. Corrected initial replay test setup and boxed only
  diagnostic metadata after enum-size lint; no remaining blocker.
- Four sequential optimized processes (10k off, 100k observed, 100k off, 10k
  observed) use ten status rounds, 1k rereads/50k occurrences, warmup/seven samples.
  All 136 metric rows/952 raw durations validate, with full exact-ID, unchanged
  bytes, no analyzer/dictionary loads, integrity/FK and scratch cleanup guards.
  100k Select All queue/repository/combined response medians:
  0.035759/7.748635/0.186407 ms; feature-off callback median 7.806115 ms. These
  identify host repository cost, not an Android bottleneck or production speedup.
- Results/hashes/CSV: VOCABULARY_PERFORMANCE.md, final section; commands:
  DEVELOPMENT.md, Native Vocabulary worker diagnostics. Logs:
  /tmp/yomoku-worker-{workspace,harness,harness-base,focused,clippy,clippy-base,
  build,build-base,build-measured,validation}.log and size/mode CSV/log pairs;
  validator /tmp/yomoku-validate-worker.py. No emulator started.
- Android/Kotlin/JNI/codegen, independent Full-loader, physical arm64, lifecycle/
  accessibility, optimized Android, exact peaks/diverse books not run here. Broad
  tasks stay unchecked. Next: actual Android native/observer-free optimized
  comparison, Anki API evidence, remaining lifecycle/accessibility matrix.
- Final staged diff caught CSV CRLF endings; the implementation commit proceeded
  despite that failed check. Follow-up normalizes only line endings, revalidates
  all raw values/guards and updates the digest. Final staged diff passes; no code
  changed and no benchmarks or Rust tests were repeated for this artifact fix.

## 2026-10-31 — Anki API evidence and pure note preparation

- Started clean; continued the planned Anki integration with primary-source
  inspection of release 2.25.3. ANKIDROID_API.md records wrapper/contract/provider/
  permission/deck/model APIs, exact first-field cross-model feasibility and HTML/
  layout limits, separate query/insert ambiguity and a supported browser-link
  candidate needing live resolution tests. No API dependency/version adopted.
- Added pure core AnkiNote and original Yomoku v1 four-field HTML/CSS model. Front
  contains only the surface; back has reading, ordered meanings and supplied
  canonical sentence with exactly one validated UTF-8 occurrence highlighted.
  Preserve raw identity; escape HTML/media brackets/provider separators. Missing
  reading/meanings remain empty. No mutable history/status input or effect.
- PASS: workspace build, 831 tests/17 ignored data-dependent fixtures, strict
  workspace/all-target/all-feature Clippy, Rustfmt/diff. Five new cases cover
  repeated words, exact Unicode/whitespace/inflection, absent definitions, literal
  markup/media/separators and malformed/stale/non-scalar target ranges.
- Updated architecture/decisions/checklist/handoff/commands. Logs:
  /tmp/yomoku-anki-{workspace,build,clippy}.log. No failed check or blocker.
- No Event/FFI/schema/dependency/Android behavior change. No APK/lint/Kotlin/JNI/
  codegen/Full-loader, provider export, device or performance runs. No emulator
  started. Next: canonical context/dictionary projection and retained typed
  export, Android provider contract/live tests, then pending native/optimized
  Vocabulary profiling and lifecycle/accessibility coverage. Full Add to Anki
  stays unchecked; physical arm64 remains user-deferred and Q188 remains pending.

## 2026-10-31 — Anki selected context and dictionary projection

- Added pure original/effective selection constructors on AnkiNote. Validate the
  owner's current page/base/revision, full token metadata and canonical ownership,
  exact surface and clipped display range; vocabulary observations may differ.
- Extract full chapter-local sentence context with shared pagination punctuation/
  closing-quote rules and canonical hard lines, retaining full crossing/manual
  targets, whitespace and a single selected occurrence highlight.
- Revalidate applicable surface/base dictionary indices; export all licensed
  source-ordered readings and English senses/name groups without repeating shared
  form matches. Preserve qualifiers; exact custom fallback requires no built-in
  match. Selected metadata reading is the final fallback; absent data stays empty.
- PASS: ten new core cases; 841 workspace tests/17 ignored data-dependent fixtures,
  workspace build, strict all-target/all-feature Clippy and Rustfmt/diff. Cover
  original/effective parity, restrictions/name data/inflections/shared forms,
  cross-page words, hard lines/quotes/decimals, literal custom/manual/absent data,
  stale/corrupt inputs and unchanged statuses/deleted vocabulary observations.
- Updated architecture, API continuation, decisions, checklist, commands/handoff.
  Logs: /tmp/yomoku-anki-selection-{workspace,build,clippy}.log. No blocker/failure.
  No dependency/schema/protocol/Android change; no Kotlin/JNI/codegen/APK/lint,
  Full-loader, provider, device or performance run. No emulator started.
- Next: retained export commands/results, Android provider contract/live tests,
  then remaining native/optimized Vocabulary profiling and lifecycle/accessibility
  checks. Full Add to Anki stays unchecked; Q188/physical arm64 remain as before.

## 2026-10-31 — Retained Anki export commands/results

- Started clean; continued the next Anki prerequisite with pure AnkiExportSession.
  Current original/effective constructors validate and freeze note, selection and
  external deck. Immutable shared payloads survive reader/dictionary/deck changes;
  fresh checked attempts and owning allocation identity reject stale, repeated
  and other-session callbacks. Typed positive signed-64 IDs retain exact values.
- Known pre-insert failures require explicit Retry with the accepted payload.
  Uncertain insertion survives clear/replacement; Retry only reconciles read-only.
  Empty/failed queries and wrong result kinds never permit another insert.
  Verified nonempty existing-note results resolve uncertainty; retain multiple
  matches without issuing a write. No vocabulary/history inputs can be mutated.
- PASS: ten new cases/20 focused selection-export cases, 851 workspace tests/
  17 ignored data-dependent fixtures, strict all-target/all-feature Clippy,
  workspace build and Rustfmt/diff. Cover frozen original/effective content,
  pre-insert failures, stale/corrupt selections, stale/foreign callbacks,
  uncertainty/reconciliation, long IDs/multiple matches and unchanged statuses,
  encounters, canonical text, page/analysis and dictionary inputs.
- Updated plan, architecture, API continuation, decisions, commands and handoff.
  Logs: /tmp/yomoku-anki-export-{workspace,build,clippy}.log. No failure/blocker.
- No dependency/schema/Event/FFI or Android/app/UI change. No APK/lint/Kotlin/JNI/
  codegen/Full-loader, provider, device or profiling check; no emulator started.
  Session-only guards do not establish process-death safety: durable pending/
  uncertain recovery and prevention of mutating replay are required before UI
  activation. Next: Android adapter/fake-provider tests, app/typed bridge/recovery/
  settings/UI and disposable real collection, then remaining Android profiling/
  lifecycle/accessibility. Full Add to Anki stays unchecked; Q188 and user-deferred
  physical arm64 remain unchanged.


## 2026-10-31 — Read-only Android Anki provider boundary

- Started clean; continued the next Anki prerequisite with unconnected
  AnkiProviderReader and narrow manifest provider visibility/database permission.
  Recheck enabled/exported provider, spec >=2 and permission for each worker-only
  operation; enumerate decks without selecting/creating. Preserve positive long
  IDs, labels and actual Boolean/string or documented integer filtered flags.
- Bound exact whole-first-field candidates span models/decks; compare complete
  returned fields again. Recognize canonical core escaping and syntax-free plain
  surfaces without arbitrary HTML decoding/normalization. Embedded NUL fails
  closed; query costs remain unmeasured. Null/failed/malformed/partial/close-failed
  cursors never publish empty/partial success. Empty reconciliation stays unresolved.
  Resolve note-ID browser links only in the current provider package, without launch.
- PASS: 12 new fake-provider cases with real Android SQLite plus three CoreSession
  regressions (15 total, 0.129 s), debug/test APKs, strict Android lint, repository
  Kotlin formatting and diff. Gradle rebuilt generated Kotlin/JNI for both ABIs
  and the debug Sudachi probe; assets reused. No independent Full-loader check.
- Corrected initial Kotlin fixture inference and cursor/URI lint findings. Approved
  execution resolved sandbox Gradle/ADB sockets. Default 6 GiB image install failed
  for storage; documented isolated 8 GiB image succeeded without clearing app data.
  Both task-started AVD runs stopped. No remaining failure/blocker. Logs:
  /tmp/yomoku-anki-read-{android,tests,format}.log and emulator logs.
- Updated plan, API evidence, architecture, decisions, commands and brief handoff.
  No dependency/schema/Rust source/Event/FFI/UI change or AnkiDroid installation,
  permission grant, real collection access or note export. Host Rust/Clippy/Rustfmt,
  physical device, broader UI/lifecycle, Full-loader and profiling were not rerun.
- Next: model-layout/mutating provider contracts, app/typed bridge/durable recovery,
  settings/UI and disposable real-collection/browser checks, then remaining Android
  profiling/accessibility/lifecycle work. Full Add to Anki remains unchecked; Q188
  and user-deferred physical arm64 are unchanged.

## 2026-10-31 — Read-only Android Anki model validation

- Started clean; continued the next independently testable Anki prerequisite.
  Added model ID/name discovery and fresh metadata/template validation by exact
  positive signed-64 ID. Same-name models remain distinct and never imply reuse.
  Require exact four-field order/CSS/name, normal type/sort field zero and one
  actual ordinal-zero forward template with exact name/question/answer and empty
  browser overrides. Default deck is separate from layout; future mutation must
  explicitly place cards in the frozen configured deck. Expected fields are
  snapshotted; production templates remain Rust-owned for the future typed bridge.
- Missing/incompatible models return false without mutation. Malformed/null/
  partial/close-failed reads fail; host/spec/permission refresh between queries
  rejects revocation/host replacement. No observation is an atomic reuse lease.
  No app/FFI/UI call site, model creation/overwrite or note insertion is enabled.
- PASS: nine new fake-provider cases plus 12 prior provider cases and three
  CoreSession regressions (24 total, 0.162 s), API-36 x86_64 isolated 8 GiB AVD;
  debug/test APKs, strict Android lint, repository Kotlin formatting and diff.
  Native/bindings/probe/assets reused; no independent Rust/Full-loader checks.
- Gradle/ADB sandbox sockets required approved execution. One pre-boot APK install
  failed; installation after boot succeeded. No app data cleared. Task-owned
  emulator stopped; no open blocker. Logs:
  /tmp/yomoku-anki-model-{android,tests,format,format-check,emulator}.log.
- Updated plan, API evidence, architecture, decisions, commands and brief handoff.
  No dependency/schema/protocol change or real collection/browser/physical-device/
  broader UI/lifecycle/performance run. Next: mutating adapter/model creation/deck
  placement, then durable pending/uncertain recovery and typed app/settings/UI
  wiring. Real collection checks remain required before compatibility claims.

## 2026-10-31 — Isolated Anki model creation

- Started clean; continued model creation as an independently testable part of
  the mutating adapter. Added worker-only `AnkiProviderModelCreator`, unconnected
  to app/FFI/UI. Refuse exact-name conflicts, insert a normal four-field/one-card
  model, validate a new positive signed-64 returned URI/ID, configure only its
  ordinal-zero template and require full metadata/template readback. Preserve
  existing models/notes; no default-deck selection or mutation. Rust owns layout.
- Refresh host/spec/permission between calls and after readback. Positive update
  counts represent properties rather than rows. All failures after submission
  are uncertain, retaining a validated new ID when known. No retry/repair/delete;
  partial models can remain. No atomic ownership/layout lease is claimed.
- PASS: 11 new fake-provider cases plus 21 prior provider cases and three
  CoreSession regressions (35 total, 0.217 s), API-36 x86_64 isolated 8 GiB AVD;
  debug/test APKs, strict lint, Kotlin formatting and diff. Native/bindings/probe/
  assets reused. Initial sandbox ADB socket attempt failed; approved execution
  passed. No app data cleared. Task-owned emulator stopped; no open blocker.
- Updated plan, API evidence, architecture, decisions, commands and handoff.
  Logs: /tmp/yomoku-anki-create-{android,tests,format,format-check,emulator}.log.
  No Rust/schema/dependency/protocol/UI change or independent Rust/Full-loader,
  real-provider/browser/collection, physical-device, broad lifecycle/performance
  check. Next: note insertion/layout/deck/duplicates/explicit placement, then
  durable pending/uncertain recovery and typed app/settings/UI wiring.

## 2026-10-31 — Isolated Anki note insertion

- Started clean; continued the next provider slice. Added worker-only
  `AnkiProviderNoteInserter` with frozen payload/model/layout/deck transport and
  shared exact-surface escaping validation. Full layout/nonfiltered-deck checks
  precede an immediate cross-model/deck duplicate recheck. No app/FFI/UI call site.
- Validate the returned canonical note URI and exact fields before touching its
  one ordinal-zero card, explicitly move to the frozen deck and require fresh
  note/card/deck readback. Refresh host/spec/permission between calls. Any failure
  after submission is uncertain, retaining payload/deck/known returned ID. No
  retry/repair/delete; read-only reconciliation cannot establish placement or
  permit insertion after an empty result. Durable ownership is still required.
- PASS: 16 new fake-provider cases plus prior provider/CoreSession regressions
  (51 total, 0.304 s), API-36 x86_64 isolated 8 GiB AVD; debug/test APKs, strict
  lint, repository Kotlin formatting and diff. Native/bindings/probe/assets reused.
  Initial sandbox Gradle wildcard-IP/ADB socket failures resolved with approved
  execution. APKs installed without clearing data; task-owned emulator stopped.
  No open blocker. Logs: /tmp/yomoku-anki-insert-{android,tests,install,format,
  format-check,emulator}.log.
- Updated plan, API evidence, architecture, decisions, commands and handoff.
  No Rust/schema/dependency/protocol/UI change or host Rust/Clippy/Rustfmt,
  independent Full-loader, real collection/browser, physical-device, broader
  UI/lifecycle/performance run. Next: durable pending/uncertain recovery with no
  restart mutation replay, typed app/FFI ownership and deck/settings/UI handling.


## 2026-10-31 — Durable Anki note-submission journal

- Started clean; continued the persistence prerequisite as an independent slice.
  Added pure validated/versioned `AnkiSubmission` data with frozen exact note,
  model/layout/deck and a migration-0021 SQLite journal. Only committed live
  preparation creates a nonserializable submission callback token; recovery
  returns separate read-only reconciliation data/IDs and never replays mutations.
- Preserve pending/uncertain obligations and known note IDs across interrupted
  insertion/placement. Empty/failed reads keep insertion blocked. Exact existing
  IDs resolve existence without claiming placement. Reject duplicate/stale
  callbacks, serialize concurrent preparation, never reuse committed IDs and keep
  obligations independent of reader/vocabulary deletion. Reader/vocabulary/
  statistics remain unchanged. Terminal payloads/results remain stored.
- PASS: 14 new host cases, 865 workspace tests/17 ignored fixtures, workspace
  build, strict Clippy, Rustfmt and diff. Initial helper-name compile error and
  four historical migration fixture failures corrected; final checks pass.
  No escalation/open blocker. Logs: /tmp/yomoku-anki-journal-{focused,workspace,
  build,clippy}.log. Updated plan, architecture/data model, decisions and handoff.
- No app/Event/FFI/provider call site, Kotlin/dependency change, Android checks,
  real collection/browser, independent Full-loader, physical-device or broader
  lifecycle/performance run. Next: retained app-owned restore/commit gates and
  typed provider bridge, model-creation recovery, then deck/settings/UI wiring.


## 2026-10-31 — Durable Anki model-creation recovery

- Started clean. Added pure validated/versioned `AnkiModelCreation` and migration
  0022 model journal with exact full layout, nonreused IDs, optional known model
  ID and retained terminal payload/results. Newly committed preparation alone
  produces a live callback token; restoration supplies read-only data/IDs.
- Pending/uncertain models block both creation and note insertion. Both journals
  share a mutation gate under immediate transactions. Empty/failed/partial reads
  preserve obligations; full-layout reconciliation requires the known returned
  ID when available. No restart replay, partial-model repair or cleanup. Stale
  callbacks cannot release later work; local reader/history deletion is separate.
- PASS: 11 new host cases, 25 focused Anki cases, 876 workspace tests/17 opt-in
  fixtures ignored; workspace build, strict Clippy, Rustfmt and diff. Corrected
  one old schema expectation and boxed an oversized enum variant after initial
  checks. No open blocker/escalation. Logs:
  /tmp/yomoku-anki-model-journal-{focused,workspace,build,clippy}.log.
- Updated architecture/data model, decisions, plan, commands and handoff. No
  dependencies, public Event/FFI or Kotlin/provider/UI call site changed. No
  Android, real collection/browser, independent Full-loader, physical device,
  broad lifecycle or performance verification. Next: retained app startup
  restore/outcome-commit gates and typed dispatch, then settings/Add to Anki UI.

## 2026-10-31 — App-owned Anki startup recovery

- Started clean. Added one-transaction combined journal restoration and moved
  validated recovery IDs/data into pure core, preserving storage reexports and
  private live callback tokens. No migration or dependency change.
- Retained Model owns closed startup/failed/uncertain gates and attempt IDs.
  Runtime schedules native restore before initial Library refresh on the existing
  FIFO; constructors stay lazy and local reader busy/errors/checkpoints remain
  independent. Explicit native Retry only; no restart provider replay or journal
  mutation. Live export/commit/reconciliation/provider/FFI/UI dispatch remain next.
- PASS: 15 new cases, 44 focused Anki cases, 891 workspace tests/17 opt-in fixtures
  ignored; workspace build, strict Clippy, Rustfmt and diff. Kotlin generation
  matches existing output byte-for-byte; recovery events/payloads have no shell
  codec. Fixed a missing asynchronous test wait and duplicate enum relocation
  during initial checks. Git staging needed sandbox escalation; no open blocker.
- Updated architecture/data model, decisions, plan, verification and handoff.
  Logs: /tmp/yomoku-anki-startup-*.log. No Android, host JNI JVM, real provider/
  collection/browser, separate Full-loader, physical device, broader lifecycle
  or performance verification. Next: live export ownership, durable dispatch/
  outcome/reconciliation gates, then visible recovery Retry/settings/actions.

## 2026-10-31 — App-owned Anki recovery evidence commits

- Started clean. Added native app/FIFO transport for read-only provider evidence
  commits, carrying the exact frozen restored pair. Under one immediate SQLite
  transaction, compare both journals, reuse existing reconciliation, validate the
  resulting pair and commit before returning it. Shared pure model-layout/known-ID
  rules now live in core with the previous storage reexport. No migration/dependency.
- App retains the obligation while saving; validates allowed results and attempt
  IDs. Errors, lost responses, stale/duplicate callbacks and newer obligations
  cannot open the gate. Failed commits require explicit read-only restore instead
  of replay. Reader busy/errors/checkpoints stay independent. Provider reads/writes,
  live export ownership, begin/outcome routing and visible UI remain unconnected.
- PASS: 18 new cases, 63 focused Anki cases, 909 workspace tests/17 ignored fixtures;
  build, strict Clippy, Rustfmt/diff and unchanged generated Kotlin output. Fixed
  two initial test-only unused-command warnings; final checks clean. No open blocker.
  Logs: /tmp/yomoku-anki-reconcile-*.log.
- Updated architecture/data model, decisions, plan, verification and handoff. No
  public codec/Kotlin/UI change. No Android, host JNI JVM, real provider/collection/
  browser, separate Full-loader, physical device, broader lifecycle or performance
  verification. Next: live export ownership, typed provider read/preflight dispatch
  and durable begin/outcome gates, then recovery Retry/settings/actions.

## 2026-10-31 — App-owned frozen Anki export preparation

- Started clean. Added native current-owner preparation for original/effective
  dictionary selections, guarded by startup/recovery-commit and existing reader/
  checkpoint/completion/departure/mutation gates. Retain exact fields/surface,
  deck, selection and full layout across duplicate events/dictionary changes/
  navigation. Shared pure constructors now freeze layout in isolated sessions too.
- Explicit Clear drops only unsubmitted data and preserves journal obligations/
  recovery commits. Preparation has no effects, live tokens, shell codecs or
  ViewModel projection; provider/preflight/begin/outcome/UI remain unfinished.
- PASS: 10 new cases, 29 focused core/FFI cases, 919 workspace tests/17 ignored;
  build, strict Clippy, Rustfmt/diff and unchanged Kotlin generation. Fixed initial
  incomplete analyzer fixture; final checks clean. No open blocker. Logs:
  /tmp/yomoku-anki-preparation-*.log. No Android, host JNI JVM, real provider,
  separate Full-loader, physical arm64, broad lifecycle or performance checks.
- Updated plan, architecture/data model, decisions, verification and handoff.
  Next: typed preflight/recovery reads and durable begin/outcome dispatch gates.


## 2026-10-31 — Native Anki recovery read callback ownership

- Started clean. Added one native read handle capturing both restored journals
  and a target; identity is unique to the app/read attempt. Foreign/stale/duplicate
  callbacks cannot consume newer reads. Wrong kinds/changed pairs fail closed;
  accepted evidence enters the existing durable commit gate with its frozen pair.
- Explicit Retry invalidates a lost read before restoring both journals. Pending
  journal commits cannot be abandoned; failed/lost commits need restore + fresh
  read. Reader state remains independent. No provider effect, shell codec, UI,
  dependency or schema change; Android dispatch/preflight/live gates remain open.
- PASS: 13 new cases, 50 focused Anki cases, 932 workspace tests/17 ignored;
  workspace build, strict Clippy, Rustfmt/diff and unchanged Kotlin generation.
  No open blocker. Logs: /tmp/yomoku-anki-read-*.log. No Android, host JNI JVM,
  real provider/collection/browser, separate Full-loader, physical arm64, broad
  lifecycle or performance checks.
- Updated plan, architecture/data model, decisions, verification and handoff.
  Next: typed Android/FFI reads/preflight and durable live begin/outcome routing.

## 2026-10-31 — User completion and physical-device checkpoints

- Recorded the user's request in AGENTS.md and the handoff: announce exhausted
  steps, recommend physical-device testing when it is more valuable than the
  next implementation step, and ask for needed product opinions/clarifications.
- Recommend a short physical arm64 local-reader/import/dictionary trial now;
  those workflows are implemented, while hardware usability/performance/lifecycle
  remain unverified. User chooses timing; no task was checked off.
- Documentation-only change; diff/whitespace checks pass. No code checks rerun.
