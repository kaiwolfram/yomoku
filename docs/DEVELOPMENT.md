# CLI development

Run commands from the repository root. Android Studio is optional. Current
behavior is defined by [PRODUCT_SPEC.md](PRODUCT_SPEC.md). Performance reports and
CSV measurements in this directory retain their original measurement context.

## Tools and assets

Use Fedora-compatible tooling: Rust/Cargo with rustfmt/Clippy, a C compiler and
Python 3.11+; Android additionally needs JDK 17, SDK platform 37.0, build tools
36.0.0, platform-tools and NDK **30.0.16248370**. Gradle is pinned by the wrapper;
Android/Kotlin dependency versions are in `android/gradle/libs.versions.toml`.
The locked Rust graph has been tested with Rust 1.100.0; this is not a minimum
supported-version declaration. Install dependencies once before offline checks.
The Android build uses Gradle 9.9.1, AGP 9.5.1, Compose BOM 2026.09.00
(UI 1.12.1) and Lifecycle 2.11.0. compileSdk and targetSdk are 37;
Java/Kotlin bytecode targets 17. Kotlin/Compose compiler remains
2.4.21. This Gradle/AGP pair exceeds JetBrains' fully tested range; verify the
local build and instrumentation when changing any of these pins. See
[AGP compatibility](https://developer.android.com/build/releases/agp-9-5-0-release-notes),
[Kotlin compatibility](https://kotlinlang.org/docs/gradle-configure-project.html),
[Compose BOM mapping](https://developer.android.com/develop/ui/compose/bom/bom-mapping)
and the [Gradle 9.9.1 security fix](https://github.com/gradle/gradle/security/advisories/GHSA-3cfg-jpcr-gfp3).

```sh
cargo fetch --locked
rustup target add aarch64-linux-android x86_64-linux-android
cargo install boltffi_cli --version 0.32.1 --locked --root .local
python3 tools/prepare-sudachi.py
# Offline alternative: tools/prepare-sudachi.py --archive /path/to/pinned-full.zip
python3 tools/prepare-dictionaries.py --sources .local/dictionaries/upstream
```

Dictionary preparation verifies pinned bytes and does not silently update sources.
See [Sudachi](../third_party/sudachi/README.md) and
[JMdict/JMnedict](../third_party/dictionaries/README.md) for exact inputs/licenses.
Gradle bundles prepared assets without implicit downloads. Keep dictionary bytes
immutable while mapped readers are alive.

Set `JAVA_HOME` to a JDK 17 directory and `ANDROID_HOME` to your SDK. An optional
`GRADLE_USER_HOME="$PWD/.local/gradle"` keeps Gradle cache in the checkout. In
restricted environments, bypass rustup launcher maintenance with the installed
stable toolchain's `bin` in PATH. Gradle/ADB/emulator need local socket access.
Do not clear an incompatible database to make a build or trial succeed. The only
accepted initial journal is `0001_base_text_initial_schema` (schema version 1),
which includes the Statistics timeframe preference.
Use a new disposable AVD for checks that require a fresh installation.

## Rust verification

```sh
cargo build --workspace --locked --offline
cargo test --workspace --all-features --locked --offline
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked --offline -- -D warnings
tools/test-sudachi.sh
```

The workspace suite explicitly ignores fixtures requiring Full; the last command
runs them with both owned and mapped Full loaders. The full read-only storage index fixture uses
`YOMOKU_DICTIONARY_INDEX` pointing to the prepared index and `cargo test -p
yomoku-storage --locked --offline -- --ignored`.
Follow scripts' environment requirements rather than treating ignored tests as
passed. Parser/dictionary compiler tests are available in `tools/test-prepare-dictionaries.py`
and `tools/test-bundle-dictionaries.py`.

## Kotlin payload generation

```sh
cargo run -p yomoku-ffi --bin generate-kotlin --features facet_typegen --locked --offline -- target/generated/kotlin
cargo run -p yomoku-ffi --bin generate-kotlin --features facet_typegen --locked --offline -- target/generated/kotlin-repeat
diff -r target/generated/kotlin target/generated/kotlin-repeat
```

Generated Kotlin stays under ignored `target/`. Crux codecs use Facet; BoltFFI
0.32.1 supplies JNI ownership/platform entry points. Regenerate payloads and
Unicode fixtures whenever Rust interfaces change. ReaderDocument codecs remain
explicitly registered for standalone Unicode fixtures, not the live render view.

## Kotlin formatting

Install the pinned ktfmt 0.64 all-dependencies JAR as `.local/ktfmt-0.64.jar`,
or set `KTFMT_JAR`. The checked-in script formats Android/shared fixtures:

```sh
tools/format-kotlin.sh
tools/format-kotlin.sh --check
# Host-only tests/kotlin files use the same formatter:
java -jar .local/ktfmt-0.64.jar --kotlinlang-style tests/kotlin
```

## Kotlin/JNI host verification

Install Kotlin CLI 2.4.21 under `.local/kotlin-2.4.21/kotlinc`, or set
`KOTLINC_BIN`. Set `JAVA_BIN`/`JNI_INCLUDE_DIR` if the selected JDK differs.
`BOLTFFI_BIN` defaults to `.local/bin/boltffi`. Prepared Full and word dictionaries
are required; `YOMOKU_SUDACHI_DICTIONARY` and `YOMOKU_WORD_DICTIONARY` override paths.

```sh
tools/test-kotlin-bridge.sh
```

This generates codecs/JNI, rebuilds native host exports after metadata expansion,
round-trips Unicode fixtures and executes real FIFO/import/lifecycle bridge cases
with `-Xcheck:jni`. It holds the native-generation lock. Run standalone Cargo
checks before or after it: BoltFFI metadata expansion shares host Cargo artifacts
and can invalidate an overlapping Rust/doc-test link.

## Android builds

```sh
./android/gradlew -p android :app:assembleDebug :app:assembleDebugAndroidTest :app:lintDebug --offline --console=plain
./android/gradlew -p android :app:assembleRelease :app:lintRelease --offline --console=plain
```

Omit `--offline` only when explicitly fetching initial Gradle dependencies.
Both variants package arm64-v8a and x86_64. Debug automatically uses the Rust
debug profile; release automatically passes `--release`. Outputs are separate:
`target/generated/{debug,release}/{kotlin,native,android}`. Native tasks serialize,
including debug probes, and share a cross-process lock. There is no manual native
prebuild/task-exclusion recipe. Release excludes probe libraries and debug classes.

APKs are under `android/app/build/outputs/apk/`. Release is unsigned; never commit
signing keys and obtain approval before signed publishing. Inspect both ABI JNI
libraries, ELF LOAD alignment and `zipalign -c -P 16 -v 4` on a release candidate.
Verify no `libandroid_app_probe.so` or debug probe classes enter release.

## Android trials

Use an authorized device or a dedicated disposable AVD with an installed x86_64
image and host KVM. Use an API 37 image for Android 17 target behavior checks,
and an older image for backward compatibility. The helper starts an existing
`medium_phone` AVD by default:

```sh
tools/start-android-emulator.sh medium_phone
"$ANDROID_HOME/platform-tools/adb" devices -l
./android/gradlew -p android :app:connectedDebugAndroidTest --offline --console=plain
```

The denied-notification import case must run alone with notification permission
revoked before instrumentation starts (revoking it during a test kills the app):

```sh
"$ANDROID_HOME/platform-tools/adb" -e shell pm revoke app.yomoku android.permission.POST_NOTIFICATIONS
"$ANDROID_HOME/platform-tools/adb" -e shell am instrument -w -r -e class app.yomoku.ImportNotificationsTest#deniedNotificationsKeepImportDialogWithoutPermissionActionOrPrompt app.yomoku.test/androidx.test.runner.AndroidJUnitRunner
```

It skips when notifications are enabled; the other import-notification cases grant
permission for their delivery checks. The denied case starts an import directly and
checks the active dialog without requesting permission inside it.

Run the permission-before-picker regression separately for each Android 13+ choice
on the disposable emulator. Reset permission before starting instrumentation, since
revoking it during instrumentation kills the app:

```sh
# Repeat this reset and instrumentation command with grant, deny, and blocked.
"$ANDROID_HOME/platform-tools/adb" -e shell pm revoke app.yomoku android.permission.POST_NOTIFICATIONS
"$ANDROID_HOME/platform-tools/adb" -e shell pm clear-permission-flags app.yomoku android.permission.POST_NOTIFICATIONS user-set user-fixed
# For blocked only, prevent the runtime prompt:
# "$ANDROID_HOME/platform-tools/adb" -e shell pm set-permission-flags app.yomoku android.permission.POST_NOTIFICATIONS user-set user-fixed
"$ANDROID_HOME/platform-tools/adb" -e shell am instrument -w -r -e class app.yomoku.ImportBookPermissionTest -e notificationChoice grant app.yomoku.test/androidx.test.runner.AndroidJUnitRunner
```

The default `notificationChoice` is `granted`; it grants permission before tapping
Import book and checks the direct-picker path. Grant/deny recreate the activity
while the system prompt is pending. Each path checks one picker launch, cancellation,
and unchanged Library state. `ImportBookRequestTest` covers duplicate/stale callbacks
and discarded process state. On pre-33 Android the picker opens directly.

Instrumentation pins Espresso 3.7.0 for Android 17's lock-free MessageQueue
compatibility; see the [platform guidance](https://developer.android.com/about/versions/17/changes/messagequeue).
Compose instrumentation uses the v2 test rules and their standard coroutine
dispatcher. Synchronize queued composition work with `waitForIdle` or `runOnIdle`
before asserting; see the [v2 migration guide](https://developer.android.com/develop/ui/compose/testing/migrate-v2).

Use instrumentation class filters for focused reader/provider cases. Do not wipe
an ordinary installation or accept unsupported development data by resetting it.
An isolated fresh AVD is suitable for initial-schema tests. Reader trials should
cover import, lookup/no-match, status, Next/Previous, reread and recreation.
Anki fake-provider tests cover failures, duplicate refresh, guards and model
compatibility; they do not establish real AnkiDroid collection/browser behavior.
Trial a disposable real collection before release and a physical arm64 device
before claiming that ABI's runtime compatibility.

The installed-provider regression is opt-in and mutates a disposable Anki collection.
Initialize AnkiDroid in a new isolated emulator, install both debug APKs and grant
its declared database permission to Yomoku. Never enable this against a personal
collection; it creates the Yomoku model and test notes, exercises external deletion/
creation, opens the browser and restarts the Yomoku session.

```sh
"$ANDROID_HOME/platform-tools/adb" -e shell pm grant app.yomoku com.ichi2.anki.permission.READ_WRITE_DATABASE
"$ANDROID_HOME/platform-tools/adb" -e shell am instrument -w -r -e class app.yomoku.AnkiInstalledProviderTest -e disposableAnkiTrial true app.yomoku.test/androidx.test.runner.AndroidJUnitRunner
```

Manual encrypted backup/restore is unfinished separate release work. A successful
APK build or primitive backup round trip does not complete that requirement.
