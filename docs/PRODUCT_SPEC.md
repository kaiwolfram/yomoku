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

- **Remove statistics details and reading-time tracking** (user scope reduction, 2026-11-04): Statistics is an overview only; do not collect, persist or aggregate reading durations. Keep the two charts and existing timeframe filters.
- **Remove Known forms statistics** (user scope reduction, 2026-10-31): no separate Known surface/dictionary-form chart or dedicated dictionary-form aggregation storage. The five-status vocabulary chart still includes Known.
- **Feed/Nostr is no longer planned** (user scope reduction, 2026-11-02). Remove the Feed placeholder from phone bottom navigation and the tablet rail. No relay configuration/networking, post reading, signing identity, reactions or forwarding. Section 5 retains historical requirements only.
- **No custom dictionary entries in v1**: no editor, fallback definitions, saved custom records or custom-entry Anki export. Bundled dictionaries alone supply definitions; unmatched words show the no-match state.
- **No in-app AI features in v1**, including BYOK API keys/provider/model settings, AI-generated definitions or translations, and AI regeneration. Do not scaffold AI merely because older design notes might mention it.
- **Only EPUB, TXT, SRT, and pasted text**. No WebVTT `.vtt`, no ASS/SSA `.ass`, no PDF, no NHK integration, and no Android Share-intent import in v1.
- **No Nostr multi-device sync or Blossom cloud backups in v1.** Secure **manual encrypted export/import** remains.
- No full-text book search, no bookmarks, no editing of imported content, no in-app audio/video players, no custom goals or streaks, and no calendar heatmap.
- An online marketing site (possibly `yomoku.app`) is not part of the essential Android MVP. Concentrate on the app and tests first.

**Priority order:** dependable persistence / accurate reading and vocabulary semantics > robust imports / dictionary lookup > usable UI > charts > external integrations > release polishing. Do not start with Nostr, signing, or intricate UI styling before the local reader works.

## 2. Required technical architecture

- **Android UI:** Kotlin with **Jetpack Compose**. Phone/tablet responsive layouts. Android min SDK **29** initially; reassess later if needed.
- **Domain/application logic:** Rust **Crux** core, ideally pure and testable without Android. Domain invariants and commands belong here.
- **Japanese analysis:** Rust **Sudachi.rs**, bundled **Sudachi Full dictionary**, one production reader using stored **Mode C** segmentation; A/B analysis is retained for dictionary components.
- **Offline dictionary data:** bundle **JMdict (English definitions)** and **JMnedict (names)** with the APK. Version them and satisfy redistribution/attribution requirements. Both sources appear in separate sections when both match.
- **Persistence:** SQLite, accessed from a separate Rust I/O/persistence layer using `rusqlite` / `tokio-rusqlite` as suitable. Keep SQL and Android-specific I/O **outside the pure Crux core**. Track schema migrations from the start.
- **Rust effects:** Rust-side `EffectRouter` executes Rust I/O. The first MVP has no Nostr networking. If Nostr is resumed later, networking belongs in Rust through **`nostr-sdk`**, not duplicated in Kotlin.
- **Android bridge:** Kotlin handles Compose, file picker, notifications/background execution, Android Keystore, AnkiDroid provider integration, and platform APIs; expose a narrowly scoped Rust FFI interface. Prefer explicit, typed state/actions over ad hoc strings.
- **Workstation workflow:** Gradle/Cargo CLI, terminal-based Android SDK/emulator/`adb`; **no Android Studio requirement** for normal build/test/run. Document native library build, ABI packaging, emulator setup, and release signing without committing secrets.
- **Repository:** MIT license for original code, respecting third-party licenses; clear module boundaries; local Rust/Kotlin tests, lint, and Android builds. CI/GitHub Actions is not required (user clarification 2026-10-27). Manual approval for signed publishing. Eventual APK distribution via GitHub Releases and Zapstore, not Google Play/F-Droid for initial publishing.

If a chosen library does not support a required capability or architecture in current versions, validate its actual API and explain the minimal deviation before changing a major architectural choice. Avoid blind dependency/version guesses.

## 3. Reader, text processing, and vocabulary behavior

### 3.1 Documents and pagination

