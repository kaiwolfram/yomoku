# Resume here

Updated: 2026-10-22. Current phase: 1, Android emulator bridge test verified.
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
- CoreSessionTest PASS on medium_phone, Android 16/API 36 x86_64: typed JNI/core
  render, isolation and handle close. Async JNI callbacks await a real async effect.
- Gradle 9.1.0, AGP 9.0.1, Kotlin/Compose compiler 2.4.21, Compose BOM 2026.03.01,
  Activity 1.13.0, JDK 17, SDK 36/build tools 36.0.0, min SDK 29 pinned.
- Separate yomoku-storage: rusqlite 0.40.2/bundled SQLite, migration 0001 journal,
  atomic upgrades, reopen/rollback/failure preservation/concurrent-open tests.
  Builds for both Android ABIs. No runtime/FFI document persistence/tokenizer yet.
  Nostr configuration stays empty.

## Next tasks (ordered)
1. Add responsive four-tab navigation/toolbar Settings using Rust state; retain
   CoreSession across configuration changes; verify phone bottom bar/tablet rail.
2. Define stable document/source/chapter IDs and Unicode offsets (phase 2), then
   add document repositories/effects/FFI on the SQLite foundation; verify reopen.
3. Add CI using the verified host Rust/Kotlin checks and unsigned Android build.
Test async JNI delivery/error handling when the first real async effect is added.

## Last verified checks
From repository root, with installed tools/caches:
```sh
export PATH=/home/kai/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin:$PATH
cargo build --workspace --locked --offline
cargo test --workspace --all-features --locked --offline
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked --offline -- -D warnings
tools/test-kotlin-bridge.sh
git diff --check
```
All PASS: 26 Rust tests (11 storage), host JNI, Rust build/fmt/clippy.
Storage Android cross-builds PASS; exact NDK C compiler commands in DEVELOPMENT.md.
Previous session: app/test APK assembly, Android lint/Kotlin formatting, packaged
ELF/ZIP alignment PASS. Current emulator session: connectedDebugAndroidTest PASS
(one test). See DEVELOPMENT.md for JDK/SDK/Gradle environment and report paths.
Launcher cold restart, retained ADB authorization, APK install/Library launch,
launcher bash syntax and git whitespace checks PASS. Emulator left headless/online.

## Environment/blockers
- Rust compiler/cargo 1.99.1; rustup launcher attempts read-only maintenance.
  Use direct PATH above. BoltFFI `.local/bin/boltffi` 0.30.1, Kotlin host compiler
  `.local/kotlin-2.4.21/kotlinc/bin/kotlinc`, ktfmt 0.64 in `.local/`.
- Full local Temurin 17.0.21+9 avoids system runtime-only Java 25; archive verified.
  Android CLI 1.0.16625370, SDK /home/kai/Android/Sdk; adb at platform-tools/adb.
  NDK/Rust targets installed with approved escalation; SDK artifacts stay external.
- Gradle needs escalation even offline for local daemon/file-lock sockets
  (sandbox error: "Could not determine a usable wildcard IP"). ADB also needs
  local sockets. Host /dev/kvm exists/usable; sandbox-only checks hide it.
- Android CLI created medium_phone with Emulator 37.2.12/API 36 Play x86_64 r7.
  Default renderer/CLI startup crashes; use tools/start-android-emulator.sh
  (foreground, headless, swangle, KVM). Accept host debugging prompt once.
  Stop with android emulator stop emulator-5554. See DEVELOPMENT.md for setup.
- First build downloads test runner 1.7.0/JUnit extension 1.3.0; caches now populated.
  Connected tests additionally need AGP UTP dependencies; cached after online run.
  AGP 9 needs main.kotlin.directories; register generated app/com folders, never
  Crux's generated standalone build.gradle.kts. SDK XML v4 notice is benign.
- tools/build-android-native.sh builds debug Rust only, locked/offline. Native
  generation/build/tests share target output: NEVER run them concurrently.
  BoltFFI metadata expansion replaces host exports; host test rebuilds afterward.
- No curated Nostr data; ask Q188 only before Citrine forwarding.
- SQLite stays outside core. Storage has no runtime/FFI dependency yet; add real
  async effect execution and Android private-path selection with document commands.

See DEVELOPMENT.md/DECISIONS.md for setup, API evidence and resolved failures.
