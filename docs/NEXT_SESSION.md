# Resume here
Updated: 2026-10-31. Pure retained Anki export commands/results verified.
Commit guidance: explanatory bodies; merges include provenance. Keep verification
results in session/progress docs and reports, out of commit message bodies.

## Current slice
- `AnkiExportSession` accepts current original/effective selections through the
  existing canonical note constructors; freezes note, selection and external deck.
  Checked attempts/owning allocation reject stale/repeated/other-session replies.
- Known pre-insert failures permit explicit Retry with identical fields/deck.
  Uncertain insertion only permits read-only Reconcile; empty/failed queries or
  wrong result kinds cannot authorize another insert. Clear/replacement remains
  blocked until verified existing-note IDs resolve the obligation.
- Typed positive signed-64 deck/note IDs; nonempty exact-identity existing-note
  observations retain multiple matches. No vocabulary/history mutation or I/O.
- Pure in-memory prerequisite: not yet owned by app Model, Event/FFI or Android.
  No settings/UI action. Durable pending/uncertain insert recovery is required
  before enabling export; restarting an empty session cannot establish safe Retry.
- Prior API evidence: ANKIDROID_API.md (AnkiDroid 2.25.3). No API dependency/minimum
  app version adopted; no provider installed/read or note exported.
- Schema 0020. Nostr starter configuration empty; ask Q188 before forwarding.

## Next tasks (ordered)
1. Implement Android ContentResolver adapter and fake-provider contract tests:
   availability/spec/permission, deck/model layout, conservative exact-surface
   duplicates across models/decks, pre-insert recheck, uncertain outcomes/read-only
   reconciliation and supported open-link resolution. Use the typed core contract;
   null/exception/transport loss after submission must not become NotInserted.
2. Add app-owned retained export state, typed Event/FFI/provider bridge and durable
   pending/uncertain insertion recovery across process death before enabling Add
   to Anki/default-deck settings/existing-note action. Reject stale shell selections;
   never replay a mutating request on restart. Test a disposable real collection
   before claiming runtime compatibility. No provider completion fence is known
   to permit safe insertion after empty reconciliation.
3. Resume actual Android native-phase/100k Select All Compose and observer-free
   optimized profiling, plus remaining accessibility/process-death matrix. Preserve
   full IDs/equality/replay guards. Host phases cannot explain device costs;
   heavy-history/diverse-book/full-load/exact peaks remain. Physical arm64 is
   user-deferred; emulator/host results do not establish hardware viability.

## Verification / blockers
- PASS: workspace build; 851 workspace tests/17 ignored data-dependent fixtures;
  strict workspace/all-target/all-feature Clippy; Rustfmt and diff check.
- Ten new core cases cover original/effective frozen payloads, failures/Retry,
  busy/clear/replacement, uncertain/empty/failed reconciliation, wrong result kind,
  stale/repeated/foreign replies, stale selection, long IDs/multiple matches and
  unchanged status/encounters/reader/dictionary inputs. No blocker or failed check.
- Commands: DEVELOPMENT.md, final retained Anki export section. Pinned Rust PATH;
  cargo test --workspace --all-features --locked --offline; cargo clippy
  --workspace --all-targets --all-features --locked --offline -- -D warnings.
- Logs: /tmp/yomoku-anki-export-{workspace,build,clippy}.log. Android/Kotlin/JNI/
  codegen/Full-loader/device/profile checks were not run for this pure-core slice.
  No dependency/schema/protocol change. No emulator started.
- Prior native diagnostics/results: VOCABULARY_PERFORMANCE.md final section and
  VOCABULARY_ANDROID_PERFORMANCE.md. Default-off storage-measurements factory
  remains host-only; Android native-phase/observer-free optimized comparisons open.
