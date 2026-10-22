# Resume here

Updated: 2026-10-22. Current phase: 1, Android project setup next.
Read AGENTS.md, plan phase 1 and handoff section 3 next.
After each completed, verified step, update progress docs and commit it
automatically; report the commit hash. No separate user request is needed.

## Working
- Pure Crux 0.20.0 app, Library default and four destinations; vocabulary rules.
- Runtime EffectRouter and per-instance FFI facade; 15 Rust integration tests.
- Reproducible Crux Kotlin event/request/view codecs (Facet 0.46.5,
  facet_generate 0.19.1 locked) and BoltFFI 0.30.1 Kotlin/JNI bindings.
- Real host JNI tests pass: navigation/codecs, typed errors, independent state,
  100 handle lifecycles, listener release and calls after close; JVM JNI checks on.
- No Android app, persistence or tokenizer yet. Async callbacks need a real effect.

## Next tasks (ordered)
1. Generate Android Compose project with installed `android` CLI, min SDK 29;
   inspect template versions, install compatible SDK/JDK/NDK and build wrapper.
2. Package native emulator/arm64 libraries and test command → view on Android.
3. Implement responsive navigation with the four destinations and toolbar Settings.
Test async JNI callbacks when the first real asynchronous effect is added.

## Last successful checks
Run from repository root:
```sh
export PATH=/home/kai/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin:$PATH
cargo build --workspace --locked --offline
cargo test --workspace --all-features --locked --offline
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked --offline -- -D warnings
tools/test-kotlin-bridge.sh
bash -n tools/test-kotlin-bridge.sh
git diff --check
```
All PASS; 15 Rust integration tests plus Kotlin/JNI suite. Compiler/cargo 1.99.1.
Crux and BoltFFI output each generated twice and diffed identically.
Fresh caches require `cargo fetch --locked` with network access first.
See DEVELOPMENT.md for exact tool setup and repeat-generation commands.

## Environment/blockers
- Plain cargo/rustc via rustup fails trying to remove read-only rustup-init;
  direct toolchain PATH above works without changing the user's installation.
- BoltFFI CLI 0.30.1: `.local/bin/boltffi`; Kotlin 2.4.21:
  `.local/kotlin-2.4.21/kotlinc/bin/kotlinc`; OpenJDK runtime 25.0.4.1.
  JNI headers downloaded from official OpenJDK jdk-25+36 into `.local/jni-25/`.
  No full JDK development package, Gradle or Kotlin lint/formatter installed.
- BoltFFI generation replaces host library exports during metadata expansion.
  Runner rebuilds Rust AFTER generation. Avoid concurrent generation/builds
  sharing target/. Generated output/tool downloads are ignored.
- Android CLI 1.0.16625370, SDK `/home/kai/Android/Sdk`, build/templates 0.1.1;
  empty-activity supports Compose/AGP 9. CLI requires approved escalation.
- No Gradle build yet. Validate actual template, JDK/Gradle/AGP/NDK compatibility
  before adopting versions. Host JVM success does not establish Android support.
- No curated Nostr starter data; keep configuration empty. Ask Q188 only when
  implementing Citrine forwarding. Neither blocks local work.

See DEVELOPMENT.md and DECISIONS.md for API evidence and known resolved failures.
