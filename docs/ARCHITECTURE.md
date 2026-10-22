# Architecture

## Implemented
Root Cargo workspace → `crates/yomoku-core`: pure Crux 0.20.0 application plus
surface identity, five vocabulary statuses, registration and completion policies.
`app::Yomoku` implements `App`; `Model` holds a transient destination (Library
default), `Event::SelectDestination` updates it, and a render command asks the
host to read `ViewModel`. The typed effect currently contains only Render.
`yomoku-runtime` maps core effects exhaustively into a separate `ShellEffect`
through Crux middleware (`EffectRouter`). Only Render exists today; future Rust
I/O handlers must run below this mapping. `yomoku-ffi::YomokuBridge` wraps that
stack using Crux's Bincode bridge, exposing update/resolve/view and typed error
categories. A supplied callback preserves the path for future async effects.
The optional `facet_typegen` feature and `generate-kotlin` CLI generate event,
view, shell effect and request types plus Bincode codecs in `app.yomoku.shared`.
Generation registers the runtime shell effects explicitly; core-only effects stay
private. Generated Kotlin codecs are compiled and exercised through host JNI.
`native::NativeBridge` exposes update/resolve/view through BoltFFI 0.30.1.
Its owned `EffectsListener` has separate success/error callbacks, retaining typed
errors without passing internal strings. Hosts must dispatch callbacks to the UI
thread. Native handles own independent bridge instances; no global app singleton.
Fifteen host integration tests cover domain rules, navigation, payloads, native
facade isolation, cross-thread access and listener release. Shared/static libraries
build with exported constructor/method/callback/release symbols. The pinned
BoltFFI configuration generates Kotlin in `app.yomoku.nativebridge` and JNI C.
`tools/test-kotlin-bridge.sh` compiles that generated C against the Rust shared
library and compiles both generated Kotlin packages with a standalone compiler.
The JVM test verifies navigation, payload round trips, categorized exceptions,
instance isolation, 100 close cycles, listener registry cleanup and rejection of
calls after close, using `-Xcheck:jni`. Asynchronous callbacks remain untested
until a real asynchronous effect exists. Native wrappers require explicit close.
`android/` contains a CLI-generated Kotlin/Compose application, minimum SDK 29,
with Gradle wrapper and a JDK 17 toolchain. Its initial shell displays a Library
heading, respects system light/dark mode and window insets, and excludes automatic
Android backup/transfer. Gradle's preBuild runs `tools/build-android-native.sh`
to generate Crux/BoltFFI Kotlin and package a combined Rust/JNI library for
arm64-v8a and x86_64, using pinned NDK r30/API 29. `CoreSession` owns the native
handle, dispatches typed events, and projects Rust Render requests into Compose
state on the main thread. The Activity reads its Library heading from that view
and closes the session on destruction. Recreation currently creates a fresh core;
interactive navigation must add retention across configuration changes.
The Android instrumentation test covers command → core → view, isolation and
explicit handle close; its execution still needs a connected device/emulator.
`yomoku-storage` separately owns SQLite I/O through rusqlite 0.40.2 with bundled
SQLite. `Storage::open` configures each connection, validates numbered migration
history and applies pending migrations under one immediate transaction. Its
connection remains private; no SQL or database handle enters core or FFI. Eleven
on-disk host tests cover initialization/reopen, upgrades, rollback, concurrent
opens and failure preservation. This crate is not wired to runtime/Android yet;
the app still has only transient state and no document persistence.

## Required boundaries (not yet implemented)
Kotlin Compose shell handles UI and Android APIs through a narrow typed FFI to the
Rust application. Crux owns commands/state/domain invariants. A separate Rust
EffectRouter runs storage, analysis/dictionaries and networking effects; SQLite
SQL and platform I/O must stay outside the core. Sudachi Full defaults to Mode B;
JMdict English and JMnedict are offline data. Nostr uses Rust nostr-sdk.

Remaining planned crates: `yomoku-analysis`, `yomoku-import`. Create these when
implementing behavior, not as empty placeholders. Android project path: `android/`.

Next architectural checkpoint: Android bridge execution, responsive navigation,
then document IDs/text offsets and storage effects. Do not freeze FFI or schema
based on assumptions about dependency APIs.
