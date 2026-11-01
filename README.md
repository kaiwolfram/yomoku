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
- Selecting multiple TXT files with UTF-8/Shift-JIS encoding choices, per-file
  failures/duplicates, current-file/batch progress and a retained import summary
  returning to Library. Cancel retains completed files. Background imports use a
  foreground service with permitted progress/completion notifications; unfinished
  batches never automatically resume after process death.
- Opening imported text in a centered, scrollable reader, with saved text
  positions and Library progress percentages.
- Searching Library titles with a retained query and a clear action.
- Renaming documents and confirmed single or bulk deletion. Long press selects
  documents; Select All includes offscreen matching titles. Deletion preserves
  vocabulary and reading history; reimports start with fresh progress.
- Library sorting by Last read, Title or Import date, with distinct-word status
  bars that refresh after global status changes and page completion.
- Bundled Noto Serif JP reader text, with regular/bold weights, EPUB emphasis
  and complete font attribution/license available offline in Settings.
- Persisted global font size and line spacing, with unchanged logical pages and
  exact reading positions across typography changes.
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
- A/B/C boundary choices and manual split/merge, with All occurrences (default)
  or This occurrence, saved rules and reversal. Reads reuse stored analysis;
  explicit missing A/C generation uses the existing Full worker.
- Japanese words have status highlights and open a panel with their full surface,
  analyzer reading/base form, expanded English definitions, separate name readings
  and lifetime encounter count. Five direct status choices persist globally across
  repeated surfaces; opening the panel leaves statuses and history unchanged.

The [pinned offline JMdict/JMnedict compiler](third_party/dictionaries/README.md)
prepares the English word/name index with complete attribution and lookup fixtures.
Rust now reads this index through a separate read-only repository, with typed
surface/base candidates and pure spelling/reading/sense restriction rules.
Android bundles the pinned index, reproducible source inputs and complete offline
Settings credits. Lazy private installation verifies size/SHA-256 before exposing
a read-only path. The lazy FIFO/typed JNI supplies the word panel with all applicable
readings/senses, compact grammatical labels, names and exact-surface lifetime counts.
Lookup failures require explicit retry; dismissal, navigation and recreation discard
the transient selection and late results.
Compose reads effective tokens resolved from original Mode B and stored overrides
through typed core/FIFO/FFI, validates the native page revision and
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
Feed shows “Coming Soon! 🌱”; Nostr integration is deferred beyond the first MVP.
Vocabulary supports literal surface-prefix search, five-status filters, four sorts, matching counts and 100-row infinite
scroll with explicit Retry. Tap or long press selects words; global Select All
includes unloaded matches and preserves hidden selections. Confirmed bulk status
changes and permanent deletion expose explicit Retry/Cancel and retain selection
and prompts across Activity recreation. Confirmations and retries freeze
IDs/status/date, and deletion recalculates history without recreating words from
existing books.

The Rust statistics repository derives read-only lifetime Japanese-occurrence
and completed-time totals plus an absolute five-status vocabulary timeline.
Known forms statistics and their dedicated association table have been removed.
Repetitions/rereads count, imports create registration jumps, and deletion semantics
recalculate from saved event rows. UTC daily/weekly/monthly timeframe buckets carry
lifetime cumulative origins and fill empty intervals from one read snapshot.
Typed native worker/core/FFI transport now retains timeframe/as-of queries,
loading/error/Retry state and stale-callback protection. The Statistics screen
renders two overview/drilldown charts, four timeframes, exact interval rows
and explicit error Retry, preserving lifetime origins through navigation/recreation.

