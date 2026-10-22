# Yomoku — Coding Agent Handover

> **Purpose:** This is the implementation handover for a coding agent. Build the application incrementally across many short coding sessions. Do not spend the first session merely rewriting specifications: inspect the repository, create the persistent plan described below, then implement and test the first smallest useful milestone.
>
> **Project:** **Yomoku** — *“Read Japanese, naturally.”* A native, offline-first Android app for reading Japanese books, text, subtitles, and curated Nostr posts while tracking vocabulary and reading progress. Inspired by Lute, but this is its own product.
>
> **Status:** Product decisions collected; implementation repository and current code state **not provided in this handover**. Inspect what exists; do not assume a blank or working repository. The user's next objective is **starting implementation now**.

## 1. Your first instructions — mandatory persistent progress tracking

1. Inspect the repository (`git status`, tree, existing docs, dependencies, build scripts, tests). **Preserve existing work.** Never overwrite working components or restructure an existing repo without necessity.
2. Create/update **`docs/IMPLEMENTATION_PLAN.md`** with a **phased, ordered Markdown checklist** (`- [ ]` / `- [x]`) broken down into tasks that can be completed and verified in individual sessions. Use the phases in Section 10 as a starting outline, but adapt to the actual repo. Keep task granularity small enough that a task has a clear testable completion condition.
3. Create/update **`docs/NEXT_SESSION.md`** as a **brief resume point** (ideally <80 lines): current phase, what works, exact next 1–3 unchecked tasks, last successful build/test commands, known failures/blockers, and files to read next. This is the **first file to read at the start of every future session**.
4. Create/update **`docs/DECISIONS.md`** for decisions not already fixed by this specification; record dates, rationale, and alternatives. Do not reopen confirmed requirements unless a technical impossibility or major cost warrants user approval.
5. Create a concise **`AGENTS.md`** explaining the session workflow: read `docs/NEXT_SESSION.md` and relevant sections of the plan, implement a small vertical slice, test, mark checkboxes, update next-session notes, and report results. Also maintain `docs/ARCHITECTURE.md` and `docs/DATA_MODEL.md` as designs become concrete. A short append-only `docs/SESSION_LOG.md` is useful if summaries are kept compact.
6. **At the end of EVERY coding session:** (a) check off only genuinely finished tasks; (b) note files changed; (c) record tests run and their results; (d) list blockers/unresolved questions; (e) update `docs/NEXT_SESSION.md` so a fresh agent can resume **without reading the entire chat**. If work is incomplete, explicitly leave its checklist item unchecked and explain the intermediate state.
7. Favor a **working, tested vertical slice** over many unverified scaffolds. Make the smallest coherent change, run relevant CLI checks, and avoid unneeded abstractions. The user works primarily on **Fedora Linux with CLI tooling**, without requiring Android Studio for the usual workflow; document Gradle, Cargo, `adb`, and emulator CLI commands.
8. If you cannot complete a feature in the current session, finish with a buildable state where possible and provide a precise continuation note. Do not claim tests passed unless you ran them. Don't quietly invent product requirements.

**Definition of progress:** Code actually present + compilation/tests passing (where the environment permits) + corresponding checklist checkbox checked + resume documentation updated.

## 2. Product scope and priorities

**v1 includes:** native Android app; native Japanese reader; local library of imported EPUB/TXT/SRT and pasted text; offline word segmentation and dictionaries; vocabulary status workflows; historical reading/vocabulary statistics; optional AnkiDroid integration; curated Nostr reading with reactions; secure manual encrypted backups; configurable settings; release tooling.

**Important late scope changes that override earlier brainstorming:**

- **No in-app AI features in v1**, including BYOK API keys/provider/model settings, AI-generated definitions or translations, and AI regeneration. Do not scaffold AI merely because older design notes might mention it.
- **Only EPUB, TXT, SRT, and pasted text**. No WebVTT `.vtt`, no ASS/SSA `.ass`, no PDF, no NHK integration, and no Android Share-intent import in v1.
- **No Nostr multi-device sync or Blossom cloud backups in v1.** Secure **manual encrypted export/import** remains.
- No full-text book search, no bookmarks, no editing of imported content, no in-app audio/video players, no custom goals or streaks, and no calendar heatmap.
- An online marketing site (possibly `yomoku.app`) is not part of the essential Android MVP. Concentrate on the app and tests first.

