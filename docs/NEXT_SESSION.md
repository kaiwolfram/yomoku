# Resume here

Updated: 2026-10-23. Global vocabulary repositories/history foundation verified.
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
- Migration 0003 stores document/chapter/byte offset. Position-only saves validate
  ownership/scalar/EOF, commit before rendering, and cannot reopen closed readers.
  Library percentage is floor(byte offset * 100 / canonical UTF-8 length), including
  NUL/whitespace; can decrease. Exact offset is separate from page start.
- Migration 0004 stores global presets 250/500/750/1000/1500, default 500.
  Settings reflow applies after commit, preserving exact offset/percentage.
  Unknown/busy selections disabled; missing/corrupt settings error without repair.
- Reader column centers/caps at 720dp with bottom counter. Transient scroll resets
  on page changes/reopen/recreation/return from Settings. Main nav hides.
  NO Next/Previous/swipes yet; forward navigation needs atomic completion.
- Migration 0005 issues completion IDs and atomically records dated page anchors/
  position. Duplicate payloads never move progress; conflicts/stale inputs error.
  Rereads use a fresh ID. Repository-only: encounters/New→Known/time must join
  the transaction before adding a core/runtime/FFI forward hook or controls.
- Migration 0006 stores exact global surfaces, nonreused VocabularyIds, five statuses
  and dated version-1 registration/manual events. Batch registration preserves
  existing status/date; lookup and unchanged selections add no history. Status/event
  writes roll back together. Concurrent registration deduplicates. Document deletion
  preserves vocabulary; test-only vocabulary deletes cascade its status history.
  Repository-only: no analyzer/import/completion/application wiring or delete API.
- Pure pagination covers all presets, paragraph/sentence fallback, explicit chapter
  boundaries and exact-offset reflow. No inferred TXT chapters.
- Storage 0001–0002 retain exact immutable text, first-line titles, provenance and
  positive nonreused i64 IDs. Dedup/atomic import registration/timestamps remain.
- Toolchain unchanged: Rust 1.99.1, Crux 0.20.0, BoltFFI 0.30.1, rusqlite 0.40.2,
  Kotlin 2.4.21, Gradle 9.1/AGP 9.0.1, JDK 17, BOM 2026.03.01, SDK 36/min 29,
  NDK r30/API 29; arm64-v8a and x86_64 build.

## Next tasks (ordered)
1. Verify Sudachi.rs/Full dictionary API/license; add analysis/surface/source anchors
   and reversible override schema. Compose registration with document import in
   one transaction; standalone register_vocabulary is only a repository prerequisite.
2. Extend completion with analyzed occurrences/New→Known events/time, then wire
   frozen IDs through core/runtime/FFI and Next/Previous/swipes. Never use manual
   status or position-only saves as forward completion. Finish-book work remains.
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
PASS: 102 Rust tests (51 storage), including 11 new vocabulary repository tests,
all 25 app reflow pairs, host JNI and seven Unicode exchanges.
PASS: Android app/test builds, strict lint and all seven API 36 x86_64 phone tests.
No tablet rerun; arm64 execution untested. Emulator left running; viewport unchanged.
Initial migration test used a nonexistent pagination API; fixed to LogicalPages.

## Environment/blockers
- Direct Rust PATH avoids rustup read-only maintenance. Local tool overrides in
  DEVELOPMENT.md. Never run native generation/build/tests concurrently.
- Gradle/ADB need escalation for sockets; dependencies cached. medium_phone API 36
  uses swangle/KVM; startup: tools/start-android-emulator.sh. Default SwiftShader
  crashes. UTP removes app; installDebug to launch. Stop: android emulator stop emulator-5554.
- Intentional-close Crux logs and ktfmt/JDK/SDK warnings remain benign.
- No local-reader blocker. Nostr config stays empty; ask Q188 before forwarding.
  No production edit/rename/delete, repair or exposed completion/history yet.
