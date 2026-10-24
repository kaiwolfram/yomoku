# Implementation checklist

Source: [Product specification](PRODUCT_SPEC.md). Check an item only when its stated
verification passes. Paths below are implementation targets, not claims that the
modules already exist. Each phase builds on preceding foundations except the
small pure-domain task explicitly pulled forward from phase 3.

Implementation strategy: prefer a working vertical slice before adding every
import format or external integration. The first reader milestone is paste text
→ segment → display pages → tap word → set status → advance page → persist
position/encounter history. Keep incremental builds green.

## 0 — Repository and continuity
- [x] Inspect repository, git history/status and available CLI toolchains; preserve handoff. Result: no existing source or commits; see DEVELOPMENT.md.
- [x] Create AGENTS.md and persistent plan, resume, decisions, architecture and data-model documents; inspect files for consistency.
- [x] Establish a passing host Rust baseline with exact commands and document Android/toolchain limitations.
- [x] Record missing curated Nostr data and Q188 without blocking local work.
- [x] Consolidate the product source of truth in `docs/PRODUCT_SPEC.md`, retain acceptance coverage and update references; verify requirements against the original and check local documentation links.

## 1 — Minimal Android/Rust bridge
- [x] Create root Cargo workspace and pure `crates/yomoku-core`; verify offline build, test, fmt and clippy.
- [x] Verify current Crux APIs/version and supported Kotlin bridge approach; record versions and evidence in DECISIONS.md before adding dependencies.
- [x] Add Crux model, typed event and view model to `crates/yomoku-core`; test one command and render effect on host.
- [x] Add `crates/yomoku-runtime` EffectRouter and `crates/yomoku-ffi`; host-test typed event/request/view serialization and boundary errors. Native exports remain below.
- [x] Add reproducible Crux Kotlin payload generation from shell event/request/view schemas; run generator twice with identical output and pass all-feature host checks. Kotlin compilation/round trips remain below.
- [x] Verify/pin BoltFFI and build native bridge exports with typed errors and callback ownership; pass host facade tests and inspect shared-library exports.
- [x] Install matching BoltFFI CLI and Kotlin tooling; generate bindings and compile/test Kotlin event/request/view serialization, errors and lifecycle across actual JNI, including listener release and calls after close. Repeat generation with identical output.
- [x] Test asynchronous callback delivery across JNI with real storage effects; host JVM verifies worker-thread delivery, document errors/retry, reopen persistence, listener release and close during I/O. Navigation remains synchronous.
- [x] Add Gradle wrapper and Kotlin Compose app under `android/`, min SDK 29; CLI assembleDebug and strict lint succeed with pinned compatible JDK/SDK/plugin versions. NDK selection belongs to native packaging below.
- [x] Verify/pin/install a compatible NDK, generate/package Kotlin/JNI and native libraries for x86_64 emulator and arm64 device, and wire a Rust-backed shell; assemble app/instrumentation APKs, pass strict lint and verify both packaged ELF architectures/exports plus 16 KB ELF/ZIP alignment. NDK r30/API 29; Android execution remains below.
- [x] Demonstrate command → core → view state on Android by running `CoreSessionTest` on a device/emulator. One instrumentation test passes on API 36 x86_64 medium_phone, covering generated JNI/codecs, Rust render state, independent sessions and handle closure.
- [x] Add Library-default navigation, Nostr/Statistics/Vocabulary placeholders and toolbar Settings using Rust state; verify phone bottom bar/tablet rail, Settings Back, retained session across Activity recreation and final native-handle cleanup on API 36 x86_64.
- [x] Hide main navigation in the active reader when the phase-2 reader route is implemented; phone/tablet landscape first-page opening/Settings/Back verified by Compose instrumentation.
- [x] Add `crates/yomoku-storage` SQLite initialization and numbered migration; empty DB creation/reopens, ordered upgrades, rollback, concurrent opens and failure preservation pass in 11 host tests; bundled SQLite compiles for both Android ABIs. Runtime/FFI document wiring remains below.
- [x] Exercise document create/read/list through runtime/FFI on device; private Android DB, worker I/O, exact Unicode/reopen and failure/retry pass on API 36 x86_64. Rename/delete CRUD remains in phase 4 with its specified semantics.
- [ ] Add `.github/workflows/ci.yml` host Rust checks, Kotlin lint/tests and unsigned Android build; verify each command locally where available.
- [x] Document SDK/emulator/adb setup, native ABIs and build/install/run commands; API 36 x86_64 AVD installed with Android CLI, KVM checked on host, launcher restart/ADB authorization/instrumentation and debug APK install/Library launch verified. Default CLI startup graphics failure and swangle fallback documented; arm64 runtime testing remains future work.

