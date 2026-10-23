# Resume here

Updated: 2026-10-23. Android standalone Full measurements verified; see
SUDACHI_MEASUREMENTS.md for exact results and limits. Read AGENTS.md, plan
phases 2–3 and PRODUCT_SPEC.md sections 3–4. Update docs/commit each verified step.

## Working
- Pure Crux core owns navigation/Library/Settings, busy/errors and typed effects.
  FIFO worker owns SQLite; only Render reaches UI. Retained Android sessions,
  private DB, Paste→Library and explicit reader opening work; fresh start Library.
- Exact chapter-local UTF-8 position and current Library percentage persist;
  five presets reflow without moving the offset. Scroll resets on reopen/move.
  Reader caps at 720dp. NO Next/Previous/swipes until atomic real completion.
- Migrations 0005–0007 store frozen completion IDs/page/position, global surfaces/
  dated five-status history, immutable versioned A/B/C anchors and reversible
  global/local edits. No override application, forward action or vocabulary UI.
- Sudachi.rs v0.7.0 stays pinned with android-loader-v1. Source verifier reverses
  patch/checks 138 files; never alter shared Cargo cache. Nine real Full fixtures
  pass. Full 20261015 remains checksum-pinned/prepared explicitly outside Git.
- Gradle bundles Full + entire LEGAL/LICENSE offline; Settings shows both notices
  through recreation. Dictionary stays APK-compressed, not loaded/copied at startup.
- Standalone Android probe/driver execute real adapter outside app/runtime/FFI.
  Three release + three debug offline API 36 x86_64 processes pass A/B/C, mixed/
  Unicode/OOV and 72 KB input. Both probe ABIs link; physical arm64 untested.
  Release init 1.03–1.20 s, debug 12.31–12.69 s; ~318 MiB anonymous after init,
  ~334–336 MiB peak RSS. Not app/JVM/lifecycle/cold-storage measurements.
- Paste text/IDs/provenance immutable; no inferred TXT chapters. Dedup and atomic
  analyzed import/registration remain. Document deletion preserves vocabulary.

## Next tasks (ordered)
1. Resolve measured Full memory cost before runtime integration: compare verified
   immutable mapped storage with owned baseline; measure private asset extraction
   and combined app memory. Keep on-demand worker initialization/reuse. Physical
   arm64 measurements remain required before claiming hardware viability.
2. Compose analysis + all unique Japanese surface registration + document import
   atomically (no encounters); preserve existing statuses, test failures/rollback.
3. Extend completion with occurrences/New→Known/active time, then wire frozen IDs
   through core/runtime/FFI and Next/Previous/swipes. Finish-book and CI remain.

## Last verified checks
Use direct Rust PATH; Android env/commands/reports in DEVELOPMENT.md:
```sh
cargo test --workspace --all-features --locked --offline
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked --offline -- -D warnings
tools/test-sudachi.sh
python3 tools/measure-sudachi-android.py --serial emulator-5554
python3 tools/measure-sudachi-android.py --serial emulator-5554 --profile debug
git diff --check
```
PASS: 119 data-free Rust tests, nine real Full fixtures/source integrity, Python
syntax, both-ABI release probe links, six offline probe runs and final auto-device/
source-verification smoke. Reports: .local/test-reports/sudachi-android/. Radios
restored/verified, temporary device files removed. No outstanding check failure.
No APK/Compose instrumentation, Android lint, Kotlin/JNI rerun for this tooling
slice. Prior bundling session: both-ABI assembly/lint, nine phone tests + two
tablet-landscape tests offline PASS; APK 184,525,421 bytes.

## Environment/blockers
- Direct Rust PATH bypasses read-only rustup maintenance (DEVELOPMENT.md). Never
  run native builds/generation/tests concurrently. Prepare Full before APK builds.
- ADB/Gradle need escalation for sockets. Probe's first build needed target CC/AR;
  fixed in driver. Initial Unicode assertion assumed rare kanji stayed standalone
  next to NUL; replaced with existing host-verified fixtures. All reruns pass.
- medium_phone API 36 uses swangle/KVM; tools/start-android-emulator.sh. Default
  SwiftShader crashes. Emulator left running, phone viewport and radios restored.
- Nostr stays empty; ask Q188 before forwarding. No production edit/rename/delete.
