# Yomoku

*Read Japanese, naturally.*

Yomoku is a native, offline-first Android app for reading Japanese and tracking
vocabulary and reading progress. It uses Kotlin and Jetpack Compose for the UI,
a pure Rust Crux core for application logic, SQLite for local persistence, and
Sudachi Full for Japanese word segmentation.

## Current status

Yomoku is under active development. The working app supports:

- Pasting text into a local Library, with exact-content deduplication and an
  import summary. Imports analyze Japanese text in Sudachi Mode B and register
  unique Japanese word surfaces without changing existing vocabulary statuses.
- Opening imported text in a centered, scrollable reader, with saved text
  positions and Library progress percentages.
- Five persisted logical page-length presets: 250, 500, 750, 1,000 and 1,500
  non-whitespace characters. Changing the preset preserves the exact text offset.
- Phone and tablet navigation, retained sessions across Activity recreation,
  and offline access to bundled Sudachi notices.
- Hidden active reading time, with automatic restore, interaction/idle accounting,
  periodic saves and final saves before background, reader actions and teardown.
- Next/Previous controls and horizontal swipes, with a content-bottom page counter,
  persisted progress in both directions and scroll reset on page navigation.
- Forward completion records repeated Japanese encounters, promotes New words to
  Known and shows a positive-count toast; deliberate rereads count again.
- Japanese words have status highlights and open a panel with their full surface,
  analyzer reading/base form and five direct status choices. Opening it leaves
  statuses and history unchanged; choices persist globally across repeated surfaces.

Dictionary definitions and encounter counts remain pending in the app. A
[pinned offline JMdict/JMnedict compiler](third_party/dictionaries/README.md)
prepares the English word/name index with complete attribution and lookup fixtures.
Rust now reads this index through a separate read-only repository, with typed
surface/base candidates and pure spelling/reading/sense restriction rules.
Android bundles the pinned index, reproducible source inputs and complete offline
Settings credits. Lazy private installation verifies size/SHA-256 before exposing
a read-only path; runtime and word-panel integration remain pending.
Compose reads stored Mode B tokens through typed core/FIFO/FFI, validates the
current page identity and
converts exact UTF-8 spans to UTF-16 without splitting Unicode scalars. Status
choices update all visible repetitions after the status/history transaction commits;
same-status selections add no history. Deleted vocabulary stays unregistered.
The Rust repository atomically saves completion encounters, New→Known
transitions, active time and position. Core/worker/FFI pending-time checkpoints
support exact retries, coalesced saves before reader movement/exit and reopen.
Explicit analyzed completion intents now drain checkpoints, freeze retries and update
progress through core/FIFO/FFI, without analyzer loading. Android's retained driver
supplies monotonic samples and waits for pending saves before native teardown.
Permanent finish checkmarks appear in Library and reader independently of current
progress. Explicitly reopening a finished book persists page 1; active-reader
recreation and completion reloads keep their current position. Finish Book (or a
final-page left swipe) completes the save and returns to Library with brief confetti
on each newly recorded completion, including rereads with zero new words.
Nostr, Statistics and Vocabulary tabs currently
show placeholders.

Planned v1 features include EPUB/TXT/SRT imports, offline JMdict/JMnedict lookup,
reading statistics, optional AnkiDroid integration, curated Nostr reading and
manual encrypted backups. See the [product specification](docs/PRODUCT_SPEC.md)
for scope and the [implementation checklist](docs/IMPLEMENTATION_PLAN.md) for
verified progress.

## Repository layout

| Path | Responsibility |
| --- | --- |
| `android/` | Compose UI and Android platform integration |
| `crates/yomoku-core/` | Pure Crux state, events, pagination and domain rules |
| `crates/yomoku-runtime/` | Rust EffectRouter and FIFO worker for I/O and analysis |
| `crates/yomoku-storage/` | SQLite repositories and numbered migrations |
| `crates/yomoku-analysis/` | Sudachi adapter and validated token snapshots |
| `crates/yomoku-ffi/` | Typed Crux/BoltFFI bridge and Kotlin payload generation |
| `third_party/sudachi/` | Pinned engine source, Android patch and license records |
| `third_party/dictionaries/` | JMdict/JMnedict source lock, attribution and index contract |
| `tools/` | CLI build, formatting, data preparation and verification scripts |
| `docs/` | Product requirements, architecture, development guide and handoff |

