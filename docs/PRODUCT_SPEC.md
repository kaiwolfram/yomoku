# Yomoku product specification

**Yomoku** — *“Read Japanese, naturally.”* A native, offline-first Android app
for reading Japanese books, text, and subtitles while
tracking vocabulary and reading progress. Inspired by Lute, but this is its own
product.

This is the authoritative product scope, behavior and required architecture.
It describes requirements, not implementation status. Confirmed requirements
must not change without user approval.

- [Architecture](ARCHITECTURE.md) and [data model](DATA_MODEL.md): implemented design and remaining boundaries.
- [Decisions](DECISIONS.md): technical decisions and dependency evidence.
- [Development](DEVELOPMENT.md): CLI setup and verified commands.
- [Agent workflow](../AGENTS.md): coding-session instructions.

## 1. Product scope and priorities

**v1 includes:** native Android app; native Japanese reader; local library of imported EPUB/TXT/SRT and pasted text; offline word segmentation and dictionaries; vocabulary status workflows; historical reading/vocabulary statistics; optional AnkiDroid integration; secure manual encrypted backups; configurable settings; release tooling.

**Important late scope changes that override earlier brainstorming:**

- **Remove Known forms statistics** (user scope reduction, 2026-10-31): no separate Known surface/dictionary-form chart or dedicated dictionary-form aggregation storage. The five-status vocabulary chart still includes Known.
- **Feed/Nostr is no longer planned** (user scope reduction, 2026-11-02). Remove the Feed placeholder from phone bottom navigation and the tablet rail. No relay configuration/networking, post reading, signing identity, reactions or forwarding. Section 5 retains historical requirements only.
- **No in-app AI features in v1**, including BYOK API keys/provider/model settings, AI-generated definitions or translations, and AI regeneration. Do not scaffold AI merely because older design notes might mention it.
- **Only EPUB, TXT, SRT, and pasted text**. No WebVTT `.vtt`, no ASS/SSA `.ass`, no PDF, no NHK integration, and no Android Share-intent import in v1.
- **No Nostr multi-device sync or Blossom cloud backups in v1.** Secure **manual encrypted export/import** remains.
- No full-text book search, no bookmarks, no editing of imported content, no in-app audio/video players, no custom goals or streaks, and no calendar heatmap.
- An online marketing site (possibly `yomoku.app`) is not part of the essential Android MVP. Concentrate on the app and tests first.

**Priority order:** dependable persistence / accurate reading and vocabulary semantics > robust imports / dictionary lookup > usable UI > charts > external integrations > release polishing. Do not start with Nostr, signing, or intricate UI styling before the local reader works.

## 2. Required technical architecture

- **Android UI:** Kotlin with **Jetpack Compose**. Phone/tablet responsive layouts. Android min SDK **29** initially; reassess later if needed.
- **Domain/application logic:** Rust **Crux** core, ideally pure and testable without Android. Domain invariants and commands belong here.
- **Japanese analysis:** Rust **Sudachi.rs**, bundled **Sudachi Full dictionary**, default segmentation **Mode B** with A/C options where relevant.
- **Offline dictionary data:** bundle **JMdict (English definitions)** and **JMnedict (names)** with the APK. Version them and satisfy redistribution/attribution requirements. Both sources appear in separate sections when both match.
- **Persistence:** SQLite, accessed from a separate Rust I/O/persistence layer using `rusqlite` / `tokio-rusqlite` as suitable. Keep SQL and Android-specific I/O **outside the pure Crux core**. Track schema migrations from the start.
- **Rust effects:** Rust-side `EffectRouter` executes Rust I/O. The first MVP has no Nostr networking. If Nostr is resumed later, networking belongs in Rust through **`nostr-sdk`**, not duplicated in Kotlin.
- **Android bridge:** Kotlin handles Compose, file picker, notifications/background execution, Android Keystore, AnkiDroid provider integration, and platform APIs; expose a narrowly scoped Rust FFI interface. Prefer explicit, typed state/actions over ad hoc strings.
- **Workstation workflow:** Gradle/Cargo CLI, terminal-based Android SDK/emulator/`adb`; **no Android Studio requirement** for normal build/test/run. Document native library build, ABI packaging, emulator setup, and release signing without committing secrets.
- **Repository:** MIT license for original code, respecting third-party licenses; clear module boundaries; local Rust/Kotlin tests, lint, and Android builds. CI/GitHub Actions is not required (user clarification 2026-10-27). Manual approval for signed publishing. Eventual APK distribution via GitHub Releases and Zapstore, not Google Play/F-Droid for initial publishing.

If a chosen library does not support a required capability or architecture in current versions, validate its actual API and explain the minimal deviation before changing a major architectural choice. Avoid blind dependency/version guesses.