- App opens on **Library**. Four main navigation items: **Library, Statistics, Vocabulary, Settings**. Settings is accessible only through the phone bottom navigation or tablet rail; keep navigation visible in Settings and remove toolbar and reader shortcuts to Settings. Settings has no toolbar back button; Android system-back behavior is unchanged. Hide main navigation while actively reading or pasting text.
- Reader is **horizontal text only**, single centered column even on landscape tablets; comfortable maximum text width, tablet margins. Dictionary panel on wide displays, bottom sheet on phones. Both portrait and landscape supported.
- Logical page length setting has **presets 250 / 500 / 750 / 1,000 characters**, default **250**. Existing saved preferences, including 500 and legacy 1,500-character values, remain unchanged without migration or automatic reflow. Count all non-whitespace Unicode characters, including punctuation and Latin text (user clarification 2026-10-22); supplementary characters count once. Split at a paragraph boundary after reaching the threshold; if a paragraph is extremely long, split after an appropriate following sentence. **Never cross an EPUB chapter boundary** when forming a logical page. TXT has no automatically inferred chapter boundaries.
- Logical pages may **scroll vertically**, e.g. for long text or images. **Never persist vertical scroll positions**: navigating forward or backward, or reopening a document, starts at the top of the destination page. Page boundaries don't depend on screen dimensions. Page-length changes reflow pages while preserving the exact underlying reading text offset.
- **Previous** and **Next** controls, with counter `12 / 85` between them, appear at the bottom of the page content. Horizontal swipes do not navigate or complete pages. Use **Previous**, **Next**, or **Finish Book** to navigate. Both forward and backward navigation update the *current* reading position; the Library's percentage uses current text offset / total text length, so it can decrease if the reader goes backward.
- Advancing **forward** completes the departing page. Simply moving **backward** does not complete or count the page. Reading a previously read page and completing it again **does count again** (idempotency must prevent accidental double-trigger/tap counting).
- Incomplete pages save position, **not** read-word events/counts/status updates until page completion. On page completion, every Japanese word occurrence (including repetitions) contributes to encounter/reading counts; any still-**New** unique words present on that page are changed directly to **Known**. Don't automatically change Learning/Familiar/Ignored. Show toast `N new words marked as Known` **only when N > 0**.
- Full-document completion: maintain permanent **finished-at-least-once** checkmark independent of current position. Show brief confetti on each completion; **Finish Book** returns to Library. Reopening a finished book begins at page 1. Deleting and later reimporting resets progress and checkmark, while global history survives.
- Reader toolbar title uses one line with an ellipsis and an accessible Back arrow. The page counter stays centered between equal-width controls, including Finish Book.
- Reader styling: bundled **Noto Serif JP**; adjustable global font size and line spacing; furigana switch **off by default**, when on show available ruby readings (source ruby when provided, else analyzer), irrespective of vocabulary status.
- EPUB supports chapters, headings, emphasis, inline images, links, and source ruby, **but ignore EPUB CSS**. Inline images keep position/ratio, fit column width; tapping opens full-screen pinch-zoom/pan viewer. Internal footnote/chapter links are **not navigable**; external website links open external browser. No TOC-jump feature.
- **Basic SVG image support is sufficient for v1** (user clarification 2026-10-27). Keep the existing bounded vectors/basic styles and self-contained embedded images. Complex SVG styling/resources (advanced stylesheet rules, patterns, masks, markers and additional resource loading) are outside v1 scope; unsupported images retain the alt/Retry or generic-cover fallback. EPUB page CSS remains ignored.
- Clipboard **Copy Sentence** was discussed but explicitly **deferred beyond v1**. Don't implement in first release.

### 3.2 Vocabulary model and dictionary panel

