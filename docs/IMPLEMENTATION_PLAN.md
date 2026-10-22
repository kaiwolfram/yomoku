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
- [ ] Test asynchronous callback delivery across JNI when the first real async effect is added (navigation is synchronous).
- [x] Add Gradle wrapper and Kotlin Compose app under `android/`, min SDK 29; CLI assembleDebug and strict lint succeed with pinned compatible JDK/SDK/plugin versions. NDK selection belongs to native packaging below.
- [x] Verify/pin/install a compatible NDK, generate/package Kotlin/JNI and native libraries for x86_64 emulator and arm64 device, and wire a Rust-backed shell; assemble app/instrumentation APKs, pass strict lint and verify both packaged ELF architectures/exports plus 16 KB ELF/ZIP alignment. NDK r30/API 29; Android execution remains below.
- [ ] Demonstrate command → core → view state on Android by running `CoreSessionTest` on a device/emulator. Test source and APK compile; no device/emulator is currently available.
- [ ] Add Library-default navigation, Nostr/Statistics/Vocabulary placeholders and toolbar Settings; verify phone bottom bar/tablet rail and hide navigation in reader.
- [x] Add `crates/yomoku-storage` SQLite initialization and numbered migration; empty DB creation/reopens, ordered upgrades, rollback, concurrent opens and failure preservation pass in 11 host tests; bundled SQLite compiles for both Android ABIs. Runtime/FFI document wiring remains below.
- [ ] Exercise document CRUD through runtime/FFI on device; verify reopen persistence and failures.
- [ ] Add `.github/workflows/ci.yml` host Rust checks, Kotlin lint/tests and unsigned Android build; verify each command locally where available.
- [ ] Document SDK/emulator/adb setup, native ABIs and build/install/run commands; execute on an available emulator/device.

## 2 — Paste-to-reader slice
- [ ] Define stable document/source/chapter IDs and text offset encoding in DATA_MODEL.md; test Unicode round trips at FFI boundaries.
- [ ] Add immutable pasted-text document command/storage with first-line title; test transaction rollback and reopen.
- [ ] Implement logical pages in `crates/yomoku-core`: presets 250/500/750/1000/1500 (default 500), paragraph splits and long-paragraph following-sentence fallback; test exact threshold, short text, mixed scripts and long paragraph.
- [ ] Enforce EPUB chapter boundaries and no inferred TXT chapters; test boundary fixtures before import wiring.
- [ ] Reflow pages preserving underlying text offset; test all presets with Japanese and supplementary Unicode.
- [ ] Add Android paste flow returning to Library, list title/current progress and reader opening; test first import/open.
- [ ] Render centered single-column pages, Next/Previous/swipes and content-bottom counter; test both directions update current position and reset scroll on navigation/reopen.
- [ ] Implement completion command IDs and atomic persistence; test double callbacks count once, rereading counts again, Previous does not complete.
- [ ] Implement hidden active-time accumulator with five-minute idle timeout; fake-clock tests cover interactions, dictionary popup, background, close/reboot and pending time committed only on completion.
- [ ] Persist finished-at-least-once independently of position; test each finish, reopen at page 1, Finish Book return and completion animation.

## 3 — Analysis, vocabulary and dictionary
- [x] Add pure surface identity, five statuses, registration policy and New-only completion transition in `crates/yomoku-core/src/vocabulary.rs`; host tests cover inflections, exact spelling, blank rejection and all statuses. Persistence/event execution remains below.
- [ ] Verify Sudachi.rs API, Full dictionary packaging/license and version; add `crates/yomoku-analysis` with default B and A/C fixtures, including inflections, proper nouns and mixed scripts.
- [ ] Record surface/token/source anchors and override schema before large imports; test Unicode anchors and reversible merge/split.
- [ ] Bundle Full dictionary and measure initialization/tokenization on device, including offline operation.
- [ ] Add vocabulary registration/status-change migrations and repositories; test identical spellings across documents, distinct inflections and preserved existing statuses.
- [ ] Register every unique surface atomically at document import, without encounters; test repeated tokens and rollback.
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
- [ ] Add SHA-256 exact duplicate identity and per-file atomic imports; test existing-document reference and failed-file isolation.
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
- [ ] Exactly 500-char boundaries; end of paragraph; end of EPUB chapter; setting changes preserve reading text position.
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
