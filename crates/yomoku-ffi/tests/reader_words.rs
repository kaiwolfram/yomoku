use crux_core::bridge::{BincodeFfiFormat, FfiFormat};
use rusqlite::Connection;
use std::{
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        mpsc, Arc,
    },
    thread,
    time::Duration,
};
use yomoku_core::{
    analysis::{AnalysisProfile, AnalyzedToken, ChapterAnalysis, SplitMode},
    app::{Event, ViewModel},
    document::{PasteText, TextAnchor, TextOffset, TextRange},
    reader::{ReaderTokenKey, ReaderWordSelection},
    storage::DocumentError,
};
use yomoku_ffi::YomokuBridge;
use yomoku_storage::Storage;

const TIMEOUT: Duration = Duration::from_secs(10);
struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let path = std::env::temp_dir().join(format!(
            "yomoku-word-ffi-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::SeqCst)
        ));
        std::fs::create_dir(&path).unwrap();
        Self(path)
    }
    fn user(&self) -> PathBuf {
        self.0.join("user.sqlite3")
    }
    fn index(&self) -> PathBuf {
        self.0.join("index.sqlite3")
    }
    fn seed(&self) -> yomoku_core::document::DocumentId {
        Storage::open(self.user())
            .unwrap()
            .create_analyzed_paste(
                &PasteText::new("東京 東京 未収録".into()).unwrap(),
                100,
                |chapter| {
                    let mut start = 0;
                    Ok::<_, std::convert::Infallible>(ChapterAnalysis {
                        profile: AnalysisProfile {
                            engine_revision: "fixture@1".into(),
                            dictionary_revision: "fixture@1".into(),
                        },
                        mode: SplitMode::C,
                        tokens: chapter
                            .text
                            .split_whitespace()
                            .map(|surface| {
                                let end = start + surface.len();
                                let token = AnalyzedToken {
                                    anchor: TextAnchor {
                                        source_id: chapter.source_id,
                                        chapter_id: chapter.chapter_id,
                                        range: TextRange {
                                            start: TextOffset::at(&chapter.text, start).unwrap(),
                                            end: TextOffset::at(&chapter.text, end).unwrap(),
                                        },
                                    },
                                    surface: surface.into(),
                                    is_japanese: true,
                                    reading: None,
                                    dictionary_form: None,
                                };
                                start = end + 1;
                                token
                            })
                            .collect(),
                    })
                },
            )
            .unwrap()
            .document
            .document_id
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
fn index(path: &Path, malformed: bool) {
    let db = Connection::open(path).unwrap();
    db.execute_batch("PRAGMA application_id=0x594f4449; PRAGMA user_version=1;
        CREATE TABLE entries(source TEXT,entry_id INTEGER,payload TEXT,PRIMARY KEY(source,entry_id)) WITHOUT ROWID;
        CREATE TABLE forms(surface TEXT COLLATE BINARY,source TEXT,entry_id INTEGER,PRIMARY KEY(surface,source,entry_id)) WITHOUT ROWID;").unwrap();
    for (source, payload) in [
        (
            "JMdict",
            r#"{"kanji":[{"text":"東京"}],"readings":[{"text":"とうきょう"},{"text":"とうけい"}],"senses":[{"pos":["noun (common) (futsuumeishi)"],"glosses":[{"text":"Tokyo","lang":"eng"}]},{"stagr":["とうけい"],"s_inf":["historic"],"glosses":[{"text":"eastern capital","lang":"eng"}]}]}"#,
        ),
        (
            "JMnedict",
            r#"{"kanji":[{"text":"東京"}],"readings":[{"text":"とうきょう"}],"names":[{"name_type":["place name"],"translations":[{"text":"Tokyo","lang":"eng"}]}]}"#,
        ),
    ] {
        db.execute(
            "INSERT INTO entries VALUES (?1,1,?2)",
            (source, if malformed { "{}" } else { payload }),
        )
        .unwrap();
        db.execute("INSERT INTO forms VALUES ('東京',?1,1)", [source])
            .unwrap();
    }
}
fn send(bridge: &YomokuBridge, event: Event) {
    let mut bytes = Vec::new();
    BincodeFfiFormat::serialize(&mut bytes, &event).unwrap();
    bridge.update(&bytes).unwrap();
}
fn view(bridge: &YomokuBridge) -> ViewModel {
    BincodeFfiFormat::deserialize(&bridge.view().unwrap()).unwrap()
}
fn dispatch(
    bridge: &YomokuBridge,
    callbacks: &mpsc::Receiver<thread::ThreadId>,
    event: Event,
) -> ViewModel {
    send(bridge, event);
    assert_ne!(
        callbacks.recv_timeout(TIMEOUT).unwrap(),
        thread::current().id()
    );
    let view = view(bridge);
    assert!(!view.storage_busy);
    view
}
fn selection(
    bridge: &YomokuBridge,
    callbacks: &mpsc::Receiver<thread::ThreadId>,
    id: yomoku_core::document::DocumentId,
) -> ReaderWordSelection {
    let opened = dispatch(bridge, callbacks, Event::OpenReader(id));
    let key = ReaderTokenKey {
        document_id: id,
        page: opened.reader.unwrap().anchor,
        analysis: opened.reader_analysis.unwrap(),
    };
    dispatch(bridge, callbacks, Event::LoadReaderTokens(key));
    ReaderWordSelection {
        key,
        token_index: 0,
    }
}

#[test]
fn selected_word_bridge_loads_lazily_retries_both_failure_kinds_reuses_fifo_and_never_writes() {
    let fixture = Fixture::new();
    let id = fixture.seed();
    index(&fixture.index(), true);
    let (tx, callbacks) = mpsc::channel();
    let (loads_tx, loads) = mpsc::channel();
    let fail = Arc::new(AtomicBool::new(true));
    let worker_fail = fail.clone();
    let path = fixture.index();
    let (bridge, lifecycle) = YomokuBridge::with_dictionaries(
        fixture.user(),
        Box::new(|| panic!("lookup never loads analyzer")),
        Box::new(move || {
            loads_tx.send(thread::current().id()).unwrap();
            if worker_fail.load(Ordering::SeqCst) {
                Err(DocumentError::Unavailable)
            } else {
                Ok(path.clone())
            }
        }),
        move |result| {
            result.unwrap();
            tx.send(thread::current().id()).unwrap();
        },
    )
    .unwrap();
    dispatch(&bridge, &callbacks, Event::RefreshLibrary);
    let selected = selection(&bridge, &callbacks, id);
    assert!(loads.try_recv().is_err());
    let user_before = std::fs::read(fixture.user()).unwrap();
    let failed = dispatch(&bridge, &callbacks, Event::LoadReaderWord(selected));
    let worker = loads.recv_timeout(TIMEOUT).unwrap();
    assert_ne!(worker, thread::current().id());
    assert_eq!(failed.reader_word_error, Some(DocumentError::Unavailable));
    assert!(failed.document_error.is_none());
    fail.store(false, Ordering::SeqCst);
    let malformed = dispatch(&bridge, &callbacks, Event::LoadReaderWord(selected));
    assert_eq!(loads.recv_timeout(TIMEOUT).unwrap(), worker);
    assert_eq!(
        malformed.reader_word_error,
        Some(DocumentError::Unavailable)
    );
    // Publish a repaired inode. A payload failure must retire the cached connection.
    let repaired = fixture.0.join("repaired.sqlite3");
    index(&repaired, false);
    std::fs::rename(repaired, fixture.index()).unwrap();
    let index_before = std::fs::read(fixture.index()).unwrap();
    let loaded = dispatch(&bridge, &callbacks, Event::LoadReaderWord(selected));
    assert_eq!(loads.recv_timeout(TIMEOUT).unwrap(), worker);
    let details = loaded.reader_word.unwrap();
    assert_eq!(details.selection(), selected);
    assert_eq!(details.encountered, 0);
    let words = details.candidates.words;
    let names = details.candidates.names;
    assert_eq!((words.len(), names.len()), (1, 1));
    assert_eq!(words[0].entry_id, names[0].entry_id);
    assert_eq!(words[0].entry.readings.len(), 2);
    assert_eq!(words[0].entry.senses.len(), 2);
    assert_eq!(words[0].forms[0].matches[0].sense_indices, vec![0]);
    assert_eq!(words[0].forms[0].matches[1].sense_indices, vec![0, 1]);
    assert_eq!(names[0].entry.names[0].name_type, vec!["place name"]);
    assert_eq!(loaded.reader, failed.reader);
    assert_eq!(loaded.reader_time, failed.reader_time);
    lifecycle.set_foreground(false).unwrap();
    let absent = dispatch(
        &bridge,
        &callbacks,
        Event::LoadReaderWord(ReaderWordSelection {
            token_index: 2,
            ..selected
        }),
    );
    let absent = absent.reader_word.unwrap();
    assert!(absent.candidates.words.is_empty() && absent.candidates.names.is_empty());
    assert_eq!(absent.encountered, 0);
    assert!(loads.try_recv().is_err());
    assert_eq!(std::fs::read(fixture.user()).unwrap(), user_before);
    assert_eq!(std::fs::read(fixture.index()).unwrap(), index_before);
}

#[test]
fn selected_word_bridge_discards_a_dismissed_selection_while_provider_is_blocked() {
    let fixture = Fixture::new();
    let id = fixture.seed();
    index(&fixture.index(), false);
    let (tx, callbacks) = mpsc::channel();
    let (entered_tx, entered) = mpsc::channel();
    let (release, wait) = mpsc::channel();
    let path = fixture.index();
    let (bridge, _) = YomokuBridge::with_dictionaries(
        fixture.user(),
        Box::new(|| panic!("no analyzer")),
        Box::new(move || {
            entered_tx.send(()).unwrap();
            wait.recv_timeout(TIMEOUT).unwrap();
            Ok(path.clone())
        }),
        move |result| {
            result.unwrap();
            tx.send(thread::current().id()).unwrap();
        },
    )
    .unwrap();
    let selected = selection(&bridge, &callbacks, id);
    send(&bridge, Event::LoadReaderWord(selected));
    entered.recv_timeout(TIMEOUT).unwrap();
    assert!(view(&bridge).storage_busy);
    send(&bridge, Event::DismissReaderWord);
    release.send(()).unwrap();
    callbacks.recv_timeout(TIMEOUT).unwrap();
    assert!(!view(&bridge).storage_busy);
    assert!(view(&bridge).reader_word_selection.is_none());
    assert!(view(&bridge).reader_word.is_none());
}

fn custom_bridge(fixture: &Fixture) -> (YomokuBridge, mpsc::Receiver<thread::ThreadId>) {
    let (tx, callbacks) = mpsc::channel();
    let path = fixture.index();
    let (bridge, _) = YomokuBridge::with_dictionaries(
        fixture.user(),
        Box::new(|| panic!("custom entries never load analyzer")),
        Box::new(move || Ok(path.clone())),
        move |result| {
            result.unwrap();
            tx.send(thread::current().id()).unwrap();
        },
    )
    .unwrap();
    (bridge, callbacks)
}
fn missing_selection(
    bridge: &YomokuBridge,
    callbacks: &mpsc::Receiver<thread::ThreadId>,
    id: yomoku_core::document::DocumentId,
) -> ReaderWordSelection {
    let selected = ReaderWordSelection {
        token_index: 2,
        ..selection(bridge, callbacks, id)
    };
    let loaded = dispatch(bridge, callbacks, Event::LoadReaderWord(selected));
    assert!(loaded.reader_word_error.is_none());
    assert!(loaded
        .reader_word
        .as_ref()
        .unwrap()
        .candidates
        .words
        .is_empty());
    selected
}
fn save_custom(selected: ReaderWordSelection, meaning: &str) -> Event {
    Event::SaveReaderCustomEntry {
        selection: selected,
        english_meaning: meaning.into(),
        reading: " か\u{3099}𠮷\0 ".into(),
    }
}

#[test]
fn custom_entry_bridge_crud_reopens_exact_unicode_and_keeps_builtins_and_reader_state_immutable() {
    let fixture = Fixture::new();
    let id = fixture.seed();
    index(&fixture.index(), false);
    let index_before = std::fs::read(fixture.index()).unwrap();
    {
        let (bridge, callbacks) = custom_bridge(&fixture);
        let selected = missing_selection(&bridge, &callbacks, id);
        let before = view(&bridge);
        assert!(before.reader_word.unwrap().custom_entry.is_none());
        let created = dispatch(
            &bridge,
            &callbacks,
            save_custom(selected, " literal meaning 📖\0 "),
        );
        let entry = created
            .reader_word
            .as_ref()
            .unwrap()
            .custom_entry
            .as_ref()
            .unwrap();
        assert_eq!(entry.surface, "未収録");
        assert_eq!(entry.english_meaning, " literal meaning 📖\0 ");
        assert_eq!(entry.reading, " か\u{3099}𠮷\0 ");
        assert_eq!(created.reader, before.reader);
        assert_eq!(created.reader_tokens, before.reader_tokens);
        assert_eq!(created.documents, before.documents);
        assert_eq!(created.reader_time, before.reader_time);
        let user_before = std::fs::read(fixture.user()).unwrap();
        dispatch(
            &bridge,
            &callbacks,
            save_custom(selected, " literal meaning 📖\0 "),
        );
        assert_eq!(std::fs::read(fixture.user()).unwrap(), user_before);
        let edited = dispatch(&bridge, &callbacks, save_custom(selected, "edited"));
        assert_eq!(
            edited
                .reader_word
                .unwrap()
                .custom_entry
                .unwrap()
                .english_meaning,
            "edited"
        );
        let builtin = ReaderWordSelection {
            token_index: 0,
            ..selected
        };
        let builtin_view = dispatch(&bridge, &callbacks, Event::LoadReaderWord(builtin));
        assert!(builtin_view
            .reader_word
            .as_ref()
            .unwrap()
            .custom_entry
            .is_none());
        let user_before = std::fs::read(fixture.user()).unwrap();
        for event in [
            save_custom(builtin, "override"),
            Event::DeleteReaderCustomEntry(builtin),
        ] {
            send(&bridge, event);
            assert!(!view(&bridge).storage_busy);
            assert!(callbacks.try_recv().is_err());
        }
        assert_eq!(std::fs::read(fixture.user()).unwrap(), user_before);
    }
    let (bridge, callbacks) = custom_bridge(&fixture);
    let selected = missing_selection(&bridge, &callbacks, id);
    assert_eq!(
        view(&bridge)
            .reader_word
            .unwrap()
            .custom_entry
            .unwrap()
            .english_meaning,
        "edited"
    );
    let deleted = dispatch(
        &bridge,
        &callbacks,
        Event::DeleteReaderCustomEntry(selected),
    );
    assert!(deleted.reader_word.unwrap().custom_entry.is_none());
    let user_before = std::fs::read(fixture.user()).unwrap();
    dispatch(
        &bridge,
        &callbacks,
        Event::DeleteReaderCustomEntry(selected),
    );
    let reread = dispatch(&bridge, &callbacks, Event::LoadReaderWord(selected));
    assert!(reread.reader_word.unwrap().custom_entry.is_none());
    assert_eq!(std::fs::read(fixture.user()).unwrap(), user_before);
    assert_eq!(std::fs::read(fixture.index()).unwrap(), index_before);
}

#[test]
fn custom_entry_bridge_save_delete_failures_keep_prior_data_and_retry_only_explicitly() {
    let fixture = Fixture::new();
    let id = fixture.seed();
    index(&fixture.index(), false);
    let (bridge, callbacks) = custom_bridge(&fixture);
    let selected = missing_selection(&bridge, &callbacks, id);
    dispatch(&bridge, &callbacks, save_custom(selected, "original"));
    let db = Connection::open(fixture.user()).unwrap();
    for delete in [false, true] {
        let (action, event) = if delete {
            ("DELETE", Event::DeleteReaderCustomEntry(selected))
        } else {
            ("UPDATE", save_custom(selected, "replacement"))
        };
        db.execute_batch(&format!("CREATE TRIGGER fail_custom BEFORE {action} ON custom_dictionary_entries BEGIN SELECT RAISE(ABORT,'fixture'); END;")).unwrap();
        let before = view(&bridge);
        let failed = dispatch(&bridge, &callbacks, event.clone());
        assert_eq!(
            failed.reader_custom_entry_error,
            Some(DocumentError::Unavailable)
        );
        assert!(failed.reader_word_error.is_none() && failed.document_error.is_none());
        assert_eq!(failed.reader_word, before.reader_word);
        assert_eq!(failed.reader_tokens, before.reader_tokens);
        assert_eq!(failed.reader, before.reader);
        let loaded = dispatch(
            &bridge,
            &callbacks,
            Event::SetReaderStatus(yomoku_core::reader::ReaderStatusChange {
                key: selected.key,
                vocabulary_id: failed
                    .reader_word
                    .as_ref()
                    .unwrap()
                    .token
                    .vocabulary
                    .unwrap()
                    .id,
                status: yomoku_core::vocabulary::VocabularyStatus::New,
                occurred_at_unix_ms: 1234,
            }),
        );
        assert_eq!(
            loaded.reader_custom_entry_error,
            Some(DocumentError::Unavailable)
        );
        db.execute_batch("DROP TRIGGER fail_custom;").unwrap();
        assert!(callbacks.try_recv().is_err());
        let retried = dispatch(&bridge, &callbacks, event);
        assert!(retried.reader_custom_entry_error.is_none());
        if delete {
            assert!(retried.reader_word.unwrap().custom_entry.is_none());
        } else {
            assert_eq!(
                retried
                    .reader_word
                    .unwrap()
                    .custom_entry
                    .unwrap()
                    .english_meaning,
                "replacement"
            );
        }
    }
}

#[test]
fn custom_entry_bridge_rechecks_dictionary_and_corruption_never_becomes_a_missing_match() {
    let fixture = Fixture::new();
    let id = fixture.seed();
    index(&fixture.index(), false);
    let (bridge, callbacks) = custom_bridge(&fixture);
    let selected = missing_selection(&bridge, &callbacks, id);
    dispatch(&bridge, &callbacks, save_custom(selected, "original"));
    let index_db = Connection::open(fixture.index()).unwrap();
    index_db.execute("INSERT OR IGNORE INTO entries VALUES ('JMdict',2,?1)", [r#"{"kanji":[{"text":"未収録"}],"readings":[{"text":"みしゅうろく"}],"senses":[{"glosses":[{"text":"new match","lang":"eng"}]}]}"#]).unwrap();
    index_db
        .execute("INSERT INTO forms VALUES ('未収録','JMdict',2)", [])
        .unwrap();
    let failed = dispatch(&bridge, &callbacks, save_custom(selected, "override"));
    assert_eq!(
        failed.reader_custom_entry_error,
        Some(DocumentError::InvalidDocument)
    );
    let loaded = dispatch(&bridge, &callbacks, Event::LoadReaderWord(selected));
    assert!(!loaded
        .reader_word
        .as_ref()
        .unwrap()
        .candidates
        .words
        .is_empty());
    assert!(loaded.reader_word.unwrap().custom_entry.is_none());
    index_db
        .execute("DELETE FROM forms WHERE surface='未収録'", [])
        .unwrap();
    dispatch(&bridge, &callbacks, Event::LoadReaderWord(selected));
    index_db
        .execute("UPDATE entries SET payload='{}' WHERE source='JMdict'", [])
        .unwrap();
    index_db.execute("INSERT OR IGNORE INTO entries VALUES ('JMdict',2,?1)", [r#"{"kanji":[{"text":"未収録"}],"readings":[{"text":"みしゅうろく"}],"senses":[{"glosses":[{"text":"new match","lang":"eng"}]}]}"#]).unwrap();
    index_db
        .execute("INSERT INTO forms VALUES ('未収録','JMdict',2)", [])
        .unwrap();
    let failed = dispatch(
        &bridge,
        &callbacks,
        Event::DeleteReaderCustomEntry(selected),
    );
    assert_eq!(
        failed.reader_custom_entry_error,
        Some(DocumentError::Unavailable)
    );
    // Repaired publication uses a new inode; failure must retire the cached connection.
    let repaired = fixture.0.join("repaired.sqlite3");
    index(&repaired, false);
    std::fs::rename(repaired, fixture.index()).unwrap();
    let loaded = dispatch(&bridge, &callbacks, Event::LoadReaderWord(selected));
    assert_eq!(
        loaded
            .reader_word
            .unwrap()
            .custom_entry
            .unwrap()
            .english_meaning,
        "original"
    );
    let user_db = Connection::open(fixture.user()).unwrap();
    user_db
        .execute(
            "UPDATE custom_dictionary_entries SET reading=' ' WHERE surface='未収録'",
            [],
        )
        .unwrap();
    let failed = dispatch(&bridge, &callbacks, Event::LoadReaderWord(selected));
    assert_eq!(
        failed.reader_word_error,
        Some(DocumentError::InvalidDocument)
    );
    assert!(failed.reader_word.is_none());
}
