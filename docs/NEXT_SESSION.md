# Resume here
Updated: 2026-10-29. Focused long-page reader stalls resolved; effective reader live.

## Current slice
- Word links now use static spans/explicit empty TextLinkStyles. Long LTR fragments
  render as annotated blocks at measured soft line ends, preserving Unicode,
  wrapping/spacing/crossing selections. Bidi/paragraph styles keep complete layouts.
  All four formerly blocked long-page cases pass; fixtures await Library counts
  and effective tokens before commands. No logical pagination/position change.
- NativeBridge constructs YomokuBridge::with_effective_reader with the existing
  verified dictionary providers, FIFO/cache, lifecycle and compact codecs.
- Reader/words/status/custom/furigana/Library/completion share effective partitions.
  Original B stays immutable and supplies contiguous Japanese editor anchors.
  No full chapter keys cross JNI; selections carry native page revision/segment.
- A/B/C and manual edits support default All versus This occurrence and reversal.
  Explicit missing A/C generation uses the import analyzer cache; stored reuse and
  reopen need no Full. Accepted writes keep native exact Retry/Cancel obligations.
- Success retires editor/page; reader shows explicit Reload. Normal invalidation
  and explicit token loads clear that page's reload marker, fixing same-book reopen
  after edits/completion. Pending writes/errors/retry payloads remain intact.
- Android token-read attempts follow base/position/preset, including same-anchor
  position saves/reflow. Same-context failures still require explicit Retry.
- Completion freezes effective occurrences/time before FIFO prepare/commit;
  duplicate/retry/reversal cannot reinterpret history. Positive-only toast and
  finish/confetti remain shared; explicit finished-book open resets to page 1.
- Imports stay atomic/deduplicated; summaries return Library without opening.
  Deletion retains global vocabulary/history/rules; reimport starts fresh.
- Time uses elapsedRealtime, 15s saves, five-minute idle/background cap and exact
  frozen/coalesced retries. Final samples and both time slots drain before movement,
  completion/exit/native close. Killing a process can lose pending in-memory work.
- TXT/SRT preserve original bytes/encoding; EPUB preserves ordered canonical
  chapters/styles/ruby/assets/B without reparsing on load. Images remain bounded
  FIFO asset reads; no BLOBs in Crux state. Strict Unicode spans fail to plain text.
- Library query/sort/rename/deletion drafts and five-status bars retain shared gates.
  Bars resolve effective membership read-only; no dictionaries, registrations or
  persistent count cache. Stored/live statuses and Missing/zero remain distinct.

## Next tasks (ordered)
1. Measure actual effective JNI/device/diverse/image-heavy costs. Host baselines
   and reproducible scratch TXT/EPUB harness: EFFECTIVE_READER_PERFORMANCE.md.
2. Phase 5: audit immutable event/provenance versions and migration tests; verify
   deleted documents retain historical source metadata.
3. Add read-only lifetime occurrence/time aggregation with repetition/reread,
   incomplete-page exclusion and duplicate-completion coverage.
Physical arm64 remains deferred; Nostr configuration empty; Q188 is required
before forwarding. These do not block local reader/history work.

## Verification / blockers
- PASS this slice: final 23-case API-36 x86_64 phone/light selection, including all
  four long-page cases, two new Unicode/bidi geometry guards and span/status/
  navigation/typography/formatting/images/furigana. Both-ABI APKs (unchanged native
  binaries), strict Android lint/Kotlin formatting/diff pass. No wire/schema change.
- PASS: nine wide/dark cases (2560x1600, 240 dpi): both geometry guards, five spans,
  actual status persistence/Retry and long-page word dismissal/navigation. Restored
  phone size/density/light and stopped this task's emulator.
- Prior 710 Rust/17 ignored and full real-Full host JNI/codecs are the preceding
  slice's baseline, not rerun for this Kotlin-only change. No full-matrix claim.
  Separate Full-loader matrix, release/process death, physical arm64 and actual
  effective device performance remain unmeasured/unrerun. Dense bidi fragments
  retain complete layouts; no scaling claim for that correctness fallback.
- Low AVD space required removing only rebuildable immutable dictionary extractions
  before APK installs; reader DB/documents/settings preserved. Use direct ADB
  instrumentation against installed APKs. No debugger port remains forwarded.
  Keep native/BoltFFI generation sequential with host JNI execution.
- Commands/results/failures: DEVELOPMENT.md, final long-page section; logs
  `/tmp/yomoku-long-page-*`. Normal AVD uses swangle with `-feature -Vulkan`.
