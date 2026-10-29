//! Exercise the activation candidate through serialized shell events/views, not
//! direct Core updates. Native requests/results still stay inside the FIFO.
use crux_core::bridge::{BincodeFfiFormat, FfiFormat, Request};
use rusqlite::Connection;
use std::{
    convert::Infallible,
    path::PathBuf,
    sync::{
        atomic::{AtomicUsize, Ordering},
        mpsc,
    },
    thread,
    time::Duration,
};
use yomoku_core::{
    active_time::PendingReadingTime,
    analysis::{AnalysisProfile, AnalyzedToken, ChapterAnalysis, OverrideScope, SplitMode},
    app::{ContentRoute, Event, ViewModel},
    document::{ChapterText, DocumentId, PasteText, TextAnchor, TextOffset, TextRange},
    effective_session::{EffectiveOverrideAction as Action, EffectiveReaderSelection},
    library::WordStatusCounts,
    reader::ReaderTokenKey,
    storage::DocumentError,
    typography::ReaderTypography,
    vocabulary::{Surface, VocabularyStatus},
};
use yomoku_ffi::{FfiError, YomokuBridge};
use yomoku_runtime::{AnalyzerLifecycle, AnalyzerLoader, PasteAnalyzer, ShellEffectFfi};
use yomoku_storage::Storage;

const TIMEOUT: Duration = Duration::from_secs(10);
const MEANING: &str = " literal\0meaning 📖 ";
const READING: &str = " か\u{3099}𠮷\0 ";

struct Fixture {
    directory: PathBuf,
    id: DocumentId,
}

fn analysis(chapter: &ChapterText) -> Result<ChapterAnalysis, Infallible> {
    let mut cursor = 0;
    Ok(ChapterAnalysis {
        profile: AnalysisProfile {
            engine_revision: "ffi@1".into(),
            dictionary_revision: "full@1".into(),
        },
        mode: SplitMode::B,
        tokens: chapter
            .text
            .split_whitespace()
            .map(|surface| {
                let start = cursor + chapter.text[cursor..].find(surface).unwrap();
                cursor = start + surface.len();
                AnalyzedToken {
                    anchor: TextAnchor {
                        source_id: chapter.source_id,
                        chapter_id: chapter.chapter_id,
                        range: TextRange {
                            start: TextOffset::at(&chapter.text, start).unwrap(),
                            end: TextOffset::at(&chapter.text, cursor).unwrap(),
                        },
                    },
                    surface: surface.into(),
                    is_japanese: true,
                    reading: Some(format!("読み{surface}")),
                    dictionary_form: Some(Surface::new(surface).unwrap()),
                }
            })
            .collect(),
    })
}

