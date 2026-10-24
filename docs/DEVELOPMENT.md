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
with 1.99.1. Current host baseline: 182 data-free Rust tests (99 storage, seven
runtime). Eleven analysis fixtures and one runtime fixture run separately with
each Full loader via tools/test-sudachi.sh (see below).

## Pure logical pagination checks
`cargo test -p yomoku-core --test pagination --all-features --locked --offline`
runs the chapter/page/reflow fixtures without Android or SQL. All five presets
and 25 reflow pairs cover exact thresholds, LF/CRLF, mixed scripts, supplementary
Unicode, combining marks, sentence fallback/quotes, EOF and invalid ownership.
Workspace build/test/fmt/clippy pass after adding this module. Android renders
the containing page at the persisted exact offset and global page-length preset.
Page movement/completion remains next.

## Pure active-time accounting checks
`cargo test -p yomoku-core --test active_time --all-features --locked --offline`
runs thirteen domain tests without Android, SQLite or real clocks. They cover
the exact five-minute idle boundary, interaction/restart after idle, dictionary
interactions, background/close, duplicate lifecycle signals, checkpoint frequency,
restoring saved durations with a fresh clock epoch, regression and overflow.
`cargo test -p yomoku-storage reading_time --locked --offline` adds twelve durable
pending-time tests: all 25 reflow pairs, exact Unicode/EOF attribution, backward
movement, SQLite reopen with fresh clock epochs, duplicate/stale/concurrent
checkpoints/completions, rereads, overflow, deletion and statement/commit/migration
rollback. Migration 0009 consumes frozen time with completion history/position.
Reboot remains simulated at the duration boundary; platform lifecycle wiring and
actual Android close/reboot verification remain unchecked. Runtime/FFI checkpoint
ordering remain future work. Analyzed occurrence/status completion now commits
atomically in the repository (migration 0010); application wiring remains open.

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
Studio. Its current UI has a Rust-backed Library list, paste form and saved-position
reader, three tab placeholders and persisted page-length Settings, system theme/window insets.
Native packaging, private-file document persistence and recreation retention are
integrated, including Library current percentage. Page controls/completion remain next.
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
The sandbox hides `/dev/kvm`; the host has usable KVM when running with escalation.
The emulator section below records verified Android execution.

Verified 2026-10-22: debug APK build and strict lint PASS; repeat after the
launcher-resource fix with `--offline` PASS. `aapt dump badging` confirms min SDK
29, target/compile SDK 36 and app ID/version. This initial build did not run
instrumentation; the later emulator check below does. Initial lint rejected the
template's unnecessary `mipmap-anydpi-v26` folder; moving its icons to
`mipmap-anydpi` fixed it without suppression.
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
Kotlin bridge tests yet. Check the new Unicode fixture test separately:
```sh
java -jar .local/ktfmt-0.64.jar --kotlinlang-style --dry-run --set-exit-if-changed tests/kotlin/TextBoundaryTest.kt
```
Android lint runs through the Gradle command above.

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
Compose view. An Activity-scoped `CoreSessionOwner` retains its native handle
across configuration changes and closes it when the Activity finishes.
`YomokuApp` uses bottom navigation below 600dp available width and a rail at
600dp or wider. Settings is a Rust-owned toolbar flow with toolbar/system Back.
Instrumentation sources are `CoreSessionTest.kt` and `NavigationTest.kt` under
`android/app/src/androidTest/java/app/yomoku/`; the test APK is
`android/app/build/outputs/apk/androidTest/debug/app-debug-androidTest.apk`.
Run on a connected API 29+ arm64 device or x86_64 emulator:
```sh
"$ANDROID_HOME/platform-tools/adb" devices -l
./android/gradlew -p android :app:connectedDebugAndroidTest --offline --console=plain
./android/gradlew -p android :app:installDebug --offline --console=plain
"$ANDROID_HOME/platform-tools/adb" shell am start -n app.yomoku/.MainActivity
```
Verified 2026-10-22: `CoreSessionTest` passes on the configured API 36 x86_64
`medium_phone` AVD. The HTML report is
`android/app/build/reports/androidTests/connected/debug/index.html`.
Initial offline execution lacked AGP's UTP host-test dependencies; an online run
populated them, then the offline test passed. This does not verify arm64 device
execution. Four tests now also verify async document storage on Android.
Debug APK installation and `am start -W -n app.yomoku/.MainActivity` also pass;
the captured screen displays the Rust-backed Library heading. The ignored
screenshot is `.local/emulator-captures/yomoku-library.png`.

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
effects; real storage worker callbacks now verify asynchronous delivery.

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

Production JNI imports now require the explicitly prepared pinned Full dictionary
(see Sudachi setup below). The runner checks
`YOMOKU_SUDACHI_DICTIONARY` or `.local/sudachi/full-20261015/system_full.dic`;
it never downloads data implicitly. Keep those bytes immutable while JNI runs.
Host-only module-opening flags enable JDK descriptor duplication in the fixture.
No Android SDK/Gradle is required.

The script defaults to those local tools/headers, and accepts `BOLTFFI_BIN`,
`KOTLINC_BIN`, `JAVA_BIN`, `CC`, `JAVA_HOME` and `JNI_INCLUDE_DIR` overrides.
Set the Rust PATH workaround above in this sandbox. Once tools and Cargo caches
are populated, the test runs offline. Keep normal Cargo target output at `target/`.
The Java command uses `--enable-native-access=ALL-UNNAMED` and `-Xcheck:jni`.

PASS: all destinations/reselection, event/request/view codecs through actual JNI,
typed InvalidEvent/InvalidResponse exceptions, recovery after errors, independent
instances, repeated close, calls after close and 100 lifecycles releasing their
Kotlin listener registry entries. Listener cleanup inspection depends on the pinned
generator's private registry; review the test when upgrading BoltFFI. Storage tests
now also cover worker-thread callbacks, exact Unicode create/list/load/reopen, typed
failures/retry, and 20 close-during-I/O cycles with eventual listener release.

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

## Unicode payload checks
`tools/test-kotlin-bridge.sh` also runs the test-only `unicode-fixtures` Cargo
example before/after Kotlin to exchange ChapterText, TextAnchor and DocumentId
payloads under ignored `target/unicode-fixtures/`. Seven fixtures cover exact
Unicode text/ranges, supplementary characters, combining/ZWJ sequences, line
breaks, NUL, empty text and full signed 64-bit IDs. Kotlin independently derives
byte ranges from UTF-16 substring indices and sends suffix anchors back to Rust;
Rust validates those and rejects invalid Kotlin-encoded IDs/offsets. The codecs
are the production-generated Crux Bincode codecs. This fixture exchange does not
exercise live commands themselves; those are now covered by the async storage tests.

Verified 2026-10-22: workspace build, 32 Rust tests, fmt/clippy, host Kotlin/JNI
including the seven Unicode fixture exchanges, Kotlin formatting and shell syntax
PASS. Android app/test APK assembly and strict lint PASS offline with the expanded
generated payload schema. No Android instrumentation rerun for this pure-domain/
codec slice; the previous three device tests remain the navigation baseline.

## SQLite storage checks
`yomoku-storage` uses pinned rusqlite with bundled SQLite; no Fedora SQLite
development package is required. A C compiler is required (the NDK supplies it
for Android). `cargo test --workspace --all-features --locked --offline` includes
21 storage tests using private temporary disk databases with automatic cleanup.
Migration 0002 and typed create/load paste commands cover literal first-line titles,
exact Unicode/NUL/CRLF, reopen/anchors, per-insert and deferred-commit rollback,
concurrent writes, committed-ID nonreuse/full-width allocation/exhaustion, and
real migration-failure preservation. Rename/deletion use test-only SQL to verify
schema separation; production commands remain unimplemented. Initialization and
paste storage now run through runtime/FFI and Android's private database. The
Android paste UI is implemented; the complete import pipeline remains pending.
The app now creates filesDir/yomoku.sqlite3 on its Rust worker at session startup.

