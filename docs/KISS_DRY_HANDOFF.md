# KISS/DRY improvements handoff

## Scope and review status

Prepared from the investigation on 2026-11-15, at revision `5df5a6d`.
The user requested investigation and handoffs; no application refactoring has
been performed. These are design recommendations, not five additional proven
bugs. The global `/home/kai/.codex/AGENTS.md` already contains DRY and received
this KISS rule during the investigation:

> Follow KISS: Choose the simplest design that meets the current requirements.
> Keep control flow and state easy to understand; avoid unnecessary abstractions,
> configuration, and special cases.

Read [AGENTS.md](../AGENTS.md), the relevant [PRODUCT_SPEC.md](PRODUCT_SPEC.md)
sections, [ARCHITECTURE.md](ARCHITECTURE.md), and [DEVELOPMENT.md](DEVELOPMENT.md).
Keep future work incremental and behavior-preserving. Maintain Kotlin/Compose,
pure Rust Crux, separate SQLite/runtime I/O, and a narrow typed FFI. Do not add
deferred features, change data-destructive semantics, or replace the architecture
as a cleanup. Follow the repository's automatic-commit policy for verified steps.

## 1. Make workflow phases carry their own data

[FileImports.kt](../android/app/src/main/java/app/yomoku/FileImports.kt) coordinates
`active`, `choosingEncoding`, `cancelling`, `waitingForCore`,
`cancellationReturned`, nullable input/error/result fields, and generation state.
These fields must agree for every callback and transition. Consider a Kotlin
sealed state with alternatives such as Reading, ChoosingEncoding, Importing,
Cancelling, and Finished, with the required payload owned by each alternative.
Keep retained input for encoding retries and the distinction between acquisition
cancellation and native cancellation explicit.

Rust's [CompletionState](../crates/yomoku-core/src/effective_session/completions.rs)
similarly has independent pending action, in-flight request and error options.
Consider an enum for idle, dispatchable, in-flight and failed phases, carrying
the existing action/request/error. Preserve the exact frozen command and attempt
identity across retry and page invalidation; failed completion cannot simply be
discarded because a write may have committed before a follow-up read failed.

Do not replace every boolean with an enum. Use alternatives where fields describe
mutually exclusive phases; keep independent facts independent.

## 2. Give the shared storage slot one typed owner

[app.rs](../crates/yomoku-core/src/app.rs) stores `storage_busy` alongside pending
reader/delete/rename state and effective, statistics, vocabulary, and word request
fields. Generic `StorageFinished` handling inspects several unrelated pending
fields before releasing the slot. Dispatch guards repeat ownership checks.

Consider one pending-operation enum for the mutually exclusive shared slot, with
the captured request carried by its variant and busy state derived from it.
Keep callback attempt matching and stale-response rejection explicit. Do not
accidentally fold the intentionally independent Anki-settings and Statistics
preference slots into this enum: they have separate ownership while sharing the
runtime FIFO. Audit actual concurrency before deciding which fields belong.

## 3. Consolidate navigation and reader exit cleanup

[app.rs](../crates/yomoku-core/src/app.rs) represents navigation with destination,
`settings_open`, `ContentRoute`, and optional reader/loading fields. Select
destination, open paste, import completion, and close document repeat similar
cleanup. Kotlin [YomokuApp.kt](../android/app/src/main/java/app/yomoku/YomokuApp.kt)
then reconstructs the visible destination from these facts.

First extract genuinely shared exit/cleanup behavior. Assess whether a route enum
carrying its reader or paste state can then eliminate contradictory combinations
and simplify exhaustive view projection. Preserve system Back, four main
navigation choices, reader typography popup behavior, and any intentional return
state. Deferred navigation must still wait for accepted reader mutations and
exact completion retries; clearing a visible reader must not clear an obligation.

## 4. Remove unsupported segmentation alternatives

[segmentation.rs](../crates/yomoku-core/src/segmentation.rs) projects stored Mode C
tokens into `ResolvedSegment` and `SegmentMetadata`. The production constructor
always supplies `original_token_index: Some(...)` and metadata `Some(...)`, with
index/identity information repeated across layers. The representation retains
alternatives associated with removed manual boundary editing.

Audit constructors, persisted payloads and tests before simplifying. Make data
mandatory where the current supported model guarantees it, and reduce redundant
projections/clones. Keep genuinely optional analyzer readings/base forms, explicit
missing-analysis state, Japanese eligibility, and old supported import profiles.
Retain compact revision/selection identities, UTF-8 anchors, crossing-token
selection and start-owned completion counting. Do not remove stale-callback
guards merely because a representation becomes simpler. Avoid a schema or
persisted-format change unless required and explicitly assessed.

## 5. Share dictionary-component ordering and deduplication

[Vocabulary form analysis](../crates/yomoku-runtime/src/imports.rs) and
[reader component collection](../crates/yomoku-storage/src/reader_words.rs) both
filter Japanese tokens, sort longer surfaces before shorter ones with source
position breaking ties, and deduplicate surface/base forms.

Extract only the common pure ordering/deduplication behavior into an appropriate
existing core module. Keep I/O and analysis loading in their current owners.
Reader components need original anchors for Anki sentence highlights; Vocabulary
uses a temporary analyzed surface and exports an empty sentence. Preserve those
differences, full-token-first dictionary ordering, and exact written identity.
Avoid a generic framework for two small callers.

## Verification and suggested sequencing

Address the demonstrated defects in their own handoffs before, or separately
from, broader refactoring:

- [Issue 1: large-import memory](IMPORT_MEMORY_HANDOFF.md).
- [Issue 2: reader EPUB I/O](READER_EPUB_IO_HANDOFF.md).
- [Issue 3: import stream cancellation](IMPORT_CANCELLATION_HANDOFF.md).

Issue 1 and issue 3 share the acquisition helper; leave workflow-wide sealed-state
changes here unless a small part is required for a bug fix. Issue 2 owns binary
I/O reduction; this handoff owns segmentation representation cleanup. Reinspect
the latest code after each fix to avoid restoring or duplicating old logic.

Implement one focused refactor at a time. Protect observable behavior and actual
invalid transitions rather than writing tests that just restate enum variants.
Prioritize failed write -> navigation -> retry/cancel, stale callback after
dismissal/reentry, cancellation during provider work, and repeated completion
actions. Preserve the existing strong Anki request/invocation ownership model.

The investigation ran the following baseline successfully: 57 tests across these
four integration targets. This is not verification of any future refactor:

```sh
cargo test -p yomoku-core --test effective_app --test effective_session --test vocabulary_transport --test statistics_transport --locked --offline
```

Add affected storage/runtime/Anki suites, rustfmt/Clippy, Kotlin formatting and
focused instrumentation as appropriate. For changed Rust event/view contracts,
regenerate ignored Kotlin payloads, run Kotlin/JNI protocol checks, and build the
affected Android variants. Sequence standalone Cargo and JNI generation as
DEVELOPMENT.md requires. Report the checks actually run, each commit, and remaining
risks. The investigation's host probes did not establish device compatibility.
