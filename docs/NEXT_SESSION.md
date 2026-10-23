# Resume here

Updated: 2026-10-23. Durable current position/Library percentage verified.
Read AGENTS.md, plan phases 2–3 and PRODUCT_SPEC.md section 3 next.
After each completed, verified step, update docs and commit it automatically.

## Working
- Pure Crux core owns top-level/Main/Paste/Reader routes, Settings, Library,
  busy/errors and typed effects. StorageFinished is private; only Render reaches UI.
- FIFO EffectRouter worker owns lazy SQLite open/migrations/create/list/load/save.
  Failed opens retry. Submitted writes finish on close without blocking Android.
- Android owner uses filesDir/yomoku.sqlite3, loads Library once and retains the
  native session through recreation. Fresh processes start Library.
- Paste preserves draft through Settings/recreation/errors; saves return to Library
  without opening. Reader opens explicitly at the saved position (default page 500).
- Migration 0003 stores document/chapter/UTF-8 byte offset. SaveReadingPosition is
  a typed position-only hook scoped to an active reader, with no seek UI.
  Validate source/chapter/scalar boundaries; failed writes preserve prior snapshot.
  Update reader/Library only after commit; late saves cannot reopen a closed reader.
- Library shows floor(current byte offset * 100 / canonical UTF-8 byte length),
  including NUL/whitespace. Can decrease. Listing avoids fetching all book text.
  Reopen retains exact offset separately from containing page start; EOF is valid.
- Reader column centers/caps at 720dp with bottom counter. Transient ScrollState
  keyed by document/page resets on position-page changes, reopen and recreation.
  Main nav hides; toolbar/system Back/Settings work. NO Next/Previous/swipes yet.
- Storage 0001–0002 retain immutable exact text, first-line titles, provenance and
  positive nonreused i64 IDs. Dedup/registration/timestamps remain import tasks.
- Pure pagination covers all five presets, paragraph/sentence fallback, supplied
  chapters and exact-offset reflow. App Settings/reflow persistence remains next.
- Toolchain unchanged: Rust 1.99.1, Crux 0.20.0, BoltFFI 0.30.1,
  rusqlite 0.40.2/bundled, Kotlin 2.4.21, Gradle 9.1/AGP 9.0.1, JDK 17,
  BOM 2026.03.01, SDK 36/min 29, NDK r30/API 29, arm64-v8a/x86_64 builds.

## Next tasks (ordered)
1. Add atomic completion command IDs/history and the analysis/vocabulary
   repositories needed for real encounters/New→Known; then wire Next/Previous/
   swipes. Forward navigation must not use position-only saves as completion.
2. Integrate/persist page-length Settings and reflow, preserving exact saved offset.
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
PASS: 75 Rust tests (26 storage), build/fmt/clippy, real host Kotlin/JNI position
save/reopen/error/lifecycle and seven Unicode exchanges, Kotlin formatting.
Android app/test APKs, strict lint and all six API 36 x86_64 tests PASS on phone
and tablet landscape (2560x1600/240dpi). Viewport restored. Arm64 execution untested.

## Environment/blockers
- Use direct Rust PATH; rustup launcher attempts read-only maintenance.
  Local BoltFFI/Kotlin/ktfmt/JDK paths and overrides are in DEVELOPMENT.md.
- Gradle/ADB need escalation for sockets; dependencies cached. Never run native
  generation/build/tests concurrently (shared target; BoltFFI replaces exports).
- medium_phone API 36 x86_64 emulator restarted with swangle/KVM and left running.
  Startup: tools/start-android-emulator.sh. Default SwiftShader crashes. Stop:
  android emulator stop emulator-5554. UTP removes app; installDebug to launch.
- Crux intentional-close weak-reference logs and ktfmt/JDK warnings remain benign.
  Initial JNI fixture used a split scalar; corrected boundaries and rerun PASS.
- No local-reader blocker. Nostr configuration stays empty; ask Q188 before
  forwarding. No production edit/rename/delete, repair, completion or history yet.