Compile the storage crate for both Android ABIs with the same pinned NDK/API:
```sh
export PATH=/home/kai/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin:$PATH
export CC_aarch64_linux_android="$ANDROID_HOME/ndk/30.0.16248370/toolchains/llvm/prebuilt/linux-x86_64/bin/aarch64-linux-android29-clang"
export AR_aarch64_linux_android="$ANDROID_HOME/ndk/30.0.16248370/toolchains/llvm/prebuilt/linux-x86_64/bin/llvm-ar"
export CC_x86_64_linux_android="$ANDROID_HOME/ndk/30.0.16248370/toolchains/llvm/prebuilt/linux-x86_64/bin/x86_64-linux-android29-clang"
export AR_x86_64_linux_android="$ANDROID_HOME/ndk/30.0.16248370/toolchains/llvm/prebuilt/linux-x86_64/bin/llvm-ar"
cargo build -p yomoku-storage --target aarch64-linux-android --locked --offline
cargo build -p yomoku-storage --target x86_64-linux-android --locked --offline
```
This compiles the Rust storage library and bundled SQLite; it does not execute
SQLite on Android or package it into the APK before runtime wiring exists.

## Android CLI emulator setup
Installed through Android CLI 1.0.16625370, without Android Studio:
```sh
android emulator create --list-profiles
android emulator create medium_phone
```
Creation automatically downloads the emulator and the profile's system image.
Verified installation: Emulator **37.2.12**, profile/AVD **medium_phone**,
`system-images;android-36;google_apis_playstore;x86_64` **revision 7**.
SDK packages live in `/home/kai/Android/Sdk`; AVD data/configuration lives in
`~/.android/avd/medium_phone.avd/`. None is tracked in Git.

The sandbox does not expose `/dev/kvm`, but the host has it. Run emulator/ADB/CLI
with escalation in this environment; the host acceleration check passes:
```sh
"$ANDROID_HOME/emulator/emulator" -accel-check
```
Do not infer that KVM is absent on the host from a sandbox-only device check.

The default SwiftShader GLES backend crashes this host's emulator RenderThread
(SIGSEGV, confirmed in GDB inside `gles_swiftshader/libGLESv2.so`). ANGLE software
graphics (`swangle`) boots and executes the test. This is a host graphics issue,
identified before Yomoku launches. The verified foreground headless launch is:
```sh
"$ANDROID_HOME/emulator/emulator" -avd medium_phone -no-window -no-audio -no-snapshot -gpu swangle -camera-back none -camera-front none -accel on -cores 2
```
`tools/start-android-emulator.sh` runs these flags (optional first argument: AVD
name). It stays in the foreground; Ctrl-C stops it. `ANDROID_HOME` or
`ANDROID_SDK_ROOT` selects the SDK, defaulting to `$HOME/Android/Sdk`.
The AVD config saves `hw.gpu.mode=swangle`, `hw.cpu.ncore=2`,
`hw.camera.back=none`, `hw.camera.front=none`, `hw.audioInput=no` and
`hw.audioOutput=no`. No AVD data reset is needed. A cold restart through
`android emulator start medium_phone --headless --cold` still exited early on
this host despite these settings; use the explicit launcher above. Android CLI
still manages package installation, AVD creation/listing and stopping.

First boot may show **Allow USB debugging?**. Accept **Always allow from this
computer** for the development host before running tests. This session used the
emulator's authenticated local control API/installed `emulator_controller.proto`
to navigate the prompt headlessly; old console touch events had no effect.
Temporary Python gRPC tooling/screenshots stay in ignored `.local/`; control
tokens, ADB keys and signing keys are never committed. A windowed first boot can
accept the same prompt manually.
The checked-in launcher was cold-booted after authorization; ADB remained
authorized and `sys.boot_completed` returned `1` before the app launch check.

After boot, run `:app:connectedDebugAndroidTest` using the environment above.
The first execution needs network access for AGP's UTP dependencies; subsequent
offline execution passes. The one test checks Android JNI/core render state,
session isolation and native-handle closure. Stop the AVD with:
```sh
android emulator stop emulator-5554
```

## Responsive navigation verification

The three instrumentation tests pass on API 36 x86_64 medium_phone at its default
1080x2400 / 420dpi (411dp width), 1600x2560 / 240dpi tablet portrait and
2560x1600 / 240dpi landscape.
Tests assert the bottom bar versus rail, selected tabs, Settings/toolbar/system
Back, the identical CoreSession after ActivityScenario.recreate(), and a closed
native handle after finishing. The layout adapts to available width; no tablet
hardware flag or Android navigation library owns domain state.

Repeat viewport checks with the same connected-test command above. These are
emulator-only display overrides, not changes to the AVD configuration:
```sh
"$ANDROID_HOME/platform-tools/adb" shell wm size 1600x2560
"$ANDROID_HOME/platform-tools/adb" shell wm density 240
./android/gradlew -p android :app:connectedDebugAndroidTest --offline --console=plain
"$ANDROID_HOME/platform-tools/adb" shell wm size 2560x1600
./android/gradlew -p android :app:connectedDebugAndroidTest --offline --console=plain
# Always restore the original viewport afterward:
"$ANDROID_HOME/platform-tools/adb" shell wm size reset
"$ANDROID_HOME/platform-tools/adb" shell wm density reset
```
The first build needed online resolution of BOM-managed Compose ui-test-junit4
1.10.6 and its transitive dependencies. Subsequent APK assembly, tests and strict
lint run offline. Generated Rust unit event variants are Kotlin data objects
(OpenSettings / CloseSettings), not constructor calls. System Back test injection
must await Compose idle after clicking Settings so BackHandler is registered.
UTP removes the app after testing; installDebug (or adb install -r the debug APK)
is required for a later manual launch/screenshot.


## Async document storage verification
NativeBridge now requires a database path before its listener. Host isolation
checks use :memory:; disk tests use temporary paths. Android's retained owner
supplies Application.filesDir/yomoku.sqlite3, and CoreSession requests Library
once at construction. Rust opens/migrates and performs all SQL on its FIFO worker.
No database/path/SQL handle is exposed to core or Compose. Closing the session
allows submitted I/O to finish and drops late results; it does not join on UI.

PASS: 49 Rust tests, real async Kotlin/JNI callbacks/reopen/errors, Android app
and test assembly, strict lint and all four API 36 x86_64 instrumentation tests.
DocumentSessionTest uses temporary private files, verifies exact Unicode and
first-line titles across new sessions, and retries a missing-directory failure.
A host SQLite lock fixture verifies write waiting does not block dispatch or
navigation. Arm64 builds pass; arm64 execution remains untested.

Crux 0.20.0 middleware prints `Inner can't be upgraded after resolving effect`
when a deliberately closed session discards a worker result. This is upstream
weak-reference shutdown logging; JNI checks and listener-release tests still pass.
Initial Kotlin error enum assertions used underscores; generated names are
EMPTYPASTE/NOTFOUND. Corrected and rerun successfully. Clippy's test type-complexity
finding was resolved with a callback receiver alias. No outstanding check failure.

