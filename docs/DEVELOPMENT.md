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
with 1.99.1. Current host baseline: 425 data-free Rust tests (188 storage, 43
runtime). Fourteen analysis fixtures and one runtime fixture run separately with
each Full loader via tools/test-sudachi.sh (see below).

## Pure TXT import checks

Verified 2026-10-25, using the Rust PATH workaround above:

```sh
cargo test -p yomoku-import --locked --offline
cargo test --workspace --all-features --locked --offline --quiet
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked --offline -- -D warnings
git diff --check
```

PASS: 13 new TXT tests plus the existing 262 data-free Rust tests, strict Clippy
and formatting. Fixtures cover strict UTF-8/Shift-JIS/Windows-31J, both-valid
ambiguity and explicit choice, leading/interior/unsupported BOMs, invalid/empty
input, exact LF/CRLF/CR and supplementary/decomposed Unicode, Aozora explicit/
implicit kanji ruby and canonical byte ranges, common typography, preserved unknown/
semantic/gaiji/malformed/nested notation, large inputs and all five page presets.
One initial test assumed empty canonical text could be checked before uncertain
decoding; fix it to explicitly select UTF-8. Review added malformed-quote cases
and preserves such content as a unit. All final tests pass. Offline Cargo cache
unpacking needed sandbox escalation; no network download was needed.

At this parser-only checkpoint the crate was not linked into runtime/Android;
current typed TXT integration is recorded below. No Android APK/lint/device,
Kotlin/JNI/generation, full-index scans, standalone Full fixtures, release or
physical-arm64 checks were rerun for this parser-only step. No open blocker.

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
Rust simulates reboot at the duration boundary. Android's retained driver now
produces lifecycle/interaction/periodic/final samples and drains before teardown;
the latest Android verification section records actual close/reboot checks.
Core/FFI checkpoints coalesce during unresolved saves and drain before reader
departure. Analyzed completion now drains checkpoints,
freezes exact retries and reconciles position through core/FIFO/FFI. Android
controls, gestures, completion toast and finished-book behavior remain open.

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

## Checkpoint coalescing and reader departure ordering

Verified 2026-10-24 with the Rust PATH workaround and Android environment above;
native generation/build/test runners executed sequentially:
```sh
cargo build --workspace --locked --offline
cargo test --workspace --all-features --locked --offline
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked --offline -- -D warnings
tools/test-kotlin-bridge.sh
PATH=/home/kai/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin:$PATH JAVA_HOME=/home/kai/repos/yomoku/.local/jdk-17/jdk-17.0.21+9 GRADLE_USER_HOME=/home/kai/repos/yomoku/.local/gradle ANDROID_HOME=/home/kai/Android/Sdk ./android/gradlew -p android :app:assembleDebug :app:assembleDebugAndroidTest :app:lintDebug --offline --console=plain
git diff --check
```
PASS: workspace build, 195 data-free Rust tests (102 storage), fmt/strict Clippy,
host real-Full Kotlin/JNI plus seven Unicode/two analysis exchanges, negative-time
rejection, both debug APK builds and strict Android lint. Three new core tests
cover maximum-duration coalescing, foreign/decreasing/duplicate updates, frozen
retries at both save stages, updates during a chained save, duplicate departures,
position ordering and all exit routes. A blocked-SQLite async FFI test verifies
only two saves per burst, FIFO position/close ordering, exact-offset durations
and revision counts after reopen, and no completion/encounter/status history.

The first FFI run expected revision 2 instead of 3 (reservation starts at 1);
corrected the test expectation. Final workspace checks pass. Prior benign Crux
JNI shutdown logging persists. No dependency/schema/public-codec change. No Kotlin
source changed. No device instrumentation, standalone owned/mapped Full suite,
release APK or probe/tablet/physical-arm64 execution. Automatic timer production,
unrelated busy-operation handling, native teardown flush and analyzed completion
remain unimplemented; actual Android close/reboot recovery is still unverified.

## Analyzed completion intent transport

Verified 2026-10-24 with the Rust PATH workaround and Android environment above;
native generation/build/test runners executed sequentially:
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
PASS: workspace build, 200 data-free Rust tests (103 storage), fmt/strict Clippy,
real-Full Kotlin/JNI plus seven Unicode/two analysis exchanges, Kotlin format,
both-ABI debug/test APK builds and strict Android lint. Three core tests cover
checkpoint-before-completion ordering, duplicate/stale callbacks, frozen preparation/
transaction retries, deferred exit, notices, rereads and explicit MissingAnalysis.
Repository preparation sums multiple saved offsets and rolls back invalid/failed
reservations without history/status/position/time consumption. A blocked-SQLite FFI
test checks nonblocking burst drain, injected completion rollback, one reserved ID,
frozen timestamp/AnalysisId/page total, 250 occurrences then 500 on reread, New-only
promotion and reopened position. JNI completes and rereads with an unavailable
dictionary, proving stored analysis avoids a Sudachi reopen.

Initial compile omissions, Clippy's single-match warning and the Android preview's
missing view fields were fixed before the final passing checks. Gradle initially
failed on sandbox lock-coordination sockets, then passed with escalation. Prior
benign Crux shutdown, ktfmt JVM Unsafe and SDK XML-version warnings persist.
No open failure, schema or external dependency version change. A workspace-only
storage dev-dependency seeds FFI fixtures. Public event/view codecs add completion
intents, pending state and command-keyed notices; Android's exhaustive error mapping
and preview fixture are updated. No device instrumentation, standalone owned/mapped
Full suite, release APK or probe/tablet/arm64 execution. Automatic Android timer,
final-save/native teardown acknowledgment, controls/swipes/toast and finished-book
state remain open; actual Android close/reboot recovery remains unverified.

## Typed reader-clock observations

Verified 2026-10-24 with the Rust PATH workaround and Android environment above,
running native generation/build/test runners sequentially:
```sh
cargo build --workspace --locked --offline
cargo test --workspace --all-features --locked --offline --quiet
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked --offline -- -D warnings
tools/test-kotlin-bridge.sh
cargo run -p yomoku-ffi --bin generate-kotlin --features facet_typegen --locked --offline -- target/generated/kotlin-repeat
diff -r target/generated/kotlin target/generated/kotlin-repeat
tools/format-kotlin.sh --check
./android/gradlew -p android :app:assembleDebug :app:assembleDebugAndroidTest :app:lintDebug --offline --console=plain
git diff --check
```
PASS: workspace build, 206 data-free Rust tests (103 storage), fmt/strict Clippy,
real-Full Kotlin/JNI plus seven Unicode/two analysis exchanges, reproducible Kotlin
generation, Kotlin formatting, both-ABI debug/test APK builds and strict lint.
Five core tests cover restored resume/interact/checkpoint/pause, the idle cap,
invalid/foreign/late samples, delayed callbacks, reflow, new epochs and final-save
drain before departure/completion after unrelated storage success/failure. The new
blocked-SQLite FFI test verifies settings-before-final-save ordering, actual SQLite
reopen with a fresh epoch, injected save failure/exact retry and no reading history.
JNI now generates periodic/final durations through clock samples and restores them
without a dictionary before analyzed completion. Timestamps remain transient.

Initial regression checks caught clearing the newer coalesced slot on frozen-save
retry; fixed. Review also added coverage for superseding an unsubmitted slot and
rejecting late samples behind an already accepted position change. Final checks
pass. Prior benign Crux JNI shutdown logging and ktfmt Unsafe warnings persist.
Gradle passed in the normal sandbox. No schema/dependency change or open failure.
No Android observation producer/lifecycle/native teardown change yet, and no device
instrumentation, standalone Full suite, release APK or probe/tablet/arm64 execution.
Actual Android timer/close/reboot recovery remains unverified.

## Android retained reader clock and draining close

Verified 2026-10-24 with the Rust PATH workaround and Android environment above.
No Rust/schema/dependency/codec changes. Commands (from repository root):
```sh
cargo test --workspace --all-features --locked --offline --quiet
tools/format-kotlin.sh --check
./android/gradlew -p android :app:assembleDebug :app:assembleDebugAndroidTest :app:lintDebug --offline --console=plain
./android/gradlew -p android :app:connectedDebugAndroidTest :app:lintDebug --offline --console=plain
./android/gradlew -p android :app:connectedDebugAndroidTest :app:lintDebug -Pandroid.testInstrumentationRunnerArguments.class=app.yomoku.PasteReaderTest,app.yomoku.ReaderClockSessionTest,app.yomoku.ReaderClockRecoveryTest --offline --console=plain
/home/kai/Android/Sdk/platform-tools/adb -s emulator-5554 install -r android/app/build/outputs/apk/debug/app-debug.apk
/home/kai/Android/Sdk/platform-tools/adb -s emulator-5554 install -r android/app/build/outputs/apk/androidTest/debug/app-debug-androidTest.apk
python3 tools/test-reader-time-recovery.py --serial emulator-5554
git diff --check
```
PASS: 206 data-free Rust tests, debug/test APK builds and strict Android lint.
The full 26-test Android suite passed, then all 11 affected tests passed after
the final same-offset idle and reader/Settings retry changes (27 distinct Android
tests covered overall). Kotlin formatting, Python parsing/help and diff checks pass.
Native generation/builds stayed cached because no Rust/FFI source changed. Android
instrumentation executes the actual packaged JNI, real Full and SQLite.

Five fake-clock/real-Handler driver tests verify periodic checkpoints, five-minute
idle/interactions, duplicate lifecycle signals, settings/reflow, same-offset idle
preservation, movement/new epochs, delayed/failed restore, closing final saves,
frozen failed-checkpoint retries plus newer final slots, and exactly one atomic
completion with its final duration before native release. Partial pages produce no
completion/encounter/status history. Compose covers real Activity pause/resume and
retention through recreation, plus Retry in reader/Settings and disabled presets
while a failed save remains unresolved. The default recovery test closes/reopens
with real elapsedRealtime rather than synthetic timestamps.

The additional host runner installs nothing implicitly, requires an emulator and
reboots it. It uses a unique private instrumentation database and verifies process
IDs, a changed boot ID, exact saved/restored durations and absent reading history;
it removes only that directory after stopping the app. Actual results: PID 6088
saved 259 ms; PID 6129 restored exactly 259 ms and saved 268 ms; after guest reboot,
PID 1531 restored exactly 268 ms. Report:
`.local/test-reports/reader-time-recovery/20261024T153735796878Z.json`.
This verifies durable final saves across a real restart/reboot, without claiming
that an OS kill can flush in-memory time since the last periodic checkpoint.

Initial ADB sandbox socket denial required escalation; the documented swangle AVD
was started without data reset. An initial lock fixture did not block the native
SQLite writer from Android's other SQLite library; replaced that assumption with
final-close assertions and explicit SQL-failure/coalescing retry coverage. Existing
blocked-SQLite host FFI tests also passed in the workspace suite. No open failure.
Emulator left running with both debug APKs installed. No release/tablet/physical
arm64, standalone Full fixture/probe or host Kotlin/JNI rerun. Next: controls,
swipes/notice rendering and finished-book behavior. Nostr/Q188 remain unchanged.

## Android reader controls, swipes and completion toast

Verified 2026-10-24 with the Rust PATH workaround and Android environment above.
Run generation/native builds sequentially. Commands from repository root:
```sh
cargo test --workspace --all-features --locked --offline --quiet
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked --offline -- -D warnings
tools/test-kotlin-bridge.sh
tools/format-kotlin.sh --check
./android/gradlew -p android :app:connectedDebugAndroidTest :app:lintDebug --offline --console=plain
git diff --check
```
PASS: 208 data-free Rust tests, strict Clippy/fmt, real-Full host Kotlin/JNI with
Unicode/analysis exchanges, Kotlin format, both-ABI debug/test APK builds, strict
Android lint and all 30 Android tests on API 36 x86_64 medium_phone (321 seconds).
Connected tests build/package both APKs and execute the actual generated JNI.
Report: `android/app/build/reports/androidTests/connected/debug/index.html`.

Two new core tests verify anchored Previous, first-page/foreign/stale/Settings
rejection, position-only persistence, duplicate delivery and final-time draining.
Three new Compose/JNI/SQLite tests verify bottom controls/counter, both gesture
directions without reaching bottom, scroll reset on navigation/reopen/recreation,
bidirectional Library progress, short/vertical gesture rejection, duplicate rendered
callbacks counted once and deliberate rereads counted again. Injected SQLite failure
keeps controls disabled and Retry commits exactly once. The toast presenter receives
one positive message, consumes its notice before recreation and stays silent at zero.
Existing real-clock final-time completion/draining-close tests also pass.

Gradle's sandbox wildcard-IP/socket failure required escalation. Initial Kotlin
smart-cast/non-null warnings and a missing test gesture import were corrected;
the final strict build/suite passes. Prior benign Crux/ktfmt warnings persist.
No schema/dependency change; generated Event adds anchored PreviousReaderPage.
No release, tablet, physical-arm64 or separate process/reboot recovery rerun.
No open failure or local blocker. Next: finished-book persistence/Finish Book/confetti.

## Repository permanent document finish state

Verified 2026-10-24 with the Rust PATH workaround and Android environment above.
Native generation/build/test runners executed sequentially:
```sh
cargo test --workspace --all-features --locked --offline --quiet
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked --offline -- -D warnings
tools/test-kotlin-bridge.sh
tools/format-kotlin.sh --check
./android/gradlew -p android :app:connectedDebugAndroidTest :app:lintDebug -Pandroid.testInstrumentationRunnerArguments.class=app.yomoku.ReaderNavigationTest,app.yomoku.ReaderClockSessionTest --offline --console=plain
git diff --check
```
PASS: 214 data-free Rust tests (109 storage), strict Clippy/fmt, workspace build
through the real-Full host Kotlin/JNI runner with Unicode/analysis exchanges,
Kotlin formatting, both-ABI native/debug/test APK builds, strict Android lint and
all eight selected API-36 x86_64 Android regression tests (151 seconds total build
and run). Report: `android/app/build/reports/androidTests/connected/debug/index.html`.

Six new repository tests cover marker durability through backward movement,
reflow/checkpoints/reopen, each reread, position-only EOF, deletion/reimport and
late replay, concurrent callbacks, migration EOF-history backfill (including NUL/
supplementary Unicode) and backfill failure rollback/retry. Extended analyzed
completion failure injection covers the marker write and confirms that all failed
statements/deferred commits preserve history/status/time/position and false marker;
the exact retry commits them together. Existing pagination completion coverage now
asserts only the final page sets true. Gradle tracks embedded SQL in native inputs.

Initial storage verification found one stale schema-version expectation, corrected
before the passing workspace suite. Gradle's sandbox wildcard-IP/lock socket error
passed with escalation. Prior benign Crux JNI shutdown and ktfmt Unsafe warnings
persist. No open failure or dependency/public-codec change. Migration 0011 is new.
No full Android suite, standalone owned/mapped Full fixture suite, release APK,
process/reboot recovery, tablet or physical-arm64 rerun. Marker projection/checkmark,
reopen-at-page-1, Finish Book and confetti remain unchecked; this slice is storage-only.

## Finish projections, checkmarks and explicit page-1 reopening

Verified 2026-10-24 with the Rust PATH workaround and Android
environment above. Commands from repository root:
```sh
cargo test --workspace --all-features --locked --offline --quiet
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked --offline -- -D warnings
tools/test-kotlin-bridge.sh
cargo run -p yomoku-ffi --bin generate-kotlin --features facet_typegen --locked --offline -- target/generated/kotlin-repeat
diff -r target/generated/kotlin target/generated/kotlin-repeat
tools/format-kotlin.sh --check
./android/gradlew -p android :app:connectedDebugAndroidTest :app:lintDebug -Pandroid.testInstrumentationRunnerArguments.class=app.yomoku.ReaderNavigationTest,app.yomoku.ReaderClockSessionTest,app.yomoku.PasteReaderTest --offline --console=plain
./android/gradlew -p android :app:connectedDebugAndroidTest :app:lintDebug '-Pandroid.testInstrumentationRunnerArguments.class=app.yomoku.PasteReaderTest#savedOffsetsUpdateLibraryAndReopenContainingPageAtTop' --offline --console=plain
git diff --check
```
PASS: 217 data-free Rust tests (111 storage), strict Clippy/fmt, real-Full host
Kotlin/JNI with seven Unicode exchanges, repeatable Kotlin generation and Kotlin
formatting. Both-ABI debug/test APK builds and strict Android lint pass. Fourteen
distinct API-36 x86_64 tests are verified: 13 pass in both selected regression runs,
then the corrected PasteReaderTest passes in the final focused run (31 seconds).
Latest report: `android/app/build/reports/androidTests/connected/debug/index.html`.

Two new storage tests cover all five presets, marker/progress projections, exact
unfinished/position-only EOF, read-only loads, durable finished-book reset, stored
Mode B identity, pending-time preservation, replay after reopening, missing IDs and
write/commit failure rollback plus retry. One new core test verifies separate
OpenReader/LoadPaste operations, committed Library updates, failed-open preservation
and finished-state reflow. Host JNI verifies completion remains at EOF, rereads,
fresh-session finish projection and analyzer-free explicit page-1 reopening.
The new Compose/JNI/SQLite test covers final-page marker failure/Retry, rereads,
accessible checkmarks at 100% and 0%, and retained recreation without reset.

The initial selected device run passed 13/14 tests, including the new finish test.
An existing PasteReaderTest assumed its Library progress row was composed offscreen;
it now scrolls the LazyColumn by document key before assertions, supporting
accumulated test documents. An initial attempt to scroll the uncomposed child
itself also failed; the final key-based correction passes.
Current snapshot queries require schema 0011, so historical migration fixtures now
seed old position/time/history directly rather than call newer readers. Initial
Kotlin unnecessary-safe-call warnings were fixed. Gradle's sandbox lock/socket
failure required escalation; prior benign Crux JNI/ktfmt warnings persist.

No schema/dependency change. Generated storage protocol adds OpenReader; document,
Library and reader payloads add the permanent boolean. No full Android suite,
standalone Full fixture/probe, release APK, process/reboot, tablet or physical-arm64
rerun. Finish Book and command-keyed confetti remain next.

## Finish Book and per-command completion confetti

Verified 2026-10-24 with the Rust PATH workaround and Android environment above.
Host JNI/native runners ran sequentially; repeat generation used a separate directory.
Commands from repository root:
```sh
cargo test --workspace --all-features --locked --offline --quiet
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked --offline -- -D warnings
tools/test-kotlin-bridge.sh
cargo run -p yomoku-ffi --bin generate-kotlin --features facet_typegen --locked --offline -- target/generated/kotlin-repeat
diff -r target/generated/kotlin target/generated/kotlin-repeat
tools/format-kotlin.sh --check
./android/gradlew -p android :app:connectedDebugAndroidTest :app:lintDebug -Pandroid.testInstrumentationRunnerArguments.class=app.yomoku.ReaderNavigationTest,app.yomoku.ReaderClockSessionTest,app.yomoku.PasteReaderTest --offline --console=plain
git diff --check
```
PASS: 219 data-free Rust tests (111 storage), strict Clippy/fmt, real-Full host
Kotlin/JNI with seven Unicode exchanges, reproducible Kotlin generation and Kotlin
formatting. Both-ABI debug/test APK assembly, strict Android lint and all 15 selected
API-36 x86_64 Android tests pass (5m42s build/run). Report:
`android/app/build/reports/androidTests/connected/debug/index.html`.

Two new core tests cover final-page Library return, command-keyed celebration for
positive/zero New words and permanent-marker rereads, AlreadyRecorded silence,
failed completion/exact retry, duplicate callbacks, stale acknowledgment and final
clock checkpoint/deferred-exit ordering. Host JNI verifies Library return and fresh
reread command IDs without analyzer loading. Updated Compose/JNI/SQLite finish test
covers duplicate Finish Book callbacks, transaction failure/Retry, 100% Library
state, recreation and rereads. A new animation test freezes the Compose clock to
verify visible confetti, bounded duration, recomposition stability, a second command
and no replay after recreation. The final-clock test now completes a one-page book
and verifies one 1,000ms completion before draining close, with no leftover time.

Initial core clock regression expected a reader after final completion; its fixture
now explicitly reopens. Initial strict Kotlin build rejected an unnecessary safe
call; fixed. The host reread fixture initially reused its pre-reopen offset, now
captures the fresh page-1 anchor/position. Final checks pass. Gradle ran without
additional escalation using its existing daemon; prior benign Crux/ktfmt warnings
persist. No schema/dependency change. Generated View/Event add a book-completion
command notice and keyed acknowledgment. No full Android suite, standalone Full
fixture/probe, release APK, process/reboot, tablet or physical-arm64 rerun.

## Read-only reader token/status foundation

Verified 2026-10-24 with the Rust PATH workaround above:
```sh
cargo build --workspace --locked --offline
cargo test --workspace --all-features --locked --offline --quiet
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked --offline -- -D warnings
tools/test-sudachi.sh
git diff --check
```
PASS: 229 data-free Rust tests (117 storage), strict Clippy/fmt and all eleven
real-Full analysis fixtures plus the runtime fixture with each owned/mapped loader.
Four new pure and six new repository tests cover page-local UTF-8 spans, original
full crossing-token identity, five presets, Unicode/repeated/global surfaces, all
five statuses, exact metadata, missing legacy analysis, deleted-entry absence and
corruption rejection. Query-only SQLite and exact database-byte checks verify lookup
does not mutate progress/statuses/history/overrides or recreate deleted vocabulary.
The existing real-Full analyzed import/completion fixture now compares projected
indices/anchors/surfaces/readings/forms and live statuses before/after completion,
including lookup at finished EOF without the explicit reader-open reset.

No schema/dependency change. Types support future Serde/Facet transport, but this
slice adds no event/view codec, generated Kotlin, Android UI or runtime command.
Android/JNI/lint, process/reboot and physical-arm64 checks were not rerun; their
previous results above remain historical. Next: wire token reads and manual status
changes through pure core/FIFO/FFI, then Compose taps/five direct choices. No failure
or local blocker.

## Typed reader-token reads through Crux/FIFO/FFI

Host verification on 2026-10-24 uses the Rust PATH workaround above. Native
generation/build/test runners ran sequentially:
```sh
cargo test --workspace --all-features --locked --offline --quiet
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked --offline -- -D warnings
tools/test-kotlin-bridge.sh
cargo run -p yomoku-ffi --bin generate-kotlin --features facet_typegen --locked --offline -- target/generated/kotlin-repeat
diff -r target/generated/kotlin target/generated/kotlin-repeat
tools/format-kotlin.sh --check
./android/gradlew -p android :app:connectedDebugAndroidTest :app:lintDebug -Pandroid.testInstrumentationRunnerArguments.class=app.yomoku.CoreSessionTest,app.yomoku.ReaderNavigationTest,app.yomoku.ReaderClockSessionTest --offline --console=plain
git diff --check
```
PASS: 234 data-free Rust tests (117 storage), strict Clippy/fmt, workspace build
through the real-Full host Kotlin/JNI runner, seven Unicode exchanges, repeatable
Kotlin generation and Kotlin formatting. Four new core tests verify current-key
request/result checks, cancellation, reflow/movement invalidation, read errors/retry,
legacy Missing, and final-time/deferred-exit draining even after read failure. A new
asynchronous FFI test verifies original Unicode metadata and all five live statuses;
exact database-byte comparisons prove reads do not write. Real-Full Kotlin/JNI
checks New before completion, Known on fresh lookup, and analyzer-free reopen.

An initial synthetic FFI fixture omitted non-Japanese nonwhitespace from analysis;
adding the required unselectable token fixed validation before the passing suite.
Gradle's sandbox wildcard-IP/lock-socket error required escalation. Both-ABI native/
debug/test APK builds and strict Android lint pass. The initial device run lost the
emulator after four passed tests; its report contains a disconnected-device error
and an incomplete finish test, without an assertion stack. Restarted the existing
AVD with documented swangle flags, retaining data. The complete rerun passes all
11 selected API-36 x86_64 tests in 2m38s, including finish/recreation/rereads and
final clock/native-close regressions. Report:
`android/app/build/reports/androidTests/connected/debug/index.html`.
No open failure or local blocker; the emulator remains running.

No schema/dependency change. Event/View/Storage gain typed token-read payloads;
Android's preview constructor supplies the added optional view field. No Compose
token request/rendering or manual status commands yet. No standalone Full fixture
suite, release APK, process/reboot recovery, tablet or physical-arm64 rerun.

## Typed manual reader status choices

Verified 2026-10-24 using the Rust PATH workaround and Android/JDK environment
above. Host JNI/native runners ran sequentially; repeat payload generation used
its separate directory. Commands from repository root:
```sh
cargo test --workspace --all-features --locked --offline --quiet
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked --offline -- -D warnings
tools/test-kotlin-bridge.sh
cargo run -p yomoku-ffi --bin generate-kotlin --features facet_typegen --locked --offline -- target/generated/kotlin-repeat
diff -r target/generated/kotlin target/generated/kotlin-repeat
tools/format-kotlin.sh --check
./android/gradlew -p android :app:connectedDebugAndroidTest :app:lintDebug -Pandroid.testInstrumentationRunnerArguments.class=app.yomoku.CoreSessionTest,app.yomoku.DocumentSessionTest,app.yomoku.ReaderNavigationTest,app.yomoku.ReaderClockSessionTest --offline --console=plain
git diff --check
```
PASS: 238 data-free Rust tests (117 storage), strict Clippy/fmt, real-Full host
Kotlin/JNI, seven Unicode exchanges, reproducible payload generation and Kotlin
formatting. After a test-fixture span adjustment, all 39 core reader-flow tests
and strict Clippy/fmt pass again.
Final FFI timestamp/foreign-key fixture assertions also pass with
`cargo test -p yomoku-ffi --test documents --all-features --locked --offline token_reads_and_manual_choices_preserve_metadata_and_global_status_semantics --quiet`;
strict Clippy passes afterward.
Both-ABI native/debug/test APK builds, strict Android lint and all 13 selected
API-36 x86_64 emulator tests pass (3m32s build/run). Report:
`android/app/build/reports/androidTests/connected/debug/index.html`.

Four new core tests cover visible existing IDs, invalid timestamps/stale keys,
Settings/busy rejection, all five choices, repeated occurrences, deleted IDs,
cancelled/mismatched results/errors, retry and final clock/deferred-exit drain on
success or failure. Extended asynchronous FFI coverage checks dated manual history,
same-status no-op, status/history rollback and explicit retry, analyzer-free fresh
worker reads, no completion/position effects and deletion without re-registration.
Real-Full Kotlin/JNI verifies every choice and unchanged metadata, repeated choices,
fresh reads and no analyzer reload. A new Android DocumentSessionTest exercises
all five choices and global exact-surface status across two documents with an
independent other surface, without moving/completing the reader.

ADB's sandbox socket denial and Gradle's wildcard-IP/lock socket error required
escalation; the escalated runs pass. Prior benign Crux JNI shutdown diagnostics
persist. No open failure or local blocker.

No schema/dependency/View change. Event/Storage gain typed manual-choice payloads.
Compose token loading/highlights/taps/five direct controls remain next. Standalone
Full fixture/probe suite, full Android suite, release APK, process/reboot recovery,
tablet and physical-arm64 were not rerun.

## Compose reader word interactions

Verified 2026-10-24 with the Rust PATH workaround and Android/JDK environment above.
Run from the repository root; the two Android groups cover seven new word tests
and seventeen reader/navigation/clock/paste regressions:
```sh
cargo test --workspace --all-features --locked --offline --quiet
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked --offline -- -D warnings
tools/format-kotlin.sh --check
./android/gradlew -p android :app:connectedDebugAndroidTest :app:lintDebug -Pandroid.testInstrumentationRunnerArguments.class=app.yomoku.ReaderWordSpansTest,app.yomoku.ReaderWordsTest --offline --console=plain
./android/gradlew -p android :app:connectedDebugAndroidTest :app:lintDebug -Pandroid.testInstrumentationRunnerArguments.class=app.yomoku.ReaderNavigationTest,app.yomoku.ReaderClockSessionTest,app.yomoku.PasteReaderTest,app.yomoku.NavigationTest --offline --console=plain
git diff --check
```
The wide-screen dark-mode matrix uses the same seven-test word command between
these reversible emulator overrides (initial mode was `no`, no size/density override):
```sh
/home/kai/Android/Sdk/platform-tools/adb shell wm size 2560x1600
/home/kai/Android/Sdk/platform-tools/adb shell wm density 240
/home/kai/Android/Sdk/platform-tools/adb shell cmd uimode night yes
# Run the seven-test word command above.
/home/kai/Android/Sdk/platform-tools/adb shell wm size reset
/home/kai/Android/Sdk/platform-tools/adb shell wm density reset
/home/kai/Android/Sdk/platform-tools/adb shell cmd uimode night no
```
Reports are copied to ignored `.local/test-reports/reader-words/` before subsequent
connected runs replace the standard Gradle report. Seven tests cover exact scalar
conversion (supplementary/combining/NUL/clipped tokens), malformed/foreign spans,
both status palettes, actual inline taps and five choices, repeated highlights,
unchanged lookup history/position, rollback/explicit repeat, deleted vocabulary,
Back/recreation, save gates and read-error clearing without automatic replay.
A fake-clock/real-JNI UI test verifies 750,000ms across sheet/panel interactions;
the unobserved-content regression stops at 599,000ms instead.

Intermediate failures fixed: Kotlin preview/default-lambda and unnecessary-assertion
warnings; a fixture assuming a different Sudachi segmentation; deprecated
Configuration sizing flagged by lint (use LocalWindowInfo); bottom-sheet container
input missing metadata touches (observe content too); uncomposed Library rows in
older tests (scroll by document key). After an emulator disconnection, restarted
the existing AVD with documented swangle flags without clearing data. An initial
post-boot Full import exceeded the fixture's 30s wait; its allowance is now 60s.
Stopped the intermediate stale APK run after diagnosing its failures; its interrupted
clock test is not an assertion result. Sandbox Gradle/ADB sockets required escalation.
No dependency, schema or FFI change. Host Kotlin/JNI, repeat generation, standalone
Full fixtures/probes, release APK, separate process/reboot and physical arm64 were
not rerun for this UI slice.

PASS: 238 Rust tests (117 storage), strict Clippy/Rust fmt/Kotlin fmt/diff, both-ABI
debug/test APK builds and strict Android lint. All 24 distinct selected API-36
x86_64 tests pass: seven phone/light word tests (1m49s build/run), seventeen existing
regressions (7m), then the same seven word tests on wide landscape/dark (1m37s).
Saved XML reports confirm 31 successful executions with no failures/errors.
Restored physical 1080x2400/420dpi and night mode `no`; AVD remains running.
No open failure or local blocker. Next: offline JMdict/JMnedict index, then actual
definitions/names/encounter counts in the existing word panel.

## Offline JMdict English/JMnedict compilation

Verified 2026-10-25 with Python 3.14.8/SQLite 3.51.2; no Python dependencies added.
Source lock, complete notices, schema/payload contract and data-refresh procedure:
`third_party/dictionaries/README.md`. Downloaded the two official daily snapshots
explicitly (network required sandbox escalation), then ran all preparation offline:

```sh
python3 tools/prepare-dictionaries.py --sources .local/dictionaries/upstream
python3 -W error tools/test-prepare-dictionaries.py --full .local/dictionaries/index-v1-20261025 -v
python3 tools/prepare-dictionaries.py --output .local/dictionaries/index-v1-20261025-repeat
diff -r .local/dictionaries/index-v1-20261025 .local/dictionaries/index-v1-20261025-repeat
python3 tools/prepare-dictionaries.py --lookup 東京
python3 -m tabnanny tools/prepare-dictionaries.py tools/test-prepare-dictionaries.py
cargo test --workspace --all-features --locked --offline --quiet
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked --offline -- -D warnings
tools/format-kotlin.sh --check
git diff --check
```

PASS: nine dictionary tests (eight synthetic plus one complete real-data fixture),
two full byte-identical bundles, 238 Rust tests (117 storage), strict Clippy and
Rust/Kotlin formatting. Without --full the real-data fixture is explicitly skipped.
Coverage: all senses/readings, inherited POS across filtered languages, restrictions,
gloss/loan metadata, separate simultaneous word/name matches, kana/homophones,
supplementary/decomposed Unicode, exact/no matches, query plan/read-only database
bytes, stable output reuse, corruption/count/date/duplicate/malformed input rollback,
notice verification and preserved published output. Real data checks 食べる, たべる,
ありがとう and 東京 across all 218,957 word/743,682 name entries.
Index SHA-256: `0be9d4493aa3fa4474dbb27291820a63a27748dbc63e5517f67be39ffea8e152`.

Initial tests exposed Python SQLite context managers leaving connections unclosed;
explicit closing fixes this and the suite passes with warnings treated as errors.
The retired wiki URL returned a closure page; retained its linked complete static
project document instead. No open failure/blocker. No app dependency/user schema/
FFI/UI change; Android APK/lint/device, host JNI/generation, standalone Full,
release, process/reboot and physical-arm64 checks were not rerun for this tooling.

## Typed read-only Rust dictionary lookup

Verified 2026-10-25 using the Rust PATH workaround above. Run from repository root:

```sh
cargo test --workspace --all-features --locked --offline --quiet
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked --offline -- -D warnings
YOMOKU_DICTIONARY_INDEX="$PWD/.local/dictionaries/index-v1-20261025/dictionary.sqlite3" cargo test -p yomoku-storage full_index_all --locked --offline -- --ignored --nocapture
python3 -W error tools/test-prepare-dictionaries.py --full .local/dictionaries/index-v1-20261025 -v
git diff --check
```

PASS: 248 data-free Rust tests (122 storage), strict Clippy/fmt, nine Python
synthetic/full tests, and one explicit full-index Rust test validating all 218,957
word/743,682 name payloads plus real inflected/kana/simultaneous-word-name lookups.
The latter took 10.14s locally, which is a validation duration, not a lookup benchmark.
Without the explicit ignored-test command the downloaded-data test does not run.

Ten new data-free tests cover all licensed readings, combined spelling/reading
sense restrictions, kana-only/re_nokanji, ambiguous forms, no applicable senses,
source-scoped overlapping IDs, surface/base provenance and deduplication, all
payload metadata, supplementary/decomposed Unicode/NUL, bound SQL/no match,
indexed query plan, read-only rejection/unchanged bytes/reopen, missing/corrupt/
unsupported assets and malformed/unknown/inconsistent payloads with no repair.

Intermediate failures fixed: `prepare_cached` is unavailable with current
rusqlite features (use existing `prepare`); the initial relative full-index path
resolved inside the crate (use the absolute environment path above). Final checks
pass; no open failure/blocker. Existing locked serde_json 1.0.154 is now a direct
storage dependency; no new packages, user migrations, generated bridge or UI changes.
Android build/lint/device, Kotlin formatting, host JNI/generation, standalone
Sudachi Full, release, process/reboot and physical-arm64 checks were not rerun.
Android verified installation/notices and dictionary FIFO/FFI/panel/counts remain.

## Verified Android word/name assets and credits

Verified 2026-10-25. Prepare the pinned index explicitly before Android builds;
Gradle never downloads data. `third_party/dictionaries/README.md` documents the
build/index lock, full legal/source bundle and immutable installation contract.
Run with the Rust PATH/JDK/SDK/Gradle variables from the Android section above:

```sh
python3 -W error tools/test-bundle-dictionaries.py -v
python3 -W error tools/test-prepare-dictionaries.py -v
python3 tools/bundle-dictionaries.py
# Repeat bundling verifies unchanged output, without replacing it.
python3 tools/bundle-dictionaries.py
cargo test --workspace --all-features --locked --offline
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked --offline -- -D warnings
tools/format-kotlin.sh --check
./android/gradlew -p android :app:assembleDebug :app:assembleDebugAndroidTest :app:lintDebug --offline --console=plain
/home/kai/Android/Sdk/platform-tools/adb -s emulator-5554 shell cmd connectivity airplane-mode enable
./android/gradlew -p android :app:connectedDebugAndroidTest :app:lintDebug -Pandroid.testInstrumentationRunnerArguments.class=app.yomoku.WordDictionaryAssetsTest,app.yomoku.SudachiDictionaryFilesTest,app.yomoku.SudachiAssetsTest --offline --console=plain
```

PASS: 248 data-free Rust tests, strict Clippy/Rust/Kotlin formatting, four bundler
tests, eight compiler tests (the real-data compiler test is explicitly skipped),
both-ABI debug/test APK compilation and strict lint. All 14 selected API-36 x86_64
phone/light tests pass: every installed index/source/compiler/notice checksum;
lazy worker-only private extraction, SQLite read-only format/東京 word+name lookup,
inode-preserving verified reopen; existing interruption/corruption/rename/concurrent
publication tests; path verification failure/repair with a held old reader; both
offline credit dialogs/recreation. No mutable user migration or FFI change.

Both credit/recreation tests also pass at 2560x1600/240dpi in dark mode and airplane
mode, using the reversible overrides documented above and this selected command:

```sh
./android/gradlew -p android :app:connectedDebugAndroidTest '-Pandroid.testInstrumentationRunnerArguments.class=app.yomoku.WordDictionaryAssetsTest#settingsShowsAllOfflineCreditsAcrossRecreation,app.yomoku.SudachiAssetsTest#settingsShowsCompleteOfflineNoticesAcrossRecreation' --offline --console=plain
```

Reports: `.local/test-reports/dictionary-assets/{phone,wide-dark}/`. Restored
1080x2400/420dpi, light mode and original airplane-mode-off state. AVD remains running.
Real index extraction: 1,392–1,433ms; query + complete cache reverification:
579–582ms in two instrumented emulator processes. These are warm host/emulator
measurements, not physical-device/cold-storage/lookup performance claims.
Index size: 279,109,632 bytes. All 12 dictionary assets: 302,244,976 raw bytes,
79,055,035 ZIP bytes. Final debug APK: 281,673,762 bytes, including existing Full
and debug analyzer probe. Compressed inputs account for ~22 MiB of raw assets.

Intermediate failures fixed: numeric date triggered TypographyDashes lint (spell
out the month); Android's asset merger expanded `.gz` inputs and removed their
suffix (retain exact archives as `.gz.bin`). The installed-byte test caught this;
all final tests pass. Preserved the initial generated directory under ignored
`target/generated/dictionary-assets-before-gzip-fix/` before fresh publication.
Gradle/ADB sockets and emulator startup required sandbox escalation. No open
failure/blocker. Rust runtime lookup provider, panel definitions/names/POS/counts
remain next. Host JNI, standalone Full fixtures, full reader matrix, release,
process/reboot recovery and physical arm64 were not rerun.

## Selected reader word dictionary/count repository

Verified 2026-10-25 using the Rust PATH workaround above. From repository root:

```sh
cargo test -p yomoku-storage reader_words --locked --offline
cargo test --workspace --all-features --locked --offline --quiet
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked --offline -- -D warnings
git diff --check
```

PASS: 256 data-free Rust tests (130 storage), strict Clippy/fmt/diff. Eight new
repository tests cover separate simultaneous word/name matches, all applicable
readings/senses and original inflected identity; exact lifetime counts across
repetitions/rereads/duplicate completions/documents/statuses with base-form isolation;
no counts for pending time; document-deletion preservation, vocabulary-deletion
reset and later registration; stale/foreign/missing/non-Japanese/off-page selection;
all-presets full crossing-token identity; supplementary/decomposed/NUL exactness;
query-only/unchanged user/index bytes; dictionary failure without partial results
and explicit retry; existing covering-index use. Existing reader-token regressions
also pass after factoring their shared read-transaction helper.

Initial test compilation required correcting fixture module placement/imports and
using the existing TextOffset conversion/PageLength default APIs. No new package,
migration or core/FFI/UI protocol. No open failure or blocker. Android/JNI/Kotlin,
payload generation, Python/full-index/Sudachi fixtures, release/process/reboot and
physical arm64 checks were not rerun for this repository-only step. Next: lazy
verified dictionary FIFO/typed FFI and panel details/count rendering.

## Selected-word lazy FIFO and typed dictionary bridge

Verified 2026-10-25 using the Rust PATH and Android/JDK environment above. From
repository root, after preparing the pinned Full and word/name index:

```sh
cargo test --workspace --all-features --locked --offline --quiet
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked --offline -- -D warnings
tools/test-kotlin-bridge.sh
cargo run -p yomoku-ffi --bin generate-kotlin --features facet_typegen --locked --offline -- target/generated/kotlin-repeat
diff -r target/generated/kotlin target/generated/kotlin-repeat
tools/format-kotlin.sh --check
bash -n tools/test-kotlin-bridge.sh
./android/gradlew -p android :app:assembleDebug :app:assembleDebugAndroidTest :app:lintDebug --offline --console=plain
./android/gradlew -p android :app:connectedDebugAndroidTest :app:lintDebug -Pandroid.testInstrumentationRunnerArguments.class=app.yomoku.ReaderWordDetailsSessionTest,app.yomoku.DocumentSessionTest,app.yomoku.CoreSessionTest --offline --console=plain
git diff --check
```

Run the host JNI script and Android builds sequentially: both BoltFFI generation
paths replace host build artifacts. Concurrent execution caused a host JNI link
failure (missing native exports); the isolated rerun passes. The host script now
also requires `YOMOKU_WORD_DICTIONARY`, defaulting to the prepared pinned
`.local/dictionaries/index-v1-20261025/dictionary.sqlite3`. It never downloads data.

PASS: 262 data-free Rust tests (130 storage; four new core and two new asynchronous
FFI tests), strict Clippy/Rust/Kotlin fmt/diff/shell syntax, reproducible payload
generation, real-Full/real-index Kotlin/JNI plus seven bidirectional Unicode
exchanges, both-ABI debug/test APK builds and strict Android lint. Four selected
API-36 x86_64 tests pass (one new transport test, two document-session regressions,
one bridge/session regression). Saved reports:
`.local/test-reports/reader-word-details/{report,results}/`. AVD remains running.

Coverage: lazy worker-only verified index callback/reuse without analyzer loads;
separate simultaneous word/name payloads, all readings/senses/applicable indices,
base-form provenance, empty matches, exact lifetime counts after reread/reopen;
load and malformed-query failures, atomic inode repair and explicit retry;
unchanged user/index bytes, blocked-provider dismissal, stale key/index/token
rejection, Settings/navigation/refresh/movement/reflow cancellation, live statuses/
deleted vocabulary, persistent word error across clock saves, final-time draining
on both read outcomes. Core/FFI prove cancellation; Compose details requests,
rendering and recreation dismissal remain next, with the old metadata/status panel.

Intermediate generator failure rejected generic Candidate/FormMatch with different
parameters; concrete word/name types fix it. Strict Clippy flagged the larger
internal Event; boxing only StorageResult's details bounds it without wire changes.
Gradle wildcard-IP/ADB socket sandbox failures required escalation; approved runs
pass. Existing benign Crux JNI shutdown diagnostics persist. No new dependency or
mutable migration. Python/compiler/full-index scans, standalone Sudachi fixtures/
probes, full Android reader matrix, tablet/dark, release, process/reboot recovery
and physical arm64 were not rerun. No open failure or blocker.

## Reader dictionary panel

Commands for the 2026-10-25 panel integration, using the Rust PATH and Android/JDK
environment above. All required data is already prepared; these commands are offline:

```sh
tools/format-kotlin.sh --check
./android/gradlew -p android :app:assembleDebug :app:assembleDebugAndroidTest :app:lintDebug --offline --console=plain
./android/gradlew -p android :app:connectedDebugAndroidTest :app:lintDebug -Pandroid.testInstrumentationRunnerArguments.class=app.yomoku.ReaderWordsTest,app.yomoku.ReaderDictionaryTest,app.yomoku.ReaderWordDetailsSessionTest,app.yomoku.ReaderWordSpansTest --offline --console=plain
./android/gradlew -p android :app:connectedDebugAndroidTest :app:lintDebug '-Pandroid.testInstrumentationRunnerArguments.class=app.yomoku.ReaderWordsTest#movingBothDirectionsDismissesDetailsAndNeverRestoresTheSelection,app.yomoku.ReaderNavigationTest,app.yomoku.ReaderClockSessionTest,app.yomoku.PasteReaderTest,app.yomoku.NavigationTest' --offline --console=plain
git diff --check
```

The first connected group has 14 tests: eight panel tests, two dictionary-view
helpers, one real-index transport test and three Unicode/status-span tests. The
second repeats the corrected movement test plus 17 reader/navigation/clock/paste
regressions. For the wide/dark panel matrix, use the first group with the documented
2560x1600/240dpi/night overrides,
then reset size/density/night mode. Save reports before each later connected run
overwrites them under `.local/test-reports/reader-dictionary-panel/`.

Panel coverage includes separate simultaneous JMdict/JMnedict matches, applicable
readings/senses and compact POS, inflected base lookup, lifetime counts after
completion, unchanged position/status/history on lookup, no-match versus error,
explicit retry after timer saves, busy/pending-save request gates, late-result
discard after close, both movement directions, same-key token refresh,
Settings/reflow/recreation dismissal, five choices/deleted entries and popup input
extending the idle deadline.

PASS: both-ABI debug/test APK builds, strict Android lint, Kotlin formatting and
diff checks; all 14 dictionary/span/session tests on phone/light and wide/dark,
and 17 phone reader/navigation/clock/paste regressions. The expanded phone group
initially passed 13/14; the corrected movement test then passed with all 17
regressions (18/18). The final wide/dark group passes 14/14. Saved reports are in
`phone-initial`, `phone-regressions` and `tablet-dark` under the report root above.

Recovered the six uncommitted Android files after the interrupted session. An old
mock dispatch did not echo the core's accepted word selection; update that fixture
and assert deferred request gates. The new movement fixture assumed page two held
cats; tap its actual visible token instead. Both fixture failures are resolved.
Gradle/ADB sandbox socket failures required escalation; approved runs pass. Restored
physical 1080x2400/420dpi and night `no`; AVD remains running, assets preserved.
No new dependency, migration, Rust or wire change. Host Rust/Clippy/JNI/generation,
Python/full-index scans, standalone Full fixtures/probes, release, separate
process/reboot recovery and physical arm64 were not rerun. No open blocker.

## Atomic TXT original-file repository

Verified 2026-10-25 using the Rust PATH workaround above; no new package/version:

```sh
cargo test -p yomoku-storage tests::txt --locked --offline
cargo test --workspace --all-features --locked --offline --quiet
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked --offline -- -D warnings
tools/test-sudachi.sh
git diff --check
```

PASS: 287 data-free Rust tests (142 storage, twelve new TXT repository/migration
fixtures), strict Clippy/fmt/diff. After strengthening the migration/history and
query-only assertions, all twelve targeted tests and strict Clippy pass again.
All twelve explicit real-Full fixtures and one runtime fixture pass with each
owned/mapped loader, including the new TXT import/token/completion/reopen fixture.
The script verifies the 138 pinned engine/patch files before running. Real-data
fixtures remain explicitly ignored by the ordinary workspace command.

Coverage: exact original bytes/encoding/filename and supplementary/decomposed/NUL/
hard-break/ruby canonical text; separate paste and encoding/BOM/markup identities;
existing duplicates bypass decode/analysis and preserve progress/finish/statuses;
all five global statuses; unique registrations without encounters; invalid files/
analysis/timestamps; completed-file isolation; every write/deferred commit rollback
and retry; concurrent create-once; deletion/reimport; stale digest and invalid ruby;
query-only unchanged-byte loads; source-table migration with old Mode B, position,
finish, completion/occurrence/status history and source ID high-water preservation;
rollback and FK-check/enforcement restoration. Existing paste/storage regressions pass.

Intermediate fixture failures corrected: use truly ambiguous C2 A9 bytes rather
than unambiguous UTF-8 あ, select encoding for ambiguous 猫, and explicitly disable
FKs when constructing the corrupt migration fixture. The full-width-ID regression
caught the rebuilt empty sources table creating a spurious sqlite_sequence row;
remove it when the old sequence was absent. All final checks pass. No sandbox
escalation/download, new wire payload, Android or UI change. Android/Kotlin/JNI/
generation, Python/full-index scans, release/process/reboot and physical arm64
were not rerun for this storage slice. No open failure/blocker.

## Typed TXT FIFO/JNI import bridge

Verified 2026-10-25 with the Rust PATH and Android/JDK environment above. Host JNI
and Android native generation/builds ran sequentially; no new package or migration:

```sh
cargo test --workspace --all-features --locked --offline --quiet
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked --offline -- -D warnings
tools/test-kotlin-bridge.sh
cargo run -p yomoku-ffi --bin generate-kotlin --features facet_typegen --locked --offline -- target/generated/kotlin-repeat
diff -r target/generated/kotlin target/generated/kotlin-repeat
tools/format-kotlin.sh --check
./android/gradlew -p android :app:assembleDebug :app:assembleDebugAndroidTest :app:lintDebug --offline --console=plain
./android/gradlew -p android :app:connectedDebugAndroidTest :app:lintDebug -Pandroid.testInstrumentationRunnerArguments.class=app.yomoku.DocumentSessionTest,app.yomoku.AnalyzedImportSessionTest,app.yomoku.CoreSessionTest --offline --console=plain
git diff --check
```

PASS: 293 data-free Rust tests (142 storage, thirteen runtime), strict Clippy/Rust
fmt/Kotlin fmt/diff, reproducible payload generation, real-Full/real-index Kotlin/JNI
and seven bidirectional Unicode fixtures, both-ABI app/test APK builds and strict
Android lint. All five selected API-36 x86_64 tests pass, including the new TXT
session fixture and existing paste/status/position/bridge regressions. Reports:
`.local/test-reports/txt-import-bridge/{report,results}/`. Existing AVD cold-booted
without data reset using the documented swangle command; left running.

Five new worker fixtures and one core fixture cover typed encoding-required/all
strict decoding errors, manual choices, exact bytes/filename/Aozora source ruby,
original Mode B, duplicate preservation of encoding/status/position after background
release/fresh session with an unavailable Full provider, analyzer/load/invalid-output
retry, every write and deferred commit rollback with earlier imports retained,
busy/pending reader-save gates and atomic background/close draining. Existing
storage fixtures now verify query-only unchanged-byte preflight and corrupt-digest
rejection. New Kotlin/JNI and Android fixtures exercise production Full, payloads,
original-source/ruby storage, read-only reopening and no automatic reader open.
APK inspection confirms all five encoding_rs copyright/license files match their
checked-in asset bytes. New-file parsing repeats around preflight; performance
on large files is unmeasured.

Intermediate strict Kotlin warnings (redundant safe calls/non-null assertion) were
fixed before passing reruns. ADB socket and .git write denials required escalation;
no download.
Existing benign Crux JNI close diagnostics persist. No open failure/blocker.
Standalone Full fixtures/probes, Python/full-index scans, release app, separate
process/reboot, tablet/dark and physical arm64 were not rerun. Next: Android picker,
encoding choice and per-file summary; batch progress/cancel/background notifications
and source-ruby reader transport/rendering remain separate tasks.

## Android multiple-TXT picker and retained encoding/summary

Run from the repository root with the Rust PATH/JDK/SDK/Gradle variables above:

```sh
tools/format-kotlin.sh --check
./android/gradlew -p android :app:assembleDebug :app:assembleDebugAndroidTest :app:lintDebug --offline --console=plain
./android/gradlew -p android :app:connectedDebugAndroidTest :app:lintDebug -Pandroid.testInstrumentationRunnerArguments.class=app.yomoku.TxtPickerTest,app.yomoku.DocumentSessionTest,app.yomoku.AnalyzedImportSessionTest,app.yomoku.CoreSessionTest --offline --console=plain
./android/gradlew -p android :app:connectedDebugAndroidTest :app:lintDebug '-Pandroid.testInstrumentationRunnerArguments.class=app.yomoku.TxtPickerTest,app.yomoku.NavigationTest,app.yomoku.PasteReaderTest#pasteReturnsToLibraryAndOpensFirstLogicalPageWithBackAndRecreation' --offline --console=plain
git diff --check
```

The two connected groups contain seven and five tests (ten distinct tests).
Save reports before subsequent runs overwrite them; this session's root is
`.local/test-reports/txt-picker/`. Picker tests intercept the actual
ACTION_OPEN_DOCUMENT contract result, then use an exported test-APK-only provider
to exercise original-byte ContentResolver reads off the UI thread. The provider
uses only Android/Java APIs because a standalone test-APK process lacks the target
APK's Kotlin runtime. No fixture provider is included in the application APK.

The nine-file batch covers Created/Existing with the existing title, unreadable
URI, unsupported extension, undecodable bytes, explicit UTF-8 and Shift-JIS,
skipping an uncertain file and successful continuation to a later uppercase TXT.
Tests inspect persisted encoding, Library membership, all copied outcomes, visible
scrollable errors/titles, no automatic reader opening, retained encoding through
recreation/background, retained summary through recreation and consumed dismissal.
Picker cancellation preserves Library. Existing groups exercise native/worker
imports, exact-source persistence, failures/retries, statuses, session lifecycle,
paste/reader and navigation.

PASS: both-ABI debug/test APK builds, strict Android lint, Kotlin formatting/diff,
and all five final picker/navigation/paste tests on API-36 x86_64 phone/light in
airplane mode. All five distinct import/session regressions also passed in the
earlier offline group (its remaining failure was the subsequently corrected test
provider). Ten distinct tests passed across these runs. Saved final UI reports:
`.local/test-reports/txt-picker/phone-final/{report,results}/`; native regression
results are in `phone-provider-failure/`. No final application/test failure remains.

Intermediate failures fixed: two member-method imports; picker monitor missing
MIME/category matching; unavailable test-APK cache path, then standalone provider
missing the target Kotlin runtime (use Java/pipe fixtures); duplicate filename
selector across Library/dialog roots (scope to summary). A cold-run native import
also exceeded 30s while Play Store updates were active; the offline rerun passed.
The new multi-file wait permits 90s with diagnostic state. No production timeouts
or parser/storage semantics changed. Gradle/ADB sockets and AVD startup needed
sandbox escalation. Restored airplane mode off; emulator remains running.

No Rust/FFI/schema/dependency change. Host Rust/Clippy/JNI/generation, standalone
Full/Python/full-index scans, tablet/dark, release app, separate process/reboot
recovery and physical arm64 were not rerun. Progress counts, transaction-aware
batch cancellation, background notifications, EPUB/SRT and reader ruby remain.


## Retained TXT current-file and batch progress

Verified 2026-10-25 with the Rust PATH/JDK/SDK/Gradle variables above:

```sh
tools/format-kotlin.sh --check
./android/gradlew -p android :app:assembleDebug :app:assembleDebugAndroidTest :app:lintDebug --offline --console=plain
./android/gradlew -p android :app:connectedDebugAndroidTest :app:lintDebug -Pandroid.testInstrumentationRunnerArguments.class=app.yomoku.TxtPickerTest,app.yomoku.NavigationTest --offline --console=plain
git diff --check
```

PASS: both-ABI debug/test APK builds, strict lint, Kotlin formatting/diff and all
five API-36 x86_64 phone/light picker/navigation tests. Reports copied to
`.local/test-reports/txt-progress/phone/{report,results}/`.

The new test-only gated pipe keeps an actual ContentResolver byte read blocked;
the queried display name (different from the URI segment), File 1 of 1 and zero
processed progress remain visible through Activity recreation. Releasing the gate
reaches the retained encoding prompt without advancing the count; Skip completes
one outcome. The nine-file fixture now asserts 5/9, 6/9 and 7/9 processed outcomes,
including duplicates, read/format/decode failures and skips; encoding choice and
background/recreation retain the count, and summary dismissal resets it. Existing
picker cancellation and navigation/native-session lifecycle regressions pass.

The initial lint run rejected count wording as a plural candidate; use the
quantity-neutral “Processed: X of N” label. Final lint passes. Gradle local socket
setup needed sandbox escalation for the connected run. No new dependency/version,
permission, Rust/wire/schema change, open failure or blocker. AVD left running.
Rust/Clippy/JNI/generation, Full fixtures, Python/full-index scans, tablet/dark,
release app, actual process-death/reboot and physical arm64 were not rerun.
Cancellation with current-transaction rollback, permitted background notifications
and process-death/no-auto-resume acceptance remain unchecked.

## Cooperative TXT repository cancellation

Verified 2026-10-25 from the repository root with the Rust PATH above:

```sh
cargo test --workspace --all-features --locked --offline --quiet
cargo clippy --workspace --all-targets --all-features --locked --offline -- -D warnings
cargo fmt --all -- --check
tools/test-sudachi.sh
git diff --check
```

PASS: 299 data-free Rust tests (148 storage), strict Clippy/fmt/diff, and all twelve
explicit real-Full fixtures plus one runtime fixture with each owned/mapped loader.
The Full script verifies the 138 pinned engine/patch files first. Six new storage
tests cover cancellation before validation/SQL, shared/idempotent requests,
channel-gated analysis rollback with earlier documents/statuses preserved after
reopen, sequence/ID rollback, fresh explicit retry, duplicate/late-cancel isolation,
commit/cancel gate arbitration including failure, and deferred-FK commit rollback.
The shared persistence helper also receives the existing paste/TXT regressions.

Initial targeted TXT run: the new blocked-analysis fixture opened its observer
after the writer had started; Storage::open checks migrations under an immediate
lock, so it failed with DatabaseBusy. Open the observer first, as existing tests
do. The final full-workspace run passes with this correction. No production lock
or timeout change. No escalation/download, new dependency/version or schema change.

This is a repository slice: runtime/JNI and Android still call the noncancellable
wrapper. Cancellation is cooperative, not a decoder/analyzer/SQLite interrupt;
cancel must run outside the FIFO and off Android's main thread because a commit
holding the gate may wait. Remaining URI disposal/summary, native lifecycle races,
background notifications and process-death/no-auto-resume acceptance remain open.
Android builds/lint/instrumentation, host Kotlin/JNI/codegen, standalone Python/index
scans, release, reboot, tablet/dark and physical arm64 were not rerun. No open blocker.

## Direct TXT runtime/JNI cancellation

Verified 2026-10-25 with the Rust PATH/JDK/SDK/Gradle environment above. Host JNI,
repeat generation and Android native builds ran in separate stages:

```sh
cargo test --workspace --all-features --locked --offline --quiet
cargo clippy --workspace --all-targets --all-features --locked --offline -- -D warnings
cargo fmt --all -- --check
tools/test-kotlin-bridge.sh
cargo run -p yomoku-ffi --bin generate-kotlin --features facet_typegen --locked --offline -- target/generated/kotlin-repeat
diff -r target/generated/kotlin target/generated/kotlin-repeat
(cd crates/yomoku-ffi && CARGO_NET_OFFLINE=true ../../.local/bin/boltffi generate kotlin --deny-skipped --output ../../target/generated/native-repeat)
diff -r target/generated/native target/generated/native-repeat
tools/format-kotlin.sh --check
./android/gradlew -p android :app:assembleDebug :app:assembleDebugAndroidTest :app:lintDebug --offline --console=plain
git diff --check
```

PASS: 304 data-free Rust tests (five new: four runtime, one core), strict Clippy/fmt,
real-Full/word-index host Kotlin/JNI and seven Unicode exchanges, reproducible Crux
and native generation, both-ABI app/test APK builds, strict lint/Kotlin formatting
and diff checks. Channel-gated worker fixtures cover cancellation while queued,
loading Full and analyzing, rejected duplicate callbacks, completed-file/status/ID
preservation, fresh/encoding retries, background release and closed-router drain.
The new real JNI fixture invokes cancellation on another JVM thread while the
dictionary-provider callback blocks, then verifies a silent cancelled result,
retained earlier file, successful fresh import/Mode B registrations and reopen.
Repository commit/cancel arbitration tests also pass in the workspace run.

Fixed a new fixture's incorrect vocabulary lookup method name before verification.
Gradle initially failed sandbox wildcard-IP/socket setup; the approved offline run
passes. Existing benign Crux close diagnostics persist. No new dependency, migration
or permission; no open failure/blocker. Android UI/instrumentation, standalone Full
fixtures, Python/full-index scans, release, process-death/reboot, tablet/dark and
physical arm64 were not rerun. Android still has no Cancel control; wire it off main
and await both the native return and current-file outcome before settling a batch.
Notifications and process-death/no-auto-resume acceptance remain unchecked.

## Android transaction-aware TXT batch Cancel

Run from the repository root with the Rust PATH/JDK/SDK/Gradle variables above:

```sh
cargo test --workspace --all-features --locked --offline --quiet
cargo clippy --workspace --all-targets --all-features --locked --offline -- -D warnings
cargo fmt --all -- --check
tools/format-kotlin.sh --check
./android/gradlew -p android :app:assembleDebug :app:assembleDebugAndroidTest :app:lintDebug --offline --console=plain
./android/gradlew -p android :app:connectedDebugAndroidTest :app:lintDebug -Pandroid.testInstrumentationRunnerArguments.class=app.yomoku.TxtPickerTest,app.yomoku.TxtCancellationSessionTest,app.yomoku.CoreSessionTest,app.yomoku.ReaderClockSessionTest,app.yomoku.NavigationTest --offline --console=plain
git diff --check
```

The new picker fixture cancels while a real provider pipe blocks, retains an
earlier Created result through recreation, starts a fresh batch before the old
provider returns, ignores its late bytes, cancels retained encoding selection
after recreation/background and explicitly retries an exact-source duplicate.
Cancelled/unsubmitted files never appear in Library; cancellation adds no fake
error, skipped result or completion count. Existing multi-file encoding/error/
duplicate/progress/summary and system-picker-cancel fixtures remain included.

Two new isolated production-JNI session tests gate the real dictionary callback
and the dedicated cancellation executor. They prove native Cancel returns while
FIFO remains blocked, the batch still awaits its outcome, double Cancel and fresh
start cannot replace the attempt, and explicit retry succeeds. The opposite order
lets a completed commit win before native Cancel executes: retain its outcome,
filename/current number/progress, discard remaining URIs and keep the native handle
alive through owner teardown until Cancel returns. A fresh session sees only that
completed document and no in-process batch. Repository/runtime tests continue to
cover blocked analysis, transaction rollback/ID preservation and commit arbitration.

Initial failures corrected: the Rust fixture asserted weak-lifecycle closure before
the final callback necessarily returned; await its existing factory-disconnect
signal first. A new UI fixture without a BOM was legitimately ambiguous UTF-8/
Shift-JIS and waited at encoding selection rather than reaching the gated read;
give cancellation-only fixtures an explicit UTF-8 BOM. No production decoding
change. Gradle wildcard-IP/local socket setup and AVD/ADB require escalation in
this sandbox. Emulator tests use API-36 x86_64 phone/light, airplane mode.

PASS: 304 data-free Rust tests, strict Clippy/Rust fmt/Kotlin fmt/diff checks,
both-ABI app/test APK builds, strict Android lint and all fourteen final connected
picker/cancellation/core/reader-clock/navigation tests. Three new Android tests;
no new Rust test count. Saved final reports:
`.local/test-reports/txt-cancel/phone-final/{report,results}/`. Earlier reports:
`phone-fixture-failure/` (one corrected ambiguous-input timeout; six tests passed)
and `phone-before-count-fix/` (all seven tests passed). Final Cancel keeps the
current file number fixed even when a winning commit increments processed count
before native cancellation returns; the new JNI assertion verifies it.

No new dependency/version, wire type, schema, permission or production Rust change;
no open failure/blocker. Restored airplane mode off; AVD remains running. Separate
host Kotlin/JNI, standalone Full/Python/index scans, release, tablet/dark, actual
process-death/reboot and physical arm64 were not rerun. Background notifications
and actual process-death/no-auto-resume acceptance remain unchecked.

## Permitted TXT background notifications and abrupt process death

Verified 2026-10-25 with the Rust PATH/JDK/SDK/Gradle variables above:

```sh
tools/format-kotlin.sh --check
./android/gradlew -p android :app:assembleDebug :app:assembleDebugAndroidTest :app:lintDebug --offline --console=plain
./android/gradlew -p android :app:connectedDebugAndroidTest :app:lintDebug -Pandroid.testInstrumentationRunnerArguments.class=app.yomoku.ImportNotificationsTest,app.yomoku.TxtPickerTest,app.yomoku.TxtCancellationSessionTest,app.yomoku.NavigationTest --offline --console=plain
./android/gradlew -p android :app:connectedDebugAndroidTest :app:lintDebug -Pandroid.testInstrumentationRunnerArguments.class=app.yomoku.ImportNotificationsTest --offline --console=plain
/home/kai/Android/Sdk/platform-tools/adb -s emulator-5554 install -r android/app/build/outputs/apk/debug/app-debug.apk
/home/kai/Android/Sdk/platform-tools/adb -s emulator-5554 install -r android/app/build/outputs/apk/androidTest/debug/app-debug-androidTest.apk
python3 tools/test-txt-import-process-death.py --serial emulator-5554
git diff --check
```

PASS: both-ABI debug/test APK builds, strict Android lint, Kotlin formatting/diff,
and ten distinct API-36 x86_64 phone/light tests across the final focused run and
the preceding regression run. The focused two-notification group passes entirely;
the eight existing picker/native cancellation/navigation tests pass in the broader
run. Its only failure was the subsequently corrected notification removal wait.
Reports: `.local/test-reports/txt-background/notifications-final/{report,results}/`
and `removal-wait-failure/{report,results}/` (eight passing existing regressions).
The first run's notification publication wait failure is also preserved under
`notification-wait-failure/`. Both intermediate failures were test assertions made
before Android finished asynchronous notification publication/removal; await the
actual NotificationManager state along with helper/service state. Final tests pass.

New coverage: real three-file continuation with stopped Activity and live dataSync
service; filename/processed progress, recreation, completion counts and PendingIntent
return without opening reader; separate encoding-needed notification with no lease;
Cancel/new batch and direct timeout callback stopping the lease before cooperative
cancellation; no fake result/remaining submission or background service restart.

The host process runner requires both installed APKs (UTP can uninstall them) and
API 33+. Its debug-only DUMP-protected Activity uses an isolated database and the
test APK's gated provider. In each permission state, import one BOM TXT with real
Full, block the second read with the third queued, send HOME then same-UID SIGKILL.
Check no graceful close/service restart; a fresh process must see exactly the
committed document, empty batch/counts and no service/progress notification.
Both granted/denied runs pass with different before/after PIDs. Report:
`.local/test-reports/txt-process-death/20261025T212646351293Z.json`.
Permission is restored and the ordinary Library relaunched. Isolated probe DBs
remain under `no_backup/txt-recovery-<run>` for inspection; normal Library is not
cleared. This is abrupt process death, not LMK pressure, reboot or storage-cache
testing. Timeout coverage invokes the framework callback, not a six-hour OS run.

Three new manifest permissions: POST_NOTIFICATIONS, FOREGROUND_SERVICE and
FOREGROUND_SERVICE_DATA_SYNC. No new dependency/version, Rust/wire or migration.
Gradle wildcard-IP/local sockets and AVD/KVM needed sandbox escalation. No open
failure/blocker; emulator remains running. Rust/Clippy/host JNI/codegen, standalone
Full/Python/index suites, tablet/dark, release/reboot and physical arm64 not rerun.

## Pure SRT import checks

Verified 2026-10-25 from the repository root with the Rust PATH above:

```sh
cargo test -p yomoku-import --locked --offline
cargo test --workspace --all-features --locked --offline --quiet
cargo clippy --workspace --all-targets --all-features --locked --offline -- -D warnings
cargo fmt --all -- --check
git diff --check
```

PASS: 317 data-free Rust tests, including thirteen new SRT tests and thirteen TXT
regressions, strict Clippy/fmt/diff. SRT fixtures cover cue ranges, speakers/numeric
dialogue, LF/CRLF/CR/BOM, missing counters/separators, dot/short/out-of-order timing,
common tags/breaks/entities, preserved malformed metadata/unknown notation, Unicode
and whitespace, empty input, 10,000 cues, long malformed text and all five presets.
Review tightened numeric entities to retain malformed signed references. Final
checks pass; no new dependency/version, schema, wire change or open blocker.

Parser-only: SRT accepts decoded text; original-byte decoding/identity, atomic
repository/runtime registration and Android picker wiring remain unchecked.
Android builds/lint/device, Kotlin/JNI/codegen, explicit Full/index fixtures, release,
process-death/reboot and physical arm64 were not rerun. The ordinary workspace run
retains fourteen ignored data-dependent fixtures, as before.

## Pure EPUB import checks

Verified 2026-10-25 from the repository root with the Rust PATH workaround above:

```sh
cargo fetch
cargo test --workspace --all-features --locked --offline --quiet
cargo clippy --workspace --all-targets --all-features --locked --offline -- -D warnings
cargo check -p yomoku-import --target aarch64-linux-android --target x86_64-linux-android --locked --offline
cargo fmt --all -- --check
git diff --check
```

PASS: 341 data-free Rust tests (24 new EPUB, 26 TXT/SRT regressions), fourteen
data-dependent fixtures ignored as before; strict Clippy/fmt/diff and both Android
target checks. Fixtures build Stored/Deflate archives from readable XML/XHTML;
verify metadata/spine order, headings/emphasis/ruby/links, exact UTF-8/UTF-16
anchors, positioned images/cover/SVG/image-only chapters, whitespace/pre/breaks,
all page presets, large content, CRC/truncation/duplicate/alias/path/symlink errors,
used-content encryption, unused fonts/CSS, decoding/declarations, invalid entities
and source/entry/expansion/node/depth budgets. No filesystem extraction or network.

Initial checks exposed missing roxmltree traversal APIs (use the actual tokenizer
and node APIs), invalid surrogate references silently replaced by the tree parser,
and stack overflow before a post-parse nesting check. Nonrecursive xmlparser
preflight now rejects excessive depth/entity definitions before roxmltree runs,
validates references and passes 10,000-nesting plus near-limit/CDATA/comment tests.
Clippy's one needless borrow was corrected. All final checks pass; no open failure.
Fetched the pinned minimal ZIP/Deflate/XML dependencies (nine new lock entries).

Parser only: no EPUB repository/runtime/FFI/picker/reader wiring or migration.
App still accepts paste/TXT. Remote images, custom/unresolved named entities and
unsupported spine fallback chains fail explicitly; image bytes aren't decoded.
Android APK/lint/device, JNI/codegen, explicit Full/index fixtures, release,
process-death/reboot and physical arm64 were not rerun. No device viability claim.


## Atomic SRT repository checks

Verified 2026-10-25 from the repository root with the Rust PATH above:

```sh
cargo test --workspace --all-features --locked --offline --quiet
cargo clippy --workspace --all-targets --all-features --locked --offline -- -D warnings
tools/test-sudachi.sh
cargo fmt --all -- --check
git diff --check
```

Both Android storage targets also check (NDK r30/API 29, including bundled SQLite):

```sh
export CC_aarch64_linux_android=/home/kai/Android/Sdk/ndk/30.0.16248370/toolchains/llvm/prebuilt/linux-x86_64/bin/aarch64-linux-android29-clang
export AR_aarch64_linux_android=/home/kai/Android/Sdk/ndk/30.0.16248370/toolchains/llvm/prebuilt/linux-x86_64/bin/llvm-ar
export CC_x86_64_linux_android=/home/kai/Android/Sdk/ndk/30.0.16248370/toolchains/llvm/prebuilt/linux-x86_64/bin/x86_64-linux-android29-clang
export AR_x86_64_linux_android=/home/kai/Android/Sdk/ndk/30.0.16248370/toolchains/llvm/prebuilt/linux-x86_64/bin/llvm-ar
cargo check -p yomoku-storage --target aarch64-linux-android --target x86_64-linux-android --locked --offline
```

PASS: 353 data-free Rust tests (twelve new SRT repository tests), strict Clippy,
Rust fmt/diff and both Android targets. Fifteen data-dependent fixtures are ignored
in the ordinary run. The explicit Full runner verifies pinned source integrity and
passes thirteen analysis fixtures plus one worker fixture for each owned/mapped
loader. The new real SRT fixture verifies canonical-only analysis, cue anchors,
unique registration, repeated occurrences, original Mode B, atomic completion,
finished state, duplicate-no-analysis, read-only reopen and explicit finished reset.

Repository fixtures cover exact original bytes/encoding/cues/filename, no Aozora
transformation, cross-TXT duplicate identity, BOM versus canonical/paste identity,
Shift-JIS and ambiguous manual selection, all five existing statuses, invalid files/
analysis/time, every statement and deferred-COMMIT failure with sequence rollback,
blocked-analysis cancellation, prior-file preservation/fresh retry, concurrent
creators, live deletion/reimport, corrupt identity/cue anchors and migration 0013
rollback/retry/high-water/foreign-key preservation. Initial storage run had one
incorrect synthetic unique-count expectation (four, not five); corrected it and
the final workspace run passes. No open failure/blocker or new dependency.

Migration 0013 is live on open; no wire/core/Kotlin change. SRT worker/FFI/picker,
EPUB persistence and mixed-format UI remain open. JNI/codegen/APK/Android lint/device,
standalone index/Python suites, release/reboot and physical arm64 were not rerun.


## SRT FIFO and typed JNI import checks

Verified 2026-10-25 from the repository root with the Rust PATH/JDK/SDK/Gradle
variables above:

```sh
cargo test --workspace --all-features --locked --offline --quiet
cargo clippy --workspace --all-targets --all-features --locked --offline -- -D warnings
cargo fmt --all -- --check
tools/test-kotlin-bridge.sh
cargo run -p yomoku-ffi --bin generate-kotlin --features facet_typegen --locked --offline -- target/generated/kotlin-repeat
diff -r target/generated/kotlin target/generated/kotlin-repeat
(cd crates/yomoku-ffi && CARGO_NET_OFFLINE=true ../../.local/bin/boltffi generate kotlin --deny-skipped --output ../../target/generated/native-repeat)
diff -r target/generated/native target/generated/native-repeat
tools/format-kotlin.sh --check
./android/gradlew -p android :app:assembleDebug :app:assembleDebugAndroidTest :app:lintDebug --offline --console=plain
git diff --check
```

PASS: 365 data-free Rust tests (ten new worker, two core), strict Clippy/Rust+Kotlin
formatting/diff, real-Full/word-index host Kotlin/JNI, seven Unicode exchanges,
reproducible Crux/native bindings, both-ABI debug/test APK builds and strict lint.
The ordinary workspace run retains fifteen ignored data-dependent fixtures.

Worker fixtures cover format-specific encoding errors before Full, metadata-only
empty subtitles, manual UTF-8/Shift-JIS retries, original bytes/cue anchors/Mode B,
shared TXT/SRT identity in both directions, duplicate/reopen without Full, preserved
statuses/position, each-write/deferred-COMMIT rollback and failed-file isolation.
Shared race schedules exercise both TXT/SRT for queued work, blocked Full loading,
blocked analysis, double Cancel/rejected callbacks, background release, fresh retry
and closed-router draining. Core fixtures gate pending reader saves and clear
cancelled attempts without summary/error/movement. The new real JNI cases verify
SRT canonicalization/encodings, Japanese registrations/live statuses, duplicate/
reopen without Full and generic Cancel on another JVM thread while the dictionary
provider blocks. Existing TXT/reader/dictionary/bridge regressions also pass.

Initial new fixtures used the wrong cue-table name and miscounted a supplementary
Unicode range; corrected expectations to subtitle_cues and exact UTF-8 byte length.
One repeat-generation comparison saw an intervening native doc-comment edit;
regeneration through the app build produces identical bindings. Gradle initially
failed sandbox wildcard-IP/local-socket setup; the approved offline run passes.
Existing benign Crux close diagnostics persist. No open failure/blocker.

No new dependency/version or migration. App picker remains TXT-only; SRT is callable
through typed FIFO/JNI and uses the current Library/reader repositories. Android
instrumentation/device, standalone Full/index/Python suites, release, process-death/
reboot, tablet/dark and physical arm64 were not rerun. Next: EPUB persistence/runtime,
then mixed EPUB/TXT/SRT picker/summary/Cancel and reader images/source ruby.

## Atomic EPUB repository and multi-chapter Library progress

Verified 2026-10-26 from the repository root with the Rust PATH above; host JNI
also uses JAVA_HOME from the existing JDK setup:

```sh
cargo test --workspace --all-features --locked --offline --quiet
cargo clippy --workspace --all-targets --all-features --locked --offline -- -D warnings
cargo fmt --all -- --check
tools/test-sudachi.sh
tools/test-kotlin-bridge.sh
git diff --check
```

Both Android storage targets check with bundled SQLite/NDK r30/API 29:

```sh
export CC_aarch64_linux_android=/home/kai/Android/Sdk/ndk/30.0.16248370/toolchains/llvm/prebuilt/linux-x86_64/bin/aarch64-linux-android29-clang
export AR_aarch64_linux_android=/home/kai/Android/Sdk/ndk/30.0.16248370/toolchains/llvm/prebuilt/linux-x86_64/bin/llvm-ar
export CC_x86_64_linux_android=/home/kai/Android/Sdk/ndk/30.0.16248370/toolchains/llvm/prebuilt/linux-x86_64/bin/x86_64-linux-android29-clang
export AR_x86_64_linux_android=/home/kai/Android/Sdk/ndk/30.0.16248370/toolchains/llvm/prebuilt/linux-x86_64/bin/llvm-ar
cargo check -p yomoku-storage --target aarch64-linux-android --target x86_64-linux-android --locked --offline
```

PASS: 379 data-free Rust tests (fourteen new repository fixtures), sixteen
data-dependent fixtures ignored in the ordinary run; strict Clippy/fmt/diff and
both Android target checks. The explicit Full runner verifies source integrity and
passes fourteen analysis fixtures plus one worker fixture per owned/mapped loader.
Its new EPUB fixture verifies every canonical chapter, original Mode B/anchors,
cross-chapter unique registration, preserved Learning, source-ruby exclusion,
chapter-bounded pagination and duplicate/reopen without rescan. Host Kotlin/JNI
passes existing TXT/SRT cancellation/imports, reader/dictionary/navigation/lifecycle
and seven Unicode exchanges against the migrated database. No EPUB wire path yet.

Repository fixtures verify original bytes/title/filename/metadata, spine ordering,
all formatting/ruby/link/placement/image bytes, image-only chapters, repeated shared
assets and case-insensitive website schemes. Tests cover all statuses, equal text
with different source bytes, prior-TXT duplicate reference, renamed/finished/later-
chapter progress preservation, every statement and deferred-COMMIT rollback with
sequence preservation, second-chapter analysis/validation failure, blocked-analysis
Cancel, pre-/late cancellation, concurrent create-once, live deletion/reimport,
corruption without repair, migration rollback/retry/high-water/FK preservation and
whole-book Library percentage in both directions with foreign-position rejection.

Clippy initially requested indirection for the large Created result; boxed that
variant and final checks pass. Existing benign Crux close diagnostics persist.
No new production dependency/version, wire change or open blocker; zip is reused
only for shared test fixtures. Migration 0014 is applied on normal database open.
No escalation/download needed. APK/Android lint/instrumentation/device, repeated
codegen reproducibility, standalone word-index/Python, release, process-death/reboot,
tablet/dark and physical arm64 were not rerun. App remains paste/TXT; generalize
single-text duplicate/reader payloads before exposing EPUB through FIFO/JNI/picker.


## Ordered-chapter reader repository checks

Verified 2026-10-26 from the repository root with the Rust PATH/JDK setup above:

```sh
cargo test --workspace --all-features --locked --offline --quiet
cargo clippy --workspace --all-targets --all-features --locked --offline -- -D warnings
cargo fmt --all -- --check
tools/test-sudachi.sh
tools/test-kotlin-bridge.sh
git diff --check
```

The existing two-ABI `cargo check -p yomoku-storage` command in the preceding
section also passes with NDK r30/API 29 and bundled SQLite.

PASS: 393 data-free Rust tests (five new domain, nine repository), strict Clippy/
fmt/diff, fourteen real-Full analysis plus one worker fixture per owned/mapped
loader, existing real-Full Kotlin/JNI and seven Unicode payload exchanges. Sixteen
data-dependent fixtures remain ignored in ordinary workspace tests. The expanded
real EPUB fixture completes both original Mode B chapters with exact pending time,
validates chapter-local token identity, New-only Known versus preserved Learning,
whole-book finish, duplicate replay and explicit finished-book reset after reopen.

Fixtures cover every preset and 25 reflow pairs, authoritative chapter order,
whole-book counters/progress, next-chapter transitions, backward position-only saves,
independent chapter/EOF time, live dictionary/status/counts, repeats/rereads, replay
including after deletion, missing/wrong analysis, foreign chapters/scalar splits,
corrupt saved content/assets/positions, extra single-text chapters and image-only
zero-new-word final completion. Statement and deferred-COMMIT failures preserve
history/status/time/position/finish; reset also rolls back on later analysis failure.

The initial all-format fixture correctly encountered uncertain TXT decoding; added
explicit UTF-8 selection. Clippy caught duplicate shared EPUB test modules; moved
one shared import to the storage test root. Final checks pass. Existing benign Crux
close diagnostics persist. No open failure/blocker or new dependency/migration.

Legacy application/worker/typed bridge payloads still use single-text snapshots;
EPUB selection is not enabled. Generic readers currently verify the complete saved
EPUB source/assets on every operation; large-book performance is unmeasured.
APK/Android lint/instrumentation/device, repeated-codegen reproducibility, standalone
index/Python suites, release/process-death/reboot/tablet/dark and physical arm64 were
not rerun. No download/escalation was needed.


## Format-independent import reference checks

Verified 2026-10-26 with the Rust PATH/JDK setup above:

```sh
cargo test --workspace --all-features --locked --offline --quiet
cargo clippy --workspace --all-targets --all-features --locked --offline -- -D warnings
cargo fmt --all -- --check
tools/test-sudachi.sh
tools/test-kotlin-bridge.sh
tools/format-kotlin.sh --check
git diff --check
```

The existing two-ABI `cargo check -p yomoku-storage` command above also passes
with NDK r30/API 29 and bundled SQLite.

PASS: 395 data-free Rust tests (two new FIFO fixtures), strict Clippy/Rust+Kotlin
formatting/diff, both Android storage targets, fourteen real-Full analysis plus
one worker fixture for each owned/mapped loader, existing real-Full Kotlin/JNI
and seven Unicode exchanges. Sixteen data-dependent fixtures remain ignored in
the ordinary workspace run. Existing benign Crux close diagnostics persist.

New worker tests seed an actual stored two-chapter EPUB, then submit identical
bytes through separate fresh TXT/SRT workers with unusable analyzer loaders and
blank filename/wrong encoding. Each returns the complete book's Library reference
with renamed title/later-chapter progress/permanent finish intact, no Full or writes
and no automatic reader. Corrupt saved EPUB metadata fails before Full; a later
independent TXT still imports. Expanded repository fixtures exercise all three
format adapters, read-only preflight and all existing EPUB corruption schedules.
Existing duplicate/concurrency/cancellation/rollback and Full fixtures now assert
Library references instead of single-text import payloads.

No open failure/blocker, migration, production dependency/version, permission or
exported event/request/view schema change. Runtime fixtures reuse the already pinned
ZIP dev dependency; no new lockfile package/download/escalation. Android app remains
paste/TXT. App/worker/typed JNI reader migration and EPUB FIFO/import wiring remain
open. Complete EPUB integrity reads repeat per reference; performance is unmeasured.
APK/Android lint/device, codegen-repeat, standalone index/Python suites, release,
process-death/reboot/tablet/dark and physical arm64 were not rerun.

## Ordered application/FIFO/typed JNI readers

Verified 2026-10-26 with the Rust PATH/JDK/SDK/Gradle setup above:

```sh
cargo test --workspace --all-features --locked --offline --quiet
cargo clippy --workspace --all-targets --all-features --locked --offline -- -D warnings
cargo fmt --all -- --check
tools/test-kotlin-bridge.sh
tools/format-kotlin.sh --check
cargo run -p yomoku-ffi --bin generate-kotlin --features facet_typegen --locked --offline -- target/generated/kotlin-repeat
diff -r target/generated/kotlin target/generated/kotlin-repeat
(cd crates/yomoku-ffi && CARGO_NET_OFFLINE=true ../../.local/bin/boltffi generate kotlin --deny-skipped --output ../../target/generated/native-repeat)
diff -r target/generated/native target/generated/native-repeat
./android/gradlew -p android :app:assembleDebug :app:assembleDebugAndroidTest :app:lintDebug --offline --console=plain
./android/gradlew -p android :app:connectedDebugAndroidTest :app:lintDebug -Pandroid.testInstrumentationRunnerArguments.class=app.yomoku.DocumentSessionTest,app.yomoku.ReaderNavigationTest,app.yomoku.ReaderClockSessionTest --offline --console=plain
git diff --check
```

PASS: 399 data-free Rust tests (one new core/three worker), strict Clippy/Rust and
Kotlin formatting/diff, real-Full Kotlin/JNI, seven Unicode exchanges with an added
seven-chapter ordered-document round trip, repeatable generated codecs/native
bindings and both-ABI debug/test APK builds with strict Android lint. Sixteen
external-data fixtures remain ignored by the ordinary Rust run. Existing benign
Crux close diagnostics persist. All 13 focused API-36 x86_64 emulator tests pass
(three DocumentSession, five ReaderNavigation, five ReaderClockSession), including
persistence/statuses/TXT encodings, controls/swipes/backward progress/scroll reset,
completion retry/rereads/checkmark/reopen/recreation/toasts/confetti and timer
restore/idle/reflow/movement/teardown/final-save ordering. Reports retained under
`.local/test-reports/ordered-reader/{report,results}/`.

The existing AVD was cold-started with documented swangle/KVM flags without wiping
data. ADB initially could not create its sandbox socket; escalation connects and
the focused Gradle run passes. The AVD remains running. No open failure/blocker.

New worker fixtures seed actual stored two-chapter EPUBs, then open/move/complete
through the production FIFO/core with unusable analyzer loaders. Verify original
per-chapter Mode B selection, tokens, whole-book counters/progress, Previous without
history, rereads, all five reflow presets with exact later-chapter offset, separate
123/0/456 ms completion durations, whole-book-only finish/confetti and fresh-worker
finished reset. Image-only final content uses U+FFFC and completes with no words.
Corrupt saved EPUB metadata fails without chapter truncation or Full; an independent
reader then opens. The core rejects missing/duplicate/foreign analysis keys, accepts
out-of-order keyed entries and preserves explicit per-chapter Missing.

Initial synthetic fixture failures were wrong time-column naming, treating an
EPUB image placeholder as empty text and asserting Library contents without first
loading Library. Corrected the fixtures; production empty-text semantics stay
unchanged. Kotlin -Werror caught an unnecessary safe call in the new codec fixture;
fixed it. Final host checks pass. No new dependency/version, migration or permission.

App picker remains paste/TXT; EPUB per-file FIFO import, mixed selection and reader
formatting/ruby/images remain open. Complete EPUB integrity reads still repeat per
reader operation; large-book performance is unmeasured. Standalone Full/index/Python,
release, process-death/reboot, tablet/dark and physical arm64 were not rerun.

## EPUB per-file FIFO and typed JNI imports

Run 2026-10-26 with the Rust PATH/JDK/SDK/Gradle setup above:

```sh
cargo test --workspace --all-features --locked --offline --quiet
cargo test -p yomoku-runtime --test imports --all-features --locked --offline --quiet
cargo clippy --workspace --all-targets --all-features --locked --offline -- -D warnings
cargo fmt --all -- --check
tools/test-kotlin-bridge.sh
tools/format-kotlin.sh --check
cargo run -p yomoku-ffi --bin generate-kotlin --features facet_typegen --locked --offline -- target/generated/kotlin-repeat
diff -r target/generated/kotlin target/generated/kotlin-repeat
(cd crates/yomoku-ffi && CARGO_NET_OFFLINE=true ../../.local/bin/boltffi generate kotlin --deny-skipped --output ../../target/generated/native-repeat)
diff -r target/generated/native target/generated/native-repeat
./android/gradlew -p android :app:assembleDebug :app:assembleDebugAndroidTest :app:lintDebug --offline --console=plain
./android/gradlew -p android :app:connectedDebugAndroidTest :app:lintDebug -Pandroid.testInstrumentationRunnerArguments.class=app.yomoku.DocumentSessionTest,app.yomoku.ReaderClockSessionTest --offline --console=plain
git diff --check
```

PASS: 409 data-free Rust tests (two new core/eight worker), strict Clippy/Rust and
Kotlin formatting/diff, both-ABI debug/test APK builds and strict Android lint.
PASS: real-Full Kotlin/JNI (new EPUB import/reopen/status/duplicate/cancellation
fixtures plus existing TXT/SRT/dictionary/lifecycle suites) and seven Unicode
exchanges. All eight API-36 x86_64 emulator regressions pass (three DocumentSession,
five ReaderClockSession). Reports/logs: `.local/test-reports/epub-worker/`.
Repeated Crux/native generation matches exactly. Existing benign Crux close diagnostics persist. No open failure/blocker.
Sixteen external-data fixtures remain ignored in ordinary workspace tests.
The worker suite also passes after sharing the archive fixture at its test root.

New fixtures import actual two-chapter EPUBs through the production FIFO/core;
assert complete stored bytes/metadata/assets/formatting/ruby, original chapter-local
Mode B, immediate unique registration, unchanged existing status and no encounters.
Whole-book Library summaries never auto-open a reader. Invalid archives/XML and
blank names fail before Full; filename fallback works. Exact-byte EPUB/TXT/SRT
preflight preserves the complete first interpretation (including renamed title,
later-chapter progress and finished marker), skips Full/name/container/encoding
validation and performs no writes. Corrupt saved content fails without truncation.

Loader/later-chapter engine/invalid-analysis failures, each EPUB write stage,
later-chapter SQL and deferred-COMMIT failures roll back the entire current file,
registrations and provisional IDs. Previously committed files survive and a later
file/retry succeeds. Second-chapter cancellation bypasses FIFO, rolls back earlier
chapters/assets/registration, ignores rejected busy events and permits a fresh
retry. Queued and closed-router cancellation also pass. No dependency/version,
migration or new platform permission.

Initial fixtures needed Library refresh on a fresh worker, one shared archive
module for strict Clippy, generated unsigned Kotlin counters and removal of the
expected-text fixture's final newline. Corrected them; no production parser change.
The initial Android build hit Gradle's sandbox wildcard-IP/lock-service restriction;
the approved outside-sandbox offline build passes.

Android picker remains paste/TXT; mixed EPUB/TXT/SRT selection/notifications and
reader formatting/ruby/images remain open. Complete saved EPUB verification repeats
per operation and container preflight parses twice; large-book costs are unmeasured.
Standalone Full/index/Python suites, release app, process-death/reboot/tablet/dark
and physical arm64 were not rerun.


## Mixed EPUB/TXT/SRT Android batches

Verified 2026-10-26 with the Rust PATH/JDK/SDK/Gradle environment above:

```sh
cargo test --workspace --all-features --locked --offline --quiet
tools/format-kotlin.sh --check
./android/gradlew -p android :app:assembleDebug :app:assembleDebugAndroidTest :app:lintDebug --offline --console=plain
./android/gradlew -p android :app:connectedDebugAndroidTest :app:lintDebug -Pandroid.testInstrumentationRunnerArguments.class=app.yomoku.MixedFilePickerTest,app.yomoku.TxtPickerTest,app.yomoku.ImportNotificationsTest,app.yomoku.TxtCancellationSessionTest,app.yomoku.NavigationTest --offline --console=plain
./android/gradlew -p android :app:connectedDebugAndroidTest :app:lintDebug -Pandroid.testInstrumentationRunnerArguments.class=app.yomoku.MixedFilePickerTest --offline --console=plain
git diff --check
git diff --cached --check
```

PASS: 409 data-free Rust tests (sixteen external-data fixtures ignored), Kotlin
formatting, both-ABI debug/test APK builds and strict Android lint. All eleven
distinct API-36 x86_64 focused tests pass across the initial regression run (ten
passed) and the corrected mixed-picker rerun (one passed). The initial mixed test
used a text selector matching both Library error and modal error; assert the unique
encoding control instead. The rerun also checks all saved chapter Mode B analyses,
immediate registration of every Japanese surface and no reading completions. No
production fix was needed after the initial build. Reports/results retained under
`.local/test-reports/mixed-file-picker/{initial,final}/`.

The mixed picker fixture verifies metadata EPUB titles/two persisted chapters,
case-insensitive extensions, identical EPUB bytes selected as TXT referencing the
whole first interpretation, SRT timestamp/tag removal and cue breaks, invalid EPUB,
undecodable SRT, excluded ASS rejection, explicit SRT UTF-8/Shift-JIS/Skip, later TXT
continuation, exact saved encodings/source kinds, recreation/summary dismissal and
no reader auto-open. The fixture provider labels even EPUB/SRT as text/plain.
Existing TXT fixtures retain read/decode/unsupported/duplicate failures and progress,
blocked-reader cancellation/generation disposal and fresh retries.

Notification tests now exercise an EPUB/SRT/TXT batch continuing with Activity
stopped, progress/completion/tap, SRT encoding lease release and timeout cancellation
of an EPUB URI read. Native cancellation tests exercise SRT blocked-Full rollback and
fresh retry, and a winning EPUB COMMIT while direct cancellation/native-close await
their return; remaining files never import. Navigation/recreation regressions pass.
The AVD was cold-started with documented swangle/KVM flags, without wiping data,
and remains running. Gradle device runs required approved outside-sandbox execution
because the sandbox blocks its wildcard-IP lock service. No open failure/blocker.

No Rust/wire/schema/dependency/version/permission change. Host JNI, standalone Full,
Clippy/Rust fmt/codegen, index/Python, release, actual process-death/reboot, tablet/dark
and physical arm64 were not rerun for this Android-only change; prior checks remain
recorded above. No persisted queue or process-survival guarantee was added. EPUB
reader formatting/ruby/images/external links/viewer and CI remain next work. Complete
EPUB integrity reads and duplicate container parsing retain their unmeasured costs.

## EPUB reader text formatting and website links

Verified 2026-10-26 using the Rust PATH/JDK/SDK/Gradle environment above:

```sh
cargo test --workspace --all-features --locked --offline --quiet
cargo clippy --workspace --all-targets --all-features --locked --offline -- -D warnings
cargo fmt --all -- --check
tools/test-kotlin-bridge.sh
tools/format-kotlin.sh --check
cargo run -p yomoku-ffi --bin generate-kotlin --features facet_typegen --locked --offline -- target/generated/kotlin-repeat
diff -r target/generated/kotlin target/generated/kotlin-repeat
./android/gradlew -p android :app:assembleDebug :app:assembleDebugAndroidTest :app:lintDebug --offline --console=plain
./android/gradlew -p android :app:connectedDebugAndroidTest :app:lintDebug -Pandroid.testInstrumentationRunnerArguments.class=app.yomoku.ReaderFormattingTest,app.yomoku.ReaderWordSpansTest,app.yomoku.ReaderNavigationTest --offline --console=plain
./android/gradlew -p android :app:connectedDebugAndroidTest :app:lintDebug -Pandroid.testInstrumentationRunnerArguments.class=app.yomoku.ReaderFormattingTest --offline --console=plain
git diff --check
```

PASS: 412 data-free Rust tests (three new pure formatting tests), strict Clippy,
Rust/Kotlin formatting, real-Full Kotlin/JNI, seven Unicode exchanges including
all four formatting variants, repeatable Kotlin codecs and both-ABI debug/test
APKs with strict Android lint. Sixteen external-data fixtures remain ignored by
the ordinary workspace suite. Existing benign Crux close diagnostics persist.
The repository read-only fixture now checks exact stored heading/emphasis/strong/
website ranges and omission of inert links. Pure tests cover every preset/page,
Unicode/combining/NUL text, crossing spans, exact offsets, chapter ownership and
malformed ranges/targets. JNI verifies actual imported EPUB annotations and
unformatted later chapters alongside all existing import/persistence suites.

All eleven distinct API-36 x86_64 focused tests pass across initial ten + final
corrected single rerun. The new UI test imports an actual EPUB, verifies styled
Compose text, intercepts a real external-browser ACTION_VIEW, taps inert internal/
other links without navigating, recreates, opens dictionary UI, closes it and
moves to the next chapter/back with original text/anchors/styles intact. Two added
word-span tests check overlapping website/dictionary tap targets, preserved status
backgrounds/full token identity, nested links and invalid UTF-8/UTF-16 inputs.

Initial compile needed the new Facet enum's explicit repr(C). Initial Android lint
rejected Uri.parse in favor of the existing KTX toUri helper; fixed. The first UI
run timed out after calling the Activity Back dispatcher rather than the dialog's
Close button. An initial wait-only test correction confirmed it had closed the
reader; using the actual sheet Close control and waiting for lookup readiness
passes. No production reader fix was needed for these test failures. Reports/logs:
`.local/test-reports/reader-formatting/{initial,dialog-test-failure,final}/`.
Gradle's sandbox wildcard-IP lock service failed; approved outside-sandbox offline
build/device runs pass. ADB log reads also needed its outside-sandbox local socket.
No open blocker, dependency/version, migration or permission change.

Source ruby/images, cover cache, image viewer and font/furigana settings remain
open. Complete EPUB integrity reads still repeat per repository operation; large-
book costs are unmeasured. Standalone Full/index/Python suites, native-binding
repeat generation, release/process-death/reboot/tablet/dark and physical arm64 were
not rerun. The generated payload codecs were repeat-checked; native ABI signatures
are unchanged. Physical arm64 validation remains deferred by the user.

## EPUB reader image reference transport

Verified 2026-10-26 using the Rust PATH/JDK/SDK/Gradle environment above:

```sh
cargo test -p yomoku-core --test reader_images --all-features --locked --offline
cargo test --workspace --all-features --locked --offline --quiet
cargo test -p yomoku-runtime --test imports --all-features --locked --offline --quiet
cargo clippy --workspace --all-targets --all-features --locked --offline -- -D warnings
cargo fmt --all -- --check
tools/test-kotlin-bridge.sh
tools/format-kotlin.sh --check
cargo run -p yomoku-ffi --bin generate-kotlin --features facet_typegen --locked --offline -- target/generated/kotlin-repeat
diff -r target/generated/kotlin target/generated/kotlin-repeat
./android/gradlew -p android :app:assembleDebug :app:assembleDebugAndroidTest :app:lintDebug --offline --console=plain
./android/gradlew -p android :app:connectedDebugAndroidTest -Pandroid.testInstrumentationRunnerArguments.class=app.yomoku.DocumentSessionTest,app.yomoku.ReaderFormattingTest,app.yomoku.ReaderWordSpansTest,app.yomoku.ReaderNavigationTest --offline --console=plain
git diff --check
```

PASS: 417 data-free Rust tests (four new pure image tests/one new repository test),
strict Clippy/Rust+Kotlin formatting/diff, real-Full Kotlin/JNI, Unicode image
document/page round trips, repeatable generated Kotlin payloads and both-ABI
debug/test APKs with strict Android lint. Sixteen external-data fixtures remain
ignored in the ordinary workspace run. Existing benign Crux close diagnostics
persist in host JNI tests. One Kotlin Android ReaderPage fixture needed its new
empty image list; no production UI behavior changed.

All 14 API-36 x86_64 emulator regressions pass (three DocumentSession, one
ReaderFormatting, five ReaderWordSpans, five ReaderNavigation). These retain actual
SQLite/JNI persistence/statuses/TXT encodings, website browser launches/inert links,
dictionary interactions, recreation, page controls/swipes/scroll reset/rereads and
completion retries/toasts/confetti. Reports/results and lint reports are retained
under `.local/test-reports/reader-image-references/`. The existing AVD was cold-
started with documented swangle/KVM flags, without wiping data, and remains running.
The APK/lint build passed in the sandbox; the device command failed Gradle's
wildcard-IP/lock-service socket check there, then passed on the approved outside-
sandbox retry. No open failure/blocker.

Pure fixtures cover source order, adjacent/shared images, image-only chapters,
all five presets/25 reflow pairs, exact scalar positions/EOF and rejection of
foreign/duplicate keys, overlap, invalid ranges/paths/media and inconsistent shared
metadata. Repository fixtures check verified stored references/alt/media, read-only
reopen, shared assets across chapters and unchanged registration/history/position.
Extended corruption fixtures reject damaged image ranges and non-image asset
metadata without writes. FIFO regression retains images on Previous and omits them
from unrelated chapters/reflow. Actual imported EPUB JNI snapshots/pages expose
the same metadata; a separate Unicode exchange includes emoji, supplementary kanji,
combining marks and NUL in surrounding text/alt, with exact UTF-8 byte ranges.

Image bytes/dimensions, bounded off-main asset retrieval/decoding, source-position/
ratio rendering, cover cache and zoom/pan viewer remain open. No dependency/version,
migration, permission or data-destructive behavior change. Complete saved EPUB
integrity reads repeat per operation; large-book costs remain unmeasured. Standalone
Full/index/Python, native-binding repeat generation, release/process-death/reboot,
tablet/dark and physical arm64 were not rerun. Native ABI signatures are unchanged;
generated payload codecs were repeat-checked. Physical arm64 remains user-deferred.

## Bounded EPUB image asset retrieval

Verified 2026-10-26 using the Rust PATH/JDK/SDK/Gradle environment above:

```sh
cargo test -p yomoku-storage epub_asset --locked --offline
cargo test --workspace --all-features --locked --offline --quiet
cargo clippy --workspace --all-targets --all-features --locked --offline -- -D warnings
cargo fmt --all -- --check
tools/test-kotlin-bridge.sh
tools/format-kotlin.sh --check
.local/jdk-17/jdk-17.0.21+9/bin/java -jar .local/ktfmt-0.64.jar --kotlinlang-style --dry-run --set-exit-if-changed tests/kotlin/EpubAssetBridgeTest.kt
(cd crates/yomoku-ffi && CARGO_NET_OFFLINE=true ../../.local/bin/boltffi generate kotlin --deny-skipped --output ../../target/generated/native-repeat)
diff -r target/generated/native target/generated/native-repeat
./android/gradlew -p android :app:assembleDebug :app:assembleDebugAndroidTest :app:lintDebug --offline --console=plain
./android/gradlew -p android :app:connectedDebugAndroidTest -Pandroid.testInstrumentationRunnerArguments.class=app.yomoku.EpubAssetBridgeTest,app.yomoku.DocumentSessionTest,app.yomoku.ReaderFormattingTest --offline --console=plain
git diff --check
```

PASS: 425 data-free Rust tests (four new repository, three worker and one native
test), strict Clippy/Rust+Kotlin formatting/diff, real-Full Kotlin/JNI and seven
Unicode exchanges, repeatable native bindings, both-ABI debug/test APKs and strict
Android lint. Sixteen external-data fixtures remain ignored by the ordinary
workspace suite. Existing benign Crux close diagnostics remain in host JNI tests.
Generated payload codecs are unchanged; the native API adds a one-shot asset
listener/error and asynchronous request. No dependency/version or schema change.

Repository fixtures verify exact inline/shared/cover bytes, opaque Unicode keys,
media/digest corruption, absent/deleted/non-EPUB identities, deletion/reimport,
read-only reopen and no history/position writes. Tests accept exactly 32 MiB and
reject a larger BLOB, then repair fixture data and retry. A narrow asset read
intentionally ignores unrelated BLOB/source corruption; ordinary document loads
retain full integrity checks. Worker fixtures verify no analyzer/index load or
Render/state change, queued import-before-read, one native thread, lazy-open
failure/retry, invalid-key rejection before opening and accepted reads draining
after router closure. Native tests verify invalid requests fail synchronously and
one-shot listeners release on both rejected and delivered requests.

Host JNI exercises actual imported Unicode asset keys/cover bytes, error objects,
reopen without Full, unchanged views and a queued asset callback after bridge
close. All five API-36 x86_64 tests pass: the new asset test submits from Android
main and checks off-main callbacks, all 256 byte values, analyzer-free reopen,
media/absent-ID errors and persistent corruption without repairs/state changes;
three document/session and one EPUB formatting/browser/recreation regressions pass.
Logs, device reports/results and lint reports: `.local/test-reports/epub-image-assets/`.

Fixed test-only assumptions: a closed core suppresses Render callbacks even as
queued asset reads drain; native error objects use PascalCase; JNI may create
different Java Thread wrappers for callbacks on the same Rust FIFO thread. The
Rust fixture still verifies exact native thread identity; JNI checks off-caller
delivery. Corrected suites pass. Gradle's sandbox wildcard-IP/lock-service socket
failed; approved outside-sandbox offline build/device runs pass. The existing
AVD remained running without a wipe. No open failure/blocker.

No production Android image caller/decoder, dimensions, rendering, cache or viewer
is added. Next: bounded decoding off the FIFO callback, stale-request disposal and
source-position/ratio rendering, then covers/shared viewer. No destructive
semantics or permission change. Complete reader snapshot integrity reads still
repeat; large-book costs remain unmeasured. Standalone Full/index/Python fixtures,
release, process-death/reboot, tablet/dark and physical arm64 were not rerun;
physical arm64 remains user-deferred.


## Bounded inline EPUB raster decoding and rendering

2026-10-26; use the Rust PATH/JDK/SDK/Gradle environment above. Commands:

```sh
tools/format-kotlin.sh --check
git diff --check
./android/gradlew -p android :app:assembleDebug :app:assembleDebugAndroidTest :app:lintDebug --offline --console=plain
./android/gradlew -p android :app:connectedDebugAndroidTest :app:lintDebug -Pandroid.testInstrumentationRunnerArguments.class=app.yomoku.EpubImageDecodeTest,app.yomoku.EpubImageLoaderTest,app.yomoku.ReaderImagesTest,app.yomoku.ReaderFormattingTest,app.yomoku.ReaderNavigationTest,app.yomoku.EpubAssetBridgeTest --offline --console=plain
./android/gradlew -p android :app:assembleDebug :app:assembleDebugAndroidTest :app:lintDebug :app:connectedDebugAndroidTest -Pandroid.testInstrumentationRunnerArguments.class=app.yomoku.EpubImageDecodeTest,app.yomoku.EpubImageLoaderTest,app.yomoku.ReaderImagesTest --offline --console=plain
./android/gradlew -p android :app:connectedDebugAndroidTest :app:lintDebug -Pandroid.testInstrumentationRunnerArguments.class=app.yomoku.EpubImageDecodeTest,app.yomoku.EpubImageLoaderTest,app.yomoku.ReaderImagesTest,app.yomoku.ReaderFormattingTest,app.yomoku.ReaderWordsTest,app.yomoku.ReaderWordSpansTest --offline --console=plain
```

The initial 13-test API-36 x86_64 run passed (six new image cases, formatting,
asset JNI and five navigation/completion regressions). The strengthened six-test
run caught a real neighboring-link hit-area issue: tapping inert text beside a
short website link launched the browser through Compose's expanded target. A
local text-only ViewConfiguration disables that expansion while preserving link
semantics and normal targets elsewhere. The final expanded 20-test run passed,
including all six image cases, formatting and thirteen word/span regressions
(26 distinct passing cases across both successful runs).
Both-ABI debug/test APK packaging, strict lint and Kotlin formatting/diff passed.
Reports/results: `.local/test-reports/epub-image-rendering/`.

Decoder fixtures check PNG/JPEG, source dimensions versus sampled rounding,
pixel/edge budgets, one-pixel sampling, CRC-valid excessive dimensions, malformed,
empty, SVG and >32 MiB data. Controlled callback/decode fixtures verify off-main
decoding, main delivery, one read/decode at a time, queue cancellation, late
results/close, mismatched media/errors/explicit retry and safe unpublished-bitmap
recycling. UTF-8/UTF-16 fixtures include emoji, supplementary kanji, adjacent and
image-only placements and malformed/overlapping ranges. Actual EPUB Compose tests
check source order, visible image pixels, source ratio, shared assets, clipping of
strong/link annotations, browser launch versus adjacent inert text, dictionary
identity, recreation, chapter navigation, image-only chapters, reflow and
unsupported-image fallback/retry without moving position.

Gradle/adb sandbox sockets failed; approved outside-sandbox offline commands
accessed the emulator; both final suites passed. Started the existing AVD with
documented swangle/KVM flags without wiping data. No dependency/version, native
payload/API, schema, permission or destructive-semantics changes. Rust/host JNI/
binding-repeat/standalone Full/index/Python, release, process-death/reboot,
tablet/dark and physical arm64 were not rerun for this Android-only slice;
physical arm64 remains user-deferred. SVG rendering, cover caching, viewer and
source ruby remain open. Ordinary reader operations still repeat complete EPUB
integrity reads; large-book costs and total process heap remain unmeasured.

## Shared full-screen image zoom/pan viewer

Verified 2026-10-26. Use the Rust PATH/JDK/SDK/Gradle environment above. Commands:

```sh
tools/format-kotlin.sh --check
git diff --check
./android/gradlew -p android :app:assembleDebug :app:assembleDebugAndroidTest :app:lintDebug :app:connectedDebugAndroidTest -Pandroid.testInstrumentationRunnerArguments.class=app.yomoku.ImageViewerGeometryTest,app.yomoku.ImageViewerTest,app.yomoku.ReaderImagesTest,app.yomoku.ReaderFormattingTest,app.yomoku.ReaderNavigationTest --offline --console=plain
./android/gradlew -p android :app:assembleDebug :app:assembleDebugAndroidTest :app:lintDebug :app:connectedDebugAndroidTest -Pandroid.testInstrumentationRunnerArguments.class=app.yomoku.ImageViewerGeometryTest,app.yomoku.ImageViewerTest --offline --console=plain
# Repeat the six-test command above between these reversible tablet/dark overrides.
/home/kai/Android/Sdk/platform-tools/adb -s emulator-5554 shell wm size 2560x1600
/home/kai/Android/Sdk/platform-tools/adb -s emulator-5554 shell wm density 240
/home/kai/Android/Sdk/platform-tools/adb -s emulator-5554 shell cmd uimode night yes
# Run the six-test command above, then restore the original phone/light settings.
/home/kai/Android/Sdk/platform-tools/adb -s emulator-5554 shell wm size reset
/home/kai/Android/Sdk/platform-tools/adb -s emulator-5554 shell wm density reset
/home/kai/Android/Sdk/platform-tools/adb -s emulator-5554 shell cmd uimode night no
/home/kai/Android/Sdk/platform-tools/adb -s emulator-5554 shell wm user-rotation
```

PASS: both-ABI debug/test APKs, strict Android lint, Kotlin formatting/diff, six
new image-viewer tests on API-36 x86_64 phone/light and tablet/dark (six cases in
each final run). Eight existing reader-image/formatting/navigation regressions
also passed in the initial broader run: 14 distinct passing cases overall.
Reports/results: `.local/test-reports/image-viewer/{initial-phone,inset-measurement,phone-final,tablet-dark}/`;
final lint XML is alongside them. The existing unrelated FileImports autoboxing
hint remains informational; no lint warning/error blocks the build.

Geometry cases verify source aspect/letterboxing/extreme tall images, centroid
anchoring, bounds on both pan axes, 1–8x zoom, invalid gestures and normalized focus
through phone/tablet viewport changes. Actual EPUB tests verify full-window bounds,
alt semantics, pinch/drag and blue/red pixels, accessible zoom/reset/close, hardware
Back dismissing only the modal, no browser launch for image taps inside links,
unchanged reader position/statuses/finish and reopening at fit. Rotation tests use
`wm user-rotation lock 0/1` (works with target-36 large screens), restore the original
mode in finally, and verify selection/zoom/panned pixels through actual rotation,
explicit recreation and chapter replacement. The source-agnostic fixture checks
loading/failure/retry, disabled/bounded controls and dialog interaction callbacks.

Two test assumptions were corrected: calling the Activity Back dispatcher directly
bypasses a dialog's separate window (initial 13/14 passed); actual injected Back
works. A strengthened size check initially measured only the safe inset content
(4/6 passed); its tag now measures the full-window black Surface. Final phone
(1m18s) and tablet/dark (1m) six-test runs both pass, with all pixels/rotation checks.
No open verification failure. Gradle's sandbox wildcard-IP/lock sockets and ADB
sockets needed approved outside-sandbox access. The existing AVD was reused without
a wipe and remains running. Original 1080x2400/420dpi, light mode and free rotation
were restored; no dependency/version, schema, native API or permission change.

The viewer reuses the current page's bounded bitmap; zoom does not decode extra
detail. SVG rendering/cover caching/source ruby remain open. No Rust/Clippy/host
JNI/binding repeat/standalone dictionary/Python/release/process-death/reboot or
physical-arm64 checks were rerun for this Android-only slice; physical arm64 remains
user-deferred. Complete EPUB integrity reads and total process-heap costs retain
their earlier limitations.

## Bounded static EPUB SVG images

Verified 2026-10-26 using the Rust PATH/JDK/SDK/Gradle environment above:

```sh
./android/gradlew -p android :app:dependencies --configuration debugRuntimeClasspath --console=plain
tools/format-kotlin.sh --check
git diff --check
./android/gradlew -p android :app:assembleDebug :app:assembleDebugAndroidTest :app:lintDebug --console=plain
./android/gradlew -p android :app:assembleDebug :app:assembleDebugAndroidTest :app:lintDebug :app:connectedDebugAndroidTest -Pandroid.testInstrumentationRunnerArguments.class=app.yomoku.EpubSvgDecodeTest,app.yomoku.EpubImageDecodeTest,app.yomoku.EpubImageLoaderTest,app.yomoku.ReaderImagesTest,app.yomoku.ImageViewerGeometryTest,app.yomoku.ImageViewerTest,app.yomoku.ReaderFormattingTest,app.yomoku.ReaderNavigationTest --offline --console=plain
# Final additional inherited-paint guard: rebuild/lint and rerun the SVG fixtures.
./android/gradlew -p android :app:assembleDebug :app:assembleDebugAndroidTest :app:lintDebug :app:connectedDebugAndroidTest -Pandroid.testInstrumentationRunnerArguments.class=app.yomoku.EpubSvgDecodeTest --offline --console=plain
/home/kai/Android/Sdk/platform-tools/adb -s emulator-5554 shell wm size 2560x1600
/home/kai/Android/Sdk/platform-tools/adb -s emulator-5554 shell wm density 240
/home/kai/Android/Sdk/platform-tools/adb -s emulator-5554 shell cmd uimode night yes
./android/gradlew -p android :app:connectedDebugAndroidTest -Pandroid.testInstrumentationRunnerArguments.class=app.yomoku.EpubSvgDecodeTest,app.yomoku.ReaderImagesTest,app.yomoku.ImageViewerGeometryTest,app.yomoku.ImageViewerTest --offline --console=plain
/home/kai/Android/Sdk/platform-tools/adb -s emulator-5554 shell wm size reset
/home/kai/Android/Sdk/platform-tools/adb -s emulator-5554 shell wm density reset
/home/kai/Android/Sdk/platform-tools/adb -s emulator-5554 shell cmd uimode night no
/home/kai/Android/Sdk/platform-tools/adb -s emulator-5554 shell wm user-rotation
```

PASS: both-ABI debug/test APKs, strict Android lint, Kotlin formatting/diff, 21
API-36 x86_64 phone/light tests (5m55s), final three SVG fixtures after adding the
inherited-paint fallback guard (17s including build/lint), and 11 tablet/dark cases
(1m44s). The three SVG cases are new; reader-image/geometry fixtures now cover SVG,
fractional source sizes and offline notices. The broader phone run retains raster,
loader cancellation/serialization, viewer, website formatting and navigation/
completion regressions. Reports/logs: `.local/test-reports/epub-svg/{phone,final-phone,tablet-dark}/`.

Fixtures verify actual blue/gradient/transparent pixels, paths/transforms/local
uses/color CSS, UTF-16 XML, physical units/viewBox/fractional sizes, byte/dimension/
pixel/edge bounds down to one pixel and wrong media types. Rejected inputs include
DTD/entities, stylesheet imports, nested/data/file/remote image resources,
malformed XML, duplicate/missing references, direct/indirect/unclosed-IRI cycles,
deep/large/exponential graphs and inherited pattern paint. A later valid decode
still succeeds. Actual imported EPUB tests verify source order/ratio, repeated
asset identity, style/word taps, external browser versus inert internal text,
SVG/EPUB image anchors opening only the viewer, zoomed pixels, recreation/reflow/
chapter disposal, image-only completion and full offline credits after recreation.
Viewer rotation/Back/pan/idle/retry regressions pass in both display configurations.

AndroidSVG 1.4 AAR/source APIs, embedded-raster behavior and entity defaults were
reviewed against upstream documentation and the published source archive before
adoption. Maven POM/dependency report confirms no transitive dependencies; exact
artifact/source SHA-256 and license provenance are in `third_party/androidsvg/`.
APK inspection confirms both native ABIs and the complete bundled SVG notices.

Initial strict lint flagged platform `Bitmap.createBitmap` (UseKtx); switched to
the already available KTX API, and final lint passes. The existing unrelated
FileImports autoboxing hint remains informational. Gradle's sandbox wildcard-IP
socket failed; approved outside-sandbox runs/downloads pass. The emulator was
disconnected; restarted the existing AVD with documented swangle/KVM flags without
wiping data. One install attempt raced boot and ran zero tests; after checking
`sys.boot_completed=1`, the complete suite passed. No open verification blocker.
Restored/verified 1080x2400, 420dpi, light mode and free rotation; AVD remains running.

Only bounded static SVG is delivered. Nested image resources, patterns/masks/
markers and resource-valued stylesheet rules retain alt/Retry until bounded
resource rendering exists; cover caching and source ruby remain open. Canvas may
allocate temporary layers beyond the retained page pixel budget; no total heap/
CPU or large-book benchmark claim. No core/FFI/schema/permission change. Rust/
Clippy/host JNI/bindings, standalone dictionaries/Python, release, process-death/
reboot and physical arm64 were not rerun; arm64 remains user-deferred.

## Metadata-only EPUB cover references

Verified 2026-10-26 using the Rust PATH/JDK/SDK/Gradle environment above:

```sh
cargo test --workspace --all-features --locked --offline --quiet
cargo clippy --workspace --all-targets --all-features --locked --offline -- -D warnings
cargo fmt --all -- --check
tools/test-kotlin-bridge.sh
(cd crates/yomoku-ffi && CARGO_NET_OFFLINE=true ../../.local/bin/boltffi generate kotlin --deny-skipped --output ../../target/generated/native-cover-repeat)
diff -r target/generated/native target/generated/native-cover-repeat
tools/format-kotlin.sh --check
.local/jdk-17/jdk-17.0.21+9/bin/java -jar .local/ktfmt-0.64.jar --kotlinlang-style --dry-run --set-exit-if-changed tests/kotlin/EpubAssetBridgeTest.kt
./android/gradlew -p android :app:assembleDebug :app:assembleDebugAndroidTest :app:lintDebug --offline --console=plain
./android/gradlew -p android :app:connectedDebugAndroidTest -Pandroid.testInstrumentationRunnerArguments.class=app.yomoku.EpubAssetBridgeTest,app.yomoku.DocumentSessionTest,app.yomoku.ReaderFormattingTest --offline --console=plain
git diff --check
```

PASS: 432 data-free Rust tests (seven new: three repository/three FIFO/one native),
strict Clippy, Rust/Kotlin formatting and diff checks, complete real-Full host JNI
suite and seven Unicode exchanges, repeatable native bindings, both-ABI debug/test
APKs and strict Android lint. Five API-36 x86_64 phone regressions pass (1m41s).
The normal Rust run still skips sixteen external-data tests; host JNI uses the
prepared pinned Full/index. APK inspection confirms arm64-v8a/x86_64 JNI libraries.
Reports/logs: `.local/test-reports/epub-cover-reference/`.

New checks cover stored path/media/32-byte digest including Unicode paths,
read-only reopen/unchanged DB, no cover/non-EPUB/absent/deleted IDs, fresh reimport
IDs, missing metadata/assets, malformed digest/media and empty/over-budget images.
Corrupt source/chapter/image BLOBs do not cause metadata-only reads to load/verify
book contents; separate asset reads still reject bad bytes. Worker tests verify
no dictionaries/Render/state/history changes, FIFO after blocked import, accepted
callbacks draining after close, closed submission rejection and failed-open retry.
Native tests verify invalid IDs fail before enqueue and one-shot listeners drop;
host/Android JNI verifies actual references/digests, absence, corruption/retry,
unchanged view and off-main callbacks. Document persistence and EPUB style/word/
external-browser/inert-internal-link/recreation regressions pass.

Initial Rust checks caught a missing test import and a corruption fixture blocked
by schema CHECK constraints; fixed the import and explicitly bypass CHECK only in
that corruption fixture. Final full suite/Clippy pass. Sandbox Gradle sockets fail
with unusable wildcard IP; approved outside-sandbox offline build/tests pass.
The existing booted AVD was reused without overrides/wipe. No open blocker.

No core payload, dependency, schema or permission change. The typed native API is
additive. A returned digest advertises metadata; future new thumbnail cache entries
must fetch hash-verified bytes through the existing asset API before bounded
decoding/publication. Thumbnail cache/Library display, remaining SVG resources and
source ruby stay open. No release, tablet/dark, process-death/reboot, standalone
dictionary benchmarks or physical-arm64 checks were rerun; arm64 remains deferred.


## Bounded Library EPUB cover thumbnails

Verified 2026-10-26 using the Rust PATH/JDK/SDK/Gradle environment above:

```sh
tools/format-kotlin.sh --check
git diff --check
./android/gradlew -p android :app:assembleDebug :app:assembleDebugAndroidTest :app:lintDebug :app:connectedDebugAndroidTest -Pandroid.testInstrumentationRunnerArguments.class=app.yomoku.EpubCoverLoaderTest,app.yomoku.LibraryCoversTest,app.yomoku.EpubImageLoaderTest,app.yomoku.EpubImageDecodeTest,app.yomoku.ReaderImagesTest,app.yomoku.NavigationTest --offline --console=plain
# Final six-test rerun after correcting the offset assertion; also repeat for tablet/dark.
./android/gradlew -p android :app:assembleDebug :app:assembleDebugAndroidTest :app:lintDebug :app:connectedDebugAndroidTest -Pandroid.testInstrumentationRunnerArguments.class=app.yomoku.EpubCoverLoaderTest,app.yomoku.LibraryCoversTest --offline --console=plain
/home/kai/Android/Sdk/platform-tools/adb -s emulator-5554 shell wm size 2560x1600
/home/kai/Android/Sdk/platform-tools/adb -s emulator-5554 shell wm density 240
/home/kai/Android/Sdk/platform-tools/adb -s emulator-5554 shell cmd uimode night yes
# Repeat the six-test command, then restore the original phone/light settings.
/home/kai/Android/Sdk/platform-tools/adb -s emulator-5554 shell wm size reset
/home/kai/Android/Sdk/platform-tools/adb -s emulator-5554 shell wm density reset
/home/kai/Android/Sdk/platform-tools/adb -s emulator-5554 shell cmd uimode night no
```

PASS: both-ABI debug/test APKs, strict Android lint, Kotlin formatting/diff, six
new cover tests on API-36 x86_64 phone/light (52s including build/lint) and six on
tablet/dark (51s). All eight existing image decode/loader/reader/navigation
regressions passed in the initial broader run: 14 distinct passing cases overall.
APK inspection confirms arm64-v8a/x86_64 libyomoku_jni.so. Results/reports and lint
XML: `.local/test-reports/epub-cover-thumbnails/{initial-phone,phone-final,tablet-dark}/`
(lint XML is in the parent directory). The existing unrelated FileImports
AutoboxingStateValue hint remains informational; no lint warnings/errors.

Four new cache/pipeline fixtures verify exact SHA-256/media keys, metadata lookup
before hits, cross-document/path sharing without another BLOB, changed bytes
between metadata/asset reads, invalid digest length, no-cover/corruption responses,
retry, byte/entry LRU bounds, replacement/clear without recycling published pixels,
queued/active cancellation, accepted-callback drain and late results after close.
They check main-thread delivery, actual source dimensions and <=64 KiB pixels.
Two real imported EPUB/Compose cases verify PNG and SVG blue pixels/source ratios,
Activity recreation, row opening/return, unchanged progress/finish and generic
fallback for absent/undecodable covers. Existing reader fixtures retain website/
word identity, shared viewer, image-only completion, reflow and chapter navigation.

The first expanded compilation caught use of unavailable DpRect width/height
properties in the new test; switched to edge differences. Gradle/ADB sandbox
sockets needed approved outside-sandbox execution. One attempt found no connected
device and ran zero tests; restarted the existing AVD with documented swangle/KVM
flags, without wiping data, and waited for sys.boot_completed=1. The initial
14-case run passed 13 and caught a test assertion comparing TextOffset to Long;
compare its numeric value instead. Both final six-case runs pass. No open failure
or blocker. Restored/verified 1080x2400, 420dpi, light mode and free rotation; the
AVD remains running.

Cache is transient: <=4 MiB/64 successful entries, <=16,384 pixels per cover,
shared serial encoded-read/decode pipeline with reader images and an off-main
comparison against the frozen metadata digest before new publication. Metadata
hits revalidate live ownership. No negative/disk cache; process restart rebuilds
from saved assets. Visible/evicted bitmaps awaiting GC and temporary decoder
allocations remain outside the retained-cache cap; no total-heap/CPU/large-library
benchmark claim. No core/FFI/schema/dependency/permission/destructive-semantics
change. Rust/Clippy/host JNI/codegen, standalone Full/index/Python, release,
process-death/reboot and physical arm64 were not rerun for this Android-only slice;
arm64 remains user-deferred. Compound SVG resources and source ruby remain open.


## Bounded embedded PNG/JPEG in SVG assets

2026-10-27, using the Rust PATH/JDK/SDK/Gradle environment above:

```sh
tools/format-kotlin.sh --check
git diff --check
./android/gradlew -p android :app:assembleDebug :app:assembleDebugAndroidTest :app:lintDebug :app:connectedDebugAndroidTest -Pandroid.testInstrumentationRunnerArguments.class=app.yomoku.EpubSvgEmbeddedImagesTest,app.yomoku.EpubSvgDecodeTest,app.yomoku.EpubImageDecodeTest,app.yomoku.EpubImageLoaderTest,app.yomoku.ReaderImagesTest,app.yomoku.LibraryCoversTest,app.yomoku.NavigationTest --offline --console=plain
/home/kai/Android/Sdk/platform-tools/adb -s emulator-5554 shell wm size 2560x1600
/home/kai/Android/Sdk/platform-tools/adb -s emulator-5554 shell wm density 240
/home/kai/Android/Sdk/platform-tools/adb -s emulator-5554 shell cmd uimode night yes
# Repeat the same Gradle command for tablet/dark, then restore phone/light.
/home/kai/Android/Sdk/platform-tools/adb -s emulator-5554 shell wm size reset
/home/kai/Android/Sdk/platform-tools/adb -s emulator-5554 shell wm density reset
/home/kai/Android/Sdk/platform-tools/adb -s emulator-5554 shell cmd uimode night no
unzip -l android/app/build/outputs/apk/debug/app-debug.apk 'lib/*/libyomoku_jni.so'
```

PASS: both-ABI debug/test APKs, strict Android lint, Kotlin formatting/diff and all
19 API-36 x86_64 tests on final phone/light (2m17s including build/lint) and
tablet/dark (1m56s); no skipped cases or open blocker. APK inspection confirms
arm64-v8a/x86_64 libyomoku_jni.so. Reports/results/lint XML:
`.local/test-reports/epub-svg-embedded/{phone-final,tablet-dark}/` (initial failure
and preliminary passing phone results are archived alongside). Restored/verified
1080x2400, 420dpi, light mode and free rotation; the AVD remains running.

Five new decoder fixtures cover actual PNG/JPEG pixels, UTF-16 XML, transforms,
clipping, local uses and repeated exact data URIs; aggregate temporary allocation,
2048-edge bounds and one-pixel shares; original-ratio letterboxing/opacity when
64 distinct resources each become 1x1; resolver key isolation and recycling;
malformed base64, mismatched codecs, huge forged PNG headers, missing/ambiguous/
foreign hrefs, image children, nested vectors, file/relative/remote references,
styled embedded SVGs and resource-count/budget failures. Two concurrent decoder
threads repeatedly render distinct colors with failures between successful calls,
checking that static resolvers never cross sessions or return recycled pixels.

An additional real imported EPUB/Compose fixture retains source order, styles,
word identities, website versus internal-link behavior, inert SVG/image anchors,
viewer zoomed pixels, recreation, chapter transitions and exact reader positions
with embedded images. Library cover coverage now includes an embedded-raster SVG
alongside PNG and vector SVG, including cache/recreation/open/return checks.
Existing vector, raster, loader cancellation/serialization, unsupported fallback,
image-only completion and navigation regressions remain in the same device run.

The initial seven-case decode run passed six and caught an incorrect assertion:
x=1 lay on the scaled image boundary rather than outside it. Corrected it to x=0
and added the 64-source/one-pixel original-ratio test. A preliminary 19-case phone
run passed; after final missing-size, malformed-base64 and stylesheet guards, the
final phone run passed all 19 in 2m17s including build/lint. The pinned source JAR
was downloaded again and matches its recorded SHA-256; reviewed upstream data-URI
bypass and resolver/static lifetime before implementation. Initial sandbox DNS,
ADB listeners and Gradle wildcard-IP sockets needed approved outside-sandbox
execution. Restarted the existing swangle/KVM AVD without wiping data and verified
sys.boot_completed=1 before tests. Existing unrelated FileImports autoboxing lint
hint remains informational.

Only self-contained base64 PNG/JPEG is added. <=64 unique exact URI identities
share sampled temporary bitmaps with a combined <= output pixel budget (in
addition to output pixels), recycled after render. Generated source-ratio wrappers
count toward reference depth/expansion limits. Relative/local/remote resources,
nested vectors, patterns/masks/markers and selector-aware resource CSS remain
unfinished. Stylesheets accompanying embedded images retain fallback because
rewritten viewport ancestry can change selector matching. No resource I/O, new
dependency, core/FFI/schema/permission/source mutation or destructive semantics.
Temporary decoder/Canvas allocations and total heap/CPU/large-book costs remain
unmeasured. Rust/Clippy/host JNI/codegen, standalone dictionaries, release,
process-death/reboot and physical arm64 were not rerun for this Android-only slice;
physical arm64 remains user-deferred.

## Bounded self-contained nested SVG images

2026-10-27, using the Rust PATH/JDK/SDK/Gradle environment above:

```sh
tools/format-kotlin.sh --check
git diff --check
./android/gradlew -p android :app:assembleDebug :app:assembleDebugAndroidTest :app:lintDebug :app:connectedDebugAndroidTest -Pandroid.testInstrumentationRunnerArguments.class=app.yomoku.EpubSvgNestedImagesTest,app.yomoku.EpubSvgEmbeddedImagesTest,app.yomoku.EpubSvgDecodeTest,app.yomoku.EpubImageDecodeTest,app.yomoku.EpubImageLoaderTest,app.yomoku.ReaderImagesTest,app.yomoku.LibraryCoversTest,app.yomoku.NavigationTest --offline --console=plain
/home/kai/Android/Sdk/platform-tools/adb -s emulator-5554 shell wm size 2560x1600
/home/kai/Android/Sdk/platform-tools/adb -s emulator-5554 shell wm density 240
/home/kai/Android/Sdk/platform-tools/adb -s emulator-5554 shell cmd uimode night yes
# Repeat the same Gradle command for tablet/dark, then restore phone/light.
/home/kai/Android/Sdk/platform-tools/adb -s emulator-5554 shell wm size reset
/home/kai/Android/Sdk/platform-tools/adb -s emulator-5554 shell wm density reset
/home/kai/Android/Sdk/platform-tools/adb -s emulator-5554 shell cmd uimode night no
unzip -l android/app/build/outputs/apk/debug/app-debug.apk 'lib/*/libyomoku_jni.so'
```

Five new decoder fixtures verify nested vectors/rasters, original/fractional ratios,
independent child IDs/styles, viewBox-only children, UTF-16 XML, transforms/clipping/
opacity, one-pixel sampling and exact-URI sharing across documents. They inspect
combined temporary allocation/recycling and reject malformed/active/external/
cyclic resources, excessive resource nesting and aggregate XML/nodes/expansions.
Repeated placements reuse child rasterization rather than multiplying vector work.
Existing concurrent-session tests now alternate raster-only and nested-vector
resources while repeatedly rendering distinct colors across two threads.

An added real imported EPUB reader/Compose case checks nested SVG pixels, source
order, adjacent styles/word identities, external website versus inert internal/SVG
links, viewer/recreation, chapter changes and exact positions. Library cover
coverage adds nested SVG pixels/ratio/cache/recreation/open/return alongside PNG,
vector SVG and embedded-raster SVG. Existing raster, cancellation/serialization,
unsupported fallback, completion and navigation cases stay in the same device run.

The initial 13-case decoder run passed 12: Android's pull parser and AndroidSVG
both accepted an unclosed child root at EOF. Added explicit single-root/closed-root
preflight validation; the expanded phone/light run passed all 25 cases in 2m53s
including builds/lint. Initial ADB listeners and Gradle wildcard-IP sockets needed
approved outside-sandbox execution; the existing API-36 AVD was already running
and reused without restart/wipe. Pinned source-JAR SHA-256 matches provenance;
upstream resolver/image paths were reviewed again. No dependency/version,
core/FFI/schema, permission, original asset mutation or destructive-semantics change.

Final PASS: both-ABI debug/test APKs, strict Android lint, Kotlin formatting/diff
and all 25 API-36 x86_64 tests each on phone/light (2m53s including build/lint) and
tablet/dark (2m34s); zero failures/skips. Reports/results/lint XML, plus the initial
failure: `.local/test-reports/epub-svg-nested/{initial,phone,tablet-dark}/`.
APK inspection confirms arm64-v8a/x86_64 libyomoku_jni.so. Restored and verified
1080x2400, 420dpi, light mode and free rotation; the AVD remains running.
The existing unrelated FileImports autoboxing lint hint is informational.
No open failure or blocker.

Nested SVGs share <=64 exact resources, <=8 image-resource levels, <=2 MiB total
root/child XML, <=4096 nodes including wrappers and <=32768 expanded nodes.
All temporary raster/intermediate SVG pixels share one additional output-pixel
budget and recycle in finally. Each document retains <=64 element/reference levels
and existing source/edge bounds. Child detail stays sampled; total heap/CPU and
large-book costs remain unmeasured. Relative/local resources, patterns/masks/markers
and selector-aware/resource CSS remain unfinished. Rust/Clippy/host JNI/codegen,
standalone dictionary probes, release, process-death/reboot and physical arm64 were
not rerun for this Android-only slice; arm64 remains user-deferred.


## Bundled Noto Serif JP reader text

2026-10-27, using the Rust PATH/JDK/SDK/Gradle environment above:

```sh
tools/format-kotlin.sh --check
git diff --check
# Broad phone/light regression run (initially 22/23; see corrected fixture below).
./android/gradlew -p android :app:assembleDebug :app:assembleDebugAndroidTest :app:lintDebug :app:connectedDebugAndroidTest -Pandroid.testInstrumentationRunnerArguments.class=app.yomoku.ReaderTypographyTest,app.yomoku.ReaderFormattingTest,app.yomoku.ReaderImagesTest,app.yomoku.ReaderWordsTest,app.yomoku.ReaderNavigationTest,app.yomoku.NavigationTest --offline --console=plain
# Final corrected font/Unicode/credits checks: all three pass on phone/light.
./android/gradlew -p android :app:assembleDebug :app:assembleDebugAndroidTest :app:lintDebug :app:connectedDebugAndroidTest -Pandroid.testInstrumentationRunnerArguments.class=app.yomoku.ReaderTypographyTest --offline --console=plain
/home/kai/Android/Sdk/platform-tools/adb -s emulator-5554 shell wm size 2560x1600
/home/kai/Android/Sdk/platform-tools/adb -s emulator-5554 shell wm density 240
/home/kai/Android/Sdk/platform-tools/adb -s emulator-5554 shell cmd uimode night yes
./android/gradlew -p android :app:assembleDebug :app:assembleDebugAndroidTest :app:lintDebug :app:connectedDebugAndroidTest -Pandroid.testInstrumentationRunnerArguments.class=app.yomoku.ReaderTypographyTest,app.yomoku.ReaderFormattingTest,app.yomoku.ReaderImagesTest --offline --console=plain
/home/kai/Android/Sdk/platform-tools/adb -s emulator-5554 shell wm size reset
/home/kai/Android/Sdk/platform-tools/adb -s emulator-5554 shell wm density reset
/home/kai/Android/Sdk/platform-tools/adb -s emulator-5554 shell cmd uimode night no
/home/kai/Android/Sdk/platform-tools/adb -s emulator-5554 shell settings put system accelerometer_rotation 1
```

PASS: both-ABI debug/test APKs, strict Android lint, Kotlin formatting/diff and
23 distinct API-36 x86_64 phone/light cases across the initial broad run and final
three-case corrected rerun (45s including build/lint), plus all eight focused
tablet/dark cases (2m16s). The broader run (10m10s) passed all 20 preexisting
formatting/image/word/navigation regressions and two new cases; its only failure
was the new font fixture's metadata assertion. Final three/eight-case runs have
zero failures/errors/skips. Reports/logs/lint XML, including initial fixture and
device failures: `.local/test-reports/reader-font/`.

Three new cases verify the actual packaged font SHA-256; regular/bold glyph
outlines against independent platform instances from the verified font bytes;
synthesized italic; mixed Japanese/kana/Latin/supplementary/emoji text with exact
canonical text, font family, column bounds and unchanged page on recreation; and
complete offline attribution/OFL through Settings/recreation. Existing EPUB
formatting/image cases now also assert the font family used by actual layouts
while exercising links, word taps, viewer, image-only completion and chapter moves.
APK inspection confirms both JNI ABIs, the exact 13,574,352-byte font (8,094,431
compressed bytes), attribution and full license. No runtime font downloads.

Initial font test wrongly equated Typeface.weight with the selected variable
axis: Android reports this file's default weight 200 even with a 400/700 axis.
Compare actual outlines instead. The independent Typeface.Builder(assets, path)
oracle then returned null for the compressed res/font entry; build reference
instances from a temporary private-cache copy of the hash-verified resource bytes
and delete it in finally. Both were test-fixture fixes; app font loading worked.
The corrected three-case rerun and tablet suite pass. One intervening attempt ran
zero cases because the AVD disconnected; restarted the existing documented
swangle/KVM emulator without wiping data and waited for sys.boot_completed=1.
Sandbox DNS/Gradle wildcard-IP sockets needed approved outside-sandbox runs.
Existing FileImports autoboxing lint hint remains informational (zero warnings).

Restored/verified 1080x2400, 420dpi, light mode and free rotation; AVD remains
running. No open blocker. The shared text component retains inherited size/line
height, status colors and annotations; logical page/UTF-8 anchors stay unchanged.
No dependency/core/FFI/schema/permission/destructive-semantics change. Persisted
font-size/line-spacing controls and default-off source/analyzer furigana remain
unchecked. Rust/Clippy/host JNI/codegen, standalone dictionaries, release,
process-death/reboot and physical-arm64 checks were not rerun for this Android-only
slice; physical arm64 remains user-deferred.

## Persisted global reader font size and line spacing

2026-10-27, using the Rust PATH/JDK/SDK/Gradle environment above:

```sh
cargo test --workspace --all-features --locked --offline --quiet
cargo clippy --workspace --all-targets --all-features --locked --offline -- -D warnings
cargo fmt --all -- --check
tools/test-kotlin-bridge.sh
cargo run -p yomoku-ffi --bin generate-kotlin --features facet_typegen --locked --offline -- target/generated/kotlin-repeat
diff -r target/generated/kotlin target/generated/kotlin-repeat
tools/format-kotlin.sh --check
git diff --check
# Initial phone run: eight existing cases pass; two new fixture assertions fixed below.
./android/gradlew -p android :app:assembleDebug :app:assembleDebugAndroidTest :app:lintDebug :app:connectedDebugAndroidTest -Pandroid.testInstrumentationRunnerArguments.class=app.yomoku.ReaderTypographySettingsTest,app.yomoku.ReaderTypographyTest,app.yomoku.ReaderFormattingTest,app.yomoku.ReaderImagesTest --offline --console=plain
# Final corrected controls plus five-status colors and existing page-length interaction.
./android/gradlew -p android :app:assembleDebug :app:assembleDebugAndroidTest :app:lintDebug :app:connectedDebugAndroidTest '-Pandroid.testInstrumentationRunnerArguments.class=app.yomoku.ReaderTypographySettingsTest,app.yomoku.ReaderWordsTest#tappingAndFiveChoicesPersistWithoutLookupHistoryAndFailedChoiceCanRepeat,app.yomoku.PasteReaderTest#pageLengthSettingsReflowWithoutMovingTheExactSavedPosition' --offline --console=plain
/home/kai/Android/Sdk/platform-tools/adb -s emulator-5554 shell wm size 2560x1600
/home/kai/Android/Sdk/platform-tools/adb -s emulator-5554 shell wm density 240
/home/kai/Android/Sdk/platform-tools/adb -s emulator-5554 shell cmd uimode night yes
# Repeat the final four-case Gradle command for tablet/dark, then restore phone/light.
/home/kai/Android/Sdk/platform-tools/adb -s emulator-5554 shell wm size reset
/home/kai/Android/Sdk/platform-tools/adb -s emulator-5554 shell wm density reset
/home/kai/Android/Sdk/platform-tools/adb -s emulator-5554 shell cmd uimode night no
unzip -l android/app/build/outputs/apk/debug/app-debug.apk 'lib/*/libyomoku_jni.so'
```

Eight new Rust cases cover migration defaults and partial-ALTER rollback/retry,
all 20 size/spacing pairs through SQLite reopen and unchanged reader snapshots,
statement/commit rollback, corrupt/missing settings without repair, core routing/
busy/no-op/failure gates, late callbacks and failed time/completion obligations.
The real-Full Kotlin/JNI fixture round-trips every pair through typed events,
checks unchanged logical page/position/analysis/time and restores the final pair
through a fresh bridge. Repeat generation is byte-for-byte identical.

Two new device cases exercise actual persisted controls, explicit failed-save
retry, exact nonzero supplementary-Unicode offsets, Settings/reader recreation,
visible text layout sizes/line heights/wrapping and global application to another
document. An imported EPUB checks smallest/largest controls, canonical fragments,
image ratio and full token identity after word taps. Existing five-status checks
cover system light/dark highlights; existing page-length checks retain all logical
presets and exact anchors. The broad run retains Noto outlines/credits, EPUB
formatting/links, image decoding/viewer and chapter navigation regressions.

The initial Rust run caught a preexisting worker lifecycle timing assertion in
`load_engine_and_invalid_output_failures_roll_back_and_retry_fresh`; it passes
alone and in the final full workspace run. Historical schema assertions
and future/failure-migration fixtures were updated from version 14 to 15/16.
The initial Android run passed all eight existing cases and caught two new fixture
mistakes: direct reader-to-reader opening raced the paused clock checkpoint;
use the normal close-to-Library transition before opening the first book. Token
indices are original analysis identities, not positions in the filtered Japanese
page token list; match `tokenIndex` explicitly. No app fix was required for either.
Initial sandbox Gradle wildcard-IP sockets needed approved outside-sandbox offline
execution. The existing API-36 AVD was reused without restart/wipe.


Final PASS: 440 Rust tests, strict Clippy/Rustfmt, Kotlin formatting/diff, real-Full
host JNI and reproducible generated payloads. Sixteen data-dependent opt-in Rust
fixtures remain ignored; their standalone Full/index probes were not rerun.
Both arm64-v8a/x86_64 debug/test APKs and strict Android lint pass (only the existing
informational FileImports autoboxing hint). The initial 10-case phone run (4m10s)
passes all eight existing cases; the corrected four-case phone run (2m01s including
build/lint) and four-case tablet/dark run (1m41s) have zero failures/errors/skips.
That is 12 distinct passing phone cases across the initial/final runs plus four
on tablet/dark. Reports, logs and lint XML, including the initial failures, are
under `.local/test-reports/reader-typography/`.

Restored and verified 1080x2400, 420dpi, light mode and free rotation; the existing
AVD remains running. No open blocker. No dependency/version, permission or
destructive-semantics change. Release, process-death/reboot, standalone dictionary
benchmarks and physical arm64 checks were not rerun; physical arm64 remains
user-deferred. Default-off source/analyzer furigana is next.


## Source ruby reader transport and clipping

2026-10-27, using the existing pinned local toolchain and running emulator:

```sh
export PATH=/home/kai/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin:$PATH
export JAVA_HOME=/home/kai/repos/yomoku/.local/jdk-17/jdk-17.0.21+9
export GRADLE_USER_HOME=/home/kai/repos/yomoku/.local/gradle
export ANDROID_HOME=/home/kai/Android/Sdk
cargo test -p yomoku-core --test reader_ruby --all-features --locked --offline
cargo test -p yomoku-storage source_ruby --all-features --locked --offline
cargo test --workspace --all-features --locked --offline --quiet
cargo clippy --workspace --all-targets --all-features --locked --offline -- -D warnings
cargo fmt --all -- --check
tools/test-kotlin-bridge.sh
cargo run -p yomoku-ffi --bin generate-kotlin --features facet_typegen --locked --offline -- target/generated/kotlin-repeat
# Wait for generation to finish before comparing or starting the Android build.
diff -r target/generated/kotlin target/generated/kotlin-repeat
tools/format-kotlin.sh --check
git diff --check
./android/gradlew -p android :app:assembleDebug :app:assembleDebugAndroidTest :app:lintDebug :app:connectedDebugAndroidTest -Pandroid.testInstrumentationRunnerArguments.class=app.yomoku.DocumentSessionTest,app.yomoku.ReaderWordSpansTest,app.yomoku.ReaderFormattingTest --offline --console=plain
```

Six new host cases cover all logical page presets, crossing and adjacent source
bases, Unicode/NUL preservation, exact nonzero saved offsets, chapter ownership,
foreign/duplicate/unordered/overlapping/empty/non-scalar rejection, read-only
TXT/EPUB reopen and save, all five vocabulary statuses, deleted vocabulary,
Missing analysis and corrupted TXT metadata without repair. The shared loader
also retains existing TXT original-source and EPUB integrity tests. No parser,
schema, dependencies or source text changes.

PASS: 446 workspace Rust tests (16 data-dependent fixtures ignored), strict
Clippy/Rustfmt, host Kotlin/JNI and byte-identical repeat generation. Host JNI
checks real Full TXT/EPUB loads, positions/reopen/statuses and source-ruby payloads;
Rust/Kotlin round trips include nonzero supplementary/combining/NUL ranges and
readings. Android's existing production TXT session test now verifies the keyed
chapter and page metadata before and after native-session recreation; the span
test fixture supplies the new generated field. Source ruby remains visually
unused pending the switch/source-preferred analyzer fallback/Compose layout.

Initial sandbox Gradle execution hit the documented wildcard-IP/local-socket
restriction; retried offline outside the sandbox with approval. Initial Android
compilation caught missing ReaderRuby/TextRange imports in the new test assertion;
fixed both before rerunning. An initial repeat-codegen comparison ran before its
generator had exited; the comparison after successful exit is identical.

Final PASS: both-ABI debug/test APKs and strict Android lint; all nine selected
API-36 x86_64 phone cases pass with no failures/errors/skips (1m39s final Gradle
run). APK inspection confirms arm64-v8a/x86_64 libyomoku_jni.so. Reports and host/
Android logs, including the initial missing-import failure, are saved under
`.local/test-reports/reader-ruby/`. Reused the existing phone/light AVD without
changing display/theme/rotation settings; it remains running. No open blocker.
Tablet/dark UI, release, standalone dictionary probes, process-death/reboot and
physical arm64 were not rerun for this transport slice; physical arm64 remains
deferred. Next: default-off persisted furigana, source-preferred analyzer fallback,
Compose layout and tap/theme verification.


## Persisted default-off furigana preference

2026-10-27, using the pinned Rust PATH/JDK/SDK/Gradle environment above:

```sh
cargo test --workspace --all-features --locked --offline --quiet
cargo clippy --workspace --all-targets --all-features --locked --offline -- -D warnings
cargo fmt --all -- --check
tools/test-kotlin-bridge.sh
cargo run -p yomoku-ffi --bin generate-kotlin --features facet_typegen --locked --offline -- target/generated/kotlin-repeat
# Wait for generation to finish before comparing.
diff -r target/generated/kotlin target/generated/kotlin-repeat
tools/format-kotlin.sh --check
git diff --check
./android/gradlew -p android :app:assembleDebug :app:assembleDebugAndroidTest :app:lintDebug :app:connectedDebugAndroidTest -Pandroid.testInstrumentationRunnerArguments.class=app.yomoku.ReaderTypographySettingsTest --offline --console=plain
unzip -l android/app/build/outputs/apk/debug/app-debug.apk 'lib/*/libyomoku_jni.so'
```

Migration 0016 adds a constrained semantic 0/1 setting, default off. Three new
Rust cases verify upgrade preserving existing font/spacing/page length and exact
supplementary-Unicode offset; migration failure preserves schema/history/bytes
and retries; flag-only core changes publish after commit/retry with the rest of
the ViewModel unchanged. Existing cases now cover all 40 font/spacing/flag
combinations through SQLite reopen, statement/deferred-COMMIT rollback of all
three fields, corrupt boolean rejection without repair, unknown/busy/pending-save
routing and late saves. Full workspace: 449 passing tests, 16 opt-in ignored.

Host real-Full Kotlin/JNI round-trips all 40 typed combinations and verifies
unchanged logical page/position, original analysis and pending time; a fresh
native session restores the enabled flag without loading Full. Repeat Kotlin
generation is identical; strict Clippy/Rustfmt/Kotlin formatting/diff pass.
The existing Android typography fixture sets the flag through typed Settings
before changing font/spacing using visible controls, checking that those changes
preserve the flag. Its second case supplies the explicit disabled flag while
retaining image/canonical-fragment/word-tap regression coverage.

The first focused/full Rust runs caught a remaining historical fixture expecting
schema 15 after successful retry; updated that assertion to 16 before the final
passing full run. No production fix was needed. Gradle's sandbox wildcard-IP
restriction required approved outside-sandbox execution. No emulator was running;
cold-started the existing medium_phone with the documented swangle/KVM flags,
without wiping data, and confirmed sys.boot_completed=1.

Final PASS: both-ABI debug/test APKs, strict Android lint (only the existing
informational FileImports autoboxing hint) and both selected API-36 x86_64 phone
cases, zero failures/errors/skips (2m11s including build/lint). APK inspection
confirms arm64-v8a/x86_64 JNI. Logs, device reports and lint XML:
`.local/test-reports/reader-furigana-preference/`. The AVD remains running at
1080x2400/420dpi/light, unchanged after boot. No open blocker.

This slice persists/transports the preference; the visible switch, source-preferred
analyzer fallback and ruby layout/taps/themes remain unchecked. No dependency,
permission or destructive-semantics change. Tablet/dark, release, process-death,
standalone dictionary benchmarks and physical-arm64 checks were not rerun;
physical arm64 remains user-deferred.


## Source-preferred furigana projection

2026-10-27, using the pinned Rust PATH/JDK/SDK/Gradle environment above:

```sh
cargo test -p yomoku-core --test reader_furigana --all-features --locked --offline
cargo test -p yomoku-core --test reader_flow --all-features --locked --offline furigana
cargo test --workspace --all-features --locked --offline --quiet
cargo clippy --workspace --all-targets --all-features --locked --offline -- -D warnings
cargo fmt --all -- --check
tools/test-kotlin-bridge.sh
cargo run -p yomoku-ffi --bin generate-kotlin --features facet_typegen --locked --offline -- target/generated/kotlin-repeat
diff -r target/generated/kotlin target/generated/kotlin-repeat
tools/format-kotlin.sh --check
git diff --check
./android/gradlew -p android :app:assembleDebug :app:assembleDebugAndroidTest :app:lintDebug :app:connectedDebugAndroidTest -Pandroid.testInstrumentationRunnerArguments.class=app.yomoku.DocumentSessionTest,app.yomoku.ReaderWordSpansTest,app.yomoku.ReaderTypographySettingsTest --offline --console=plain
unzip -l android/app/build/outputs/apk/debug/app-debug.apk 'lib/*/libyomoku_jni.so'
```

Four new Rust cases cover source overlap with multiple/partial analyzer tokens,
all five statuses and absent/deleted vocabulary, no readings/Missing analysis,
complete crossing readings at all five logical presets, supplementary/combining/
NUL text, stale document/page/analysis keys, foreign anchors, malformed/clipped/
non-scalar/reversed ranges, canonical surface mismatch, unordered/duplicate tokens,
committed preference gating and refresh failure/retry/reflow/departure. Source ruby
remains usable when analyzer data is unavailable/rejected. No I/O, schema, parser,
dependency, text/anchor, history or vocabulary semantics change.

Real-Full host Kotlin/JNI and Android's production TXT session fixture now assert
an empty disabled projection, source readings before loading tokens, literal
source hiragana over analyzer katakana, fallback for a second unannotated cat,
unchanged readings after a status choice, fresh-session preference/source restore
and dictionary-free token loads. Existing EPUB/SRT/dictionary/Unicode host checks
pass. Generated ViewModel transports readerFurigana; Compose does not consume it
yet. The visible switch/layout/taps/themes task remains unchecked.

Final PASS: 453 workspace Rust tests (16 opt-in ignored), strict Clippy/Rustfmt,
full host Kotlin/JNI, byte-identical repeat generation, Kotlin formatting/diff,
both arm64-v8a/x86_64 debug/test APKs and strict Android lint. Ten selected
API-36 x86_64 phone cases pass with zero failures/errors/skips (3m03s including
build/lint); APK inspection confirms both JNI ABIs. The preexisting FileImports
autoboxing lint hint remains informational. Logs/reports/lint XML are under
`.local/test-reports/reader-furigana-projection/` (host JNI output is in the session
tool transcript).

Initial checks caught an old explicit ViewModel initializer missing the new field,
a new reflow fixture whose single long paragraph produced identical logical pages,
and Clippy's lazy-Option/clone-in-assert suggestions. Updated the initializer,
used two paragraph boundaries to exercise actual reflow, and applied the lint
suggestions before the final passing runs. Gradle and ADB local sockets required
approved outside-sandbox execution; no downloads. Reused the existing AVD without
restart/wipe/display/theme changes; verified 1080x2400/420dpi. No blocker.
Tablet/dark layout, release, standalone dictionary benchmarks, process-death/reboot
and physical-arm64 checks were not rerun; physical arm64 remains user-deferred.


## Furigana switch and Compose ruby layout

2026-10-27, using the pinned Rust PATH/JDK/SDK/Gradle environment above:

```sh
# Focused layout/Settings/EPUB/TXT and existing typography cases.
./android/gradlew -p android :app:assembleDebug :app:assembleDebugAndroidTest :app:lintDebug :app:connectedDebugAndroidTest -Pandroid.testInstrumentationRunnerArguments.class=app.yomoku.ReaderRubyLayoutTest,app.yomoku.ReaderFuriganaTest,app.yomoku.ReaderTypographySettingsTest --offline --console=plain
# Corrected TXT crossing-page fixture plus existing formatting/word/image regressions.
./android/gradlew -p android :app:assembleDebug :app:assembleDebugAndroidTest :app:lintDebug :app:connectedDebugAndroidTest '-Pandroid.testInstrumentationRunnerArguments.class=app.yomoku.ReaderFuriganaTest#txtCrossingSourceBaseKeepsCompleteReadingThroughLogicalReflowAndMovement,app.yomoku.ReaderFormattingTest,app.yomoku.ReaderWordSpansTest,app.yomoku.ReaderImagesTest' --offline --console=plain
/home/kai/Android/Sdk/platform-tools/adb -s emulator-5554 shell wm size 2560x1600
/home/kai/Android/Sdk/platform-tools/adb -s emulator-5554 shell wm density 240
/home/kai/Android/Sdk/platform-tools/adb -s emulator-5554 shell cmd uimode night yes
# Repeat the focused six-case command for tablet/dark; restore phone/light afterward.
/home/kai/Android/Sdk/platform-tools/adb -s emulator-5554 shell wm size reset
/home/kai/Android/Sdk/platform-tools/adb -s emulator-5554 shell wm density reset
/home/kai/Android/Sdk/platform-tools/adb -s emulator-5554 shell cmd uimode night no
tools/format-kotlin.sh --check
cargo fmt --all -- --check
git diff --check
unzip -l android/app/build/outputs/apk/debug/app-debug.apk 'lib/*/libyomoku_jni.so'
```

Four new device cases cover strict UTF-8/UTF-16 projection validation, supplementary/
combining/NUL text, clipped bases with complete readings, real measured/pixel ruby
above wrapped headings, long-reading width fitting, 14sp/24sp and 1.6x system font
scaling in light/dark palettes, source precedence/analyzer fallback, visible switch
failure/retry/recreation, all five vocabulary statuses, unchanged word/link/image
identities and logical reflow/movement. Canonical Text remains the only semantic/
gesture text. Existing typography assertions now allow measured ruby headroom;
the disabled image/typography path keeps exact chosen line height.

Phone results: the second focused run passes five cases; the corrected TXT case
and ten existing formatting/span/image cases all pass in the final 11-case run
(2m42s), for 16 distinct passing cases. Both-ABI debug/test APKs, strict Android
lint, Kotlin formatting, Rustfmt and whitespace checks pass; APK inspection confirms
arm64-v8a/x86_64 JNI. All six focused tablet/dark cases pass (1m50s), zero
failures/errors/skips. The isolated paint fixture exercises both palettes on each
viewport. Restored 1080x2400/420dpi/light afterward; the AVD remains running.

Initial compilation caught the new fixture importing Compose TextRange instead of
the typed bridge value. The first device run exposed a fixture-only missing default
ComponentActivity; use MainActivity and replace its content for the isolated paint
fixture. The initial 300-character paragraph stayed whole under the existing
paragraph-first pagination policy, even after adding sentences (splitting only
starts above twice the preset). Use a 600-character sentence-bearing source base,
250 preset and 1500 reflow to exercise a real crossing without changing pagination.
No production fix was required after the initial successful build.

Gradle's sandbox wildcard-IP/local-socket restriction required approved outside-
sandbox execution. No emulator was running; cold-started the existing swangle/KVM
AVD without wiping data, waited for boot and verified 1080x2400/420dpi. Reports,
initial failures and lint XML: `.local/test-reports/reader-furigana-layout/`.
No Rust/core/FFI/schema/dependency change; full Rust tests/Clippy/host JNI/codegen,
release, standalone dictionary benchmarks, process-death and physical-arm64 checks
were not rerun. Physical arm64 remains user-deferred.


## Missing-match custom dictionary persistence

2026-10-27, using the pinned local Rust/JDK/SDK environment:

```sh
export PATH=/home/kai/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin:$PATH
export JAVA_HOME=/home/kai/repos/yomoku/.local/jdk-17/jdk-17.0.21+9
export GRADLE_USER_HOME=/home/kai/repos/yomoku/.local/gradle
export ANDROID_HOME=/home/kai/Android/Sdk
cargo test -p yomoku-storage custom_dictionary --all-features --locked --offline
cargo test --workspace --all-features --locked --offline --quiet
cargo clippy --workspace --all-targets --all-features --locked --offline -- -D warnings
cargo fmt --all -- --check
tools/test-kotlin-bridge.sh
cargo run -p yomoku-ffi --bin generate-kotlin --features facet_typegen --locked --offline -- target/generated/kotlin-repeat
# Compare after generation exits.
diff -r target/generated/kotlin target/generated/kotlin-repeat
tools/format-kotlin.sh --check
git diff --check
./android/gradlew -p android :app:assembleDebug :app:assembleDebugAndroidTest :app:lintDebug :app:connectedDebugAndroidTest -Pandroid.testInstrumentationRunnerArguments.class=app.yomoku.DocumentSessionTest,app.yomoku.ReaderWordsTest --offline --console=plain
unzip -l android/app/build/outputs/apk/debug/app-debug.apk 'lib/*/libyomoku_jni.so'
```

Migration 0017 adds an independent versioned exact-surface custom dictionary table.
The pure domain type requires nonblank English meaning and reading while retaining
literal Unicode. Selected-token read/create/edit/delete repositories derive surface
and base from validated original Mode B, checking both against the separate immutable
word/name index on every operation. Any built-in candidate blocks access; lookup
failure never permits a custom fallback. No vocabulary/analysis/history/position/
time changes or custom base-form propagation occur. Runtime/FFI/editor remain open.

PASS: eleven new domain/storage cases; 464 workspace Rust tests (16 opt-in ignored),
strict Clippy/Rustfmt, Kotlin formatting, real-Full host Kotlin/JNI and identical
repeat generation. Tests cover exact Unicode/NUL/combining spelling, global reuse,
edit/delete/reopen/read-only bytes, all statuses/deleted entries, all five logical
presets and crossing-token identity, invalid/stale/foreign/non-Japanese selections,
word/name/base protection and dictionary failures, corruption/no implicit repair,
concurrent writers, identical no-op retries, create/edit/delete statement and
deferred-COMMIT rollback, migration preservation/failure/retry. Existing migration
fixtures advance current/future schemas to 17/18.

Initial focused checks caught a fixture passing a chapter instead of a chapter
slice to LogicalPages and expecting three registrations for two distinct surfaces;
fixed both before the passing full run. Gradle hit the documented sandbox
wildcard-IP/local-socket restriction; reran offline outside the sandbox with
approval. Host JNI log and Android logs: .local/test-reports/custom-dictionary-storage/.
Final PASS: both arm64-v8a/x86_64 debug/test APKs, strict Android lint (only the
existing informational FileImports autoboxing hint) and all eleven selected
API-36 x86_64 phone cases, zero failures/errors/skips (5m52s including native
build/lint). APK inspection confirms both JNI ABIs. Preserved device reports and
lint XML alongside logs in .local/test-reports/custom-dictionary-storage/.
Reused the running 1080x2400/420dpi phone AVD without restart, wipe or display
changes; it remains running. No open blocker. No dependency/version, permission
or existing destructive-semantics change.

This is a domain/storage slice: no public bridge schema or Android editor change.
Next: core/FIFO/FFI custom read/save/delete and selection/retry lifecycle, then
missing-match Android editor/deletion. Tablet/dark, release, standalone dictionary
benchmarks, process-death/reboot and physical-arm64 checks were not rerun; hardware
validation remains user-deferred.

## Missing-match custom dictionary core/FIFO/FFI transport

2026-10-27, using the pinned Rust/JDK/SDK/Gradle environment above:

```sh
cargo test -p yomoku-core --test reader_flow custom_entry --all-features --locked --offline
cargo test -p yomoku-ffi --test reader_words --all-features --locked --offline
cargo test --workspace --all-features --locked --offline --quiet
cargo clippy --workspace --all-targets --all-features --locked --offline -- -D warnings
cargo fmt --all -- --check
tools/test-kotlin-bridge.sh
cargo run -p yomoku-ffi --bin generate-kotlin --features facet_typegen --locked --offline -- target/generated/kotlin-repeat
# Compare after generation exits.
diff -r target/generated/kotlin target/generated/kotlin-repeat
tools/format-kotlin.sh --check
git diff --check
./android/gradlew -p android :app:assembleDebug :app:assembleDebugAndroidTest :app:lintDebug :app:connectedDebugAndroidTest -Pandroid.testInstrumentationRunnerArguments.class=app.yomoku.ReaderWordDetailsSessionTest,app.yomoku.ReaderWordsTest,app.yomoku.DocumentSessionTest --offline --console=plain
unzip -l android/app/build/outputs/apk/debug/app-debug.apk 'lib/*/libyomoku_jni.so'
```

Selected-word reads carry a typed exact-surface custom entry from the same user-state
transaction, only after successful empty word/name lookup. Explicit save/delete events
use the existing shared gates/FIFO and recheck stored original-token selection and
built-ins. Results attach only to the matching pending/current selection and surface.
Failures retain committed data with an independent error; explicit retries and final
clock drains work without an automatic mutation retry loop. No analyzer, schema,
dependency, canonical text, status/history/time or destructive-semantics change.
Android editor controls remain the next slice.

PASS: six new Rust cases, 470 workspace tests (16 opt-in ignored), strict Clippy,
Rustfmt/Kotlin formatting, real-Full host Kotlin/JNI and identical repeat generation.
Coverage includes literal supplementary/combining/NUL Unicode, create/edit/delete/
reopen, identical no-op retries, built-in protection and dictionary rechecks, corrupted
index/custom values, repaired index publication, write failure/explicit retry,
selection/busy gates, late success/error cancellation and queued final clock saves.
The host and new Android session fixture use a real Full out-of-vocabulary katakana
surface; built-in Tokyo word/name definitions remain immutable. The visible editor
is not part of this transport slice.

Initial checks found the existing explicit Rust ViewModel initializer needing its new
field, and the prior storage equality fixture needing its now-present custom entry.
New FFI fixtures needed a genuinely indexed matching payload and an unrelated status
save instead of a second no-op RestoreReadingTime callback. Corrected all before the
passing workspace run. Gradle/ADB local sockets required approved outside-sandbox
execution; no downloads. The initial 13-case phone run passed the new transport,
existing dictionary transport and seven word-panel cases (nine total), then the
AVD disappeared during the movement case. The runner reports an empty failure and
`device not found`, with three document-session cases never started. Preserve that
incomplete run separately; no application assertion diagnosis is established.
Restarted the existing swangle/KVM AVD without wiping data or changing display.
Final continuation results and preserved reports follow below.

Continuation command after AVD boot (`sys.boot_completed=1`), retaining the nine
completed cases from the initial run:

```sh
ANDROID_HOME=/home/kai/Android/Sdk tools/start-android-emulator.sh
./android/gradlew -p android :app:assembleDebug :app:assembleDebugAndroidTest :app:lintDebug :app:connectedDebugAndroidTest '-Pandroid.testInstrumentationRunnerArguments.class=app.yomoku.ReaderWordsTest#movingBothDirectionsDismissesDetailsAndNeverRestoresTheSelection,app.yomoku.DocumentSessionTest' --offline --console=plain
```

The first continuation failed before running any tests with
`INSTALL_FAILED_INSUFFICIENT_STORAGE` during post-boot APK installation. The AVD
stayed connected, reported 1.4 GiB available and no third-party packages. After the
failed install session ended, direct `adb -s emulator-5554 install -r -t
android/app/build/outputs/apk/debug/app-debug.apk` succeeded; reran the same four-case
command. No data/cache cleanup, uninstall, display change or AVD wipe was performed.


Final PASS: all four continuation cases, zero failures/errors/skips (2m24s),
including the interrupted two-direction navigation case. Together with the nine
successful initial cases, thirteen distinct API-36 x86_64 phone cases pass, including
the new custom transport test. Both-ABI debug/test APKs, strict Android lint and APK
ABI inspection pass; lint contains only the existing informational FileImports
AutoboxingStateCreation hint. The original incomplete run and failed post-boot
install are retained alongside final device/lint/host logs and reports under
`.local/test-reports/custom-dictionary-transport/`. No unresolved application or
verification blocker. The restarted AVD remains running at 1080x2400/420dpi.
Tablet/dark, release, standalone dictionary benchmarks, process-death and physical
arm64 were not rerun; hardware validation remains user-deferred.

## Android missing-match custom dictionary editor

2026-10-27, using the pinned Rust PATH/JDK/SDK/Gradle environment above:

```sh
# Initial complete word-panel/transport run.
./android/gradlew -p android :app:assembleDebug :app:assembleDebugAndroidTest :app:lintDebug :app:connectedDebugAndroidTest -Pandroid.testInstrumentationRunnerArguments.class=app.yomoku.ReaderWordsTest,app.yomoku.ReaderWordDetailsSessionTest --offline --console=plain
# Focused editor/lookup cases, including keyboard input and cross-book reuse.
./android/gradlew -p android :app:assembleDebug :app:assembleDebugAndroidTest :app:lintDebug :app:connectedDebugAndroidTest '-Pandroid.testInstrumentationRunnerArguments.class=app.yomoku.ReaderWordsTest#customEntryCreateEditDeleteSurvivesRecreationAndIsGlobalWithoutChangingReaderHistory,app.yomoku.ReaderWordsTest#customEntrySaveAndDeleteFailuresRetainDataAndOnlyRetryOnExplicitActions,app.yomoku.ReaderWordsTest#customDraftsAreSelectionScopedAndPendingSavesGateEveryMutation,app.yomoku.ReaderWordsTest#dictionaryFailureNeedsExplicitRetryAndEmptyMatchesAreSuccessful' --offline --console=plain
# Repeat only the cross-book case after correcting its reader-close/time-drain flow.
./android/gradlew -p android :app:assembleDebug :app:assembleDebugAndroidTest :app:lintDebug :app:connectedDebugAndroidTest '-Pandroid.testInstrumentationRunnerArguments.class=app.yomoku.ReaderWordsTest#customEntryCreateEditDeleteSurvivesRecreationAndIsGlobalWithoutChangingReaderHistory' --offline --console=plain
/home/kai/Android/Sdk/platform-tools/adb -s emulator-5554 shell wm size 2560x1600
/home/kai/Android/Sdk/platform-tools/adb -s emulator-5554 shell wm density 240
/home/kai/Android/Sdk/platform-tools/adb -s emulator-5554 shell cmd uimode night yes
# Repeat the four-case focused command above, then restore phone/light.
/home/kai/Android/Sdk/platform-tools/adb -s emulator-5554 shell wm size reset
/home/kai/Android/Sdk/platform-tools/adb -s emulator-5554 shell wm density reset
/home/kai/Android/Sdk/platform-tools/adb -s emulator-5554 shell cmd uimode night no
tools/format-kotlin.sh --check
cargo fmt --all -- --check
git diff --check
unzip -l android/app/build/outputs/apk/debug/app-debug.apk 'lib/*/libyomoku_jni.so'
```

Three new device cases exercise the visible Add/Edit/Delete controls within the
existing sheet/panel, required nonblank fields and literal supplementary/combining
Unicode/whitespace, Cancel, committed-only display, recreation/reselection, multiple
original occurrences and cross-book exact-surface reuse. They compare original
page/tokens and database vocabulary/status/completion counts before and after CRUD.
Tokyo word/name matches and failed dictionary reads never offer custom controls.

Real SQLite INSERT/DELETE triggers verify retained drafts/committed data, separate
save/delete errors, no automatic retry after clock writes, and explicit retries of
the exact failed payload even after draft edits. An isolated scrollable fixture
verifies storage/time/completion gates on fields/Save/Edit/Delete/Retry, selection
changes discarding drafts/errors, and edit callbacks recording reader interaction.
Production core/storage/FFI/schema/dependencies remain unchanged. The formatter also
wraps one existing assertion in the exercised transport fixture.

Phone results: initial run 12/13 passed, with the new isolated fixture failing only
because its test container lacked a scroll parent. Added that fixture container;
the next four-case run passed three cases including keyboard edits, exact retries
and pending-save gates. Its enhanced cross-book test attempted CreatePaste directly
from the active reader while final time was saving; the core correctly rejected the
busy command. Follow the real Library flow: CloseDocument, await final-time drain,
then import/open; close/drain again before returning to the original book. The
corrected single case passes (1m06s), including unchanged database counts. Thirteen
distinct phone cases pass across these runs. No production workaround was needed.

PASS: both arm64-v8a/x86_64 debug/test APKs, strict Android lint (only the existing
informational FileImports autoboxing hint), Kotlin formatting/Rustfmt/diff and APK
ABI inspection. Gradle wildcard-IP/local sockets required approved outside-sandbox
execution; no downloads. Preserved initial/focused/final device logs and reports:
`.local/test-reports/custom-dictionary-editor/`. Rust tests/Clippy/host JNI/repeat
codegen, release, standalone dictionary benchmarks, process-death/reboot and physical
arm64 were not rerun for this Android-only slice; physical hardware stays deferred.

Final tablet/dark results: all four focused cases pass (1m49s), zero failures/
errors/skips, including actual field focus/keyboard edits in the wide popup and
recreation, retries and cross-book persistence. Preserved reports/lint XML alongside
phone runs. Restored the existing AVD to 1080x2400/420dpi/light without wiping or
restarting it; it remains running. No unresolved verification/application blocker.

## Effective segmentation boundary resolution

2026-10-27, from the repository root with the pinned Rust/JDK environment:

```sh
export PATH=/home/kai/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin:$PATH
export JAVA_HOME=/home/kai/repos/yomoku/.local/jdk-17/jdk-17.0.21+9
cargo test -p yomoku-core --test segmentation --all-features --locked --offline
cargo test -p yomoku-storage segmentation --all-features --locked --offline
cargo test --workspace --all-features --locked --offline --quiet
tools/test-sudachi.sh
cargo clippy --workspace --all-targets --all-features --locked --offline -- -D warnings
cargo fmt --all -- --check
tools/test-kotlin-bridge.sh
cargo run -p yomoku-ffi --bin generate-kotlin --features facet_typegen --locked --offline -- target/generated/kotlin-repeat
# Compare only after generation exits.
diff -r target/generated/kotlin target/generated/kotlin-repeat
tools/format-kotlin.sh --check
git diff --check
```

The pure resolver applies migration-0007 partitions over complete adjacent Japanese
Mode B tokens, with literal UTF-8 and local/newer-ID overlap precedence. A/B/C
partition construction requires complete tokens and the exact original profile.
Read-only `load_chapter_segmentation` restores original B plus active global/local
rules in one transaction. It returns separate boundary data and original-token
references without manufacturing analyzer metadata or changing existing reader/
completion behavior. No dependency, migration or public bridge change.

PASS: eight new core and five new storage cases; 483 workspace tests (17 opt-in
ignored), strict Clippy/Rustfmt, Kotlin formatting, whitespace, real-Full host
Kotlin/JNI and byte-identical repeat generation. All fifteen real-Full analysis fixtures plus one runtime fixture
pass for EACH owned/mapped loader (32 invocations). The new real fixture exercises
A/B/C, manual conjugation merge, local B restoration, reversal, SQLite reopen and
unchanged immutable snapshots against pinned Full; both loaders pass it.

Data-free coverage includes supplementary/combining/NUL surfaces, no substring/
normalization/gap/punctuation matching, repeated/self-overlapping matches, local
and newest-rule precedence independent of input order, reverse revealing still-
active overlaps, inactive superseded rules, cross-document globals, deleted-origin
provenance/reimport identity, same-profile snapshot validation, pinned original B,
Missing B/foreign modes, corrupt rules/snapshots and query-only byte preservation.
Full row/value comparisons verify save/replace/resolve/reverse leave analyses,
vocabulary/status events, completions/occurrences, position, settings and time intact.

Initial focused storage checks caught a test-only reference to a nonexistent
`document_reading_state` table; compare `documents` and `pending_reading_time`
instead. Initial Clippy requested a borrowed singleton in one new assertion;
corrected before the final passing run. No production workaround or new blocker.
The complete host bridge run passes Unicode/codecs, imports/cancellation,
selected-word/custom CRUD, navigation and handle lifecycle checks; the known Crux
middleware teardown messages remain informational. No sandbox escalation or
network download was needed for these checks.
APK/device/Android lint, release, performance, process-death and physical arm64
were not rerun for this domain/storage slice. The existing AVD was not changed.
Next: effective token/word/metadata/registration and frozen future-completion
identities, then typed commands and Android controls. Prior occurrences stay immutable.

## Effective segmentation metadata provenance

2026-10-27, from the repository root with the pinned Rust/JDK environment:

```sh
export PATH=/home/kai/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin:$PATH
export JAVA_HOME=/home/kai/repos/yomoku/.local/jdk-17/jdk-17.0.21+9
cargo test -p yomoku-core --test segmentation --all-features --locked --offline
cargo test -p yomoku-storage segmentation --all-features --locked --offline
cargo test --workspace --all-features --locked --offline --quiet
tools/test-sudachi.sh
cargo clippy --workspace --all-targets --all-features --locked --offline -- -D warnings
cargo fmt --all -- --check
tools/test-kotlin-bridge.sh
cargo run -p yomoku-ffi --bin generate-kotlin --features facet_typegen --locked --offline -- target/generated/kotlin-repeat
# Compare after generation exits.
diff -r target/generated/kotlin target/generated/kotlin-repeat
tools/format-kotlin.sh --check
git diff --check
```

Pure effective segmentation now attaches optional exact-span analyzer metadata,
retaining source AnalysisId/mode/token index. Original B wins, then same-profile A,
then C, independently of input order. Arbitrary manual spans remain unresolved,
including classification; absent metadata differs from known tokens with absent
reading/lemma. No neighboring reading/lemma concatenation or surface-only reuse.
The new read-only repository transaction loads original B, existing same-profile
A/C and active rules together. Later profiles are excluded; no analyzer is loaded.
Live reader/word/furigana/completion behavior remains original B for this prerequisite.

PASS: five new data-free cases, 488 workspace Rust tests (17 opt-in ignored),
strict Clippy/Rustfmt/Kotlin formatting/diff and byte-identical repeat generation.
All fifteen Full analysis fixtures plus one runtime fixture pass for EACH owned/
mapped loader (32 invocations), including new exact metadata/provenance assertions
in the real segmentation fixture and unresolved conjugation merges/reopen.
Synthetic cases cover exact repeated contextual readings, B/A/C precedence,
Unicode/NUL/combining boundaries, duplicate/foreign IDs/modes/profiles, whole-chapter
corruption, missing B, irrelevant later-profile corruption, global rules after
origin deletion, reversal, SQLite reopen/query-only byte preservation and unchanged
original tokens/vocabulary/status/completion/history/progress/settings/time rows.

Initial focused compilation needed an explicit Vec type on a new test closure.
The first workspace run failed the existing runtime import test's immediate
`set_foreground(false).is_err()` teardown assertion after dropping its router.
The isolated test and subsequent complete workspace run pass; no runtime production
or fixture change was made. Host logs: /tmp/yomoku-effective-metadata-tests.log and
/tmp/yomoku-effective-metadata-bridge.log. No dependency/version, schema, public
bridge or Android changes. APK/device/lint, release, performance, process-death and
physical arm64 checks were not rerun; the existing AVD was not touched.
Next: effective eligibility/token/word/registration plus frozen future completions,
then core/FIFO/FFI controls and Android choices. Past occurrences remain immutable.

Final host PASS: real-Full Kotlin/JNI Unicode/codecs, selected-word/custom-entry,
imports/direct cancellation, reader/navigation and handle-lifecycle checks. Repeat
Kotlin generation is byte-identical. Known Crux middleware teardown messages stay
informational. No escalation/network download or unresolved blocker for this slice.

## Effective reader eligibility and page/status projection

2026-10-27, from the repository root with the pinned Rust/JDK environment:

```sh
export PATH=/home/kai/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin:$PATH
export JAVA_HOME=/home/kai/repos/yomoku/.local/jdk-17/jdk-17.0.21+9
cargo test -p yomoku-core --test segmentation --all-features --locked --offline
cargo test -p yomoku-storage segmentation --all-features --locked --offline
cargo test --workspace --all-features --locked --offline --quiet
tools/test-sudachi.sh
cargo clippy --workspace --all-targets --all-features --locked --offline -- -D warnings
cargo fmt --all -- --check
tools/format-kotlin.sh --check
tools/test-kotlin-bridge.sh
cargo run -p yomoku-ffi --bin generate-kotlin --features facet_typegen --locked --offline -- target/generated/kotlin-repeat
# Compare only after generation exits.
diff -r target/generated/kotlin target/generated/kotlin-repeat
git diff --check
```

PASS: four new pure and three new storage cases; 495 workspace tests (17 opt-in
ignored), strict Clippy/Rustfmt/Kotlin formatting/diff, real-Full host Kotlin/JNI
and byte-identical repeat generation. All fifteen Full analysis fixtures plus one
runtime fixture pass for EACH owned/mapped loader (32 invocations). Extended the
real segmentation fixture with effective manual eligibility, non-Japanese exclusions,
complete resolution provenance and identical page/status results after reopen.

The pure opaque effective chapter borrows canonical text without copying it and
retains validated boundaries/metadata. Exact analyzer classification governs known
tokens; unresolved manual partitions explicitly inherit Japanese-span eligibility
without fabricated metadata. Effective segment indices differ from analyzer indices.
Visible crossings retain complete surfaces/anchors/metadata on both pages; future
occurrences own starts exactly once, preserving repetitions at every page preset.
Synthetic crossing tokens deliberately include sentence/newline boundaries so the
paragraph/sentence-aware paginator exercises clipping under every preset.

The separate effective page/status repository validates ownership and whole snapshots
in the same read-only transaction as live indexed exact-surface vocabulary lookups,
sharing repeated spellings. Query-only bytes/row comparisons, status changes, deleted
entries, absent B/deleted documents, UTF-8/foreign/empty pages, off-page corruption,
global reversal and reopen pass. Original B reader/completion behavior is unchanged;
new or deleted split/merge spellings remain absent until future explicit registration.

Initial focused checks caught two nonexistent page-preset names and a missing test
status import. Replaced the preset list with PageLength::ALL and added the import.
The initial crossing fixture had sentence boundaries only between complete merged
tokens, so it never crossed a logical page. Corrected the synthetic fixture before
the final passing focused/workspace/Full runs. No production workaround was needed.
Logs: /tmp/yomoku-effective-reader-{tests,full,clippy,bridge}.log. No dependencies,
schema/public bridge or Android source changes. No APK/device/Android lint, release,
performance, process-death or physical-arm64 rerun; AVD unchanged. No blocker.
Next: effective selected-word/registration and frozen future-completion identities,
then the live reader/furigana switch and core/FIFO/FFI/Android controls. Past reading
occurrences remain immutable.

## Effective selected-word lookup and frozen selection identity

2026-10-27, from the repository root with the pinned Rust/JDK environment:

```sh
export PATH=/home/kai/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin:$PATH
export JAVA_HOME=/home/kai/repos/yomoku/.local/jdk-17/jdk-17.0.21+9
cargo test -p yomoku-storage reader_words --all-features --locked --offline --quiet
cargo test -p yomoku-storage effective_reader_words --all-features --locked --offline
cargo test --workspace --all-features --locked --offline --quiet
tools/test-sudachi.sh
cargo clippy --workspace --all-targets --all-features --locked --offline -- -D warnings
cargo fmt --all -- --check
tools/format-kotlin.sh --check
tools/test-kotlin-bridge.sh
cargo run -p yomoku-ffi --bin generate-kotlin --features facet_typegen --locked --offline -- target/generated/kotlin-repeat
# Compare after generation exits.
diff -r target/generated/kotlin target/generated/kotlin-repeat
git diff --check
```

PASS: six new effective-word cases, 501 workspace Rust tests (17 opt-in ignored),
strict Clippy/Rustfmt/Kotlin formatting/diff and byte-identical repeat generation.
All fifteen Full analysis fixtures plus one runtime fixture pass EACH owned/mapped
(32 invocations). Extended real segmentation coverage verifies selected effective
metadata and surface/base query identities, manual unresolved terms, read-only bytes
and identical complete details after SQLite reopen using an empty immutable index.

Synthetic cases verify simultaneous word/name matches and complete applicable views,
base-only matches from exact contextual analyzer metadata, surface-only manual terms,
literal custom Unicode/NUL fields, built-in suppression, all five live statuses,
repetition/cross-document lifetime counts, source/vocabulary deletion and explicit
re-registration. Every page preset exercises a complete crossing supplementary/
combining/NUL token, strict UTF-8 clipping, off-page/non-Japanese indices, missing B,
foreign/invalid/deleted pages, off-page corrupt snapshots and forged metadata.
Reversal/replacement and newly available exact metadata invalidate frozen selections;
status/custom/count changes remain live. Dictionary/custom corruption propagates,
and explicit retries recover without cached empty results or user-state writes.

The first new focused run caught a test expectation that base==surface should be
removed from DictionaryCandidates. Existing semantics retain the analyzer base
while deduplicating query matches; corrected the assertion, with no production
workaround. The initial workspace command omitted --locked once to add the already
pinned rusqlite 0.40.2 analysis dev-dependency to Cargo.lock; final locked workspace
checks pass. No version upgrade, production dependency, migration or bridge change.
Logs: /tmp/yomoku-effective-word-{tests,full,clippy,bridge}.log.
APK/device/Android lint, release, performance, process-death and physical arm64 were
not rerun; the existing AVD was not touched. No open implementation blocker.
Next: effective registration/custom mutations and frozen future completions, then
the live reader/furigana/word integration and typed segmentation controls.

Final host PASS: real-Full Kotlin/JNI selected-word/custom-entry payloads and
Unicode/reopen/retry/counts, import/direct-cancellation regressions, navigation,
codecs and 100 handle lifecycles; all seven cross-language fixtures verify.
Known Crux middleware teardown messages and ktfmt JDK deprecation warnings remain
informational. Repeat generation is byte-identical. No escalation/network download
was required; no unresolved verification blocker.

## Effective custom-entry CRUD

2026-10-27, from the repository root with the pinned Rust/JDK environment:

```sh
export PATH=/home/kai/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin:$PATH
export JAVA_HOME=/home/kai/repos/yomoku/.local/jdk-17/jdk-17.0.21+9
cargo test -p yomoku-storage effective_custom_dictionary --all-features --locked --offline
cargo test --workspace --all-features --locked --offline --quiet
tools/test-sudachi.sh
cargo clippy --workspace --all-targets --all-features --locked --offline -- -D warnings
cargo fmt --all -- --check
tools/format-kotlin.sh --check
tools/test-kotlin-bridge.sh
cargo run -p yomoku-ffi --bin generate-kotlin --features facet_typegen --locked --offline -- target/generated/kotlin-repeat
# Compare after generation exits.
diff -r target/generated/kotlin target/generated/kotlin-repeat
git diff --check
```

PASS: seven new effective-custom cases, 508 workspace Rust tests (17 opt-in ignored),
strict Clippy/Rustfmt/Kotlin formatting/diff and byte-identical repeat generation.
All fifteen Full analysis fixtures plus one runtime fixture pass EACH owned/mapped
(32 invocations). The real segmentation fixture exercises custom CRUD for both exact
analyzer spans and unresolved manual merges, literal fields, SQLite reopen and
stale mutation rejection after reversal. Stored B/A/C and effective page/status
snapshots remain identical through the writes.

Data-free cases cover global spelling reuse across repetitions/documents/original
and effective paths; literal supplementary/combining/NUL fields; manual spans with
no invented metadata; absent/deleted vocabulary and deleted documents; word/name/
base-only built-in gates and later exact metadata; reversal/replacement/forged keys,
off-page/non-Japanese indices, missing B and invalid UTF-8 page boundaries. All page
presets preserve complete crossing identities. Read-only loads preserve file bytes;
invalid/corrupt entries and dictionary failures never permit replacement/deletion.
Identical saves/repeated deletes stay no-ops. Statement and deferred-commit failures
roll back create/edit/delete and exact retries recover.

Logs: /tmp/yomoku-effective-custom-{tests,full,clippy,bridge,codegen,kotlin-format}.log.
No dependencies, schema, bridge payloads or live reader behavior changed.
APK/device/Android lint, release, performance, process-death and physical arm64 were
not rerun; the existing AVD was untouched. No implementation blocker.
Next: effective registration and frozen future completions, then the live reader/
furigana/word switch and typed segmentation controls.

Final host PASS: real-Full selected-word/custom-entry Kotlin/JNI payloads, literal
Unicode/reopen/retry, import/direct-cancellation regressions, navigation/codecs and
100 handle lifecycles; seven cross-language fixtures verify. Known Crux middleware
teardown messages remain informational. No escalation/download was required.


## Frozen effective completion registration and history

2026-10-27, from the repository root with the pinned Rust/JDK environment:

```sh
export PATH=/home/kai/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin:$PATH
export JAVA_HOME=/home/kai/repos/yomoku/.local/jdk-17/jdk-17.0.21+9
cargo test -p yomoku-storage effective_completions --all-features --locked --offline
cargo test --workspace --all-features --locked --offline --quiet
tools/test-sudachi.sh
cargo clippy --workspace --all-targets --all-features --locked --offline -- -D warnings
cargo fmt --all -- --check
tools/format-kotlin.sh --check
tools/test-kotlin-bridge.sh
cargo run -p yomoku-ffi --bin generate-kotlin --features facet_typegen --locked --offline -- target/generated/kotlin-repeat
# Compare after both generation commands exit.
diff -r target/generated/kotlin target/generated/kotlin-repeat
git diff --check
```

PASS: nine new effective-completion storage cases, 517 workspace Rust tests (17
opt-in ignored), strict Clippy/Rustfmt/Kotlin formatting/diff. Fifteen Full analysis
fixtures plus one runtime fixture pass EACH owned/mapped (32 invocations). The real
segmentation fixture now freezes effective occurrences, reverses the rule, commits
exact surfaces/metadata/provenance, and verifies replay after SQLite reopen.

Migration 0018 adds versioned durable effective command payloads without rewriting
old history. Preparation validates the entire rendered key and drained time, with
no registration/encounters. Commit authenticates the exact reservation and shares
original-B page-only registration/New-only transitions, time/position/finish. Later
rule/metadata changes cannot reinterpret a reserved command. Old APIs reject its ID.
Unfinished reservations never auto-replay; process-restart recovery is not added.

Coverage includes repetitions, exact A metadata and non-Japanese exclusion, unresolved
manual spans, absent/deleted spellings, stale/forged/foreign/missing keys, all-preset
supplementary/combining/NUL crossings, off-page nonregistration, concurrent duplicates,
fresh zero-New rereads, deletion/reopen replay, corrupt payloads, reservation and
every-write/deferred-commit rollback/retry, old-history preservation and failed upgrade.

Initial focused compilation caught a borrowed-set loop and test-only snapshot method
names; corrected. One rollback trigger targeted DELETE while time consumption uses
UPDATE; corrected the test. The first workspace run caught historical schema version
assertions and a v11 fixture invoking the current completion repository: use an empty
TEMP reservation table only in that historical fixture, leaving its on-disk v11 schema
intact. An integration assertion initially requested rusqlite u64 FromSql; use i64
with checked conversion. Final workspace/Clippy/Full runs pass with no production
workaround. Logs: /tmp/yomoku-effective-completions-{tests,full,clippy,bridge,codegen,kotlin-format}.log.

No dependencies/versions, public bridge payloads or live Android behavior changed.
APK/device/Android lint, release, performance, process-death and physical arm64 were
not rerun; existing AVD unchanged. Effective live completion/token/furigana/word/custom
integration, full-chapter payload-cost review, atomic override-time registration and
core/FIFO/FFI/Android controls remain. No implementation blocker.


Final host PASS: real-Full Kotlin/JNI selected-word/custom CRUD, imports/direct
cancellation, navigation/codecs, seven cross-language fixtures and 100 native handle
lifecycles. Repeat Kotlin generation is byte-identical. Existing Crux middleware
teardown messages remain informational. No escalation/download or blocker.


## Atomic segmentation-override registration

2026-10-27, from the repository root:

```sh
export PATH=/home/kai/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin:$PATH
cargo test -p yomoku-storage registered_override --all-features --locked --offline
cargo test --workspace --all-features --locked --offline --quiet
tools/test-sudachi.sh
cargo clippy --workspace --all-targets --all-features --locked --offline -- -D warnings
cargo fmt --all -- --check
git diff --check
```

PASS: seven new storage cases; 524 workspace Rust tests (17 opt-in ignored), strict
Clippy, Rustfmt and diff checks. Fifteen Full analysis fixtures plus one runtime
fixture pass EACH owned/mapped (32 invocations). The real segmentation fixture now
checks atomic A/B/C choice registration, identical retry no-ops, immutable original
B and unchanged completion/occurrence counts after SQLite reopen.

Coverage includes all five preserved statuses, repeated parts/occurrences, local
precedence and exact non-Japanese metadata, missing/corrupt whole snapshots/rules,
foreign/gapped/invalid UTF-8 spans, supplementary/combining/NUL spelling identity,
concurrent saves, existing active-rule retries after deletion/reopen, fresh saves
after reversal, unrelated deleted-word nonregistration, nonempty completed history,
pending time/progress/finish preservation, replacement/part/vocabulary/status-event
statement failure and deferred-COMMIT rollback. Only selected winning parts register;
other occurrences/documents are not rescanned for registration.

Initial checks caught an incorrect test call to the two-argument analyzed completion
API, and reuse of a moved saved-snapshot fixture vector; corrected the fixtures.
A retry test initially enabled SQLite query_only before BEGIN IMMEDIATE, which SQLite
rejects even without row writes. Use aborting write triggers plus exact row/file-byte
comparisons to verify the no-op instead. No production workaround was needed. Final
focused/workspace/Full/Clippy checks all pass. Logs:
/tmp/yomoku-registered-overrides-{tests,full,clippy}.log.

No dependency/version, migration, bridge or Android source changes. Host Kotlin/JNI,
repeat codegen, Kotlin formatting, APK/device/Android lint, release, performance,
process-death and physical arm64 were not rerun; AVD untouched. No escalation/download
or unresolved blocker. Next: integrate effective completion/token/word/custom/furigana
through core/FIFO/FFI and review payload cost; guarded typed override controls and
Android A/B/C/manual scope selection remain after that. The current reader uses B.

## Effective furigana and synthetic payload review

2026-10-27, from the repository root:

```sh
export PATH=/home/kai/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin:$PATH
export JAVA_HOME=/home/kai/repos/yomoku/.local/jdk-17/jdk-17.0.21+9
cargo test -p yomoku-core --test effective_furigana --test reader_furigana --all-features --locked --offline
cargo test --workspace --all-features --locked --offline --quiet
tools/test-sudachi.sh
cargo run -p yomoku-storage --example measure_effective_reader_payload --locked --offline
cargo clippy --workspace --all-targets --all-features --locked --offline -- -D warnings
cargo fmt --all -- --check
tools/format-kotlin.sh --check
tools/test-kotlin-bridge.sh
cargo run -p yomoku-ffi --bin generate-kotlin --features facet_typegen --locked --offline -- target/generated/kotlin-repeat
# Compare after both generation commands exit.
diff -r target/generated/kotlin target/generated/kotlin-repeat
git diff --check
```

PASS: five new effective-furigana cases and three original-B regression cases;
529 workspace Rust tests (17 opt-in ignored), strict Clippy, Rustfmt and Kotlin
formatting. Fifteen Full analysis fixtures plus one runtime fixture pass EACH
owned/mapped (32 invocations). Real A/B/C segmentation checks now project furigana
from the same frozen resolution and verify literal source-ruby precedence.

Coverage includes exact B/A/C readings and contextual repetitions, unresolved manual
and non-Japanese exclusions, missing readings, partial/multiple source overlaps,
reversal and old-resolution stability, all five presets, complete crossing readings,
supplementary/combining/NUL text and UTF-16 offsets, foreign/empty/malformed pages and
off-page source-ruby corruption. Both projections share one source-precedence helper.

The payload example reproduces full-key JSON sizes of 139,145 / 1,409,312 / 14,290,979
bytes for 1k / 10k / 100k synthetic characters. Visible page tokens at 10k and 100k
stay 42,519 bytes. EFFECTIVE_READER_PAYLOAD.md records the fixture, exact page sizes,
interpretation and limits; this does not measure JNI bytes or actual device latency.
No dependency/version, migration, public schema or Android source changes.

Logs: /tmp/yomoku-effective-furigana-{tests,full,clippy,bridge,payload,kotlin-format}.log.
APK/device/Android lint, release, real-book performance, process-death and physical
arm64 were not rerun; the existing AVD was untouched. No implementation blocker.
Next: effective bridge identity/lifecycle using the payload evidence, coordinated
completion/token/word/custom/furigana integration, then guarded typed override UI.

Final PASS: real-Full Kotlin/JNI selected-word/custom CRUD, all file imports/direct
cancellation, navigation/codecs, seven cross-language Unicode fixtures and 100 native
handle lifecycles. Existing Crux middleware teardown messages remain informational.
Repeat generation is byte-identical. No download or unresolved verification failure.
Git staging/commit requires the already authorized filesystem escalation because
the sandbox mounts `.git` read-only; source edits/checks need no escalation.

## Compact effective reader identity/lifecycle prerequisite

2026-10-27, repository root, pinned offline Rust/JDK:

```sh
export PATH=/home/kai/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin:$PATH
export JAVA_HOME=/home/kai/repos/yomoku/.local/jdk-17/jdk-17.0.21+9
cargo test -p yomoku-core --test effective_session --all-features --locked --offline
cargo test --workspace --all-features --locked --offline --quiet
tools/test-sudachi.sh
cargo run -p yomoku-storage --example measure_effective_reader_payload --locked --offline
cargo clippy --workspace --all-targets --all-features --locked --offline -- -D warnings
cargo fmt --all -- --check
tools/format-kotlin.sh --check
tools/test-kotlin-bridge.sh
cargo run -p yomoku-ffi --bin generate-kotlin --features facet_typegen --locked --offline -- target/generated/kotlin-repeat
# Compare after generation exits.
diff -r target/generated/kotlin target/generated/kotlin-repeat
git diff --check
```

PASS: eight new Rust cases (seven integration, one counter-exhaustion unit), 537
workspace Rust tests (17 opt-in ignored), strict Clippy/Rustfmt/Kotlin formatting.
Fifteen Full analysis fixtures plus one runtime fixture pass EACH owned/mapped
(32 invocations). Repeat Kotlin generation is byte-identical.

The pure core component retains one full key and compact view, with unique per-load
revisions and immediate invalidation. Coverage includes late/duplicate/foreign loads,
same-page explicit retry, Missing B, clipped crossings/literal Unicode, punctuation/
off-page rejection, all live statuses/deleted vocabulary, unresolved manual parts,
metadata changes and separately retained command keys across invalidation.

The payload example preserves the original measurements and adds compact page JSON:
66,233 / 32,983 / 32,983 bytes for 1k/10k/100k synthetic chapter characters, with
136-byte revision-1 identity and 163-byte selection JSON. Generated Bincode codec
round trips use 60/68 bytes for available-B identity/selection, including maximum
unsigned revision/index, large IDs, literal supplementary/combining/NUL text and
resolved/unresolved metadata. These are serialized sizes, not device performance.

Initial host Kotlin compilation caught assertions using sealed-type names for
generated unit enums. Corrected to the generator's uppercase enum values and reran
the complete host suite. Known Crux middleware teardown messages remain informational.
Logs: /tmp/yomoku-effective-session-{tests,full,clippy,bridge,payload,codegen,kotlin-format}.log.

No dependencies, migrations or Android source changes. New schemas are standalone
prerequisites; live reader events/views/completion/token/word/custom/furigana still
use original B. APK/device/Android lint/release, real-book memory/JNI latency,
process-death and physical arm64 were not rerun; existing AVD untouched. Next: the
coordinated live-reader switch, then guarded override controls. No implementation
blocker; local staging/commit needs the existing authorized `.git` escalation.

## Native effective-reader FIFO protocol

2026-10-27, repository root, pinned offline Rust/JDK:

```sh
export PATH=/home/kai/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin:$PATH
export JAVA_HOME=/home/kai/repos/yomoku/.local/jdk-17/jdk-17.0.21+9
cargo test -p yomoku-runtime effective_worker --all-features --locked --offline
cargo test -p yomoku-ffi --test effective_protocol --all-features --locked --offline
cargo test --workspace --all-features --locked --offline --quiet
tools/test-sudachi.sh
cargo clippy --workspace --all-targets --all-features --locked --offline -- -D warnings
cargo fmt --all -- --check
tools/format-kotlin.sh --check
tools/test-kotlin-bridge.sh
cargo run -p yomoku-ffi --bin generate-kotlin --features facet_typegen --locked --offline -- target/generated/kotlin-repeat
# Compare after generation exits.
diff -r target/generated/kotlin target/generated/kotlin-repeat
git diff --check
```

PASS: seven new cases (one core accepted-command ownership, five actual asynchronous
FIFO worker cases and one FFI codec exclusion), 544 workspace Rust tests (17 opt-in
ignored), strict Clippy/Rustfmt/Kotlin formatting/diff and byte-identical repeat
generation. Fifteen Full analysis fixtures plus one runtime fixture pass EACH
owned/mapped (32 invocations). Host Kotlin/JNI verifies real-Full word/custom paths,
imports/direct cancellation, navigation/codecs, seven Unicode fixtures and 100 handles.

Native effective tokens/word/custom/preparation/completion use the existing FIFO and
dictionary/error/reopen policy. Private-field accepted page/selection commands share
one immutable full-key allocation with the retained page. Invalidation retires shell
callbacks while accepted commands/retries keep their exact key. Full native protocol
variants are skipped by Serde/Facet; no full chapter or reservation enters shell
values. Storage revalidates stale provenance; reserved completion replay remains exact.

Tests verify read-only bytes/no Sudachi loads, unresolved manual fields/live statuses,
literal supplementary/combining/NUL custom values, identical-save no-ops, built-in
gates, corrupt dictionary protection/reopen/explicit retry, Missing B/deleted sources,
reversed-rule stale selection rejection, blocked-SQLite final checkpoint before
preparation, atomic completion rollback, reversal/reopen exact retry, no automatic
reservation replay and fresh reread counts. Original B remains immutable.

Initial compilation required marking the skipped native payload fields Facet-opaque;
Facet skip alone still requires nested reflection. The codec/codegen checks verify
the final representation stays native-only. Existing Crux middleware teardown and
ktfmt JDK deprecation messages remain informational. Logs:
`/tmp/yomoku-effective-worker-{tests,full,clippy,bridge,codec}.log`.

No dependency/version, migration, public shell schema or Android source changes.
Live app core/FFI/Android completion/token/word/custom/furigana integration remains;
the current reader still uses B. APK/device/Android lint/release, performance,
process-death and physical arm64 were not rerun; AVD untouched. No implementation
blocker. Local staging/commit requires the already authorized `.git` escalation.

## Retained effective word/custom lifecycle

2026-10-27, repository root, existing pinned offline Rust/JDK:

```sh
export PATH=/home/kai/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin:$PATH
export JAVA_HOME=/home/kai/repos/yomoku/.local/jdk-17/jdk-17.0.21+9
cargo test -p yomoku-core --test effective_session --all-features --locked --offline
cargo test -p yomoku-runtime effective_worker_retained --all-features --locked --offline
cargo test --workspace --all-features --locked --offline --quiet
cargo clippy --workspace --all-targets --all-features --locked --offline -- -D warnings
cargo fmt --all -- --check
tools/format-kotlin.sh --check
tools/test-sudachi.sh
tools/test-kotlin-bridge.sh
cargo run -p yomoku-ffi --bin generate-kotlin --features facet_typegen --locked --offline -- target/generated/kotlin-repeat
# Compare after both generation commands exit.
diff -r target/generated/kotlin target/generated/kotlin-repeat
git diff --check
```

PASS: eight new cases (seven pure retained-word cases and one actual SQLite FIFO
case), 552 workspace Rust tests (17 opt-in ignored), strict Clippy/Rustfmt/Kotlin
formatting/diff and byte-identical repeat generation. Fifteen Full analysis fixtures
plus one runtime fixture pass EACH owned/mapped (32 invocations). Host Kotlin/JNI
checks real-Full word/custom paths, imports/direct cancellation, navigation/codecs,
seven Unicode fixtures and 100 handles, plus the standalone compact effective word
state schema with literal custom/error fields and maximum unsigned counts.

The pure retained reader session now owns selected-word/custom lifecycle. Full
native key and visible fields authenticate results before compact presentation;
no resolved chapter enters the generated word-state codec. Fresh native callback
attempts reject stale/duplicate deliveries, including previous exact-payload retries.
Read-only lookup observes live statuses/deletion and updates repeated spellings.
Missing-match controls preserve literal fields and committed data; built-ins and
failed lookups cannot create custom commands. Pending custom payloads survive
invalidation for explicit retry, while explicit Cancel can discard a failed retry.
New explicit loads/edits can replace failed retries without accepting old callbacks.
In-flight writes drain and never resurrect dismissed presentation. The FIFO case
injects a real SQLite write failure, verifies exact repair/retry/delete, no analyzer
loads and unchanged reading/time/completion tables.

Initial targeted compilation exposed a test module-path error and an unavailable
test-only JSON import; the final tests use existing generated codecs without a new
dependency. Kotlin `-Werror` caught an unnecessary safe call in a new assertion;
the final rerun uses an explicit non-null local. Existing Crux middleware teardown
messages remain informational. Logs: `/tmp/yomoku-effective-word-state-{tests,
clippy,full,bridge,format,codegen}.log`.

No dependencies, migration or live Android source changes. This is a retained
lifecycle/codec slice; live app events/views and completion/token/word/custom/
furigana remain original B. Next adopt native page/word requests and completion
reservations together in core/FFI/Android, then guarded override controls. APK/
device/Android lint/release/performance/process-death/physical arm64 were not rerun.
No implementation blocker; local commit uses the authorized `.git` escalation.

## Retained effective completion lifecycle

2026-10-28, repository root, existing pinned offline Rust/JDK:

```sh
export PATH=/home/kai/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin:$PATH
export JAVA_HOME=/home/kai/repos/yomoku/.local/jdk-17/jdk-17.0.21+9
cargo test -p yomoku-core --test effective_session --all-features --locked --offline
cargo test -p yomoku-runtime effective_worker_retained_completion --all-features --locked --offline
cargo test --workspace --all-features --locked --offline --quiet
cargo clippy --workspace --all-targets --all-features --locked --offline -- -D warnings
cargo fmt --all -- --check
tools/format-kotlin.sh --check
tools/test-sudachi.sh
tools/test-kotlin-bridge.sh
cargo run -p yomoku-ffi --bin generate-kotlin --features facet_typegen --locked --offline -- target/generated/kotlin-repeat
# Compare after generation exits.
diff -r target/generated/kotlin target/generated/kotlin-repeat
git diff --check
```

PASS: seven new pure retained-completion cases and one actual FIFO/SQLite case;
560 workspace Rust tests (17 opt-in ignored), strict Clippy/Rustfmt/Kotlin
formatting. Fifteen Full analysis fixtures plus one runtime fixture pass EACH
owned/mapped (32 invocations).

Acceptance freezes the rendered effective key/intent before clock drain, retires UI
callbacks and gates word/custom obligations. Separate dispatch allows the app to
drain accepted/coalesced checkpoints before reservation. Fresh callback attempts
reject duplicates, stale retries, wrong-stage results and changed reservations.
Preparation retries preserve intent; commit retries share the exact reservation.
Only a consumed success returns position/time/notice inputs. The real FIFO case
blocks the first checkpoint, drains a newer absolute value, injects preparation
and completion failures, reverses the rule, reopens the worker and loses a
successful response. Final AlreadyRecorded replay leaves one completion/six
occurrences/2345 ms and consumes pending duration without duplicate effects.

Initial checks caught a wrong test enum name, an oversized progress enum (boxed
the returned document) and an assertion expecting consumed time rows to be deleted
(existing semantics reset duration while retaining revision). Corrected and reran.
A server restart interrupted the first Full run; the complete rerun passed.
Host Kotlin/JNI also passes real-Full imports/direct cancellation, selected-word/
custom CRUD, navigation/codecs, seven Unicode fixtures and 100 handle lifecycles.
Repeat code generation is byte-identical; `git diff --check` passes. Existing
Crux middleware teardown messages remain informational.
Logs: `/tmp/yomoku-effective-completion-session-{tests,clippy,full,bridge,format,codegen}.log`.

No dependency, migration, shell schema or live Android source changes. This is a
native retained-lifecycle slice. Live events/views and completion/token/word/custom/
furigana still use B; adopt the retained requests together next, then guarded
override controls. APK/device/Android lint/release/performance/process-death/arm64
were not rerun. No unresolved implementation blocker.


## Effective manual status commands and retained lifecycle

2026-10-28, repository root, existing pinned offline Rust/JDK:

```sh
export PATH=/home/kai/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin:$PATH
export JAVA_HOME=/home/kai/repos/yomoku/.local/jdk-17/jdk-17.0.21+9
cargo test -p yomoku-core --test effective_session --all-features --locked --offline
cargo test -p yomoku-storage effective_status --all-features --locked --offline
cargo test -p yomoku-runtime effective_worker_retained_status --all-features --locked --offline
cargo test --workspace --all-features --locked --offline --quiet
cargo clippy --workspace --all-targets --all-features --locked --offline -- -D warnings
cargo fmt --all -- --check
tools/format-kotlin.sh --check
tools/test-sudachi.sh
tools/test-kotlin-bridge.sh
cargo run -p yomoku-ffi --bin generate-kotlin --features facet_typegen --locked --offline -- target/generated/kotlin-repeat
# Compare after generation exits.
diff -r target/generated/kotlin target/generated/kotlin-repeat
git diff --check
```

PASS: 11 new cases (six pure retained-status, four SQLite and one actual FIFO
worker), 571 workspace Rust tests (17 opt-in ignored), strict Clippy and Rustfmt.
Fifteen Full fixtures plus one runtime fixture pass EACH owned/mapped (32 runs).
All five manual choices share the existing status/history helper with validation
of the complete rendered key, visible segment and live exact-surface ID in one
immediate transaction. Tests verify literal supplementary/combining/NUL surfaces,
same-status no-ops, repeated spellings/selected details, deleted/replaced IDs,
stale/reversed/off-page selections, statement/deferred-commit rollback and exact
retry after reopen/lost success. No dictionary/analyzer loads or reading writes.
Accepted status choices retain frozen values across invalidation with fresh
native callback attempts; pending status/word/custom/completion mutually gate.

Host Kotlin/JNI passes real-Full imports/direct cancellation, word/custom CRUD,
seven Unicode fixtures and 100 handle lifecycles. Repeat code generation is
byte-identical; Kotlin formatting and diff check pass. Existing Crux middleware
teardown messages remain informational.

No dependency, migration, shell schema or live Android change. Live reader still
uses original B; include status in the coordinated token/word/custom/completion/
furigana switch next. APK/device/Android lint/release/performance/process-death/
physical arm64 not rerun. No implementation blocker or initial check failures.
Logs: `/tmp/yomoku-effective-status-{core,storage,worker,tests,clippy,full,bridge,format,codegen}.log`.

## Guarded effective override repositories

2026-10-28, repository root, pinned offline Rust/JDK:

```sh
export PATH=/home/kai/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin:$PATH
export JAVA_HOME=/home/kai/repos/yomoku/.local/jdk-17/jdk-17.0.21+9
cargo test -p yomoku-storage effective_override --all-features --locked --offline
cargo test --workspace --all-features --locked --offline --quiet
cargo clippy --workspace --all-targets --all-features --locked --offline -- -D warnings
cargo fmt --all -- --check
tools/test-sudachi.sh
tools/test-kotlin-bridge.sh
git diff --check
```

PASS: 12 new SQLite cases, 583 workspace Rust tests (17 opt-in ignored), strict
Clippy/Rustfmt. Fifteen Full analysis fixtures plus one runtime fixture pass EACH
owned/mapped (32 invocations); the existing real segmentation fixture now exercises
stored A/B/C prompt choices and guarded save/reversal/stale retries. After moving
only the I/O-free snapshot types into core, workspace tests/Clippy/format pass again;
that real segmentation fixture was also rerun individually with EACH Full loader.

Contexts share the native full key through Arc and capture the rule-table revision
in the same read-only transaction as stored exact same-profile choices/winning
rules. Immediate save/register/reverse transactions reject stale resolutions,
intervening split-then-reversal and competing connections. Rollback permits an
explicit retry; successful mutations require reload, including lost responses.
Tests cover statement/deferred-commit rollback, reopen, same-rule no-ops after
vocabulary deletion, deleted-origin globals, foreign/off-page/non-Japanese spans,
changed exact metadata, literal supplementary/combining/NUL text and page crossings.
Nonempty completed history, current position/finish and pending time are preserved.

Host Kotlin/JNI passes real-Full imports/direct cancellation, selected-word/custom
CRUD, navigation/codecs, seven Unicode fixtures and 100 handle lifecycles. Existing
Crux middleware teardown messages remain informational. Initial tests exposed an
incorrect deletion fixture column and an unseeded pending-time completion fixture;
both were corrected before the passing reruns. Initial diff check also caught
extra documentation EOF blank lines; removed them. Logs:
`/tmp/yomoku-guarded-overrides-{targeted,tests,clippy,format,full,full-final,bridge}.log`.

No dependency, migration, shell schema or live Android change. Guarded storage and
pure snapshot types are complete; retained core/FIFO adapters, on-demand A/C and
Android controls remain pending. The coordinated live token/status/word/custom/
completion/furigana switch is still next. APK/device/Android lint/release/performance/
process-death/physical arm64 were not rerun. No implementation blocker.

## Retained guarded override requests and FIFO adapter

2026-10-28, repository root, existing pinned offline Rust/JDK:

```sh
export PATH=/home/kai/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin:$PATH
export JAVA_HOME=/home/kai/repos/yomoku/.local/jdk-17/jdk-17.0.21+9
cargo test -p yomoku-core --test effective_session --all-features --locked --offline
cargo test -p yomoku-runtime effective_worker_ --all-features --locked --offline
cargo test --workspace --all-features --locked --offline --quiet
cargo clippy --workspace --all-targets --all-features --locked --offline -- -D warnings
cargo fmt --all -- --check
tools/format-kotlin.sh --check
tools/test-sudachi.sh
tools/test-kotlin-bridge.sh
cargo run -p yomoku-ffi --bin generate-kotlin --features facet_typegen --locked --offline -- target/generated/kotlin-repeat
# Compare only after both generation commands finish.
diff -r target/generated/kotlin target/generated/kotlin-repeat
git diff --check
```

PASS: seven new pure cases and two real SQLite/FIFO cases; 592 workspace Rust tests
(17 opt-in ignored), strict Clippy/Rustfmt/Kotlin formatting. Fifteen Full fixtures
plus one runtime fixture pass EACH owned/mapped (32 invocations). Host Kotlin/JNI
passes real-Full imports/cancellation, selected-word/custom CRUD, navigation/codecs,
seven Unicode fixtures and 100 handle lifecycles. Repeat generation is byte-identical.

Tests cover exact shared-context/manual values/scope/timestamp across invalidation
and explicit retry/reopen; callback attempts, dismissed reads, stored A/B/C choices,
winning-rule reversal, stale reload and mutual word/custom/status/completion gates.
An injected write failure rolls back; a lost successful response makes the retained
before-image stale and never recreates subsequently deleted vocabulary. Read/error/
stale paths preserve database bytes; edits preserve history, position, finish and
nonzero pending duration. Both analyzer and word-dictionary loaders panic if invoked
by these worker cases. No automatic replay or rebase.

Initial test compilation caught a private helper and a shadowed fixture function;
the worker fixture also used the wrong pending-time column name. Corrected and
reran affected checks. Existing Crux middleware teardown/ktfmt JDK messages remain
informational. Logs: `/tmp/yomoku-retained-overrides-{core,worker,tests,clippy,
full,bridge,format,codegen}.log`.

No dependency, migration, public shell schema or live Android change. Coordinated
live token/status/word/custom/completion/furigana adoption is still next, followed
by on-demand same-profile A/C and compact override controls. APK/device/Android
lint/release/performance/process-death/physical arm64 were not rerun. No blocker.

## Guarded on-demand same-profile A/C generation

2026-10-28, repository root, pinned offline Rust/JDK:

```sh
export PATH=/home/kai/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin:$PATH
export JAVA_HOME=/home/kai/repos/yomoku/.local/jdk-17/jdk-17.0.21+9
cargo test --workspace --all-features --locked --offline --quiet
cargo clippy --workspace --all-targets --all-features --locked --offline -- -D warnings
cargo fmt --all -- --check
tools/test-sudachi.sh
tools/test-kotlin-bridge.sh
tools/format-kotlin.sh --check
cargo run -p yomoku-ffi --bin generate-kotlin --features facet_typegen --locked --offline -- target/generated/kotlin-repeat
diff -r target/generated/kotlin target/generated/kotlin-repeat
git diff --check
```

PASS: 13 new cases (two pure, seven SQLite, four real FIFO); 605 workspace Rust
tests (17 opt-in ignored), strict Clippy/Rustfmt/Kotlin formatting. Fifteen real Full
analysis fixtures plus one runtime fixture pass EACH owned/mapped (32 invocations).
The real segmentation fixture now generates A/C through guarded storage with exact
whole-chapter/profile checks. Host Kotlin/JNI passes actual Full imports/direct
cancellation, selected-word/custom CRUD, navigation/codecs, seven Unicode fixtures
and 100 handle lifecycles. Repeat code generation is byte-identical.

Coverage includes lazy/stored reuse without Full, canonical whole-chapter and
literal Unicode/crossing anchors, omitted unaligned choices, bad profile/mode/
anchors/coverage, loader/engine failure, analyzer reuse/background release,
statement/deferred-commit rollback and explicit frozen retries after reopen.
Concurrent connections can edit while analysis runs outside SQLite's transaction;
final revalidation rejects intervening edits/reversals. New metadata invalidates
old contexts, including lost success; reserved completions retain frozen meanings
and exact replay after generation/reopen. Vocabulary/history/progress/time stay
unchanged by generation.

Initial test compilation caught incorrect TextOffset conversions and an assertion
on the wrong token field. Corrected before the passing workspace/Clippy reruns;
Rustfmt then required collapsing that corrected assertion. Existing middleware/JDK
messages remain informational. Logs: `/tmp/yomoku-ondemand-{storage,core,worker,
tests,clippy,full,bridge,format,codegen}.log`.

No dependency, migration, public shell schema or Android source change. Native
analyzer mode capability is ready; coordinated live reader adoption and Android
controls remain next. APK/device/Android lint/release/performance/process-death/
physical arm64 were not rerun. No blocker.

## Compact override prompt/action contracts

2026-10-28, repository root, pinned offline Rust/JDK:

```sh
export PATH=/home/kai/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin:$PATH
export JAVA_HOME=/home/kai/repos/yomoku/.local/jdk-17/jdk-17.0.21+9
cargo test -p yomoku-core --test effective_session --all-features --locked --offline
cargo test --workspace --all-features --locked --offline --quiet
cargo clippy --workspace --all-targets --all-features --locked --offline -- -D warnings
cargo fmt --all -- --check
tools/format-kotlin.sh --check
"$JAVA_HOME/bin/java" -jar .local/ktfmt-0.64.jar --kotlinlang-style --dry-run --set-exit-if-changed tests/kotlin/EffectiveOverrideCodecTest.kt
tools/test-kotlin-bridge.sh
cargo run -p yomoku-ffi --bin generate-kotlin --features facet_typegen --locked --offline -- target/generated/kotlin-repeat
# Compare after generation exits.
diff -r target/generated/kotlin target/generated/kotlin-repeat
git diff --check
```

PASS: six new pure cases; 611 workspace Rust tests (17 opt-in ignored), strict
Clippy, Rustfmt and Kotlin formatting. The existing actual SQLite/FIFO case now
uses compact commands for literal manual save, injected rollback, exact retry after
reopen, lost success/stale rejection and winning-rule reversal, preserving all
reading/time/position data and deleted vocabulary.

Five presentation fixtures and all eight action variants (15 action fixtures)
round-trip Rust→generated Kotlin→Rust. They verify default All/both scopes,
A/B/C, winning rule IDs/parts, distinct storage/invalid/stale failures, maximum
revisions/IDs/timestamps and literal supplementary/combining/NUL parts. Prompt
identity is 68 B with available B; no full context or chapter crosses the codec.
Fresh prompt revisions reject same-page/span dismissed actions; exact write retries
keep accepted identity and fresh native callback attempts. In-flight Cancel rejects.

Host Kotlin/JNI also passes real-Full TXT/SRT/EPUB import/direct cancellation,
selected-word/custom CRUD, navigation/codecs, seven Unicode cases and 100 handle
lifecycles. Repeat generation is byte-identical; diff check passes. Initial targeted
compilation caught missing Facet enum representations; added explicit repr(C) before
all passing reruns. Existing Crux middleware teardown/JDK messages are informational.
Logs: `/tmp/yomoku-override-shell-{tests,clippy,bridge,format,codegen}.log`.

Standalone schemas/handler only: live Event/ViewModel/Android still use original B.
Next switch token/status/word/custom/completion/furigana together, then wire compact
override controls. No dependency, migration or persistent-data change. Separate
Full-loader matrix, APK/device/Android lint/release/performance/process-death/arm64
were not rerun. No implementation blocker.


## Retained effective rendering adapters

2026-10-28, repository root, pinned offline Rust/JDK:

```sh
export PATH=/home/kai/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin:$PATH
export JAVA_HOME=/home/kai/repos/yomoku/.local/jdk-17/jdk-17.0.21+9
export GRADLE_USER_HOME=/home/kai/repos/yomoku/.local/gradle
export ANDROID_HOME=/home/kai/Android/Sdk
cargo test -p yomoku-core --test effective_session --test effective_furigana --test reader_furigana --all-features --locked --offline
cargo test --workspace --all-features --locked --offline --quiet
cargo clippy --workspace --all-targets --all-features --locked --offline -- -D warnings
cargo fmt --all -- --check
tools/format-kotlin.sh --check
"$JAVA_HOME/bin/java" -jar .local/ktfmt-0.64.jar --kotlinlang-style --dry-run --set-exit-if-changed tests/kotlin/EffectiveReaderDisplayTest.kt
tools/test-kotlin-bridge.sh
cargo run -p yomoku-ffi --bin generate-kotlin --features facet_typegen --locked --offline -- target/generated/kotlin-repeat
# Compare after generation exits.
diff -r target/generated/kotlin target/generated/kotlin-repeat
./android/gradlew -p android :app:assembleDebug :app:assembleDebugAndroidTest :app:lintDebug --offline --console=plain
./android/gradlew -p android :app:connectedDebugAndroidTest -Pandroid.testInstrumentationRunnerArguments.class=app.yomoku.ReaderWordSpansTest,app.yomoku.ReaderFuriganaTest,app.yomoku.ReaderRubyLayoutTest,app.yomoku.ReaderFormattingTest --offline --console=plain
git diff --check
```

PASS: three new pure cases; 614 workspace Rust tests (17 opt-in ignored), strict
Clippy/Rustfmt/Kotlin formatting, host Kotlin/JNI and byte-identical repeat codegen.
The Kotlin runner now compiles the production Android range helper and tests
supplementary/combining/NUL text, invalid UTF-16/UTF-8 offsets, stale revisions,
foreign bases, complete crossing surfaces/selections, all statuses/deletion,
contextual repeated spellings and unresolved manual metadata guards. Existing
host JNI Full imports/direct cancellation, dictionary/custom CRUD, Unicode/codecs
and 100 handle lifecycles pass. Debug app/test APK assembly and Android lint pass.
All 10 focused emulator tests above pass on medium_phone, Android 16/API 36; these
cover the live B renderer sharing the refactored geometry, source word/link/image
identities, formatting/recreation, furigana reflow and ruby layout/theme/font scale.

The retained ruby path is compared with the validated chapter projection for
B/A/C provenance, source partial overlaps, unresolved spans, reversal, contextual
repetitions and complete crossing readings at all five page presets. New lifecycle
cases cover pending/failed/refresh/Missing B/invalidated/foreign reads and malformed
whole projections while keeping independently validated source ruby usable.
No full key is cloned/re-resolved by the production projection.

Initial focused test compilation caught a fixture borrow conflict; fixed before
all passing reruns. Gradle's sandbox wildcard-IP failure required the usual local
socket escalation; the offline build/lint then passed. Existing Crux teardown
messages remain informational. Logs: `/tmp/yomoku-effective-render-{tests,clippy,
bridge,format,format-check,codegen,android,device,emulator}.log`.

No dependency, migration or wire-schema change. The live reader still uses B;
next adopt the retained protocol and these render adapters in Event/ViewModel/
FFI/Compose together, then wire override controls. Separate Full-loader matrix,
full device matrix, performance/process-death/release/physical arm64 not rerun.
No implementation blocker.

## Native effective reader callback adapter

2026-10-28, repository root, pinned offline Rust/JDK:

```sh
export PATH=/home/kai/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin:$PATH
export JAVA_HOME=/home/kai/repos/yomoku/.local/jdk-17/jdk-17.0.21+9
cargo test -p yomoku-core --test effective_session --all-features --locked --offline
cargo test -p yomoku-runtime effective_worker_ --all-features --locked --offline
cargo test --workspace --all-features --locked --offline --quiet
cargo clippy --workspace --all-targets --all-features --locked --offline -- -D warnings
cargo fmt --all -- --check
tools/test-kotlin-bridge.sh
cargo run -p yomoku-ffi --bin generate-kotlin --features facet_typegen --locked --offline -- target/generated/kotlin-repeat
# Compare only after both generation commands finish.
diff -r target/generated/kotlin target/generated/kotlin-repeat
git diff --check
```

PASS: eight new cases (seven pure, one actual FIFO); 622 workspace Rust tests
(17 opt-in ignored), strict Clippy/Rustfmt and byte-identical repeat codegen.
All 53 retained-session and 15 focused effective-worker tests pass. Existing worker
token/word/custom/completion helpers now exercise native Crux callback capture and
the common response adapter, including SQLite rollback/reopen/lost success, final
checkpoint ordering and exact reservations. Additional cases cover wrong envelopes,
foreign/late/duplicate attempts, explicit retries and invalid token results consuming
only their own outstanding read. The new worker case also verifies status observing
deleted vocabulary clears the selected-word count, read-only DB bytes, override
save/reload/reversal and preserved completion partitions. Dictionary data is local;
the effective worker fixtures panic if Sudachi is loaded unexpectedly.

Host Kotlin/JNI passes real-Full TXT/SRT/EPUB imports/direct cancellation,
word/name/custom CRUD/retry, compact schemas/Unicode and 100 handle lifecycles.
Existing Crux middleware teardown diagnostics remain informational. No failed checks.
Logs: `/tmp/yomoku-effective-callback-{tests,clippy,bridge,codegen}.log`.

No dependency, migration, public wire-schema or Android source change. No new
automatic follow-up/retry: app clock/FIFO/lifecycle gates still apply. The live
reader uses B. Continue in `app.rs`: retain the effective session, use native-only
finished events carrying captured requests, and switch Event/ViewModel/FFI/Android
completion/token/status/word/custom/furigana together using `command`/`finish_request`
and existing render adapters. Then add compact override controls. Separate Full
loader matrix, APK/device/lint/release/performance/process-death/physical arm64 were
not rerun. No implementation blocker.

## Library title search

2026-10-28, repository root, pinned offline Rust/JDK:

```sh
export PATH=/home/kai/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin:$PATH
export JAVA_HOME=/home/kai/repos/yomoku/.local/jdk-17/jdk-17.0.21+9
export GRADLE_USER_HOME=/home/kai/repos/yomoku/.local/gradle
export ANDROID_HOME=/home/kai/Android/Sdk
cargo test -p yomoku-core --test library_search --test app --all-features --locked --offline
cargo test --workspace --all-features --locked --offline --quiet
cargo clippy --workspace --all-targets --all-features --locked --offline -- -D warnings
cargo fmt --all -- --check
tools/format-kotlin.sh --check
"$JAVA_HOME/bin/java" -jar .local/ktfmt-0.64.jar --kotlinlang-style --dry-run --set-exit-if-changed tests/kotlin/LibrarySearchBridgeTest.kt
tools/test-kotlin-bridge.sh
cargo run -p yomoku-ffi --bin generate-kotlin --features facet_typegen --locked --offline -- target/generated/kotlin-repeat
# Compare after generation exits; do not generate during a native build.
diff -r target/generated/kotlin target/generated/kotlin-repeat
./android/gradlew -p android :app:assembleDebug :app:assembleDebugAndroidTest :app:lintDebug --offline --console=plain
./android/gradlew -p android :app:connectedDebugAndroidTest '-Pandroid.testInstrumentationRunnerArguments.class=app.yomoku.LibrarySearchTest,app.yomoku.NavigationTest,app.yomoku.LibraryCoversTest,app.yomoku.PasteReaderTest#pasteReturnsToLibraryAndOpensFirstLogicalPageWithBackAndRecreation' --offline --console=plain
/home/kai/Android/Sdk/platform-tools/adb -s emulator-5554 shell wm size 2560x1600
/home/kai/Android/Sdk/platform-tools/adb -s emulator-5554 shell wm density 240
/home/kai/Android/Sdk/platform-tools/adb -s emulator-5554 shell cmd uimode night yes
./android/gradlew -p android :app:connectedDebugAndroidTest -Pandroid.testInstrumentationRunnerArguments.class=app.yomoku.LibrarySearchTest --offline --console=plain
/home/kai/Android/Sdk/platform-tools/adb -s emulator-5554 shell wm size reset
/home/kai/Android/Sdk/platform-tools/adb -s emulator-5554 shell wm density reset
/home/kai/Android/Sdk/platform-tools/adb -s emulator-5554 shell cmd uimode night no
git diff --check
```

PASS: four new core cases; 626 workspace Rust tests (17 opt-in ignored), strict
Clippy/Rustfmt/Kotlin formatting and byte-identical repeat codegen. The new actual
JNI case verifies title-only literal substring matching, Unicode/combining/NUL/
wildcards, explicit clearing, retained refresh, reset on a fresh native session,
unchanged SQLite bytes and no additional dictionary loading. Existing real-Full
TXT/SRT/EPUB imports/cancellation/custom/Unicode/100-handle regressions pass.

Both-ABI debug app/instrumentation APKs and strict Android lint pass. All six
focused API-36 x86_64 phone cases above pass, covering title search/clear/empty
results, retained navigation/recreation/refresh, covers and paste/reader return.
The search case also passes wide/dark at 2560x1600/240dpi. Display size, density
and light mode were restored afterward.

Initial phone verification found two test assumptions exposed by the smaller
Library list: returning to an uncomposed row needs stable-key lazy scrolling,
and cover pixel sampling must settle the row click animation. Corrected both
tests without weakening image assertions; rebuilt/linted and all six passed.
Gradle's sandbox wildcard-IP error required the usual local-socket escalation.
A direct ADB instrumentation attempt found no installed test package after
Gradle cleanup; using the Gradle test task installed it and passed the wide case.
Existing Crux teardown/JDK diagnostics remain informational. Logs:
`/tmp/yomoku-library-search-{tests,clippy,bridge,format-check,host-format-check,
codegen,android,device-initial,device,wide,emulator}.log`.

No dependency or migration. Search adds ephemeral Event/ViewModel fields only;
the full Library stays in core and matching titles are projected without writes.
The coordinated effective-reader switch remains next; Library sorts/status bars
are still unchecked. Release, separate Full-loader matrix, performance,
process-death and physical arm64 were not rerun. No remaining blocker.


## Library sorting and durable recency

2026-10-28, repository root, pinned offline Rust/JDK:

```sh
export PATH=/home/kai/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin:$PATH
export JAVA_HOME=/home/kai/repos/yomoku/.local/jdk-17/jdk-17.0.21+9
export GRADLE_USER_HOME=/home/kai/repos/yomoku/.local/gradle
export ANDROID_HOME=/home/kai/Android/Sdk
cargo test -p yomoku-storage library_order --all-features --locked --offline
cargo test --workspace --all-features --locked --offline --quiet
cargo clippy --workspace --all-targets --all-features --locked --offline -- -D warnings
cargo fmt --all -- --check
tools/format-kotlin.sh --check
"$JAVA_HOME/bin/java" -jar .local/ktfmt-0.64.jar --kotlinlang-style --dry-run --set-exit-if-changed tests/kotlin/LibrarySearchBridgeTest.kt tests/kotlin/LibrarySortBridgeTest.kt
tools/test-kotlin-bridge.sh
cargo run -p yomoku-ffi --bin generate-kotlin --features facet_typegen --locked --offline -- target/generated/kotlin-repeat
# Compare after generation exits; do not generate during a native build.
diff -r target/generated/kotlin target/generated/kotlin-repeat
./android/gradlew -p android :app:assembleDebug :app:assembleDebugAndroidTest :app:lintDebug --offline --console=plain
./android/gradlew -p android :app:connectedDebugAndroidTest '-Pandroid.testInstrumentationRunnerArguments.class=app.yomoku.LibrarySortTest,app.yomoku.LibrarySearchTest,app.yomoku.NavigationTest,app.yomoku.LibraryCoversTest,app.yomoku.PasteReaderTest#pasteReturnsToLibraryAndOpensFirstLogicalPageWithBackAndRecreation' --offline --console=plain
/home/kai/Android/Sdk/platform-tools/adb -s emulator-5554 shell wm size 2560x1600
/home/kai/Android/Sdk/platform-tools/adb -s emulator-5554 shell wm density 240
/home/kai/Android/Sdk/platform-tools/adb -s emulator-5554 shell cmd uimode night yes
./android/gradlew -p android :app:connectedDebugAndroidTest -Pandroid.testInstrumentationRunnerArguments.class=app.yomoku.LibrarySortTest,app.yomoku.LibrarySearchTest --offline --console=plain
/home/kai/Android/Sdk/platform-tools/adb -s emulator-5554 shell wm size reset
/home/kai/Android/Sdk/platform-tools/adb -s emulator-5554 shell wm density reset
/home/kai/Android/Sdk/platform-tools/adb -s emulator-5554 shell cmd uimode night no
git diff --check
```

PASS: four new pure-core and seven new SQLite cases; 637 workspace Rust tests
(17 opt-in ignored), strict Clippy/Rustfmt/Kotlin formatting and byte-identical
repeat codegen. Coverage includes default recent/unopened order, Unicode/title
ties, filtered sorts, retained selection during refresh/errors/navigation, update
reconciliation, migration/data preservation, concurrent entry, counter exhaustion,
statement/deferred-commit rollback and read-only queries. Exact TXT/SRT/EPUB file
references preserve recency without decoding, analysis or database writes.

Real-Full host JNI verifies every sort, title filtering, retained navigation,
explicit-open promotion, duplicate paste preservation, fresh-session default and
durable recency after reopening, with unchanged SQLite bytes during sort/search
and no added dictionary loading. Existing imports/direct cancellation, word/name/
custom CRUD/Unicode codecs and 100 native handle lifecycles pass.
Both-ABI debug app/test APKs and strict Android lint pass. All seven phone tests
above pass on medium_phone, Android 16/API 36, including actual menu selection,
visible row order, retained recreation, search, covers and paste/reader navigation.
Both sorting/search cases also pass at 2560x1600/240dpi in dark mode. Display size,
density and light mode were restored afterward.

Initial verification caught old schema-version/order/snapshot assumptions, a
historical v16 fixture using a current-only snapshot query, generated enum spelling
and the preview's positional constructor. Updated those fixtures/preview; the new
short-Japanese TXT fixture now explicitly selects UTF-8 as required. All passing
reruns followed those fixes. Emulator/Gradle required the usual AVD/local-socket
escalation. Existing Crux teardown diagnostics remain informational. Logs:
`/tmp/yomoku-library-sort-{tests,storage,clippy,bridge,format-check,host-format-check,
codegen,android,device,wide,emulator}.log`.

Migration 0019 stores bounded ordinal recency without wall timestamps; sort choice
remains ephemeral and fresh sessions default to Last read. No dependency change.
The coordinated effective-reader switch remains next; Library status bars remain
unchecked. Separate Full-loader matrix, release/performance/process-death and
physical arm64 were not rerun here. No remaining implementation blocker.

## Library distinct-surface status bars

2026-10-28, repository root, pinned offline Rust/JDK:

```sh
export PATH=/home/kai/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin:$PATH
export JAVA_HOME=/home/kai/repos/yomoku/.local/jdk-17/jdk-17.0.21+9
export GRADLE_USER_HOME=/home/kai/repos/yomoku/.local/gradle
export ANDROID_HOME=/home/kai/Android/Sdk
cargo test -p yomoku-storage library_words --all-features --locked --offline
cargo test -p yomoku-core --test library_words --all-features --locked --offline
cargo test --workspace --all-features --locked --offline --quiet
cargo clippy --workspace --all-targets --all-features --locked --offline -- -D warnings
cargo fmt --all -- --check
tools/format-kotlin.sh --check
"$JAVA_HOME/bin/java" -jar .local/ktfmt-0.64.jar --kotlinlang-style --dry-run --set-exit-if-changed tests/kotlin/LibraryWordsBridgeTest.kt
tools/test-kotlin-bridge.sh
cargo run -p yomoku-ffi --bin generate-kotlin --features facet_typegen --locked --offline -- target/generated/kotlin-repeat
# Compare after generation exits; keep native generation/builds sequential.
diff -r target/generated/kotlin target/generated/kotlin-repeat
./android/gradlew -p android :app:assembleDebug :app:assembleDebugAndroidTest :app:lintDebug --offline --console=plain
./android/gradlew -p android :app:connectedDebugAndroidTest :app:lintDebug '-Pandroid.testInstrumentationRunnerArguments.class=app.yomoku.LibraryWordsTest,app.yomoku.LibrarySortTest,app.yomoku.LibrarySearchTest,app.yomoku.NavigationTest,app.yomoku.PasteReaderTest#pasteReturnsToLibraryAndOpensFirstLogicalPageWithBackAndRecreation,app.yomoku.TxtPickerTest#cancelBlockedReadRetainsCompletedFilesAndFreshBatchIgnoresLateProviderResult,app.yomoku.TxtPickerTest#pickerBatchRetainsEncodingAndSummarizesEveryFileWithoutOpeningReader' --offline --console=plain
/home/kai/Android/Sdk/platform-tools/adb -s emulator-5554 shell wm size 2560x1600
/home/kai/Android/Sdk/platform-tools/adb -s emulator-5554 shell wm density 240
/home/kai/Android/Sdk/platform-tools/adb -s emulator-5554 shell cmd uimode night yes
./android/gradlew -p android :app:connectedDebugAndroidTest -Pandroid.testInstrumentationRunnerArguments.class=app.yomoku.LibraryWordsTest --offline --console=plain
/home/kai/Android/Sdk/platform-tools/adb -s emulator-5554 shell wm size reset
/home/kai/Android/Sdk/platform-tools/adb -s emulator-5554 shell wm density reset
/home/kai/Android/Sdk/platform-tools/adb -s emulator-5554 shell cmd uimode night no
git diff --check
```

PASS: four new core/four new SQLite cases; 645 workspace Rust tests (17 opt-in
ignored), strict Clippy/Rustfmt/Kotlin formatting and byte-identical repeat codegen.
Cases cover global five-status changes, exact Unicode/inflection identity,
repetitions, distinct surfaces across EPUB chapters, first-B pinning against
later profiles/A/C, missing analysis versus zero words, deleted vocabulary/documents,
query-only/reopen/unchanged database bytes, retained presentation and explicit
retry/busy/navigation guards. No source/assets or dictionary load during count reads.

Real-Full host JNI verifies count refresh across two documents after manual status
and final-page completion, unchanged SQLite bytes/dictionary loading on reads,
search reuse and maximum unsigned count codecs. Existing TXT/SRT/EPUB imports,
direct cancellation, word/custom CRUD, Unicode and 100-handle regressions pass.
Both-ABI debug app/test APKs and strict Android lint pass.
All eight focused phone tests above pass on medium_phone, Android 16/API 36.
The new bar test verifies six distinct surfaces despite repeated tokens, all five
colors/proportions, exact accessible counts, two-document global refresh after
manual choices/completion and retained Activity recreation. Existing title/sort/
navigation/paste regressions and mixed picker/cancellation cases pass. The bar
case also passes at 2560x1600/240dpi in dark mode; original display settings and
light mode were restored afterward.

Initial verification caught missing default view fields and an EPUB fixture's
chapter wrapper; corrected before the passing Rust/JNI/APK reruns. The first device
bar test timed out because it polled core before Compose ran the lazy read; flushing
composition before awaiting native counts fixed it. Review also found a batch
provider-read race with automatic count scans; MainActivity now defers them (and
their Retry) for an active batch, verified by the blocked-provider cancellation and
mixed-format picker regressions. Gradle/ADB required the usual local-socket
escalation. The emulator disconnected after the successful phone suite; the first
wide attempt had no device. Restarting it and rerunning the wide case passed.
Existing Crux teardown diagnostics remain informational. No remaining blocker.
Logs: `/tmp/yomoku-library-words-{tests,core,clippy,bridge,codegen,format-check,
host-format-check,android,device,device-focused,wide,emulator}.log`.

No dependency or migration. Title/progress listing and duplicate preflight remain
independent of this lazy token query. Counts match the current original-B reader;
adopt effective membership with the coordinated live-reader switch before exposing
boundary controls. Library scan performance, separate Full-loader matrix, release,
process-death and physical arm64 were not measured/rerun here.

## Effective Library membership storage

2026-10-28, repository root, pinned offline Rust:

```sh
export PATH=/home/kai/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin:$PATH
cargo test -p yomoku-storage library_words --all-features --locked --offline
cargo test --workspace --all-features --locked --offline --quiet
python3 tools/verify-sudachi-source.py
export YOMOKU_SUDACHI_DICTIONARY=/home/kai/repos/yomoku/.local/sudachi/full-20261015/system_full.dic
for loader in owned mapped; do
    YOMOKU_SUDACHI_STORAGE="$loader" cargo test -p yomoku-analysis --test full_dictionary real_segmentation_choices_resolve_scopes_merges_and_reversal_without_replacing_b --locked --offline -- --ignored
done
cargo clippy --workspace --all-targets --all-features --locked --offline -- -D warnings
cargo fmt --all -- --check
git diff --check
```

PASS: seven new SQLite cases (11 Library storage cases total), 652 workspace Rust
tests (17 opt-in ignored), strict Clippy/Rustfmt/diff checks. The existing real
Full A/B/C segmentation fixture now checks effective Library counts against
registered distinct reader surfaces; it passes separately with owned and mapped
Full. Pinned Sudachi source/patch verification passes.

Coverage includes split/merge/complete-B-boundary matching, global/local/newer
precedence/reversal, exact same-profile classification and later-profile exclusion,
literal supplementary/combining/inflected spelling, all five live statuses across
connections, deleted/unregistered entries, Missing B versus zero, cross-chapter
deduplication and global rules after origin deletion. Query-only/unchanged database
bytes and preserved history/position/command rows verify no writes. Corrupt
available analyses/rules/statuses fail closed. Damaged original EPUB/image BLOBs
do not affect these canonical-text reads.

Initial focused compilation caught a missing test import and two rusqlite iterator
lifetimes; corrected before passing reruns. No remaining blocker. Logs:
`/tmp/yomoku-effective-library-{tests,clippy,full-owned,full-mapped}.log`.

No dependency, migration or wire-schema changes. Live Library/reader still use B;
switch the existing FIFO call to `effective_library_word_distributions` with the
coordinated core/FFI/Android reader adoption. APK/lint/device, host Kotlin/JNI,
separate full-loader matrix, release/process-death/physical arm64 and real-book
performance were not rerun/measured for this Rust storage slice.


## Confirmed single-document deletion

2026-10-28, repository root, pinned offline Rust/JDK:

```sh
export PATH=/home/kai/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin:$PATH
export JAVA_HOME=/home/kai/repos/yomoku/.local/jdk-17/jdk-17.0.21+9
export GRADLE_USER_HOME=/home/kai/repos/yomoku/.local/gradle
export ANDROID_HOME=/home/kai/Android/Sdk
cargo test -p yomoku-core --test document_deletion --all-features --locked --offline
cargo test --workspace --all-features --locked --offline --quiet
# After converting older deletion simulations to the production repository API:
cargo test -p yomoku-storage --all-features --locked --offline --quiet
cargo clippy --workspace --all-targets --all-features --locked --offline -- -D warnings
cargo fmt --all -- --check
tools/format-kotlin.sh --check
"$JAVA_HOME/bin/java" -jar .local/ktfmt-0.64.jar --kotlinlang-style --dry-run --set-exit-if-changed tests/kotlin/DocumentDeletionBridgeTest.kt
tools/test-kotlin-bridge.sh
# Wait for native builds/generation to exit before repeating generation.
cargo run -p yomoku-ffi --bin generate-kotlin --features facet_typegen --locked --offline -- target/generated/kotlin-repeat
diff -r target/generated/kotlin target/generated/kotlin-repeat
./android/gradlew -p android :app:assembleDebug :app:assembleDebugAndroidTest :app:lintDebug --offline --console=plain
./android/gradlew -p android :app:connectedDebugAndroidTest '-Pandroid.testInstrumentationRunnerArguments.class=app.yomoku.DocumentDeletionTest,app.yomoku.LibraryWordsTest,app.yomoku.LibrarySortTest,app.yomoku.LibrarySearchTest,app.yomoku.LibraryCoversTest,app.yomoku.NavigationTest,app.yomoku.PasteReaderTest#pasteReturnsToLibraryAndOpensFirstLogicalPageWithBackAndRecreation,app.yomoku.TxtPickerTest#cancelBlockedReadRetainsCompletedFilesAndFreshBatchIgnoresLateProviderResult' --offline --console=plain
git diff --check
```

PASS: four pure-core, three SQLite and one actual FIFO case; 660 workspace Rust
tests (17 opt-in ignored), all 294 storage cases after replacing older simulated
deletions, strict Clippy/Rustfmt/Kotlin formatting and byte-identical repeat codegen.
The SQLite cases verify completed/partial time, occurrence/status/analysis history,
custom entries, reversible global rules and source/chapter provenance; exact-byte
idempotent deletion/reopen; fresh reimport IDs/progress/finish/recency; unrelated
documents; TXT ruby/SRT cues/multi-chapter EPUB assets; cascade/statement/deferred
COMMIT rollback and retry. Existing original/effective completion replay and word
count regressions now call the production deletion API.

Real-Full host JNI verifies confirmation/Cancel without database changes, successful
deletion without dictionary loads, shared vocabulary/encounter counts, fresh reimport
and reopened sessions. Existing imports/direct cancellation/selected-word/custom/
Unicode/100-handle JNI cases pass. Both-ABI app/test APKs and strict Android lint pass.

No migration/dependency. Deletion follows the specified history/vocabulary retention
policy, using one existing-FIFO transaction and native core confirmation state.
Rename, bulk/Select All and the coordinated effective-reader switch remain pending.
Separate Full-loader matrix, release/process-death, physical arm64 and performance
were not rerun/measured.

Device follow-up commands after correcting the test fixture:

```sh
./android/gradlew -p android :app:connectedDebugAndroidTest :app:lintDebug -Pandroid.testInstrumentationRunnerArguments.class=app.yomoku.DocumentDeletionTest --offline --console=plain
/home/kai/Android/Sdk/platform-tools/adb -s emulator-5554 shell wm size 2560x1600
/home/kai/Android/Sdk/platform-tools/adb -s emulator-5554 shell wm density 240
/home/kai/Android/Sdk/platform-tools/adb -s emulator-5554 shell cmd uimode night yes
./android/gradlew -p android :app:connectedDebugAndroidTest -Pandroid.testInstrumentationRunnerArguments.class=app.yomoku.DocumentDeletionTest --offline --console=plain
/home/kai/Android/Sdk/platform-tools/adb -s emulator-5554 shell wm size reset
/home/kai/Android/Sdk/platform-tools/adb -s emulator-5554 shell wm density reset
/home/kai/Android/Sdk/platform-tools/adb -s emulator-5554 shell cmd uimode night no
```

The nine existing phone Library/bar/search/sort/cover/navigation/paste/batch-cancel
regressions pass. The new phone deletion case passes after its initial assertion
confused a fully empty Library with an empty title filter; it now seeds a surviving
document and checks both retained history and that unrelated live row. Actual UI
coverage includes confirmation text, Cancel, retained Activity recreation before
confirmation and after a forced SQLite failure, explicit Retry, filtered-row removal,
retained query and fresh reimport without progress/finish/recency. Lint passes again
with the corrected fixture.

Initial compilation caught the import-layer TextEncoding type, a test module path,
a reread checkpoint needing the persisted position reset, and an unnecessary Kotlin
safe call under -Werror. All were corrected before passing native/JNI reruns.
Gradle/ADB required the usual local-socket escalation. The emulator exited after the
first phone suite, so the first corrected-case launch reported no devices; restarting
medium_phone restored it and the corrected case passed. Existing Crux teardown
messages are informational. Logs: `/tmp/yomoku-library-delete-{tests,storage,focused,
clippy,bridge,codegen,format,format-check,android,device-initial,device-disconnected,
device,wide,emulator}.log`.

The deletion case also passes at 2560x1600/240dpi in dark mode on API-36 x86_64.
Original display size/density and light mode were restored. Ten distinct focused
phone cases and this wide case pass across the initial/corrected runs; no remaining
implementation blocker. No production code changed after the passing native/JNI
suite; only the corrected device fixture and documentation changed afterward.


## Library rename

2026-10-28, repository root, pinned offline Rust/JDK:

```sh
export PATH=/home/kai/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin:$PATH
export JAVA_HOME=/home/kai/repos/yomoku/.local/jdk-17/jdk-17.0.21+9
export GRADLE_USER_HOME=/home/kai/repos/yomoku/.local/gradle
export ANDROID_HOME=/home/kai/Android/Sdk
cargo test --workspace --all-features --locked --offline --quiet
cargo clippy --workspace --all-targets --all-features --locked --offline -- -D warnings
cargo fmt --all -- --check
tools/format-kotlin.sh --check
"$JAVA_HOME/bin/java" -jar .local/ktfmt-0.64.jar --kotlinlang-style --dry-run --set-exit-if-changed tests/kotlin/DocumentRenameBridgeTest.kt
tools/test-kotlin-bridge.sh
# After native builds/generation finish:
cargo run -p yomoku-ffi --bin generate-kotlin --features facet_typegen --locked --offline -- target/generated/kotlin-repeat
diff -r target/generated/kotlin target/generated/kotlin-repeat
./android/gradlew -p android :app:assembleDebug :app:assembleDebugAndroidTest :app:lintDebug --offline --console=plain
./android/gradlew -p android :app:connectedDebugAndroidTest '-Pandroid.testInstrumentationRunnerArguments.class=app.yomoku.DocumentRenameTest,app.yomoku.DocumentDeletionTest,app.yomoku.LibraryWordsTest,app.yomoku.LibrarySortTest,app.yomoku.LibrarySearchTest,app.yomoku.LibraryCoversTest,app.yomoku.NavigationTest,app.yomoku.PasteReaderTest#pasteReturnsToLibraryAndOpensFirstLogicalPageWithBackAndRecreation,app.yomoku.TxtPickerTest#cancelBlockedReadRetainsCompletedFilesAndFreshBatchIgnoresLateProviderResult' --offline --console=plain
/home/kai/Android/Sdk/platform-tools/adb -s emulator-5554 shell wm size 2560x1600
/home/kai/Android/Sdk/platform-tools/adb -s emulator-5554 shell wm density 240
/home/kai/Android/Sdk/platform-tools/adb -s emulator-5554 shell cmd uimode night yes
./android/gradlew -p android :app:connectedDebugAndroidTest -Pandroid.testInstrumentationRunnerArguments.class=app.yomoku.DocumentRenameTest --offline --console=plain
/home/kai/Android/Sdk/platform-tools/adb -s emulator-5554 shell wm size reset
/home/kai/Android/Sdk/platform-tools/adb -s emulator-5554 shell wm density reset
/home/kai/Android/Sdk/platform-tools/adb -s emulator-5554 shell cmd uimode night no
git diff --check
```

PASS: eight new Rust cases (four core/three SQLite/one FIFO); 668 workspace tests
(17 opt-in ignored), strict Clippy/Rustfmt/Kotlin formatting and byte-identical
repeat codegen. Storage compares every table outside documents plus full reader/
Library snapshots, including completed/partial time, source provenance/history,
original bytes, ruby/cues, EPUB chapters/assets, analyses/statuses and dedup keys.
All four formats retain renamed titles on duplicate import. Exact same-title
retries preserve database bytes; statement/deferred-COMMIT failures roll back;
missing/deleted IDs cannot recreate content. Pure tests cover visibility/ID guards,
blank drafts, literal Unicode/NUL, query/sort updates, Cancel, separate errors,
double submissions, navigation and foreign callback rejection.

Real-Full host JNI verifies literal supplementary/combining/NUL titles, Cancel
without writes, progress/finish/count retention, duplicate references and reopened
sessions without additional dictionary loads. Existing import/cancellation/word/
custom/codecs/100-handle JNI regressions pass. Both-ABI app/test APKs and strict
Android lint pass. All eleven focused phone tests above pass on Android 16/API 36.
The new UI case verifies retained drafts and failed saves across Activity recreation,
blank submission gating, Cancel, forced SQLite failure/explicit Retry, search removal,
unchanged finished-book state/history/counts and renamed duplicate/reader titles.
The same rename case passes at 2560x1600/240dpi in dark mode. Original phone
display size/density and light mode were restored; no remaining blocker.

Initial test compilation needed the new ViewModel field, correct duplicate outcome
fields and a typed no-analysis closure. The storage fixture initially used a raw
legacy survivor, whose dedup backfill legitimately wrote on a later import; using
the production analyzed import fixed the fixture. Android compilation found the
positional Compose preview constructor; it now uses named fields. All corrections
precede passing reruns. Gradle/ADB needed the usual local-socket escalation.
Existing Crux teardown diagnostics remain informational. Logs:
`/tmp/yomoku-library-rename-{tests,focused,clippy,bridge,codegen,format,format-check,
android,device,wide}.log`.

No dependency or migration. The effective-reader switch/override controls and bulk
Library deletion remain pending. Separate Full-loader matrix, release/process-death,
physical arm64 and performance checks were not rerun/measured for this slice.

## Library bulk deletion

2026-10-28, repository root, pinned offline Rust/JDK:

```sh
export PATH=/home/kai/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin:$PATH
export JAVA_HOME=/home/kai/repos/yomoku/.local/jdk-17/jdk-17.0.21+9
export GRADLE_USER_HOME=/home/kai/repos/yomoku/.local/gradle
export ANDROID_HOME=/home/kai/Android/Sdk
cargo test --workspace --all-features --locked --offline --quiet
cargo clippy --workspace --all-targets --all-features --locked --offline -- -D warnings
cargo fmt --all -- --check
tools/format-kotlin.sh --check
"$JAVA_HOME/bin/java" -jar .local/ktfmt-0.64.jar --kotlinlang-style --dry-run --set-exit-if-changed tests/kotlin/BulkDocumentDeletionBridgeTest.kt
tools/test-kotlin-bridge.sh
# After native builds/generation finish:
cargo run -p yomoku-ffi --bin generate-kotlin --features facet_typegen --locked --offline -- target/generated/kotlin-repeat
diff -r target/generated/kotlin target/generated/kotlin-repeat
./android/gradlew -p android :app:assembleDebug :app:assembleDebugAndroidTest :app:lintDebug --offline --console=plain
./android/gradlew -p android :app:connectedDebugAndroidTest '-Pandroid.testInstrumentationRunnerArguments.class=app.yomoku.DocumentDeletionTest,app.yomoku.DocumentRenameTest,app.yomoku.LibraryWordsTest,app.yomoku.LibrarySortTest,app.yomoku.LibrarySearchTest,app.yomoku.LibraryCoversTest,app.yomoku.NavigationTest,app.yomoku.PasteReaderTest#pasteReturnsToLibraryAndOpensFirstLogicalPageWithBackAndRecreation,app.yomoku.TxtPickerTest#cancelBlockedReadRetainsCompletedFilesAndFreshBatchIgnoresLateProviderResult' --offline --console=plain
git diff --check
```

PASS: eight new Rust cases (four core/three SQLite/one FIFO); 676 workspace tests
(17 opt-in ignored), strict Clippy/Rustfmt/Kotlin formatting, real-Full host JNI,
byte-identical repeat codegen, both-ABI app/test APKs and strict Android lint.
Core checks cover 250 title matches, hidden selections, exact frozen confirmation,
Cancel, navigation, double submission, independent errors and explicit retries.
Storage verifies one transaction across paste/TXT/SRT/multi-chapter EPUB, retained
history/time/vocabulary/provenance/global state, fresh reimport IDs/progress/finish,
empty/duplicate/absent IDs, write-free retries and a 1,100-document selection.
Later-document text/asset/document failures and deferred-COMMIT failure roll back
every earlier deletion; reopen and explicit Retry succeed. The real FIFO fixture
verifies the same late-document failure and no analyzer activity during deletion.
JNI checks retained exact selections/Unicode titles, read-only Cancel/invalid
confirmation, shared encounter/status retention, fresh reimports and session reopen.
Existing import/direct-cancel/selected-word/custom/codecs/100-handle checks pass.

Initial fixture needed an explicit position reset before saving reread time. Kotlin
-Werror caught an unnecessary safe call; instrumentation used an unavailable Espresso
helper, replaced with the existing Activity back dispatcher; lint required plural
resources for selection/confirmation counts. Corrected before final passing runs.
Gradle/ADB/emulator needed local-socket/AVD escalation. The initial sandboxed emulator
could not update its AVD snapshot state; the normal escalated launch succeeded.
Existing Crux teardown diagnostics remain informational. No dependency or migration.
Logs: `/tmp/yomoku-library-bulk-{tests,storage,fifo,clippy,bridge,codegen,format,
format-check,android-final,device,emulator}.log`.

Device follow-up after correcting the offscreen-row fixture:

```sh
./android/gradlew -p android :app:connectedDebugAndroidTest :app:lintDebug '-Pandroid.testInstrumentationRunnerArguments.class=app.yomoku.DocumentDeletionTest#longPressSelectAllIncludesOffscreenRowsAndBulkRetryIsAtomicAcrossRecreation' --offline --console=plain
/home/kai/Android/Sdk/platform-tools/adb -s emulator-5554 shell wm size 2560x1600
/home/kai/Android/Sdk/platform-tools/adb -s emulator-5554 shell wm density 240
/home/kai/Android/Sdk/platform-tools/adb -s emulator-5554 shell cmd uimode night yes
./android/gradlew -p android :app:connectedDebugAndroidTest '-Pandroid.testInstrumentationRunnerArguments.class=app.yomoku.DocumentDeletionTest#longPressSelectAllIncludesOffscreenRowsAndBulkRetryIsAtomicAcrossRecreation' --offline --console=plain
/home/kai/Android/Sdk/platform-tools/adb -s emulator-5554 shell wm size reset
/home/kai/Android/Sdk/platform-tools/adb -s emulator-5554 shell wm density reset
/home/kai/Android/Sdk/platform-tools/adb -s emulator-5554 shell cmd uimode night no
```

The eleven existing focused phone regressions pass in the first 12-case run.
The new bulk case initially assumed sorting reset the list's scroll position;
the test now explicitly scrolls to the first row and asserts the oldest row is
offscreen rather than assuming its node is disposed. The corrected case passes
on API-36 x86_64 phone/light and 2560x1600/240dpi wide/dark. It verifies long press,
tap toggles/zero-selection gating, Select All across 16 offscreen/filtered titles,
retained native session/confirmation/failure across Activity recreation, Cancel,
Back, later-document failure with no partial deletion, exact Retry, retained query/
sort/history/status/survivor and a fresh reimport without progress/finish/recency.
Lint passes with the corrected fixture. Original 1080x2400/420dpi/light display was
restored. Twelve distinct phone cases and one wide case pass across the initial
and corrected runs; no remaining blocker. Only the test fixture/docs changed after
the passing native/JNI/production APK checks. Additional logs:
`/tmp/yomoku-library-bulk-{device-corrected,wide}.log`.

No dependency/migration. Coordinated effective-reader/Library adoption and Android
override controls remain next. Separate Full-loader matrix, release/process-death,
physical arm64 and real-book/performance checks were not rerun/measured.

## Effective reader host performance and exact metadata lookup

2026-10-28, repository root, pinned offline host Rust:

```sh
export PATH=/home/kai/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin:$PATH
cargo test -p yomoku-core --test segmentation --all-features --locked --offline
cargo test --workspace --all-features --locked --offline --quiet
python3 tools/verify-sudachi-source.py
export YOMOKU_SUDACHI_DICTIONARY=/home/kai/repos/yomoku/.local/sudachi/full-20261015/system_full.dic
for loader in owned mapped; do
    YOMOKU_SUDACHI_STORAGE="$loader" cargo test -p yomoku-analysis --test full_dictionary real_segmentation_choices_resolve_scopes_merges_and_reversal_without_replacing_b --locked --offline -- --ignored
done
cargo run -p yomoku-storage --example measure_effective_reader_payload --locked --offline
cargo clippy --workspace --all-targets --all-features --locked --offline -- -D warnings
cargo fmt --all -- --check
git diff --check
# Prepare/verify the Aozora corpus as in EFFECTIVE_READER_PERFORMANCE.md.
cargo run -p yomoku-analysis --example measure_effective_reader --release \
  --locked --offline -- .local/sudachi/full-20261015/system_full.dic \
  7 /tmp/yomoku-aozora-cat.txt
```

PASS: 677 Rust tests (17 opt-in ignored), including the new full-scan provenance
oracle over all 4,096 combinations of B/A/C/manual four-unit partitions. Coverage
includes exact starts with differing ends, A-before-C independent of input order,
conflicting classification, absent readings/lemmas, contextual repeated spellings,
supplementary/combining/NUL text, unresolved parts and punctuation. All prior storage
keys/completion/replay/status/custom/furigana tests pass. The real Full segmentation
fixture also passes separately with owned and mapped loaders; pinned source/patch
verification passes. Strict Clippy/Rustfmt/diff pass. Synthetic full-key/page/compact
JSON sizes match every existing value in EFFECTIVE_READER_PAYLOAD.md.

The optional benchmark uses production TXT/EPUB imports and a disposable file-backed
database. The real Aozora TXT fixture has 329,187 canonical characters/211,016 B tokens.
Sequential optimized before/after runs with identical workloads, warmup and seven
samples measure pure resolution, original/effective Library scans and retained page
load/accept under B-only, stored-A/B/C and global-split phases. Database SHA-256 stays
unchanged during reads. Full results/corpus hashes/reproduction/limitations are in
EFFECTIVE_READER_PERFORMANCE.md. B-only pure median drops 64.946→41.270 ms; stored-A/B/C
repository reads still cost hundreds of milliseconds. Whole-chapter A/C analysis and
storage setup are reported separately. A synthetic two-chapter EPUB + TXT + duplicate
TXT smoke run (two iterations) verifies multi-input dedup, largest nonfirst chapter,
even medians and scratch cleanup; five invalid CLI cases reject without importing.

The first corpus download needed network escalation after sandbox DNS failure.
Exploratory timings that overlapped compilation were discarded. Comparing separate
source copies in a shared Cargo target reused the last executable; the production
source/example were explicitly rebuilt and distinct executable hashes verified
before the sequential final runs. Future comparisons should use separate targets.
No remaining implementation blocker. Logs: `/tmp/yomoku-effective-performance-`
`{focused,tests,clippy,full-owned,full-mapped,payload}.log` and
`/tmp/yomoku-effective-measure-{before-final,after-final,smoke}.{csv,log}`.

No dependency/migration/wire change. Live Android/Library still use B; the coordinated
switch and compact override UI remain next. APK/Android lint/JNI/codegen/device,
full separate loader matrix, real EPUB/large-Library performance, peak allocations,
guarded on-demand FIFO timings, release APK/process death and physical arm64 were
not rerun/measured for this host-only slice.

## Real EPUB / repeated-book Library scaling

2026-10-28, repository root, pinned offline host Rust:

```sh
export PATH=/home/kai/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin:$PATH
cargo test -p yomoku-storage effective_library --all-features --locked --offline
cargo test -p yomoku-analysis --example measure_effective_reader --locked --offline
cargo test --workspace --all-features --locked --offline --quiet
cargo clippy --workspace --all-targets --all-features --locked --offline -- -D warnings
cargo fmt --all -- --check
export YOMOKU_SUDACHI_DICTIONARY=/home/kai/repos/yomoku/.local/sudachi/full-20261015/system_full.dic
for loader in owned mapped; do
    YOMOKU_SUDACHI_STORAGE="$loader" cargo test -p yomoku-analysis --test full_dictionary real_segmentation_choices_resolve_scopes_merges_and_reversal_without_replacing_b --locked --offline -- --ignored
done
cargo build -p yomoku-analysis --example measure_effective_reader --release --locked --offline
# Download/check the pinned EPUB as in EFFECTIVE_READER_PERFORMANCE.md.
target/release/examples/measure_effective_reader \
  .local/sudachi/full-20261015/system_full.dic 5 /tmp/yomoku-kusamakura.epub
mkdir -p .local/effective-measure
TMPDIR="$PWD/.local/effective-measure" target/release/examples/measure_effective_reader \
  .local/sudachi/full-20261015/system_full.dic 5 --epub-copies 100 /tmp/yomoku-kusamakura.epub
git diff --check
```

PASS: eight focused effective-Library cases, 678 workspace tests (17 opt-in ignored),
the additional archive-copy example test, strict Clippy/Rustfmt/diff and the selected
real Full segmentation/Library fixture with both owned/mapped loaders. The new
24-document storage case checks per-book exact Unicode spelling counts/all five
statuses, external changes between scans on the same connection, deletion/fresh-ID
re-registration, corrupt later analysis and explicit recovery after the failed scan.
Every successful/failed read compares unchanged database bytes in query-only mode.
The example test checks deterministic distinct source bytes without changing ordered
ZIP member names/compression/CRC/content, including supplementary/NUL text. Ten
invalid CLI cases reject before dictionary loading/import: missing dictionary/iteration,
0/102 iterations, missing/0/1001 copies, repeated copy option (including count 1), TXT
with copies and multiple EPUB inputs with copies.

Sequential release measurements use the unmodified IDPF Kusamakura EPUB and a
100-document/1,600-chapter repeated-book fixture, one warmup/five samples per metric.
The before executable uses production code from `ab9ed06` plus this slice's harness;
the after executable adds only transaction-local status sharing. Distinct executable
hashes were checked. Initial single-book observations from a pre-existing target
artifact are excluded; the final single-book pair was rerun with the same saved
before/after executables used for 100 books. Initial CSV files remain at
`/tmp/yomoku-epub-initial-{before,after}.csv`. No builds/tests overlap the timed runs. Each phase compares
database hashes, and normal exit removes the scratch database. Corpus hashes,
setup/results/limitations are in EFFECTIVE_READER_PERFORMANCE.md. Inputs stay out
of Git. The download needed network escalation after sandbox DNS failure.
The first cleanup assertion used an overly broad glob matching prior-session logs/
source copies; restricting it to numeric PID/timestamp scratch directories passes.
Existing unrelated artifacts were preserved.

No dependency/migration/wire change or remaining implementation blocker. Live
reader/Library adoption and compact Android override controls remain next. APK,
Android lint/JNI/codegen/device, separate full loader matrix, peak allocations,
diverse/image-heavy Libraries, guarded FIFO/on-demand latency, release/process death
and physical arm64 were not rerun/measured for this host-only slice. Logs:
`/tmp/yomoku-epub-{focused,tests,example-test,clippy,full-owned,full-mapped,build-before,build-after}.log`,
`/tmp/yomoku-epub-{before,after}.{csv,log}` and
`/tmp/yomoku-epub-100-{before,after}.{csv,log}`.

## Isolated Android compact boundary editor

2026-10-28, repository root, pinned offline tools:

```sh
export PATH=/home/kai/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin:$PATH
export JAVA_HOME=/home/kai/repos/yomoku/.local/jdk-17/jdk-17.0.21+9
export GRADLE_USER_HOME=/home/kai/repos/yomoku/.local/gradle
export ANDROID_HOME=/home/kai/Android/Sdk
cargo test -p yomoku-core --test effective_session --all-features --locked --offline
tools/test-kotlin-bridge.sh
tools/format-kotlin.sh --check
"$JAVA_HOME/bin/java" -jar .local/ktfmt-0.64.jar --kotlinlang-style --dry-run --set-exit-if-changed tests/kotlin/ReaderBoundaryDraftTest.kt
./android/gradlew -p android :app:assembleDebug :app:assembleDebugAndroidTest :app:lintDebug --offline --console=plain
./android/gradlew -p android :app:connectedDebugAndroidTest -Pandroid.testInstrumentationRunnerArguments.class=app.yomoku.ReaderBoundaryEditorTest --offline --console=plain
/home/kai/Android/Sdk/platform-tools/adb -s emulator-5554 shell wm size 2560x1600
/home/kai/Android/Sdk/platform-tools/adb -s emulator-5554 shell wm density 240
/home/kai/Android/Sdk/platform-tools/adb -s emulator-5554 shell cmd uimode night yes
./android/gradlew -p android :app:connectedDebugAndroidTest -Pandroid.testInstrumentationRunnerArguments.class=app.yomoku.ReaderBoundaryEditorTest --offline --console=plain
/home/kai/Android/Sdk/platform-tools/adb -s emulator-5554 shell wm size reset
/home/kai/Android/Sdk/platform-tools/adb -s emulator-5554 shell wm density reset
/home/kai/Android/Sdk/platform-tools/adb -s emulator-5554 shell cmd uimode night no
git diff --check
```

PASS: 53 retained-session Rust tests, host Kotlin/JNI suite with real Full imports,
new pure Kotlin partition checks, app/test APKs, strict Android lint, Kotlin
formatting and diff checks. All eight Compose cases pass on the API-36 x86_64
phone (1080×2400, 420 dpi) and tablet (2560×1600, 240 dpi) configurations.
Original phone dimensions/density/system-light mode were restored afterward.

The eight Compose cases use compact typed fixtures in a debug-only, non-exported
ComponentActivity without a native app session. They verify default All/This,
stored modes/explicit missing-C generation, exact large-ID actions, global/local
reversal, literal supplementary/combining/NUL partitions, whitespace guards,
busy/pending gates, exact native Retry/Cancel identities, explicit reload after
stale/invalid/failed reads, dismissal, prompt/session replacement and saved-state
restoration. Long spans scroll at 1.6 font scale in both local light/dark themes;
the fixture intentionally controls its theme independently of system mode.
The host helper additionally checks every scalar partition of mixed-script/
supplementary/combining/NUL/whitespace samples against exact text preservation,
strict surrogate validation and Rust/JVM whitespace differences.

The initial build rejected an unnecessary `!!` under Kotlin -Werror; removed.
The first device run found cuts/scope saved inside Dialog's separate view registry
were lost by the restoration test. Hoist the saveable holder above Dialog. A
second failure exposed nullable inner state restoration on owner replacement;
reject the complete MutableState holder so rememberSaveable initializes a fresh
draft, rather than accepting MutableState(null). The same-session restoration,
old-owner saved-state rejection and prompt-change cases then pass. A visual check
also led to visible scope radio controls and explicit unavailable-mode labels.
Gradle/ADB/emulator required the usual local-socket/AVD sandbox escalation.

No native/schema/dependency/wire change. Tests here do not prove live JNI editor
integration, persistence or device performance: the reader still uses original B.
Next coordinate the reader/Library switch, then open validated original-B spans
and connect this component to retained events/state with shared time/FIFO gates.
Full workspace/Clippy, separate Full owned/mapped matrix, process death, release,
physical arm64 and diverse-book performance were not rerun for this UI slice.
Logs: `/tmp/yomoku-boundary-{core,host,android,device,tablet,format-check}.log`;
initial failures: `/tmp/yomoku-boundary-device-{initial,restore-holder}.log`.


## Staged effective application pages and completion

2026-10-29, repository root, pinned offline tools:

```sh
export PATH=/home/kai/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin:$PATH
export JAVA_HOME=/home/kai/repos/yomoku/.local/jdk-17/jdk-17.0.21+9
export GRADLE_USER_HOME=/home/kai/repos/yomoku/.local/gradle
export ANDROID_HOME=/home/kai/Android/Sdk
cargo test -p yomoku-core --test effective_app --all-features --locked --offline
cargo test -p yomoku-runtime effective_application_fifo --all-features --locked --offline
cargo test --workspace --all-features --locked --offline --quiet
cargo clippy --workspace --all-targets --all-features --locked --offline -- -D warnings
cargo fmt --all -- --check
tools/test-kotlin-bridge.sh
tools/format-kotlin.sh --check
"$JAVA_HOME/bin/java" -jar .local/ktfmt-0.64.jar --kotlinlang-style --dry-run --set-exit-if-changed tests/kotlin/TextBoundaryTest.kt
./android/gradlew -p android :app:assembleDebug :app:assembleDebugAndroidTest :app:lintDebug --offline --console=plain
./android/gradlew -p android :app:connectedDebugAndroidTest -Pandroid.testInstrumentationRunnerArguments.class=app.yomoku.CoreSessionTest,app.yomoku.ReaderNavigationTest,app.yomoku.ReaderClockSessionTest --offline --console=plain
git diff --check
```

PASS: 685 Rust tests (17 opt-in ignored), including six new actual-app cases and
one actual-app/FIFO/SQLite case; strict Clippy/Rustfmt/diff, host Kotlin/JNI suite
with real Full imports, new event/populated-view Rust-Kotlin-Rust fixtures,
Kotlin formatting, both-ABI app/test APKs and strict Android lint.
Kotlin regeneration compares SHA-256 for all 18 generated files before/after the
same generate-kotlin command; every file is identical.
PASS: all 11 existing bridge/navigation/clock cases on API-36 x86_64 phone
(1080×2400, 420 dpi), including retry/finish/reread/positive-only notices,
celebration/recreation, idle/movement and final-time close draining. No skipped or
failed cases. The emulator started for this run was shut down afterward.

The pure cases verify effective-only page reads/furigana, unresolved manual
metadata, no legacy B completion fallback, both final-time slots, failed saves/
preparation/commit, frozen exact retries, deferred departure, stale/malformed/
dismissed callbacks, busy-slot ownership, movement/reflow/refresh and Missing B.
The real worker fixture uses actual Yomoku app events, a failed commit trigger,
rule reversal and retry: six frozen effective occurrences, four newly Known and
1,250 ms record once. Fresh app/reopen never auto-replays; an explicit reread uses
the reversed B partition and records four more occurrences with a fresh command.
Dictionary loaders panic if called. Existing production B regressions also pass.

Initial compile errors required parent-module method visibility and an opaque
native Facet field. Clippy found the combined native Event variant too large;
boxing its captured request fixes it without changing shell codecs. Fixture
expectations were corrected to omit non-Japanese punctuation. A deferred-departure
case exposed a read still accepting its page before timer drain: invalidate the
page when deferring, retain only its matching app FIFO obligation, then drain.
All corrected before final Rust checks. Gradle first failed inside the sandbox
on wildcard-IP/local sockets; rerunning outside succeeds. ADB/emulator likewise
needed local-socket/AVD escalation. Existing host teardown diagnostics (Inner
cannot be upgraded after resolving effect) remain nonfatal; the suite exits zero.

Production still constructs Model::default. Model::with_effective_reader is a
native staging constructor, not a user setting. Remaining word/status/custom/
override app consumers, effective Library membership and Android activation are
next; no schema/dependency/data-destructive change. No new performance claim.
Separate Full owned/mapped opt-in matrix, tablet/dark, process death, release,
physical arm64 and effective JNI/device measurements were not rerun.
Logs: `/tmp/yomoku-effective-app-{focused,worker,tests,clippy,host,android,device,format}.log`.

## Staged effective application word/status/custom consumers

2026-10-29, repository root, pinned offline tools:

```sh
export PATH=/home/kai/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin:$PATH
export JAVA_HOME=/home/kai/repos/yomoku/.local/jdk-17/jdk-17.0.21+9
export GRADLE_USER_HOME=/home/kai/repos/yomoku/.local/gradle
export ANDROID_HOME=/home/kai/Android/Sdk
cargo test -p yomoku-core --test effective_app --test effective_session --all-features --locked --offline
cargo test -p yomoku-runtime effective_application_fifo --all-features --locked --offline
cargo test --workspace --all-features --locked --offline --quiet
cargo test -p yomoku-runtime --test imports load_engine_and_invalid_output_failures_roll_back_and_retry_fresh --all-features --locked --offline
# Repeat workspace tests after the initial transient import teardown assertion.
cargo test --workspace --all-features --locked --offline --quiet
cargo clippy --workspace --all-targets --all-features --locked --offline -- -D warnings
cargo fmt --all -- --check
tools/test-kotlin-bridge.sh
tools/format-kotlin.sh --check
"$JAVA_HOME/bin/java" -jar .local/ktfmt-0.64.jar --kotlinlang-style --dry-run --set-exit-if-changed tests/kotlin/TextBoundaryTest.kt
./android/gradlew -p android :app:assembleDebug :app:assembleDebugAndroidTest :app:lintDebug --offline --console=plain
git diff --check
```

PASS: 13 pure actual-app cases (seven new), 53 retained-session cases, three real
actual-app/FIFO/SQLite cases (two new), all 694 workspace tests (17 opt-in ignored),
strict Clippy/Rustfmt/diff, real-Full host Kotlin/JNI suite, eight new compact event
and populated word/status ViewModel Rust-Kotlin-Rust exchanges, Kotlin formatting,
both-ABI app/test APKs and strict Android lint. After the final exported comment
change, two identical generate-kotlin runs compare SHA-256 for all 18 generated
files; all identical. Identity/selection remain 60/68 bytes.

The app tests cover visible revision/segment guards, malformed/read failure and
explicit retry, unresolved manual metadata, all five statuses/repeated spellings,
deleted vocabulary/counts, custom literal Unicode/NUL fields, built-in gates,
rollback/reopen, exact retry/Cancel, pending error preservation after dismissal,
late native callback ownership, coalesced final time and deferred departure/reflow.
The real status fixture has panic dictionary loaders and checks no reading writes;
the real custom fixture compares unchanged DB bytes on lookup/built-in rejection,
reopens committed literal entries, and verifies no registration/status/history.

Initial test compilation needed an explicit submodule path, the actual custom
repository API (the assertion uses SQLite to read global literal entries), and a
borrow fix in the test fixture. The first full run hit the existing import test's
immediate post-drop lifecycle assertion at imports.rs:439; its isolated rerun and
the entire second workspace run pass. No unrelated production change was made.
Gradle initially failed on sandbox wildcard-IP/local sockets; the escalated rerun
passes. Existing nonfatal host teardown diagnostics remain. No remaining blocker.

Production still constructs Model::default; override app events/effective Library
and Android activation remain next. No schema/dependency change. Device/emulator,
separate Full owned/mapped opt-in matrix, release/process-death, physical arm64,
effective JNI/device performance and diverse/image-heavy costs were not rerun.
Logs: `/tmp/yomoku-effective-words-{core,worker,tests,tests-retry,import-retry,clippy,host,android,format,codegen}.log`.

## Staged effective application override consumers

2026-10-29, repository root, pinned offline tools:

```sh
export PATH=/home/kai/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin:$PATH
export JAVA_HOME=/home/kai/repos/yomoku/.local/jdk-17/jdk-17.0.21+9
export GRADLE_USER_HOME=/home/kai/repos/yomoku/.local/gradle
export ANDROID_HOME=/home/kai/Android/Sdk
cargo test -p yomoku-core --test effective_app --test effective_session --all-features --locked --offline
cargo test -p yomoku-runtime effective_application_fifo --all-features --locked --offline
cargo test --workspace --all-features --locked --offline --quiet
cargo clippy --workspace --all-targets --all-features --locked --offline -- -D warnings
cargo fmt --all -- --check
tools/test-kotlin-bridge.sh
tools/format-kotlin.sh --check
"$JAVA_HOME/bin/java" -jar .local/ktfmt-0.64.jar --kotlinlang-style --dry-run --set-exit-if-changed tests/kotlin/EffectiveOverrideCodecTest.kt
./android/gradlew -p android :app:assembleDebug :app:assembleDebugAndroidTest :app:lintDebug --offline --console=plain
cargo run -p yomoku-ffi --bin generate-kotlin --features facet_typegen --locked --offline -- target/generated/kotlin
# Repeat generation and compare SHA-256 of every generated Kotlin file.
git diff --check
```

PASS: 17 pure app cases (four new), 53 retained-session cases and all 699 workspace
tests (17 opt-in ignored), including four real app/SQLite/FIFO cases (one new).
Strict Clippy/Rustfmt/diff, Kotlin formatting, both-ABI app/test APKs and strict
Android lint pass. Two identical generator runs compare SHA-256 for all 18
generated Kotlin files; every file is unchanged.

PASS: the complete real-Full host Kotlin/JNI suite and Rust-Kotlin-Rust codec
verification, including all 15 override app events and five populated app views.
The existing nonfatal host teardown diagnostics remain; the suite exits zero.

The new app cases cover current-page/prompt guards, dismissal/malformed reads,
stored choices/manual edits/explicit A/C dispatch/reversal, immediate invalidation,
stale reload, exact frozen Retry/Cancel and both time slots before deferred exit.
The actual worker case checks read-only prompt bytes, rolled-back saves, exact
retry, local-over-global precedence, reversal, supplementary/combining/NUL manual
parts, fresh-session persistence, and unchanged reading history/manual statuses/
position. Dictionary loaders panic if this stored-choice/manual path loads them.

Initial checks corrected a stale-action rendering assertion, retained identity
expectation after dismissal, a moved test value, a missing ViewModel field in an
existing default-view fixture and the optional initial saved-position expectation.
All corrected before the final full Rust run. Gradle initially failed on sandbox
wildcard-IP/local sockets; the escalated rerun passes. No schema/dependency change.
Production remains B pending effective Library and Android activation. Device,
separate Full owned/mapped opt-in matrix, release/process death, physical arm64 and
effective JNI/device/diverse-book performance were not rerun/measured.
Logs: `/tmp/yomoku-effective-overrides-{initial,core,worker,tests,clippy,host,android,format,codegen}.log`.


## Staged effective application Library membership

2026-10-29, repository root, pinned offline tools:

```sh
export PATH=/home/kai/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin:$PATH
export JAVA_HOME=/home/kai/repos/yomoku/.local/jdk-17/jdk-17.0.21+9
export GRADLE_USER_HOME=/home/kai/repos/yomoku/.local/gradle
export ANDROID_HOME=/home/kai/Android/Sdk
cargo test -p yomoku-core --test library_words --test effective_app --all-features --locked --offline
cargo test -p yomoku-runtime effective_application_fifo --all-features --locked --offline
cargo test --workspace --all-features --locked --offline --quiet
cargo clippy --workspace --all-targets --all-features --locked --offline -- -D warnings
cargo fmt --all -- --check
tools/test-kotlin-bridge.sh
tools/format-kotlin.sh --check
./android/gradlew -p android :app:assembleDebug :app:assembleDebugAndroidTest :app:lintDebug --offline --console=plain
cargo run -p yomoku-ffi --bin generate-kotlin --features facet_typegen --locked --offline -- target/generated/kotlin
# Compare SHA-256 of every generated .kt before/after this regeneration.
git diff --check
```

PASS: all 702 workspace tests (17 opt-in ignored), strict Clippy/Rustfmt/diff,
Kotlin formatting, both-ABI app/test APKs and strict Android lint. Five core
Library cases now exercise both native models (one new); six real app/SQLite/FIFO
cases pass (two new). The new native operation rejects shell serialization in the
existing FFI protocol test. Regeneration preserves SHA-256 for all 17 `.kt` files;
the new operation is absent from the generated Kotlin schema.

PASS: the complete real-Full host Kotlin/JNI suite and Rust-Kotlin-Rust codec
verification. Existing nonfatal teardown diagnostics remain; the suite exits zero.

The new cases verify effective split membership with registered/absent parts,
deduplicated repetitions, fresh live status/deletion, reversal/reopen, production
B isolation, Missing versus zero, whole-chapter corruption and explicit recovery.
Each real scan compares unchanged database bytes; no analyzer/word dictionary is
loaded. Shared core contracts cover cache invalidation, separate errors, explicit
Retry, hidden/busy rejection and retained literal rename/frozen bulk-delete prompts.

Gradle initially failed on sandbox wildcard-IP/local sockets; the escalated rerun
passes. No schema/dependency change. Production remains B until coordinated
Android activation. Device/emulator, separate Full owned/mapped opt-in matrix,
release/process death, physical arm64 and effective JNI/device/diverse-book
performance were not rerun/measured.
Logs: `/tmp/yomoku-effective-library-{core,worker,tests,clippy,host,android,format,codegen}.log`.

## Staged Android effective word consumers

2026-10-29, repository root, pinned offline tools:

```sh
export PATH=/home/kai/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin:$PATH
export JAVA_HOME=/home/kai/repos/yomoku/.local/jdk-17/jdk-17.0.21+9
export GRADLE_USER_HOME=/home/kai/repos/yomoku/.local/gradle
export ANDROID_HOME=/home/kai/Android/Sdk
cargo test -p yomoku-core --test effective_app --test effective_session --all-features --locked --offline
tools/test-kotlin-bridge.sh
tools/format-kotlin.sh --check
./android/gradlew -p android :app:assembleDebug :app:assembleDebugAndroidTest :app:lintDebug --offline --console=plain
./android/gradlew -p android :app:connectedDebugAndroidTest -Pandroid.testInstrumentationRunnerArguments.class=app.yomoku.EffectiveReaderWordsTest,app.yomoku.ReaderWordsTest,app.yomoku.ReaderWordSpansTest --offline --console=plain
# After adding the final cases, run the complete compact component suite:
./android/gradlew -p android :app:connectedDebugAndroidTest -Pandroid.testInstrumentationRunnerArguments.class=app.yomoku.EffectiveReaderWordsTest --offline --console=plain
/home/kai/Android/Sdk/platform-tools/adb -s emulator-5554 shell wm size 2560x1600
/home/kai/Android/Sdk/platform-tools/adb -s emulator-5554 shell wm density 240
/home/kai/Android/Sdk/platform-tools/adb -s emulator-5554 shell cmd uimode night yes
./android/gradlew -p android :app:connectedDebugAndroidTest -Pandroid.testInstrumentationRunnerArguments.class=app.yomoku.EffectiveReaderWordsTest --offline --console=plain
/home/kai/Android/Sdk/platform-tools/adb -s emulator-5554 shell wm size reset
/home/kai/Android/Sdk/platform-tools/adb -s emulator-5554 shell wm density reset
/home/kai/Android/Sdk/platform-tools/adb -s emulator-5554 shell cmd uimode night no
git diff --check
```

PASS: 17 actual-app and 53 retained-session Rust tests, real-Full host Kotlin/JNI
and Rust-Kotlin-Rust codec fixtures, Kotlin formatting/diff checks, both-ABI debug
app/test APKs and strict Android lint. The initial phone run passes all 23 cases
(seven compact fixtures plus 16 existing production word/span regressions). After
adding the remaining cases/guards, all 11 compact cases pass on the phone: 27
distinct phone cases pass in total, with no skipped/failed cases.

The isolated compact fixtures verify maximum compact revisions/segment/vocabulary
IDs, unresolved manual and full crossing surfaces, all five statuses/repetitions/
deleted vocabulary, built-in protection, literal custom save/edit/delete/Cancel,
Rust whitespace differences, exact native Retry/Cancel after dismissal/retirement,
time/storage/completion/override gates, explicit page reload and later navigation,
malformed/Missing projections, stale link callbacks and native-session replacement.
The production regressions exercise real imports/dictionaries/SQLite, lifecycle,
manual status/custom errors/retries and unchanged history/word-span rendering.

PASS: all 11 compact cases also pass in the wide dark configuration
(2560×1600, 240 dpi). Phone baseline is 1080×2400, 420 dpi, system light.
Size/density/system-light were restored and the task-started emulator was stopped.
No skipped/failed cases in either compact run. These use an isolated ComponentActivity
and compact typed fixtures; they do not activate the effective native app in Android.

Initial compilation corrected an Int fixture offset to Long. Review added explicit
reload ownership and a guard against carrying that requirement onto later pages.
Final phone/wide runs include those guards plus Rust/JVM whitespace differences,
full crossing surface, built-in gate and live status/deletion cases. Gradle first
failed on sandbox wildcard-IP/local sockets; escalation passes. Existing host
teardown diagnostics remain nonfatal; the host suite exits zero. No remaining
blocker, schema, dependency or generated-code change.

Production still uses original B. Next wire this component with a retained session
key, migrate Android navigation/completion and original-B span selection/editor,
then activate core/FFI/Android together. Full workspace/Clippy, separate Full-loader
opt-in matrix, release/process death, physical arm64 and actual effective JNI/device
performance were not rerun/measured. Logs:
`/tmp/yomoku-effective-ui-{core,host,android,device,phone,tablet,format-check}.log`.
The three device-result directories share that prefix under `/tmp`.

## Staged Android effective navigation and retained session ownership

2026-10-29, repository root, pinned offline tools:

```sh
export PATH=/home/kai/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin:$PATH
export JAVA_HOME=/home/kai/repos/yomoku/.local/jdk-17/jdk-17.0.21+9
export GRADLE_USER_HOME=/home/kai/repos/yomoku/.local/gradle
export ANDROID_HOME=/home/kai/Android/Sdk
cargo test -p yomoku-core --test effective_app --test effective_session --all-features --locked --offline
tools/test-kotlin-bridge.sh
tools/format-kotlin.sh --check
./android/gradlew -p android :app:assembleDebug :app:assembleDebugAndroidTest :app:lintDebug --offline --console=plain
# Final passing phone selection (15 cases):
./android/gradlew -p android :app:assembleDebug :app:assembleDebugAndroidTest :app:lintDebug :app:connectedDebugAndroidTest -Pandroid.testInstrumentationRunnerArguments.class=app.yomoku.EffectiveReaderNavigationTest,app.yomoku.ReaderClockSessionTest,app.yomoku.CoreSessionTest,app.yomoku.NavigationTest,app.yomoku.ReaderNavigationTest#positiveCompletionToastIsConsumedOnceAndZeroIsSilent,app.yomoku.ReaderNavigationTest#celebrationAnimatesOncePerCommandAndDoesNotReplayAfterRecreation --offline --console=plain
/home/kai/Android/Sdk/platform-tools/adb -s emulator-5554 shell wm size 2560x1600
/home/kai/Android/Sdk/platform-tools/adb -s emulator-5554 shell wm density 240
/home/kai/Android/Sdk/platform-tools/adb -s emulator-5554 shell cmd uimode night yes
./android/gradlew -p android :app:connectedDebugAndroidTest -Pandroid.testInstrumentationRunnerArguments.class=app.yomoku.EffectiveReaderNavigationTest --offline --console=plain
/home/kai/Android/Sdk/platform-tools/adb -s emulator-5554 shell wm size reset
/home/kai/Android/Sdk/platform-tools/adb -s emulator-5554 shell wm density reset
/home/kai/Android/Sdk/platform-tools/adb -s emulator-5554 shell cmd uimode night no
git diff --check
```

PASS: 70 focused Rust tests, complete real-Full host Kotlin/JNI and codec suite,
Kotlin formatting/diff, both-ABI app/test APKs and strict Android lint. No native,
generated-code, schema or dependency change. Existing nonfatal host teardown
diagnostics remain; the complete host suite exits zero.

PASS: 36 distinct phone cases across the runs: all 11 compact word cases, ten
production word cases, four new actual-shell fixtures, six real JNI/SQLite clock
cases (one new), native session-key independence, two navigation/lifecycle cases
and two completion-notice/celebration cases. The final 15-case selection passes
without failures/skips. The four shell cases also pass wide/dark (2560×1600,
240 dpi); phone is 1080×2400, 420 dpi, system light. Display settings restored and
the task-started emulator stopped. No physical-arm64 or performance claim.

The new fixtures verify actual app routing to effective word consumers, exact
compact revision/position for Next/Finish/buttons/swipes, position-only Previous,
unresolved/empty valid partitions, Missing/foreign/malformed/retired pages, native
mutation/time gates, same-page revision updates and selection reset on native
session replacement. The new clock case checks that even a rejected staged
completion samples before dispatch and drains exactly 1,000 ms on immediate close
at 2,000 ms, without history/status writes. Existing clock tests cover accepted
B completion/final saves. Native effective acceptance/drain remains covered by
the focused Rust cases; production FFI stays B until span/editor activation.

Two initial unnecessary null assertions failed Kotlin's warnings-as-errors; fixed.
An initial fixture incorrectly assumed Material button semantics retain frozen
callbacks; they call the current button callback. Corrected the test to assert
the current revision, busy gates, live gesture revision and session selection reset.
The final runs contain these corrections. Gradle/ADB/AVD needed socket escalation.

The full 40-case device selection did NOT pass. Long-page
`ReaderWordsTest#movingBothDirectionsDismissesDetailsAndNeverRestoresTheSelection`
repeatedly times out in Compose while opening its long document. The following
`ReaderNavigationTest#controlsAndSwipesPersistBothDirectionsResetScrollAndCountOnlyForward`
stalls after scrolling, waiting for Activity idle; stopped the test process.
JDB shows main in `HardwareRenderer.nSyncAndDrawFrame` and instrumentation in
`ActivityScenario.onActivity`/`Instrumentation.waitForIdleSync`, at navigation
test line 138. Temporary debugger forwarding was removed.

Diagnostic comparison restored the previous ReaderScreen function from the
session-start HEAD (only an unused sessionKey parameter added for caller
compatibility). The same single ReaderWords long-page case still fails with
ComposeNotIdleException (57.772 s); baseline result is retained below. Restored
the new source byte-for-byte, rebuilt and passed the final phone selection.
This establishes a failure with the previous reader too; no speculative product
rendering fix was made. Full long-page verification remains unresolved.

Original swangle and fresh swangle with `-feature -Vulkan` both reproduce the
failure. `-gpu swiftshader` crashes at boot (exit 139); one attempted test run
therefore reports no online device. Final passing phone/wide runs use:
`emulator -avd medium_phone -no-window -no-audio -no-snapshot -gpu swangle -feature -Vulkan -camera-back none -camera-front none -accel on -cores 2`.
The remaining long-page navigation cases `failedForwardDisablesControlsAndRetryCommitsExactlyOnce`
and `finishCheckmarkPersistsThroughRetryRecreationAndRereadsWhileReopeningResetsPosition`
were not reached in this session. Investigate rendering and rerun these four
long-page cases before claiming the complete device matrix passes.

Logs: `/tmp/yomoku-effective-navigation-{core,host,android,format-check,device,phone,swiftshader-device,long-pages,baseline,final-phone,wide}.log`.
Result directories: `/tmp/yomoku-effective-navigation-{initial,retry,final-phone,wide}-results`;
baseline XML: `/tmp/yomoku-effective-navigation-baseline-results.xml`.
Full Rust/Clippy, Full owned/mapped opt-in matrix, release/process death,
physical arm64 and effective JNI/device/diverse-book costs were not rerun/measured.

## Staged original-B span selection and boundary editor integration

2026-10-29, repository root, pinned offline tools:

```sh
export PATH=/home/kai/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin:$PATH
export JAVA_HOME=/home/kai/repos/yomoku/.local/jdk-17/jdk-17.0.21+9
export GRADLE_USER_HOME=/home/kai/repos/yomoku/.local/gradle
export ANDROID_HOME=/home/kai/Android/Sdk
cargo test --workspace --all-features --locked --offline --quiet
cargo test -p yomoku-runtime effective_application_fifo_override_rollback_exact_retry_reload_and_reverse --all-features --locked --offline
cargo clippy --workspace --all-targets --all-features --locked --offline -- -D warnings
cargo fmt --all -- --check
tools/test-kotlin-bridge.sh
tools/format-kotlin.sh --check
"$JAVA_HOME/bin/java" -jar .local/ktfmt-0.64.jar --kotlinlang-style --dry-run --set-exit-if-changed tests/kotlin/EffectiveReaderDisplayTest.kt tests/kotlin/TextBoundaryTest.kt
cargo run -p yomoku-storage --example measure_effective_reader_payload --locked --offline
cargo run -p yomoku-ffi --bin generate-kotlin --features facet_typegen --locked --offline -- target/generated/kotlin
# Compare SHA-256 of all generated .kt files before/after generation.
./android/gradlew -p android :app:assembleDebug :app:assembleDebugAndroidTest :app:lintDebug --offline --console=plain
./android/gradlew -p android :app:connectedDebugAndroidTest -Pandroid.testInstrumentationRunnerArguments.class=app.yomoku.EffectiveReaderBoundariesTest,app.yomoku.ReaderBoundaryEditorTest,app.yomoku.EffectiveReaderNavigationTest,app.yomoku.EffectiveReaderWordsTest --offline --console=plain
# Final Kotlin guard; reuse the verified native binaries to avoid concurrent generation:
./android/gradlew -p android :app:assembleDebug :app:assembleDebugAndroidTest :app:lintDebug :app:connectedDebugAndroidTest -x :app:buildRustBridge -Pandroid.testInstrumentationRunnerArguments.class=app.yomoku.EffectiveReaderBoundariesTest --offline --console=plain
/home/kai/Android/Sdk/platform-tools/adb -s emulator-5554 shell wm size 2560x1600
/home/kai/Android/Sdk/platform-tools/adb -s emulator-5554 shell wm density 240
/home/kai/Android/Sdk/platform-tools/adb -s emulator-5554 shell cmd uimode night yes
./android/gradlew -p android :app:connectedDebugAndroidTest -Pandroid.testInstrumentationRunnerArguments.class=app.yomoku.EffectiveReaderBoundariesTest,app.yomoku.ReaderBoundaryEditorTest --offline --console=plain
/home/kai/Android/Sdk/platform-tools/adb -s emulator-5554 shell wm size reset
/home/kai/Android/Sdk/platform-tools/adb -s emulator-5554 shell wm density reset
/home/kai/Android/Sdk/platform-tools/adb -s emulator-5554 shell cmd uimode night no
git diff --check
```

PASS: all 704 workspace tests (17 opt-in ignored), strict Clippy/Rustfmt/diff,
Kotlin formatting, both-ABI debug app/test APKs and strict Android lint. Two new
pure cases plus expanded core/SQLite/FIFO assertions verify original B after
manual splits/reversal/reopen, every preset, whole crossing tokens, literal
supplementary/combining/NUL text, whole-chapter corruption, Missing B, unchanged
read-only bytes and no extra analyzer/dictionary load. The real app/FIFO case
now opens edits using the returned B anchors and checks they survive writes/reopen.

PASS: the complete real-Full host Kotlin/JNI and Rust-Kotlin-Rust codec suite,
including populated original-B spans, strict Unicode/contiguity/identity projection
and unchanged 60/68-byte page/selection identities. All 17 generated Kotlin files
remain byte-identical on regeneration. Existing nonfatal teardown diagnostics
remain; the sequential host suite exits zero. Synthetic visible-page JSON now
includes B edit spans: 100,885 B for the 1k-character fixture and 50,135 B for both
10k/100k fixtures; identity/selection and full-key sizes stay unchanged. See
EFFECTIVE_READER_PAYLOAD.md. These are sizes, not device performance measurements.

PASS: 28 phone cases (five new selector/editor integrations, eight independent
editor cases, four navigation and 11 word cases), without failures/skips. New
fixtures use the actual shell with compact native projections, maximum typed IDs,
original crossing Unicode spans independent of effective tokens, contiguous
endpoints/gap rejection, typed A/B/C/manual/scope/reversal, native Retry/Cancel
and explicit reload after retirement, FIFO/time/mutation gates and saveable
native-session/page ownership. They do not activate the production native model.

Initial checks corrected test enum/API names, a Missing-B fixture's new field,
a Kotlin unnecessary-safe-call warning and generated-event fixture property names.
The first host attempt raced Android's BoltFFI metadata generation and failed to
load a native callback symbol; the sequential complete retry passes. Keep these
generation steps sequential. Gradle needs local-socket escalation; emulator startup
needs AVD write access. No dependency/durable schema/destructive-semantic change.

Production still constructs Model::default (B). Next activate native/FFI/Android
and verify actual effective JNI/SQLite reader/edit/Library/completion flows. The
existing long-page HWUI stall above remains unresolved; long-page cases, separate
Full owned/mapped matrix, release/process death, physical arm64 and actual effective
JNI/device/diverse/image-heavy timings were not rerun/measured.
Logs: `/tmp/yomoku-boundary-selection-{tests,worker,clippy-final,host,host-retry,android,phone,final-phone,format-check,host-format-check,payload,emulator}.log`.

PASS: the final malformed-effective-page guard is included in all five phone
selector/editor reruns and all 13 wide/dark selector/editor cases, with no failures
or skips. Wide is 2560×1600 / 240 dpi / dark; phone is 1080×2400 / 420 dpi / light.
The wide run uses the normal buildRustBridge task; only the Kotlin-only final
phone run skips that task to avoid concurrent metadata generation. Display size,
density and system light mode were restored; the task-started emulator was stopped.
Wide log: `/tmp/yomoku-boundary-selection-wide.log`; result copies:
`/tmp/yomoku-boundary-selection-{phone,final-phone,wide}-results`.

## Serialized effective-reader activation candidate

2026-10-29, repository root, pinned offline tools:

```sh
export PATH=/home/kai/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin:$PATH
export JAVA_HOME=/home/kai/repos/yomoku/.local/jdk-17/jdk-17.0.21+9
cargo test -p yomoku-ffi --test effective_reader --all-features --locked --offline
cargo test --workspace --all-features --locked --offline --quiet
cargo clippy --workspace --all-targets --all-features --locked --offline -- -D warnings
cargo fmt --all -- --check
tools/test-kotlin-bridge.sh
git diff --check
```

PASS: five new serialized FFI/SQLite cases and all 709 workspace tests (17 opt-in
ignored), strict Clippy/Rustfmt/diff. The Rust-only effective factory uses the
existing dictionary loaders, FIFO and lifecycle; tests deserialize actual Render
callbacks and compact views instead of bypassing the bridge via Core::update.
Deterministic synthetic analysis and a tiny SQLite dictionary cover A/C generation
on the worker, stored A/B/C choices, All/default/local scope, manual edits and
reversal/reopen, original-B anchors, read-only Library membership and word loads,
all statuses/repetitions, literal NUL/combining/supplementary custom fields,
rollback and exact custom Retry after dismissal, deletion and stale selection.
Missing B and legacy commands fail closed. Completion failure/retry freezes five
occurrences and 321 ms despite an intervening reversal; furigana does not guess
manual readings and a new bridge does not replay completed work.

Initial fixture compilation used the wrong Crux request/effect types and a
nonexistent status iterator; fixed to bridge::Request/ShellEffectFfi and explicit
five statuses. The initial typography fixture omitted the Settings gate; fixed
to open/close Settings. The final five cases and complete workspace pass.

PASS: the complete existing real-Full host Kotlin/JNI suite and bidirectional
Rust/Kotlin Unicode codecs, including native navigation, persistence, file imports,
cancellation, selected words/custom CRUD, Library/document operations and 100
handle lifecycles. Existing nonfatal teardown diagnostics remain; the sequential
suite and final codec verification exit zero. This still exercises production B;
the new effective candidate is covered through the Rust serialized bridge only.

NativeBridge/Android production still use original B. This is serialized Rust
coverage, not effective JNI/device activation. No Kotlin, BoltFFI export,
dependency, durable schema or wire-type change. APK/lint/device, separate Full
owned/mapped fixtures, physical arm64 and performance were not rerun/measured.
The existing long-page HWUI stall remains unresolved. Logs:
`/tmp/yomoku-effective-ffi-{tests,clippy,host}.log`.

## Production effective-reader activation

2026-10-29, repository root, pinned offline tools:

```sh
export PATH=/home/kai/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin:$PATH
export JAVA_HOME=/home/kai/repos/yomoku/.local/jdk-17/jdk-17.0.21+9
export GRADLE_USER_HOME=/home/kai/repos/yomoku/.local/gradle
export ANDROID_HOME=/home/kai/Android/Sdk
cargo test --workspace --all-features --locked --offline --quiet
cargo clippy --workspace --all-targets --all-features --locked --offline -- -D warnings
cargo fmt --all -- --check
# Keep native/BoltFFI generation sequential with host JNI execution.
./android/gradlew -p android :app:assembleDebug :app:assembleDebugAndroidTest :app:lintDebug --offline --console=plain
tools/test-kotlin-bridge.sh
tools/format-kotlin.sh --check
"$JAVA_HOME/bin/java" -jar .local/ktfmt-0.64.jar --kotlinlang-style --dry-run --set-exit-if-changed tests/kotlin/EffectiveReaderBridgeTest.kt
# All 17 generated Kotlin file hashes must match before/after regeneration.
git diff --check

# Existing medium_phone AVD, swangle with Vulkan disabled; restore phone baseline first.
"$ANDROID_HOME/platform-tools/adb" -s emulator-5554 shell wm size reset
"$ANDROID_HOME/platform-tools/adb" -s emulator-5554 shell wm density reset
"$ANDROID_HOME/platform-tools/adb" -s emulator-5554 shell cmd uimode night no
"$ANDROID_HOME/platform-tools/adb" -s emulator-5554 install -r -t android/app/build/outputs/apk/debug/app-debug.apk
"$ANDROID_HOME/platform-tools/adb" -s emulator-5554 install -r -t android/app/build/outputs/apk/androidTest/debug/app-debug-androidTest.apk
"$ANDROID_HOME/platform-tools/adb" -s emulator-5554 shell am instrument -w -r -e class app.yomoku.EffectiveReaderProductionTest app.yomoku.test/androidx.test.runner.AndroidJUnitRunner

# 43 real-native regressions; omit the documented pre-existing long-page stall.
reader_cases=$(python3 - <<'PY'
from pathlib import Path
import re
root = Path('android/app/src/androidTest/java/app/yomoku')
classes = ['CoreSessionTest', 'DocumentSessionTest', 'ReaderWordDetailsSessionTest',
    'ReaderClockSessionTest', 'LibraryWordsTest', 'DocumentRenameTest',
    'DocumentDeletionTest', 'ReaderFormattingTest', 'ReaderFuriganaTest',
    'ReaderImagesTest', 'ImageViewerTest', 'ReaderTypographySettingsTest',
    'ReaderTypographyTest']
cases = ['app.yomoku.' + name for name in classes]
for method in re.findall(r'@Test\s+fun (\w+)', (root / 'ReaderWordsTest.kt').read_text()):
    if method != 'movingBothDirectionsDismissesDetailsAndNeverRestoresTheSelection':
        cases.append('app.yomoku.ReaderWordsTest#' + method)
cases += ['app.yomoku.ReaderNavigationTest#positiveCompletionToastIsConsumedOnceAndZeroIsSilent',
    'app.yomoku.ReaderNavigationTest#celebrationAnimatesOncePerCommandAndDoesNotReplayAfterRecreation']
print(','.join(cases))
PY
)
"$ANDROID_HOME/platform-tools/adb" -s emulator-5554 shell am instrument -w -r -e class "$reader_cases" app.yomoku.test/androidx.test.runner.AndroidJUnitRunner
# Wide/dark production controls:
"$ANDROID_HOME/platform-tools/adb" -s emulator-5554 shell wm size 2560x1600
"$ANDROID_HOME/platform-tools/adb" -s emulator-5554 shell wm density 240
"$ANDROID_HOME/platform-tools/adb" -s emulator-5554 shell cmd uimode night yes
"$ANDROID_HOME/platform-tools/adb" -s emulator-5554 shell am instrument -w -r -e class app.yomoku.EffectiveReaderProductionTest app.yomoku.test/androidx.test.runner.AndroidJUnitRunner
"$ANDROID_HOME/platform-tools/adb" -s emulator-5554 shell wm size reset
"$ANDROID_HOME/platform-tools/adb" -s emulator-5554 shell wm density reset
"$ANDROID_HOME/platform-tools/adb" -s emulator-5554 shell cmd uimode night no
```

PASS: all 710 workspace Rust tests (17 opt-in ignored), including the new pure
same-book reopen regression and strengthened explicit-reload assertions; strict
Clippy/Rustfmt/diff. PASS: complete real-Full host Kotlin/JNI suite on the final
native implementation, including real A/B/C generation/stored reuse, default All
and local/manual edits, reversal/reopen, effective Library/furigana/completion,
existing imports/cancellation/words/status/custom/document operations and 100
handle lifecycles. Both directions of all seven Unicode codecs pass; all 17
schema-generated Kotlin files remain byte-identical. Existing nonfatal native
teardown diagnostics remain; the complete host command exits zero.

PASS: both-ABI debug app/test APKs, strict Android lint, Android Kotlin formatting
and new host-test formatting. No dependency, generated wire, durable schema or
history rewrite. NativeBridge now selects the effective factory. Original B stays
immutable; full retained keys and accepted retry payloads remain native.

PASS: both new actual production JNI/Full/SQLite/Compose cases on API-36 x86_64
phone/light (1080×2400, 420 dpi). The editor case checks real A/B/C generation and
stored reuse, default All/local/manual edits, reversal and close/new-native-session
reopen, original B anchors, unresolved metadata and live Library membership without
reading history or Full reloading. The rollback/completion case checks exact
failed-edit Retry, effective furigana without guessed manual readings,
frozen five-occurrence/321 ms completion after intervening rule reversal, one
commit, retained-session finished-book reopening and normal initial loading.
Its clock is fixed for deterministic duration; existing clock regressions validate
actual platform sample/drain behavior separately.

Initial fixture compilation corrected compact enum/field types and Kotlin's
warnings-as-errors. Actual native success retires the prompt, so the production
fixture uses the reader's existing reload action rather than a stale editor.
That action now says Reload after an edit. Device completion/reopen exposed a
native reload marker surviving into a new reader; normal invalidation/explicit
load now clears the marker while accepted writes/errors/retries remain intact.
Pure and actual JNI/Compose checks cover the correction. One host fixture needed
RefreshLibrary before checking summaries in its new bridge; another still sent a
legacy rename-test completion and now loads/authenticates the compact identity.

The initial emulator boot retained wide/night settings and System UI became
unresponsive; reset display/night and rebooted. An initial test wait also required
tokens while a failed write deliberately retained a retired page; corrected the
wait to accept pending overrides. An interrupted run and a later APK reinstall
failed with device-offline/INSTALL_FAILED_INSUFFICIENT_STORAGE. Android cache
trimming and removal of this task's newly installed test app/private dictionary
copies allowed the final APK to install. Subsequent runs use direct Android
instrumentation against the built APKs, avoiding repeated large UTP reinstalls.
This does not alter product data or deletion semantics. Gradle/ADB/AVD need the
previously documented local-socket/KVM/AVD-write escalation.

Logs: `/tmp/yomoku-activation-{rust-final,reload-core,clippy-final,native-final,host-final-native,format-check,host-format-check,production-verified,regressions,emulator}.log`.
The pre-fix completion/reopen failure is retained as
`/tmp/yomoku-activation-production-reopen-failure.log`; native regression selection
is `/tmp/yomoku-activation-regression-classes.txt`. The existing long-page HWUI
stall remains unresolved: the four previously documented long-page cases are
excluded, so this session cannot establish that the full device matrix passes.
Separate Full owned/mapped fixtures, release/process-death, physical arm64 and
actual effective JNI/device/diverse/image-heavy performance were not measured.

The 43-case phone regression run initially finished with 39 passes/four failures.
The real typography/position case found a same-anchor position save retiring
native tokens while Android's one-attempt guard still remembered the old read.
Scope that guard to base/position/preset and preserve explicit Retry for failures
within the same context. A new compact case also verifies unchanged-anchor reflow
and that clearing an error cannot replay a failed read. The three remaining
fixture corrections add a scrolling Column around compact components (preventing
overlapping buttons), scroll to Close after the new native Retry controls, and
compare token/boundary/base payloads while asserting a fresh revision after
recreation/reopening. All other 39 regressions passed; final reruns are recorded
below rather than treating that initial run as a passing matrix.

PASS: the final 19-case phone selection, without failures/skips, on the final APK:
all 12 compact-word cases (including the new same-page/preset/error-clear guard),
both actual production editor/completion cases, all four initially failed
regressions and the isolated empty-candidate fixture whose layout also changed.
Together with the initial 39 passing regressions, 57 distinct phone cases pass.
The four initial failures are covered by passing final reruns; no unrun case is
counted. Exact passed-case manifest:
`/tmp/yomoku-activation-passed-phone-cases.txt`. Final Kotlin-only build/lint:
`/tmp/yomoku-activation-ui-final-build.log`; final phone log:
`/tmp/yomoku-activation-final-phone.log`.

```sh
# After the last Kotlin-only corrections, reuse the already verified native ABIs.
./android/gradlew -p android :app:assembleDebug :app:assembleDebugAndroidTest :app:lintDebug -x :app:buildRustBridge --offline --console=plain
final_cases=(
  app.yomoku.EffectiveReaderWordsTest
  app.yomoku.EffectiveReaderProductionTest
  'app.yomoku.ReaderTypographySettingsTest#controlsCommitRetryPreserveExactOffsetAndPersistAcrossRecreationAndDocuments'
  'app.yomoku.ReaderWordsTest#loadingWaitsForStorageAndErrorsRequireExplicitRetryWhilePendingSavesDisableChoices'
  'app.yomoku.ReaderWordsTest#tappingAndFiveChoicesPersistWithoutLookupHistoryAndFailedChoiceCanRepeat'
  'app.yomoku.ReaderWordsTest#customEntryCreateEditDeleteSurvivesRecreationAndIsGlobalWithoutChangingReaderHistory'
  'app.yomoku.ReaderWordsTest#dictionaryFailureNeedsExplicitRetryAndEmptyMatchesAreSuccessful'
)
printf -v final_selection '%s,' "${final_cases[@]}"
"$ANDROID_HOME/platform-tools/adb" -s emulator-5554 shell am instrument -w -r -e class "${final_selection%,}" app.yomoku.test/androidx.test.runner.AndroidJUnitRunner
# Additional wide/dark same-page guard (alongside the production class above):
"$ANDROID_HOME/platform-tools/adb" -s emulator-5554 shell am instrument -w -r -e class 'app.yomoku.EffectiveReaderProductionTest,app.yomoku.EffectiveReaderWordsTest#samePagePositionAndPresetChangesReloadTokensWithoutReplayingFailedReads' app.yomoku.test/androidx.test.runner.AndroidJUnitRunner
```

PASS: all three final wide/dark cases (2560×1600, 240 dpi), no failures/skips:
both production JNI/Full/SQLite editor/completion flows and the compact same-page
position/preset reload guard. Log: `/tmp/yomoku-activation-wide.log`. Restored and
verified physical phone size 1080×2400/density 420, reset system light, and stopped
the task-started emulator. Final aggregate: 57 distinct phone plus three wide
cases; 710 Rust pass/17 opt-in ignored, full final host JNI/codecs, both-ABI APKs,
strict Clippy/Android lint/formatting/diff and unchanged generated Kotlin. The
known four long-page emulator cases and performance/hardware gaps remain as above.

## Long-page interactive text layouts

2026-10-29, pinned offline Android tools, API-36 x86_64 medium_phone:

```sh
export JAVA_HOME=/home/kai/repos/yomoku/.local/jdk-17/jdk-17.0.21+9
export GRADLE_USER_HOME=/home/kai/repos/yomoku/.local/gradle
export ANDROID_HOME=/home/kai/Android/Sdk
tools/format-kotlin.sh --check
# Kotlin-only change: retain the verified native binaries and both packaged ABIs.
./android/gradlew -p android :app:assembleDebug :app:assembleDebugAndroidTest :app:lintDebug -x :app:buildRustBridge --offline --console=plain
"$ANDROID_HOME/platform-tools/adb" -s emulator-5554 install -r -t android/app/build/outputs/apk/debug/app-debug.apk
"$ANDROID_HOME/platform-tools/adb" -s emulator-5554 install -r -t android/app/build/outputs/apk/androidTest/debug/app-debug-androidTest.apk
cases='app.yomoku.ReaderLongPageTest,app.yomoku.ReaderWordSpansTest,app.yomoku.ReaderWordsTest#movingBothDirectionsDismissesDetailsAndNeverRestoresTheSelection,app.yomoku.ReaderWordsTest#tappingAndFiveChoicesPersistWithoutLookupHistoryAndFailedChoiceCanRepeat,app.yomoku.ReaderNavigationTest,app.yomoku.ReaderTypographySettingsTest,app.yomoku.ReaderFormattingTest,app.yomoku.ReaderImagesTest,app.yomoku.ReaderFuriganaTest'
"$ANDROID_HOME/platform-tools/adb" -s emulator-5554 shell am instrument -w -r -e class "$cases" app.yomoku.test/androidx.test.runner.AndroidJUnitRunner
git diff --check
```

Diagnostic baseline: the existing long word case stalls; debugger samples show
main in Compose `TextLinkScope.applyAnnotators`/`replaceStyle` and Android span
geometry, extending the previous HWUI-only observation. Stop the diagnostic
process after sampling; it is not a passing/timed benchmark. Explicit empty
`TextLinkStyles` prevents Material website-style injection; static spans carry
vocabulary colors/decoration. Long left-to-right fragments render in annotated
blocks at measured soft line boundaries. Bidi/direction controls and explicit
paragraph styling retain their complete layout. No native/schema/dependency change.

Initial iterations exposed/corrected an invalid Compose import; null link styles
causing Material defaults; a geometry assertion comparing invisible trailing-space
advance instead of glyph positions; and forced partial paragraph direction adding
an extra visual line. The final tests compare exact concatenated Unicode, every
line's offsets, edge glyph coordinates/baselines, crossing callback identity and
unchanged complete bidi layout. Native navigation fixtures also needed to await
Library's automatic distribution load and effective tokens after movement; the
shared native gates correctly reject commands sent during those reads. Typography
fixtures wait for the same Library load before their direct imports.

The initial 12-case run passed ten, including long word selection, completion
Retry and finish/reopen. Controls/swipes then passed with explicit token readiness;
both typography cases passed with Library readiness. These intermediate results
do not substitute for final APK verification. Logs retain failed iterations.

Gradle's sandbox local-socket failure requires escalation as before. APK installs
again hit the 6-GiB AVD's space limit; cache trimming was insufficient. Stopped the
test process and removed only the two immutable dictionary extractions under
`no_backup/{sudachi,dictionaries}` before reinstalling. The app republishes them
from verified APK assets. Reader SQLite/documents/settings were preserved; no
app data wipe/uninstall/AVD reset. Keep test execution separate from APK installs.

Logs: `/tmp/yomoku-long-page-{baseline,focused,block-focused,final-focused,corrected,render-regressions,final-build,format-check,final-phone,emulator}.log`.
The diagnostic JDB port forward was removed. Rust/JNI suites, separate Full-loader
matrix, release/process-death, physical arm64 and actual effective JNI/device/
diverse/image-heavy performance are not rerun/measured by this Kotlin slice.

PASS: final 23-case phone/light run, without failures/skips, on the final APK.
All four formerly unresolved long-page cases pass, as do the two new Unicode/bidi
geometry regressions, five span/palette guards, actual five-status persistence/
failure Retry, all five reader-navigation cases, both typography Settings cases,
EPUB formatting, four image cases and both furigana cases. Phone viewport verified
as 1080×2400 / 420 dpi / system light. Final log:
`/tmp/yomoku-long-page-final-phone.log` (363.512 s). This is test-suite duration,
not an application performance measurement. Both-ABI debug app/test APKs, strict
Android lint, Kotlin formatting and diff pass; native binaries were unchanged.

PASS: final nine-case wide/dark run, no failures/skips, at 2560×1600 / 240 dpi:
both Unicode/bidi geometry cases, five span/palette cases and both actual native
word/status cases, including long-page movement/dismissal. Log:
`/tmp/yomoku-long-page-final-wide.log` (69.947 s, suite duration only).

```sh
"$ANDROID_HOME/platform-tools/adb" -s emulator-5554 shell wm size 2560x1600
"$ANDROID_HOME/platform-tools/adb" -s emulator-5554 shell wm density 240
"$ANDROID_HOME/platform-tools/adb" -s emulator-5554 shell cmd uimode night yes
wide_cases='app.yomoku.ReaderLongPageTest,app.yomoku.ReaderWordSpansTest,app.yomoku.ReaderWordsTest#movingBothDirectionsDismissesDetailsAndNeverRestoresTheSelection,app.yomoku.ReaderWordsTest#tappingAndFiveChoicesPersistWithoutLookupHistoryAndFailedChoiceCanRepeat'
"$ANDROID_HOME/platform-tools/adb" -s emulator-5554 shell am instrument -w -r -e class "$wide_cases" app.yomoku.test/androidx.test.runner.AndroidJUnitRunner
"$ANDROID_HOME/platform-tools/adb" -s emulator-5554 shell wm size reset
"$ANDROID_HOME/platform-tools/adb" -s emulator-5554 shell wm density reset
"$ANDROID_HOME/platform-tools/adb" -s emulator-5554 shell cmd uimode night no
"$ANDROID_HOME/platform-tools/adb" -s emulator-5554 emu kill
```

Verified restored physical phone size 1080×2400/density 420 and system light,
then stopped this task's emulator. All four formerly blocked long-page cases
now pass on the final phone APK. This closes that focused verification gap;
it does not establish the full device/lifecycle/hardware/performance matrix.

## Android effective-reader measurements and large import buffers

2026-10-29, pinned offline tools and the existing API-36 x86_64 medium_phone AVD
(KVM, two cores, swangle, Vulkan disabled). Native bridge/generated bindings are
unchanged; reuse the verified native output. Production URI imports now retain a
private primitive byte buffer through a read-only typed list, rather than a boxed
object/list slot per byte. The measurement test uses the same conversion.

```sh
export JAVA_HOME=/home/kai/repos/yomoku/.local/jdk-17/jdk-17.0.21+9
export GRADLE_USER_HOME=/home/kai/repos/yomoku/.local/gradle
export ANDROID_HOME=/home/kai/Android/Sdk
./android/gradlew -p android :app:assembleDebug :app:assembleDebugAndroidTest :app:lintDebug -x :app:buildRustBridge --offline --console=plain
tools/format-kotlin.sh --check
git diff --check
"$ANDROID_HOME/platform-tools/adb" -s emulator-5554 install -r -t android/app/build/outputs/apk/debug/app-debug.apk
"$ANDROID_HOME/platform-tools/adb" -s emulator-5554 install -r -t android/app/build/outputs/apk/androidTest/debug/app-debug-androidTest.apk
"$ANDROID_HOME/platform-tools/adb" -s emulator-5554 shell wm size reset
"$ANDROID_HOME/platform-tools/adb" -s emulator-5554 shell wm density reset
"$ANDROID_HOME/platform-tools/adb" -s emulator-5554 shell cmd uimode night no
"$ANDROID_HOME/platform-tools/adb" -s emulator-5554 shell am instrument -w -r -e class app.yomoku.FileImportBytesTest,app.yomoku.MixedFilePickerTest,app.yomoku.TxtPickerTest,app.yomoku.ImportNotificationsTest,app.yomoku.DocumentSessionTest,app.yomoku.AnalyzedImportSessionTest,app.yomoku.EffectiveReaderPerformanceTest app.yomoku.test/androidx.test.runner.AndroidJUnitRunner
# Corpus staging and the opt-in command: EFFECTIVE_READER_PERFORMANCE.md.
```

The new 18 MiB codec regression checks all byte values by SHA-256 at the exact
wire payload offset, serializes the same event twice, checks literal supplementary
Unicode filenames and round-trips TXT/SRT/EPUB with both text encodings on a small
all-values fixture. No large boxed reference oracle or new dependencies.

Initial failures/corrections:
- A stored C snapshot can cross a selected B span and not appear as a choice for
  that span. The benchmark now verifies the alternate's actual chapter/mode row,
  not an invalid assumption about selectable boundaries.
- The real 17,940,673-byte EPUB exhausted the 192 MiB Java heap while eagerly
  mapping file bytes to boxed UByte objects, before native import. Fixed the shared
  production/harness conversion; input/wire/identity/cancellation policy unchanged.
- Picker/notification startup fixtures raced automatic Library distributions;
  wait for counts/error as well as idle, allowing absent counts during active
  batches. Cancellation/recreation and system-picker Cancel pass after this fix.
- Repeated mixed-picker runs correctly deduplicated its constant SRT content.
  Include the run prefix in both input and canonical-text expectation; preserve
  the separate intentional duplicate case and all persistence assertions.
- Gradle's local lock socket needed sandbox escalation. ADB `exec-in ... tee`
  stalled/truncated input copies; use push plus app-owned cp and check corpus hashes.
- Low AVD space blocked updating the main APK. Removed only the two rebuildable,
  immutable Full/index extractions, then installed successfully. Reader database,
  documents and settings were preserved; normal providers republish dictionaries.

PASS: 12 distinct phone/light import regressions (18 MiB codec, mixed picker,
all four TXT picker cases, both notification cases, three document-session cases
and atomic/deduplicated native import), plus the measurement's default opt-in skip.
The corrected MixedFilePickerTest passes in its focused final run; the preceding
13-case selection passed the other eleven regressions and default skip. Only its
constant-SRT assertion failed before the fixture correction; no blanket passing
claim for that earlier selection. Both-ABI APKs (unchanged native binaries), strict
Android lint and Kotlin formatting pass. No Rust/host-JNI/generation/Full-loader
matrix was rerun for this Kotlin-only change.

PASS: final isolated opt-in benchmark, 281.686 s, both verified real books and
synthetic image EPUB, five timed samples after one warmup per read metric. All
48 CSV rows/raw samples validate against their rounded min/median/max. Real EPUB
imports successfully after the fix; fresh revisions, unchanged read hashes, zero
completion/occurrence/pending-time rows, bounded image outputs and all scratch
cleanup guards pass. Results and debug-native-profile limits are in
EFFECTIVE_READER_PERFORMANCE.md. No optimized-build, Compose/frame, peak-memory,
diverse many-book, process-death or physical-arm64 result is claimed. The main
APK SHA-256 and unchanged JNI library hash are recorded with the results.

Final source changes: FileImports.kt; FileImportBytesTest.kt and
EffectiveReaderPerformanceTest.kt; TXT/mixed-picker/notification fixture waits and
repeatable SRT data. Progress/architecture/decisions/performance docs and CSV updated.
No database migration. Public emulator corpus copies and task-started emulator
cleaned up, phone/light restored; host corpora remain in /tmp for reproduction.
Logs: /tmp/yomoku-effective-measure-{android,format-check,smoke,boxed-oom,
regressions-initial,regressions,mixed-final,final}.log. The regressions log contains
the corrected later Mixed fixture's earlier failure; use mixed-final for its pass.

## Phase-5 historical event/provenance audit

Verified 2026-10-29 from the repository root:

```sh
export PATH=/home/kai/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin:$PATH
cargo test -p yomoku-storage history --locked --offline
cargo test --workspace --all-features --locked --offline --quiet
cargo clippy --workspace --all-targets --all-features --locked --offline -- -D warnings
cargo fmt --all -- --check
git diff --check
```

PASS: 43 focused history tests; 713 workspace Rust tests, 17 opt-in ignored;
strict Clippy/Rustfmt/diff. Three new audit tests cover all schema versions 5–19,
late failure/retry from 9/10/13/17/18, exact retained historical fields and source
joins after deletion/reopen, ID high-water marks, and real v18 completed/unused
effective payloads across upgrade/deletion. No failed verification or blocker.
No production code/schema/FFI/Kotlin change; Android, JNI, generated bindings,
Full-loader matrix, release/process-death and physical arm64 were not rerun.
Logs: `/tmp/yomoku-history-schema-{tests,clippy}.log`.

## Read-only lifetime reading aggregation

Verified 2026-10-29 from the repository root:

```sh
export PATH=/home/kai/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin:$PATH
cargo test -p yomoku-storage statistics --locked --offline
cargo test --workspace --all-features --locked --offline --quiet
cargo clippy --workspace --all-targets --all-features --locked --offline -- -D warnings
cargo build --workspace --locked --offline
cargo fmt --all -- --check
git diff --check
```

PASS: seven new aggregation tests; 720 workspace Rust tests, 17 opt-in ignored;
workspace build, strict Clippy/Rustfmt/diff. The schema-5–19 matrix also verifies
current totals after upgrade, deletion and reopen. Counts/time preserve repeats,
original/effective rereads, all statuses, literal Unicode and zero-word pages;
pending/unused/legacy work is excluded. Query-only/unchanged-file checks, failure
rollback/concurrent duplicate delivery, document/vocabulary deletion, unknown
versions/corrupt time and exact signed-integer overflow pass. No failed check.
No dependency/migration/FFI/Kotlin change; Android, JNI, generated bindings,
Full-loader matrix, release/process-death and physical arm64 were not rerun.
Statistics transport/charts remain unimplemented; this is a pure value and native
read-only repository slice. Logs: `/tmp/yomoku-lifetime-statistics-{tests,clippy,build}.log`.

## Read-only five-status vocabulary timeline

Verified 2026-10-29 from the repository root:

```sh
export PATH=/home/kai/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin:$PATH
cargo test -p yomoku-core --test statistics --locked --offline
cargo test -p yomoku-storage statistics --locked --offline
cargo test --workspace --all-features --locked --offline --quiet
cargo clippy --workspace --all-targets --all-features --locked --offline -- -D warnings
cargo build --workspace --locked --offline
cargo fmt --all -- --check
git diff --check
```

PASS: six new pure tests, eight new SQLite timeline tests (15 total storage
statistics tests), extended schema-5–19 upgrade/deletion/reopen projections; 734
workspace Rust tests, 17 opt-in ignored; workspace build, strict Clippy/Rustfmt/diff.
Coverage includes import registration jumps, all five statuses, exact Unicode and
inflections, simultaneous changes, unread manual Known, original/effective page
completion, repetitions/rereads, manual demotion, concurrent duplicate callbacks,
failure/uncommitted isolation, vocabulary-history erasure and fresh IDs without
rescans, regressing wall/frozen completion dates, invalid chains/versions/provenance,
query-only operation and exact unchanged DB bytes. No failed check or blocker.

No dependency/migration/FFI/Kotlin change; Android, JNI, generated bindings,
Full-loader matrix, release/process-death and physical arm64 were not rerun.
Known dictionary-form links/counts, timeframe buckets and statistics transport/UI
remain open. Logs: `/tmp/yomoku-status-statistics-{tests,workspace,clippy,build}.log`.


## Durable vocabulary dictionary-form links

Verified 2026-10-30 from the repository root:

```sh
export PATH=/home/kai/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin:$PATH
cargo test -p yomoku-storage vocabulary_links --locked --offline
cargo test --workspace --all-features --locked --offline --quiet
cargo clippy --workspace --all-targets --all-features --locked --offline -- -D warnings
cargo build --workspace --locked --offline
cargo fmt --all -- --check
git diff --check
```

PASS: seven new storage tests; 741 workspace Rust tests, 17 opt-in ignored;
workspace build, strict Clippy/Rustfmt/diff. Migration 0020 retains exact analyzer
spellings independently of documents. Fixtures cover unread document deletion,
Unicode/inflections, repetitions, unresolved/non-Japanese/manual metadata, distinct
context resolutions, raw registration/read/snapshot retry without rescan, fresh
vocabulary generations, selected edit metadata, original/effective completion,
unused reservations, exact duplicate delivery, statement/deferred-COMMIT rollback
and exact retry. Upgrade recovers surviving unread snapshots plus completed
occurrences from deleted documents; late failure rolls back byte-identically and
preserves totals/timeline. Existing history matrix now exercises schemas 5–20.
Unread metadata deleted before upgrade cannot be recovered; no inferred mappings.

Initial corrections: use enabled rusqlite `prepare` rather than the unavailable
cached-statement API; explicitly reconstruct pre-20 fixture tables before historical
upgrade tests; replace a cloned singleton slice after strict Clippy. Final focused
and workspace runs pass. The interrupted workspace run was rerun to completion.
No unresolved failure or implementation blocker. No dependency/FFI/Kotlin change;
Android/JNI/codegen/Full-loader matrix, release/process-death and physical arm64
were not rerun. Known dictionary-form timeline counts and UI remain unimplemented.
Logs: `/tmp/yomoku-dictionary-links-{tests,workspace,clippy,build}.log`;
initial corrected compile/fixture failures: `dictionary-links-initial.log`.

## Known dictionary-form timeline counts

Verified 2026-10-30 from the repository root:

```sh
export PATH=/home/kai/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin:$PATH
cargo test -p yomoku-core --test statistics --all-features --locked --offline
cargo test -p yomoku-storage --locked --offline --quiet
cargo test --workspace --all-features --locked --offline --quiet
cargo clippy --workspace --all-targets --all-features --locked --offline -- -D warnings
cargo build --workspace --locked --offline
cargo fmt --all -- --check
git diff --check
```

PASS: five new core/six SQLite cases; 11 focused core tests, 332 storage tests
(one ignored), 752 workspace tests (17 opt-in ignored), build, strict Clippy,
Rustfmt and diff checks. Schema-5–20 upgrade/deletion/reopen fixtures now verify
Known-form counts. Coverage includes shared forms and all status demotions,
unresolved exclusion, exact Unicode/reading/context identity, chronological replay
across unrelated regressing dates, simultaneous handoffs, unread document deletion,
vocabulary erasure/fresh generations without rescans, original/effective duplicate
and failing completions, read-only byte preservation, uncommitted link/status
isolation and corrupt empty evidence. Retained undated links apply throughout
their vocabulary generation's replay; new evidence can refine earlier counts.

No failed check or blocker. No dependency/migration/FFI/Kotlin change; Android,
JNI/codegen, Full-loader matrix, release/process-death and physical arm64 were not
rerun. Date buckets, typed statistics transport and charts remain unimplemented.
Logs: `/tmp/yomoku-known-forms-{storage,workspace,clippy,build}.log`.


## Read-only statistics calendar buckets

Verified 2026-10-30 from the repository root with the pinned Rust PATH workaround:

```sh
export PATH=/home/kai/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin:$PATH
cargo test -p yomoku-core --test statistics_buckets --all-features --locked --offline
cargo test -p yomoku-storage statistics --locked --offline
cargo test --workspace --all-features --locked --offline --quiet
cargo clippy --workspace --all-targets --all-features --locked --offline -- -D warnings
cargo build --workspace --locked --offline
cargo fmt --all -- --check
git diff --check
```

PASS: eight new core/seven SQLite cases; 22 focused storage statistics tests;
767 workspace Rust tests, 17 opt-in ignored; build, strict Clippy/Rustfmt/diff.
The focused core run passed its initial seven cases; the final workspace run also
executes the added eighth Monday-boundary case. Coverage includes month-end,
leap/century/year/Monday transitions, exact range/as-of ownership, daily/weekly/
monthly gaps, lifetime origin, absolute status/Known lines, epoch/year-9999 bounds,
future-only history, original/effective rereads/duplicates, zero-word page time,
query-only/unchanged DB bytes/reopen, document/vocabulary deletion and fresh IDs,
failed/uncommitted isolation, invalid versions/dates/durations and same-/cross-date
integer overflow. Read errors release transactions and do not repair data.

Initial corrections: add the new native error to runtime's exhaustive mapping;
use explicitly resolved dictionary metadata for Known-form fixture expectations
(the existing general import fixture is deliberately unresolved). Final focused
and full checks pass. No unresolved failure or implementation blocker.
No dependency/migration/wire/Kotlin change; Android/JNI/generated bindings,
Full-loader matrix, release/process-death and physical arm64 were not rerun.
Transport, retained loading/Retry state and charts remain open.
Logs: `/tmp/yomoku-statistics-buckets-{storage,workspace,clippy,build}.log`.


## Retained statistics worker/core/FFI transport

Verified 2026-10-30 from the repository root:

```sh
export PATH=/home/kai/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin:$PATH
cargo test -p yomoku-core --test statistics_transport --all-features --locked --offline
cargo test -p yomoku-runtime --test imports statistics --all-features --locked --offline
cargo test --workspace --all-features --locked --offline --quiet
cargo clippy --workspace --all-targets --all-features --locked --offline -- -D warnings
export JAVA_HOME="$PWD/.local/jdk-17/jdk-17.0.21+9"
export JAVA_BIN="$JAVA_HOME/bin/java"
tools/test-kotlin-bridge.sh
cargo run -p yomoku-ffi --bin generate-kotlin --features facet_typegen --locked --offline -- target/generated/kotlin-repeat
diff -r target/generated/kotlin target/generated/kotlin-repeat
cargo build --workspace --locked --offline
cargo fmt --all -- --check
"$JAVA_BIN" -jar .local/ktfmt-0.64.jar --kotlinlang-style --dry-run --set-exit-if-changed tests/kotlin/StatisticsBridgeTest.kt tests/kotlin/StatisticsCodecTest.kt
git diff --check
```

PASS: 780 workspace Rust tests, 17 opt-in ignored; nine new core/three worker/one
native-protocol cases. Real-Full production Kotlin/JNI plus all existing bridge
fixtures pass with strict Kotlin compilation, Rust/Kotlin/Rust byte exchanges and
100 handle lifecycles. Generated output repeats identically; workspace build,
strict Clippy/Rustfmt/new-Kotlin-format/diff pass. Generation and JNI execution
remain sequential; Clippy's metadata-only checks can run during JNI execution.

Coverage: default year, all four exact repository projections and absolute lines,
completed time, lifetime opening values, no analyzer/dictionary loads or writes,
reopen/document-deleted history, manual status/reread/import and external mutation
refresh, exact Retry cutoff, error isolation, coalesced selections, stale request
IDs/equal-query callbacks and Settings-hidden follow-ups. Codec fixtures preserve
u64::MAX and values above 2^53 for counts/time/forms and absent/loading/error state.
Initial corruption fixtures were corrected for CHECK constraints/existing typed
error mapping; a Kotlin unnecessary-safe-call warning was fixed. Review added a
regression/fix for resuming an unattempted query after Settings closes. No unresolved
failure/blocker. Expected Crux close-during-I/O diagnostics remain documented above.
No dependency/migration/Compose change. Android APK/lint/device, separate Full-loader
matrix, release/process-death/performance and physical-arm64 checks were not rerun.
Logs: `/tmp/yomoku-statistics-transport-{workspace,jni,clippy,codegen,build}.log`.


## Statistics Compose overview and drilldowns

Verified 2026-10-30 from the repository root:

```sh
export PATH=/home/kai/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin:$PATH
export JAVA_HOME="$PWD/.local/jdk-17/jdk-17.0.21+9"
export GRADLE_USER_HOME="$PWD/.local/gradle"
export ANDROID_HOME=/home/kai/Android/Sdk
./android/gradlew -p android :app:assembleDebug :app:assembleDebugAndroidTest :app:lintDebug --offline --console=plain
"$ANDROID_HOME/platform-tools/adb" -s emulator-5554 install -r -t android/app/build/outputs/apk/debug/app-debug.apk
"$ANDROID_HOME/platform-tools/adb" -s emulator-5554 install -r -t android/app/build/outputs/apk/androidTest/debug/app-debug-androidTest.apk
statistics_cases=app.yomoku.StatisticsDatasetTest,app.yomoku.StatisticsScreenTest,app.yomoku.StatisticsProductionTest,app.yomoku.NavigationTest,app.yomoku.EffectiveReaderNavigationTest
"$ANDROID_HOME/platform-tools/adb" -s emulator-5554 shell am instrument -w -r -e class "$statistics_cases" app.yomoku.test/androidx.test.runner.AndroidJUnitRunner
"$ANDROID_HOME/platform-tools/adb" -s emulator-5554 shell wm size 2560x1600
"$ANDROID_HOME/platform-tools/adb" -s emulator-5554 shell wm density 240
"$ANDROID_HOME/platform-tools/adb" -s emulator-5554 shell cmd uimode night yes
"$ANDROID_HOME/platform-tools/adb" -s emulator-5554 shell am instrument -w -r -e class "$statistics_cases" app.yomoku.test/androidx.test.runner.AndroidJUnitRunner
"$ANDROID_HOME/platform-tools/adb" -s emulator-5554 shell wm size reset
"$ANDROID_HOME/platform-tools/adb" -s emulator-5554 shell wm density reset
"$ANDROID_HOME/platform-tools/adb" -s emulator-5554 shell cmd uimode night no
"$JAVA_HOME/bin/java" -jar .local/ktfmt-0.64.jar --kotlinlang-style --dry-run --set-exit-if-changed android/app/src/main/java/app/yomoku/{StatisticsChart,StatisticsScreen,YomokuApp}.kt android/app/src/androidTest/java/app/yomoku/{StatisticsDataset,StatisticsScreen,StatisticsProduction,EffectiveReaderBoundaries,EffectiveReaderWords,EffectiveReaderNavigation}Test.kt
git diff --check
```

PASS: both-ABI native/Kotlin regeneration, debug/test APKs, strict Android lint,
all changed Kotlin formatting and diff checks. Thirteen cases pass without failures
or skips on phone/light (1080×2400, 420 dpi; 98.594 s) and tablet/dark (2560×1600,
240 dpi; 59.513 s), 26 executions. Seven new cases cover lifetime opening/closing
values, all eight lines, four timeframes, actual timestamp spacing, status demotion,
constant/empty data, UTC dates and exact u64::MAX drawing distances. Compose verifies
exact chart summaries/opening/interval rows, Settings/Back, stable visible/idle
cutoffs and explicit error Retry. Real-Full production verifies import registration
without reading increments, manual status changes, completed occurrences, every
timeframe and native session/drilldown/query retention across recreation/Settings.
Six existing main/reader navigation regressions pass on both layouts. Updated
preview/three reader fixtures for the previous slice's required Statistics field.

The original emulator had only 496 MiB free and could not stage the APK. Cache
trimming did not help; a temporary storage-reserve reduction was restored to its
original absent setting after installation still failed. Existing Library data
was preserved. A /tmp isolated-image attempt failed its host free-space preflight;
use the ignored workspace data directory instead, with ample host space:

```sh
mkdir -p "$PWD/.local/statistics-ui-avd"
"$ANDROID_HOME/emulator/emulator" -avd medium_phone -datadir "$PWD/.local/statistics-ui-avd" -data "$PWD/.local/statistics-ui-avd/userdata-qemu.img" -cache "$PWD/.local/statistics-ui-avd/cache.img" -partition-size 8192 -no-window -no-audio -no-snapshot -gpu swangle -feature -Vulkan -camera-back none -camera-front none -accel on -cores 2
```

Gradle required local-socket escalation; initial sandbox startup failed before
building. The isolated emulator boots and installs successfully. Visual phone and
tablet checks pass; screenshots: /tmp/yomoku-statistics-ui-{phone,tablet}.png.
Display size/density/light mode restored; task-started emulators stopped. No
unresolved source/test failure or blocker. No new dependency, migration or Rust
source change. Rust tests/Clippy, host JNI, codegen reproducibility, separate
Full-loader matrix, release/process-death, performance and physical arm64 were
not rerun. Logs: /tmp/yomoku-statistics-ui-{android,android-final,phone,tablet,
emulator,isolated-emulator}.log. Generation remained sequential to host JNI.


## Read-only indexed Vocabulary browser

Verified 2026-10-30 from the repository root with the pinned Rust PATH workaround:

```sh
export PATH=/home/kai/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin:$PATH
cargo test -p yomoku-storage vocabulary_browser --locked --offline
cargo test --workspace --all-features --locked --offline --quiet
cargo clippy --workspace --all-targets --all-features --locked --offline -- -D warnings
cargo build --workspace --locked --offline
cargo fmt --all -- --check
git diff --check
```

PASS: nine new SQLite cases; 789 workspace Rust tests, 17 opt-in ignored;
workspace build, strict Clippy/Rustfmt/diff checks. Query plans use the existing
BINARY surface range index and per-entry occurrence index for all four sorts.
Coverage includes all 32 status subsets, duplicate/empty filters, literal wildcard/
quote/whitespace/NUL/case prefixes, supplementary/decomposed Unicode and scalar
successor edge bounds, reading/lemma exclusion, 100-row pages with filtered/unloaded
results and stable ties, huge offsets, original/effective repeated/reread history,
regressing dates, manual status/lookup exclusion, read-only unchanged bytes/reopen,
document-deleted history, vocabulary erasure with surviving live documents/fresh
New generations, pending/unused/legacy/failed/uncommitted isolation, corrupt
versions/dates/statuses, missing provenance and query failure cleanup without repairs.

Initial test compilation lacked a VocabularyId import; corrected and rerun.
Focused tests were rerun after strengthening surviving-document and missing-
provenance assertions; the full suite was rerun after the final SQL change.
No unresolved failure/blocker. No dependency, migration, shell wire or Android
source change. Android APK/lint/device, host JNI/codegen reproducibility, separate
Full-loader matrix, release/process-death/performance and physical arm64 were not
rerun. Frequency/date sorts may evaluate every matching history; no large-history
performance claim. Vocabulary retained worker/core/FFI/Compose integration remains
open. Logs: /tmp/yomoku-vocabulary-browser-{storage,workspace,clippy,build}.log;
corrected initial compile log: /tmp/yomoku-vocabulary-browser-initial.log.


## Retained Vocabulary worker/core/FFI transport

Verified 2026-10-30 from the repository root:

```sh
export PATH=/home/kai/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin:$PATH
cargo test -p yomoku-core --test vocabulary_transport --all-features --locked --offline
cargo test -p yomoku-runtime --test imports vocabulary --all-features --locked --offline
cargo test --workspace --all-features --locked --offline --quiet
cargo clippy --workspace --all-targets --all-features --locked --offline -- -D warnings
export JAVA_HOME="$PWD/.local/jdk-17/jdk-17.0.21+9"
export JAVA_BIN="$JAVA_HOME/bin/java"
tools/test-kotlin-bridge.sh
cargo run -p yomoku-ffi --bin generate-kotlin --features facet_typegen --locked --offline -- target/generated/kotlin-repeat
diff -r target/generated/kotlin target/generated/kotlin-repeat
cargo fmt --all -- --check
git diff --check
```

PASS: eight new core/three worker/one native-protocol cases, 801 workspace Rust
tests (17 opt-in ignored), strict Clippy, workspace build through the bridge
script, Kotlin strict compilation and full real-Full production JNI regressions,
including 100 handle lifecycles. Generated output repeats identically; Rust/Kotlin/
Rust fixtures preserve literal Unicode/NUL prefixes, nullable/error/loading state,
u64::MAX counts and values above 2^53 for offsets/dates. Native read requests,
results and captured callbacks remain unavailable to the shell codec.

Tests cover three-page append, empty results/all sorts/status sets, failed first
and append exact Retry, coalescing including A -> B -> A, stale/duplicate/generic
callbacks, unrelated busy work/error isolation, hidden Settings followups, reentry,
import/status/original/effective completion invalidation, external count restart,
read-only bytes, no analyzer/dictionary loads, deleted-document history and reopen.
Initial corrections: use the actual vocabulary table name in corruption fixtures,
remove unused mut, and add the new field to the existing Rust/Android view fixtures.
Final full/focused checks pass. Expected Crux close-during-I/O diagnostics remain
as documented above. No new dependency or migration. Compose browser, bulk actions,
large-history/serialization measurements and phone/tablet recreation remain open.

Android PASS: native regeneration for both ABIs, debug/test APKs, strict lint,
and changed Kotlin formatting. Commands (after host JNI finishes):

```sh
export GRADLE_USER_HOME="$PWD/.local/gradle"
export ANDROID_HOME=/home/kai/Android/Sdk
./android/gradlew -p android :app:assembleDebug :app:assembleDebugAndroidTest :app:lintDebug --offline --console=plain
"$JAVA_BIN" -jar .local/ktfmt-0.64.jar --kotlinlang-style --dry-run --set-exit-if-changed tests/kotlin/VocabularyBridgeTest.kt tests/kotlin/VocabularyCodecTest.kt android/app/src/main/java/app/yomoku/YomokuApp.kt android/app/src/androidTest/java/app/yomoku/{StatisticsScreen,EffectiveReaderNavigation,EffectiveReaderWords,EffectiveReaderBoundaries}Test.kt
```

Gradle's sandbox startup failed before building because local socket access was
unavailable; escalated offline rerun passes. No unresolved failure or blocker.
No device instrumentation, separate Full-loader matrix, release/process-death,
performance or physical arm64 execution rerun. Logs:
`/tmp/yomoku-vocabulary-transport-{core,worker,workspace,clippy,jni,codegen,android}.log`.
Native/BoltFFI generation remained sequential to host JNI execution.


## Vocabulary Compose browser and infinite scroll

Verified 2026-10-30 from the repository root:

```sh
export PATH=/home/kai/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin:$PATH
export JAVA_HOME="$PWD/.local/jdk-17/jdk-17.0.21+9"
export GRADLE_USER_HOME="$PWD/.local/gradle"
export ANDROID_HOME=/home/kai/Android/Sdk
./android/gradlew -p android :app:assembleDebug :app:assembleDebugAndroidTest :app:lintDebug --offline --console=plain
"$ANDROID_HOME/platform-tools/adb" -s emulator-5554 install -r -t android/app/build/outputs/apk/debug/app-debug.apk
"$ANDROID_HOME/platform-tools/adb" -s emulator-5554 install -r -t android/app/build/outputs/apk/androidTest/debug/app-debug-androidTest.apk
vocabulary_cases=app.yomoku.VocabularyScreenTest,app.yomoku.VocabularyProductionTest,app.yomoku.NavigationTest,app.yomoku.EffectiveReaderNavigationTest
"$ANDROID_HOME/platform-tools/adb" -s emulator-5554 shell am instrument -w -r -e class "$vocabulary_cases" app.yomoku.test/androidx.test.runner.AndroidJUnitRunner
"$ANDROID_HOME/platform-tools/adb" -s emulator-5554 shell wm size 2560x1600
"$ANDROID_HOME/platform-tools/adb" -s emulator-5554 shell wm density 240
"$ANDROID_HOME/platform-tools/adb" -s emulator-5554 shell cmd uimode night yes
"$ANDROID_HOME/platform-tools/adb" -s emulator-5554 shell am instrument -w -r -e class "$vocabulary_cases" app.yomoku.test/androidx.test.runner.AndroidJUnitRunner
"$ANDROID_HOME/platform-tools/adb" -s emulator-5554 shell wm size reset
"$ANDROID_HOME/platform-tools/adb" -s emulator-5554 shell wm density reset
"$ANDROID_HOME/platform-tools/adb" -s emulator-5554 shell cmd uimode night no
"$JAVA_HOME/bin/java" -jar .local/ktfmt-0.64.jar --kotlinlang-style --dry-run --set-exit-if-changed android/app/src/main/java/app/yomoku/{VocabularyScreen,YomokuApp,ReaderWords}.kt android/app/src/androidTest/java/app/yomoku/{VocabularyScreen,VocabularyProduction}Test.kt
git diff --check
```

PASS: debug/test APKs, strict Android lint, all changed Kotlin formatting and diff.
Native generation was up-to-date; existing both-ABI libraries packaged. Eleven
cases pass on API-36 x86_64 phone/light (1080x2400, 420 dpi; 114.611 s) and
landscape tablet/dark (2560x1600, 240 dpi; 82.566 s), 22 executions without
failures or skips. Five new cases cover literal prefix/clear, five status toggles
including none, all sorts, exact ULong.MAX_VALUE labels, idle/input gating,
initial/append explicit Retry, retained rows and scroll/query reset. Native SQLite
fixture verifies 205 rows across three pages, an unloaded prefix match, table
read failure/repair without automatic retry, Settings and native session/query/
pages/scroll recreation. Real Full import, effective-reader manual changes and
completion refresh browser statuses, completed counts and last-seen dates.
Six existing main/reader navigation cases pass on both layouts.

Initial phone run found two test assumptions: Retry can be offscreen on return
from Settings, and reopening a reader requires its new effective selection
identity. Corrected the assertions/selection and reran the complete focused suite
on both layouts. Initial sandbox Gradle and redirected ADB commands failed socket
startup; escalated reruns pass. Early APK installs were retried after boot.
Used the ignored isolated .local/statistics-ui-avd image with the 8 GiB partition
and startup command recorded in the Statistics Compose section. Visual checks
pass; screenshots /tmp/yomoku-vocabulary-ui-{phone,tablet}.png. Display/density/
light settings restored and task-started emulator stopped. No unresolved failure
or blocker. No Rust/native source, dependency, migration or protocol change;
Rust tests/Clippy, host JNI/codegen, separate Full-loader matrix, release/process-
death/performance and physical arm64 were not rerun. Logs:
/tmp/yomoku-vocabulary-ui-{android,phone,phone-initial,tablet,emulator}.log.

## Frozen Vocabulary bulk repositories

Verified 2026-10-30 from the repository root:

```sh
export PATH=/home/kai/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin:$PATH
cargo test -p yomoku-storage vocabulary_bulk --locked --offline
cargo test --workspace --all-features --locked --offline --quiet
cargo clippy --workspace --all-targets --all-features --locked --offline -- -D warnings
cargo build --workspace --locked --offline
cargo fmt --all -- --check
git diff --check
```

PASS: seven new SQLite cases; 808 workspace tests pass, 17 opt-in ignored;
strict Clippy, workspace build, Rustfmt and diff checks. Coverage includes 1,205
matching/unloaded rows, all 32 filter subsets and four sorts, literal Unicode/NUL/
wildcard/case/whitespace prefixes, read-only unchanged bytes/reopen, all five
statuses and immutable actual-transition events, duplicate/absent/empty selections,
negative dates, concurrent retries, frozen IDs across later imports/status changes/
replacement generations, later-row/event/cascade/deferred-COMMIT rollback and exact
retry after reopen. Original/effective history cases verify occurrence/status/
Known-form chart recalculation in all four timeframes, retained completed time/
provenance/live analysis, absent words after reader/token/Library reload, fresh New
generations on later import and no resurrection on lost-success completion replay.

Initial test compile assumptions corrected: mutable document snapshot reads,
WordStatusCounts has no total method, and Recorded carries newly_known. Initial
Clippy found an unnecessary clone for a single-element slice; fixed with from_ref
and reran focused tests/Clippy. No unresolved failure/blocker. No migration,
dependency, core/shell protocol or Android source changes. Android/JNI/codegen,
separate Full-loader matrix, release/process-death/performance and physical arm64
were not rerun. Retained confirmation/FIFO/FFI/Compose integration remains open.
Logs: /tmp/yomoku-vocabulary-bulk-{storage,workspace,clippy,build}.log.


## Retained native Vocabulary bulk transport

Verified 2026-10-30 from the repository root with the pinned environment:

```sh
export PATH=/home/kai/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin:$PATH
cargo test -p yomoku-core --test vocabulary_transport --all-features --locked --offline
cargo test -p yomoku-runtime --test imports vocabulary_bulk --all-features --locked --offline
cargo test --workspace --all-features --locked --offline --quiet
cargo clippy --workspace --all-targets --all-features --locked --offline -- -D warnings
export JAVA_HOME="$PWD/.local/jdk-17/jdk-17.0.21+9"
export JAVA_BIN="$JAVA_HOME/bin/java"
tools/test-kotlin-bridge.sh
cargo run -p yomoku-ffi --bin generate-kotlin --features facet_typegen --locked --offline -- target/generated/kotlin-repeat
diff -r target/generated/kotlin target/generated/kotlin-repeat
cargo fmt --all -- --check
"$JAVA_BIN" -jar .local/ktfmt-0.64.jar --kotlinlang-style --dry-run --set-exit-if-changed tests/kotlin/VocabularyBridgeTest.kt tests/kotlin/VocabularyCodecTest.kt android/app/src/main/java/app/yomoku/YomokuApp.kt android/app/src/androidTest/java/app/yomoku/{VocabularyScreen,StatisticsScreen,EffectiveReaderBoundaries,EffectiveReaderNavigation,EffectiveReaderWords}Test.kt
git diff --check
```

PASS: four new core/three worker/one native-protocol cases; 816 workspace Rust
tests (17 opt-in ignored), strict Clippy/Rustfmt/diff and workspace build through
the host bridge script. Both core configurations cover loaded-only toggles,
1,205 unloaded matches, hidden selections/query changes/Settings, exact prompts,
all five status choices/nonnegative frozen date, explicit capture/write Retry,
Cancel, pending/unrelated-work gates, malformed/late/foreign/generic callbacks
and isolated errors. Native callbacks/operations/results have no shell codec.

SQLite FIFO cases cover read-only capture/confirmation/Cancel, later-row update
and delete rollback, exact Retry excluding later matches and replacement IDs,
immutable manual events at the frozen date, reopen and analyzer-free execution.
The dependent-view case refreshes browser, all-time status/occurrence/time charts,
Library distributions and original reader tokens without re-registering erased
words; retained page time survives deletion.

PASS: complete real-Full host Kotlin/JNI and codec regression suite. Production
effective-reader bulk status/deletion refreshes actual browser/statistics/Library/
tokens without analyzer reload. Generated public events preserve all actions,
all statuses, optional/empty selection, capture loading/errors, and exact IDs and
dates above 2^53 through Rust-Kotlin-Rust exchanges. Generation repeats identically;
all changed Kotlin formatting passes. Existing nonfatal native close-during-I/O
teardown diagnostics remain; the complete host command exits zero. Native/BoltFFI
generation and Android builds remain sequential to host JNI execution.

PASS: regenerated native bridge for x86_64/arm64-v8a, debug and instrumentation
APKs, and strict Android lint. Existing preview/instrumentation fixtures initialize
the new capture-loading flag; Compose bulk controls remain unconnected. Command:

```sh
PATH=/home/kai/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin:$PATH JAVA_HOME=/home/kai/repos/yomoku/.local/jdk-17/jdk-17.0.21+9 GRADLE_USER_HOME=/home/kai/repos/yomoku/.local/gradle ANDROID_HOME=/home/kai/Android/Sdk ./android/gradlew -p android :app:assembleDebug :app:assembleDebugAndroidTest :app:lintDebug --offline --console=plain
```

Initial worker-refresh test compilation used incorrect field names; corrected
before focused/full passes. Initial sandbox Gradle startup could not determine a
wildcard IP; reran with approved local-socket access. No schema/dependency change.
Compose bulk controls/Back and phone/tablet recreation acceptance remain open.
Device execution, separate Full-loader matrix, release/process-death/performance
and physical arm64 were not rerun. Logs:
/tmp/yomoku-vocabulary-bulk-transport-{workspace,clippy,host,android}.log.

## Compose Vocabulary selection and confirmed bulk actions

Verification commands (2026-10-30, repository root):

```sh
export PATH=/home/kai/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin:$PATH
export JAVA_HOME="$PWD/.local/jdk-17/jdk-17.0.21+9"
export GRADLE_USER_HOME="$PWD/.local/gradle"
export ANDROID_HOME=/home/kai/Android/Sdk
"$ANDROID_HOME/platform-tools/adb" -s emulator-5554 shell wm size reset
"$ANDROID_HOME/platform-tools/adb" -s emulator-5554 shell wm density reset
"$ANDROID_HOME/platform-tools/adb" -s emulator-5554 shell cmd uimode night no
./android/gradlew -p android :app:assembleDebug :app:assembleDebugAndroidTest :app:lintDebug --offline --console=plain
"$ANDROID_HOME/platform-tools/adb" -s emulator-5554 install -r -t android/app/build/outputs/apk/debug/app-debug.apk
"$ANDROID_HOME/platform-tools/adb" -s emulator-5554 install -r -t android/app/build/outputs/apk/androidTest/debug/app-debug-androidTest.apk
vocabulary_cases=app.yomoku.VocabularyScreenTest,app.yomoku.VocabularyProductionTest,app.yomoku.NavigationTest,app.yomoku.EffectiveReaderNavigationTest
"$ANDROID_HOME/platform-tools/adb" -s emulator-5554 shell am instrument -w -r -e class "$vocabulary_cases" app.yomoku.test/androidx.test.runner.AndroidJUnitRunner
"$ANDROID_HOME/platform-tools/adb" -s emulator-5554 shell wm size 2560x1600
"$ANDROID_HOME/platform-tools/adb" -s emulator-5554 shell wm density 240
"$ANDROID_HOME/platform-tools/adb" -s emulator-5554 shell cmd uimode night yes
"$ANDROID_HOME/platform-tools/adb" -s emulator-5554 shell am instrument -w -r -e class "$vocabulary_cases" app.yomoku.test/androidx.test.runner.AndroidJUnitRunner
"$ANDROID_HOME/platform-tools/adb" -s emulator-5554 shell wm size reset
"$ANDROID_HOME/platform-tools/adb" -s emulator-5554 shell wm density reset
"$ANDROID_HOME/platform-tools/adb" -s emulator-5554 shell cmd uimode night no
"$JAVA_HOME/bin/java" -jar .local/ktfmt-0.64.jar --kotlinlang-style --dry-run --set-exit-if-changed android/app/src/main/java/app/yomoku/{VocabularyScreen,VocabularySelection,YomokuApp}.kt android/app/src/androidTest/java/app/yomoku/{VocabularyScreen,VocabularyProduction}Test.kt
git diff --check
```

PASS: debug/test APKs with the existing x86_64/arm64-v8a native libraries, strict
Android lint, changed Kotlin formatting and diff checks. Sixteen cases pass on
explicit API-36 x86_64 phone/light (1080x2400, 420 dpi; 238.314 s) and landscape
tablet/dark (2560x1600, 240 dpi; 152.128 s), 32 final executions with no failures
or skips. Five new cases verify tap/long-press selection, all five status choices,
exact retained confirmations/large dates/Retry events, pending gates, 205 unloaded
matches plus hidden selection, Settings/Cancel/Back/recreation, capture/update/
delete failures, later-row rollback, frozen manual dates and exclusion of later
matches from write Retry. Real Full import/completion then bulk status/deletion
refresh browser/statistics/Library/effective tokens, preserve completed time,
erase occurrences, avoid old-document rescans and register a fresh New ID on
later import. Eleven existing browser/navigation cases pass on both layouts.

Initial Back assertion ran before selection recomposition; synchronize on the
visible selected count, then the full corrected suite passes. An earlier corrected
16-case pass used inherited 240-dpi/dark overrides at the physical phone resolution;
the explicit phone/light and tablet/dark runs above establish the final matrix.
Always reset display, density and night mode before the first run. Tablet ADB's
host wrapper returned 143 after complete OK output; device TestRunner confirms
16 tests/0 failed/0 ignored and AndroidRuntime exits 0. No unresolved source/test
failure or blocker. Initial sandbox Gradle/ADB socket startup required approved
access. Phone selection visual check passes; screenshot:
/tmp/yomoku-vocabulary-bulk-ui-phone.png. Used the existing ignored isolated
.local/statistics-ui-avd image (8 GiB partition); display/density/light restored
and task-started emulator stopped. No native/schema/dependency change. Rust tests/
Clippy, host JNI/codegen reproducibility, separate Full-loader matrix, release/
process-death/performance and physical arm64 were not rerun. Logs:
/tmp/yomoku-vocabulary-bulk-ui-{android,phone-initial,phone,phone-light,tablet,emulator}.log.

## Vocabulary host history profiling and bounded Newest reads

2026-10-30: the opt-in host example creates/migrates a disposable database,
seeds it exclusively through production APIs, then measures repository calls,
effective-factory FIFO callbacks, full ViewModel projection/Bincode encoding and
Rust decoding. No input database/dictionary files, new dependency or migration.
See VOCABULARY_PERFORMANCE.md for fixture definitions, raw samples, executable
hashes, measured before/after results and host-only limitations.

```sh
export PATH=/home/kai/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin:$PATH
cargo test --workspace --all-features --locked --offline --quiet
cargo test -p yomoku-ffi --example measure_vocabulary --all-features --locked --offline
cargo build --workspace --all-features --locked --offline
cargo clippy --workspace --all-targets --all-features --locked --offline -- -D warnings
cargo fmt --all -- --check
git diff --check
cargo build -p yomoku-ffi --example measure_vocabulary --release --locked --offline
# Run timings only after all builds/tests have finished.
target/release/examples/measure_vocabulary 1000 10 1000 7
target/release/examples/measure_vocabulary 10000 10 1000 7
target/release/examples/measure_vocabulary 100000 10 1000 7
target/release/examples/measure_vocabulary 100 0 0 2
```

PASS: 817 workspace tests (17 opt-in ignored), two example cases, build, strict
Clippy/formatting/diff; 132 before/after metric rows validate seven raw samples
and rounded extrema/median, plus zero-round/reread even-sample smoke. Newest now
projects histories only after selecting its ordered page; the VM-step regression
adds 200 offscreen original/effective occurrences without adding query work, then
verifies exact counts/dates and corrupt-history rejection on the final page.
Existing ten browser cases retain Unicode/filter/tie/paging/index/read-only and
rollback/error coverage. Main/example guards verify exact unchanged database bytes
and scratch cleanup. The initial example used a nonexistent Year enum spelling;
corrected to LastYear. No remaining blocker.

100k entries: Newest first-page median 108.177→30.456 ms, native FIFO callback
111.877→30.455 ms. Year statistics remains ~1.04 s with 1.08m status events;
selected IDs and frozen confirmation yield a ~1.6 MB view. No Android/JNI/Kotlin,
codegen/Full-loader matrix, Compose/frame/peak-memory, diverse-book, process-death
or physical-arm64 check/claim is made for this host slice. Timed runs use isolated
sequential release executables and RAM-backed /tmp with uncontrolled OS caches.
Logs: /tmp/yomoku-vocabulary-performance-{tests,example,build,clippy}.log;
/tmp/yomoku-vocabulary-measure-{after-,}*.{csv,log}. No emulator was started.

## Statistics SQL stepping, pure replay and streaming event reads

2026-10-30: extended `measure_vocabulary` with four diagnostic layers: the frozen
former production UNION's SQL stepping, shared split-query SQL stepping, pure
builder replay over predecoded fixture events and the full validated repository
timeline. Exact replay equality and existing byte/integrity/FK/cleanup guards
cover diagnostics. Only the opt-in harness retains the decoded events; production
still streams them. Measurements/limits/hashes: VOCABULARY_PERFORMANCE.md.

```sh
export PATH=/home/kai/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin:$PATH
cargo test -p yomoku-storage statistics --locked --offline
cargo test --workspace --all-features --locked --offline --quiet
cargo test -p yomoku-ffi --example measure_vocabulary --all-features --locked --offline
cargo build --workspace --all-features --locked --offline
cargo clippy --workspace --all-targets --all-features --locked --offline -- -D warnings
cargo fmt --all -- --check
git diff --check
cargo build -p yomoku-ffi --example measure_vocabulary --release --locked --offline
# No overlapping builds/tests during timings; preserve separate executables for comparison.
target/release/examples/measure_vocabulary 10000 10 1000 7
target/release/examples/measure_vocabulary 100000 10 1000 7
target/release/examples/measure_vocabulary 100 0 0 2
```

PASS: 24 focused statistics cases, 819 workspace tests (17 opt-in ignored), two
example cases, workspace build, strict all-target/all-feature Clippy, Rustfmt/diff.
Two new storage cases require zero statement sorts, old-generation link/event
consistency despite a committed intervening WAL deletion/reimport, read-only use,
and transaction cleanup after either query fails. Existing corrupt-history cases
also require cleanup; version/cause/provenance/chain/date checks are unchanged.
The initial focused compile needed crate-scoped helper/SQL visibility for the
regressions; corrected before successful verification. No remaining blocker.

Four sequential release runs (before/after, 10k/100k entries) pass exact byte
guards/cleanup, with seven samples after warmup. All 104 CSV rows were checked
against raw sample counts/extrema/medians; zero-round/reread even-sample smoke
passes. 100k year statistics median: 944.157→542.853 ms; 10k:
86.947→53.254 ms. SQL steps at 100k fall 65,882,372→39,960,891 and sorts 2→0.
New SQL files are shared by production and diagnostics; schema remains 0020.
No dependency, core/wire or Android code change. Android/JNI/Kotlin/codegen,
Full-loader matrix, device/process-death/physical-arm64 and Compose/frame/
peak-memory/diverse-book measurements were not rerun or claimed. No emulator
started. Logs: `/tmp/yomoku-status-stream-{tests,workspace,example,build,clippy,release}.log`;
`/tmp/yomoku-status-profile-{before,after}-{10000,100000}.{csv,log}`;
`/tmp/yomoku-status-profile-zero.{csv,log}`. Raw comparison:
`docs/measurements/statistics-host-2026-10-30.csv`.

## Vocabulary Frequency/LastSeen candidate profiling

2026-10-30: the existing opt-in example additionally compiles the production page
SQL builder and reports metric-sort stepping, VM steps and sort counts. Each raw
row is checked against the validated repository page before timing. Fixtures,
limits, samples and executable hashes: VOCABULARY_PERFORMANCE.md, final section.

```sh
export PATH=/home/kai/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin:$PATH
cargo test -p yomoku-storage vocabulary_browser --locked --offline
cargo test --workspace --all-features --locked --offline --quiet
cargo test -p yomoku-ffi --example measure_vocabulary --all-features --locked --offline
cargo build --workspace --all-features --locked --offline
cargo clippy --workspace --all-targets --all-features --locked --offline -- -D warnings
cargo fmt --all -- --check
git diff --check
cargo build -p yomoku-ffi --example measure_vocabulary --release --locked --offline
# No concurrent builds/tests during timings; retain separate executables for comparisons.
target/release/examples/measure_vocabulary 10000 10 1000 7
target/release/examples/measure_vocabulary 100000 10 1000 7
target/release/examples/measure_vocabulary 100 0 0 2
```

PASS: 12 browser cases, 821 workspace tests (17 opt-in ignored), two example cases,
build, strict Clippy, Rustfmt/diff. The VM-growth regression fails against former
production SQL (7,500 additional Frequency steps versus 600 for its required
aggregate), then passes both changed sorts. Multi-page expected values cover
mixed original/effective histories, ties, regressing dates, filters, unread rows
and deleted documents. All-sort invalid-data/SQL failures release snapshots and
preserve bytes. Initial profiler compile used unsupported u64 SQL bindings;
corrected to checked i64 conversions. No open blocker.

Four sequential release runs pass replay/byte/FK/integrity/cleanup guards; all 120
seven-sample metric rows validate min/median/max. Zero-history/two-sample smoke
passes all 21 metrics. At 100k entries first-page medians improve 108.717→64.419 ms
(Frequency) and 109.834→65.600 ms (LastSeen); deep offsets still cost ~103–107 ms.
Raw results: measurements/vocabulary-metric-sorts-host-2026-10-30.csv. Logs:
/tmp/yomoku-browser-page-{tests,workspace,example,build,clippy,release}.log;
expected baseline failure: /tmp/yomoku-browser-page-regression-baseline.log;
measurements: /tmp/yomoku-browser-profile-{before,after}-{10000,100000}.{csv,log};
zero smoke: /tmp/yomoku-browser-profile-zero.{csv,log}. No Android/JNI/Kotlin/codegen,
separate Full-loader matrix, lifecycle/process-death, physical-arm64 or device/
Compose/peak-memory check or claim. No emulator started; schema remains 0020.

## Statistics row decoding and borrowed enum text

Verification commands (2026-10-30, repository root):

```sh
export PATH=/home/kai/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin:$PATH
cargo test -p yomoku-storage statistics --locked --offline
cargo test --workspace --all-features --locked --offline --quiet
cargo test -p yomoku-ffi --example measure_vocabulary --all-features --locked --offline
cargo build --workspace --all-features --locked --offline
cargo clippy --workspace --all-targets --all-features --locked --offline -- -D warnings
cargo fmt --all -- --check
git diff --check
cargo build -p yomoku-ffi --example measure_vocabulary --release --locked --offline
# Run preserved before/after executables sequentially, with no builds/tests overlapping.
target/release/examples/measure_vocabulary 10000 10 1000 7
target/release/examples/measure_vocabulary 100000 10 1000 7
target/release/examples/measure_vocabulary 100 0 0 2
```

PASS: 26 focused statistics cases, 823 workspace tests (17 opt-in ignored), three
harness cases (including the shared decoder's type/NULL test), all-feature build,
strict all-target/all-feature Clippy, Rustfmt/diff. Two new regressions require
strict borrowed TEXT/NULL decoding plus invalid UTF-8/literal spelling rejection
through public timeline/calendar reads, unchanged bytes, snapshot cleanup and
exact recovery after fixture repair. Existing provenance, causal replay, Known-
form, read-only and concurrent generation/snapshot guards pass. The initial
corruption fixture attempted BLOB writes into a STRICT table; SQLite correctly
rejected them. Raw SELECT rows now cover type/NULL gates without altering schema.
No unresolved failure/blocker. Shared decoder diagnostics retain the exact
production parser/provenance validation; no SQL/schema/dependency/wire change.

Four sequential release runs (before/after at 10k/100k) and the 100-entry,
zero-round/zero-reread/two-sample smoke pass. All 124 comparison rows and 22 smoke
rows validate raw sample counts/extrema/medians; byte/replay/FK/integrity/cleanup
guards pass. At 100k, median SQL plus decode/validation is 596.927→532.782 ms;
complete status timeline 584.478→545.795 ms. Raw samples/hashes/limits:
VOCABULARY_PERFORMANCE.md, final section, and
measurements/statistics-decoding-host-2026-10-30.csv. Logs:
`/tmp/yomoku-status-decode-{tests,workspace,example,build,clippy,release}.log`;
baseline extraction verification uses `-baseline-{tests,example,release}.log`.
Measurements: `/tmp/yomoku-status-decode-{before,after}-{10000,100000}.{csv,log}`;
zero smoke: `/tmp/yomoku-status-decode-zero.{csv,log}`. No Android/JNI/Kotlin/
codegen, Full-loader matrix, device/lifecycle/process-death/physical-arm64 or
Compose/peak-memory/diverse-book measurements were rerun or claimed. No emulator
started. Required decoding/replay, deep offsets and device measurements remain.

## Vocabulary Compose and sampled-memory measurements

Use the existing isolated API-36 x86_64 medium_phone AVD and pinned CLI tools.
The opt-in test uses its own disposable database and native session; it does not
open the app's normal database. Full method, parameters and results:
[VOCABULARY_ANDROID_PERFORMANCE.md](VOCABULARY_ANDROID_PERFORMANCE.md).

```sh
export PATH=/home/kai/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin:$PATH
export JAVA_HOME="$PWD/.local/jdk-17/jdk-17.0.21+9"
export GRADLE_USER_HOME="$PWD/.local/gradle"
export ANDROID_HOME=/home/kai/Android/Sdk
./android/gradlew -p android :app:assembleDebug :app:assembleDebugAndroidTest :app:lintDebug --offline --console=plain
tools/format-kotlin.sh --check
git diff --check
"$ANDROID_HOME/platform-tools/adb" -s emulator-5554 install -r -t android/app/build/outputs/apk/debug/app-debug.apk
"$ANDROID_HOME/platform-tools/adb" -s emulator-5554 install -r -t android/app/build/outputs/apk/androidTest/debug/app-debug-androidTest.apk
"$ANDROID_HOME/platform-tools/adb" -s emulator-5554 shell wm size reset
"$ANDROID_HOME/platform-tools/adb" -s emulator-5554 shell wm density reset
"$ANDROID_HOME/platform-tools/adb" -s emulator-5554 shell cmd uimode night no
vocabulary_cases=app.yomoku.VocabularyScreenTest,app.yomoku.VocabularyProductionTest,app.yomoku.NavigationTest,app.yomoku.EffectiveReaderNavigationTest,app.yomoku.VocabularyPerformanceTest
"$ANDROID_HOME/platform-tools/adb" -s emulator-5554 shell am instrument -w -r -e class "$vocabulary_cases" app.yomoku.test/androidx.test.runner.AndroidJUnitRunner
# Wait until builds and ordinary regressions finish before measuring.
"$ANDROID_HOME/platform-tools/adb" -s emulator-5554 shell am instrument -w -r -e class app.yomoku.VocabularyPerformanceTest -e yomokuMeasure true -e yomokuVocabularyEntries 10000 -e yomokuVocabularyLoaded 1000 -e yomokuMeasureIterations 5 app.yomoku.test/androidx.test.runner.AndroidJUnitRunner
"$ANDROID_HOME/platform-tools/adb" -s emulator-5554 shell am instrument -w -r -e class app.yomoku.VocabularyPerformanceTest -e yomokuMeasure true -e yomokuVocabularyEntries 100000 -e yomokuVocabularyLoaded 1000 -e yomokuMeasureIterations 5 app.yomoku.test/androidx.test.runner.AndroidJUnitRunner
```

2026-10-30 PASS: both-ABI native rebuild/generated bindings, debug/test APKs,
strict Android lint, repository Kotlin formatting and diff. Sixteen existing
Vocabulary/navigation cases pass plus the harness's default assumption skip
(17 reported; 205.770 s). Separate opt-in 10k/100k processes pass in
32.330/49.535 s with one scenario warmup and five measured scenarios each.
All 50 action samples and 110 frame durations validate: exact counts/medians/max,
reference-frame counts, positive memory observations, payload sizes and zero
reported drops. Unchanged database bytes, no analyzer, exact IDs/frozen prompts,
no reading effects, integrity/FK/native closure/scratch cleanup pass. Post-run
ADB confirms scratch directories absent. Results/hashes/limits/raw CSV are in
VOCABULARY_ANDROID_PERFORMANCE.md. At 100k, Select All median idle time is
865.486 ms, confirmation view ~1.65 MB; callback/composition profiling remains.

Initial sandbox Gradle lock-socket failure required an approved rerun; no source
or instrumentation failure. Used the existing isolated .local/statistics-ui-avd
(8 GiB partition), reset to phone/light, and stopped the task-started emulator.
Logs: /tmp/yomoku-vocabulary-compose-{android,format,regressions,10000,100000}.log.
No production code/schema/dependency/wire changes. Rust host tests/Clippy, separate
host JNI/codegen reproducibility, Full-loader matrix, optimized build, diverse
books/heavy-history device data, continuous-fling/tablet/dark performance,
process-death and physical arm64 were not rerun or claimed.

## Vocabulary selection membership

Use the pinned tool environment and APK build/install commands in the preceding
Vocabulary Compose section. The focused regression set now also includes
`app.yomoku.VocabularySelectionLookupTest` (exact signed identity membership and
bounded ID reads at 100k). Run it with the existing production/Compose/navigation
cases, followed by a separate opt-in measurement process without concurrent
builds/tests/profilers:

```sh
vocabulary_cases=app.yomoku.VocabularyScreenTest,app.yomoku.VocabularyProductionTest,app.yomoku.VocabularySelectionLookupTest,app.yomoku.NavigationTest,app.yomoku.EffectiveReaderNavigationTest,app.yomoku.VocabularyPerformanceTest
"$ANDROID_HOME/platform-tools/adb" -s emulator-5554 shell am instrument -w -r -e class "$vocabulary_cases" app.yomoku.test/androidx.test.runner.AndroidJUnitRunner
"$ANDROID_HOME/platform-tools/adb" -s emulator-5554 shell am instrument -w -r -e class app.yomoku.VocabularyPerformanceTest -e yomokuMeasure true -e yomokuVocabularyEntries 100000 -e yomokuVocabularyLoaded 1000 -e yomokuMeasureIterations 5 app.yomoku.test/androidx.test.runner.AndroidJUnitRunner
```

2026-10-30 PASS: debug/test APKs, strict lint, repository Kotlin formatting/diff;
19 regressions plus one assumption skip (20 reported, 228.889 s). Native/binding/
asset tasks reused existing outputs. Sequential baseline/changed 100k processes
pass (51.327/50.541 s); 50 action rows/116 raw frames validate alongside exact
IDs, unchanged bytes, no analyzer/reading effects, integrity/FK/closure/cleanup.
Select All median idle is 881.920→765.376 ms with overlapping ranges; no consistent
frame/confirmation improvement. Raw data, hashes, sampled-memory limits and
remaining checks: VOCABULARY_ANDROID_PERFORMANCE.md, final section. Logs:
`/tmp/yomoku-selection-{android,format-check,regressions,before-100000,after-100000}.log`.
Initial sandbox Gradle local lock-socket failure required approved rerun; no open
failure. Existing isolated AVD used; no Rust/schema/dependency/wire change or
host/optimized/Full-loader/arm64/lifecycle/peak-memory claim.

## Vocabulary snapshot phase profiling

Use the pinned Rust/JDK/SDK/Gradle environment and build/install/phone-light
commands from the Vocabulary Compose section. Include `app.yomoku.CoreSessionTest`
in the selection-membership regression class list. After all builds and ordinary
regressions finish, run separate opt-in processes with the additional observer:

```sh
"$ANDROID_HOME/platform-tools/adb" -s emulator-5554 shell am instrument -w -r -e class app.yomoku.VocabularyPerformanceTest -e yomokuMeasure true -e yomokuMeasureRender true -e yomokuVocabularyEntries 10000 -e yomokuVocabularyLoaded 1000 -e yomokuMeasureIterations 5 app.yomoku.test/androidx.test.runner.AndroidJUnitRunner
"$ANDROID_HOME/platform-tools/adb" -s emulator-5554 shell am instrument -w -r -e class app.yomoku.VocabularyPerformanceTest -e yomokuMeasure true -e yomokuMeasureRender true -e yomokuVocabularyEntries 100000 -e yomokuVocabularyLoaded 1000 -e yomokuMeasureIterations 5 app.yomoku.test/androidx.test.runner.AndroidJUnitRunner
```

`RENDER_COLUMNS` describes each per-render phase/count row. See
VOCABULARY_ANDROID_PERFORMANCE.md, snapshot phase section, for phase boundaries,
raw samples and limits. `publish_ms` includes equality plus state notification;
do not label it as isolated equality or subsequent composition time. The normal
app and default harness leave this diagnostic observer disabled.

2026-10-30 PASS: debug/test APKs, strict Android lint, Kotlin formatting/diff,
20 regressions plus default assumption skip (21 reported, 219.050 s). Separate
10k/100k profiles pass (36.355/53.160 s); 70 render rows/50 actions/107 raw frames
validate with exact selected/frozen IDs, unchanged bytes, no analyzer/reading
effects, integrity/FK/closure/cleanup. ADB confirms scratch directories absent;
the task-started isolated AVD was stopped with phone/light settings retained.
Native/binding/asset tasks reused outputs. Initial sandbox Gradle lock sockets
needed approved rerun; initial nullable-selection compile error was corrected.
No unresolved failure. Raw CSV/results/hashes/limits:
VOCABULARY_ANDROID_PERFORMANCE.md, snapshot phase section. Logs:
`/tmp/yomoku-render-{android,format-check,regressions,10000,100000,validation}.log`.
No Rust/schema/dependency/wire change or host Rust/Clippy, independent JNI/codegen/
Full-loader, optimized/tablet/dark/fully-loaded/heavy-history/diverse-book,
equal-snapshot, exact peak/PSS, process-death or physical-arm64 claim.

## Vocabulary request phases and equal confirmation replay

Use the same pinned environment, build/install commands and phone/light isolated
AVD as above. Add `app.yomoku.CoreSessionTest` to the selection-membership class
list; it now covers observer removal, thread guards and equal replay identity.
After builds/regressions finish, the existing `yomokuMeasureRender=true` commands
also emit `REQUEST` rows and measure one `equal_confirmation` replay per scenario:

```sh
"$ANDROID_HOME/platform-tools/adb" -s emulator-5554 shell am instrument -w -r -e class app.yomoku.VocabularyPerformanceTest -e yomokuMeasure true -e yomokuMeasureRender true -e yomokuVocabularyEntries 10000 -e yomokuVocabularyLoaded 1000 -e yomokuMeasureIterations 5 app.yomoku.test/androidx.test.runner.AndroidJUnitRunner
"$ANDROID_HOME/platform-tools/adb" -s emulator-5554 shell am instrument -w -r -e class app.yomoku.VocabularyPerformanceTest -e yomokuMeasure true -e yomokuMeasureRender true -e yomokuVocabularyEntries 100000 -e yomokuVocabularyLoaded 1000 -e yomokuMeasureIterations 5 app.yomoku.test/androidx.test.runner.AndroidJUnitRunner
```

`REQUEST_COLUMNS` describes event encoding, synchronous native update and request
decoding. Callback encoding/update cells are empty, not zero-duration estimates.
Render `source` can now be `replay`; this diagnostic reads the settled native view
and executes normal decode/publication without dispatching an event or requests.
For `equal_confirmation`, the action CSV's `dispatch_ms` column times that replay
call instead of dispatch. It is not an actual UI action. Exact state identity,
all selected/frozen IDs and no requests are asserted. State-publication timings
include equality and are not isolated equality or composition benchmarks.

2026-10-31 PASS: debug/test APKs, strict Android lint, Kotlin formatting/diff;
21 regressions plus default assumption skip (22 reported, 240.384 s). Separate
10k/100k processes pass in 38.092/60.011 s, preserving all exact-ID/read-only/
integrity/FK/no-analyzer/no-reading-effects/closure/cleanup guards. Validated
80 render/70 request/60 action rows and 111 raw frames. All ten equal renders
retain the original state instance; 70 changed renders publish. At 100k native
Select All/confirmation updates have medians 0.298/0.335 ms; equal publication
median is 5.594 ms (3.936–35.825 ms range). Full raw CSV/results/hashes/limits:
VOCABULARY_ANDROID_PERFORMANCE.md, final request/replay section. No timing
improvement, composition, exact peak or physical-device claim. Native/bindings/
assets reused Gradle outputs; host Rust/Clippy and independent JNI/codegen/Full-
loader/optimized/lifecycle/broader device matrices were not rerun. Initial
sandbox Gradle lock-socket access required approved rerun; no open failure.
ADB confirms scratch directories absent; task-started isolated AVD stopped.
Logs: `/tmp/yomoku-request-{android,format-check,regressions,10000,100000,validation}.log`.

## Controlled Vocabulary ID-list decoding

Use the pinned build/install/environment and isolated API-36 emulator commands
above. This new harness opens no database or CoreSession and is opt-in; its two
correctness cases run normally. Production generated code remains unchanged.

```sh
tools/format-kotlin.sh --check
./android/gradlew -p android :app:assembleDebug :app:assembleDebugAndroidTest :app:lintDebug --offline --console=plain
"$ANDROID_HOME/platform-tools/adb" -s emulator-5554 install -r -t android/app/build/outputs/apk/debug/app-debug.apk
"$ANDROID_HOME/platform-tools/adb" -s emulator-5554 install -r -t android/app/build/outputs/apk/androidTest/debug/app-debug-androidTest.apk
"$ANDROID_HOME/platform-tools/adb" -s emulator-5554 shell am instrument -w -r -e class app.yomoku.CoreSessionTest,app.yomoku.VocabularySelectionLookupTest,app.yomoku.VocabularyDecodingTest app.yomoku.test/androidx.test.runner.AndroidJUnitRunner
"$ANDROID_HOME/platform-tools/adb" -s emulator-5554 shell am instrument -w -r -e class 'app.yomoku.VocabularyDecodingTest#measureIdenticalIdPayloads' -e yomokuMeasureDecode true -e yomokuVocabularyEntries 10000 -e yomokuMeasureIterations 8 app.yomoku.test/androidx.test.runner.AndroidJUnitRunner
"$ANDROID_HOME/platform-tools/adb" -s emulator-5554 shell am instrument -w -r -e class 'app.yomoku.VocabularyDecodingTest#measureIdenticalIdPayloads' -e yomokuMeasureDecode true -e yomokuVocabularyEntries 100000 -e yomokuMeasureIterations 8 app.yomoku.test/androidx.test.runner.AndroidJUnitRunner
"$JAVA_HOME/bin/javap" -c -p android/app/build/intermediates/built_in_kotlinc/debug/compileDebugKotlin/classes/com/novi/serde/BinaryDeserializer.class
git diff --check
```

2026-10-31 PASS: debug/test APKs, strict lint, repository Kotlin formatting/diff;
native/bindings/assets reused Gradle outputs. Six regressions plus the default
measurement assumption skip (seven reported, 0.720 s). Two opt-in processes per
size pass (10k/1 0.671 s, 100k/1 2.156 s, 100k/2 2.135 s, 10k/2 0.651 s), with no
competing builds/tests/profilers. Four warmup/eight measured balanced rounds per
process. All 128 raw rows, exact-ID/generated-byte round trips, immutable inputs,
malformed lengths/truncation/trailers and counters validate. See
VOCABULARY_ANDROID_PERFORMANCE.md, controlled ID-list section, and
measurements/vocabulary-decoding-android-2026-10-31.csv for results and limits.
Capacity-only timings overlap generated; specialized readers are faster in these
synthetic samples but omit generic/container calls too. Bytecode exposes per-read
range/progression construction in generated 64-bit loops. Approximate process ART
deltas include lag/GC/unrelated work and do not establish exact allocations or
peak/retained memory. Do not infer full snapshot/action/composition improvements.
No production/Rust/schema/dependency/wire change. Initial sandbox Gradle socket
failure needed approved rerun; no open source/test failure. Host Rust/Clippy/JNI,
independent generation and full Compose/native/optimized/arm64/lifecycle profiles
were not rerun. Logs: /tmp/yomoku-decoding-{android,format-check,regressions,
10000-1,10000-2,100000-1,100000-2,validation}.log; bytecode -bytecode.txt.


## Scalar Kotlin decoder integration

The payload generator adapts only the pinned unsigned/signed 64-bit loops; all
other generated files and list/container code remain unchanged. Shared scalar
and frozen-snapshot checks run in both the host JNI runner and Android tests.
The opt-in comparison uses unchanged generated codecs with the test-only legacy
scalar reader. No database, analyzer, Compose publication or event is involved.

```sh
export PATH=/home/kai/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin:$PATH
export JAVA_HOME="$PWD/.local/jdk-17/jdk-17.0.21+9"
export JAVA_BIN="$JAVA_HOME/bin/java"
export GRADLE_USER_HOME="$PWD/.local/gradle"
export ANDROID_HOME=/home/kai/Android/Sdk
cargo test -p yomoku-ffi --bin generate-kotlin --features facet_typegen --locked --offline
cargo clippy -p yomoku-ffi --bin generate-kotlin --features facet_typegen --locked --offline -- -D warnings
cargo fmt --all -- --check
tools/test-kotlin-bridge.sh
# No concurrent bridge/native builds during generation.
cargo run -p yomoku-ffi --bin generate-kotlin --features facet_typegen --locked --offline -- target/generated/kotlin-scalar-repeat
diff -r target/generated/kotlin target/generated/kotlin-scalar-repeat
./android/gradlew -p android :app:assembleDebug :app:assembleDebugAndroidTest :app:lintDebug --offline --console=plain
tools/format-kotlin.sh --check
git diff --check
"$ANDROID_HOME/platform-tools/adb" -s emulator-5554 install -r -t android/app/build/outputs/apk/debug/app-debug.apk
"$ANDROID_HOME/platform-tools/adb" -s emulator-5554 install -r -t android/app/build/outputs/apk/androidTest/debug/app-debug-androidTest.apk
"$ANDROID_HOME/platform-tools/adb" -s emulator-5554 shell am instrument -w -r -e class app.yomoku.CoreSessionTest,app.yomoku.VocabularySelectionLookupTest,app.yomoku.VocabularyDecodingTest,app.yomoku.ScalarDecodingTest app.yomoku.test/androidx.test.runner.AndroidJUnitRunner
"$ANDROID_HOME/platform-tools/adb" -s emulator-5554 shell am instrument -w -r -e class app.yomoku.VocabularyProductionTest,app.yomoku.StatisticsProductionTest,app.yomoku.EffectiveReaderProductionTest app.yomoku.test/androidx.test.runner.AndroidJUnitRunner
# Four fresh instrumentation processes, sequentially: 10k/1, 100k/1, 100k/2, 10k/2.
# Substitute 10000/100000 for the entry count; no competing builds/tests/profilers.
"$ANDROID_HOME/platform-tools/adb" -s emulator-5554 shell am instrument -w -r -e class 'app.yomoku.ScalarDecodingTest#measureLegacyAndGenerated' -e yomokuMeasureScalar true -e yomokuVocabularyEntries 100000 app.yomoku.test/androidx.test.runner.AndroidJUnitRunner
"$JAVA_HOME/bin/javap" -c -p android/app/build/intermediates/built_in_kotlinc/debug/compileDebugKotlin/classes/com/novi/serde/BinaryDeserializer.class
"$JAVA_HOME/bin/javap" -c -p android/app/build/intermediates/built_in_kotlinc/debugAndroidTest/compileDebugAndroidTestKotlin/classes/app/yomoku/LegacyScalarDeserializer.class
```


2026-10-31 PASS: generator drift guard, targeted strict Clippy, Rustfmt, shared/
repository Kotlin formatting/diff, full real-Full host JNI/codecs/Unicode fixtures,
latest shared JVM checks and all 17 repeated Kotlin files identical. Only the two
BinaryDeserializer loops differ from the previous output. Production bytecode
has no RangesKt calls; legacy oracle has until/step in both loops. Debug/test APKs
and strict Android lint pass. Eight focused cases plus two default assumption
skips pass (10 reported, 1.226 s); all eight production Vocabulary/Statistics/
effective-reader cases pass (249.438 s). Four measurement processes pass in
1.441/3.433/3.416/0.922 s, with all 128 rows/positions/payload hashes/exact IDs/
round trips validated. 100k synthetic frozen medians legacy 55.207/53.924 ms
versus scalar 34.515/35.366 ms; not a full callback/composition/speed/peak claim.
Results/raw CSV/hashes/limits: VOCABULARY_ANDROID_PERFORMANCE.md, final section.
Initial sandbox Gradle lock sockets required approved rerun; existing nonfatal
host teardown diagnostics persist, with no source/test failure or open blocker.
Logs: /tmp/yomoku-scalar-{generator-test,clippy,bridge,host,codegen,codegen-diff,
android,format,regressions,production,10000-1,10000-2,100000-1,100000-2,
validation}.log; -bytecode.txt and -legacy-bytecode.txt. Task-started isolated AVD
stopped; ignored caches/image retained. No schema/dependency/wire change. Full
workspace Rust/Clippy, broader history/Compose/optimized/arm64/peak/lifecycle
checks remain outside this slice; see the handoff.

## Vocabulary callback queue and Compose passes

Use the pinned Rust/JDK/SDK/Gradle environment, build/install commands and isolated
phone/light API-36 AVD from the preceding sections. Native/assets/bindings need
no source change. The new composition observer is instrumentation-only and opts
into the already pinned Compose runtime APIs; no dependency is added.

```sh
./android/gradlew -p android :app:assembleDebug :app:assembleDebugAndroidTest :app:lintDebug --offline --console=plain
tools/format-kotlin.sh --check
git diff --check
"$ANDROID_HOME/platform-tools/adb" -s emulator-5554 install -r -t android/app/build/outputs/apk/debug/app-debug.apk
"$ANDROID_HOME/platform-tools/adb" -s emulator-5554 install -r -t android/app/build/outputs/apk/androidTest/debug/app-debug-androidTest.apk
callback_cases=app.yomoku.CoreSessionTest,app.yomoku.CompositionMeasurementsTest,app.yomoku.VocabularySelectionLookupTest,app.yomoku.VocabularyDecodingTest,app.yomoku.ScalarDecodingTest,app.yomoku.VocabularyPerformanceTest
"$ANDROID_HOME/platform-tools/adb" -s emulator-5554 shell am instrument -w -r -e class "$callback_cases" app.yomoku.test/androidx.test.runner.AndroidJUnitRunner
vocabulary_cases=app.yomoku.VocabularyScreenTest,app.yomoku.VocabularyProductionTest,app.yomoku.NavigationTest,app.yomoku.EffectiveReaderNavigationTest
"$ANDROID_HOME/platform-tools/adb" -s emulator-5554 shell am instrument -w -r -e class "$vocabulary_cases" app.yomoku.test/androidx.test.runner.AndroidJUnitRunner
# Separate fresh processes, 10000 then 100000; no concurrent builds/tests/profilers.
"$ANDROID_HOME/platform-tools/adb" -s emulator-5554 shell am instrument -w -r -e class app.yomoku.VocabularyPerformanceTest -e yomokuMeasure true -e yomokuMeasureRender true -e yomokuMeasureComposition true -e yomokuVocabularyEntries 10000 -e yomokuVocabularyLoaded 1000 -e yomokuMeasureIterations 5 app.yomoku.test/androidx.test.runner.AndroidJUnitRunner
"$ANDROID_HOME/platform-tools/adb" -s emulator-5554 shell am instrument -w -r -e class app.yomoku.VocabularyPerformanceTest -e yomokuMeasure true -e yomokuMeasureRender true -e yomokuMeasureComposition true -e yomokuVocabularyEntries 100000 -e yomokuVocabularyLoaded 1000 -e yomokuMeasureIterations 5 app.yomoku.test/androidx.test.runner.AndroidJUnitRunner
```

`CALLBACK_COLUMNS` records Kotlin listener arrival relative to the action start,
posted main-handler queue delay and handler elapsed time. Arrival includes action
scheduling/setup and is not a native worker duration. Handler time includes the
existing render/request observers and excludes its own observer invocation.
`COMPOSITION_COLUMNS` records start/end offsets, duration and nesting depth for
each pass. Use interval union, never the sum of overlapping nested passes; apply,
layout/draw and scheduler waiting remain separate. Observer hooks add work even
when scope callbacks are no-ops. End-of-action now includes a main-thread barrier
that stops pass collection before taking the end timestamp. See
VOCABULARY_ANDROID_PERFORMANCE.md for results, raw CSVs and remaining work.

2026-10-31 PASS: debug/test APKs, strict Android lint, repository Kotlin
formatting/diff. Native/bindings/assets reused build outputs. Ten focused cases
plus three default opt-in skips pass (13 reported, 11.216 s), followed by all 17
Vocabulary UI/production/navigation regressions (221.766 s). Separate 10k/100k
profiles pass (37.639/54.632 s); 60 actions/80 renders/70 requests/30 callbacks/
1,982 composition intervals/108 raw frames validate, including full exact IDs,
unchanged-file/no-analyzer/no-reading-effect/integrity/FK/closure/cleanup guards.
Every equal replay retains the existing state with no request/callback/pass.
100k Select All median queue/handler/composition union is 54.153/71.460/192.414 ms;
these observer/debug timings establish no before/after speed improvement. Results,
hashes and six raw CSVs: VOCABULARY_ANDROID_PERFORMANCE.md, final section.
Logs: /tmp/yomoku-callback-{android,format,focused,regressions,10000,100000,
validation}.log; validator /tmp/yomoku-validate-callback-profile.py. Initial
sandbox Gradle local-socket failure needed approved rerun; no source/test failure
or open blocker. Task-started AVD stopped; existing ignored image/caches retained.
No dependency/schema/wire change. Host Rust/Clippy/JNI/codegen, independent Full
loader, native worker breakdown, optimized/tablet/dark/fling/heavy-history/diverse
books/full-load/exact peaks/lifecycle/process-death/physical arm64 not rerun.
