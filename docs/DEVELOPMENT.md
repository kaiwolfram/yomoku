# CLI development

Run from the repository root. Dependencies are pinned by Cargo.lock; on a fresh
workstation run `cargo fetch --locked` with network access before offline checks.
The agent sandbox required escalation for downloads into the Cargo cache.

```sh
# Current sandbox: bypass rustup launcher self-maintenance in a read-only home.
export PATH=/home/kai/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin:$PATH
cargo build --workspace --offline
cargo test --workspace --offline
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --offline -- -D warnings
```

Verified 2026-10-22 using rustc 1.99.1 and cargo 1.99.1. A normal workstation
with functioning rustup can omit the PATH workaround. Do not assume this compiler
is the project's minimum supported version; establish that with dependencies.
Cargo.lock is retained for reproducible application development.
Crux 0.20.0 declares Rust 1.90 minimum; the full locked graph has only been tested
with 1.99.1. Current host baseline: thirteen integration tests (four app, five domain, four bridge).
Also verified `cargo check -p yomoku-runtime --features typegen --offline`.
This checks generator feature compilation, not generated Kotlin or native exports.

## Android environment and next setup
`android` is installed at `/home/kai/.local/bin/android`. It needs write access to
`~/.android/cli/bundles/lock`; in the agent sandbox this requires escalation.
The user approved the `android` command prefix. Verified commands:

```sh
android --help
android info
android create --help
android create --list
android sdk list
```

Reported CLI version: 1.0.16625370, launcher 1.0.16500706.
SDK location: `/home/kai/Android/Sdk`. Installed packages: only
`build/templates` 0.1.1. Available template: `empty-activity` (Compose, AGP 9).
`adb`, `sdkmanager`, `emulator` and `gradle` were not on PATH during inspection.
Java reports OpenJDK 25.0.4.1; the Java 21 directory contains no observed JDK binary.
Android builds have NOT been run; no Gradle project exists yet.

Next setup can use `android create` into `android/` with `--name=Yomoku` and
`--min-sdk=29`, after selecting a provisional namespace and inspecting template
versions. Use `android sdk --help` for package installation. Validate JDK/Gradle/
AGP/NDK compatibility against primary documentation, then record exact Gradle,
native packaging, emulator and adb commands here after running them. Do not
present guessed future build commands as a successful baseline.