- Five globally tracked statuses: **New**, **Learning**, **Familiar**, **Known**, **Ignored**.
- Status colors: New blue (`#A9CCFF` light / `#28558D` dark), Learning light red (`#FFC5C0` / `#783E44`), Familiar yellow (`#FFE29A` / `#725A2D`), **Known unhighlighted in text** (green in graphs), Ignored pronounced gray (`#BEC5CF` / `#505B6B`). Word highlights inset each visible fragment by 1 dp on each horizontal side without altering text or tap ranges. Follow Android light/dark system theme; **no in-app manual theme override**.
- Vocabulary identity is the **exact written surface form**, NOT a lemma: `食べる`, `食べた`, `食べます` are separate entries with separate statuses and encounter counts. Same spelling with different readings is **one** status entry; show all appropriate dictionary readings/senses. Retain Sudachi dictionary forms where needed for dictionary lookup; no aggregate lemma statistics.
- **No manual correction of surface→dictionary-form mappings in v1** (complexity reduction). Do not edit bundled JMdict/JMnedict definitions or add user-created definitions. When bundled dictionaries have no result, show the existing no-match state.
- Default every newly encountered/imported written Japanese word to **New**. Upon **book import**, register **all unique word surfaces in its content immediately**, not lazily only when read. Deferred Nostr behavior: register vocabulary when a post is **first displayed**, even if it is later skipped. Registration is different from a **reading encounter**.
- Word tap opens dictionary UI with the original surface header and entries containing applicable readings/base forms, all **JMdict English senses immediately expanded and grouped by sense**, compact human-readable grammatical labels (e.g. noun, Ichidan/Godan verb, adjectives), and **JMnedict name readings/categories in a separate section** when available. Proper names are treated like other vocabulary, not excluded.
- Display **`Encountered 47 times`** (lifetime completed-reading occurrence count) for that surface. Opening popup does **not** change status; use one visible **segmented pill with five direct status choices**, colored New blue, Learning coral, Familiar amber, Known green and Ignored gray. Clearly mark the selected section with straight interior edges and rounded outward corners only on the end segments; no increment/decrement controls. Tapping the selected status dismisses the panel without a status-change event; tapping another status persists it and keeps the panel open. There is no Close button; outside tap and system back also dismiss it. Omit standalone reading and dictionary-form header rows: dictionary entries supply those details.
- New imports use **Sudachi Mode C followed by deterministic conjugation joining** for highlighting, exact-surface vocabulary registration and completed-reading occurrences. Join adjacent verb/adjective stems and grammatical auxiliaries (including polite, negative, past, passive and causative forms), plus `て`/`で` followed by Sudachi non-independent helper verbs with lemmas `いる`, `ある`, `おく`, `しまう`, `みる`, `いく`, `くる`, `くれる`, `もらう`, or `あげる`; helpers and endings may repeat. For example, `できません`, `食べました`, `高くない`, `食べている`, and `食べてしまった` each form one highlighted surface. Use POS/conjugation metadata and exact source adjacency; stop at whitespace, punctuation, sentence boundaries, other particles, and independent words. Standalone auxiliaries and ordinary lexical helper uses retain their boundaries. Preserve the exact source surface/span, the head’s dictionary form, and concatenated analyzer readings only when every part has one. Version this analysis profile; existing books keep immutable boundaries/history and support lazy raw A/B dictionary components under their original profile. Deleting and reimporting explicitly adopts the new profile with the existing progress-reset behavior. There is no manual boundary editing. The dictionary panel shows all applicable entries for the selected token and its base form, plus its finer A/B components and their base forms. Full-token results appear first, then longer components before shorter ones, with source position breaking ties. Deduplicate entries while preserving applicable readings/senses and matched forms. Status, encounters and Library distributions belong to the highlighted joined C surface. Anki actions belong to individual dictionary entries, including components and their base forms.
- Custom dictionary entries are excluded. When bundled dictionaries have no result, show the no-match state.
- Show a small star labeled **High priority entry** beside qualifying JMdict headword/reading pairs, independently of Anki setup. Priority uses `ichi1`, `news1`, `gai1`, `spec1` or `spec2`; a written pair must share a qualifying tag in its spelling and reading, while kana-only pairs use reading tags alone. Do not inherit priority from other forms or reorder results.
- No preknown vocabulary list. Nothing is implicitly Known from parser import, dictionary lookup, or Anki export.

### 3.3 Imports and library

- Supported **EPUB, TXT, SRT**, and **Paste Text**. Imported source content is **immutable**; users may rename but not edit. To change text, delete/reimport.
- Android picker selects **one book at a time**; cancelling the picker leaves application state unchanged. Each import is **atomic**. Detect an **exact duplicate with SHA-256** and reference the existing document instead of creating another copy. Upon new import register all unique surface forms as New without overwriting already existing statuses.
- Titles: EPUB metadata where present, otherwise filename; pasted text uses first line as initial title; user can rename.
- EPUB compatibility: if the spine repeats a manifest item, import it once at its first position and skip later references. Preserve the original file bytes.
- TXT: UTF-8 and Shift-JIS; attempt appropriate detection and allow manual selection when uncertain. Preserve original hard line breaks; no reflow of adjacent lines. Parse **common Aozora Bunko markup**, including ruby `｜漢字《かんじ》`, and hide/interpret common typographic directives. Don't infer chapters from titles such as `第一章`.
- SRT: best-effort parsing of mildly malformed files; strip timestamps/cue numbers and formatting tags (e.g. `<i>`), retain **separate cue blocks**, breaks within cues, and sensible spacing; do not merge successive cues into prose. Preserve speaker indications when meaningful. No VTT or ASS support in v1.
- Show the current filename with **indeterminate import progress**, without file counts; permit **Cancel** to discard an uncommitted import. A successful commit wins a cancellation race. Retain encoding selection and retries; there is no Skip File action. Continue imports on Android while user opens another app when permitted by OS background policies; show progress/completion notifications. If Android kills the process, **keep committed imports but do NOT automatically resume** the unfinished import on next launch. Do not misrepresent OS guarantees.
- On any completed import, show a single success, duplicate, or error result and return to Library; **never automatically open reader**.
- Library: compact list with EPUB cover thumbnails (cache) or generic TXT/SRT/paste icons; title, current progress, permanent checkmark, and a **stacked five-status distribution bar of distinct word surfaces present in that document** (not token frequencies), updated whenever global statuses change. The top bar has a **+** menu for **Paste Text** and **Import book**. No Library search. With two or more documents, show sorting by last read (default), title, or import date with an ascending/descending arrow and a control to reverse the order; preserve the selected order below that threshold. Show new/unopened sensibly. The empty state centers a book icon, “No documents yet”, and “Tap + to import a book or paste text.” horizontally and vertically beneath status messages; message and hint use the Material `onSurfaceVariant` color. No folders/collections.
- Long-press a document to open a **Rename / Delete** dropdown; no multi-selection or inline action buttons. Delete individually with confirmation; **no Undo**. Deleting removes content/current position and preserves source metadata needed for historical event provenance, historical reading stats, encounter records, and all global vocab statuses. If reimported later, treat as a **new document starting at 0%**, no restored completion checkmark.

