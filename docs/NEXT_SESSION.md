# Resume here

Updated: 2026-10-25. Typed read-only Rust word/name lookup verified.
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
  UTF-8 ranges. Crossing completion uses token start. Deleted IDs remain absent.
  Whole chapter validated per lookup (performance unmeasured); base-only for now.
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
  extends idle clock. Definitions/names/count await app integration.
- Clock restore reserves FIFO revision. Absolute checkpoints freeze payload/revision
  for retry, coalesce maxima and chain revisions. Anchored monotonic observations
  enforce five-minute idle cap, pause/background exclusion, invalid-sample rejection.
  Reflow/same-offset saves preserve idle deadline. Movement/exit retire epoch.
- CoreSession uses elapsedRealtime + 15s Handler checkpoints, Activity resume/pause,
  scrolling and restore-before-Resume. Actions Pause/drain first. Retry in reader/
  Settings; pending saves disable presets. close rejects dispatch, retains callbacks
  until busy/BOTH pending flags clear; failed obligations retry each second after
  onCleared. Abrupt kill can lose pending. View exposes duration, no partial history.
- Completion freezes page/position/preset/AnalysisId/timestamp, drains final time,
  then FIFO reserves ID + whole-page duration for exact retries. One transaction
  commits occurrences/New-only Known/history/time/position/finish. Missing blocks.
  Reload failure retries exact command; AlreadyRecorded is silent. Positive notice
  toasts once; zero silent. Final new Recorded returns Library + consumed-before-
  animation 1.4s confetti, including zero/rereads; recreation cannot replay.
- NativeBridge requires owned verified FD+1; Rust rehashes/maps/closes all paths.
  Lazy FIFO analyzer releases after accepted background work; recreation retains it.
  Never modify live Full inodes. Sudachi v0.7.0/android-loader-v1 + Full 20261015
  pinned; 138 source files verified; Full outside Git, notices bundled. Measurements:
  SUDACHI_MEASUREMENTS.md. Physical arm64 remains deferred user trial.
- prepare-dictionaries.py builds pinned 2026-10-25 JMdict English/JMnedict immutable
  SQLite + JSON/BINARY forms, retaining full metadata, sources, checksums and CC
  BY-SA 4.0 notices. Offline by default; never replaces published output. Contract:
  third_party/dictionaries/README.md. No APK wiring yet.
- storage::dictionary::Dictionary opens existing read-only assets, checks format
  identity (not checksum), unions exact surface/base candidates with per-form
  provenance and separate source IDs. Typed core payloads retain all fields;
  applicable index views enforce re_restr/re_nokanji/stagk/stagr. Display views,
  not raw senses. Keep all licensed readings, never filter by analyzer reading.

## Next tasks (ordered)
1. Bundle/verify immutable dictionary index and complete notices; expose Settings
   credits. Installer must verify bytes before supplying the Rust repository path.
2. Connect applicable definitions/readings/compact POS, separate names and lifetime
   encounter count to panel via FIFO/FFI. Retain current reader-key cancellation.
3. User-run physical arm64 validation once usable. Formats/CI remain open.

## Verification / environment
- PASS: 248 data-free Rust tests (122 storage), strict Clippy/Rust fmt/diff; five
  pure + five repository lookup tests; explicit full-index test validates 962,639
  payloads and real lookups; nine Python tests. Commands: DEVELOPMENT.md/README.
- Generated data: .local/dictionaries/index-v1-20261025{,-repeat}/ (not in Git).
  No user schema/FFI/UI change; existing locked serde_json added directly to storage.
  Android/JNI/generation/Full/release/recovery/physical-arm64 not rerun. Prior 24
  Android tests + wide/dark word matrix passed: .local/test-reports/reader-words/.
  No open failure/blocker. Nostr empty; ask Q188 before forwarding.