**Priority order:** dependable persistence / accurate reading and vocabulary semantics > robust imports / dictionary lookup > usable UI > charts > external integrations > release polishing. Do not start with Nostr, signing, or intricate UI styling before the local reader works.

## 3. Required technical architecture

- **Android UI:** Kotlin with **Jetpack Compose**. Phone/tablet responsive layouts. Android min SDK **29** initially; reassess later if needed.
- **Domain/application logic:** Rust **Crux** core, ideally pure and testable without Android. Domain invariants and commands belong here.
- **Japanese analysis:** Rust **Sudachi.rs**, bundled **Sudachi Full dictionary**, default segmentation **Mode B** with A/C options where relevant.
- **Offline dictionary data:** bundle **JMdict (English definitions)** and **JMnedict (names)** with the APK. Version them and satisfy redistribution/attribution requirements. Both sources appear in separate sections when both match.
- **Persistence:** SQLite, accessed from a separate Rust I/O/persistence layer using `rusqlite` / `tokio-rusqlite` as suitable. Keep SQL and Android-specific I/O **outside the pure Crux core**. Track schema migrations from the start.
- **Rust effects:** Rust-side `EffectRouter` executes Rust I/O. Nostr networking through Rust **`nostr-sdk`**, not duplicated in Kotlin.
- **Android bridge:** Kotlin handles Compose, file picker, notifications/background execution, Android Keystore, AnkiDroid provider integration, and platform APIs; expose a narrowly scoped Rust FFI interface. Prefer explicit, typed state/actions over ad hoc strings.
- **Workstation workflow:** Gradle/Cargo CLI, terminal-based Android SDK/emulator/`adb`; **no Android Studio requirement** for normal build/test/run. Document native library build, ABI packaging, emulator setup, and release signing without committing secrets.
- **Repository:** MIT license for original code, respecting third-party licenses; clear module boundaries; tests and CI (GitHub Actions) for Rust, Kotlin, lint, and Android builds. Manual approval for signed publishing. Eventual APK distribution via GitHub Releases and Zapstore, not Google Play/F-Droid for initial publishing.

If a chosen library does not support a required capability or architecture in current versions, validate its actual API and explain the minimal deviation before changing a major architectural choice. Avoid blind dependency/version guesses.

## 4. Reader, text processing, and vocabulary behavior

### 4.1 Documents and pagination

- App opens on **Library**. Four top-level destinations: **Library, Nostr, Statistics, Vocabulary**; Settings accessed via toolbar. Use bottom navigation on phones, rail on tablets; hide main navigation while actively reading.
- Reader is **horizontal text only**, single centered column even on landscape tablets; comfortable maximum text width, tablet margins. Dictionary panel on wide displays, bottom sheet on phones. Both portrait and landscape supported.
- Logical page length setting has **presets 250 / 500 / 750 / 1,000 / 1,500 Japanese characters**, default **500**. Split at a paragraph boundary after reaching the threshold; if a paragraph is extremely long, split after an appropriate following sentence. **Never cross an EPUB chapter boundary** when forming a logical page. TXT has no automatically inferred chapter boundaries.
- Logical pages may **scroll vertically**, e.g. for long text or images. **Never persist vertical scroll positions**: navigating forward or backward, or reopening a document, starts at the top of the destination page. Page boundaries don't depend on screen dimensions. Page-length changes reflow pages while preserving the exact underlying reading text offset.
- **Previous** and **Next** controls, with counter `12 / 85` between them, appear at the bottom of the page content. Left swipe = next logical page, right swipe = previous, even before the bottom is reached. **No bottom-reached requirement**. Both forward and backward navigation update the *current* reading position; the Library's percentage uses current text offset / total text length, so it can decrease if the reader goes backward.
- Advancing **forward** completes the departing page, even if the user swiped without reaching the bottom. Simply moving **backward** does not complete or count the page. Reading a previously read page and completing it again **does count again** (idempotency must prevent accidental double-trigger/tap counting).
- Incomplete pages save position and pending time, **not** read-word events/counts/status updates until page completion. On page completion, every Japanese word occurrence (including repetitions) contributes to encounter/reading counts; any still-**New** unique words present on that page are changed directly to **Known**. Don't automatically change Learning/Familiar/Ignored. Show toast `N new words marked as Known` **only when N > 0**.
- Full-document completion: maintain permanent **finished-at-least-once** checkmark independent of current position. Show brief confetti on each completion; **Finish Book** returns to Library. Reopening a finished book begins at page 1. Deleting and later reimporting resets progress and checkmark, while global history survives.
- Reader styling: bundled **Noto Serif JP**; adjustable global font size and line spacing; furigana switch **off by default**, when on show available ruby readings (source ruby when provided, else analyzer), irrespective of vocabulary status.
- EPUB supports chapters, headings, emphasis, inline images, links, and source ruby, **but ignore EPUB CSS**. Inline images keep position/ratio, fit column width; tapping opens full-screen pinch-zoom/pan viewer. Internal footnote/chapter links are **not navigable**; external website links open external browser. No TOC-jump feature.
- Clipboard **Copy Sentence** was discussed but explicitly **deferred beyond v1**. Don't implement in first release.