## 4. Statistics, durable history, and vocabulary browser

### 4.1 Events and statistics

- Design schema around immutable, dated **status-change events**, **reading-completion events**, **every Japanese surface occurrence encountered on completed pages/posts**, and source identity/provenance. History matters more than cached counters; use migration-safe IDs and explicit versioned semantics.
- Multiple occurrences of a word on one completed page count multiple times. Re-reading counts again. Avoid duplicate counting on accidental repeated click/gesture. Non-Japanese content and punctuation aren't included in Japanese word counts.
- Statistics: one overview with two charts; no detail pages or drilldown actions. Timeframes in one horizontally scrolling row: **last week (default)**, **last month**, **last six months**, **last year**, **all time**; group **daily / daily / weekly / weekly / monthly** respectively. Last week includes today and the preceding six UTC dates, through the supplied current instant. Persist the selected timeframe across restarts and keep its chip visible on entry.
- Graph 1: **cumulative lifetime Japanese word occurrences read**. Time filter changes visible x-range but **not** origin of cumulative y-values (e.g. 85k→100k for last month).
- Graph 2: **five distinct time-series lines** of absolute number of registered word surfaces in each status, with statuses' graph colors; import-time registration causes jumps in New.
- There are **two charts only**: cumulative reading occurrences and the five vocabulary statuses. No separate **Known forms** chart or distinct dictionary-form count. Keep the Known status line in Graph 2 and dictionary-form metadata required by reader/dictionary/history workflows.
- No calendar heatmaps, goals, streaks, or per-source charts in v1 (keep source identities internally).
- Vocabulary entries cannot be deleted through the app. Document deletion preserves vocabulary and its encounter/status history. Previously deleted vocabulary is not automatically recovered.

### 4.2 Vocabulary browser