## 2 — Paste-to-reader slice
- [x] Define stable document/source/chapter IDs and text offset encoding in DATA_MODEL.md; test Unicode round trips at FFI boundaries. Positive i64 IDs, chapter-local UTF-8 byte anchors and checked UTF-16 conversion; seven Rust/generated-Kotlin/Rust payload exchanges pass. Live document JNI awaits storage commands.
- [x] Add immutable pasted-text document command/storage with first-line title; typed PasteText and repository create/load commands pass exact Unicode/reopen, per-insert/commit rollback, concurrent-write and committed-ID nonreuse/exhaustion tests. Crux events/runtime/FFI are verified by the phase-1 document wiring task; Android paste UI/full import semantics remain below.
- [x] Implement logical pages in `crates/yomoku-core`: presets 250/500/750/1000/1500 (default 500), paragraph splits and long-paragraph following-sentence fallback; exact threshold, short text, mixed scripts and long paragraph pass. User clarified all non-whitespace Unicode characters count; routine boundary rules are in DECISIONS.md.
- [x] Enforce EPUB chapter boundaries and no inferred TXT chapters; explicit ordered chapter/heading fixtures pass before import wiring. EPUB/TXT importers remain phase 4.
- [x] Reflow pages preserving underlying text offset; all 25 preset pairs pass with Japanese, supplementary Unicode, chapter EOF and invalid anchors. Fourteen pagination tests total.
- [x] Integrate/persist the five page-length Settings presets and reflow the active reader without changing its exact saved offset or Library percentage; migration/rollback, all 25 preset pairs, fresh-session reopen, JNI and Android Settings/recreation pass on phone/tablet landscape.
- [x] Add Android paste flow returning to Library, list titles and first-page reader opening; test first import/open, draft/error retention, recreation and Back. Rust owns routes and default-500 first-page projection; current progress is the separate task below.
- [x] Add Library current progress with persisted reader position; test forward/backward percentage changes and reopen. Migration 0003 and typed position-only saves validate ownership/UTF-8 boundaries, retain exact offsets, survive reopen and roll back failures; Rust/JNI/Android tests pass. Page controls/forward completion remain below; no seek UI is added.
- [ ] Render centered single-column pages, Next/Previous/swipes and content-bottom counter; test both directions update current position and reset scroll on navigation/reopen.
- [x] Add repository completion command IDs/history and atomic position persistence; test duplicate callbacks count once (also concurrently/after reopen), rereading with a fresh ID counts again, position-only Previous adds no history, failures roll back, and provenance survives deletion. Migration 0005; ten new repository tests pass.
- [ ] Wire completion intents through core/runtime/FFI with encounters/New→Known/time in the same transaction, then connect Next/Previous/swipes; verify duplicate UI callbacks and rereads end to end. The repository foundation above is not exposed as a forward action yet.
- [x] Implement pure hidden active-time accounting with five-minute idle timeout; thirteen fake-clock/domain tests cover interactions, dictionary popup, background/close, restored durations with a fresh clock epoch, duplicate lifecycle signals, checkpoints and invalid clocks/overflow. Checkpoints retain pending time without creating statistics; no platform/persistence integration yet.
- [ ] Persist partial-page active time and wire the accumulator to platform monotonic clock/lifecycle/interactions; verify actual close/reopen/reboot recovery and commit time only with atomic page/post completion. Pure accounting is verified above; page/reflow attribution and durable completion handoff remain open.
- [ ] Persist finished-at-least-once independently of position; test each finish, reopen at page 1, Finish Book return and completion animation.