### 4.2 Vocabulary model and dictionary panel

- Five globally tracked statuses: **New**, **Learning**, **Familiar**, **Known**, **Ignored**.
- Status colors: New blue (`#A9CCFF` light / `#28558D` dark), Learning light red (`#FFC5C0` / `#783E44`), Familiar yellow (`#FFE29A` / `#725A2D`), **Known unhighlighted in text** (green in graphs), Ignored pronounced gray (`#BEC5CF` / `#505B6B`). Follow Android light/dark system theme; **no in-app manual theme override**.
- Vocabulary identity is the **exact written surface form**, NOT a lemma: `食べる`, `食べた`, `食べます` are separate entries with separate statuses and encounter counts. Same spelling with different readings is **one** status entry; show all appropriate dictionary readings/senses. Link each surface form to a dictionary form determined automatically by Sudachi, for aggregate lemma statistics.
- **No manual correction of surface→dictionary-form mappings in v1** (complexity reduction). Don't edit built-in JMdict/JMnedict definitions; allow user-created, editable/deletable **English meaning + reading** dictionary entry when no dictionary match exists. No global overriding of built-in definitions.
- Default every newly encountered/imported written Japanese word to **New**. Upon **book import**, register **all unique word surfaces in its content immediately**, not lazily only when read. For Nostr, register vocabulary when a post is **first displayed**, even if it is later skipped. Registration is different from a **reading encounter**.
- Word tap opens dictionary UI with original surface form, readings, linked base form, all **JMdict English senses immediately expanded and grouped by sense**, compact human-readable grammatical labels (e.g. noun, Ichidan/Godan verb, adjectives), and **JMnedict name readings/categories in a separate section** when available. Proper names are treated like other vocabulary, not excluded.
- Display **`Encountered 47 times`** (lifetime completed-reading occurrence count) for that surface. Opening popup does **not** change status; use one visible row of **five direct status choices** with no increment/decrement controls. Show Anki integration action if applicable.
- Sudachi defaults to Mode B. Users may adjust token boundaries manually (merge/split) and access A/B/C alternate segmentation when necessary. On override prompt **All occurrences** (default) vs **This occurrence**; overrides persist and can be reversed globally where applicable. Conjugated expressions should remain usable as selectable surface forms. Make this testable before large-scale data population because text anchoring/migrations are sensitive.
- No preknown vocabulary list. Nothing is implicitly Known from parser import, dictionary lookup, or Anki export.

### 4.3 Imports and library

