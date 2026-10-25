# Resume here

Updated: 2026-10-25. Verified atomic TXT original-file repository.
Read AGENTS.md, plan phases 2–4 and PRODUCT_SPEC.md sections 3–4.
## Working
- Pure Crux owns navigation/Library/Settings; only Render reaches UI. FIFO owns
  SQLite/analysis. Fresh sessions start Library. No I/O/SQL in core.
- Exact UTF-8 position/Library percentage + five presets persist; reflow preserves
  offset. Scroll resets on navigation/reopen; column capped at 720dp. Next/Previous/
  counter below content; 48dp net horizontal swipes require no bottom. Previous
  saves position only. Busy/pending gates reject duplicate callbacks; rereads count.
- Paste atomically deduplicates exact SHA-256, saves original Mode B + unique
  Japanese registrations. Existing reuses oldest live match without rescan/writes.
  Created/Existing show dismissible Library summary, never auto-open.
- load_reader_snapshot/load_paste stay read-only. Explicit open_reader_snapshot
  atomically resets finished books to page 1/preset/Mode B; failure rolls back.
  Permanent migration-0011 finish marker survives reflow/position/time, resets on
  deletion; history remains. Missing analysis never rescans; EOF needed for backfill.
- Tokens atomically read original B + live statuses, full metadata + clipped local
  UTF-8 ranges; completion uses token start. Deleted IDs absent. Whole chapter
  validated per lookup (performance unmeasured); base-only for now.
- Compose requests after busy/pending saves clear through gated FIFO/JNI. Current/
  pending/result keys must match; refresh/movement/reflow invalidates snapshots.
  Failed reads need Retry even when clock saves clear errors. UTF-8→UTF-16 checks
  scalar/identity/clipped text; invalid is plain. Known/absent unhighlighted.
- SetReaderStatus validates loaded key/existing ID/timestamp outside Settings.
  Atomic global status/history commit updates repetitions; same-status no-op.
  Deleted ID clears references without registration. Failure preserves snapshot for
  explicit repeat, no replay. Both outcomes drain waiting final time.
- Phone sheet/wide panel show full surface/analyzer reading/base form + five choices.
  Read-only opens; busy/pending/absent disables choices. Committed selection only;
  navigation/Settings/recreation/invalidation dismisses. Container/content input
  extends idle clock. All applicable senses/readings expand with compact POS;
  separate names/categories and lifetime count. Failure requires explicit Retry.
- Clock restore reserves FIFO revision; absolute checkpoint retries freeze payload/
  revision, coalesce maxima and chain revisions. Anchored samples enforce five-minute
  idle cap/pause/background/validity. Reflow preserves deadline; movement retires epoch.
- CoreSession uses elapsedRealtime + 15s Handler checkpoints, Activity resume/pause,
  scrolling and restore-before-Resume. Actions Pause/drain first. Retry in reader/
  Settings; pending saves disable presets. close retains callbacks until busy/BOTH
  pending flags clear; retries each second after onCleared. Abrupt kill can lose pending.
- Completion freezes page/position/preset/AnalysisId/timestamp, drains final time,
  then FIFO reserves ID + whole-page duration for exact retries. One transaction
  commits occurrences/New-only Known/history/time/position/finish. Missing blocks.
  Reload retries exact command; AlreadyRecorded/zero silent. Positive toast once.
  Final new Recorded returns Library + consumed 1.4s confetti (zero/rereads too).
- NativeBridge requires owned verified FD+1; Rust rehashes/maps/closes all paths.
  Lazy FIFO analyzer releases after background work; keep live Full inodes immutable.
- Pinned 2026-10-25 JMdict English/JMnedict SQLite/JSON/BINARY + metadata/sources/
  notices: third_party/dictionaries/README.md. Gradle bundles original .gz.bin sources
  and compiler/notices; complete offline Settings credits survive recreation.
- Verified immutable dictionary path opens lazily on FIFO, reuses connection until
  close/query failure. load_reader_word validates original B/live status/lifetime
  count in one user read transaction. Exact surface/base word/name candidates retain
  all metadata and restriction-aware views. Details: ARCHITECTURE.md/DATA_MODEL.md.
- LoadReaderWord/DismissReaderWord + details/selection/error now reach typed JNI.
  Current/pending/selected/full token identities gate attach; live statuses refresh
  repetitions. Errors survive clock saves; both outcomes drain time. Compose requests
  once gates clear; close/disposal/recreation dismiss. Late callbacks cannot reopen.
- TXT pure strict UTF-8/Shift-JIS/Aozora parser now composes with atomic storage.
  Migration 0012 widens sources.kind preserving IDs/history/FK enforcement. File
  SHA-256 covers original BLOB, separate from paste; duplicate skips decoding/analysis
  and preserves encoding/title/status/progress/finish. Filename title + original
  bytes/encoding + canonical text/ruby + B/unique registrations commit together.
  load_txt_source verifies bytes/ruby read-only; single-text reader/completion accepts
  TXT. No runtime/FFI file path, picker or reader ruby payload/rendering yet.
## Next tasks (ordered)
1. Wire TXT import through lazy FIFO runtime/typed FFI: original bytes/display name,
   encoding-required/error outcomes, Created/Existing and existing analyzer lifecycle.
2. Android multiple-file picker/encoding choice, per-file summary returning Library;
   progress/cancel/background semantics remain separate. CI/custom entries independent.
3. User-run physical arm64 validation once usable.
## Verification
- PASS: 287 data-free Rust tests (12 new TXT storage/migration), strict Clippy/fmt/diff;
  all 12 real Full + 1 runtime fixtures per owned/mapped loader. DEVELOPMENT.md commands.
- No Android/Kotlin/JNI/generation/Python/full-index/release/process/reboot/arm64
  rerun this storage slice. No new package/version or escalation required.
- Host JNI/Android builds must run sequentially: BoltFFI replaces shared artifacts.
- No open failure/blocker. Nostr empty; ask Q188 before forwarding.
