# Resume here

Updated: 2026-10-23. Private Full extraction/recovery verified independently of
analyzer integration; see SUDACHI_MEASUREMENTS.md. Read AGENTS.md, plan phases
2–3 and PRODUCT_SPEC.md sections 3–4. Update docs/commit each verified step.

## Working
- Pure Crux core owns navigation/Library/Settings, busy/errors and typed effects.
  FIFO worker owns SQLite; only Render reaches UI. Retained Android sessions,
  private DB, Paste→Library and explicit reader opening work; fresh start Library.
- Exact chapter-local UTF-8 position and Library percentage persist; five presets
  reflow without moving the offset. Scroll resets on reopen/move; column 720dp.
  NO Next/Previous/swipes until atomic real completion.
- Migrations 0005–0007 store frozen completion IDs/page/position, global surfaces/
  dated five-status history, immutable versioned A/B/C anchors and reversible
  global/local edits. No override application, forward action or vocabulary UI.
- Sudachi.rs v0.7.0/android-loader-v1 and Full 20261015 stay pinned. Source verifier
  checks 138 files; never alter shared Cargo cache. Ten real fixtures per loader.
  Full stays prepared explicitly outside Git. Gradle bundles compressed Full and
  complete notices; Settings displays notices. No eager dictionary copy/load.
- Lazy worker-only Kotlin helper verifies size/SHA-256, syncs and atomically
  publishes private read-only files; returns the same verified descriptor.
  Retry/repair never writes/truncates published inodes. Eight device tests pass.
  No startup/runtime/FFI caller yet; native mapping/worker reuse remains separate.
- Standalone owned/mapped comparison passes three pairs per debug/release profile
  offline API-36 x86_64; both release ABIs link. Mapped release init 0.932–0.944 s;
  anonymous RSS ~2.5 MiB versus ~318 MiB owned. Initial total RSS/peak stay similar
  after checksum touches all pages. No app/lifecycle/cold-storage/arm64 claim.
- Unsafe mapped API requires backing-inode immutability through analyzer drop;
  hashes the mapping itself. Safe owned loader remains. Full A/B/C parity passes.
- Paste text/IDs/provenance immutable; no inferred TXT chapters. Dedup and atomic
  analyzed import/registration remain. Document deletion preserves vocabulary.

## Next tasks (ordered)
1. Resolve remaining Full loading gate before runtime integration: consume the
   verified private descriptor in an in-app analyzer measurement harness. Measure
   combined app memory/lifecycle/low-memory behavior and worker analyzer reuse.
   Physical arm64 remains required before claiming hardware viability.
2. Compose analysis + all unique Japanese surface registration + document import
   atomically (no encounters); preserve existing statuses, test failures/rollback.
3. Extend completion with occurrences/New→Known/active time, then wire frozen IDs
   through core/runtime/FFI and Next/Previous/swipes. Finish-book and CI remain.

## Last verified checks
Android environment/commands/reports in DEVELOPMENT.md; most recent slice:
```sh
./android/gradlew -p android :app:assembleDebug :app:assembleDebugAndroidTest :app:lintDebug --offline --console=plain
./android/gradlew -p android :app:connectedDebugAndroidTest :app:lintDebug --offline --console=plain
tools/format-kotlin.sh --check
git diff --check
```
PASS: both-ABI APK/test assembly, strict lint, seventeen API-36 x86_64 phone tests,
Kotlin formatting/whitespace. Final eight focused tests/lint also pass after an
interrupted-input fixture fix. Extraction+verification 2.273–3.273 s, verified
reopen 0.632–0.689 s across two processes; no analyzer memory/cold-storage claim.
Reports: .local/test-reports/sudachi-extraction/. Test-private files removed;
viewport/radios unchanged. No tablet, Rust/host JNI or native probe rerun for this
Kotlin/platform slice. Prior baseline: 119 Rust tests, ten Full fixtures per
loader, source integrity, fmt/Clippy and twelve offline Android probe processes.

## Environment/blockers
- Direct Rust PATH bypasses read-only rustup maintenance (DEVELOPMENT.md). Never
  run native builds/generation/tests concurrently. Prepare Full before APK builds.
- ADB/Gradle need escalation for sockets. Initial compile used non-public
  O_DIRECTORY; public O_RDONLY directory sync passes. Focused rerun initially
  found no device; emulator restart and final verification pass. No open failure.
- Never prepare/replace/truncate dictionary bytes during mapped host tests/probes.
- medium_phone API 36 uses swangle/KVM; tools/start-android-emulator.sh. Default
  SwiftShader crashes. Emulator restarted and left running at phone viewport.
- Nostr stays empty; ask Q188 before forwarding. No production edit/rename/delete.
