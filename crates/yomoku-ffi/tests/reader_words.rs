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
    effective_session::EffectiveReaderSelection,
    reader::ReaderTokenKey,
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
        let mut storage = Storage::open(self.user()).unwrap();
        let imported = storage
            .import_analyzed_paste(
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
            .map(|outcome| match outcome {
                yomoku_storage::PasteImportOutcome::Created(value) => value,
                _ => panic!("expected fresh import"),
            })
            .unwrap();
        for mode in [SplitMode::A, SplitMode::B] {
            let mut analysis = storage
                .chapter_analysis(imported.analysis_id)
                .unwrap()
                .unwrap();
            analysis.mode = mode;
            storage
                .store_chapter_analysis(imported.document.chapter.chapter_id, &analysis)
                .unwrap();
        }
        imported.document.document_id
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
) -> EffectiveReaderSelection {
    let opened = dispatch(bridge, callbacks, Event::OpenReader(id));
    let key = ReaderTokenKey {
        document_id: id,
        page: opened.reader.unwrap().anchor,
        analysis: opened.reader_analysis.unwrap(),
    };
    let page = dispatch(bridge, callbacks, Event::LoadReaderTokens(key))
        .effective_reader_tokens
        .unwrap();
    EffectiveReaderSelection {
        page: page.identity,
        segment_index: 0,
    }
}

#[test]
fn selected_word_bridge_retries_marker_and_lookup_failures_and_restores_the_saved_occurrence() {
    let fixture = Fixture::new();
    let id = fixture.seed();
    index(&fixture.index(), true);
    let (tx, callbacks) = mpsc::channel();
    let (loads_tx, loads) = mpsc::channel();
    let fail = Arc::new(AtomicBool::new(true));
    let worker_fail = fail.clone();
    let path = fixture.index();
    let (bridge, lifecycle) = YomokuBridge::new(
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
    let selected = EffectiveReaderSelection {
        segment_index: 1,
        ..selection(&bridge, &callbacks, id)
    };
    let marker = view(&bridge).effective_reader_tokens.unwrap().tokens[1].anchor;
    assert!(loads.try_recv().is_err());
    let connection = Connection::open(fixture.user()).unwrap();
    connection
        .execute_batch(
            "CREATE TRIGGER fail_marker BEFORE INSERT ON reader_word_markers
         BEGIN SELECT RAISE(ABORT, 'fixture marker failure'); END;",
        )
        .unwrap();
    let write_failed = dispatch(
        &bridge,
        &callbacks,
        Event::LoadEffectiveReaderWord(selected),
    );
    assert!(write_failed.effective_reader_word.unwrap().error.is_some());
    assert_eq!(write_failed.reader_word_marker, Some(marker));
    assert!(loads.try_recv().is_err());
    assert_eq!(
        connection
            .query_row("SELECT COUNT(*) FROM reader_word_markers", [], |row| row
                .get::<_, i64>(0))
            .unwrap(),
        0
    );
    connection
        .execute_batch("DROP TRIGGER fail_marker;")
        .unwrap();
    let failed = dispatch(
        &bridge,
        &callbacks,
        Event::LoadEffectiveReaderWord(selected),
    );
    let worker = loads.recv_timeout(TIMEOUT).unwrap();
    assert_ne!(worker, thread::current().id());
    assert_eq!(
        failed.effective_reader_word.as_ref().unwrap().error,
        Some(DocumentError::Unavailable)
    );
    assert!(failed.document_error.is_none());
    assert_eq!(
        Storage::open(fixture.user())
            .unwrap()
            .load_document_reader_snapshot(id)
            .unwrap()
            .unwrap()
            .word_marker,
        Some(marker)
    );
    fail.store(false, Ordering::SeqCst);
    let malformed = dispatch(
        &bridge,
        &callbacks,
        Event::LoadEffectiveReaderWord(selected),
    );
    assert_eq!(loads.recv_timeout(TIMEOUT).unwrap(), worker);
    assert_eq!(
        malformed.effective_reader_word.as_ref().unwrap().error,
        Some(DocumentError::Unavailable)
    );
    // Publish a repaired inode. A payload failure must retire the cached connection.
    let repaired = fixture.0.join("repaired.sqlite3");
    index(&repaired, false);
    std::fs::rename(repaired, fixture.index()).unwrap();
    let index_before = std::fs::read(fixture.index()).unwrap();
    let loaded = dispatch(
        &bridge,
        &callbacks,
        Event::LoadEffectiveReaderWord(selected),
    );
    assert_eq!(loads.recv_timeout(TIMEOUT).unwrap(), worker);
    let details = loaded
        .effective_reader_word
        .as_ref()
        .unwrap()
        .details
        .clone()
        .unwrap();
    assert_eq!(details.selection, selected);
    // Reader details include each exact-surface occurrence on the visible page.
    assert_eq!(details.encountered, 2);
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

    lifecycle.set_foreground(false).unwrap();
    let absent = dispatch(
        &bridge,
        &callbacks,
        Event::LoadEffectiveReaderWord(EffectiveReaderSelection {
            segment_index: 2,
            ..selected
        }),
    );
    let absent = absent.effective_reader_word.unwrap().details.unwrap();
    assert!(absent.candidates.words.is_empty() && absent.candidates.names.is_empty());
    assert_eq!(absent.encountered, 1);
    assert!(loads.try_recv().is_err());
    assert_eq!(std::fs::read(fixture.index()).unwrap(), index_before);
    drop(bridge);
    drop(lifecycle);

    let (tx, callbacks) = mpsc::channel();
    let (fresh, _) = YomokuBridge::new(
        fixture.user(),
        Box::new(|| panic!("reopening never loads analyzer")),
        Box::new(|| panic!("reopening never loads dictionary")),
        move |result| {
            result.unwrap();
            tx.send(thread::current().id()).unwrap();
        },
    )
    .unwrap();
    let reopened = dispatch(&fresh, &callbacks, Event::OpenReader(id));
    assert_eq!(reopened.reader, loaded.reader);
    assert_eq!(reopened.reader_word_marker, Some(absent.token.anchor));
    assert!(reopened.effective_reader_tokens.is_none());
    assert!(reopened.effective_reader_word.unwrap().selection.is_none());
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
    let (bridge, _) = YomokuBridge::new(
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
    send(&bridge, Event::LoadEffectiveReaderWord(selected));
    entered.recv_timeout(TIMEOUT).unwrap();
    let pending = view(&bridge);
    assert!(pending.storage_busy);
    let marker = pending.effective_reader_tokens.unwrap().tokens[0].anchor;
    assert_eq!(pending.reader_word_marker, Some(marker));
    send(&bridge, Event::DismissReaderWord);
    release.send(()).unwrap();
    callbacks.recv_timeout(TIMEOUT).unwrap();
    assert!(!view(&bridge).storage_busy);
    assert_eq!(view(&bridge).reader_word_marker, Some(marker));
    assert!(view(&bridge)
        .effective_reader_word
        .unwrap()
        .selection
        .is_none());
    assert!(view(&bridge)
        .effective_reader_word
        .unwrap()
        .details
        .is_none());
}
