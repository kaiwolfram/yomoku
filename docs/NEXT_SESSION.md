# Resume here

Updated: 2026-10-24. Production analyzed paste and Android lifecycle verified.
Read AGENTS.md, plan phases 2–4 and PRODUCT_SPEC.md sections 3–4.
Update docs/commit each verified step. Physical arm64 is the later user trial.

## Working
- Pure Crux owns navigation/Library/Settings and typed effects; FIFO worker owns
  SQLite/analysis. Only Render reaches UI. Fresh sessions start Library.
- Exact UTF-8 position/Library percentage and five page presets persist; reflow
  preserves the offset. Scroll resets; column 720dp. NO Next/Previous/swipes yet.
- Migrations 0005–0007 store frozen completion IDs/page/position, five-status
  vocabulary/history, immutable A/B/C anchors and reversible global/local edits.
  No forward completion, override application, dictionary or vocabulary UI yet.
- Migration 0008 + import_analyzed_paste atomically deduplicate exact UTF-8 SHA-256,
  persist canonical text/Mode B tokens and register unique Japanese surfaces.
  Existing references oldest live exact match without rescan/progress/status edits.
  Legacy indexing is lazy under write lock; duplicates remain. Failures roll back.
- NEW NativeBridge requires DictionaryProvider and selects the analyzed router;
  Android transfers a verified owned FD+1 (zero means failure), Rust rehashes/maps
  that exact file and closes FD on success/failure. No production text-only fallback.
  Raw Rust constructors remain low-level storage fixtures. No new dependency version.
- Analyzer loads lazily before write transaction on existing FIFO worker, need not
  be Send, and stays on that worker. Load/engine errors can retry; SQL rollback
  reuses healthy cache. Foreground permits reuse without eager loading.
- NEW Activity onStart permits caching; onStop queues release except configuration
  recreation retains warm worker/cache. Hidden imports finish then release; final
  onCleared closes native handle, draining accepted work without joining UI.
  Immutable private publication never rewrites live inodes; later work re-verifies.
- Created/Existing show dismissible typed summary above Library; never open reader.
  Settings/refresh/recreation preserve it; dismissal/tab/new paste/reader clear it.
  Fresh session none; failed import retains draft. Android now delivers duplicates.
- Pure active_time has supplied monotonic ms, five-minute idle cap and reader/
  dictionary interactions/checkpoints; restored duration pauses in new clock epoch.
  Thirteen tests; no SQLite/runtime/FFI/Android timer or page/reflow attribution yet.
- Sudachi.rs v0.7.0/android-loader-v1 + Full 20261015 pinned; native builds verify
  138 source files. Full prepared outside Git; APK bundles dictionary/notices.
- Prior debug JNI/ADB harnesses verify actual process death/recovery and two pressure
  runs reclaiming 67/143 MiB dictionary RSS on API-36 x86_64. No LMKD/arm64 claim.
  Limits: SUDACHI_MEASUREMENTS.md. No eager Library/per-page loading/new executor.
- Paste text/IDs/provenance immutable; no inferred TXT chapters. File imports must
  hash original bytes separately. Document deletion retains vocabulary/history.

## Next tasks (ordered)
1. Persist pending page time, deciding page/reflow attribution under the spec; test
   checkpoints/reopen/reboot and prepare atomic completion handoff.
2. Extend completion with occurrences/New→Known/time, wire frozen command IDs and
   Android clock/lifecycle/interactions, then Next/Previous/swipes. Finish, format
   imports and CI remain unchecked.
3. User-run physical arm64 validation once usable; defer until their app trial.

## Verification / environment
- PASS: workspace build, 157 data-free Rust tests, fmt/strict Clippy, eleven analysis
  + one worker fixture per Full loader; host real-Full Kotlin/JNI + seven Unicode
  exchanges; Kotlin format, shell syntax, both-ABI debug/test APKs, strict lint and
  all 19 API-36 x86_64 Android tests. Reports: .local/test-reports/analyzed-import/.
- Initial full runs 18/19 and focused notices failed at Loading. Traces showed its
  producer could resume off-main under UI tests. Publish notices on Main.immediate;
  original assertions/10s timeout retained. Focused 2/2 and final full 19/19 pass.
  Earlier failed/diagnostic reports retained; temporary logging removed.
- Real Full worker/JNI old 10s timeout fixed with 60s load waits only there; data-free
  waits unchanged, Android storage 30s. Native TID verifies worker affinity (Java
  Thread identity changes across JNI attachment). Benign Crux shutdown logs persist.
- Emulator disconnected and restarted with swangle/no data reset; ADB escalation
  used. No open failure. No unsigned release, standalone/pressure/death probes,
  tablet or physical-arm64 rerun. Prior measurements retain their documented limits.
- Commands/env: DEVELOPMENT.md. Rust PATH workaround; locked/offline build/test/
  fmt/Clippy, tools/test-sudachi.sh, tools/test-kotlin-bridge.sh. Host JNI now needs
  explicitly prepared pinned Full. Never run native builds/generation/tests together
  or modify Full while mapped. Gradle/ADB can need socket escalation.
- Nostr configuration empty; ask Q188 before forwarding. No local reader blocker.