impl Fixture {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let directory = std::env::temp_dir().join(format!(
            "yomoku-effective-ffi-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&directory).unwrap();
        let id = Storage::open(directory.join("user.sqlite3"))
            .unwrap()
            .create_analyzed_paste(
                &PasteText::new("東京 東京 ヌヌ𠮷か\u{3099}\0".into()).unwrap(),
                100,
                analysis,
            )
            .unwrap()
            .document
            .document_id;
        let index = Connection::open(directory.join("words.sqlite3")).unwrap();
        index.execute_batch("PRAGMA application_id=0x594f4449; PRAGMA user_version=1;
            CREATE TABLE entries(source TEXT,entry_id INTEGER,payload TEXT,PRIMARY KEY(source,entry_id)) WITHOUT ROWID;
            CREATE TABLE forms(surface TEXT COLLATE BINARY,source TEXT,entry_id INTEGER,PRIMARY KEY(surface,source,entry_id)) WITHOUT ROWID;").unwrap();
        index.execute("INSERT INTO entries VALUES ('JMdict',1,?1)", [r#"{"kanji":[{"text":"東京"}],"readings":[{"text":"とうきょう"}],"senses":[{"glosses":[{"text":"Tokyo","lang":"eng"}]}]}"#]).unwrap();
        index
            .execute("INSERT INTO forms VALUES ('東京','JMdict',1)", [])
            .unwrap();
        Self { directory, id }
    }

    fn path(&self) -> PathBuf {
        self.directory.join("user.sqlite3")
    }

    fn sql(&self) -> Connection {
        Connection::open(self.path()).unwrap()
    }

    fn bridge(&self) -> Harness {
        self.bridge_with_loader(Box::new(|| {
            panic!("stored effective reads/writes must not load Sudachi")
        }))
    }

    fn bridge_with_loader(&self, loader: AnalyzerLoader) -> Harness {
        let (tx, callbacks) = mpsc::channel();
        let index = self.directory.join("words.sqlite3");
        let (bridge, lifecycle) = YomokuBridge::with_effective_reader(
            self.path(),
            loader,
            Box::new(move || Ok(index.clone())),
            move |result| tx.send((thread::current().id(), result)).unwrap(),
        )
        .unwrap();
        Harness {
            bridge,
            callbacks,
            _lifecycle: lifecycle,
        }
    }

    fn count(&self, table: &str) -> i64 {
        self.sql()
            .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |r| r.get(0))
            .unwrap()
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.directory);
    }
}

struct Harness {
    bridge: YomokuBridge,
    callbacks: mpsc::Receiver<(thread::ThreadId, Result<Vec<u8>, FfiError>)>,
    _lifecycle: AnalyzerLifecycle,
}

fn renders(bytes: &[u8]) {
    let requests: Vec<Request<ShellEffectFfi>> = BincodeFfiFormat::deserialize(bytes).unwrap();
    assert!(!requests.is_empty());
    assert!(requests
        .iter()
        .all(|request| matches!(request.effect, ShellEffectFfi::Render(_))));
}

impl Harness {
    fn view(&self) -> ViewModel {
        BincodeFfiFormat::deserialize(&self.bridge.view().unwrap()).unwrap()
    }

    fn send(&self, event: Event) -> Vec<u8> {
        let mut bytes = Vec::new();
        BincodeFfiFormat::serialize(&mut bytes, &event).unwrap();
        self.bridge.update(&bytes).unwrap()
    }

    fn dispatch(&self, event: Event) -> ViewModel {
        self.dispatch_n(event, 1)
    }

    fn dispatch_n(&self, event: Event, callbacks: usize) -> ViewModel {
        renders(&self.send(event));
        for _ in 0..callbacks {
            let (worker, result) = self.callbacks.recv_timeout(TIMEOUT).unwrap();
            assert_ne!(worker, thread::current().id());
            renders(&result.unwrap());
        }
        let view = self.view();
        assert!(!view.storage_busy);
        assert!(self.callbacks.try_recv().is_err());
        view
    }

    fn load(&self, id: DocumentId) -> ViewModel {
        let opened = self.dispatch(Event::OpenReader(id));
        self.dispatch(Event::LoadReaderTokens(ReaderTokenKey {
            document_id: id,
            page: opened.reader.unwrap().anchor,
            analysis: opened.reader_analysis.unwrap(),
        }))
    }

    fn reload(&self, view: &ViewModel) -> ViewModel {
        self.dispatch(Event::LoadReaderTokens(ReaderTokenKey {
            document_id: view.reader.as_ref().unwrap().document_id,
            page: view.reader.as_ref().unwrap().anchor,
            analysis: view.reader_analysis.unwrap(),
        }))
    }

    fn open_edit(&self, view: &ViewModel) -> ViewModel {
        let page = view.effective_reader_tokens.as_ref().unwrap();
        self.dispatch(Event::EffectiveReaderOverride(Action::Open {
            page: page.identity,
            anchor: page.original_boundaries[0].anchor,
        }))
    }
}

fn selection(view: &ViewModel, surface: &str) -> EffectiveReaderSelection {
    let page = view.effective_reader_tokens.as_ref().unwrap();
    EffectiveReaderSelection {
        page: page.identity,
        segment_index: page
            .tokens
            .iter()
            .find(|t| t.surface == surface)
            .unwrap()
            .segment_index,
    }
}

fn counts(h: &Harness) -> WordStatusCounts {
    h.dispatch(Event::LoadLibraryWordDistributions)
        .library_word_distributions
        .unwrap()[0]
        .counts
        .unwrap()
}

#[test]
fn serialized_edits_reopen_reverse_and_library_use_effective_membership() {
    let f = Fixture::new();
    let h = f.bridge();
    let loaded = h.load(f.id);
    assert!(loaded.reader_tokens.is_none());
    let original = loaded
        .effective_reader_tokens
        .as_ref()
        .unwrap()
        .original_boundaries
        .clone();
    let before = std::fs::read(f.path()).unwrap();
    let opened = h.open_edit(&loaded);
    assert_eq!(std::fs::read(f.path()).unwrap(), before);
    let prompt = opened.effective_reader_override.unwrap().prompt.unwrap();
    assert_eq!(prompt.default_scope, OverrideScope::AllOccurrences);
    assert_eq!(prompt.choices[0].mode, SplitMode::B);
    let saved = h.dispatch(Event::EffectiveReaderOverride(Action::Save {
        identity: prompt.identity,
        scope: OverrideScope::AllOccurrences,
        parts: vec!["東".into(), "京".into()],
        registered_at_unix_ms: 200,
    }));
    assert!(saved.effective_reader_tokens.is_none());
    assert!(
        saved
            .effective_reader_override
            .as_ref()
            .unwrap()
            .reload_required
    );
    let split = h.reload(&saved);
    let page = split.effective_reader_tokens.as_ref().unwrap();
    assert_eq!(page.original_boundaries, original);
    assert_eq!(
        page.tokens
            .iter()
            .map(|t| t.surface.as_str())
            .collect::<Vec<_>>(),
        ["東", "京", "東", "京", "ヌヌ𠮷か\u{3099}\0"]
    );
    assert!(page.tokens[..4]
        .iter()
        .all(|t| t.reading.is_none() && t.dictionary_form.is_none()));
    h.send(Event::CloseDocument);
    let before = std::fs::read(f.path()).unwrap();
    assert_eq!(
        counts(&h),
        WordStatusCounts {
            new: 3,
            ..Default::default()
        }
    );
    assert_eq!(std::fs::read(f.path()).unwrap(), before);
    drop(h);

    let h = f.bridge();
    let split = h.load(f.id);
    assert_eq!(
        split.effective_reader_tokens.as_ref().unwrap().tokens[0].surface,
        "東"
    );
    let opened = h.open_edit(&split);
    let prompt = opened.effective_reader_override.unwrap().prompt.unwrap();
    let rule = prompt.winning_rules[0].id;
    let reversed = h.dispatch(Event::EffectiveReaderOverride(Action::Reverse {
        identity: prompt.identity,
        id: rule,
    }));
    let restored = h.reload(&reversed);
    assert_eq!(
        restored.effective_reader_tokens.as_ref().unwrap().tokens[0].surface,
        "東京"
    );
    h.send(Event::CloseDocument);
    assert_eq!(
        counts(&h),
        WordStatusCounts {
            new: 2,
            ..Default::default()
        }
    );
    assert_eq!(f.count("reading_completions"), 0);
    assert_eq!(f.count("reading_occurrences"), 0);
    // Reversal also survives an independent serialized bridge.
    let fresh = f.bridge();
    assert_eq!(
        fresh.load(f.id).effective_reader_tokens.unwrap().tokens[0].surface,
        "東京"
    );
}

#[test]
fn serialized_word_status_custom_retry_unicode_reopen_and_stale_guards() {
    let f = Fixture::new();
    let h = f.bridge();
    let loaded = h.load(f.id);
    let selected = selection(&loaded, "東京");
    let before = std::fs::read(f.path()).unwrap();
    let word = h.dispatch(Event::LoadEffectiveReaderWord(selected));
    assert_eq!(std::fs::read(f.path()).unwrap(), before);
    let details = word.effective_reader_word.unwrap().details.unwrap();
    assert!(!details.candidates.words.is_empty());
    let id = details.token.vocabulary.unwrap().id;
    for status in [
        VocabularyStatus::New,
        VocabularyStatus::Learning,
        VocabularyStatus::Familiar,
        VocabularyStatus::Known,
        VocabularyStatus::Ignored,
    ] {
        let changed = h.dispatch(Event::SetEffectiveReaderStatus {
            selection: selected,
            vocabulary_id: id,
            status,
            occurred_at_unix_ms: 300,
        });
        assert!(
            changed.effective_reader_tokens.as_ref().unwrap().tokens[..2]
                .iter()
                .all(|t| t.vocabulary.unwrap().status == status)
        );
        assert_eq!(
            changed
                .effective_reader_word
                .unwrap()
                .details
                .unwrap()
                .token
                .vocabulary
                .unwrap()
                .status,
            status
        );
    }
    let missing = selection(&h.view(), "ヌヌ𠮷か\u{3099}\0");
    h.dispatch(Event::LoadEffectiveReaderWord(missing));
    f.sql().execute_batch("CREATE TRIGGER fail_custom BEFORE INSERT ON custom_dictionary_entries BEGIN SELECT RAISE(ABORT,'fixture'); END;").unwrap();
    let failed = h.dispatch(Event::SaveEffectiveReaderCustomEntry {
        selection: missing,
        english_meaning: MEANING.into(),
        reading: READING.into(),
    });
    assert!(failed.effective_reader_word.unwrap().custom_entry_pending);
    assert_eq!(f.count("custom_dictionary_entries"), 0);
    h.send(Event::DismissReaderWord);
    f.sql().execute_batch("DROP TRIGGER fail_custom").unwrap();
    h.dispatch(Event::RetryEffectiveReaderCustomEntry);
    assert_eq!(f.count("custom_dictionary_entries"), 1);
    drop(h);

    let h = f.bridge();
    let loaded = h.load(f.id);
    let selected = selection(&loaded, "ヌヌ𠮷か\u{3099}\0");
    let word = h.dispatch(Event::LoadEffectiveReaderWord(selected));
    let entry = word
        .effective_reader_word
        .unwrap()
        .details
        .unwrap()
        .custom_entry
        .unwrap();
    assert_eq!(
        (entry.english_meaning.as_str(), entry.reading.as_str()),
        (MEANING, READING)
    );
    let reloaded = h.reload(&loaded);
    assert_ne!(
        reloaded.effective_reader_tokens.as_ref().unwrap().identity,
        selected.page
    );
    let before = std::fs::read(f.path()).unwrap();
    h.send(Event::DeleteEffectiveReaderCustomEntry(selected));
    assert!(!h.view().storage_busy);
    assert!(h.callbacks.try_recv().is_err());
    assert_eq!(std::fs::read(f.path()).unwrap(), before);
    let selected = selection(&reloaded, "ヌヌ𠮷か\u{3099}\0");
    h.dispatch(Event::LoadEffectiveReaderWord(selected));
    let deleted = h.dispatch(Event::DeleteEffectiveReaderCustomEntry(selected));
    assert!(deleted
        .effective_reader_word
        .unwrap()
        .details
        .unwrap()
        .custom_entry
        .is_none());
    assert_eq!(f.count("reading_occurrences"), 0);
}

#[test]
fn serialized_completion_freezes_partition_time_and_retry_across_reversal() {
    let f = Fixture::new();
    let h = f.bridge();
    let loaded = h.load(f.id);
    let opened = h.open_edit(&loaded);
    let prompt = opened.effective_reader_override.unwrap().prompt.unwrap();
    let saved = h.dispatch(Event::EffectiveReaderOverride(Action::Save {
        identity: prompt.identity,
        scope: OverrideScope::AllOccurrences,
        parts: vec!["東".into(), "京".into()],
        registered_at_unix_ms: 200,
    }));
    let split = h.reload(&saved);
    h.send(Event::OpenSettings);
    h.dispatch(Event::SetReaderTypography(ReaderTypography {
        furigana: true,
        ..Default::default()
    }));
    h.send(Event::CloseSettings);
    let split = h.reload(&split);
    // Manual parts have no fabricated readings; the untouched exact B token does.
    assert_eq!(split.reader_furigana.len(), 1);
    let time = h.dispatch(Event::RestoreReadingTime).reader_time.unwrap();
    h.dispatch(Event::CheckpointReadingTime(
        yomoku_core::active_time::ReadingTimeUpdate {
            pending: PendingReadingTime::try_from(321).unwrap(),
            ..time
        },
    ));
    f.sql().execute_batch("CREATE TRIGGER fail_completion BEFORE INSERT ON reading_completions BEGIN SELECT RAISE(ABORT,'fixture'); END;").unwrap();
    let page = split.reader.as_ref().unwrap();
    let identity = split.effective_reader_tokens.as_ref().unwrap().identity;
    let failed = h.dispatch_n(
        Event::CompleteEffectiveReaderPage {
            identity,
            position: page.position,
            completed_at_unix_ms: 500,
        },
        2,
    );
    assert!(failed.completion_save_pending && failed.document_error.is_some());
    assert_eq!(f.count("effective_completion_commands"), 1);
    assert_eq!(f.count("reading_occurrences"), 0);
    // An independent connection changes the rule after reservation. Retry must
    // commit the frozen five occurrences and duration, not today's B partition.
    f.sql()
        .execute_batch("UPDATE segmentation_overrides SET active=0; DROP TRIGGER fail_completion;")
        .unwrap();
    let completed = h.dispatch(Event::RetryReaderCompletion);
    assert_eq!(completed.content_route, ContentRoute::Main);
    assert!(!completed.completion_save_pending);
    assert_eq!(completed.completion_notice.unwrap().newly_known, 3);
    assert!(completed.book_completion_notice.is_some());
    assert_eq!(f.count("reading_occurrences"), 5);
    assert_eq!(f.count("reading_completions"), 1);
    assert_eq!(
        f.sql()
            .query_row("SELECT active_time_ms FROM reading_completions", [], |r| {
                r.get::<_, i64>(0)
            })
            .unwrap(),
        321
    );
    h.send(Event::RetryReaderCompletion);
    assert!(h.callbacks.try_recv().is_err());
    drop(h);
    let h = f.bridge();
    let reopened = h.load(f.id);
    assert_eq!(
        reopened.reader.as_ref().unwrap().position.offset,
        TextOffset::START
    );
    assert_eq!(
        reopened.effective_reader_tokens.as_ref().unwrap().tokens[0].surface,
        "東京"
    );
    assert_eq!(f.count("reading_completions"), 1);
}

struct AlternateAnalyzer(mpsc::Sender<(SplitMode, thread::ThreadId)>);

impl PasteAnalyzer for AlternateAnalyzer {
    fn analyze(&mut self, _: &ChapterText) -> Result<ChapterAnalysis, DocumentError> {
        panic!("alternate generation must explicitly request a mode")
    }

    fn analyze_mode(
        &mut self,
        chapter: &ChapterText,
        mode: SplitMode,
        profile: &AnalysisProfile,
    ) -> Result<ChapterAnalysis, DocumentError> {
        self.0.send((mode, thread::current().id())).unwrap();
        assert_eq!(chapter.text, "東京 東京 ヌヌ𠮷か\u{3099}\0");
        let mut output = analysis(chapter).unwrap();
        assert_eq!(&output.profile, profile);
        output.mode = mode;
        if mode == SplitMode::A {
            output.tokens = output
                .tokens
                .into_iter()
                .flat_map(|token| {
                    if token.surface != "東京" {
                        return vec![token];
                    }
                    let start = token.anchor.range.start.byte_index(&chapter.text).unwrap();
                    ["東", "京"]
                        .into_iter()
                        .enumerate()
                        .map(|(index, surface)| {
                            let start = start + index * surface.len();
                            AnalyzedToken {
                                anchor: TextAnchor {
                                    range: TextRange {
                                        start: TextOffset::at(&chapter.text, start).unwrap(),
                                        end: TextOffset::at(&chapter.text, start + surface.len())
                                            .unwrap(),
                                    },
                                    ..token.anchor
                                },
                                surface: surface.into(),
                                reading: Some(format!("A{surface}")),
                                dictionary_form: Some(Surface::new(surface).unwrap()),
                                is_japanese: true,
                            }
                        })
                        .collect()
                })
                .collect();
        }
        Ok(output)
    }
}

#[test]
fn serialized_alternate_generation_choose_local_scope_and_stored_reuse() {
    let f = Fixture::new();
    let (tx, trace) = mpsc::channel();
    let h = f.bridge_with_loader(Box::new(move || {
        Ok(Box::new(AlternateAnalyzer(tx.clone())))
    }));
    let loaded = h.load(f.id);
    assert!(trace.try_recv().is_err());
    let mut current = loaded;
    for mode in [SplitMode::A, SplitMode::C] {
        let opened = h.open_edit(&current);
        let identity = opened.effective_reader_override.unwrap().identity.unwrap();
        let generated = h.dispatch(Event::EffectiveReaderOverride(Action::Generate {
            identity,
            mode,
        }));
        assert!(
            generated
                .effective_reader_override
                .as_ref()
                .unwrap()
                .reload_required
        );
        let (requested, worker) = trace.recv_timeout(TIMEOUT).unwrap();
        assert_eq!(requested, mode);
        assert_ne!(worker, thread::current().id());
        current = h.reload(&generated);
    }
    drop(h);
    // No analyzer is available in this new bridge. Both choices must be stored.
    let h = f.bridge();
    let loaded = h.load(f.id);
    let opened = h.open_edit(&loaded);
    let prompt = opened.effective_reader_override.unwrap().prompt.unwrap();
    assert_eq!(
        prompt.choices.iter().map(|c| c.mode).collect::<Vec<_>>(),
        [SplitMode::A, SplitMode::B, SplitMode::C]
    );
    let chosen = h.dispatch(Event::EffectiveReaderOverride(Action::Choose {
        identity: prompt.identity,
        mode: SplitMode::A,
        scope: OverrideScope::ThisOccurrence,
        registered_at_unix_ms: 600,
    }));
    let resolved = h.reload(&chosen);
    let tokens = &resolved.effective_reader_tokens.as_ref().unwrap().tokens;
    assert_eq!(
        tokens
            .iter()
            .map(|t| t.surface.as_str())
            .collect::<Vec<_>>(),
        ["東", "京", "東京", "ヌヌ𠮷か\u{3099}\0"]
    );
    assert_eq!(tokens[0].reading.as_deref(), Some("A東"));
    assert_eq!(tokens[1].dictionary_form.as_deref(), Some("京"));
    assert_eq!(f.count("reading_completions"), 0);
    assert_eq!(f.count("reading_occurrences"), 0);
}

#[test]
fn serialized_missing_analysis_and_legacy_commands_fail_closed() {
    let f = Fixture::new();
    let missing = Storage::open(f.path())
        .unwrap()
        .create_paste(&PasteText::new("未解析".into()).unwrap())
        .unwrap();
    let h = f.bridge();
    let loaded = h.load(missing.document_id);
    let page = loaded.effective_reader_tokens.as_ref().unwrap();
    assert!(page.tokens.is_empty() && page.original_boundaries.is_empty());
    assert!(loaded.reader_tokens.is_none());
    let before = std::fs::read(f.path()).unwrap();
    h.send(Event::CompleteEffectiveReaderPage {
        identity: page.identity,
        position: loaded.reader.as_ref().unwrap().position,
        completed_at_unix_ms: 500,
    });
    assert_eq!(
        h.view().document_error,
        Some(DocumentError::MissingAnalysis)
    );
    assert!(h.callbacks.try_recv().is_err());
    assert_eq!(std::fs::read(f.path()).unwrap(), before);
    let loaded = h.load(f.id);
    let page = loaded.reader.as_ref().unwrap();
    let before = std::fs::read(f.path()).unwrap();
    for event in [
        Event::CompleteReaderPage {
            document_id: f.id,
            page: page.anchor,
            position: page.position,
            completed_at_unix_ms: 500,
        },
        Event::LoadReaderWord(yomoku_core::reader::ReaderWordSelection {
            key: loaded
                .effective_reader_tokens
                .as_ref()
                .unwrap()
                .identity
                .base,
            token_index: 0,
        }),
    ] {
        h.send(event);
        assert!(!h.view().storage_busy);
        assert!(h.callbacks.try_recv().is_err());
    }
    assert_eq!(std::fs::read(f.path()).unwrap(), before);
    assert_eq!(f.count("reading_completions"), 0);
}