## 3 — Analysis, vocabulary and dictionary
- [x] Add pure surface identity, five statuses, registration policy and New-only completion transition in `crates/yomoku-core/src/vocabulary.rs`; host tests cover inflections, exact spelling, blank rejection and all statuses. Persistence/event execution remains below.
- [x] Verify Sudachi.rs API, Full dictionary packaging/license and version; add `crates/yomoku-analysis` with default B and A/C fixtures, including inflections, proper nouns and mixed scripts. Engine v0.7.0 pinned by commit, Full 20261015 pinned by archive/data SHA-256 with complete notices; nine real-data host fixtures pass, including normalization, rare kanji, long input and SQLite reopen. Android cfg fix, bundling and measurements are tracked separately below.
- [x] Record surface/token/source anchors and override schema before large imports; test Unicode anchors and reversible merge/split. Migration 0007 and pure validation preserve immutable versioned A/B/C snapshots, exact surfaces/UTF-8 anchors and separately reversible global/local boundary edits. Host tests cover Unicode, reopen, concurrency and rollback. Actual Sudachi output, cross-occurrence rule resolution and UI remain in their separate tasks.
- [x] Unblock Sudachi Android compilation with a documented source patch at the pinned engine revision; verify source integrity, both Android targets and the existing real Full fixtures. Vendored dependency-only snapshot adds Android to the existing `.so` cfg; no runtime/FFI/UI integration or device performance claim.
- [x] Bundle the pinned Full dictionary with complete app-accessible notices; installed dictionary/notice checksums and complete offline Settings access/recreation pass on API 36 x86_64 phone and tablet landscape. Device analyzer measurements remain separate below.
- [x] Measure Full initialization, memory and tokenization on Android, including offline analysis, before runtime integration. Standalone API 36 x86_64 probe passes three debug and three release processes with real A/B/C/Unicode/long-input validation; both ABIs link. Exact baseline/limits are in SUDACHI_MEASUREMENTS.md; physical arm64 execution remains untested.
- [x] Compare verified immutable mapped Full storage with the owned baseline before runtime integration. Ten real Full fixtures per loader and complete A/B/C output parity pass; three owned/mapped pairs per debug/release profile pass offline on API 36 x86_64. Mapped release anonymous RSS is ~2.5 MiB versus ~318 MiB owned; initial total RSS remains similar. Both release probe ABIs link; see SUDACHI_MEASUREMENTS.md for exact results and safety/lifetime limits.
- [x] Implement/measure on-demand verified immutable private Full asset extraction and failure recovery independently of analyzer integration. Eight new API-36 x86_64 tests pass: lazy/worker-only access, exact checksum/size, cache reuse, read-only descriptors, abandoned/interrupted/corrupt copies, rename failure/retry, held reader/mapping preservation and concurrent requests. Installed compressed Full extraction/verification takes 2.273–3.273 s; verified reopen 0.632–0.689 s across two instrumented processes (no cold-storage/arm64 claim).
- [x] Consume the verified private Full descriptor in a debug-only in-app measurement harness; verify worker reuse through Activity recreation/stop/resume, JNI failure/retry and affinity, actual unmapping on close and fresh-worker reopen. Combined Compose/JVM/native PSS/RSS and real A/B/C/Unicode/page/long-input measurements pass in two API-36 x86_64 processes; all eighteen Android tests pass. This is measurement scaffolding, not production runtime integration; exact values/limits in SUDACHI_MEASUREMENTS.md.
- [x] Verify actual app process death with Full mapped and recovery through the cached verified descriptor; two API-36 x86_64 force-stop/restart pairs pass A/B/C/Unicode/page/long-input validation, preserve the published inode and skip graceful close. Debug-only ADB harness; no pressure or hardware claim.
- [x] Resolve remaining Full loading/memory gate before runtime integration: two bounded API-36 x86_64 pressure runs reclaim 67/143 MiB of dictionary residency and preserve same-worker A/B/C/Unicode/page/long-input correctness. Choose verified immutable mapping, lazy reuse on the existing FIFO worker, release idle analyzer on UI-hidden/background and final session close; integration must implement/test this lifecycle. Results/limits: SUDACHI_MEASUREMENTS.md. Physical arm64 remains the separate deferred user trial.
- [ ] User-run physical arm64 validation once the app is usable; record device/build, reader/import correctness, performance and lifecycle observations. Deferred by user on 2026-10-24; emulator results do not establish hardware viability.
- [x] Add vocabulary registration/status-change migrations and repositories; test identical spellings across documents, distinct inflections and preserved existing statuses. Migration 0006 and 11 repository tests verify exact identity, dated registration/manual events, all statuses, concurrent deduplication, reopen and rollback. Analyzer/import/completion and UI wiring remain separate tasks.
- [x] Add an atomic analyzed-paste repository composition: document/text + validated Mode B snapshot + all unique Japanese surface registrations in one transaction, without encounters. Six host storage tests cover repeated/Unicode/non-Japanese tokens, preserved statuses, concurrent imports, analyzer/validation/write/commit rollback and reopen; real Full composition passes with both loaders. Production import wiring remains below.
- [x] Add an analyzed-import EffectRouter entry point with lazy FIFO-worker analyzer ownership, Created/Existing handling and queued cache lifecycle; seven data-free runtime tests and a real Full fixture per loader verify deduplication, status/progress preservation, every write/commit rollback, retry, background release and close during accepted work. This entry point is not selected by NativeBridge/Android yet.
- [x] Project committed Created/Existing outcomes into a dismissible single-paste import summary returning to Library without opening the reader. Core/runtime tests verify both outcomes, preserved duplicate progress, failures and transient-state clearing; host JNI verifies Unicode summary delivery, and three API-36 x86_64 paste-reader tests verify visible Created confirmation, dismissal, Settings/recreation and reader opening. Native duplicate delivery remains with the production integration task.
- [ ] Wire atomic analyzed-paste registration into production imports after the Full loading/lifecycle gate; extend to all supported document formats, without encounters. Verify repeated tokens and rollback end to end. Rust worker and typed summary are verified above; native verified-descriptor delivery, Android lifecycle and end-to-end duplicate summary delivery remain.
- [ ] Persist every Japanese occurrence at completion and New→Known changes in one transaction; test repetition, reread, duplicate command and unchanged Learning/Familiar/Ignored.
- [ ] Add tappable tokens, five direct status choices and N>0 toast; verify opening lookup never changes status.
- [ ] Verify JMdict English/JMnedict data licenses/versions; build reproducible offline index under `tools/` with attribution and lookup fixtures.
- [ ] Render all senses/readings, human-readable POS, separate name section and lifetime encounter count; test simultaneous dictionary/name matches and no match.
- [ ] Add editable/deletable custom English meaning + reading only for missing matches; test built-in definitions remain immutable.
- [ ] Bundle Noto Serif JP with attribution; add global font/line spacing and default-off furigana (source ruby preferred), system theme/status colors; test phone/tablet dark/light.
- [ ] Expose A/B/C and manual split/merge, default All occurrences versus This occurrence; persist/reverse overrides and verify anchors/history semantics.

