# Resume here

Updated: 2026-10-22. Current phase: 1, Rust foundation; one pure phase-3 rule slice
was pulled forward. Read AGENTS.md, plan phase 1 and handoff sections 3/4.2 next.

## Working
- Initial repository contained only untracked handoff; preserved unchanged.
- Cargo workspace with dependency-free `yomoku-core`.
- Exact surface identities, five statuses, non-overwriting registration policy,
  New-only completion transition; five passing integration tests.
- Continuity/architecture/data model/decision docs exist.
- No Android app, Crux app, persistence, tokenizer, FFI or UI yet.

## Next tasks (ordered)
1. Verify current Crux APIs and Kotlin bridge options; record selected versions
   and evidence, then add typed event/model/view and host command/effect test.
2. Add runtime EffectRouter and FFI typed round-trip/error test using that API.
3. Generate Android Compose project with installed `android` CLI, min SDK 29;
   inspect template versions, install compatible SDK/JDK/NDK and build with wrapper.

## Last successful checks
Run from repository root:
```sh
export PATH=/home/kai/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin:$PATH
cargo build --workspace --offline
cargo test --workspace --offline
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --offline -- -D warnings
```
All PASS; five integration tests. Compiler/cargo 1.99.1.
`android --help`, `android info`, `android create --help`,
`android create --list`, `android sdk list` also PASS with sandbox escalation.

## Environment/blockers
- Plain cargo/rustc via rustup fails trying to remove read-only rustup-init;
  direct toolchain PATH above works without modifying the user's installation.
- Android CLI IS installed (1.0.16625370), SDK `/home/kai/Android/Sdk`.
  Only build/templates 0.1.1 installed; empty-activity supports Compose/AGP 9.
- Android CLI needs its home lock writable; `android` escalation prefix approved.
- No Gradle project/build yet; JDK 25 available, compatible build JDK to verify.
- No curated Nostr starter data; keep empty configuration. Ask Q188 only when
  implementing Citrine forwarding. Neither blocks local work.

See DEVELOPMENT.md for environment detail; DECISIONS.md for scope decisions.
