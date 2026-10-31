# Resume here
Updated: 2026-10-31. First MVP excludes Nostr and Known forms statistics.
Commit guidance: explanatory bodies; merges include provenance. Keep verification
results in progress docs/reports, out of commit message bodies.

## User checkpoints
- Announce when no plan steps remain; ask about product ambiguities. Recommend
  physical testing when it provides more value than the next implementation step.
- User's phone trial was acceptable; detailed device/workflow validation remains
  unchecked. Small polish can wait while the implementation plan continues.
- Feed is only `Coming Soon! 🌱`; Nostr/identity/Q188 are beyond the first MVP.
- Known forms chart/aggregate removed. Keep Known in the five-status chart.

## Current slice
- Schema 0023 drops only vocabulary_dictionary_forms; no current writer/reader
  uses it. Historical migrations remain intact. Vocabulary, status/occurrence/
  completion history, token dictionary metadata and Anki journals remain intact.
- Statistics now has cumulative occurrences and five-status charts. Native bucket
  payloads/Kotlin codecs and the opt-in profiler no longer carry form aggregates.
- Migration cases preserve all other tables on a populated schema-22 upgrade;
  failure after DROP rolls back bytes/history and permits exact retry. Historical
  schema upgrades, imports, original/effective completions and deletion pass.

## Anki continuation (unchanged)
- Native BeginAnkiRecoveryRead(Note|Model) captures both restored journals before
  dispatch. One immutable shared-allocation handle identifies app owner/attempt;
  equal IDs/payloads cannot recreate it. Callback must return the same handle,
  matching target and unchanged pair. Foreign/stale/duplicate callbacks cannot
  consume a newer read; matching invalid evidence records a separate read error.
- Accepted evidence consumes the handle and enters FIFO CommitAnkiRecovery with
  the captured pair. No provider effects are emitted yet. Direct evidence commits
  cannot compete with an active read. Native accessors are for a future coordinator,
  not polling; no shell codec/ViewModel/Android/FFI read call site is enabled.
- Explicit Retry invalidates a lost read and restores BOTH journals read-only.
  Pending commits cannot be abandoned. Failed/lost commit responses require restore
  plus fresh observations; never replay evidence or provider mutations.
- Native PrepareAnkiExport authenticates the current visible original/effective
  selection and freezes fields/surface/sentence/deck/full layout. Reader busy,
  checkpoint/completion/departure/mutation gates apply; both journals must be
  restored empty without pending/failed commits. Clear drops only unsubmitted
  intent. Restart retains neither prepared intent nor read handles.
- Initial Library refresh schedules restore before List on the existing FIFO.
  Recovery commits compare both journals in one immediate transaction. Both begin
  methods gate BOTH journals and commit before one live invocation; only a newly
  committed preparation grants private live tokens. Recovery IDs never grant writes.
- Note IDs prove existence, not deck placement. Model evidence requires exact full
  layout and original ID when known. Empty/failed/partial/wrong reads retain doubt.
  Terminal journals survive deletion. No replay/repair/rename/delete/cleanup or
  automatic retries. Isolated AnkiExportSession/CheckAndAdd remains unowned.
- API evidence: ANKIDROID_API.md, pinned 2.25.3. Isolated worker adapters have no
  app call site or real-provider claim; no Anki dependency/minimum adopted.

## Next tasks (ordered)
1. Recommend a physical-phone upgrade trial against the existing Library/history
   before further integration work; rebuild an optimized APK when requested.
2. Connect native read handles/prepared intent to typed Android/FFI read-only
   dispatch/preflight, then durable begin/outcome commits before one invocation.
   Validate provider/permission/deck/model failures and death/stale/lost callbacks.
3. Add recovery/error/explicit Retry projection, deck settings, Add to Anki and
   existing-note actions after gates; validate a disposable real collection/browser.

## Verification / blockers
- PASS: 916 workspace tests/17 ignored opt-in fixtures, three profiling-harness
  cases, focused status-query check, strict Clippy, Rustfmt, Kotlin formatting,
  full host JNI/Unicode suite; Kotlin regeneration identical for host/Android.
- PASS: debug/test APKs, strict lintDebug; statistics/navigation 9/9 on phone
  and 9/9 on tablet-sized API-36 x86_64. Display restored; no remaining request work.
- No open blocker. Release/physical trials and performance/accessibility matrix
  not rerun. Commands/results: DEVELOPMENT.md “Remove Known forms” section;
  logs: /tmp/yomoku-remove-known-*. Older phone APK remains source 0374365.