## 3. Reader, text processing, and vocabulary behavior

### 3.1 Documents and pagination

- App opens on **Library**. Four main navigation items: **Library, Statistics, Vocabulary, Settings**. Settings is accessible only through the phone bottom navigation or tablet rail; keep navigation visible in Settings and remove toolbar and reader shortcuts to Settings. Hide main navigation while actively reading or pasting text.
- Reader is **horizontal text only**, single centered column even on landscape tablets; comfortable maximum text width, tablet margins. Dictionary panel on wide displays, bottom sheet on phones. Both portrait and landscape supported.
- Logical page length setting has **presets 250 / 500 / 750 / 1,000 / 1,500 characters**, default **500**. Count all non-whitespace Unicode characters, including punctuation and Latin text (user clarification 2026-10-22); supplementary characters count once. Split at a paragraph boundary after reaching the threshold; if a paragraph is extremely long, split after an appropriate following sentence. **Never cross an EPUB chapter boundary** when forming a logical page. TXT has no automatically inferred chapter boundaries.
- Logical pages may **scroll vertically**, e.g. for long text or images. **Never persist vertical scroll positions**: navigating forward or backward, or reopening a document, starts at the top of the destination page. Page boundaries don't depend on screen dimensions. Page-length changes reflow pages while preserving the exact underlying reading text offset.
- **Previous** and **Next** controls, with counter `12 / 85` between them, appear at the bottom of the page content. Left swipe = next logical page, right swipe = previous, even before the bottom is reached. **No bottom-reached requirement**. Both forward and backward navigation update the *current* reading position; the Library's percentage uses current text offset / total text length, so it can decrease if the reader goes backward.
- Advancing **forward** completes the departing page, even if the user swiped without reaching the bottom. Simply moving **backward** does not complete or count the page. Reading a previously read page and completing it again **does count again** (idempotency must prevent accidental double-trigger/tap counting).
- Incomplete pages save position and pending time, **not** read-word events/counts/status updates until page completion. On page completion, every Japanese word occurrence (including repetitions) contributes to encounter/reading counts; any still-**New** unique words present on that page are changed directly to **Known**. Don't automatically change Learning/Familiar/Ignored. Show toast `N new words marked as Known` **only when N > 0**.
- Full-document completion: maintain permanent **finished-at-least-once** checkmark independent of current position. Show brief confetti on each completion; **Finish Book** returns to Library. Reopening a finished book begins at page 1. Deleting and later reimporting resets progress and checkmark, while global history survives.
- Reader styling: bundled **Noto Serif JP**; adjustable global font size and line spacing; furigana switch **off by default**, when on show available ruby readings (source ruby when provided, else analyzer), irrespective of vocabulary status.
- EPUB supports chapters, headings, emphasis, inline images, links, and source ruby, **but ignore EPUB CSS**. Inline images keep position/ratio, fit column width; tapping opens full-screen pinch-zoom/pan viewer. Internal footnote/chapter links are **not navigable**; external website links open external browser. No TOC-jump feature.
- **Basic SVG image support is sufficient for v1** (user clarification 2026-10-27). Keep the existing bounded vectors/basic styles and self-contained embedded images. Complex SVG styling/resources (advanced stylesheet rules, patterns, masks, markers and additional resource loading) are outside v1 scope; unsupported images retain the alt/Retry or generic-cover fallback. EPUB page CSS remains ignored.
- Clipboard **Copy Sentence** was discussed but explicitly **deferred beyond v1**. Don't implement in first release.

### 3.2 Vocabulary model and dictionary panel

