# Resume here

Updated: 2026-10-23. Repository completion IDs/history foundation verified.
Read AGENTS.md, plan phases 2–3 and PRODUCT_SPEC.md sections 3–4 next.
After each completed, verified step, update docs and commit it automatically.

## Working
- Pure Crux core owns Main/Paste/Reader routes, Settings, Library, busy/errors
  and typed effects. StorageFinished is private; only Render reaches UI.
- FIFO worker owns lazy SQLite open/migrations/create/list/load/position/settings
  saves. Failed opens retry. Submitted writes finish on close without blocking UI.
- Android uses filesDir/yomoku.sqlite3; retained native session survives recreation.
  Fresh processes start Library and load documents/global page length once.
- Paste keeps draft through Settings/recreation/errors; saves return to Library,
  never open it. Reader opens explicitly at the exact saved UTF-8 offset.
- Migration 0003 stores document/chapter/byte offset. SaveReadingPosition is a
  position-only active-reader hook; no seek UI. Validate ownership/scalar/EOF.
  Commit drives reader/Library updates; failures retain prior state. Late saves
  cannot reopen a closed reader. Listing avoids loading all book text.
- Library percentage is floor(current byte offset * 100 / canonical UTF-8 length),
  including NUL/whitespace; can decrease. Exact offset is separate from page start.
- Migration 0004 stores global presets 250/500/750/1000/1500, default 500.
  Settings radio selection/reflow applies only after commit; preserves exact
  offset and percentage. Missing/corrupt settings error without repair. Unknown/
  busy selections disabled; loading failures can retry. Fresh sessions retain it.
- Reader column centers/caps at 720dp with bottom counter. Transient scroll resets
  on page changes/reopen/recreation/return from Settings. Main nav hides.
  NO Next/Previous/swipes yet; forward navigation needs atomic completion.
- Migration 0005 issues completion IDs and atomically records dated page anchors/
  current position. Duplicate payloads never move progress; conflicts/stale inputs
  error. Rereads use a fresh ID. Repository-only: no app/runtime/FFI forward hook;
  encounters/New→Known/time must join this transaction before exposing controls.
- Pure pagination covers all presets, paragraph/sentence fallback, explicit chapter
  boundaries and exact-offset reflow. No inferred TXT chapters.
- Storage 0001–0002 retain exact immutable text, first-line titles, provenance and
  positive nonreused i64 IDs. Dedup/registration/timestamps remain import tasks.
- Toolchain unchanged: Rust 1.99.1, Crux 0.20.0, BoltFFI 0.30.1, rusqlite 0.40.2,
  Kotlin 2.4.21, Gradle 9.1/AGP 9.0.1, JDK 17, BOM 2026.03.01, SDK 36/min 29,
  NDK r30/API 29; arm64-v8a and x86_64 build.

## Next tasks (ordered)
1. Verify Sudachi.rs/Full dictionary API/license; add analysis/surface anchors,
   vocabulary registration and atomic encounter/New→Known prerequisites.
2. Extend the completion transaction with occurrences/status/time; wire frozen
   IDs through core/runtime/FFI and Next/Previous/swipes. Never use position-only
   saves as forward completion. Active timer/finished-book tasks stay unchecked.
3. Add CI for verified host Rust/Kotlin checks and unsigned Android build.

## Last verified checks
Root (exact Android env/commands/reports in DEVELOPMENT.md):
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
PASS: 91 Rust tests (40 storage), including ten completion repository tests,
all 25 app reflow pairs, host JNI and seven Unicode exchanges. Android app/test
APKs, strict lint and all seven API 36 x86_64 phone tests PASS. Viewport unchanged;
tablet tests were not rerun for this repository-only slice.
Arm64 execution untested. Initial test-vector clippy finding fixed; rerun PASS.

## Environment/blockers
- Direct Rust PATH avoids rustup read-only maintenance. Local tool overrides in
  DEVELOPMENT.md. Never run native generation/build/tests concurrently.
- Gradle/ADB need escalation for sockets; dependencies cached. medium_phone API 36
  was restarted with swangle/KVM after server restart and left running. Startup:
  tools/start-android-emulator.sh. Default SwiftShader crashes. UTP removes app;
  installDebug to launch. Stop: android emulator stop emulator-5554.
- Intentional-close Crux logs and ktfmt/JDK/SDK warnings remain benign.
- No local-reader blocker. Nostr config stays empty; ask Q188 before forwarding.
  No production edit/rename/delete, repair or exposed completion/history yet.