## Paste and first-page reader verification
PASS 2026-10-22: 67 Rust tests, workspace build, fmt/clippy, host Kotlin/JNI
reader payloads/storage/reopen/lifecycle and seven Unicode fixture exchanges,
Kotlin formatting, Android app/test APK assembly and strict lint. No dependency
or migration was added. Commands are the same as above with --locked for Cargo.

All five instrumentation tests PASS on API 36 x86_64 medium_phone at default
1080x2400/420dpi and tablet landscape 2560x1600/240dpi. Use the connected-test
command above after applying the documented wm size/density overrides, then
restore both. PasteReaderTest exercises the actual paste form, disabled empty
submission, draft retention through Settings/recreation, typed failure/retry,
return to Library without auto-opening, exact first logical page/counter, hidden
main navigation, retained reader on recreation and toolbar/system Back/reopen.
Private-file storage tests continue to cover fresh-session reopen persistence.
The emulator was restarted using the documented swangle flags and its phone
viewport restored after testing. Arm64 native packaging builds; execution remains
untested. Initial new Rust test API assumptions and the FFI test's private-event
ordinal were corrected before the complete passing rerun. Intentional-close Crux
logs and ktfmt/JDK warnings remain benign. Page controls, saved current position,
Library percentage, completion/history and reader Settings remain future slices.

## Durable current-position verification
PASS 2026-10-23: 75 Rust tests, workspace build, fmt/clippy, host Kotlin/JNI with
exact position saves/reopen/typed invalid-offset failures, seven Unicode exchanges,
Kotlin formatting, Android app/test APK assembly and strict lint. Existing locked/
offline commands above are unchanged. Migration 0003 is appended; versions of
dependencies and toolchains are unchanged.

All six instrumentation tests PASS on API 36 x86_64 at the default phone viewport
and tablet landscape 2560x1600/240dpi. Size/density overrides were restored.
DocumentSessionTest
also saves/loads an exact position across fresh native sessions. PasteReaderTest
adds position-only hook coverage across three logical pages: current percentage
in both directions, exact offset within a page on reopen, top-of-page reset on
page change/reopen/recreation, and long scrollable content. The UI still has no
Next/Previous/swipes; forward completion/history is separate work. The first host
JNI fixture used byte 4 inside a Japanese title character; corrected to boundary
5, with byte 3 as the intentionally invalid fixture, and the full rerun passed.

## Persisted page-length Settings verification
PASS 2026-10-23: 81 Rust tests (30 storage), locked/offline workspace build,
fmt/clippy, real host Kotlin/JNI (all five settings presets/reopen plus Unicode,
errors and lifecycle), seven Unicode fixture exchanges, Kotlin formatting and
whitespace. Migration 0004 upgrades version 3 without changing text/positions;
write/commit rollback, corrupt/missing settings and all 25 app reflow pairs pass.
No dependency/toolchain versions changed. Existing commands above apply.

App/test APK assembly and strict lint PASS. All seven API 36 x86_64 tests PASS
at phone 1080x2400/420dpi and tablet landscape 2560x1600/240dpi; overrides restored.
PasteReaderTest now selects presets through Settings, checks exact offset and
Library percentage preservation, expected page text/counter, recreation, reopen
and scroll reset. DocumentSessionTest verifies the global preset across new
native sessions. Arm64 builds pass; arm64 execution remains untested.
Initial lint flagged the numeric character label as PluralsCandidate; changed to
a plural resource and reran successfully. A server restart interrupted the first
build observation; inspected reports/state and resumed. Emulator was restarted
with the documented swangle/KVM command. No remaining failures or blockers.

## Completion repository foundation verification
PASS 2026-10-23: 91 Rust tests (40 storage), locked/offline workspace build,
fmt/clippy, host Kotlin/JNI, seven Unicode exchanges, Kotlin formatting and
whitespace. The existing commands above are unchanged. Ten new repository tests
cover completion duplicate/reopen/concurrency, reread versus Previous, event/
position/commit rollback and retry, stale/conflicting/invalid inputs, Unicode EOF,
version-4 upgrade/migration failure, historical provenance and ID exhaustion.

App/test APK assembly and strict lint PASS with both native ABIs. All seven
existing Android tests PASS on API 36 x86_64 medium_phone at 1080x2400/420dpi;
viewport unchanged. Tablet tests were not rerun for this repository-only slice;
arm64 execution remains untested. New completion operations have no app/FFI entry
point, so their behavior is verified on host; device tests cover existing flows
with migration 0005. Initial clippy vec_init_then_push finding in the new fixture
was fixed with vec! and rerun successfully. Intentional-close Crux and ktfmt/JDK
warnings remain benign. No outstanding failure or blocker.

## Vocabulary repository foundation verification
PASS 2026-10-23: 102 Rust tests (51 storage), locked/offline workspace build,
fmt/clippy, host Kotlin/JNI, seven Unicode exchanges, Kotlin formatting and
whitespace. Existing commands above apply. Migration 0006 and 11 new storage tests
cover exact global surface identity, inflections/Unicode/NUL, all five statuses,
dated registration/manual history, no-op lookup/selection, concurrency, reopen,
statement/deferred-commit rollback/retry, version-5 upgrade/failure preservation
and document/vocabulary deletion schema independence. No dependencies changed.

App/test APK assembly and strict lint PASS with both native ABIs. All seven
existing Android tests PASS on API 36 x86_64 medium_phone; the emulator was left
running and no viewport overrides were made. No tablet rerun for this repository
slice; arm64 execution remains untested. New vocabulary methods are repository-only
and tested on host; existing device flows exercise migration 0006. Analyzer/import,
completion New→Known/occurrences/time and vocabulary UI remain unimplemented.
Initial migration test referenced a nonexistent pagination function; corrected to
LogicalPages and the full rerun passed. Existing intentional-close Crux logging
and ktfmt/JDK warnings remain benign. No outstanding failure or blocker.

## Analysis anchor and reversible override verification
PASS 2026-10-23: 117 Rust tests (61 storage), locked/offline workspace build,
fmt/clippy, host Kotlin/JNI, seven Unicode exchanges, Kotlin formatting and
whitespace. Commands above remain unchanged. Migration 0007 and fifteen new core/
storage tests verify exact original Unicode/UTF-8 anchors, versioned immutable
A/B/C snapshots, separate reversible global/local split/merge partitions,
concurrent retries, reopen, corruption rejection and statement/commit rollback.
Version-6 upgrade does not rescan existing documents; a failed migration preserves
the prior schema/content. Deleting canonical content removes live analyses but
retains global override provenance/reversibility. These use synthetic adapter
output; Sudachi segmentation/performance and actual rule application are untested.

App/test APK assembly and strict lint PASS with both native ABIs. No dependency
or exported payload was changed. The first workspace run found an old migration
test still expecting schema version 6; fixed to 7 and the complete rerun passed.
All seven existing Android tests PASS on API 36 x86_64 medium_phone at its standard
1080x2400/420dpi viewport. Initial connected-test execution found no device; started
the documented swangle/KVM emulator, confirmed boot and reran successfully. Emulator
left running; viewport unchanged. No tablet rerun; arm64 execution remains untested.
New repository methods are host-tested and have no UI/FFI exposure; existing device
flows exercise migration 0007. Intentional-close Crux and ktfmt/JDK warnings remain
benign. No outstanding failure or blocker.


