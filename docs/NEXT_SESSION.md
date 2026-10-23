# Resume here

Updated: 2026-10-23. Android Full assets and complete offline notices verified.
Read AGENTS.md, plan phases 2–3 and PRODUCT_SPEC.md sections 3–4 next.
After each completed, verified step, update docs and commit automatically.

## Working
- Pure Crux core owns navigation/Library/Settings, busy/errors and typed effects.
  FIFO worker owns SQLite; only Render reaches UI. Android retained sessions,
  private DB, Paste→Library and explicit reader opening work; fresh start Library.
- Exact chapter-local UTF-8 position and current Library percentage persist;
  five presets reflow without moving the offset. Scroll resets on reopen/move.
  Reader caps at 720dp. NO Next/Previous/swipes until atomic real completion.
- Migration 0005 freezes completion IDs/page anchors/position atomically;
  duplicate commands never move progress, rereads need fresh IDs. No forward hook.
- Migration 0006 stores global exact surfaces, five statuses and dated history.
  Registration preserves statuses/dates; lookup/no-op changes add no history.
- Migration 0007 stores immutable versioned A/B/C analysis/token/source anchors
  and separate reversible global/local edits. No override application or UI yet.
- Sudachi.rs v0.7.0 remains pinned with android-loader-v1 source patch. The source
  verifier reverses the patch and checks 138 files; never alter shared Cargo cache.
  Both Android library targets compile; nine actual Full-data host fixtures pass.
- Full 20261015 remains checksum-pinned, prepared explicitly outside Git.
  Gradle bundleSudachi verifies/stages Full + complete LEGAL/LICENSE as APK assets
  without downloads. Settings displays both entire notices offline/recreation.
  Dictionary is APK-compressed; app does not copy/load it at startup. No analyzer
  runtime/FFI/UI entry point. Current owned constructor needs ~315 MiB for bytes
  before engine structures; actual device analyzer measurements remain next.
- Paste text/IDs/provenance immutable; no inferred TXT chapters. Dedup and atomic
  analyzed import/registration remain. Document deletion preserves vocabulary.

## Next tasks (ordered)
1. Measure Full initialization, memory, tokenization and offline analysis on
   Android before runtime integration; decide loading/storage from measurements.
   Bundling/notices are done. Debug APK is 184,525,421 bytes; compressed dictionary
   entry 146,067,671 bytes. Arm64 execution still untested.
2. Compose analysis + all unique Japanese surface registration + document import
   atomically (no encounters); preserve existing statuses, test failures/rollback.
3. Extend completion with occurrences/New→Known/active time, then wire frozen IDs
   through core/runtime/FFI and Next/Previous/swipes. Finish-book and CI remain.

## Last verified checks
Use direct Rust PATH; exact Android env/commands/reports in DEVELOPMENT.md:
```sh
cargo test --workspace --all-features --locked --offline
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked --offline -- -D warnings
python3 tools/prepare-sudachi.py --archive .local/sudachi/sudachi-dictionary-20261015-full.zip
python3 tools/prepare-sudachi.py --android-assets
tools/test-sudachi.sh
tools/format-kotlin.sh --check
git diff --check
```
PASS: 119 data-free Rust tests + nine real Full fixtures/source integrity;
Python syntax, missing/corrupt bundle preservation, offline/repeated preparation;
Kotlin formatting; both-ABI app/test APK assembly + strict lint; installed/APK
SHA-256 and ZIP 16 KB alignment. All nine API 36 x86_64 phone tests + two new
asset/notices tests in tablet landscape PASS in airplane mode. XML copied to
.local/test-reports/sudachi-bundling/{phone,tablet-landscape}/. Host JNI not rerun.
No device analyzer/performance claims. No outstanding failure.

## Environment/blockers
- Direct Rust PATH bypasses rustup read-only maintenance (see DEVELOPMENT.md).
  Never run native generation/build/tests concurrently. Prepare Full before APKs.
- Gradle/ADB need escalation for sockets; initial tablet commands failed without
  it, then passed with escalation. medium_phone API 36 uses swangle/KVM;
  tools/start-android-emulator.sh. Default SwiftShader crashes.
- Emulator left running; original 1080x2400/420dpi and airplane mode off restored.
- Nostr stays empty; ask Q188 before forwarding. No production edit/rename/delete.
