# Resume here

Updated: 2026-10-25. Verified selected-word dictionary/count repository.
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
  extends idle clock. Definitions/names/count await app integration.
- Clock restore reserves FIFO revision. Absolute checkpoints freeze payload/revision
  for retry, coalesce maxima and chain revisions. Anchored monotonic observations
  enforce five-minute idle cap, pause/background exclusion, invalid-sample rejection.
  Reflow/same-offset saves preserve idle deadline. Movement/exit retire epoch.
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
  Lazy FIFO analyzer releases after accepted background work; recreation retains it.
  Never modify live Full inodes. Sudachi v0.7.0/android-loader-v1 + Full 20261015
  pinned; 138 files verified; Full outside Git. See SUDACHI_MEASUREMENTS.md.
- prepare-dictionaries.py builds pinned 2026-10-25 JMdict English/JMnedict immutable
  SQLite/JSON/BINARY + metadata/sources/notices. Contract: third_party/dictionaries/README.md.
  Gradle bundles pinned build/index/compiler/sources/notices; .gz.bin keeps original
  archives; complete offline Settings credits survive recreation.
- storage::dictionary::Dictionary opens existing read-only assets, checks format
  identity (not checksum), unions exact surface/base candidates with per-form
  provenance/separate source IDs. Typed payloads retain all fields. Display applicable
  views enforcing re_restr/re_nokanji/stagk/stagr, with all licensed readings.
- wordDictionaryFiles is lazy/worker-only, private/no-backup; verifies size/SHA-256
  before verifiedPath()/cache reopen. Repair preserves held inodes. No FIFO use yet.
- load_reader_word(dictionary,key,index) reads validated original B token/status +
  indexed exact-surface lifetime count in one user transaction; separate word/name
  candidates retain applicable views; absent stays zero. Invalid selection/errors
  differ from empty matches. Repository-only; no shell/UI yet.

## Next tasks (ordered)
1. Connect verified dictionary path + load_reader_word via lazy FIFO/typed FFI.
   Add serializable details/request/view + current reader/selection cancellation.
2. Render all applicable senses/readings, compact POS, separate names and count;
   test simultaneous matches/no match, retry, movement/recreation and read-only opens.
3. User-run physical arm64 validation once usable. Formats/CI remain open.

## Verification / environment
- PASS this step: 256 data-free Rust tests (130 storage; eight new), strict
  Clippy/Rust fmt/diff. Commands + coverage: DEVELOPMENT.md, latest section.
- Android/JNI/Kotlin/generation/Python/real data/release/recovery/physical arm64
  not rerun. Prior Android: 14 phone assets + two wide/dark credits; lint passed.
- Data: .local/dictionaries/index-v1-20261025{,-repeat}/; Android reports:
  .local/test-reports/dictionary-assets/. Full/assets preserved (all outside Git).
- No open failure/blocker. Nostr empty; ask Q188 before forwarding.
