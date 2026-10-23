# Resume here

Updated: 2026-10-23. Android standalone owned/mapped Full comparison verified; see
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
  patch/checks 138 files; never alter shared Cargo cache. Ten real Full fixtures
  per loader pass. Full 20261015 stays pinned/prepared explicitly outside Git.
- Gradle bundles Full + entire LEGAL/LICENSE offline; Settings shows both notices
  through recreation. Dictionary stays APK-compressed, not loaded/copied at startup.
- Standalone probe/driver compare real owned/mapped loaders outside app/runtime/
  FFI. Three pairs per debug/release profile pass offline API 36 x86_64; both
  release ABIs link. Mapped release init 0.932–0.944 s; ~2.5 MiB anonymous versus
  ~318 MiB owned. Initial total RSS/peak remain similar after checksum touches
  all pages. No app/lifecycle/cold-storage/reclamation or physical arm64 claim.
- Unsafe mapped API requires backing-file immutability until analyzer drop;
  hashes the mapping itself. Safe owned API remains. Full A/B/C output parity
  passes, including metadata/anchors and closed original file handle.
- Paste text/IDs/provenance immutable; no inferred TXT chapters. Dedup and atomic
  analyzed import/registration remain. Document deletion preserves vocabulary.

## Next tasks (ordered)
1. Resolve remaining Full loading gate before runtime integration: implement/
   measure on-demand private extraction, verified immutable publication/failure
   recovery, combined app memory and lifecycle/low-memory behavior. Reuse a worker
   analyzer. Physical arm64 remains required before claiming hardware viability.
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
python3 tools/measure-sudachi-android.py --serial emulator-5554 --storage both
python3 tools/measure-sudachi-android.py --serial emulator-5554 --storage both --profile debug
git diff --check
```
PASS: workspace build, 119 data-free Rust tests, ten Full fixtures per loader/
source integrity, fmt/Clippy, Python/shell syntax, both-ABI release probe links,
twelve offline probe processes. Reports: .local/test-reports/sudachi-android/. Radios
restored/verified, temporary device files removed. No outstanding check failure.
No APK/Compose instrumentation, Android lint, Kotlin/JNI rerun for this tooling
slice. Prior bundling session: both-ABI assembly/lint, nine phone tests + two
tablet-landscape tests offline PASS; APK 184,525,421 bytes.

## Environment/blockers
- Direct Rust PATH bypasses read-only rustup maintenance (DEVELOPMENT.md). Never
  run native builds/generation/tests concurrently. Prepare Full before APK builds.
- ADB/Gradle need escalation for sockets. First comparison stopped before analysis
  while mobile-data disable was still applying; bounded setting polls now verify
  both offline state and restoration. Both full reruns pass. No outstanding failure.
- Never prepare/replace/truncate dictionary bytes during mapped host tests/probes.
- medium_phone API 36 uses swangle/KVM; tools/start-android-emulator.sh. Default
  SwiftShader crashes. Emulator left running, phone viewport and radios restored.
- Nostr stays empty; ask Q188 before forwarding. No production edit/rename/delete.
