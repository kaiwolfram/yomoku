# Resume here
Updated: 2026-10-31. First MVP excludes Nostr and Known forms statistics.
Commit guidance: explanatory bodies; merges include provenance. Keep verification
results in progress docs/reports, out of commit message bodies.

## User checkpoints
- Announce when no plan steps remain; ask about product ambiguities. Recommend
  physical testing when it provides more value than the next implementation step.
- User's phone trial was acceptable; detailed device/workflow validation remains
  unchecked. Recommend a schema-23 upgrade trial with existing Library/history
  before enabling live integration; rebuild optimized APK when requested.
- Feed is only `Coming Soon! 🌱`; Nostr/identity/Q188 are beyond the first MVP.
- Statistics retains cumulative occurrences and all five statuses including Known.
  Schema 0023 drops only unused vocabulary_dictionary_forms; all other history,
  vocabulary, token dictionary metadata and Anki journals are preserved.

## Current native Anki live ownership
- BeginAnkiLive requires frozen current-owner preparation, successful matching
  preflight and both empty restored journals with no recovery read/commit/error.
  Existing duplicates/conflicts/partial reads cannot begin. No public event codec.
- Existing SQLite FIFO retains private AnkiLiveAttempt after atomic begin COMMIT.
  Only matching validated begin acknowledgement changes app state to Ready.
  Model::take_anki_live_invocation consumes Ready once into a move-only permit;
  consuming the permit creates the native callback with exact frozen intent.
- App stages reject foreign/stale/duplicate/wrong-kind callbacks. Worker consumes
  its token even on outcome failure; exact committed snapshot acknowledgement
  updates recovery. Uncertain payload/known IDs persist. Reader busy/errors stay
  independent. Weak request identities prevent replay without retaining payloads.
- Native Abandon invalidates lost callbacks; explicit Retry restores read-only,
  drops live/prepared/preflight state and invalidates worker tokens even on failed
  reads. Restart restores data only. Clear cannot erase active/failed live work;
  clearing a finished presentation preserves durable obligations.
- Pure intent/outcome types moved to core with storage reexports. No new SQL in
  core, schema/dependency/FFI/Kotlin change or enabled provider mutation.
- IMPORTANT continuation: the permit has NO production consumer. Wire it once
  through typed provider dispatch with CoreSession worker/main/close ownership;
  these native tests do not establish Android close/invocation behavior.

## Retained read-only prerequisites
- Native preparation authenticates visible original/effective selection and
  freezes fields/surface/sentence/deck/full layout; reader/time/mutation gates apply.
- Typed preflight/recovery use CoreSession's separate provider worker/main/close
  gates, retain native handles and discard partial/failed evidence. Retry reads
  fresh evidence; foreign/stale/late/duplicate callbacks cannot cross owners.
- Read-only recovery evidence commits use the FIFO; snapshots/IDs never grant
  writes. Public events precede skipped callbacks to align Rust/Kotlin indices.
- ANKIDROID_API.md pins 2.25.3 evidence. Isolated mutating adapters still have no
  app call site or real-provider claim; no API dependency/minimum adopted.

## Next tasks (ordered)
1. Consume the native live permit through typed FFI/Android provider invocation.
   Enforce one invocation and worker/main/close/session gates; refresh provider,
   permission/spec, frozen deck/layout and exact duplicate checks immediately.
   Route every outcome through FinishAnkiLive; lost responses restore read-only.
2. Add recovery/error/explicit Retry projection, deck settings and Add/existing
   note actions. Validate a disposable real AnkiDroid collection/browser.
3. Encrypted-backup format/library review follows completed Anki integration.

## Verification / blockers
- PASS: 20 new cases (12 core/8 runtime); 137 focused Anki cases; 967 workspace
  cases/17 ignored; build, strict Clippy, Rustfmt, identical Kotlin generation
  and diff. Commands/logs: DEVELOPMENT.md, Native app/FIFO live Anki ownership.
- Initial runtime test harness assumed a render callback for native Command::done;
  fixed its completion fence. All final checks pass; no open blocker or remaining
  implementation/verification work for this native lifecycle slice.
- JNI, Android APK/lint/emulator, real-provider, physical and performance trials
  were not rerun. Physical schema-23 upgrade and broad device matrix remain open.
  Older phone APK is source 0374365; no release requested.
