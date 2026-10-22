# Resume here

Updated: 2026-10-22. Android paste/Library/first-page reader opening verified.
Read AGENTS.md, plan phase 2 and PRODUCT_SPEC.md section 3 next.
After each completed, verified step, update docs and commit it automatically.

## Working
- Pure Crux 0.20.0 core owns top-level navigation, Main/Paste/Reader routes,
  Settings overlay, Library summaries, busy/errors and typed storage effects.
  StorageFinished is skipped by Serde/Facet; only Render reaches Kotlin.
- EffectRouter executes lazy SQLite open/migrations/create/list/load on one FIFO
  worker per bridge. Failed opens retry; submitted writes finish on close without
  blocking UI. NativeBridge has typed errors and async JNI render callbacks.
- Android CoreSessionOwner uses filesDir/yomoku.sqlite3, loads Library once,
  retains across recreation and closes onCleared. Fresh sessions start Library.
- Paste form retains draft through Settings/recreation/error, saves to Library
  without auto-opening. Library shows exact titles and explicitly opens reader.
  Rust ReaderPage projects exact default-500 first page, anchor, separate reading
  offset and counter. Late loads cannot reopen a cancelled reader route.
- Reader column centers/caps at 720dp and scrolls; content-bottom counter present.
  Toolbar/system Back and Settings return work; main navigation hides in reader.
  Reader currently starts at page 1; controls/swipes/progress/completion pending.
- Storage 0001–0002: immutable exact text, first-line titles, separate provenance,
  positive i64 IDs/nonreuse/exhaustion. Dedup/registration/timestamps remain next
  import tasks; paste creation adds no reading encounters.
- Typed chapter-local UTF-8 anchors/checked UTF-16; seven Rust/Kotlin exchanges.
  Pure pagination covers all five presets, paragraph/sentence fallback, supplied
  chapters and exact-offset reflow (14 tests). All non-whitespace Unicode scalars
  count, including punctuation/Latin. App Settings/reflow persistence remain next.
- Pinned toolchain unchanged: Rust 1.99.1, Crux 0.20.0, BoltFFI 0.30.1,
  rusqlite 0.40.2/bundled, Kotlin 2.4.21, Gradle 9.1/AGP 9.0.1, JDK 17,
  BOM 2026.03.01, SDK 36/min 29, NDK r30/API 29, arm64-v8a/x86_64 builds.

## Next tasks (ordered)
1. Add Rust-owned page controls/swipes and persist current chapter/byte offset in
   both directions; expose Library percentage. Keep exact offset separate from
   derived page start through reflow. Reset vertical scroll on navigation/reopen.
2. Add atomic completion command IDs/history and New→Known encounters after
   analysis/vocabulary repositories are ready; retain reread/Previous semantics.
   Settings page presets still need app/persistence integration.
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
PASS: 67 Rust tests, build/fmt/clippy, real host Kotlin/JNI reader/storage/lifecycle
and seven Unicode exchanges, Kotlin formatting/whitespace. Android app/test APKs,
strict lint and all five API 36 x86_64 tests PASS on phone and tablet landscape
(2560x1600/240dpi). Emulator viewport restored. Arm64 execution untested.

## Environment/blockers
- Use direct Rust PATH; rustup launcher attempts read-only maintenance.
  Local BoltFFI/Kotlin/ktfmt/JDK paths and overrides are in DEVELOPMENT.md.
- Gradle/ADB need escalation for sockets; dependencies cached. Never run native
  generation/build/tests concurrently (shared target; BoltFFI replaces exports).
- medium_phone API 36 x86_64 emulator running via swangle/KVM. Startup fallback:
  tools/start-android-emulator.sh. Default SwiftShader crashes. Stop with:
  android emulator stop emulator-5554. UTP removes app; installDebug to launch.
- Crux logs discarded weak-reference effects after intentional close; JNI/registry
  cleanup passes. SDK/ktfmt warnings benign. Wait for Compose idle before Back.
- Initial new-test constructor/offset API mistakes and stale private-event ordinal
  corrected; all reruns pass. No dependency/migration changes or local blocker.
- Nostr configuration stays empty; ask Q188 only before implementing forwarding.
  No production edit/rename/delete or destructive repair.
