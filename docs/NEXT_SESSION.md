# Resume here

Updated: 2026-10-23. Real Sudachi/Full host adapter verified; Android cfg blocker.
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
  surfaces/UTF-8 anchors and optional analyzer readings/dictionary forms. Separate
  reversible global/local boundary edits retain provenance; base output stays
  immutable. No overlap resolution, cross-occurrence rule application or edit UI.
- New yomoku-analysis uses Sudachi.rs v0.7.0 pinned by commit, embedded defaults,
  Full 20261015 V1 pinned by archive/data SHA-256. Host A/B/C, normalization,
  inflections/names, rare kanji, mixed scripts, long input and SQLite reopen pass.
  tools/prepare-sudachi.py explicitly prepares ignored data + complete notices;
  tools/test-sudachi.sh runs nine real-data fixtures (ignored by ordinary tests).
  See third_party/sudachi/README.md and DECISIONS.md. No runtime/FFI/UI wiring.
- Paste text/IDs/provenance immutable; no inferred TXT chapters. Dedup and atomic
  analyzed import/registration remain. Toolchain/Android versions in DEVELOPMENT.md.

## Next tasks (ordered)
1. Resolve upstream Sudachi plugin/loader.rs Android cfg (E0425 undefined
   make_system_specific_name; Linux/FreeBSD .so definition omits Android), using
   pinned upstream fix or documented source patch. Then bundle Full + notices,
   measure device startup/memory/tokenization/offline use. Current safe constructor
   owns about 315 MiB dictionary bytes before engine structures; review on device.
2. Compose analysis + all unique Japanese surface registration + document import
   atomically (no encounters); preserve existing statuses, test failures/rollback.
   Raw Sudachi inflections currently stay split; override application is separate.
3. Extend completion with analyzed occurrences/New→Known/active time, then wire
   frozen IDs through core/runtime/FFI and Next/Previous/swipes. Finish-book and
   CI remain unchecked in IMPLEMENTATION_PLAN.md; do not approximate completion.

## Last verified checks
Root (Android env/commands/reports in DEVELOPMENT.md):
```sh
export PATH=/home/kai/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin:$PATH
cargo build --workspace --locked --offline
cargo test --workspace --all-features --locked --offline
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked --offline -- -D warnings
tools/test-sudachi.sh
tools/test-kotlin-bridge.sh
tools/format-kotlin.sh --check
git diff --check
```
PASS: 119 data-free Rust tests + nine explicit real Full tests; host JNI and seven
Unicode exchanges. Both-ABI app/test APKs and strict lint pass. Python/shell syntax,
offline/repeated data preparation and corrupt-archive preservation pass.
Android analyzer target checks FAIL as above. No instrumentation/device measurement
rerun for this host-only slice. Previous seven API 36 phone tests passed; no tablet
rerun or arm64 execution. No emulator/viewport changes this session.
Initial fixture run fixed two expected outputs/input length and OOV classification;
initial compile fixed sha2-0.11 hex formatting and borrowed-surface type inference.

## Environment/blockers
- Direct Rust PATH avoids rustup read-only maintenance. Local tool overrides in
  DEVELOPMENT.md. Never run native generation/build/tests concurrently.
- Gradle/ADB need escalation for sockets. medium_phone API 36 uses swangle/KVM;
  startup tools/start-android-emulator.sh; default SwiftShader crashes.
- Analyzer Android cfg blocks its integration, not existing local-reader builds.
  Nostr stays empty; ask Q188 before forwarding. No production edit/rename/delete.
