# Resume here

Updated: 2026-10-22. Current phase: 1, Rust foundation; one pure phase-3 rule slice
was pulled forward. Read AGENTS.md, plan phase 1 and handoff section 3 next.

## Working
- Clean starting tree; existing vocabulary implementation preserved.
- Pure `yomoku-core` now implements Crux 0.20.0 App with typed navigation event,
  model, view and render effect; Library default. Thirteen host integration tests across core and bridge.
- Exact surface identities, five statuses, non-overwriting registration policy,
  New-only completion transition remain tested.
- Runtime EffectRouter maps Render to shell effects; Rust FFI facade uses Crux
  Bincode update/resolve/view with typed errors and independent instances.
- No native exports, Kotlin bindings, Android app, persistence or tokenizer yet.

## Next tasks (ordered)
1. Verify/pin BoltFFI and Crux Kotlin generation; add native exports around
   YomokuBridge and test generated event/request/view/error round trips.
2. Generate Android Compose project with installed `android` CLI, min SDK 29;
   inspect template versions, install compatible SDK/JDK/NDK and build wrapper.
3. Package native emulator/arm64 libraries and test command → view on Android.

## Last successful checks
Run from repository root:
```sh
export PATH=/home/kai/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin:$PATH
cargo build --workspace --offline
cargo test --workspace --offline
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --offline -- -D warnings
cargo check -p yomoku-runtime --features typegen --offline
```
All PASS; thirteen integration tests. Compiler/cargo 1.99.1.
Initial bridge compile needed effect(typegen); fixed. Optional generator deps
were absent offline; approved cargo fetch resolved this.
Fresh caches require `cargo fetch --locked` with network access first.

## Environment/blockers
- Plain cargo/rustc via rustup fails trying to remove read-only rustup-init;
  direct toolchain PATH above works without modifying the user's installation.
- Crux and locked dependencies, including optional Serde generator, now cached.
  Browser docs fetch failed; verified published local crate source instead.
- Android CLI 1.0.16625370, SDK `/home/kai/Android/Sdk`, build/templates 0.1.1;
  empty-activity supports Compose/AGP 9. CLI requires approved escalation.
- No Gradle build yet; JDK 25 available, compatible build JDK to verify.
- No curated Nostr starter data; keep empty configuration. Ask Q188 only when
  implementing Citrine forwarding. Neither blocks local work.

See DEVELOPMENT.md for environment detail; DECISIONS.md for API evidence.