## Pinned Sudachi Full adapter (host)
Verified 2026-10-24: Sudachi.rs v0.7.0 at
`5de0410b6edfd428fc510cc9763440772e9619d5`, Full 20261015 V1.
Exact artifact/checksums/notices: third_party/sudachi/README.md. Python 3.11+ and
standard library only; no dictionary download is triggered by a Cargo build.
```sh
python3 tools/prepare-sudachi.py
# Offline preparation from the official pinned ZIP:
python3 tools/prepare-sudachi.py --archive .local/sudachi/sudachi-dictionary-20261015-full.zip
tools/test-sudachi.sh
# Optional local data path; its dictionary checksum must still match:
YOMOKU_SUDACHI_DICTIONARY=/path/to/system_full.dic tools/test-sudachi.sh
```
PASS: all eleven actual Full-data tests per owned/mapped loader (A/B/C, inflections, proper nouns,
normalization, Unicode/OOV, 72 KB input, SQLite snapshots/reopen and atomic
analyzed-paste registration). Ordinary
workspace tests explicitly report these eleven as ignored; run the script to
exercise them. Prepared dictionary size is 330,506,339 bytes (about 315 MiB).
Keep the dictionary immutable until the mapped test process exits; do not run
preparation/replacement concurrently. The parity fixture also compares complete
outputs across storage modes after closing the original mapped file handle.
Preparation/repeated verification, corrupt-archive rejection without replacement,
Python syntax and shell syntax pass. Data remains ignored; complete notices are
retained both in Git and beside prepared data.

PASS: workspace locked/offline build and 119 data-free tests, fmt/clippy, host
Kotlin/JNI including seven Unicode exchanges, and Kotlin formatting. Existing
Android app/test APKs and strict lint also pass with both native ABIs; no Android
instrumentation rerun for this host-only slice. No app or bridge entry point uses
the analyzer yet. Existing intentional-close/JDK formatter warnings remain benign.

Previously FAILED (unpatched upstream, resolved by the source patch below):
```sh
cargo check -p yomoku-analysis --lib --target aarch64-linux-android --target x86_64-linux-android --locked --offline
```
Sudachi src/plugin/loader.rs:92, E0425: make_system_specific_name is undefined for
Android. Its Linux/FreeBSD .so cfg omits Android. Resolve with a pinned upstream
fix or documented source patch before app linking; do not fake target cfgs or
modify the shared Cargo cache. Device startup/memory/offline measurements and
bundling remain unchecked; host constructor uses verified owned dictionary bytes.

## Sudachi Android compile unblock

Verified 2026-10-23: root Cargo source patch selects a dependency-only vendored
v0.7.0 snapshot at the same upstream commit. The complete patch/original file
hashes and licenses are under third_party/sudachi. Only the loader's Android cfg
and dependency-only manifests differ; no dependency versions changed. The engine
profile now includes +android-loader-v1. The source verifier requires Python
3.11+ and Git, uses a temporary copy, and works offline:

```sh
python3 tools/verify-sudachi-source.py
cargo check -p yomoku-analysis --lib --target aarch64-linux-android --target x86_64-linux-android --locked --offline
cargo build -p yomoku-analysis --lib --target aarch64-linux-android --target x86_64-linux-android --locked --offline
```

Both target checks and library builds PASS, using installed Rust Android targets;
these library-only commands do not link an APK or need the NDK C compiler.
Source verification PASS for all 138 files after reversing the complete patch;
isolated altered/missing/extra source and divergent-patch checks correctly fail.
Workspace build, 119 data-free tests, nine real Full fixtures, fmt/clippy and
host Kotlin/JNI including seven Unicode exchanges PASS. Python/shell syntax and
Kotlin formatting PASS. Do not run upstream test targets in the dependency-only
snapshot; their external fixtures/dev dependencies are intentionally omitted.

No Android APK, instrumentation or device measurement rerun for this compile-only
slice. The analyzer remains outside runtime/FFI dependencies. Bundling Full with
app-accessible notices and measuring initialization/memory/tokenization/offline
operation remain unchecked, including whether owned dictionary storage is viable.

## Android Sudachi Full assets and notices

Android builds now require the already pinned/prepared Full data. Prepare once
explicitly (or use the offline --archive command above); Gradle never downloads
it. The existing APK assembly/lint commands then run bundleSudachi, verifying
all three files before staging generated assets. Gradle tracks the source
directory/script and skips unchanged bundles. To stage/verify manually:

```sh
python3 tools/prepare-sudachi.py --android-assets
```

Settings → Sudachi licenses and notices shows the complete LEGAL and Apache-2.0
license, with selectable/scrollable text loaded off the main thread. Both files
and the 330,506,339-byte dictionary are packaged as assets, with streaming
checksum tests against the installed APK. The app does not initialize Sudachi
or extract/copy the dictionary at startup. No dependency versions changed.

The debug APK is 184,525,421 bytes; the dictionary's compressed APK entry is
146,067,671 bytes. All three APK SHA-256 values match the pinned inputs; ZIP
16 KB alignment passes. Missing/corrupt bundle inputs fail without replacing
previous outputs; repeated/offline preparation and bundling pass. Python syntax,
Kotlin formatting, app/test APK assembly and strict lint pass, along with 119
data-free Rust tests, fmt/clippy and nine real Full-data fixtures. Host JNI was
not rerun for this packaging/UI slice.

PASS: all nine API 36 x86_64 phone instrumentation tests, plus the two new
SudachiAssetsTest tests at tablet landscape 2560x1600/240dpi. Both runs used
airplane mode. Tests verify all installed bytes, complete displayed notices,
recreation and dismissal. Original 1080x2400/420dpi and airplane mode off were
restored; emulator left running. Copied XML results are ignored under
.local/test-reports/sudachi-bundling/{phone,tablet-landscape}/. Arm64 libraries
build/package; arm64 execution remains untested. The usual connected-test
command runs all nine; add this Gradle property for just the new tests:

```sh
-Pandroid.testInstrumentationRunnerArguments.class=app.yomoku.SudachiAssetsTest
```

This session's initial tablet commands hit the documented sandbox socket
restrictions for ADB/Gradle; rerunning with escalation passed. No outstanding
check failure. No viewport configuration files were changed.

Analyzer measurements are now recorded below; bundled assets/notices alone did
not establish memory viability. The production loading strategy remains gated.

## Standalone Android Sudachi measurements

Use the existing direct Rust PATH and ANDROID_HOME environment. The new driver
verifies all 138 pinned engine files and prepared Full bytes, builds a standalone
native example using NDK r30/API 29, and runs three fresh processes on the selected
device. Cargo stays locked/offline; the app/runtime/FFI remain untouched.

```sh
python3 tools/measure-sudachi-android.py --serial emulator-5554
python3 tools/measure-sudachi-android.py --serial emulator-5554 --profile debug
# For a physical arm64 device, select its serial; target selection is automatic.
```

ADB socket access requires sandbox escalation. The tool temporarily enables
airplane mode and disables Wi-Fi/mobile data, checks those values, and restores
and verifies the original radio settings in finally. It removes only its unique
device directory. Raw output, device metadata and per-stage RSS/HWM/anonymous/
file/swap fields go to `.local/test-reports/sudachi-android/`. Partial failed
reports are retained; probe errors are never treated as successful measurements.
Unknown initial radio values cause failure before changing the device.

PASS on API 36 x86_64: three release and three debug offline processes, A/B/C
golden results, mixed-script/OOV/normalization/Unicode anchors and 72 KB unbroken
input. Both Android probe ABIs link; physical arm64 execution remains untested.
Full methodology, exact results and limitations are in
[SUDACHI_MEASUREMENTS.md](SUDACHI_MEASUREMENTS.md). Release initialization was
1.03–1.20 s versus debug 12.31–12.69 s; approximately 318 MiB anonymous memory
motivates a measured mapped-storage comparison before app integration. No APK
decompression/JVM/Compose/lifecycle performance claim. No APK/instrumentation,
Android lint or host JNI rerun for this standalone Rust/tooling slice.

