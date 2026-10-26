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