## 4 — Imports and Library
- [ ] Implement TXT UTF-8/Shift-JIS detection/manual fallback in `crates/yomoku-import`; test hard breaks, uncertain encoding and common Aozora ruby/directives.
- [ ] Implement EPUB metadata/chapter/headings/emphasis/ruby/image extraction ignoring CSS; fixtures cover malformed archives and chapter boundaries.
- [ ] Render EPUB images at source position/ratio, cover cache and external links; verify internal links are noninteractive and no TOC navigation.
- [ ] Add shared full-screen image zoom/pan component; verify phone/tablet rotation.
- [ ] Implement tolerant SRT parser stripping numbers/timestamps/tags while preserving cue blocks, breaks and speakers; test malformed fixtures.
- [x] Add repository exact-byte paste deduplication around atomic analyzed import; migration 0008, existing-document reference without analysis/rescan, concurrent create-once, Unicode/NUL identity, legacy duplicate preservation, rollback and deletion/reimport pass in nine host tests and both real Full loaders. No production wiring or file import identity yet.
- [ ] Add SHA-256 exact duplicate identity for original file bytes and per-file atomic imports; test existing-document reference and failed-file isolation. Paste repository foundation is verified above; Android/runtime dedup wiring remains with analyzed import integration.
- [ ] Wire Android multiple-file picker and batch summary returning to Library; test mixed formats/failures, never auto-opening reader.
- [ ] Add progress/current-file/X-of-N, cancellation and permitted background notifications; test retained completed imports, discarded current partial and no process-death auto-resume.
- [ ] Add Library title search/sorts, icons/covers and distinct-surface five-status bar; test global status changes and unopened ordering.
- [ ] Add rename and confirmed single/bulk/Select All document deletion; test preserved history/vocabulary and fresh progress/checkmark on reimport.

## 5 — History and Vocabulary browser
- [ ] Finalize versioned immutable event/provenance schema and migration tests; verify deleted documents retain historical source metadata.
- [ ] Add lifetime occurrence/time aggregation; test repetition, reread, incomplete-page exclusion and duplicate completion protection.
- [ ] Add status timeline aggregation including registration jumps and Known dictionary-form counts (any Known surface, unresolved excluded, spelling identity); test transitions and deletion recalculation.
- [ ] Add timeframe buckets: last month daily, six months weekly, year weekly default, all time monthly; test date boundaries and lifetime cumulative origin.
- [ ] Render overview/drilldowns: cumulative occurrences, five status lines, Known surface/lemma lines; test datasets and all timeframes.
- [ ] Add indexed surface-prefix search, status filters, matching count, 100-row paging and newest/frequency/last-seen/alphabetical sorting; test query plans and pagination.
- [ ] Add global matching Select All and confirmed bulk status/deletion; test unloaded rows and immutable status events.
- [ ] Verify vocabulary deletion erases its history and recomputes charts, does not rescan existing documents, and subsequent appearance registers New.