## Verified mapped Full storage comparison

The safe owned loader remains the baseline. `Analyzer::open_full_mapped(&File)`
is explicitly unsafe: callers must keep backing bytes immutable until analyzer
drop. It hashes the actual mapping and passes the same storage to Sudachi; a
checksum/read-only descriptor alone cannot enforce the lifetime contract.
The direct memmap2 dependency pins the already locked 0.9.12; no graph versions
or engine/analysis profile identities change. App/runtime/FFI stay independent.

```sh
python3 tools/measure-sudachi-android.py --serial emulator-5554 --storage both
python3 tools/measure-sudachi-android.py --serial emulator-5554 --storage both --profile debug
```

`--storage owned` remains the default; `mapped` selects only that loader and
`both` alternates fresh owned/mapped processes for each of three runs. The driver
owns a unique private directory, makes the published file read-only, and never
rewrites/removes it during any probe. Never modify prepared files during mapped
host tests or manual probe invocations. Radio settings are verified with bounded
polling because Android applies `svc` changes asynchronously, including restoration.

PASS: 119 data-free Rust tests, ten Full fixtures per loader (including complete
A/B/C output parity/closed file handle), pinned source integrity, workspace build,
fmt, strict all-target/all-feature Clippy, Python/shell syntax and whitespace.
Both Android release probe ABIs link; all twelve offline API 36 x86_64 processes
pass (three owned/mapped pairs per debug/release profile). Reports/results are in
SUDACHI_MEASUREMENTS.md. Initial mobile-data verification raced Android state and
stopped before analysis; bounded polling fixed it, and both complete reruns pass.
Temporary device files removed and original radios restored/verified.

Release mapped init 0.932–0.944 s, ~2.5 MiB anonymous RSS versus ~318 MiB owned;
initial total RSS and peak remain similar after verification touches all pages.
No low-memory reclamation/app-viability claim. Physical arm64, private asset
extraction and combined app/lifecycle measurements remain pending. No APK/Compose
instrumentation, Android lint or host Kotlin/JNI rerun for this adapter/tooling
slice; the analyzer remains outside app/runtime/FFI dependencies.

## On-demand private Full extraction

The Android platform helper in `SudachiDictionaryFiles.kt` has no startup or
production analyzer caller yet. A worker can open the pinned Full asset into
noBackupFilesDir/sudachi on demand. It returns a verified read-only descriptor,
not an unchecked path. Always close the handle; do not introduce writers to the
published inode. Repair uses a new inode/atomic rename so held readers/mappings
keep their original bytes. First extraction needs about 315 MiB private disk in
addition to the compressed APK; cache verification still reads the whole file.

Use the existing Android environment above. Build/check both packaged ABIs and
run all tests, or select just the eight new extraction tests:

```sh
./android/gradlew -p android :app:assembleDebug :app:assembleDebugAndroidTest :app:lintDebug --offline --console=plain
./android/gradlew -p android :app:connectedDebugAndroidTest :app:lintDebug --offline --console=plain
./android/gradlew -p android :app:connectedDebugAndroidTest :app:lintDebug -Pandroid.testInstrumentationRunnerArguments.class=app.yomoku.SudachiDictionaryFilesTest --offline --console=plain
tools/format-kotlin.sh --check
git diff --check
```

PASS: app/test APK assembly, strict lint, Kotlin formatting and all seventeen
API-36 x86_64 phone tests. Eight new tests cover lazy/worker-only access,
size/checksum failures/retry, corrupt/writable cache, abandoned staging,
failed rename/cleanup retry, read-only handles, held reader/mapping preservation,
concurrent requests and real APK Full extraction/cache verification. Test data
uses unique private directories and is removed; production user files and device
viewport/radios remain unchanged. Reports: .local/test-reports/sudachi-extraction/.
Timing/method/limits: SUDACHI_MEASUREMENTS.md. Physical arm64, actual power loss,
ENOSPC and cross-process lock contention are untested. App analyzer memory/reuse/
lifecycle remains gated; no runtime/FFI entry point or dependencies changed.

Initial compile used non-public O_DIRECTORY; removed after inspecting the API-36
android.jar and verified directory fsync on device. An interrupted-stream fixture
was tightened to keep failing after the first read exception; all eight final
focused tests and lint pass after restarting a disconnected emulator with swangle.
A sandbox Gradle socket failure was rerun with escalation. No separate Rust tests,
host JNI or standalone probes rerun for this Kotlin/platform slice; native
packaging uses the existing bridge.

## Debug-only in-app Sudachi descriptor/reuse probe

The debug source set bundles a separate release-optimized Rust cdylib example
for both ABIs; release apps exclude its library and Kotlin facade. Gradle builds
it sequentially after the existing bridge, verifies pinned engine source, and
uses NDK r30/API 29 with locked/offline Cargo. No extra setup/dependency is needed
after the existing Full preparation. There is no startup or production caller.

With the usual PATH/JAVA_HOME/GRADLE_USER_HOME/ANDROID_HOME above:

```sh
./android/gradlew -p android :app:assembleDebug :app:assembleDebugAndroidTest :app:lintDebug --offline --console=plain
./android/gradlew -p android :app:connectedDebugAndroidTest :app:lintDebug -Pandroid.testInstrumentationRunnerArguments.class=app.yomoku.SudachiAppProbeTest --offline --console=plain
# Run all reader/platform tests:
./android/gradlew -p android :app:connectedDebugAndroidTest :app:lintDebug --offline --console=plain
# Unsigned release build to check that the probe is excluded:
./android/gradlew -p android :app:assembleRelease --offline --console=plain
```

`SudachiAppProbeTest` extracts real compressed Full into its unique no-backup
directory, supplies the verified descriptor through JNI, closes the Kotlin
handles, and validates A/B/C, OOV/normalization/Unicode/NUL, repeated pages and
72 KB unbroken input on one native worker. It checks failed-load retry, bad/stale/
foreign-worker calls, one initialization across Activity recreation/stop/resume,
close on ViewModel clearance, actual unmapping, and fresh-worker cache reopen.
The test owner/executor is measurement scaffolding, not production integration.
Cleanup waits for native close/thread termination before removing owned files.

Per-stage PSS/private-dirty and kernel RSS/HWM/anonymous/file/swap plus timings
are in Logcat tag `SudachiAppProbe` and Gradle's per-test logcat files under
`android/app/build/outputs/androidTest-results/connected/debug/`. Preserve that
directory before another run overwrites it. Test-owned directories are removed;
device viewport/radios and production dictionary files are untouched. Test timing
includes JNI/executor overhead; the app is a debug Compose/instrumentation build
with release Rust. No cold-storage/pressure/process-death/physical-arm64 claim.
Exact results, local report paths and unresolved gate: SUDACHI_MEASUREMENTS.md.

Verified 2026-10-23: debug app/test APKs, strict lint, the focused test and all
eighteen phone tests pass. Unsigned release APK assembly passes; ZIP/DEX inspection
confirms both probe ABIs/facade are present only in debug. Both probe ELF LOAD
segments and both APKs have 16 KB alignment. Debug APK: 190,468,605 bytes;
unsigned release: 165,940,973 bytes. Pinned source verification, workspace build,
119 data-free Rust tests, fmt/strict Clippy, Python syntax, Kotlin formatting and
whitespace pass. No host JNI, host Full fixtures, standalone probe or tablet rerun.
Final reports: .local/test-reports/sudachi-app/{focused-1,full-suite}/.
First connected Gradle command needed socket escalation; the initial map assertion
used an absolute Context path while /proc can use another private-directory alias.
The unique test-directory suffix check passes in both final processes. No open
check failure; actual pressure/process-death/physical-arm64 gates remain.

