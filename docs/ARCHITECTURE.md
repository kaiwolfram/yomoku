# Architecture

## Implemented
Root Cargo workspace → `crates/yomoku-core`: pure Crux 0.20.0 application plus
surface identity, five vocabulary statuses, registration and completion policies.
`app::Yomoku` implements `App`; `Model` holds a transient destination (Library
default), `Event::SelectDestination` updates it, and a render command asks the
host to read `ViewModel`. The typed effect currently contains only Render.
Nine integration tests exercise the application through `Core` and domain rules.
No platform I/O, persisted state or Android app yet.

## Required boundaries (not yet implemented)
Kotlin Compose shell handles UI and Android APIs through a narrow typed FFI to the
Rust application. Crux owns commands/state/domain invariants. A separate Rust
EffectRouter runs storage, analysis/dictionaries and networking effects; SQLite
SQL and platform I/O must stay outside the core. Sudachi Full defaults to Mode B;
JMdict English and JMnedict are offline data. Nostr uses Rust nostr-sdk.

Planned crates: `yomoku-runtime`, `yomoku-ffi`, `yomoku-storage`,
`yomoku-analysis`, `yomoku-import`. Create these when implementing behavior,
not as empty placeholders. Android project target path: `android/`.

Next architectural checkpoint: runtime routing and typed FFI round trip,
then migration-safe storage and text offsets. Do not freeze FFI or schema based
on assumptions about dependency APIs.