- Five globally tracked statuses: **New**, **Learning**, **Familiar**, **Known**, **Ignored**.
- Status colors: New blue (`#A9CCFF` light / `#28558D` dark), Learning light red (`#FFC5C0` / `#783E44`), Familiar yellow (`#FFE29A` / `#725A2D`), **Known unhighlighted in text** (green in graphs), Ignored pronounced gray (`#BEC5CF` / `#505B6B`). Follow Android light/dark system theme; **no in-app manual theme override**.
- Vocabulary identity is the **exact written surface form**, NOT a lemma: `食べる`, `食べた`, `食べます` are separate entries with separate statuses and encounter counts. Same spelling with different readings is **one** status entry; show all appropriate dictionary readings/senses. Retain Sudachi dictionary forms where needed for dictionary lookup; no aggregate lemma statistics.
- **No manual correction of surface→dictionary-form mappings in v1** (complexity reduction). Don't edit built-in JMdict/JMnedict definitions; allow user-created, editable/deletable **English meaning + reading** dictionary entry when no dictionary match exists. No global overriding of built-in definitions.
- Default every newly encountered/imported written Japanese word to **New**. Upon **book import**, register **all unique word surfaces in its content immediately**, not lazily only when read. Deferred Nostr behavior: register vocabulary when a post is **first displayed**, even if it is later skipped. Registration is different from a **reading encounter**.
- Word tap opens dictionary UI with original surface form, readings, linked base form, all **JMdict English senses immediately expanded and grouped by sense**, compact human-readable grammatical labels (e.g. noun, Ichidan/Godan verb, adjectives), and **JMnedict name readings/categories in a separate section** when available. Proper names are treated like other vocabulary, not excluded.
- Display **`Encountered 47 times`** (lifetime completed-reading occurrence count) for that surface. Opening popup does **not** change status; use one visible row of **five direct status choices** with no increment/decrement controls. Show Anki integration action if applicable.
- Sudachi uses **Mode C (longest tokens)** for highlighting, vocabulary registration and completed-reading occurrences. There is no manual boundary editing. The dictionary panel shows all applicable entries for the selected token and its base form, plus its finer A/B components and their base forms. Full-token results appear first, then longer components before shorter ones, with source position breaking ties. Deduplicate entries while preserving applicable readings/senses and matched forms. Component results are informational; status, encounters, custom entries and Anki actions belong to the highlighted C surface.
- No preknown vocabulary list. Nothing is implicitly Known from parser import, dictionary lookup, or Anki export.

### 3.3 Imports and library

- Supported **EPUB, TXT, SRT**, and **Paste Text**. Imported source content is **immutable**; users may rename but not edit. To change text, delete/reimport.
- Android picker supports **multiple files per batch**. Per-file import is **atomic**; one error never aborts other files. Detect an **exact duplicate with SHA-256** and open/reference the existing document instead of creating another copy. Upon new import register all unique surface forms as New without overwriting already existing statuses.
- Titles: EPUB metadata where present, otherwise filename; pasted text uses first line as initial title; user can rename.
- TXT: UTF-8 and Shift-JIS; attempt appropriate detection and allow manual selection when uncertain. Preserve original hard line breaks; no reflow of adjacent lines. Parse **common Aozora Bunko markup**, including ruby `｜漢字《かんじ》`, and hide/interpret common typographic directives. Don't infer chapters from titles such as `第一章`.
- SRT: best-effort parsing of mildly malformed files; strip timestamps/cue numbers and formatting tags (e.g. `<i>`), retain **separate cue blocks**, breaks within cues, and sensible spacing; do not merge successive cues into prose. Preserve speaker indications when meaningful. No VTT or ASS support in v1.
- Show **batch import progress** with current file and X of N; permit **Cancel** (retain completed imports; discard current partial and remaining). Continue imports on Android while user opens another app when permitted by OS background policies; show progress/completion notifications. If Android kills the process, **keep finished imports but do NOT automatically resume** the unfinished batch on next launch. Do not misrepresent OS guarantees.
- On any completed import (single/batch), show summary and return to Library; **never automatically open reader**.
- Library: compact list with EPUB cover thumbnails (cache) or generic TXT/SRT/paste icons; title, current progress, permanent checkmark, and a **stacked five-status distribution bar of distinct word surfaces present in that document** (not token frequencies), updated whenever global statuses change. The top bar has a **+** menu for **Paste Text** and **Import Files**. Search titles; sort last read (default), title, import date with an ascending/descending arrow and a control to reverse the order; show new/unopened sensibly. No folders/collections.
- Long-press a document to open a **Rename / Delete** dropdown; no multi-selection or inline action buttons. Delete individually with confirmation; **no Undo**. Deleting removes content/current position and preserves source metadata needed for historical event provenance, historical reading stats, encounter records, and all global vocab statuses. If reimported later, treat as a **new document starting at 0%**, no restored completion checkmark.

## 4. Statistics, durable history, and vocabulary browser

### 4.1 Events and statistics

