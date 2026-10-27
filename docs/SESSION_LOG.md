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