## 6 — External integrations (after local reader)
- [ ] Verify supported AnkiDroid provider/deck/model/duplicate APIs; record feasible exact-surface duplicate behavior.
- [ ] Implement optional Add to Anki, configured deck, note model and existing-note action; test absent provider, duplicate, target sentence highlight and unchanged vocab status.
- [ ] Verify `nostr-sdk` APIs; add enabled relay/curated-author configuration, initially empty unless supplied; test kind-1-only and Japanese local filtering.
- [ ] Add dedup/newest-first fetching and partial relay failure recovery; verify no arbitrary authors or age cutoff.
- [ ] Implement stable current+9 queue, 20 previous, tab resume, restart reset and explicit refresh; test arrivals do not reorder active queue.
- [ ] Register on first display and reuse completion semantics for Next/swipe; test Skip/Previous/parent views produce no encounters, status changes or forwarding.
- [ ] Add kind-0 author metadata/fallback, relative time, reply indicator and on-demand parent; verify nonclickable references and mixed-script interaction/counts.
- [ ] Load remote images only on display with 250 MB disk LRU, shared viewer and external audio/video links; test prefetch/cache limits.
- [ ] Generate BIP39/NIP-06 identity and Keystore encryption; verify first launch/reopen without logging secrets.
- [ ] Add optimistic kind-7 reactions/kind-5 own-reaction deletion and durable offline retry; test restart/connectivity recovery.
- [ ] Resolve Q188 with user before optional Citrine forwarding; implement original signed-event forwarding and test selected failure policy.

## 7 — Backup, hardening and release
- [ ] Specify versioned full-state backup format with reviewed Argon2id/AEAD libraries/parameters; record design and dependency evidence.
- [ ] Implement encrypted export of documents/history/settings/custom entries/identity; test complete round trip.
- [ ] Validate/authenticate before confirmed replace restore and device key re-encryption; test wrong password, tampering, interrupted replacement and preserved original state.
- [ ] Test supported schema/version upgrades and restored identity usability; never merge backups.
- [ ] Benchmark large EPUBs, Full dictionary, many books and heavy history; record measurements and fix demonstrated bottlenecks.
- [ ] Run phone/tablet portrait/landscape, dark/light, airplane mode, accessibility and process-death/lifecycle matrix.
- [ ] Review privacy, secrets, third-party redistribution/attribution, errors and asset/APK size.
- [ ] Complete CI and document signed APK/ABI/release commands with secrets excluded; test unsigned release build.
- [ ] Prepare GitHub Releases/Zapstore artifacts and instructions; obtain manual approval before signed publishing.

## Cross-feature acceptance matrix

- [ ] Short Japanese text, long paragraph, ruby, mixed Japanese/Latin, punctuation and kanji-only proper noun.
- [ ] Exactly 500-char boundaries; end of paragraph; end of EPUB chapter; setting changes preserve reading text position. Pure pagination/reflow and persisted Android Settings pass; EPUB import/rendering remains.
- [ ] Page left swipe completion vs previous/right swipe; replay reread counts again; accidental double callbacks only count once.
- [ ] Page scroll always resets on navigation/app reopen; current document position updates in both directions.
- [ ] Import registers New surfaces but does not add reading occurrences; popup opens without status change.
- [ ] Every same surface spelling shares one global status even across documents; inflected spellings remain distinct.
- [ ] Completed page New→Known only; Learning/Familiar/Ignored untouched; global history updated correctly.
- [ ] Repeat occurrences and rereads reflected in cumulative chart; per-timeframe y-axis remains lifetime cumulative.
- [ ] Import duplicate handling; one failed file does not block batch; cancel leaves no partial document; killed process doesn't resume.
- [ ] Document deletion preserves historical graphs; vocabulary deletion removes that entry's event history and recalculates.
- [ ] Mixed JP/non-JP Nostr counting, Next vs Skip vs Previous, relays partially offline, newest-first curated authors only.
- [ ] Failed backups/bad passwords do not harm existing data; restored identity remains usable.