The pure [EPUB parser](crates/yomoku-import/src/epub.rs) extracts spine-ordered
chapters, title, headings/emphasis, source ruby, positioned images/covers and
links while ignoring CSS. Its [atomic repository](crates/yomoku-storage/src/epub.rs)
now saves original-file identity, ordered chapters/formatting/ruby, image bytes and
per-chapter Mode B with unique vocabulary registrations. Host fixtures verify
rollback, cancellation, duplicates, read-only reopen and multi-chapter Library
progress; both real Full loaders pass. Whole-document Rust reader repositories now
support ordered chapters, original per-chapter analysis, exact positions/time,
tokens/dictionary details and atomic completion with whole-book finish semantics.
Core/FIFO/generated bridge readers now retain ordered chapters and select each
chapter's original analysis. Worker fixtures verify transitions, backward progress,
exact reflow/time and whole-book finish without Full.
Reader pages now carry validated headings/emphasis/strong styles and website links
through generated Kotlin/JNI into Compose. External links open the browser; internal
links stay inert. Canonical offsets, word highlighting and dictionary taps survive
formatting, page movement and recreation. A bounded one-shot image-byte API reads
verified saved assets on the Rust FIFO without dictionary loading or reader-state
changes. Android decodes supported raster images off main with page memory bounds
and renders at source position/ratio, preserving text/link/word identities and
discarding disposed requests. Tapping an image opens the shared full-screen viewer
with bounded pinch zoom/pan, accessible controls and rotation/recreation recovery.
It reuses the page bitmap and isolates gestures from reader completion swipes.
The typed cover-reference API reads stored path/media/SHA-256 metadata without
copying book or image BLOBs. Library displays bounded cover thumbnails with a
4 MiB memory LRU, verified asset bytes, source ratios and generic fallback icons.
Bounded static SVGs also render off main with fractional source ratios and offline
renderer credits. Embedded base64 PNG/JPEG and nested self-contained SVG resources
share bounded temporary bitmaps, retain their original aspect ratio and work in
the same viewer/cover pipeline. This basic SVG support satisfies v1 scope;
complex styles/resources retain the existing fallback. TXT/EPUB source ruby now
travels through validated chapter/page snapshots and typed JNI, preserving exact
text positions. A default-off furigana preference persists through SQLite and the
typed bridge. Pure core now projects source-preferred readings with
exact effective analyzer fallback, independent of vocabulary status, through
generated JNI. Unresolved manual spans never receive guessed readings.
Settings exposes the furigana switch; Compose paints readings above canonical
text while preserving word/link taps, logical pages and saved offsets.
The pure [SRT parser](crates/yomoku-import/src/srt.rs) has host fixtures for cue
boundaries, formatting, speakers and malformed input. Its atomic repository now
persists original bytes/encoding, canonical cues, Mode B and vocabulary registrations
with duplicate detection and cancellation. Typed FIFO/JNI imports, encoding retries,
duplicate-no-Full and direct cancellation are verified; Android mixed selection is connected.
Import results now use format-independent Library references. TXT/SRT duplicate
attempts can reference an existing EPUB while preserving its whole-book progress,
title and finish marker without decoding or loading Full. EPUB FIFO/typed JNI
imports now validate containers before Full, persist/analyze every chapter atomically,
and share direct cancellation. Host fixtures cover later-chapter/write/commit
rollback, duplicates, original analysis/status/progress and failed-file recovery.
Android supports mixed EPUB/TXT/SRT batches, per-file encoding/errors, progress,
Cancel and completion summaries. Imports return to Library without opening a reader.
Planned v1 features include custom missing-word definitions,
reading statistics, optional AnkiDroid integration and
manual encrypted backups. See the [product specification](docs/PRODUCT_SPEC.md)
for scope and the [implementation checklist](docs/IMPLEMENTATION_PLAN.md) for
verified progress.

The [manual backup format](docs/BACKUP_FORMAT.md) specifies the full-state snapshot
and encryption profile. The isolated Rust backup crate implements strict metadata
and bounded authenticated snapshot streams. Rust storage now creates compact
private SQLite snapshots; complete encrypted fixture round trips preserve every
durable table and ID sequence. Private candidate structure/schema checks and
supported migrations are implemented. Application validation, app export/import
and confirmed replacement restore remain incomplete.

The Rust TXT parser, atomic repository and typed worker/JNI path support strict
UTF-8/Shift-JIS decoding with explicit selection for ambiguous bytes, preserved hard
breaks, and common
Aozora ruby/directive handling. Original-file SHA-256 deduplication, original bytes,
canonical text/ruby, Mode B analysis and vocabulary registration commit together.
Typed errors support encoding retries; duplicates skip analyzer loading even after
reopen. Android now connects multiple-TXT selection, encoding choices and per-file
summaries, current-file/batch progress, transaction-aware Cancel and permitted
background notifications. Android may still stop the process; finished files
remain, and unfinished files require a new explicit selection.

## Repository layout

| Path | Responsibility |
| --- | --- |
| `android/` | Compose UI and Android platform integration |
| `crates/yomoku-core/` | Pure Crux state, events, pagination and domain rules |
| `crates/yomoku-runtime/` | Rust EffectRouter and FIFO worker for I/O and analysis |
| `crates/yomoku-storage/` | SQLite repositories and numbered migrations |
| `crates/yomoku-analysis/` | Sudachi adapter and validated token snapshots |
| `crates/yomoku-import/` | Pure EPUB extraction, TXT/Aozora decoding and SRT parsing |
| `crates/yomoku-backup/` | Versioned metadata and bounded authenticated snapshot streams; app integration pending |
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

### Release APK

Complete the Rust and Android setup above first, including both Rust Android
targets, BoltFFI 0.30.1, prepared Sudachi Full and the pinned word/name index.
Fetch Cargo and Gradle dependencies once before using these offline commands.
Run from the repository root with `JAVA_HOME` and `ANDROID_HOME` set.