- Separate top-level **Vocabulary** screen.
- Search by **written-surface prefix** only, indexed; no reading/lemma/full-text search.
- Default sort newest registration first; alternatives frequency, last seen, alphabetical. The arrow reverses the complete ordering; choosing a criterion restores its default direction (descending metrics, ascending alphabetical).
- Filter by five statuses; show total matching count; query 100 at a time with infinite scroll.
- Search scrolls with the list, above selection actions, filters, sorting and matching count.
- A normal word tap opens the shared Word sheet. Long press enters selection mode; taps then toggle selection. Select All is available only in selection mode and targets **every matching row, including results not loaded on screen**. Bulk status changes require confirmation and record status events. There are no vocabulary deletion controls.
- Vocabulary uses the reader's surface header, encounter count, expanded definitions/readings/grammatical labels, separate names, priority markers, and five-status pill. Anki actions remain in the reader. Analyze the exact saved surface temporarily with the bundled analyzer/profile, independently of source passages or book availability; show applicable full-surface/base results, then A/B components with the reader's ordering and deduplication. Lookup never registers words, persists analysis, records encounters, or changes status.
- Choosing another status persists immediately without confirmation and keeps the sheet open, including when the word leaves the current filter. Choosing the current status, outside tap, and Back dismiss. Successful writes refresh Vocabulary results and invalidate Statistics, Library distributions, and retained reader statuses. Lookup failures require explicit Retry; failed status commands retain their original ID, status, and timestamp for Retry or Cancel, even after sheet dismissal. Ignore stale lookup responses after dismissal or navigation.

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
- Configure AnkiDroid permission, the default deck and the built-in **Yomoku** HTML/CSS note model in **Settings only**. Preserve existing models and cards. Once setup is verified, show a small **green + button beside each applicable dictionary headword/reading pair**; one tap adds that entry with the lowercase `yomoku` tag. Existing duplicate notes remain untouched. No Anki setup or export editor appears in the word sheet.
- Check duplicates by the entry’s **exact written headword and kana reading**, across models/decks when supported, including notes created elsewhere with an identifiable `Reading` field (case-insensitive field name). Notes without a verified matching reading do not block adding. Do not split multi-reading fields or normalize written forms/readings. Show a **pink button with a blue book icon** for existing notes; it opens AnkiDroid’s browser filtered to the verified note IDs. Check on lookup, after returning from Anki, and again before insertion. Failed checks must never imply absence.
- The model is unversioned. There are no legacy model definitions, compatibility paths or migrations; existing AnkiDroid notes/models are not automatically changed or deleted.
- Front: **the dictionary headword only** (kana for kana-only entries). Back: the selected kana reading, dictionary source, numbered applicable English senses with grammatical/usage labels and sense notes, and the original Japanese sentence with the matched source occurrence highlighted. The source can be inflected, e.g. 食べた for 食べる; component highlights use retained analyzer offsets. Use compact readable light/dark styling. Add and book buttons have a 32 dp visible background, 16 dp icon and 48 dp touch target. **No audio**. Export does **not** change Yomoku vocabulary status.
- Export preparation, fresh query results and active-operation ownership remain in memory. Repeated taps and stale callbacks cannot submit an active operation twice; navigation does not retarget it. Revalidate permission, deck and model before writes, and refresh exact-headword-and-reading queries on lookup, returning to the app and immediately before insertion. Failed or incomplete queries remain errors. After interruption, perform fresh queries before an explicit retry; never replay a write automatically. There is no durable recovery or exactly-once guarantee across process death. An interrupted write may have completed partially; subsequent actions depend on fresh AnkiDroid observations. Never overwrite an incompatible or partially created model.

- Confirmed AnkiDroid insertions show `Added to AnkiDroid.` once. Duplicate, failed or uncertain results do not show success. App toasts replace the current toast; transient notices are consumed before display.

## 7. Secure manual backup and restore

- **Manual export/import only**: all content, reading position/completion, vocabulary and history, and settings. The first MVP has no Nostr mnemonic/identity to export. No Nostr sync, Blossom, cloud account, or automatic backups.
- Password-encrypted archive with a well-reviewed authenticated design (e.g. **Argon2id** KDF and **AEAD** encryption); record schema/format version, salt/nonce/KDF parameters. Do not invent home-grown cryptography or log secrets.
- Restore is a **complete replacement, never merge**. Authenticate/decrypt and validate backup before destructive work, warn and confirm before replacement, swap atomically/transactionally as possible. An incorrect password/corrupt backup must **leave existing installation intact**.
- Anki export history and recovery records are excluded from saved data and backups. Restoring a Yomoku backup does not require Anki recovery. Restoring never replays external writes.
- Existing development databases and backups are unsupported. Incompatible databases produce an error and are never automatically cleared. The first release uses one initial schema, including the Statistics timeframe preference, with a migration journal for future releases.
- Tests: correct password round trip, bad password, tampering, interrupted restore, new version/backward-compatible schema strategy, and complete state restoration.

## 8. UX and other confirmed decisions

- App title **Yomoku**, tagline *Read Japanese, naturally.* Branding concept: dark navy `#1B2637`, off-white open-book motif, blue `#579FF5` highlighted word; apply after core app is functional.
- Follow **Android system light/dark mode** automatically; no in-app override.
- All user-facing interface and offline definitions **English**.
- No Undo in v1; destructive bulk actions require confirmation.
- No reading-time tracking, and no mandatory bottom-of-page reached check.
- Keep designs compact, tablet-first but usable on phones. Don't add unrequested document editing, longform Nostr support, AI, etc.

## 9. Open questions and inputs

**Deferred beyond the first MVP:** **Question 188:** If a completed Nostr post is to be forwarded to optional Citrine while Citrine is offline, should Yomoku (A) persist it in a retry queue or (B) only attempt forwarding immediately and drop on failure? Ask the user before implementing that behavior. The existence of an **offline retry queue for heart reactions** is a *separate, already-confirmed requirement*.

Curated Nostr starter npubs, relay URLs and Q188 are not first-MVP blockers; revisit them only if Nostr integration is resumed. Technical license/data packaging, final text-token offset encoding, and dependency API feasibility must be settled during implementation, not invented as product facts.