- Supported **EPUB, TXT, SRT**, and **Paste Text**. Imported source content is **immutable**; users may rename but not edit. To change text, delete/reimport.
- Android picker supports **multiple files per batch**. Per-file import is **atomic**; one error never aborts other files. Detect an **exact duplicate with SHA-256** and open/reference the existing document instead of creating another copy. Upon new import register all unique surface forms as New without overwriting already existing statuses.
- Titles: EPUB metadata where present, otherwise filename; pasted text uses first line as initial title; user can rename.
- TXT: UTF-8 and Shift-JIS; attempt appropriate detection and allow manual selection when uncertain. Preserve original hard line breaks; no reflow of adjacent lines. Parse **common Aozora Bunko markup**, including ruby `｜漢字《かんじ》`, and hide/interpret common typographic directives. Don't infer chapters from titles such as `第一章`.
- SRT: best-effort parsing of mildly malformed files; strip timestamps/cue numbers and formatting tags (e.g. `<i>`), retain **separate cue blocks**, breaks within cues, and sensible spacing; do not merge successive cues into prose. Preserve speaker indications when meaningful. No VTT or ASS support in v1.
- Show **batch import progress** with current file and X of N; permit **Cancel** (retain completed imports; discard current partial and remaining). Continue imports on Android while user opens another app when permitted by OS background policies; show progress/completion notifications. If Android kills the process, **keep finished imports but do NOT automatically resume** the unfinished batch on next launch. Do not misrepresent OS guarantees.
- On any completed import (single/batch), show summary and return to Library; **never automatically open reader**.
- Library: compact list with EPUB cover thumbnails (cache) or generic TXT/SRT/paste icons; title, current progress, permanent checkmark, and a **stacked five-status distribution bar of distinct word surfaces present in that document** (not token frequencies), updated whenever global statuses change. Search titles; sort last read (default), title, import date; show new/unopened sensibly. No folders/collections.
- Long-press multi-select including Select All for documents; delete individually or in bulk with confirmation; **no Undo**. Deleting removes content/current position and preserves source metadata needed for historical event provenance, historical reading stats, encounter records, and all global vocab statuses. If reimported later, treat as a **new document starting at 0%**, no restored completion checkmark.

## 5. Statistics, durable history, and vocabulary browser

### 5.1 Events and statistics

- Design schema around immutable, dated **status-change events**, **reading-completion events**, **every Japanese surface occurrence encountered on completed pages/posts**, source identity/provenance, and **active reading time**. History matters more than cached counters; use migration-safe IDs and explicit versioned semantics.
- Multiple occurrences of a word on one completed page count multiple times. Re-reading counts again. Avoid duplicate counting on accidental repeated click/gesture. Non-Japanese content and punctuation aren't included in Japanese word counts.
- Active time starts while reader open and includes dictionary popups, but pauses in background/when reader closes and after **5 min with no interaction**. Reset idle timer on any interaction. Persist partial-page accumulated time across closing/reopening/reboots; commit time to reading statistics **only after that page/post is completed**. Hidden timer (no timer in reader UI).
- Statistics: overview + tap into detailed charts. Timeframes: **last month**, **last six months**, **last year (default)**, **all time**; group **daily / weekly / weekly / monthly** respectively.
- Graph 1: **cumulative lifetime Japanese word occurrences read**. Time filter changes visible x-range but **not** origin of cumulative y-values (e.g. 85k→100k for last month).
- Graph 2: **five distinct time-series lines** of absolute number of registered word surfaces in each status, with statuses' graph colors; import-time registration causes jumps in New.
- Graph 3: **two lines**: number of Known surface forms and Known distinct **dictionary forms**, counted Known if any linked surface is Known. Lemma uniqueness based on written spelling, not reading; if Sudachi cannot resolve dictionary form, **exclude** it from Known-lemma counter.
- No calendar heatmaps, goals, streaks, or per-source charts in v1 (keep source identities internally).
- **Vocabulary deletion semantics differ from document deletion:** permanently deleting a vocabulary entry wipes its encounter/status history and recalculates history-based stats. If the form appears again later, it starts as New; existing documents are not automatically rescanned to recreate it just because a word was deleted.

### 5.2 Vocabulary browser

- Separate top-level **Vocabulary** screen.
- Search by **written-surface prefix** only, indexed; no reading/lemma/full-text search.
- Default sort newest registration first; alternatives frequency, last seen, alphabetical.
- Filter by five statuses; show total matching count; query 100 at a time with infinite scroll.
- Select All targets **every matching row, including results not loaded on screen**. Bulk change status or permanently delete with confirmation. Record status change events; enforce delete/history semantics above.