SQL and platform I/O stay outside the pure core. Android sends typed events
through generated bindings; Rust executes effects and returns render state.
See [architecture](docs/ARCHITECTURE.md) and [data model](docs/DATA_MODEL.md).

## Build and test

Run commands from the repository root. Normal development uses CLI tools and
does not require Android Studio. The [development guide](docs/DEVELOPMENT.md)
contains complete Fedora setup, tool overrides and emulator troubleshooting.

### Host Rust checks

Install Rust with Cargo, rustfmt and Clippy, plus a C compiler for bundled SQLite.
The locked dependency graph has been tested with Rust 1.99.1; the full project's
minimum supported Rust version has not been established.

```sh
# Fetch dependencies once with network access.
cargo fetch --locked

cargo build --workspace --locked --offline
cargo test --workspace --all-features --locked --offline
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked --offline -- -D warnings
```

These checks do not require Android or downloaded dictionary data. Real Full
dictionary fixtures run separately after data preparation:

```sh
# Requires Python 3.11+. Downloads and verifies the pinned Full dictionary.
python3 tools/prepare-sudachi.py
# For offline preparation, use --archive /path/to/pinned-full.zip instead.
tools/test-sudachi.sh
```

Dictionary data and complete redistribution notices are prepared under ignored
`.local/sudachi/`. Keep dictionary bytes immutable while analyzers or mapped
tests are running. The [Sudachi notes](third_party/sudachi/README.md) document
the pinned source, data and licenses.

### Android app

Install JDK 17, Python 3.11+, the Android SDK platform 36, build tools 36.0.0,
platform-tools, and NDK r30 (`30.0.16248370`). The Gradle wrapper pins the remaining
Android build dependencies. Prepare Sudachi Full as above before building; Android
builds bundle already verified dictionary data and never download it implicitly.
Also prepare the pinned word/name index before Android builds (see the
[dictionary setup](third_party/dictionaries/README.md)); Gradle verifies its locked
manifest, index, source archives and notices without network access.

```sh
export JAVA_HOME=/path/to/jdk-17
export ANDROID_HOME="$HOME/Android/Sdk"

rustup target add aarch64-linux-android x86_64-linux-android
cargo install boltffi_cli --version 0.30.1 --locked --root .local

# Allow network access for the first Gradle dependency download.
./android/gradlew -p android :app:assembleDebug :app:lintDebug --console=plain
```

The app requires Android 10 (API 29) or newer. Builds package both `arm64-v8a`
and `x86_64`; Gradle generates the bindings and builds the Rust/JNI libraries.
The debug APK is `android/app/build/outputs/apk/debug/app-debug.apk`.

With an authorized device or emulator connected:

```sh
./android/gradlew -p android :app:connectedDebugAndroidTest --console=plain
./android/gradlew -p android :app:installDebug --console=plain
"$ANDROID_HOME/platform-tools/adb" shell am start -n app.yomoku/.MainActivity
```

Run native builds, binding generation and JNI tests sequentially because they
share generated outputs. The development guide also covers
[Kotlin formatting](docs/DEVELOPMENT.md#kotlin-formatting) and
[host Kotlin/JNI tests](docs/DEVELOPMENT.md#kotlinjni-host-verification).
Android execution has been verified on an API 36 x86_64 emulator; physical arm64
validation remains pending. Measurements and their limits are recorded in
[Sudachi measurements](docs/SUDACHI_MEASUREMENTS.md).

## Contributing

Start with [AGENTS.md](AGENTS.md) and the [current handoff](docs/NEXT_SESSION.md),
then follow the relevant unchecked implementation tasks. The product
specification is the source of truth; record routine technical choices in
[decisions](docs/DECISIONS.md).

## License

Original Yomoku code is licensed under the [MIT License](LICENSE). Third-party
engine code and dictionary data retain their own licenses and attribution
requirements; see the [Sudachi notices](third_party/sudachi/README.md) and
[JMdict/JMnedict notices](third_party/dictionaries/README.md).