## App Full process-death recovery

Build the current debug APK with the usual Android environment above, then:

```sh
/home/kai/Android/Sdk/platform-tools/adb -s emulator-5554 install -r android/app/build/outputs/apk/debug/app-debug.apk
python3 tools/test-sudachi-process-death.py --serial emulator-5554
```

The driver requires an installed debug APK; it never builds/downloads/installs
implicitly. Connected instrumentation uninstalls the app, so reinstall debug before
running the driver afterward; preflight rejects a missing/non-debuggable package.
It force-stops app.yomoku (retaining user data), launches MainActivity
and an ADB-only debug Activity in the same process, waits for real Full validation
with a live mapping, kills the process, and validates cached recovery in a new
PID. Two pairs run by default (`--runs N`). The debug manifest protects the
exported probe with the system DUMP permission. It accepts fixed phases and a
32-hex run token only. Neither the Activity nor its manifest entry is in release.

Each run owns no_backup/process-probe-<token>. JSON reports include PID, actual
verified-FD inode/mtime/size/permissions, timing and PSS; normal onDestroy would
write a native-close marker, which must be absent after force-stop. Every process
must exit before that directory is removed. A failed check retains the host
report; cleanup still confirms exit first. No app data clear, radio/viewport
changes, memory-pressure simulation or filesystem-cache flushing. The driver
finishes by opening ordinary Library. Do not run it alongside instrumentation.
ADB sockets can require escalation in the sandbox; command failures include
captured stdout/stderr.

PASS 2026-10-23: two API-36 x86_64 death/recovery pairs (four real Full processes),
all eighteen Android tests, strict lint, both-ABI debug/test and unsigned release
APK assembly, Kotlin formatting, Python syntax and whitespace. APK manifest/DEX/
ZIP inspection confirms the probe Activity/facade/library are debug-only. Reports:
.local/test-reports/sudachi-process-death/20261023T194512308957Z.json and full-suite/.
Initial driver command hit sandbox ADB restrictions; escalation passed. A later
rerun found instrumentation had uninstalled the app; explicit reinstall resolves
it, and the new preflight reports that prerequisite before any process/file work.
Final driver repeats both pairs successfully; report 20261023T194946330811Z.json
in the same directory. Rust/host JNI/host Full fixtures/standalone probes/tablet
were not rerun for this debug-only
Kotlin/driver slice. Pressure/LMKD, mid-extraction death, power loss and physical
arm64 remain unverified; production loading/lifecycle remains gated.


## Atomic analyzed-paste repository

`Storage::create_analyzed_paste` composes canonical text, a validated Mode B
snapshot and all unique Japanese registrations in one immediate transaction.
Preload/reuse the analyzer before the callback, which holds the write lock and
must not access SQLite or expose provisional IDs. The repository has no Sudachi
dependency; the real adapter composition lives in an explicit Full-data fixture.
Production runtime/FFI/Android import wiring remains behind the loading gate;
no app behavior, schema or dependency version changed in this slice.

Verified 2026-10-24 with the Rust PATH workaround above:
```sh
cargo build --workspace --locked --offline
cargo test --workspace --all-features --locked --offline
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked --offline -- -D warnings
tools/test-sudachi.sh
tools/test-kotlin-bridge.sh
git diff --check
```
PASS: 125 data-free Rust tests, including 67 storage tests; eleven real Full-data
fixtures with each owned/mapped loader; 138-file source integrity; host Kotlin/JNI
and seven Rust/Kotlin Unicode exchanges. Six new storage tests cover exact
Unicode/repetition/non-Japanese exclusion, all existing statuses, concurrent
imports, analyzer/validation failure, every write stage, deferred commit failure,
retry and reopen. No completion/encounter/position is created by import.

The initial new Full fixture assumed a half-width compound was a single B token;
corrected it to validate original source spans and Japanese classification across
the actual token boundaries. Both complete loader suites pass. JNI retains its
previous nonfatal weak-reference shutdown logging; all assertions pass. No open
failure. No Android build/lint/instrumentation, emulator changes or standalone
probe rerun for this Rust repository-only change; prior device results stand.

## Exact-byte paste deduplication repository

Verified 2026-10-24 with the same workspace/build/test/fmt/Clippy, Full-data,
Kotlin/JNI and whitespace commands immediately above. PASS: 134 data-free Rust
tests (76 storage), eleven real Full fixtures with each loader, 138-file source
integrity and seven Rust/Kotlin Unicode exchanges. Nine new storage tests cover
exact SHA-256 byte identity, preserved state on duplicate/reopen, concurrent
create-once, legacy duplicate preservation/lazy indexing, every write/commit
failure, typed analyzer failure, corrupt hash rejection and fresh reimport after
test-only deletion with retained history/vocabulary. The real import fixture
also checks duplicate reference after reopen without invoking the analyzer.

Migration 0008 creates the hash index without scanning content on open.
`import_analyzed_paste` is repository-only; production integration remains behind
the Full gate. Direct sha2 0.11.1 reuses the existing lockfile package/version.
All checks pass; prior nonfatal JNI weak-reference shutdown logging persists.
No Android/Gradle/device/standalone/tablet rerun or emulator/user-file changes.

## Bounded emulator Full reclamation probe

Build/install the debug APK as in the process-death section, then run:

```sh
python3 tools/test-sudachi-memory-pressure.py --serial emulator-5554
# Optional smaller allocation ceiling, in multiples of 32 MiB:
python3 tools/test-sudachi-memory-pressure.py --serial emulator-5554 --max-mib 1024
```

The driver rejects physical devices, non-debuggable/missing app installs and
bounds over 70% of guest RAM. It compiles only `tools/android-memory-pressure.c`
with pinned NDK r30/API 29 and `-Wall -Wextra -Werror`; no Rust build/download or
APK install is implicit. Never run concurrently with instrumentation/native builds.
It force-stops the app without clearing data, launches Compose plus the debug
pressure phase, and leaves verified Full mapped while a separate shell process
allocates bounded incompressible anonymous memory. Every 128 MiB it checks actual
dictionary RSS from smaps and kernel reclaim counters. Allocation stops after
>=64 MiB dictionary reclaim or at the ceiling (default 1536 MiB). Reaching the
ceiling without reclaim fails the gate; it does not silently raise the limit.

Quit/EOF releases the helper's memory; a 180-second device alarm is the fallback.
The original app worker/generation reruns all fixed fixtures after release.
Cleanup confirms app exit before removing only this run's dictionary directory;
it removes the unique helper and opens ordinary Library. No settings/app-data
edits, published-inode writes or explicit filesystem-cache flushing. Reports retain samples, app
status, global meminfo/vmstat, PSI availability, exact cleanup paths and outcome in
`.local/test-reports/sudachi-memory-pressure/`. If ADB disconnects, reconnect and
confirm helper exit/app force-stop before removing those exact paths. Preserve
failed reports; never count partial samples as a pass. ADB can need escalation.
Actual results and loading decision are in SUDACHI_MEASUREMENTS.md/DECISIONS.md.