## 6. Nostr reading (curated, not a general social client)

- Use Rust `nostr-sdk`. Fetch only **kind-1** text events from **explicitly enabled/curated author npubs** across enabled relays, with editable lists and add-custom options. **Never sample arbitrary relay authors**. Filter candidate post text **locally** for Japanese with lightweight hiragana/katakana + kanji heuristics; accept occasional inaccuracies. Hardcoded starter author/relay lists are a product-data decision: **do not invent accounts or relay endpoints; ask if no actual curated list is supplied**, or use a clearly empty/configurable list initially.
- One full post at a time; no social feed, no long-form kind 30023, no repost kind 6. Replies posted by curated authors may appear with reply indicator. Offer **View Parent** for immediate reply parent (fetch parent on demand if needed); deeper navigation is on demand. Just viewing parents must not create reading stats.
- Newest-first, **no age cutoff**; previously read and skipped posts are eligible again. An active queue holds **10** events (current + 9 prefetched); retain up to **20 previous** posts in memory. Keep queue stable if new events are published while user reads.
- On entering Nostr, fetch latest posts. Switching app tabs **within same process** resumes the current post/queue; background refresh should not interrupt it. Explicit **Refresh** begins a new latest-first queue. Full app restart **does not preserve queue/current post**; start fresh.
- Relay failures are nonblocking: show posts from available relays, unobtrusive warning, retry on next refresh. Deduplicate same events across relays.
- Display author's **name + avatar** from kind-0 profile metadata (fallback shortened npub/generic avatar) and **relative publication time**. Names, avatars, `npub` and `note` references are **not clickable**; standard external HTTP(S) links open in browser.
- Display every long post fully, vertically scroll if needed; **no Read More**, no content-warning gate, no separate kind-5 deleted-post filtering for *other people's posts* in v1. Non-Japanese parts of mixed posts display subdued, are not tappable vocab, and don't count as Japanese reading.
- Inline remote images load **when the post is displayed**, not during prefetch; placeholder while loading, **250 MB disk LRU** image cache; image taps use same full-screen pinch-zoom/pan viewer as EPUB. Audio/video show **Open Audio/Video** and open externally; no in-app player.
- Advance via **Next Post** or reading swipe, completing the post: record occurrences/time, change displayed still-New words to Known, and attempt forwarding **the original signed Nostr event** to an optional local Citrine-compatible relay. **Skip is an explicit button only**; Skip must not affect stats, statuses, or Citrine forwarding. Previous does not count completion. Never silently treat a swipe as Skip.
- Optional local relay (such as Citrine) can provide offline access to forwarded originals; the Yomoku database does **not** persist all external event bodies. Exact forwarding failure/retry policy is **UNRESOLVED Question 188**: ask user **A: queue forward attempts for retry vs B: best-effort no queue** before implementing this piece. Do **not** assume an answer.
- Automatically generate a random **12-word BIP39 mnemonic** on first launch, derive a Nostr signing key via NIP-06, store it encrypted at rest using Android Keystore, include it in the encrypted manual export. No login/signer UI.
- Heart reaction publishes Nostr kind-7 `+` to all enabled relays; removing own reaction uses a NIP-09 kind-5 deletion event, distinct from ignoring deletion filtering for other authors. **Optimistic UI** and persistent offline **reaction publish queue** for retry on connectivity return; no reaction counts, only own reacted/not reacted state.

## 7. AnkiDroid integration

- Only when AnkiDroid is installed and its supported ContentProvider integration is available; app works fully without AnkiDroid.
- From word dictionary UI, **Add to Anki** to a configured default deck, using a Yomitan-inspired HTML/CSS Yomoku note model. Detect duplicate note for the **same exact written surface** even if note was made elsewhere when feasible via supported API; show existing-note indicator/open action.
- Front: **the word only**. Back: kana reading, English meaning(s), and the original Japanese sentence with target word highlighted. **No audio**. Export does **not** change Yomoku vocabulary status.
- Confirm what the current AnkiDroid API actually supports before finalizing duplicate handling; don't invent APIs.

## 8. Secure manual backup and restore

