# Resume here

Updated: 2026-10-23. Sudachi Android cfg fixed; both target library builds pass.
Read AGENTS.md, plan phases 2–3 and PRODUCT_SPEC.md sections 3–4 next.
After each completed, verified step, update docs and commit it automatically.

## Working
- Pure Crux core owns routes/Settings/Library, busy/errors and typed effects.
  FIFO worker owns SQLite I/O; only Render reaches UI. Failed opens retry;
  submitted writes finish on close without blocking UI.
- Android private DB/retained session, Paste→Library and explicit reader opening
  work. Draft survives Settings/recreation/errors. Fresh processes start Library.
- Exact chapter-local UTF-8 position persists independently of page start/scroll.
  Library percentage is floor(offset * 100 / canonical UTF-8 length), can decrease.
  Five presets reflow after commit without moving offset. Reader caps at 720dp;
  transient scroll resets. NO Next/Previous/swipes until atomic real completion.
- Migration 0005 freezes completion IDs/page anchors/position atomically.
  Identical duplicates never move progress; conflicting/stale inputs error.
  Rereads need fresh IDs. No forward app/runtime/FFI hook yet.
- Migration 0006 stores exact global surfaces, nonreused IDs, five statuses and
  dated registration/manual events. Registration preserves existing status/date;
  lookup/unchanged choices add no history. Document deletion preserves vocabulary.
- Migration 0007 stores immutable versioned A/B/C token snapshots, exact source
  surfaces/UTF-8 anchors and optional readings/dictionary forms. Separate reversible
  global/local edits retain provenance; no overlap resolution/rule application/UI.
- yomoku-analysis uses Sudachi.rs v0.7.0 at its pinned commit + android-loader-v1
  source patch. Root Cargo patch selects third_party/sudachi/engine; only Android
  .so cfg and dependency-only manifests differ. Source verifier reverses the patch
  and checks all 138 original files/hashes; never alter the shared Cargo cache.
- Full 20261015 V1 stays pinned by archive/data SHA-256; complete notices retained.
  tools/prepare-sudachi.py explicitly prepares ignored data; tools/test-sudachi.sh
  verifies engine source then runs nine real-data fixtures. Host A/B/C, Unicode,
  normalization, inflections/names, long input and SQLite reopen pass. Raw Sudachi
  inflections stay split; overrides are separate. No runtime/FFI/UI integration.
- Paste text/IDs/provenance immutable; no inferred TXT chapters. Dedup and atomic
  analyzed import/registration remain. Versions/commands in DEVELOPMENT.md.

## Next tasks (ordered)
1. Bundle Full + complete, app-accessible notices; measure device startup, memory,
   tokenization and offline use before runtime integration. Android cfg blocker is
   resolved. Current safe constructor owns about 315 MiB before engine structures.
2. Compose analysis + all unique Japanese surface registration + document import
   atomically (no encounters); preserve existing statuses, test failures/rollback.
3. Extend completion with analyzed occurrences/New→Known/active time, then wire
   frozen IDs through core/runtime/FFI and Next/Previous/swipes. Finish-book and
   CI remain unchecked; do not approximate completion.

## Last verified checks
Use direct Rust PATH; exact Android env/commands/reports in DEVELOPMENT.md:
```sh
cargo build --workspace --locked --offline
cargo test --workspace --all-features --locked --offline
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked --offline -- -D warnings
python3 tools/verify-sudachi-source.py
cargo check -p yomoku-analysis --lib --target aarch64-linux-android --target x86_64-linux-android --locked --offline
cargo build -p yomoku-analysis --lib --target aarch64-linux-android --target x86_64-linux-android --locked --offline
tools/test-sudachi.sh
tools/test-kotlin-bridge.sh
tools/format-kotlin.sh --check
git diff --check
```
PASS: 119 data-free Rust tests + nine real Full fixtures; both Android target
checks/library builds; source integrity and four negative checks; Python/shell
syntax; host JNI/seven Unicode exchanges; Kotlin formatting. No outstanding
failure. No APK/instrumentation/device measurements rerun for this compile-only
slice. Previous both-ABI APKs/strict lint and seven API 36 phone tests passed;
no tablet rerun or arm64 execution. No emulator/viewport changes this session.

## Environment/blockers
- Direct Rust PATH bypasses rustup read-only maintenance (see DEVELOPMENT.md).
  Never run native generation/build/tests concurrently.
- Gradle/ADB need escalation for sockets. medium_phone API 36 uses swangle/KVM;
  tools/start-android-emulator.sh; default SwiftShader crashes.
- Device memory/performance remains unmeasured. Nostr stays empty; ask Q188 before
  forwarding. No production edit/rename/delete. No new dependency versions.
