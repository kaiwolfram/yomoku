# Resume here
Updated: 2026-10-31. Anki API evidence and pure note preparation verified.
Commit guidance: explanatory bodies; merges include provenance. Keep verification
results in session/progress docs and reports, out of commit message bodies.

## Current slice
- Reviewed official AnkiDroid 2.25.3 wrapper/contract/provider/manifest/handler;
  exact methods, permission/spec gates, cross-model first-field limits, ambiguous
  insertion Retry and browser-link candidate are in ANKIDROID_API.md.
- No API dependency or minimum app version adopted; no provider installed/read
  or note exported. API wiki's old artifact must not be assumed current.
- Pure core anki::AnkiNote prepares four escaped fields and retains raw Surface.
  Original Yomoku v1 single-card HTML/CSS: word-only front; reading/ordered English
  meanings/supplied canonical sentence with one selected occurrence on the back.
  Validate UTF-8 target and exact surface; escape HTML/media brackets/U+001F.
  Preserve Unicode/whitespace/inflection; absent readings/meanings stay empty.
- No state mutation, Event/FFI, provider I/O, settings or UI action yet. Callers
  still resolve sentence boundaries, current selection and applicable dictionary/
  custom data; the builder does not invent those policies. Architecture/decisions
  and implementation checklist updated; full Add to Anki remains unchecked.
- Schema 0020. Nostr starter configuration empty; ask Q188 before forwarding.

## Next tasks (ordered)
1. Project canonical sentence/selected target and applicable readings/English
   meanings from original/effective dictionary selections; add retained typed
   Anki export commands/results with stale-selection/unchanged-status tests.
2. Implement Android ContentResolver adapter and fake-provider contract tests:
   availability/spec/permission, deck/model validation, conservative exact-surface
   duplicates, insert ambiguity and supported open-link resolution. Wire default
   deck/settings/Add to Anki only with verified results; test a disposable real
   collection before claiming runtime compatibility. See ANKIDROID_API.md.
3. Resume actual Android native-phase/100k Select All Compose and observer-free
   optimized profiling, plus remaining accessibility/process-death matrix. Preserve
   full IDs/equality/replay guards. Host phases cannot explain device costs;
   heavy-history/diverse-book/full-load/exact peaks remain. Physical arm64 is
   user-deferred; emulator/host results do not establish hardware viability.

## Verification / blockers
- PASS: workspace build; 831 workspace tests/17 ignored data-dependent fixtures;
  strict workspace/all-target/all-feature Clippy; Rustfmt and diff check.
- Five new core cases cover repeated-word occurrence highlighting, literal
  HTML/media/separator content, escaped sentence context, exact Unicode/whitespace/
  inflection identity/absent definitions and invalid/stale/non-scalar ranges.
- Commands: DEVELOPMENT.md, final Anki note preparation section. Pinned Rust PATH;
  cargo test --workspace --all-features --locked --offline; cargo clippy
  --workspace --all-targets --all-features --locked --offline -- -D warnings.
- Logs: /tmp/yomoku-anki-{workspace,build,clippy}.log. No failed check or blocker.
  Android/Kotlin/JNI/codegen/Full-loader/device/profile checks were not run for
  this pure-core/doc slice. No dependency/schema/protocol/product behavior change.
- Prior native diagnostics/results: VOCABULARY_PERFORMANCE.md final section and
  VOCABULARY_ANDROID_PERFORMANCE.md; DEVELOPMENT.md retains commands. Default-off
  storage-measurements factory remains host-only; Android native-phase and
  observer-free optimized comparisons are still open. No emulator started here.