- **Manual export/import only**: all content, reading position/completion, vocabulary and history, settings, custom dictionary records, and the generated Nostr mnemonic/identity. No Nostr sync, Blossom, cloud account, or automatic backups.
- Password-encrypted archive with a well-reviewed authenticated design (e.g. **Argon2id** KDF and **AEAD** encryption); record schema/format version, salt/nonce/KDF parameters. Do not invent home-grown cryptography or log secrets.
- Restore is a **complete replacement, never merge**. Authenticate/decrypt and validate backup before destructive work, warn and confirm before replacement, swap atomically/transactionally as possible. An incorrect password/corrupt backup must **leave existing installation intact**. Ensure key/mnemonic re-encryption for current device's Keystore as needed.
- Tests: correct password round trip, bad password, tampering, interrupted restore, new version/backward-compatible schema strategy, and complete state restoration.

## 9. UX and other confirmed decisions

- App title **Yomoku**, tagline *Read Japanese, naturally.* Branding concept: dark navy `#1B2637`, off-white open-book motif, blue `#579FF5` highlighted word; apply after core app is functional.
- Follow **Android system light/dark mode** automatically; no in-app override.
- All user-facing interface and offline definitions **English**.
- No Undo in v1; destructive bulk actions require confirmation.
- No reading-session timer visible, and no mandatory bottom-of-page reached check.
- Keep designs compact, tablet-first but usable on phones. Don't add unrequested document editing, longform Nostr support, AI, etc.

## 10. Suggested implementation roadmap — adapt into `docs/IMPLEMENTATION_PLAN.md`

Turn these into fine-grained Markdown checkboxes with concrete file paths, verification commands, and dependencies. The coding agent must **write the actual plan file into the repository and maintain it**.

### Phase 0 — Repository reconnaissance and continuity
- [ ] Inspect repo status, architecture, existing code/tests, Gradle, Cargo, Android SDK availability.
- [ ] Record actual build/test commands; establish a passing baseline or document precise blockers.
- [ ] Create `AGENTS.md`, `docs/IMPLEMENTATION_PLAN.md`, `docs/NEXT_SESSION.md`, `docs/DECISIONS.md`, `docs/ARCHITECTURE.md`, `docs/DATA_MODEL.md` as appropriate, without replacing existing documents blindly.
- [ ] List missing **essential** user inputs (e.g. actual curated Nostr npubs/relays, Q188), but proceed on independent tasks.

### Phase 1 — Minimal Android/Rust vertical slice
- [ ] Set up/verify Android Kotlin Compose app and responsive app navigation with placeholder Library/Nostr/Statistics/Vocabulary.
- [ ] Set up/verify Rust crates for Crux core, effects, storage, tokenizer/dictionary, FFI; wire one typed command/response end-to-end.
- [ ] Document and run CLI builds/tests/emulator launch; define CI baseline.
- [ ] Add persistent SQLite DB initialization and first migration; exercise CRUD through real FFI path.

### Phase 2 — Local reading MVP (first end-to-end valuable feature)
- [ ] Paste Japanese text; create document in SQLite with stable text IDs and offsets.
- [ ] Implement logical page algorithm, configurable presets, paragraph-aware split, stable progress offsets; unit tests for boundaries.
- [ ] Implement single-column reader, Next/Previous/swipes, page counter, scroll-to-top, current position persistence.
- [ ] Implement atomic page-completion state machine and active reading timer; test rereading, previous, swipes, double taps, app reopen.
- [ ] Wire minimal Library list with title and position; finish-book behavior and checkmark.

### Phase 3 — Japanese analysis and vocabulary
- [ ] Bundle/initialize Sudachi Full with correct licensing; tokenizer modes and conjugation-related lookup; benchmark on device.
- [ ] Define token/surface identity, source offsets, vocabulary registration, five statuses and history events.
- [ ] Register all unique surfaces on document import; implement word tap and status UI with no automatic popup transitions.
- [ ] Implement page completion: word-occurrence events, New→Known, toast when N>0, idempotency.
- [ ] Bundle/index JMdict + JMnedict offline; implement lookup with all senses/POS and names; add word encounter count.
- [ ] Implement furigana, typography, theme/colors, token split/merge overrides, missing-entry custom definitions.