- Design schema around immutable, dated **status-change events**, **reading-completion events**, **every Japanese surface occurrence encountered on completed pages/posts**, source identity/provenance, and **active reading time**. History matters more than cached counters; use migration-safe IDs and explicit versioned semantics.
- Multiple occurrences of a word on one completed page count multiple times. Re-reading counts again. Avoid duplicate counting on accidental repeated click/gesture. Non-Japanese content and punctuation aren't included in Japanese word counts.
- Active time starts while reader open and includes dictionary popups, but pauses in background/when reader closes and after **5 min with no interaction**. Reset idle timer on any interaction. Persist partial-page accumulated time across closing/reopening/reboots; commit time to reading statistics **only after that page/post is completed**. Hidden timer (no timer in reader UI).
- Statistics: overview + tap into detailed charts. Timeframes: **last month**, **last six months**, **last year (default)**, **all time**; group **daily / weekly / weekly / monthly** respectively.
- Graph 1: **cumulative lifetime Japanese word occurrences read**. Time filter changes visible x-range but **not** origin of cumulative y-values (e.g. 85k→100k for last month).
- Graph 2: **five distinct time-series lines** of absolute number of registered word surfaces in each status, with statuses' graph colors; import-time registration causes jumps in New.
- There are **two charts only**: cumulative reading occurrences and the five vocabulary statuses. No separate **Known forms** chart or distinct dictionary-form count. Keep the Known status line in Graph 2 and dictionary-form metadata required by reader/dictionary/history workflows.
- No calendar heatmaps, goals, streaks, or per-source charts in v1 (keep source identities internally).
- **Vocabulary deletion semantics differ from document deletion:** permanently deleting a vocabulary entry wipes its encounter/status history and recalculates history-based stats. If the form appears again later, it starts as New; existing documents are not automatically rescanned to recreate it just because a word was deleted.

### 4.2 Vocabulary browser

- Separate top-level **Vocabulary** screen.
- Search by **written-surface prefix** only, indexed; no reading/lemma/full-text search.
- Default sort newest registration first; alternatives frequency, last seen, alphabetical.
- Filter by five statuses; show total matching count; query 100 at a time with infinite scroll.
- Select All targets **every matching row, including results not loaded on screen**. Bulk change status or permanently delete with confirmation. Record status change events; enforce delete/history semantics above.

## 5. Historical Feed/Nostr requirements (no longer planned)

Feed/Nostr is no longer planned. Neither phone bottom navigation nor the tablet
rail exposes a Feed placeholder.

The following Nostr requirements are retained as historical reference only;
they must not drive implementation unless the user explicitly reopens this scope.

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

## 6. AnkiDroid integration

- Only when AnkiDroid is installed and its supported ContentProvider integration is available; app works fully without AnkiDroid.
- From word dictionary UI, **Add to Anki** to a configured default deck, using a Yomitan-inspired HTML/CSS Yomoku note model. Detect duplicate note for the **same exact written surface** even if note was made elsewhere when feasible via supported API; show existing-note indicator/open action.
- Front: **the word only**. Back: kana reading, English meaning(s), and the original Japanese sentence with target word highlighted. **No audio**. Export does **not** change Yomoku vocabulary status.
- Confirm what the current AnkiDroid API actually supports before finalizing duplicate handling; don't invent APIs.

## 7. Secure manual backup and restore

- **Manual export/import only**: all content, reading position/completion, vocabulary and history, settings, and custom dictionary records. The first MVP has no Nostr mnemonic/identity to export. No Nostr sync, Blossom, cloud account, or automatic backups.
- Password-encrypted archive with a well-reviewed authenticated design (e.g. **Argon2id** KDF and **AEAD** encryption); record schema/format version, salt/nonce/KDF parameters. Do not invent home-grown cryptography or log secrets.
- Restore is a **complete replacement, never merge**. Authenticate/decrypt and validate backup before destructive work, warn and confirm before replacement, swap atomically/transactionally as possible. An incorrect password/corrupt backup must **leave existing installation intact**.
- **Resolve current Anki recovery before restore** (user clarification, 2026-11-01): pending/uncertain Anki note or model writes in the current installation must be resolved before replacement. A generic replacement confirmation cannot discard those records. Preserve the backup's own recovery records without replaying external writes.
- Tests: correct password round trip, bad password, tampering, interrupted restore, new version/backward-compatible schema strategy, and complete state restoration.

## 8. UX and other confirmed decisions

- App title **Yomoku**, tagline *Read Japanese, naturally.* Branding concept: dark navy `#1B2637`, off-white open-book motif, blue `#579FF5` highlighted word; apply after core app is functional.
- Follow **Android system light/dark mode** automatically; no in-app override.
- All user-facing interface and offline definitions **English**.
- No Undo in v1; destructive bulk actions require confirmation.
- No reading-session timer visible, and no mandatory bottom-of-page reached check.
- Keep designs compact, tablet-first but usable on phones. Don't add unrequested document editing, longform Nostr support, AI, etc.

## 9. Open questions and inputs

**Deferred beyond the first MVP:** **Question 188:** If a completed Nostr post is to be forwarded to optional Citrine while Citrine is offline, should Yomoku (A) persist it in a retry queue or (B) only attempt forwarding immediately and drop on failure? Ask the user before implementing that behavior. The existence of an **offline retry queue for heart reactions** is a *separate, already-confirmed requirement*.

Curated Nostr starter npubs, relay URLs and Q188 are not first-MVP blockers; revisit them only if Nostr integration is resumed. Technical license/data packaging, final text-token offset encoding, and dependency API feasibility must be settled during implementation, not invented as product facts.
