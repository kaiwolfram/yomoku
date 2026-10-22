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
private. Generated Kotlin has not yet been compiled or executed.
`native::NativeBridge` exposes update/resolve/view through BoltFFI 0.30.1.
Its owned `EffectsListener` has separate success/error callbacks, retaining typed
errors without passing internal strings. Hosts must dispatch callbacks to the UI
thread. Native handles own independent bridge instances; no global app singleton.
Fifteen host integration tests cover domain rules, navigation, payloads, native
facade isolation, cross-thread access and listener release. Shared/static libraries
build with exported constructor/method/callback/release symbols. Generated JNI
bindings and native handle lifecycle have not yet been executed from Kotlin.
No platform I/O, persisted state or Android app yet.

## Required boundaries (not yet implemented)
Kotlin Compose shell handles UI and Android APIs through a narrow typed FFI to the
Rust application. Crux owns commands/state/domain invariants. A separate Rust
EffectRouter runs storage, analysis/dictionaries and networking effects; SQLite
SQL and platform I/O must stay outside the core. Sudachi Full defaults to Mode B;
JMdict English and JMnedict are offline data. Nostr uses Rust nostr-sdk.

Remaining planned crates: `yomoku-storage`,
`yomoku-analysis`, `yomoku-import`. Create these when implementing behavior,
not as empty placeholders. Android project target path: `android/`.

Next architectural checkpoint: generated Kotlin/native FFI round trip,
then migration-safe storage and text offsets. Do not freeze FFI or schema based
on assumptions about dependency APIs.
