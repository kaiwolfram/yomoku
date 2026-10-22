# Resume here

Updated: 2026-10-22. Current phase: 1, native Android bridge packaged.
Read AGENTS.md, plan phase 1 and PRODUCT_SPEC.md section 2 next.
After each completed, verified step, update progress docs and commit it
without waiting for another request; report the commit hash.

## Working
- Pure Crux 0.20.0 core: Library default/four destination commands, vocabulary rules.
- Separate EffectRouter/typed FFI; 15 Rust integration tests and host Kotlin/JNI
  codecs/errors/isolation/listener release/100 lifecycle checks pass.
- Gradle preBuild generates Crux/BoltFFI Kotlin and combined Rust/JNI libraries
  for arm64-v8a and x86_64. NDK r30 30.0.16248370/API 29; Rust targets installed.
- Compose Library heading now reads CoreSession's Rust render snapshot; no domain
  state in Kotlin. Activity closes native handle; recreation starts a fresh core.
- CoreSessionTest commands/core/view/isolation/close test APK compiles. Android
  execution is NOT verified. Async JNI callbacks await a real async effect.
- Gradle 9.1.0, AGP 9.0.1, Kotlin/Compose compiler 2.4.21, Compose BOM 2026.03.01,
  Activity 1.13.0, JDK 17, SDK 36/build tools 36.0.0, min SDK 29 pinned.
- No persistence/tokenizer yet. Nostr configuration stays empty.

## Next tasks (ordered)
1. Connect API 29+ arm64 device/x86_64 emulator; run :app:connectedDebugAndroidTest
   and launch app. No device attached, emulator installed or /dev/kvm here.
2. Add responsive four-tab navigation/toolbar Settings using Rust state; retain
   CoreSession across configuration changes; verify phone bottom bar/tablet rail.
3. Add separate Rust SQLite storage with numbered migrations and reopen tests.
Test async JNI delivery/error handling when the first real async effect is added.

## Last successful checks
From repository root, with installed tools/caches:
```sh
export PATH=/home/kai/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin:$PATH
cargo build --workspace --locked --offline
cargo test --workspace --all-features --locked --offline
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked --offline -- -D warnings
tools/test-kotlin-bridge.sh
export JAVA_HOME="$PWD/.local/jdk-17/jdk-17.0.21+9"
export GRADLE_USER_HOME="$PWD/.local/gradle"
export ANDROID_HOME=/home/kai/Android/Sdk
./android/gradlew -p android :app:assembleDebug :app:assembleDebugAndroidTest :app:lintDebug --offline --console=plain
tools/format-kotlin.sh --check
bash -n android/gradlew tools/format-kotlin.sh tools/test-kotlin-bridge.sh tools/build-android-native.sh
git diff --check
```
All PASS: 15 Rust tests, host JNI, debug app/test APK assembly, strict Android lint.
Packaged arm64/x86_64 ELF architectures, JNI/Rust exports, 16 KB LOAD alignment
and APK ZIP alignment PASS; see DEVELOPMENT.md. No Android tests/app launch run.

## Environment/blockers
- Rust compiler/cargo 1.99.1; rustup launcher attempts read-only maintenance.
  Use direct PATH above. BoltFFI `.local/bin/boltffi` 0.30.1, Kotlin host compiler
  `.local/kotlin-2.4.21/kotlinc/bin/kotlinc`, ktfmt 0.64 in `.local/`.
- Full local Temurin 17.0.21+9 avoids system runtime-only Java 25; archive verified.
  Android CLI 1.0.16625370, SDK /home/kai/Android/Sdk; adb at platform-tools/adb.
  NDK/Rust targets installed with approved escalation; SDK artifacts stay external.
- Gradle needs escalation even offline for local daemon/file-lock sockets
  (sandbox error: "Could not determine a usable wildcard IP"). ADB also needs
  local sockets; approved `adb devices -l` succeeded with no attached devices.
- First build downloads test runner 1.7.0/JUnit extension 1.3.0; caches now populated.
  AGP 9 needs main.kotlin.directories; register generated app/com folders, never
  Crux's generated standalone build.gradle.kts. SDK XML v4 notice is benign.
- tools/build-android-native.sh builds debug Rust only, locked/offline. Native
  generation/build/tests share target output: NEVER run them concurrently.
  BoltFFI metadata expansion replaces host exports; host test rebuilds afterward.
- No curated Nostr data; ask Q188 only before Citrine forwarding.

See DEVELOPMENT.md/DECISIONS.md for setup, API evidence and resolved failures.
