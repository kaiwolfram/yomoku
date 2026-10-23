# Resume here

Updated: 2026-10-23. Analysis anchors/reversible override schema verified.
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
  Settings presets 250/500/750/1000/1500 reflow after commit without moving offset.
- Reader centers/caps at 720dp with content-bottom counter; transient scroll resets.
  NO Next/Previous/swipes yet: real encounters/New→Known/time must be atomic first.
- Migration 0005 issues frozen completion IDs and atomically records page anchors/
  position. Identical duplicates never move progress; conflicting/stale inputs error.
  Rereads need fresh IDs. Repository-only; no forward app/runtime/FFI hook yet.
- Migration 0006 stores exact global surfaces, nonreused IDs, five statuses and
  dated version-1 registration/manual events. Registration preserves existing
  status/date; lookup/unchanged choices add no history. Document deletion preserves
  vocabulary. No analyzer/import/completion/UI wiring or production delete API.
- Migration 0007 stores immutable chapter analysis snapshots keyed by exact engine/
  dictionary revisions and A/B/C mode (default B), original token surfaces, UTF-8
  anchors, Japanese flags and optional analyzer reading/dictionary form.
  Validation rejects ownership/scalar/coverage errors and normalized surfaces.
- Separate global/local overrides retain selected bytes/provenance and ordered
  split/merge parts. Identical saves reuse IDs; replacement/reversal are atomic.
  Reversal disables only that ID, keeps superseded edits inactive and base output
  unchanged. Global rules survive originating-content deletion. Synthetic fixtures
  ONLY; no real tokenizer, overlap resolution, cross-occurrence matching or UI.
- Pure pagination covers all presets, chapter boundaries and exact-offset reflow.
  No inferred TXT chapters. Immutable paste IDs/text/provenance and positions work;
  dedup/atomic import registration/timestamps remain. See DATA_MODEL.md for schema.
- Toolchain unchanged: Rust 1.99.1, Crux 0.20.0, BoltFFI 0.30.1, rusqlite 0.40.2,
  Kotlin 2.4.21, Gradle 9.1/AGP 9.0.1, JDK 17, BOM 2026.03.01, SDK 36/min 29,
  NDK r30/API 29; arm64-v8a and x86_64 build.

## Next tasks (ordered)
1. Pin/test exact Sudachi.rs revision + compatible Full artifact/checksum/notices;
   build yomoku-analysis (default B, A/C fixtures, Unicode/normalization/proper nouns).
   DECISIONS.md has API/license evidence: upstream 0.7 requires V1 dictionaries;
   hosted API pages mix versions. No dependency/dictionary adopted yet. Bundle and
   measure offline/device use; compose analysis/registration/import atomically.
2. Extend completion with analyzed occurrences/New→Known events/active time, then
   wire frozen IDs through core/runtime/FFI and Next/Previous/swipes. Do not use
   manual status or position-only saves as forward completion. Finish-book remains.
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
PASS: 117 Rust tests (61 storage), including 15 new anchor/override tests;
all app reflow pairs, host JNI and seven Unicode exchanges. App/test builds and
strict lint pass for both ABIs. All seven API 36 x86_64 phone tests PASS after
restarting swangle/KVM emulator (first run had no device). Viewport 1080x2400/420dpi
unchanged; emulator left running. No tablet rerun; arm64 execution untested.
Initial workspace run found one old schema-6 expectation; fixed to 7, full rerun PASS.

## Environment/blockers
- Direct Rust PATH avoids rustup read-only maintenance. Local tool overrides in
  DEVELOPMENT.md. Never run native generation/build/tests concurrently.
- Gradle/ADB need escalation for sockets. medium_phone API 36 uses swangle/KVM;
  startup: tools/start-android-emulator.sh. Default SwiftShader crashes.
  UTP removes app; installDebug to launch. Stop: android emulator stop emulator-5554.
- No local-reader blocker. Nostr config stays empty; ask Q188 before forwarding.
  No production edit/rename/delete, repair or exposed completion/history yet.
