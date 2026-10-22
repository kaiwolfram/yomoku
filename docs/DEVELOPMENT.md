# CLI development

Run from the repository root. Dependencies are pinned by Cargo.lock; on a fresh
workstation run `cargo fetch --locked` with network access before offline checks.
The agent sandbox required escalation for downloads into the Cargo cache.

```sh
# Current sandbox: bypass rustup launcher self-maintenance in a read-only home.
export PATH=/home/kai/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin:$PATH
cargo build --workspace --offline
cargo test --workspace --all-features --offline
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --offline -- -D warnings
```

Verified 2026-10-22 using rustc 1.99.1 and cargo 1.99.1. A normal workstation
with functioning rustup can omit the PATH workaround. Do not assume this compiler
is the project's minimum supported version; establish that with dependencies.
Cargo.lock is retained for reproducible application development.
Crux 0.20.0 declares Rust 1.90 minimum; the full locked graph has only been tested
with 1.99.1. Current host baseline: fifteen integration tests (four app, five domain, four bridge, two native facade).

## Kotlin payload generation
```sh
cargo run -p yomoku-ffi --bin generate-kotlin --features facet_typegen --offline -- target/generated/kotlin
```
Uses Crux 0.20.0, Facet 0.46.5 and locked facet_generate 0.19.1. The old
`typegen` feature was replaced with `facet_typegen`: Crux's Serde generator
has no Kotlin backend. Generated artifacts stay under ignored `target/`.
Use a dedicated generated output directory: upstream replaces its package folder.
Provisional internal package is `app.yomoku.shared`, not a finalized Android ID.
Output includes `app/yomoku/shared/Shared.kt` and `com/novi` Bincode/Serde runtime.
Generate again into `target/generated/kotlin-repeat`, then verify reproducibility:
```sh
diff -r target/generated/kotlin target/generated/kotlin-repeat
```
Generation and this comparison PASS. Standalone Kotlin 2.4.21 and BoltFFI CLI
0.30.1 are installed under ignored `.local/`; host Kotlin/JNI execution passes
(see below). The Android Gradle wrapper and its pinned toolchain are described below.

## Android Compose shell
The CLI-generated project lives in `android/`; normal builds need no Android
Studio. Its current UI is a Rust-backed Library heading with system light/dark theme and
window insets. Native packaging is integrated; interactive navigation and
persistence are separate steps.
Namespace/application ID `app.yomoku` is provisional, not a publishing commitment.
Automatic cloud backup and device transfer are excluded; the product requires
manual encrypted backups.

Pinned versions: Gradle **9.1.0** (wrapper distribution SHA-256 checked),
AGP **9.0.1**, Kotlin/Compose compiler **2.4.21**, Compose BOM **2026.03.01**,
Activity Compose **1.13.0**, JDK **17**, compile/target SDK **36**, build tools
**36.0.0**, minimum SDK **29**, NDK r30 **30.0.16248370**. AGP uses built-in Kotlin;
do not also apply `org.jetbrains.kotlin.android`.

On Fedora install `java-17-openjdk-devel` and point `JAVA_HOME` at the full JDK.
This sandbox instead uses the official Temurin **17.0.21+9** Linux x64 archive,
verified against the SHA-256 in Adoptium's release API metadata and extracted
under ignored `.local/jdk-17/jdk-17.0.21+9`. It leaves system Java unchanged.
Archive URL:
`https://github.com/adoptium/temurin17-binaries/releases/download/jdk-17.0.21.9/OpenJDK17U-jdk_x64_linux_hotspot_17.0.21_9.tar.gz`
SHA-256: `8b9b611d917e3dfd8419d3566b5949226f69b052fed7e109c1368add1dfe8711`.

Installed Android CLI **1.0.16625370**, template bundle **0.1.1**, SDK at
`/home/kai/Android/Sdk`. Generation installed platform 36 (revision 2). The first Gradle build also
installed platform-tools **37.0.1** (`adb` 1.0.41), available at
`$ANDROID_HOME/platform-tools/adb`. Generation/setup commands:
```sh
# Already generated: do not rerun over the existing android/ project.
android create empty-activity --name=Yomoku --namespace=app.yomoku --application-id=app.yomoku --min-sdk=29 --output=android
android sdk install 'build-tools;36.0.0'
```
On a fresh workstation also use `android sdk install 'platforms;android-36'`.
The CLI needs sandbox escalation for its bundle lock and SDK writes; Gradle needs
network access for the first wrapper/dependency downloads. In this sandbox use:
```sh
export JAVA_HOME="$PWD/.local/jdk-17/jdk-17.0.21+9"
export GRADLE_USER_HOME="$PWD/.local/gradle"
export ANDROID_HOME=/home/kai/Android/Sdk
./android/gradlew -p android --version
./android/gradlew -p android :app:assembleDebug :app:lintDebug --console=plain
```
Debug APK: `android/app/build/outputs/apk/debug/app-debug.apk`.
Build products, SDK paths, Gradle caches and keys are ignored. Kotlin compiler
warnings and Android lint warnings fail the build; dependency-update reminders
are reviewed separately from lint. The generated example data repository,
Kotlin navigation state and template tests were removed: application state belongs
in Rust. The new CoreSession instrumentation test exercises the packaged bridge;
see the native Android section below for its build and execution commands.
There is no `/dev/kvm` in this sandbox; device/emulator execution is unverified.

