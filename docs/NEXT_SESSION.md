# Resume here

Updated: 2026-10-22. Phase 2: immutable paste storage verified on host.
Read AGENTS.md, plan phases 1–2 and PRODUCT_SPEC.md section 3 next.
After each completed, verified step, update progress docs and commit it;
report the commit hash without waiting for another request.

## Working
- Pure Crux 0.20.0 core: Library-default/four destination commands, toolbar
  Settings open/close/return state and vocabulary rules. No SQL/platform I/O.
- Separate EffectRouter/typed FFI; 23 Rust integration tests and host Kotlin/JNI
  codecs/errors/isolation/listener release/100 lifecycle checks pass.
- Compose bottom bar below 600dp available width, scrollable rail from 600dp;
  four empty screens and Settings placeholder. Toolbar/system Back returns to tab.
- Activity-scoped CoreSessionOwner retains the same native session on recreation;
  onCleared closes it. Fresh core/process starts Library. No Kotlin domain state.
- Three Android tests PASS on API 36 x86_64: core JNI/isolation/close, navigation/
  Settings/Back, recreation/final cleanup. Phone and tablet portrait/landscape
  viewports verified. Screenshots visually inspected; phone viewport restored.
- Gradle preBuild generates Crux/BoltFFI Kotlin and combined Rust/JNI libraries
  for arm64-v8a/x86_64, NDK r30 30.0.16248370/API 29; targets installed.
- Pinned Gradle 9.1.0, AGP 9.0.1, Kotlin/Compose compiler 2.4.21, BOM 2026.03.01,
  Activity 1.13.0, Lifecycle 2.9.4, SDK 36/build tools 36.0.0, min SDK 29, JDK 17.
- Separate yomoku-storage: rusqlite 0.40.2/bundled SQLite, migrations 0001–0002,
  typed create/load paste commands, literal first-line titles and exact text.
  21 disk tests cover migrations/reopen, per-insert/commit rollback, concurrent
  writes and ID nonreuse/full-i64/exhaustion; both Android ABIs compile.
  No runtime/FFI document persistence/tokenizer yet. Nostr config empty.
- Pure document/source/chapter positive i64 IDs; chapter-local UTF-8 byte anchors,
  validated ranges/ownership and checked UTF-16 conversions. Seven Rust/generated
  Kotlin/Rust codec fixtures pass (supplementary/combining/ZWJ Unicode, full i64).
  Sources/chapters retain provenance separately from live titles/canonical text.

## Next tasks (ordered)
1. Wire document effects/runtime/FFI and Android private DB path off the UI thread;
   verify persistence/failures on device and real async JNI callbacks/errors.
   Reuse PasteText/PastedDocument + Storage::create_paste/load_paste; add Crux
   events/serialization and Library listing. App still creates no database.
2. Add logical pages/reflow using chapter-local offsets (phase 2); default 500,
   paragraph/sentence boundaries and Japanese/supplementary Unicode tests.
3. Add CI for verified host Rust/Kotlin checks and unsigned Android build.
Hide main navigation when the real phase-2 reader route is added (unchecked).

## Last verified checks
From repository root, with installed tools/caches (this slice):
```sh
export PATH=/home/kai/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin:$PATH
cargo build --workspace --locked --offline
cargo test --workspace --all-features --locked --offline
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked --offline -- -D warnings
tools/test-kotlin-bridge.sh
tools/format-kotlin.sh --check
git diff --check
```
PASS: 44 Rust tests (21 storage), host JNI + 7 Unicode exchanges, fmt/clippy,
Kotlin formatting and locked/offline storage builds for both Android ABIs
(exact NDK environment/commands in DEVELOPMENT.md, SQLite storage checks).
No APK/lint/device rerun this slice; prior app/test assembly, strict Android lint
and 3 phone/tablet tests passed. See DEVELOPMENT.md for Android commands/reports.
Initial history-corruption fixture assumed one migration row; scoped its updates
to version 1 and reran the full suite successfully. No remaining check failure.

## Environment/blockers
- Rust 1.99.1; rustup launcher attempts read-only maintenance. Use direct PATH.
  BoltFFI .local/bin/boltffi 0.30.1; Kotlin .local/kotlin-2.4.21/kotlinc/bin/kotlinc;
  ktfmt 0.64 in .local/. Full Temurin JDK 17 avoids runtime-only system Java 25.
- Gradle/ADB need escalation for local sockets even offline; dependencies cached.
  Back tests must wait for Compose idle before injection (BackHandler registration).
- Android CLI 1.0.16625370, SDK /home/kai/Android/Sdk, API 36 Play x86_64 r7 AVD
  medium_phone, Emulator 37.2.12. Host KVM works; sandbox hides /dev/kvm.
  Default CLI/SwiftShader startup crashes; use tools/start-android-emulator.sh
  (headless/swangle/KVM). ADB trust retained. Stop: android emulator stop emulator-5554.
- Native generation/build/tests share target: NEVER run concurrently. BoltFFI
  metadata expansion replaces host exports; host JNI script rebuilds afterward.
  SDK XML v4 notice/ktfmt Java 25 Unsafe warning benign. Arm64 execution untested.
- No curated Nostr data; ask Q188 only before forwarding. SQLite stays outside
  core; select Android private DB path and off-main-thread I/O with real effects.
- create_paste is storage only: dedup/registration/timestamps/progress await their
  planned tasks. No production rename/deletion API; tests use raw SQL fixtures.