**1. Build optimized native libraries and the unsigned release APK.** The normal
Gradle native task builds Rust in debug mode, even for `assembleRelease`. Build
both native ABIs explicitly with `--release`, then exclude that task from Gradle:

```sh
export ANDROID_NDK_HOME="$ANDROID_HOME/ndk/30.0.16248370"
export CARGO_NET_OFFLINE=true

# Stop if any command fails; do not package stale native libraries.
(
  set -eu
  python3 tools/verify-sudachi-source.py
  cargo run -p yomoku-ffi --bin generate-kotlin --features facet_typegen \
    --locked --offline -- target/generated/kotlin
  (cd crates/yomoku-ffi && ../../.local/bin/boltffi pack android --release \
    --deny-skipped --cargo-arg=--locked --cargo-arg=--offline)
  ./android/gradlew -p android :app:assembleRelease :app:lintRelease \
    -x :app:buildRustBridge --offline --console=plain
)
```

Always run the complete sequence; `-x :app:buildRustBridge` alone can package
stale or debug libraries. Do not run other native/debug builds or binding
generation concurrently. The unsigned output is
`android/app/build/outputs/apk/release/app-release-unsigned.apk`; it supports
Android 10+ and includes `arm64-v8a` and `x86_64`. It cannot be installed until signed.

**2. Align and sign for local phone testing.** This uses the workstation's existing
development key, also used by debug builds and the earlier phone-test APK.
If `$HOME/.android/debug.keystore` does not exist, build a debug APK using the
Android app instructions above, then repeat step 1 to restore optimized libraries.

```sh
mkdir -p target/releases
"$ANDROID_HOME/build-tools/36.0.0/zipalign" -f -P 16 4 \
  android/app/build/outputs/apk/release/app-release-unsigned.apk \
  target/releases/yomoku-release-aligned.apk
"$ANDROID_HOME/build-tools/36.0.0/apksigner" sign \
  --ks "$HOME/.android/debug.keystore" --ks-key-alias androiddebugkey \
  --ks-pass pass:android --key-pass pass:android \
  --out target/releases/yomoku-release.apk target/releases/yomoku-release-aligned.apk
```

These are standard development-key passwords. For distribution, use your own
private release key instead. Create it once outside the repository if you do not
already have one, then replace the `apksigner sign` command above with:

```sh
# One-time creation only; keytool prompts for passwords and certificate details.
mkdir -p "$HOME/.android"
"$JAVA_HOME/bin/keytool" -genkeypair -keystore "$HOME/.android/yomoku-release.jks" \
  -alias yomoku -keyalg RSA -keysize 3072 -validity 10000

# Reuse the same key for later releases; apksigner prompts for its password.
"$ANDROID_HOME/build-tools/36.0.0/apksigner" sign \
  --ks "$HOME/.android/yomoku-release.jks" --ks-key-alias yomoku \
  --out target/releases/yomoku-release.apk target/releases/yomoku-release-aligned.apk
```

Keep private keys/passwords out of Git and back up the release key securely.
Before distributing a new version, increase `versionCode` and update `versionName`
in `android/app/build.gradle.kts`, then rebuild. Publishing signed releases
requires approval.

**3. Verify and install.** Align before signing; modifying the signed APK
invalidates its signature. See Android's
[command-line signing guide](https://developer.android.com/build/building-cmdline#sign_cmdline)
and [apksigner reference](https://developer.android.com/tools/apksigner).

```sh
"$ANDROID_HOME/build-tools/36.0.0/apksigner" verify --verbose --print-certs \
  target/releases/yomoku-release.apk
"$ANDROID_HOME/build-tools/36.0.0/zipalign" -c -P 16 4 target/releases/yomoku-release.apk
"$ANDROID_HOME/platform-tools/adb" install -r target/releases/yomoku-release.apk
"$ANDROID_HOME/platform-tools/adb" shell am start -n app.yomoku/.MainActivity
```

The installable APK is **`target/releases/yomoku-release.apk`**. Updating an existing
installation requires its signing certificate and a compatible version code;
use the original workstation development key to update earlier local test builds.
If Android reports a certificate mismatch, use the matching key. Uninstalling
deletes the local Library, vocabulary and history.

## Contributing

Start with [AGENTS.md](AGENTS.md) and the [current handoff](docs/NEXT_SESSION.md),
then follow the relevant unchecked implementation tasks. The product
specification is the source of truth; record routine technical choices in
[decisions](docs/DECISIONS.md).

## License

Original Yomoku code is licensed under the [MIT License](LICENSE). Third-party
engine code and dictionary data retain their own licenses and attribution
requirements; see the [Sudachi notices](third_party/sudachi/README.md) and
[JMdict/JMnedict notices](third_party/dictionaries/README.md),
[AndroidSVG notices](third_party/androidsvg/README.md), and
[Noto Serif JP notices](third_party/noto-serif-jp/README.md).
