# Architecture

## Implemented
Root Cargo workspace → `crates/yomoku-core`: pure Rust surface identity,
five vocabulary statuses, registration decision and completion status policy.
No I/O, external dependencies, persisted state, Crux application or Android app yet.
Integration tests under `crates/yomoku-core/tests` verify these domain rules.

## Required boundaries (not yet implemented)
Kotlin Compose shell handles UI and Android APIs through a narrow typed FFI to the
Rust application. Crux owns commands/state/domain invariants. A separate Rust
EffectRouter runs storage, analysis/dictionaries and networking effects; SQLite
SQL and platform I/O must stay outside the core. Sudachi Full defaults to Mode B;
JMdict English and JMnedict are offline data. Nostr uses Rust nostr-sdk.

Planned crates: `yomoku-runtime`, `yomoku-ffi`, `yomoku-storage`,
`yomoku-analysis`, `yomoku-import`. Create these when implementing behavior,
not as empty placeholders. Android project target path: `android/`.

Next architectural checkpoint: actual Crux effect/bridge APIs and typed round trip,
then migration-safe storage and text offsets. Do not freeze FFI or schema based
on assumptions about dependency APIs.