Verified 2026-10-22: debug APK build and strict lint PASS; repeat after the
launcher-resource fix with `--offline` PASS. `aapt dump badging` confirms min SDK
29, target/compile SDK 36 and app ID/version. Android unit/instrumentation tests
were not run. Initial lint rejected the template's unnecessary `mipmap-anydpi-v26`
folder; moving its icons to `mipmap-anydpi` fixed it without suppression.
The first SDK scan printed an XML version 4 compatibility warning and left the
AndroidX graphics native library unstripped; neither prevented the build/lint.
Gradle requires sandbox escalation even with `--offline` because its daemon and
file-lock service need local sockets (otherwise "Could not determine a usable
wildcard IP"). Fresh-cache network access is also required.

## Kotlin formatting
Use standalone **ktfmt 0.64** (Apache-2.0), with Kotlin language style, for Android
Kotlin and Gradle Kotlin scripts. Generated bindings stay excluded. Install once:
```sh
curl -fL https://repo.maven.apache.org/maven2/com/facebook/ktfmt/0.64/ktfmt-0.64-with-dependencies.jar -o .local/ktfmt-0.64.jar
tools/format-kotlin.sh
tools/format-kotlin.sh --check
bash -n tools/format-kotlin.sh
```
The script uses `JAVA_HOME/bin/java` when set, otherwise `java`; `JAVA_BIN` and
`KTFMT_JAR` override those defaults. This does not reformat the standalone
Kotlin bridge test yet. Android lint runs through the Gradle command above.

## Native Android build and test

Install the pinned NDK and matching Rust targets once (no Studio required):
```sh
android sdk install 'ndk;30.0.16248370'
rustup target add aarch64-linux-android x86_64-linux-android
```
The NDK was installed through the Android CLI into the SDK above; Rust target
components match the existing stable 1.99.1 compiler. Configure PATH/JAVA_HOME/
GRADLE_USER_HOME/ANDROID_HOME as above. Install BoltFFI CLI as described below.
Fetch Rust dependencies and allow Gradle network access once on a fresh cache.

```sh
./android/gradlew -p android :app:assembleDebug :app:assembleDebugAndroidTest :app:lintDebug --offline --console=plain
# Or generate/package native artifacts without Gradle:
tools/build-android-native.sh
```

Gradle preBuild invokes the same script with its SDK path and tracks Rust inputs
and generated outputs. It generates Crux codecs and uses BoltFFI 0.30.1 to build
and combine Rust static archives with generated JNI C. Outputs stay under
`target/generated/{kotlin,native,android}`; BoltFFI also places temporary linker
files under `crates/yomoku-ffi/target/`. All are ignored. Both development ABIs
(`arm64-v8a`, `x86_64`) are always packaged. The Rust profile is currently debug;
optimized release tooling remains a later task. `BOLTFFI_BIN` overrides the local
CLI path; `ANDROID_NDK_HOME` can select an installation of the exact pinned NDK.
The script fails early for another NDK or missing targets and uses locked/offline
Cargo. Do not run native generation/build/test scripts concurrently.

`CoreSession` dispatches generated events and projects Render requests into a
Compose view, owning/closing a native handle with the Activity. Interactive
navigation still needs configuration-change retention. Instrumentation source is
`android/app/src/androidTest/java/app/yomoku/CoreSessionTest.kt`; its APK is
`android/app/build/outputs/apk/androidTest/debug/app-debug-androidTest.apk`.
Run on a connected API 29+ arm64 device or x86_64 emulator:
```sh
"$ANDROID_HOME/platform-tools/adb" devices -l
./android/gradlew -p android :app:connectedDebugAndroidTest --offline --console=plain
./android/gradlew -p android :app:installDebug --offline --console=plain
"$ANDROID_HOME/platform-tools/adb" shell am start -n app.yomoku/.MainActivity
```
No device is currently connected; no emulator/system image or `/dev/kvm` is
available here. Instrumentation execution and app launch remain unverified.
Install/configure an emulator on a workstation with virtualization, or connect a
device with USB debugging, before claiming Android behavior passes.

Check APK packaging with the pinned CLI tools:
```sh
unzip -l android/app/build/outputs/apk/debug/app-debug.apk
"$ANDROID_HOME/build-tools/36.0.0/zipalign" -c -P 16 -v 4 android/app/build/outputs/apk/debug/app-debug.apk
"$ANDROID_HOME/ndk/30.0.16248370/toolchains/llvm/prebuilt/linux-x86_64/bin/llvm-readelf" -l target/generated/android/jniLibs/arm64-v8a/libyomoku_jni.so
"$ANDROID_HOME/ndk/30.0.16248370/toolchains/llvm/prebuilt/linux-x86_64/bin/llvm-readelf" -l target/generated/android/jniLibs/x86_64/libyomoku_jni.so
```

Verified 2026-10-22: app and instrumentation APK assembly plus strict lint PASS
offline. Both APK-packaged bridge libraries have the correct ELF architecture,
JNI/Rust exports and 16 KB LOAD alignment; APK ZIP alignment PASS. The bridge is
combined into a single library per ABI, with no separate Rust .so or host-libc
dependency. Stripped bridge sizes: 956,888 bytes arm64 and 978,128 bytes x86_64.
Host Kotlin/JNI suite, 15 Rust integration tests, fmt/clippy, Kotlin formatting
and shell syntax PASS. Instrumentation execution/app launch were **not run**.
First offline test dependency resolution failed; the approved online build
populated the cache. Initial source registration attempts failed until using
AGP's Kotlin directories and excluding Crux's standalone generated .kts file.

## Native bridge host build
`cargo build --workspace --offline` emits `target/debug/libyomoku_ffi.so` and
`libyomoku_ffi.a` alongside the Rust library. BoltFFI 0.30.1 generates the native
exports; no hand-written unsafe ABI is maintained. Verified exported symbols:
```sh
nm -D --defined-only target/debug/libyomoku_ffi.so | rg 'boltffi_.*(native_bridge|effects_listener)'
```
Constructor, update/resolve/view, callback creation/registration and release are
present. Host facade tests verify isolation, errors, thread sharing and listener
release. The additional Kotlin/JNI test below verifies the generated ABI, Kotlin
exceptions and foreign handle/listener cleanup. Navigation emits only synchronous
effects; asynchronous delivery must be exercised once an actual asynchronous
runtime operation is introduced.

## Kotlin/JNI host verification

Verified with BoltFFI CLI 0.30.1, Kotlin/JVM 2.4.21 (bytecode target 17), Fedora
OpenJDK runtime 25.0.4.1 and `cc`. No Android SDK or Gradle is needed for this test.
Install the generator from the repository root:

```sh
cargo install boltffi_cli --version 0.30.1 --locked --root .local
```

Download the official [Kotlin 2.4.21 compiler archive](https://github.com/JetBrains/kotlin/releases/tag/v2.4.21)
and unpack into `.local/kotlin-2.4.21/` (containing `kotlinc/bin/kotlinc`). See the
[Kotlin CLI instructions](https://kotlinlang.org/docs/command-line.html).
Use a Linux JDK's headers via `JAVA_HOME` or `JNI_INCLUDE_DIR`; Fedora's
`java-25-openjdk-devel` supplies them. This sandbox has only the runtime, so the
two unmodified official OpenJDK headers were downloaded locally instead:

```sh
mkdir -p .local/jni-25/include/linux
curl -fL 'https://raw.githubusercontent.com/openjdk/jdk/jdk-25%2B36/src/java.base/share/native/include/jni.h' -o .local/jni-25/include/jni.h
curl -fL 'https://raw.githubusercontent.com/openjdk/jdk/jdk-25%2B36/src/java.base/unix/native/include/jni_md.h' -o .local/jni-25/include/linux/jni_md.h
tools/test-kotlin-bridge.sh
```

The script defaults to those local tools/headers, and accepts `BOLTFFI_BIN`,
`KOTLINC_BIN`, `JAVA_BIN`, `CC`, `JAVA_HOME` and `JNI_INCLUDE_DIR` overrides.
Set the Rust PATH workaround above in this sandbox. Once tools and Cargo caches
are populated, the test runs offline. Keep normal Cargo target output at `target/`.
The Java command uses `--enable-native-access=ALL-UNNAMED` and `-Xcheck:jni`.

PASS: all destinations/reselection, event/request/view codecs through actual JNI,
typed InvalidEvent/InvalidResponse exceptions, recovery after errors, independent
instances, repeated close, calls after close and 100 lifecycles releasing their
Kotlin listener registry entries. Listener cleanup inspection depends on the pinned
generator's private registry; review the test when upgrading BoltFFI. No synthetic
async product event is added; async callbacks await the first real async effect.

The script compiles generated C and Kotlin with warnings treated as errors.
`bash -n tools/test-kotlin-bridge.sh` checks script syntax. Android lint and ktfmt check the handwritten Android Kotlin (see above).

`crates/yomoku-ffi/boltffi.toml` selects native package
`app.yomoku.nativebridge`, library `yomoku_jni`, system loading and min SDK 29.
Generated output stays under `target/generated/native/`. Generation invokes
metadata expansion that replaces the host shared-library artifact, so the runner
**rebuilds Rust after BoltFFI generation**, before linking JNI. A build performed
only before generation fails at link time with missing native-bridge exports.
Do not run generation concurrently with native builds/tests in the same target dir.
Both Crux payload and BoltFFI generation were repeated and recursively diffed
successfully. To repeat native generation manually:

```sh
(cd crates/yomoku-ffi && ../../.local/bin/boltffi generate kotlin --deny-skipped --output ../../target/generated/native-repeat)
diff -r target/generated/native target/generated/native-repeat
cargo build --workspace --locked --offline
```
