# Resume here

Updated: 2026-10-22. Phase 2: stable IDs/Unicode text anchors verified.
Read AGENTS.md, plan phases 1–2 and PRODUCT_SPEC.md section 3 next.
After each completed, verified step, update progress docs and commit it;
report the commit hash without waiting for another request.

## Working
- Pure Crux 0.20.0 core: Library-default/four destination commands, toolbar
  Settings open/close/return state and vocabulary rules. No SQL/platform I/O.
- Separate EffectRouter/typed FFI; 21 Rust integration tests and host Kotlin/JNI
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
- Separate yomoku-storage: rusqlite 0.40.2/bundled SQLite, migration 0001 journal,
  atomic upgrades, 11 reopen/rollback/failure/concurrent-open tests; both ABIs
  compile. No runtime/FFI document persistence/tokenizer yet. Nostr config empty.
- Pure document/source/chapter positive i64 IDs; chapter-local UTF-8 byte anchors,
  validated ranges/ownership and checked UTF-16 conversions. Seven Rust/generated
  Kotlin/Rust codec fixtures pass (supplementary/combining/ZWJ Unicode, full i64).
  DATA_MODEL.md defines allocation/provenance contracts for future repositories.

## Next tasks (ordered)
1. Add immutable paste documents/first-line titles and numbered SQLite migration
   using document.rs IDs/anchors; verify transaction rollback, ID nonreuse/reopen.
2. Wire document effects/runtime/FFI and Android private DB path off the UI thread;
   verify persistence/failures on device and real async JNI callbacks/errors.
3. Add CI for verified host Rust/Kotlin checks and unsigned Android build.
Hide main navigation when the real phase-2 reader route is added (unchecked).

## Last verified checks
From repository root, with installed tools/caches:
```sh
export PATH=/home/kai/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin:$PATH
cargo build --workspace --locked --offline
cargo test --workspace --all-features --locked --offline
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked --offline -- -D warnings
tools/test-kotlin-bridge.sh
tools/format-kotlin.sh --check
git diff --check
export JAVA_HOME="$PWD/.local/jdk-17/jdk-17.0.21+9"
export GRADLE_USER_HOME="$PWD/.local/gradle"
export ANDROID_HOME=/home/kai/Android/Sdk
./android/gradlew -p android :app:assembleDebug :app:assembleDebugAndroidTest :app:lintDebug --offline --console=plain
./android/gradlew -p android :app:connectedDebugAndroidTest --offline --console=plain
```
Current slice PASS: 32 Rust tests (11 storage), host JNI + 7 Unicode payload
exchanges, fmt/clippy/Kotlin formatting, app/test APK assembly, strict Android lint.
No device test rerun this slice; previous 3 tests passed at each viewport.
New fixture formatting: java -jar .local/ktfmt-0.64.jar --kotlinlang-style
--dry-run --set-exit-if-changed tests/kotlin/TextBoundaryTest.kt (one command).
See DEVELOPMENT.md for viewport commands, setup and report paths. UTP removes
app after tests; debug APK reinstalled/launched for visual checks and left open.

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
