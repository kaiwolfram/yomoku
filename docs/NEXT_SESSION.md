# Resume here
Updated: 2026-10-29. Production effective reader activated; boundary editing is live.

## Current slice
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
1. Investigate the emulator long-page HWUI stall and rerun the four unresolved
   cases: ReaderWords movingBothDirectionsDismissesDetailsAndNeverRestoresTheSelection;
   ReaderNavigation controlsAndSwipesPersistBothDirectionsResetScrollAndCountOnlyForward,
   failedForwardDisablesControlsAndRetryCommitsExactlyOnce, and
   finishCheckmarkPersistsThroughRetryRecreationAndRereadsWhileReopeningResetsPosition.
   Previous ReaderScreen also fails; JDB main waits in HardwareRenderer draw.
2. Measure actual effective JNI/device/diverse/image-heavy costs. Host baselines
   and reproducible scratch TXT/EPUB harness: EFFECTIVE_READER_PERFORMANCE.md.
3. Continue phase 5 with the immutable event/provenance audit and lifetime
   occurrence/time aggregation; keep status/history semantics versioned.
Physical arm64 remains deferred; Nostr configuration empty; Q188 is required
before forwarding. These do not block local reader/history work.

## Verification / blockers
- PASS: 710 Rust (17 opt-in ignored), strict Clippy/fmt/diff, full final real-Full
  host JNI/codecs, unchanged 17 generated Kotlin files, both-ABI APKs/Android lint,
  Android/new host-test formatting. No dependency/wire/durable schema change.
- PASS: 57 distinct phone cases across runs; final 19-case selection passes all
  four corrected failures and the 12 compact-word guards. Three final wide/dark
  cases pass (2560x1600, 240 dpi). Phone/light restored and task emulator stopped.
- The four long-page cases above remain unverified this slice; no full-device-matrix
  claim. Separate Full owned/mapped matrix, release/process death, physical arm64
  and actual effective device performance were not rerun/measured.
- Initial System UI ANR/display overrides required reset/reboot. Reinstalls can
  exhaust the 6 GiB AVD after dictionary extraction. Final checks use direct ADB
  instrumentation against installed APKs; only this task's test app was removed
  to free space. Keep native/BoltFFI generation sequential with host JNI execution.
- Commands/results/failures/logs: DEVELOPMENT.md, final activation section;
  `/tmp/yomoku-activation-*`. Normal AVD uses swangle with `-feature -Vulkan`.
