# Resume here

Updated: 2026-10-24. Atomic analyzed-paste repository verified; Full gate remains.
Read AGENTS.md, plan phases 2–3 and PRODUCT_SPEC.md sections 3–4.
Update docs/commit each verified step.

## Working
- Pure Crux core owns navigation/Library/Settings and typed effects. FIFO worker
  owns SQLite; only Render reaches UI. Retained Android sessions/private DB,
  Paste→Library and explicit reader opening work; fresh start Library.
- Exact chapter-local UTF-8 position/Library percentage persist; five presets
  reflow without moving the offset. Scroll resets on reopen/move; column 720dp.
  NO Next/Previous/swipes until atomic real completion.
- Migrations 0005–0007 store frozen completion IDs/page/position, global surfaces/
  dated five-status history, immutable versioned A/B/C anchors and reversible
  global/local edits. No override application, forward action or vocabulary UI.
- NEW Storage::create_analyzed_paste composes canonical paste allocation, callback
  analysis, Mode B snapshot and unique Japanese registrations in one transaction.
  Existing statuses/dates/IDs remain; no encounters/position/dedup. Typed analyzer
  errors survive; all failures roll back. Preload/reuse analyzer before callback:
  it holds the write lock and must not do DB work or publish provisional IDs.
  Repository only: production Android/runtime still use text-only create_paste.
- Sudachi.rs v0.7.0/android-loader-v1 and Full 20261015 stay pinned. Source verifier
  checks 138 files; never alter shared Cargo cache. Eleven real fixtures per loader.
  Full is prepared outside Git; compressed APK includes complete notices.
- Worker-only private publisher verifies size/SHA-256, syncs and atomically
  publishes read-only files and returns that same verified descriptor. Repair
  never writes/truncates live inodes. Eight extraction/recovery device tests pass.
- Debug JNI app probe verifies real fixtures, retry/affinity/stale IDs, recreation/
  stop/resume reuse, close/unmap/fresh-worker reopen. ADB death driver verifies
  two actual force-stop/restart pairs with live Full mappings in new PIDs and
  unchanged cached inode/mtime/read-only bytes. No production analyzer integration.
  Methods/results: SUDACHI_MEASUREMENTS.md; no pressure/LMKD/hardware claim.
- Safe owned loader remains; unsafe mapping requires inode immutability until
  analyzer drop. Initial checksums touch all pages; production lifecycle undecided.
- Paste text/IDs/provenance immutable; no inferred TXT chapters. Exact-hash dedup
  and production atomic import wiring remain. Document deletion preserves vocab.

## Next tasks (ordered)
1. Finish Full gate: actual memory-pressure/reclamation; choose production worker
   lifecycle/reuse. Physical arm64 required before claiming hardware viability.
2. After gate, wire atomic analyzed-paste import into runtime/FFI/Android with
   reused analyzer and exact-hash dedup; extend format imports later.
3. Extend completion with occurrences/New→Known/active time, then wire frozen IDs
   through core/runtime/FFI and Next/Previous/swipes. Finish-book and CI remain.

## Last verified checks
```sh
export PATH=/home/kai/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin:$PATH
cargo build --workspace --locked --offline
cargo test --workspace --all-features --locked --offline
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked --offline -- -D warnings
tools/test-sudachi.sh
tools/test-kotlin-bridge.sh
git diff --check
```
PASS: 125 data-free Rust tests (67 storage), 11 real Full fixtures per owned/mapped
loader, source integrity, host Kotlin/JNI plus seven Unicode exchanges, fmt/Clippy.
Initial Full fixture assumed a half-width compound was one B token; corrected to
validate original source spans, both complete loader suites pass. No open failure.
No Android/Gradle/device/standalone/tablet rerun for this repository-only slice.
Prior 18 Android tests/lint/debug+release builds and death driver pass; commands
and report paths remain in DEVELOPMENT.md and SUDACHI_MEASUREMENTS.md.

## Environment/blockers
- Never run native builds/generation/tests concurrently. Prepare Full before APK
  builds; never prepare/replace/truncate dictionary bytes during mapped probes.
- Android env/commands: DEVELOPMENT.md. ADB/Gradle sockets may need escalation.
  Instrumentation uninstalls app: reinstall debug before separate death driver.
- medium_phone API 36 uses swangle/KVM; tools/start-android-emulator.sh. Default
  SwiftShader crashes. No emulator/user-file/radio/viewport changes this session.
- Actual pressure/LMKD, mid-extraction death/power loss and physical arm64 remain
  unverified. Stop/resume and force-stop are not memory-pressure tests.
- Nostr stays empty; ask Q188 before forwarding. No production edit/rename/delete.
