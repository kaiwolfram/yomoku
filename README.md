<p align="center">
  <img src="android/app/src/main/res/mipmap-xhdpi/ic_launcher.webp" width="96" height="96" alt="Yomoku app icon">
</p>

<h1 align="center">Yomoku</h1>

<p align="center"><em>Learn Japanese by reading</em></p>

Yomoku is an Android reader that keeps track of the Japanese words you come across. Import EPUB books, plain-text files, and SRT subtitles, or paste Japanese text directly into the app.

- **Look up words.** Tap a word to see its reading and English definition in the built-in offline dictionary.
- **Track your vocabulary.** See how often you’ve come across a word and mark it as learning or known.
- **Make Anki flashcards.** Send words to [AnkiDroid](https://github.com/ankidroid/Anki-Android) with the original sentence when you export from the reader.

I like learning Japanese by reading. I found [Lute](https://github.com/LuteOrg/lute-v3) a hassle to set up, so I built Yomoku to bring the reading and vocabulary workflow I liked to my Android phone.

## Build the Android release

Run these commands from the repository root.

### Prepare the toolchain and dictionaries

Use JDK 17, Rust with Cargo, a C compiler, Git, `rg`, and `flock`. Install Android
SDK Platform 37.0, Build Tools 36.0.0, Platform-Tools, and NDK 30.0.16248370.

Use Python 3.14.8 with SQLite 3.51.2 to reproduce the Android dictionary bundle.
Place the pinned 2026-10-25 `JMdict_e.gz` and `JMnedict.xml.gz` files in
`.local/dictionaries/upstream/`. The upstream downloads rotate daily. See the
[dictionary guide](third_party/dictionaries/README.md) for the pinned inputs and
how to recover them from an APK that bundles them.

```sh
export JAVA_HOME=/path/to/jdk-17
export ANDROID_HOME="$HOME/Android/Sdk"

rustup target add aarch64-linux-android x86_64-linux-android
cargo fetch --locked
cargo install boltffi_cli --version 0.32.1 --locked --root .local
python3 tools/prepare-sudachi.py
python3 tools/prepare-dictionaries.py --sources .local/dictionaries/upstream
```

For offline Sudachi preparation, see the [pinned archive instructions](third_party/sudachi/README.md).

### Build and lint the release

```sh
./android/gradlew -p android :app:assembleRelease :app:lintRelease --console=plain
```

Add `--offline` after Gradle has cached its dependencies. Gradle builds the Rust
JNI libraries in release mode and writes two unsigned APKs to
`android/app/build/outputs/apk/release/`.

| APK | Supported ABIs |
| --- | --- |
| `app-release-unsigned.apk` | ARM64 (`arm64-v8a`) and `x86_64` |
| `app-arm64-v8a-release-unsigned.apk` | ARM64 only |

### Sign and install an APK

Use the same release key for updates and keep the keystore outside the checkout.
If you need a keystore, see the
[Android command-line signing guide](https://developer.android.com/build/building-cmdline#sign_cmdline).

This example signs the universal APK. For ARM64 only, use
`app-arm64-v8a-release-unsigned.apk` and add `-arm64-v8a` before `-release-signed`
in the output filename.

```sh
build_tools="$ANDROID_HOME/build-tools/36.0.0"
apk_dir=android/app/build/outputs/apk/release
version=0.1.0  # Use versionName from android/app/build.gradle.kts.
unsigned_apk="$apk_dir/app-release-unsigned.apk"
signed_apk=".local/apks/yomoku-$version-$(git rev-parse --short HEAD)-release-signed.apk"

mkdir -p .local/apks
"$build_tools/zipalign" -c -P 16 4 "$unsigned_apk" &&
  "$build_tools/apksigner" sign --ks /path/to/release-keystore.p12 \
    --out "$signed_apk" "$unsigned_apk" &&
  "$build_tools/apksigner" verify "$signed_apk"
```

After verification succeeds, install on a connected device or emulator.

```sh
"$ANDROID_HOME/platform-tools/adb" install -r "$signed_apk"
```

## Rust checks

```sh
cargo fetch --locked
cargo test --workspace --all-features --locked --offline
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked --offline -- -D warnings
```

[Development guide](docs/DEVELOPMENT.md) · [Architecture](docs/ARCHITECTURE.md) · [Product spec](docs/PRODUCT_SPEC.md)

Yomoku’s code is available under the [MIT License](LICENSE).
