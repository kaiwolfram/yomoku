# Resume here

Updated: 2026-10-23. Debug-app descriptor/reuse/lifecycle baseline verified; see
SUDACHI_MEASUREMENTS.md. Read AGENTS.md, plan phases 2–3 and PRODUCT_SPEC.md
sections 3–4. Update docs/commit each verified step.

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
  Full is explicitly prepared outside Git; compressed APK includes complete notices.
- Worker-only private publisher verifies size/SHA-256, syncs and atomically
  publishes read-only files, returning the same verified descriptor. Repair never
  writes/truncates live inodes. Eight extraction/recovery device tests pass.
- Debug-only app probe consumes that descriptor with release Rust, rehashes its
  mapping, and keeps one analyzer on a test worker through Activity recreate/
  stop/resume. A/B/C/Unicode/page/72 KB validation, retry/affinity/stale IDs,
  ViewModel close/unmap and fresh-worker reopen pass. Production runtime/FFI and
  startup remain independent; test executor/owner are measurement scaffolding.
- Two final x86_64 processes: first extraction+map/init 2.965–3.295 s; cache
  verify+map/init 1.974–3.306 s. App PSS ~403–409 MiB mapped, ~89–98 MiB after
  close. Initial checksum still touches all pages. No pressure/hardware claim.
  Safe owned loader remains; unsafe mapped loader requires inode immutability
  through analyzer drop. Prior standalone owned/mapped and host parity pass.
- Paste text/IDs/provenance immutable; no inferred TXT chapters. Dedup and atomic
  analyzed import/registration remain. Document deletion preserves vocabulary.

## Next tasks (ordered)
1. Finish Full gate: actual memory-pressure/reclamation and process-death recovery
   using the app probe; choose production worker lifecycle/reuse only after these
   results. Physical arm64 remains required before claiming hardware viability.
2. Compose analysis + all unique Japanese surface registration + document import
   atomically (no encounters); preserve existing statuses, test failures/rollback.
3. Extend completion with occurrences/New→Known/active time, then wire frozen IDs
   through core/runtime/FFI and Next/Previous/swipes. Finish-book and CI remain.

## Last verified checks
Android environment in DEVELOPMENT.md. Latest slice:
```sh
./android/gradlew -p android :app:assembleDebug :app:assembleDebugAndroidTest :app:lintDebug --offline --console=plain
./android/gradlew -p android :app:connectedDebugAndroidTest :app:lintDebug --offline --console=plain
./android/gradlew -p android :app:assembleRelease --offline --console=plain
tools/format-kotlin.sh --check
git diff --check
```
PASS: eighteen API-36 x86_64 tests, focused probe, strict lint, both-ABI debug/
unsigned release APKs, 16 KB ELF/ZIP alignment; probe library/facade absent from
release. Reports: .local/test-reports/sudachi-app/{focused-1,full-suite}/.
Workspace build/119 Rust tests, fmt/strict Clippy and source integrity pass.
No host JNI, tablet, standalone probe or host Full-fixture rerun this slice.

## Environment/blockers
- Direct Rust PATH bypasses read-only rustup maintenance (DEVELOPMENT.md). Never
  run native builds/generation/tests concurrently. Prepare Full before APK builds.
- ADB/Gradle may require escalation for sockets. Initial added map assertion
  assumed identical Context and /proc directory aliases; unique test-directory
  suffix fixes it. Final focused + complete suite pass; no open check failure.
- Never prepare/replace/truncate dictionary bytes during mapped tests/probes.
- medium_phone API 36 uses swangle/KVM; tools/start-android-emulator.sh. Default
  SwiftShader crashes. Emulator left running; test-private files removed and
  viewport/radios unchanged. Actual pressure, process death and physical arm64
  remain unverified; do not treat stop/resume as a memory-pressure test.
- Nostr stays empty; ask Q188 before forwarding. No production edit/rename/delete.
