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