Verified 2026-10-24: two actual pressure/revalidation runs pass on API-36 x86_64;
strict C compilation, debug/test APK assembly, strict lint, Kotlin formatting,
Python syntax/bound rejection and whitespace pass. Initial full Android suite
passed 17/18 (existing notices dialog timeout); focused notices 2/2 and final full
18/18 pass without changing tests/timeouts. Preserved Android reports:
`.local/test-reports/sudachi-pressure/{full-suite-initial,notices-focused,full-suite-final}/`.
The first pressure attempt lost ADB transport and required manual cleanup after
confirmed process exit; final successful reports/limits are in the measurements.
Gradle/ADB sockets needed escalation for the new/focused drivers. No Rust source,
schema or dependency changes; host Rust/Full/JNI and unsigned release were not
rerun for this debug-only slice. No tablet/physical-arm64 execution.
Both process-death/recovery regression pairs also pass with the updated Activity:
`.local/test-reports/sudachi-process-death/20261024T024602925933Z.json`.
The unprivileged driver's preflight masked a socket failure as a missing debug
install; the already installed app passes immediately with ADB escalation.

## Analyzed-import worker integration

The opt-in `effect_router_with_analyzed_imports` runs a lazy analyzer factory on
its existing FIFO SQLite worker. `PasteAnalyzer` need not be Send; platform asset
verification/loading must finish before the repository's transaction callback.
`AnalyzerLifecycle::set_foreground` queues cache policy and holds only a weak
worker reference. NativeBridge/Android still use the previous text-only router;
verified-descriptor delivery, platform signals and import summaries remain next.

Verified 2026-10-24 using the Rust PATH workaround:
```sh
cargo build --workspace --locked --offline
cargo test --workspace --all-features --locked --offline
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked --offline -- -D warnings
tools/test-sudachi.sh
tools/test-kotlin-bridge.sh
bash -n tools/test-sudachi.sh
git diff --check
```
PASS: 154 data-free tests, including seven new runtime tests; 11 analysis fixtures
and one new worker fixture with each owned/mapped Full loader; pinned 138-file
source integrity; host Kotlin/JNI and seven bidirectional Unicode exchanges.
The Full runner now explicitly runs the ignored runtime fixture too. New tests
cover worker-only construction/use/drop with a non-Send analyzer, lazy Library/
settings/blank input, database/load/engine/invalid-output retries, every write and
deferred-commit rollback, unchanged vocabulary/status/progress on duplicate/reopen,
background/hidden release, cache reuse and close during accepted atomic work.
No import creates reading events or encounters. JNI's previous nonfatal
weak-reference shutdown logging persists; assertions pass. No open failure.
No Android build/lint/instrumentation, emulator/probe, tablet or physical-arm64
rerun for this Rust-only step. No new dependency version or schema migration.

## Single-paste import confirmation

Core projects committed Created/Existing results into transient ImportSummary
state; Android shows the current title and Imported/Already in Library with
Close above the list. Both outcomes return to Library without opening the reader.
NativeBridge still selects text-only creation, so production duplicate delivery
awaits the verified-descriptor/analyzer lifecycle integration.

Verified 2026-10-24 with the Rust/Android environments above:
```sh
cargo build --workspace --locked --offline
cargo test --workspace --all-features --locked --offline
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked --offline -- -D warnings
tools/test-kotlin-bridge.sh
tools/format-kotlin.sh --check
./android/gradlew -p android :app:assembleDebug :app:assembleDebugAndroidTest :app:lintDebug --offline --console=plain
./android/gradlew -p android :app:connectedDebugAndroidTest -Pandroid.testInstrumentationRunnerArguments.class=app.yomoku.PasteReaderTest --offline --console=plain
git diff --check
```
PASS: 156 data-free Rust tests, host Kotlin/JNI and seven Unicode exchanges,
workspace build, fmt, strict Clippy, Kotlin formatting, both-ABI debug/test APKs,
strict Android lint and three API-36 x86_64 paste-reader tests. Core/runtime cover
both outcomes, unchanged duplicate progress and transient-state clearing; JNI
checks exact Unicode title delivery and fresh-session absence. The device test
checks visible Created confirmation, explicit dismissal and retained behavior
through Settings/recreation before opening the reader. Reports preserved under
`.local/test-reports/import-summary/paste-reader/`.

The initial workspace run caught a stale hard-coded internal-event tag in an FFI
rejection test; it now derives the tag from the final public event. Kotlin -Werror
caught a redundant safe call after a smart cast; fixed before final passing JNI
run. Previous nonfatal JNI weak-reference shutdown logging persists. Connected
Gradle initially failed on sandbox sockets and passed with escalation. No open
failure. No Full fixtures, full Android suite, release APK, probes, tablet or
physical-arm64 rerun; previous results retain their documented limits. No schema,
dependency version or production analyzer behavior changed.


## Production analyzed paste imports

NativeBridge requires a DictionaryProvider; Android's retained CoreSessionOwner
supplies the verified private publisher. FD+1 transfers an owned descriptor while
zero signals failure, including the generator's default for a failed callback.
The existing FIFO worker maps/rechecks Full before starting the atomic repository
transaction. It caches only while foreground, preserves cache across configuration
recreation, releases on Activity stop and drains accepted work on final close.
No analyzer, SQLite handle or Android I/O enters core; no new executor/schema or
third-party version is introduced. Native builds now verify all 138 pinned Sudachi
files and Gradle tracks the vendored source/verifier as production native inputs.

Verification commands (use the Rust/Android environments above, sequentially):
```sh
cargo build --workspace --locked --offline
cargo test --workspace --all-features --locked --offline
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked --offline -- -D warnings
tools/test-sudachi.sh
tools/test-kotlin-bridge.sh
tools/format-kotlin.sh --check
bash -n tools/build-android-native.sh tools/test-kotlin-bridge.sh
./android/gradlew -p android :app:assembleDebug :app:assembleDebugAndroidTest :app:lintDebug --offline --console=plain
./android/gradlew -p android :app:connectedDebugAndroidTest --offline --console=plain
git diff --check
```

The host JNI runner now exercises real Full failure/retry, exact duplicate summary
and progress preservation, foreground reuse and background release/lazy reopen,
plus eventual cleanup of both callback registries. Data-free Rust tests explicitly
prove production NativeBridge never falls back to unanalysed text on loader failure;
raw Rust bridge storage fixtures remain data-free. Real Full worker and JNI load
waits allow 60 seconds; data-free worker waits remain 10 seconds. Android import
waits allow 30 seconds. These bounded test waits are not performance guarantees.

AnalyzedImportSessionTest checks actual private Full through production JNI,
worker native TID, no eager load, typed provider failure, injected SQL rollback
across content/hash/tokens/vocabulary/history, healthy-cache retry, unique Japanese
registrations, Mode B, no reading events, unchanged duplicate progress/status and
history, hidden work, actual unmapping and accepted import draining after close.
PasteReaderTest reimports through the form and checks visible Already in Library,
retained warm mapping through recreation, background unmapping and lazy resume.


PASS 2026-10-24: workspace build, 157 data-free Rust tests (76 storage), fmt/strict
Clippy, eleven analysis + one worker Full fixture with each owned/mapped loader,
138-file source integrity, host real-Full Kotlin/JNI + seven Unicode exchanges,
Kotlin format/shell syntax/whitespace, both-ABI debug/test APKs, strict lint and
all nineteen API-36 x86_64 Android tests. Final device results:
`.local/test-reports/analyzed-import/full-suite-final/`;
focused notices results: `notices-main-thread/` in the same parent.

Initial full runs passed 18/19; the existing notices test also failed in isolation.
Temporary diagnostics caught its tagged Loading state after ten seconds. Tracing
showed the producer could resume on the I/O thread under the test effect context.
Publish the loaded state on Dispatchers.Main.immediate; the original content,
recreation assertions and ten-second timeout remain unchanged. Focused 2/2 and
final full 19/19 pass. Earlier reports/traces remain under the same report parent;
temporary logging/test diagnostics were removed. No open check failure.

