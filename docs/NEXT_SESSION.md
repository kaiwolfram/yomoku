# Resume here
Updated: 2026-10-31. Anki selected context/dictionary projection verified.
Commit guidance: explanatory bodies; merges include provenance. Keep verification
results in session/progress docs and reports, out of commit message bodies.

## Current slice
- AnkiNote::from_original_selection/from_effective_selection prepare a note from
  the owner's current page/selection and loaded dictionary details. Validate
  page/base/revision, whole token metadata, canonical ownership/exact target and
  clipped display ranges; allow live vocabulary-only observations to differ.
- Sentence context uses full chapter text, pagination punctuation/closing-quote
  rules and CRLF/CR/LF. Keep whitespace, full cross-page targets and one selected
  occurrence; ignore boundaries inside a manually merged target.
- Project licensed readings and English senses/name groups in source order;
  deduplicate readings/shared form senses, retain translation qualifiers. Custom
  entry only without built-in matches; selected analyzer reading is last fallback.
  Recompute matches before indexing; reject unrelated/corrupt dictionary data.
- Pure, read-only prerequisite only: no retained export state, Event/FFI, provider
  adapter, settings or UI action. Android compatibility remains unverified.
- Prior API evidence: ANKIDROID_API.md (AnkiDroid 2.25.3). No API dependency or
  minimum app version adopted; no provider installed/read or note exported.
- Schema 0020. Nostr starter configuration empty; ask Q188 before forwarding.

## Next tasks (ordered)
1. Add retained typed Anki export commands/results around prepared notes and the
   current original/effective selection. Preserve frozen payloads across callbacks;
   test stale selection/results and unchanged vocabulary/reading history. Account
   for ambiguous insert outcomes before allowing a retry; see ANKIDROID_API.md.
2. Implement Android ContentResolver adapter and fake-provider contract tests:
   availability/spec/permission, deck/model validation, conservative exact-surface
   duplicates, insert ambiguity and supported open-link resolution. Wire default
   deck/settings/Add to Anki only with verified results; test a disposable real
   collection before claiming runtime compatibility.
3. Resume actual Android native-phase/100k Select All Compose and observer-free
   optimized profiling, plus remaining accessibility/process-death matrix. Preserve
   full IDs/equality/replay guards. Host phases cannot explain device costs;
   heavy-history/diverse-book/full-load/exact peaks remain. Physical arm64 is
   user-deferred; emulator/host results do not establish hardware viability.

## Verification / blockers
- PASS: workspace build; 841 workspace tests/17 ignored data-dependent fixtures;
  strict workspace/all-target/all-feature Clippy; Rustfmt and diff check.
- Ten new core cases cover original/effective parity, licensed readings/English
  senses/names, surface/base unions, inflections, cross-page words, hard lines/
  quotes/decimals, literal custom/manual/absent data, corrupt/stale results and
  unchanged vocabulary statuses/deleted entries. No failed check or blocker.
- Commands: DEVELOPMENT.md, final Anki selected context section. Pinned Rust PATH;
  cargo test --workspace --all-features --locked --offline; cargo clippy
  --workspace --all-targets --all-features --locked --offline -- -D warnings.
- Logs: /tmp/yomoku-anki-selection-{workspace,build,clippy}.log. Android/Kotlin/JNI/
  codegen/Full-loader/device/profile checks were not run for this pure-core slice.
  No dependency/schema/protocol change. No emulator started.
- Prior native diagnostics/results: VOCABULARY_PERFORMANCE.md final section and
  VOCABULARY_ANDROID_PERFORMANCE.md. Default-off storage-measurements factory
  remains host-only; Android native-phase/observer-free optimized comparisons open.
