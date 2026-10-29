# Resume here
Updated: 2026-10-29. Phase-5 historical event/provenance audit verified.

## Current slice
- Phase 5 schema audit is complete with the existing schema 0019: dated v1
  completion/status/analysis headers and independent v1 effective reservations.
  Occurrences inherit the analyzed command contract; reservation presence selects
  effective indices. Repository APIs never edit historical rows.
- Every history-bearing schema (5–19) upgrades with historical values and ID
  high-water marks intact. Late failures from 9/10/13/17/18 roll back byte-identically.
  Deleted documents retain joinable original source titles/kinds/chapter order.
  Real completed/unused effective JSON survives upgrade/deletion unchanged;
  unused reservations never replay/count. See DATA_MODEL.md historical contract.
- Production reader/Library/completion use native effective A/B/C/manual partitions;
  original B is immutable and supplies editor anchors. Edits expose explicit Reload;
  accepted exact Retry/Cancel obligations survive dismissal/invalidation.
- Completion freezes effective occurrences/time before FIFO prepare/commit. Partial
  time saves every 15s with a five-minute idle/background cap and drains before
  movement/exit/close. Process death can lose pending in-memory work.
- Imports remain atomic/deduplicated and return Library summaries without opening.
  Deletion retains global vocabulary/history/rules; reimport starts fresh.
- Android FileImportBytes owns a primitive buffer behind the typed byte list; the
  real 17.9 MB EPUB now imports without the former boxed-list heap exhaustion.
  Five-sample debug JNI measurements/method/CSV: EFFECTIVE_READER_PERFORMANCE.md.
  Long-page rendering regressions pass; optimized/frame-time/peak-memory and
  physical-arm64 claims remain unverified.

## Next tasks (ordered)
1. Add read-only lifetime occurrence/time aggregation with repetition/reread,
   incomplete-page exclusion and duplicate completion coverage. Start with the
   historical contract in DATA_MODEL.md and storage encounters/completion tests.
2. Add status timeline aggregation with registration jumps and Known dictionary-form
   counts, spelling identity, unresolved exclusion and vocabulary-deletion recalculation.
3. Profile optimized Android repository/encoding/callback costs before a performance
   change; extend diverse many-book/peak-memory/actual Compose measurements.
Nostr configuration stays empty; ask Q188 before forwarding.

## Verification / blockers
- PASS: 43 focused history tests; 713 workspace Rust tests, 17 opt-in ignored;
  strict Clippy/Rustfmt/diff. No production code/schema/FFI/Kotlin changed.
- Commands (Rust PATH workaround from DEVELOPMENT.md):
  cargo test -p yomoku-storage history --locked --offline
  cargo test --workspace --all-features --locked --offline --quiet
  cargo clippy --workspace --all-targets --all-features --locked --offline -- -D warnings
  cargo fmt --all -- --check
  git diff --check
- No failed checks or implementation blocker. Logs: /tmp/yomoku-history-schema-*.log.
  No Android/JNI/codegen/Full-loader/release/process-death/physical-arm64 rerun.
  Keep native/BoltFFI generation sequential with host JNI execution.
