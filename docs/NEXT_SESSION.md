# Resume here

Updated: 2026-10-23. Full app force-stop/recovery verified; pressure gate remains.
Read AGENTS.md, plan phases 2–3 and PRODUCT_SPEC.md sections 3–4.
Update docs/commit each verified step.

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
- Debug-only JNI app probe rehashes the verified mapping; real A/B/C/Unicode/page/
  72 KB fixtures, retry/affinity/stale IDs, recreation/stop/resume reuse and close/
  unmap/fresh-worker reopen pass. No production runtime/FFI/startup integration.
- New ADB-only debug Activity + tools/test-sudachi-process-death.py: two actual
  force-stop/restart pairs with live Full mappings pass in new PIDs, preserving
  cached inode/mtime/read-only bytes without graceful close. Cached verify/map/init
  2.704–3.012 s; PSS ~419 MiB after validation. No pressure/LMKD/hardware claim.
  Exact methodology/results: SUDACHI_MEASUREMENTS.md. Probe absent from release.
- Safe owned loader remains; unsafe mapped loader requires inode immutability
  through analyzer drop. Initial checksums still touch all pages. Prior standalone
  owned/mapped and host parity pass. Production lifecycle remains undecided.
- Paste text/IDs/provenance immutable; no inferred TXT chapters. Dedup and atomic
  analyzed import/registration remain. Document deletion preserves vocabulary.

## Next tasks (ordered)
1. Finish Full gate: actual memory-pressure/reclamation; choose production worker
   lifecycle/reuse based on results. Force-stop recovery is now verified, but
   physical arm64 remains required before claiming hardware viability.
2. Compose analysis + all unique Japanese surface registration + document import
   atomically (no encounters); preserve existing statuses, test failures/rollback.
3. Extend completion with occurrences/New→Known/active time, then wire frozen IDs
   through core/runtime/FFI and Next/Previous/swipes. Finish-book and CI remain.

## Last verified checks
Android environment in DEVELOPMENT.md. Latest slice:
```sh
./android/gradlew -p android :app:assembleDebug :app:assembleDebugAndroidTest :app:lintDebug --offline --console=plain
/home/kai/Android/Sdk/platform-tools/adb -s emulator-5554 install -r android/app/build/outputs/apk/debug/app-debug.apk
python3 tools/test-sudachi-process-death.py --serial emulator-5554
./android/gradlew -p android :app:connectedDebugAndroidTest :app:lintDebug --offline --console=plain
./android/gradlew -p android :app:assembleRelease --offline --console=plain
tools/format-kotlin.sh --check
git diff --check
```
PASS: two death/recovery pairs, eighteen API-36 x86_64 tests, strict lint, both-ABI
APK builds, manifest/DEX/native probe release exclusion, Kotlin format/Python syntax.
Reports: .local/test-reports/sudachi-process-death/{20261023T194512308957Z.json,full-suite/}.
Final driver repeat also passes: 20261023T194946330811Z.json in that directory.
No Rust/host JNI/host Full fixtures/standalone/tablet rerun this platform slice;
prior workspace 119 Rust tests, fmt/strict Clippy and source integrity pass.

## Environment/blockers
- Direct Rust PATH bypasses read-only rustup maintenance (DEVELOPMENT.md). Never
  run native builds/generation/tests concurrently. Prepare Full before APK builds.
- ADB/Gradle may require socket escalation. Connected tests uninstall the app:
  reinstall debug before the death driver (checked by preflight). No open failure.
- Never prepare/replace/truncate dictionary bytes during mapped tests/probes.
- medium_phone API 36 uses swangle/KVM; tools/start-android-emulator.sh. Default
  SwiftShader crashes. Emulator left running; test-private files removed and
  viewport/radios unchanged. Run death driver separately from instrumentation.
- Actual pressure/LMKD, mid-extraction death/power loss and physical arm64 remain
  unverified. Neither stop/resume nor force-stop is a memory-pressure test.
- Nostr stays empty; ask Q188 before forwarding. No production edit/rename/delete.
