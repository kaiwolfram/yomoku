# Resume here

Updated: 2026-10-24. Analyzed-import Rust worker verified; native/Android next.
Read AGENTS.md, plan phases 2–4 and PRODUCT_SPEC.md sections 3–4.
Update docs/commit each verified step. Physical arm64 is the later user trial.

## Working
- Pure Crux core owns navigation/Library/Settings and typed effects. FIFO worker
  owns SQLite; only Render reaches UI. Retained Android sessions/private DB,
  Paste→Library and explicit reader opening work; fresh start Library.
- Exact UTF-8 position/Library percentage persist; five presets preserve offset.
  Scroll resets; column 720dp. NO Next/Previous/swipes before atomic completion.
- Migrations 0005–0007 store frozen completion IDs/page/position, global surfaces/
  dated five-status history, immutable versioned A/B/C anchors and reversible
  global/local edits. No override application, forward action or vocabulary UI.
- Storage::import_analyzed_paste atomically deduplicates exact UTF-8 SHA-256,
  canonical text, Mode B snapshot and unique Japanese registrations. Existing
  returns oldest live match without analysis/rescan/title/progress/status changes.
  Migration 0008 lazily hashes legacy pastes under write lock, preserving duplicates.
  All failures roll back; document deletion/reimport gets fresh IDs/progress.
- NEW effect_router_with_analyzed_imports uses that repository on the existing
  FIFO worker. Send loader is lazy; analyzer need not be Send, lives/drops only
  on worker, loads before write transaction. Wall time dates registrations.
  Failed load/engine/invalid output can retry; SQL rollback reuses healthy cache.
  Created/Existing upsert one Library row by ID and never open the reader.
- NEW AnalyzerLifecycle weak handle queues cache policy without retaining worker.
  Foreground permits reuse without eager loading; background releases after prior
  accepted work. Hidden imports finish then release; final close drains/drops.
  Seven runtime tests + one real Full fixture per loader verify these boundaries.
  NativeBridge/Android STILL select legacy text-only router. No visible summary,
  verified descriptor callback or platform lifecycle signals wired yet. Remove
  production fallback when connecting native shell; do not leave optional behavior.
- Pure active_time uses supplied monotonic ms, five-minute idle cap and reader/
  dictionary interactions. Checkpoints retain pending time without resetting idle;
  restore duration paused in fresh clock epoch. Thirteen tests; no SQLite/runtime/
  FFI/Android timer wiring or page/reflow attribution yet.
- Sudachi.rs v0.7.0/android-loader-v1 + Full 20261015 pinned; source verifier checks
  138 files. Full prepared outside Git; APK bundles complete dictionary/notices.
- Worker-only private publisher verifies size/SHA-256, syncs/atomically publishes
  read-only files and returns same verified descriptor. Never rewrite live inodes.
  Eight device extraction/recovery tests pass. Safe owned loader for arbitrary paths.
- Debug JNI/ADB harnesses verify reuse/affinity/retry/close, two process-death pairs
  and two pressure runs reclaiming 67/143 MiB dictionary RSS on API-36 x86_64.
  Limits/results: SUDACHI_MEASUREMENTS.md. No LMKD/physical-arm64 claim.
- Production choice: lazy immutable mapping on existing FIFO worker, retained over
  configuration changes, idle cache release on UI-hidden/background/final close.
  Later work re-verifies/reopens. No eager Library load/per-page reload/new worker.
- Paste text/IDs/provenance immutable; no inferred TXT chapters. Future file imports
  hash original bytes separately. Document deletion keeps vocab. No import encounters.

## Next tasks (ordered)
1. Connect analyzed-import router through typed native verified-descriptor delivery,
   Android retained/background lifecycle and Created/Existing summary. Test JNI/
   device rollback/dedup/vocab/reuse/release/retry and no eager Library load.
2. Persist pending page time (page/reflow attribution open), extend completion with
   occurrences/New→Known/time, then wire frozen IDs and Android clock/lifecycle/
   interactions + Next/Previous/swipes. Finish/format imports/CI remain unchecked.
3. User-run physical arm64 validation once usable; defer until their app trial.

## Verification / environment
- This step: 154 data-free Rust tests (76 storage, seven runtime); workspace build,
  fmt, strict Clippy; 11 analysis + one runtime real Full fixture per owned/mapped
  loader, 138-file source integrity; host Kotlin/JNI + seven Unicode exchanges;
  shell syntax and whitespace pass. No open check failure/blocker.
- No Android build/lint/instrumentation/emulator/probe rerun for this Rust slice.
  Previous final device baseline: 18/18 tests, strict lint, debug/test APKs, two
  death/recovery pairs and two real pressure runs pass; reports in .local/.
- Commands/env: DEVELOPMENT.md. Rust PATH workaround, locked/offline workspace
  build/test/fmt/Clippy, tools/test-sudachi.sh, tools/test-kotlin-bridge.sh.
  Never run native builds/generation/tests concurrently; never prepare/replace/
  truncate Full during mapped tests. Gradle/ADB sockets can need escalation.
- Nostr configuration empty; ask Q188 before forwarding. No local-reader blocker.
