<p align="center">
  <img src="android/app/src/main/res/mipmap-xhdpi/ic_launcher.webp" width="96" height="96" alt="Yomoku app icon">
</p>

<h1 align="center">Yomoku</h1>

<p align="center"><em>Learn Japanese by reading</em></p>

Yomoku is an Android reader that keeps track of the Japanese words you come across. Import EPUB books, plain-text files, and SRT subtitles, or paste Japanese text directly into the app.

- **Look up words.** Tap a word to see its reading and English definition in the built-in offline dictionary.
- **Track your vocabulary.** See how often you’ve come across a word and mark it as learning or known.
- **Make Anki flashcards.** Send words to AnkiDroid with the original sentence when you export from the reader.

I like learning Japanese by reading. I found Lute a hassle to set up, so I built Yomoku to bring the reading and vocabulary workflow I liked to my Android phone. [Lute project](https://github.com/LuteOrg/lute-v3)

## Build and verify

Run these commands from the repository root.

### Rust checks

```sh
cargo fetch --locked
cargo test --workspace --all-features --locked --offline
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked --offline -- -D warnings
```

### Android builds

Use JDK 17, Android SDK platform 37.0, build tools 36.0.0, platform-tools, and NDK 30.0.16248370. To reproduce the Android dictionary bundle, use Python 3.14.8 with SQLite 3.51.2 and the pinned files in the [dictionary guide](third_party/dictionaries/README.md).

```sh
rustup target add aarch64-linux-android x86_64-linux-android
cargo install boltffi_cli --version 0.32.1 --locked --root .local
python3 tools/prepare-sudachi.py
python3 tools/prepare-dictionaries.py --sources .local/dictionaries/upstream

export JAVA_HOME=/path/to/jdk-17
export ANDROID_HOME="$HOME/Android/Sdk"
./android/gradlew -p android :app:assembleDebug :app:lintDebug --offline --console=plain
./android/gradlew -p android :app:assembleRelease :app:lintRelease --offline --console=plain
```

Omit Gradle’s `--offline` flag when downloading dependencies for the first time.

[Development guide](docs/DEVELOPMENT.md) · [Architecture](docs/ARCHITECTURE.md) · [Product spec](docs/PRODUCT_SPEC.md)

Yomoku’s code is available under the [MIT License](LICENSE).
