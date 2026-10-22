# Resume here

Updated: 2026-10-22. Current phase: 1, Rust foundation; one pure phase-3 rule slice
was pulled forward. Read AGENTS.md, plan phase 1 and handoff section 3 next.

## Working
- Clean starting tree; existing vocabulary implementation preserved.
- Pure `yomoku-core` now implements Crux 0.20.0 App with typed navigation event,
  model, view and render effect; Library default. Nine host integration tests.
- Exact surface identities, five statuses, non-overwriting registration policy,
  New-only completion transition remain tested.
- No Android app, persistence, tokenizer, runtime or FFI yet.

## Next tasks (ordered)
1. Add runtime EffectRouter and narrow FFI typed round-trip/error test. Inspect
   Crux 0.20.0 published bridge/middleware APIs; keep Rust I/O effects in Rust.
2. Verify/pin BoltFFI and Crux Kotlin type generation (current upstream Android
   approach); test generated types/serialization rather than hand-writing codecs.
3. Generate Android Compose project with installed `android` CLI, min SDK 29;
   inspect template versions, install compatible SDK/JDK/NDK and build wrapper.

## Last successful checks
Run from repository root:
```sh
export PATH=/home/kai/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin:$PATH
cargo build --workspace --offline
cargo test --workspace --offline
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --offline -- -D warnings
```
All PASS; nine integration tests. Compiler/cargo 1.99.1.
Initial fmt check flagged one assertion; formatted and reran successfully.
Fresh caches require `cargo fetch --locked` with network access first.

## Environment/blockers
- Plain cargo/rustc via rustup fails trying to remove read-only rustup-init;
  direct toolchain PATH above works without modifying the user's installation.
- Sandbox crates.io DNS failed; escalated cargo info/fetch succeeded. Crux and
  locked dependencies now cached. Offline initial build lacked bincode; resolved.
- Android CLI 1.0.16625370, SDK `/home/kai/Android/Sdk`, build/templates 0.1.1;
  empty-activity supports Compose/AGP 9. CLI requires approved escalation.
- No Gradle build yet; JDK 25 available, compatible build JDK to verify.
- No curated Nostr starter data; keep empty configuration. Ask Q188 only when
  implementing Citrine forwarding. Neither blocks local work.

See DEVELOPMENT.md for environment detail; DECISIONS.md for API evidence.
