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
with 1.99.1. Current host baseline: 102 tests (six app, nine reader flow, six document, 14 pagination,
five vocabulary, five bridge, four async document, two native facade and 51 storage tests).

## Pure logical pagination checks
`cargo test -p yomoku-core --test pagination --all-features --locked --offline`
runs the chapter/page/reflow fixtures without Android or SQL. All five presets
and 25 reflow pairs cover exact thresholds, LF/CRLF, mixed scripts, supplementary
Unicode, combining marks, sentence fallback/quotes, EOF and invalid ownership.
Workspace build/test/fmt/clippy pass after adding this module. Android renders
the containing page at the persisted exact offset and global page-length preset.
Page movement/completion remains next.

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
