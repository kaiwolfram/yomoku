# Resume here
Updated: 2026-10-29. Android large-file import heap fix and JNI measurements verified.

## Current slice
- URI imports and the opt-in benchmark use FileImportBytes: a read-only typed list
  owning the private ByteArray. The former eager UByte list exhausted the 192 MiB
  Java heap on the real 17.9 MB EPUB. Literal bytes/encoding/retries stay identical;
  no new dependency, wire/schema, size limit or data-destructive semantics.
- EffectiveReaderPerformanceTest uses one disposable database per input, actual
  CoreSession/JNI/FIFO/codecs/providers, no Activity or reader clock. Fresh revisions,
  unchanged read-only DB hashes, zero reading history/time and handle cleanup pass.
- Five-sample debug Android measurements cover the real 329,187-character TXT,
  16-chapter/96,296-character EPUB and a 12-asset image page. CSV, exact method,
  corpora/hashes and commands: EFFECTIVE_READER_PERFORMANCE.md.
- Debug page medians B-only→largest-chapter A/B/C: TXT 903→2057 ms; EPUB 770→826 ms.
  Twelve-image batch ~251–254 ms. EPUB import now succeeds (20.3 s total, 2.27 s
  dispatch); no optimized-build/responsiveness or peak-memory claim.
- Picker/notification fixtures await automatic Library counts before commands.
  Mixed-picker SRT content is unique per run, preserving intentional duplicate tests.
- Production effective reader, word/status/custom/furigana/Library/completion share
  native partitions. Original B remains immutable and supplies editor anchors.
  A/B/C and manual All/default versus This occurrence edits/reversal are durable.
  Success retires the page and exposes explicit Reload; accepted exact Retry/Cancel
  obligations survive dismissal/invalidation. No full chapter keys cross JNI.
- Long LTR interactive text is bounded at measured soft-line ends with static word
  spans. Unicode/wrapping/spacing/crossing selections pass; bidi/paragraph styles
  retain complete layouts. The former four long-page regressions were resolved.
- Completion freezes effective occurrences/time before FIFO prepare/commit; duplicate,
  retry/reversal cannot reinterpret history. Partial time uses elapsedRealtime,
  15s saves and five-minute idle/background cap, draining before movement/exit/close.
  Process death can lose pending in-memory work. Finished-book open resets to page 1.
- Imports remain atomic/deduplicated and return Library summaries without opening.
  Deletion keeps global vocabulary/history/rules; reimport starts fresh. Canonical
  EPUB/ruby/styles/assets/B load without reparsing; asset BLOBs stay outside core.
- Library query/sort/rename/deletion and five-status bars keep shared gates; effective
  membership reads never load dictionaries/register words or persist count caches.

## Next tasks (ordered)
1. Phase 5: audit immutable event/provenance versions and migration tests; verify
   deleted documents retain historical source metadata. Start with DATA_MODEL.md,
   storage migrations/completions/encounters/vocabulary and effective completion tests.
2. Add read-only lifetime occurrence/time aggregation with repetition/reread,
   incomplete-page exclusion and duplicate-completion coverage.
3. Profile optimized Android repository/encoding/callback costs before a performance
   change; extend diverse many-book/peak-memory/actual Compose measurements.
Physical arm64 remains deferred; Nostr configuration empty; ask Q188 before forwarding.

## Verification / blockers
- PASS: both-ABI APKs with unchanged native output, strict Android lint, Kotlin
  formatting/diff; 18 MiB exact-byte/repeated-codec test and 11 other distinct phone
  import regressions, default benchmark skip and final isolated five-sample benchmark.
  Mixed fixture passes its focused final run; earlier selections had corrected
  Library-race/constant-SRT failures. See DEVELOPMENT.md final section for commands.
- No Rust/host-JNI/generation/Full-loader matrix rerun for this Kotlin-only change.
  Release/process-death, full UI matrix, optimized-device and physical-arm64 work
  remain unrun. Earlier native baseline: 710 Rust/17 opt-in ignored.
- Low AVD space required removing only rebuildable Full/index extractions for APK
  install; readers/settings/history preserved and providers republished dictionaries.
  Corpus copies and this task's emulator are cleaned up; phone/light baseline restored.
- No implementation blocker. Logs: /tmp/yomoku-effective-measure-*; source corpora
  remain under /tmp. Keep native/BoltFFI generation sequential with host JNI tests.
