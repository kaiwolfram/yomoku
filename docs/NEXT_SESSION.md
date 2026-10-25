# Resume here

Updated: 2026-10-25. Verified reader dictionary panel.
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
  Lazy FIFO analyzer releases after accepted background work; recreation retains it.
  Never modify live Full inodes; 138-file pins/loader: SUDACHI_MEASUREMENTS.md.
- Pinned 2026-10-25 JMdict English/JMnedict SQLite/JSON/BINARY + metadata/sources/
  notices: third_party/dictionaries/README.md. Gradle bundles original .gz.bin sources
  and compiler/notices; complete offline Settings credits survive recreation.
- storage::dictionary::Dictionary opens existing read-only assets, checks format
  identity (not checksum), unions exact surface/base candidates with per-form
  provenance/separate source IDs. Typed payloads retain all fields. Display applicable
  views enforcing re_restr/re_nokanji/stagk/stagr, with all licensed readings.
- wordDictionaryFiles verifies pinned size/hash on FIFO-only initial path access;
  cached immutable connection reuses until close. Query failure retires it for repair.
- load_reader_word validates original B token + live status/exact lifetime count in
  one user read transaction; separate word/name candidates keep all applicable views.
- LoadReaderWord/DismissReaderWord + details/selection/error now reach typed JNI.
  Current/pending/selected/full token identities gate attach; live statuses refresh
  repetitions. Errors survive clock saves; both outcomes drain time. Compose requests
  once gates clear; close/disposal/recreation dismiss. Late callbacks cannot reopen.

## Next tasks (ordered)
1. Continue phase-4 TXT imports: UTF-8/Shift-JIS/manual fallback and Aozora fixtures;
   verify dependency APIs before adoption, then atomic analysis/registration wiring.
2. CI or missing-match custom definitions remain independent open tasks.
3. User-run physical arm64 validation once usable.

## Verification / environment
- PASS this slice: Kotlin fmt/diff, both-ABI debug/test APKs + strict Android lint;
  14 panel/span/session tests phone/light + wide/dark; 17 reader/clock/paste/nav tests.
  Two fixture failures corrected; phone movement rerun passes. DEVELOPMENT.md commands.
- Prior 262 Rust tests/Clippy/host JNI/generation unchanged, not rerun; no release/
  Python/full scans/standalone Full/process/reboot/physical arm64 rerun.
- Reports: .local/test-reports/reader-dictionary-panel/. AVD restored to phone/light.
- Run host JNI and Android builds sequentially: BoltFFI replaces shared artifacts.
- No open failure/blocker. Nostr empty; ask Q188 before forwarding.
