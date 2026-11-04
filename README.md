# Yomoku

*Read Japanese, naturally.*

Yomoku is a native offline-first Android reader built with Kotlin/Compose,
a pure Rust Crux core, SQLite and Sudachi Full. It imports EPUB, TXT and SRT,
keeps exact reading positions, tracks vocabulary and reading history, and looks up
words and names in bundled English JMdict/JMnedict dictionaries.

The production reader uses stored Mode C analysis, with A/B dictionary components,
source/analyzer furigana, word highlights, five vocabulary statuses, EPUB images
and links. Completion saves occurrences, status changes, time and position
atomically; deliberate rereads count again. Imports retain original bytes and
canonical content, deduplicate atomically and return to Library.

AnkiDroid is optional. Settings selects a deck and verifies the Yomoku v2 model;
dictionary entries offer Add or existing-note actions based on fresh provider
queries. Only the default deck is persisted. Interrupted external writes are never
automatically replayed. There are no custom dictionary entries or Anki export logs.

The app is under development. Manual encrypted backup/restore remains unfinished
release work. Existing development databases and backups are unsupported and are
rejected without automatic clearing. Nostr/Feed is deferred beyond this release.
The [product specification](docs/PRODUCT_SPEC.md) defines scope.

## Build and verify

Normal development uses Fedora-compatible CLI tools; Android Studio is optional.
Install Rust/Cargo, a C compiler, JDK 17, Python 3.11+, Android SDK platform 36,
build tools 36.0.0, platform-tools and NDK 30.0.16248370. See
[DEVELOPMENT.md](docs/DEVELOPMENT.md) for pinned tools, dictionary preparation,
Kotlin/JNI checks and device setup.

```sh
cargo fetch --locked
cargo test --workspace --all-features --locked --offline
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked --offline -- -D warnings

python3 tools/prepare-sudachi.py
# Prepare the pinned JMdict/JMnedict index per third_party/dictionaries/README.md.
rustup target add aarch64-linux-android x86_64-linux-android
cargo install boltffi_cli --version 0.30.1 --locked --root .local
export JAVA_HOME=/path/to/jdk-17
export ANDROID_HOME="$HOME/Android/Sdk"
./android/gradlew -p android :app:assembleDebug :app:lintDebug --console=plain
./android/gradlew -p android :app:assembleRelease :app:lintRelease --offline --console=plain
```

Gradle selects matching debug/release Rust profiles, generates Kotlin/JNI bindings
and packages arm64-v8a and x86_64. The release APK is unsigned; signed publishing
requires approval. Native generation is serialized; run host JNI and standalone
Cargo verification separately from Android native generation.

## Repository

| Path | Purpose |
| --- | --- |
| `android/` | Compose and platform adapters |
| `crates/yomoku-core/` | Pure application/domain rules |
| `crates/yomoku-runtime/` | EffectRouter and FIFO I/O worker |
| `crates/yomoku-storage/` | SQLite repositories and initial schema |
| `crates/yomoku-analysis/`, `crates/yomoku-import/` | Analysis and pure format parsing |
| `crates/yomoku-ffi/` | Typed bridge and codec generation |
| `crates/yomoku-backup/` | Authenticated snapshot stream primitives |
| `third_party/`, `tools/` | Pinned assets, notices and CLI tooling |

See [architecture](docs/ARCHITECTURE.md), [data model](docs/DATA_MODEL.md),
[Anki provider contract](docs/ANKIDROID_API.md) and
[backup format](docs/BACKUP_FORMAT.md). Historical development evidence and
[Sudachi measurements](docs/SUDACHI_MEASUREMENTS.md) remain available; they do not
override current product behavior or establish current device compatibility.
