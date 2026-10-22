# Resume here

Updated: 2026-10-22. Current phase: 1, native bridge and Kotlin integration.
Read AGENTS.md, plan phase 1 and handoff section 3 next.
After each completed, verified step, update progress docs and commit it
automatically; report the commit hash. No separate user request is needed.

## Working
- Clean starting tree; existing app/domain/bridge behavior preserved.
- Pure Crux 0.20.0 app, Library default and four destinations; vocabulary rules.
- Runtime EffectRouter and per-instance serialized FFI facade; 15 host tests.
- `generate-kotlin` emits shell event/request/view types and Bincode codecs.
  Facet 0.46.5, facet_generate 0.19.1 locked; repeated output is identical.
- BoltFFI 0.30.1 NativeBridge exports, typed errors and owned callback listener;
  host static/shared libraries build and exported symbols verified.
- No compiled Kotlin, Android app, persistence or tokenizer yet.

## Next tasks (ordered)
1. Install BoltFFI CLI 0.30.1 and Kotlin tooling; generate native bindings and
   test payloads/errors/handle lifecycle across real JNI. Exercise callbacks
   when the first asynchronous effect exists (navigation is synchronous).
2. Generate Android Compose project with installed `android` CLI, min SDK 29;
   inspect template versions, install compatible SDK/JDK/NDK and build wrapper.
3. Package native emulator/arm64 libraries and test command → view on Android.

## Last successful checks
Run from repository root:
```sh
export PATH=/home/kai/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin:$PATH
cargo build --workspace --offline
cargo test --workspace --all-features --offline
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --offline -- -D warnings
cargo run -p yomoku-ffi --bin generate-kotlin --features facet_typegen --offline -- target/generated/kotlin
cargo run -p yomoku-ffi --bin generate-kotlin --features facet_typegen --offline -- target/generated/kotlin-repeat
diff -r target/generated/kotlin target/generated/kotlin-repeat
```
All PASS; 15 integration tests. Compiler/cargo 1.99.1.
Fresh caches require `cargo fetch --locked` with network access first.
Use dedicated generated folders: upstream replaces its package directory.

## Environment/blockers
- Plain cargo/rustc via rustup fails trying to remove read-only rustup-init;
  direct toolchain PATH above works without changing the user's installation.
- Crux Serde `typegen` cannot generate Kotlin; replaced with `facet_typegen`.
  Initially missing optional deps fetched successfully with approved cargo fetch.
- No kotlinc, Gradle or BoltFFI CLI on PATH. Kotlin/native execution remains unverified.
  Do not adopt the generated Gradle template's versions without verification.
- Android CLI 1.0.16625370, SDK `/home/kai/Android/Sdk`, build/templates 0.1.1;
  empty-activity supports Compose/AGP 9. CLI requires approved escalation.
- No Gradle build yet; JDK 25 available, compatible build JDK to verify.
- No curated Nostr starter data; keep configuration empty. Ask Q188 only when
  implementing Citrine forwarding. Neither blocks local work.

See DEVELOPMENT.md for commands and DECISIONS.md for generator API evidence.