### Phase 4 — Document parsing and Library management
- [ ] TXT UTF-8/Shift-JIS + encoding fallback + hard line breaks + common Aozora markup.
- [ ] EPUB extraction with chapter boundaries, headings/ruby/inline image rendering; cover thumbnail cache.
- [ ] Full-screen image viewer shared component; external links; internal links noninteractive.
- [ ] SRT best-effort import, separate cue blocks, stripped metadata/tags.
- [ ] Android picker batch import, SHA-256 duplicate rules, atomic per-file processing, error summary.
- [ ] Import progress/cancel/background Android execution/notifications; no auto-resume after termination.
- [ ] Library sorting/title search/status distributions/completion badges; rename, confirm delete, bulk delete/reimport semantics.

### Phase 5 — History, graphs, and Vocabulary browser
- [ ] Store durable events/provenance, implement accurate stat aggregation and migrations.
- [ ] Reading time accounting; duplicate/suspend/resume tests.
- [ ] Four timeframes and cumulative word-count chart, five-status chart, Known surfaces vs Known dictionary forms.
- [ ] Vocabulary prefix search, status filters, indexed paging/sorts/count, global Select All and bulk operations.
- [ ] Verify document deletion preserves stats, while word deletion removes relevant history and recomputes graphs.

### Phase 6 — External integrations
- [ ] Add optional AnkiDroid note model, deck configuration, duplicate detection and word popup action.
- [ ] Build Rust Nostr relay/client configuration, curated-author filtering, Japanese text heuristic, dedup and partial failure handling.
- [ ] Implement queue/current/previous/skip/refresh behavior, local reading semantics and metadata display.
- [ ] Implement images/cache/viewer, external media links and reply View Parent.
- [ ] Add generated encrypted Nostr identity, reactions/undo via kind-5, persistent offline reaction queue.
- [ ] Implement Citrine forwarding **after resolving Q188**.

### Phase 7 — Backup, hardening, and release
- [ ] Versioned password-encrypted full export/replace restore; destructive failure tests.
- [ ] Performance tests with larger EPUBs, Sudachi Full, dictionary size, many imported books, heavy vocab history.
- [ ] Tablet + phone, portrait + landscape, dark/light, airplane mode, process death and Android lifecycle checks.
- [ ] Review privacy/security, licenses/attribution, asset packaging, error states and accessibility.
- [ ] CI checks, release APK/ABI/signing instructions, GitHub Releases and Zapstore handoff.

**Implementation strategy:** Prefer completing a **vertical slice** before implementing every import format or external feature. A sensible first milestone is **paste text → segment → display pages → tap word → set status → advance page → persist position/encounter history**. Keep incremental builds green.

## 11. Critical test matrix (add to plan)

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

## 12. Open questions and decision discipline

**Known unresolved:** **Question 188:** If a completed Nostr post is to be forwarded to optional Citrine while Citrine is offline, should Yomoku (A) persist it in a retry queue or (B) only attempt forwarding immediately and drop on failure? Ask the user before implementing that behavior. The existence of an **offline retry queue for heart reactions** is a *separate, already-confirmed requirement*.

Actual curated Nostr starter npubs and relay URLs, technical license/data packaging, final text-token offset encoding, and any dependency API feasibility must be settled during implementation, not invented as product facts.

For routine coding conventions or reversible implementation details, choose a simple, well-tested option and record it in `docs/DECISIONS.md`. For **architecture, new libraries with major cost, data-destructive semantics, or changes to confirmed product behavior**, ask the user. If blocked, work on another independent checklist item rather than consuming an entire session waiting for confirmation.

## 13. Exact end-of-session report format

Use a short final message resembling:

```text
Implemented:
- [x] <precise completed plan task>

Files changed:
- path/to/file — why

Verified:
- <command> — PASS / FAIL / NOT RUN (+ reason)

Next:
1. <next task>
2. <next task>

Blockers/questions:
- None / <concise question>

Updated: docs/IMPLEMENTATION_PLAN.md, docs/NEXT_SESSION.md
```

**Start immediately:** inspect repository, write the persistent checklist and resume file, then implement the first feasible unchecked item in the current session.
