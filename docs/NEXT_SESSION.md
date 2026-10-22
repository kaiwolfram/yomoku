# Resume here

Updated: 2026-10-22. Runtime storage effects and Android persistence verified.
Read AGENTS.md, plan phase 2 and PRODUCT_SPEC.md section 3 next.
After each completed, verified step, update docs and commit it automatically.

## Working
- Pure Crux 0.20.0 core owns navigation/Settings, document snapshots, busy/error
  state and typed create/load/list effects. Internal StorageFinished is skipped
  by Serde/Facet. Only Render reaches the generated Kotlin shell protocol.
- EffectRouter consumes storage effects on one FIFO std thread per bridge. Lazy
  SQLite open/migrations and all I/O stay off UI; failed opens can retry safely.
- NativeBridge(databasePath, listener) has typed errors and real async JNI renders.
  Kotlin posts callbacks to main and ignores them after close. Submitted I/O
  finishes on close; worker exits without joining UI. Listener cleanup verified.
- Android CoreSessionOwner uses Application.filesDir/yomoku.sqlite3, requests
  Library on session construction, retains across recreation and closes onCleared.
  Fresh process starts Library. Four placeholders/Settings and phone/tablet nav
  remain; no paste UI, Library list rendering or active reader route yet.
- Four device tests PASS on API 36 x86_64: JNI/isolation/close, navigation/Back,
  recreation/cleanup, private-file paste/list/load/reopen and open failure/retry.
- Storage 0001–0002: immutable exact text, first-line titles, separate provenance,
  positive i64 IDs/nonreuse/exhaustion; 21 storage tests pass. Runtime create is
  repository plumbing: dedup/registration/timestamps/progress remain future tasks.
- Typed UTF-8 chapter-local anchors/checked UTF-16; seven Rust/Kotlin exchanges.
- Pinned toolchain unchanged: Rust 1.99.1, Crux 0.20.0, BoltFFI 0.30.1,
  rusqlite 0.40.2/bundled, Kotlin 2.4.21, Gradle 9.1/AGP 9.0.1, JDK 17,
  BOM 2026.03.01, SDK 36/min 29, NDK r30/API 29, arm64-v8a/x86_64 builds.

## Next tasks (ordered)
1. Implement pure logical pages: presets 250/500/750/1000/1500 (default 500),
   paragraph/sentence fallback, explicit chapter boundaries, no inferred TXT
   chapters, reflow preserving exact text offset; Japanese/supplementary tests.
2. Add Android paste flow returning Library, list/reader opening and navigation
   hiding, then page controls/position persistence. Do not claim full import
   semantics until planned dedup/vocabulary registration are added.
3. Add CI for verified host Rust/Kotlin checks and unsigned Android build.

## Last verified checks
Repository root (exact Android env/commands/reports in DEVELOPMENT.md):
```sh
export PATH=/home/kai/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin:$PATH
cargo build --workspace --locked --offline
cargo test --workspace --all-features --locked --offline
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked --offline -- -D warnings
tools/test-kotlin-bridge.sh
tools/format-kotlin.sh --check
git diff --check
```
PASS: 49 Rust tests, host JNI/Unicode, fmt/clippy, Kotlin formatting,
Android app/test APK assembly, strict lint and 4 emulator tests offline.
SQLite-lock fixture proves dispatch/view/navigation do not wait on database I/O.
Kotlin enum naming and Clippy test type complexity corrected; no failing check.

## Environment/blockers
- Use direct Rust PATH; rustup launcher attempts read-only maintenance.
  Local BoltFFI/Kotlin/ktfmt/JDK paths and overrides are in DEVELOPMENT.md.
- Gradle/ADB need escalation for sockets; dependencies cached. Never run native
  generation/build/tests concurrently (shared target; BoltFFI replaces host exports).
- Emulator medium_phone API 36 x86_64 currently connected. Startup fallback:
  tools/start-android-emulator.sh (swangle/KVM). Default SwiftShader crashes.
  Stop: android emulator stop emulator-5554. Arm64 execution remains untested.
- Crux logs "Inner can't be upgraded after resolving effect" for discarded work
  after intentional close. JNI/registry cleanup passes; SDK/ktfmt warnings benign.
- Wait for Compose idle before Back injection. UTP removes app after tests;
  installDebug before manual launch. No new viewport changes this session.
- Nostr configuration stays empty; ask Q188 only before implementing forwarding.
  No local-reader blocker. No production edit/rename/delete or destructive repair.
