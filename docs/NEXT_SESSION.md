# Resume here

Updated: 2026-10-22. Current phase: 1, Android native packaging next.
Read AGENTS.md, plan phase 1 and PRODUCT_SPEC.md section 2 next.
After each completed, verified step, update progress docs and commit it
without waiting for another request; report the commit hash.

## Working
- Pure Crux 0.20.0 core, Library default/four destination commands, vocabulary rules.
- Rust EffectRouter/FFI with 15 integration tests; Crux Kotlin payload generation
  (Facet 0.46.5/facet_generate 0.19.1) and BoltFFI 0.30.1 Kotlin/JNI bindings.
- Host JNI tests pass: codecs/navigation, typed errors, state isolation, listener
  release and 100 handle lifecycles. Async callbacks await a real async effect.
- `android/`: Compose scaffold, Library heading, system theme/insets, min SDK 29;
  manual-backup policy excludes automatic backup/transfer. No Rust loading yet.
- Gradle 9.1.0/SHA-256, AGP 9.0.1, Kotlin/Compose compiler 2.4.21, Compose BOM
  2026.03.01, Activity 1.13.0, JDK 17, SDK 36/build tools 36.0.0 pinned.
- No persistence or tokenizer yet. Nostr configuration stays empty.

## Next tasks (ordered)
1. Verify/pin/install NDK, add Rust x86_64/arm64 Android targets, generate and
   package Kotlin/JNI/native libraries. Do not duplicate application state in Kotlin.
2. Test command → core → view on Android emulator/device; then responsive four-tab
   navigation and toolbar Settings (phone bottom bar/tablet rail).
3. Add separate Rust SQLite storage with numbered migrations and reopen tests.
Test async JNI callback delivery when the first actual async effect is added.

## Last successful checks
From repository root, with local tools/caches already installed:
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
./android/gradlew -p android :app:assembleDebug :app:lintDebug --offline --console=plain
tools/format-kotlin.sh --check
bash -n android/gradlew tools/format-kotlin.sh tools/test-kotlin-bridge.sh
git diff --check
```
All PASS: 15 Rust tests, host Kotlin/JNI suite, debug APK and strict Android lint.
APK metadata confirms min 29, target/compile 36. No Android behavior tests exist or
were run yet. First lint found a template v26 icon folder; moved to mipmap-anydpi.

## Environment/blockers
- Rust compiler/cargo 1.99.1; plain rustup launcher attempts read-only maintenance.
  Use direct PATH above. BoltFFI `.local/bin/boltffi` 0.30.1; Kotlin host compiler
  `.local/kotlin-2.4.21/kotlinc/bin/kotlinc`. Tools/generated output are ignored.
- Official Temurin 17.0.21+9 archive SHA-256 checked; full local JDK above avoids
  system runtime-only Java 25. ktfmt 0.64 jar is in `.local/`; Java 25 emits an
  upstream Unsafe warning, JDK 17 formatting is clean. See DEVELOPMENT.md setup.
- Android CLI 1.0.16625370/template 0.1.1 uses approved escalation. SDK path above;
  platform 36 revision 2, build tools 36.0.0, platform-tools 37.0.1 installed.
  adb is at `$ANDROID_HOME/platform-tools/adb`. No NDK/emulator installed by us;
  no `/dev/kvm`. Device/emulator execution remains unverified.
- Gradle requires escalation even offline for local daemon/file-lock sockets;
  sandbox failure is "Could not determine a usable wildcard IP". Initial downloads
  need network access. SDK XML v4/unstripped AndroidX graphics notices are benign.
- BoltFFI metadata expansion replaces host exports: rebuild Rust AFTER generation
  (host runner does this). Avoid concurrent generation/builds sharing target/.
- No curated Nostr starter data; ask Q188 only before Citrine forwarding.

See DEVELOPMENT.md/DECISIONS.md for setup, API evidence and resolved failures.