Initial real Full worker/JNI waits also exceeded their old ten-second allowance.
Only these real-data load waits now allow sixty seconds; data-free worker waits
remain ten. Android storage waits allow thirty seconds. JNI Thread objects can
change across detach/attach, so actual worker affinity uses native TID on device.
The emulator disconnected before a diagnostic run; restarted with the documented
swangle flags, retaining data. Gradle/ADB escalation was used. Previous nonfatal
Crux shutdown logging persists. No unsigned release, standalone/pressure/death
probe, tablet or physical-arm64 rerun; prior measurements retain their limits.


## Durable pending reading time

Verified 2026-10-24 with the Rust PATH workaround above, sequentially:
```sh
cargo build --workspace --locked --offline
cargo test --workspace --all-features --locked --offline
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked --offline -- -D warnings
tools/test-kotlin-bridge.sh
git diff --check
```
PASS: 169 data-free Rust tests, including 88 storage tests and twelve new durable
time tests, workspace build, formatting, strict Clippy, host real-Full Kotlin/JNI
and seven bidirectional Unicode exchanges. Existing nonfatal Crux shutdown logs
persist in JNI. No open failure or dependency version change.

Migration 0009 stores only durations at exact UTF-8 positions with checkpoint
revision tokens. Completion validates a frozen page total, writes active_time_ms,
consumes that page's buckets and advances position atomically. Tests cover all
25 preset reflow pairs, backward movement, SQLite reopen with a fresh clock epoch,
concurrent/duplicate/stale delivery, reread protection, EOF, deletion, overflow and
statement/commit/migration rollback. Existing migration/history fixtures now expect
schema 9; the v5 upgrade fixture inserts the historical event shape directly.

No Android source or FFI payload change; timer application/worker/FFI and platform
wiring remain open. No standalone Full fixtures, Android build/lint/instrumentation,
release, probes, tablet or physical-arm64 rerun. Actual Android close/reboot recovery
is not established by the simulated fresh-epoch repository tests.


## Atomic analyzed page completion

Verified 2026-10-24 using the Rust PATH workaround above, sequentially:
```sh
cargo build --workspace --locked --offline
cargo test --workspace --all-features --locked --offline
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked --offline -- -D warnings
tools/test-sudachi.sh
tools/test-kotlin-bridge.sh
git diff --check
```
PASS: workspace build, 182 data-free Rust tests (99 storage), formatting/strict
Clippy, 138-file source integrity, eleven analysis plus one worker Full fixture
with each owned/mapped loader, host real-Full Kotlin/JNI and seven bidirectional
Unicode exchanges. Eleven new storage tests and two pure tests cover exact
Unicode/metadata, repetition, every status, presets/boundaries, rereads, duplicate/
concurrent callbacks, frozen/missing/foreign/corrupt snapshots, deletion, absent
surface registration, every write/deferred-commit failure and migration rollback.
Version-9 upgrades preserve status-event IDs/high-water marks, including empty
history after deletion. The existing real Mode B import fixture now exercises
analyzed completion, retry and reread with the same prepared analyzer output.

Migration 0010 adds historical analysis identities, every Japanese occurrence and
completion-linked New→Known events, atomically with time consumption and position.
Legacy completion data remains explicitly without analyzed history. Only base
snapshots are selected; override resolution and all application/worker/FFI timing/
completion ordering remain open. No new dependency or platform payload changes.

Initial strict Clippy flagged a test tuple's complexity; extracted a column type
alias and the final check passes. No open failure; prior nonfatal Crux JNI shutdown
logging persists. No Android build/lint/instrumentation, release/probes/tablet or
physical-arm64 rerun for this Rust repository slice. Prior device results stand.


## Reader analysis identity restoration

Verified 2026-10-24 using the Rust PATH workaround above, sequentially:
```sh
cargo build --workspace --locked --offline
cargo test --workspace --all-features --locked --offline
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked --offline -- -D warnings
tools/test-kotlin-bridge.sh
tools/format-kotlin.sh --check
PATH=/home/kai/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin:$PATH JAVA_HOME=/home/kai/repos/yomoku/.local/jdk-17/jdk-17.0.21+9 GRADLE_USER_HOME=/home/kai/repos/yomoku/.local/gradle ANDROID_HOME=/home/kai/Android/Sdk ./android/gradlew -p android :app:assembleDebug :app:assembleDebugAndroidTest :app:lintDebug --offline --console=plain
git diff --check
```
PASS: 187 data-free Rust tests (102 storage), workspace build, fmt/strict Clippy,
real-Full Kotlin/JNI with seven Unicode and two analysis-state round trips,
Kotlin format, both debug APK builds and strict Android lint. The read-only
repository snapshot preserves exact text/position/preset and pins the earliest
Mode B identity through reopen, independent of later profiles. Tests cover legacy
Missing, wrong modes/chapters, corrupt metadata/settings, frozen core identity,
route clearing and worker/JNI reopening without dictionary availability/loading.
The codec fixture includes a full-width AnalysisId and the Missing variant.

Initial missing test import fixed; no open failure or dependency change. Existing
benign Crux shutdown logging, ktfmt JVM Unsafe warning and SDK XML-version warning
persist. No Android instrumentation, standalone owned/mapped Full fixture suite,
release APK, probes, tablet or physical-arm64 rerun. Android timer/close/reboot and
completion/navigation remain unchecked. No schema change or legacy rescan added.

## Explicit pending-time checkpoint transport

Verified 2026-10-24 using the Rust PATH workaround and Android environment above,
with native generation/build/test runners executed sequentially:
```sh
cargo build --workspace --locked --offline
cargo test --workspace --all-features --locked --offline
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked --offline -- -D warnings
tools/test-kotlin-bridge.sh
tools/format-kotlin.sh --check
PATH=/home/kai/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin:$PATH JAVA_HOME=/home/kai/repos/yomoku/.local/jdk-17/jdk-17.0.21+9 GRADLE_USER_HOME=/home/kai/repos/yomoku/.local/gradle ANDROID_HOME=/home/kai/Android/Sdk ./android/gradlew -p android :app:assembleDebug :app:assembleDebugAndroidTest :app:lintDebug --offline --console=plain
git diff --check
```
PASS: 191 data-free Rust tests (102 storage), workspace build, fmt/strict Clippy,
real-Full Kotlin/JNI plus seven Unicode/two analysis-state exchanges, Kotlin
formatting, both-ABI debug/test APK builds and strict lint. Four new tests cover
core checkpoint state and asynchronous worker/FFI SQLite persistence. Existing
JNI checks now save/restore distinct offset durations, preserve them through
reflow and reopen with an unavailable dictionary, and reject negative duration
payloads without changing state. Rust numeric decoding also checks full i64 range.

Core freezes absolute checkpoints/prior revisions for exact retries, blocks other
storage operations while unresolved, preserves retries after reader exit and
ignores cancelled/late visible-state callbacks. Tests inject restore/save SQL
failures, check repeated saves do not add time, restore offsets independently
across movement/reopen and confirm no completion/encounter/status history appears.
Read-only reader opening remains covered by the unchanged repository tests.

Initial checks caught the added fields missing from the expected initial view
and the FFI rejection test's outdated final public event; both corrected. Existing
benign Crux JNI shutdown messages and ktfmt JVM Unsafe warnings persist. No open
failure, dependency or migration change. Gradle passed in the normal sandbox.
No instrumentation, standalone Full fixture suite, release APK or execution of
probes/tablet/physical arm64 tests. Automatic timer/lifecycle/periodic/final saves,
actual Android close/reboot recovery and completion sequencing remain open.
