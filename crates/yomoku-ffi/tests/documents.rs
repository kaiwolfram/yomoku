use std::{
    path::PathBuf,
    sync::{
        atomic::{AtomicU64, Ordering},
        mpsc, Arc, Weak,
    },
    thread::{self, ThreadId},
    time::Duration,
};

use crux_core::bridge::{BincodeFfiFormat, FfiFormat, Requests};
use yomoku_core::{
    app::{Destination, Event, ViewModel},
    document::DocumentId,
    reader::ReaderAnalysis,
    storage::{DocumentError, StorageResult},
};
use yomoku_ffi::{FfiError, YomokuBridge as NativeBridge};
use yomoku_runtime::ShellEffectFfi;

type Callbacks = mpsc::Receiver<(ThreadId, Vec<u8>)>;

fn bridge(path: &str) -> (NativeBridge, Callbacks, Weak<()>) {
    let (sender, receiver) = mpsc::channel();
    let life = Arc::new(());
    let weak = Arc::downgrade(&life);
    (
        NativeBridge::new(path.into(), move |result| {
            let _keep_alive = &life;
            let _ = sender.send((thread::current().id(), result.unwrap()));
        })
        .unwrap(),
        receiver,
        weak,
    )
}

fn view(bridge: &NativeBridge) -> ViewModel {
    BincodeFfiFormat::deserialize(&bridge.view().unwrap()).unwrap()
}

fn dispatch(bridge: &NativeBridge, receiver: &Callbacks, event: Event) -> ViewModel {
    let mut bytes = vec![];
    BincodeFfiFormat::serialize(&mut bytes, &event).unwrap();
    let immediate: Requests<ShellEffectFfi> =
        BincodeFfiFormat::deserialize(&bridge.update(&bytes).unwrap()).unwrap();
    assert_eq!(immediate.0.len(), 1);
    let (worker, bytes) = receiver.recv_timeout(Duration::from_secs(10)).unwrap();
    assert_ne!(worker, thread::current().id());
    let requests: Requests<ShellEffectFfi> = BincodeFfiFormat::deserialize(&bytes).unwrap();
    assert_eq!(requests.0.len(), 1);
    assert!(matches!(requests.0[0].effect, ShellEffectFfi::Render(_)));
    let view = view(bridge);
    assert!(!view.storage_busy);
    view
}

struct Database(PathBuf);
impl Database {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        Self(std::env::temp_dir().join(format!(
            "yomoku-ffi-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        )))
    }
    fn path(&self) -> String {
        self.0.join("db.sqlite3").to_str().unwrap().to_owned()
    }
}
impl Drop for Database {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[test]
fn async_native_pastes_list_load_reopen_and_release_listeners() {
    let db = Database::new();
    std::fs::create_dir(&db.0).unwrap();
    let (native, callbacks, life) = bridge(&db.path());
    assert!(!std::path::Path::new(&db.path()).exists()); // constructor performs no I/O
    assert!(dispatch(&native, &callbacks, Event::RefreshLibrary)
        .documents
        .is_empty());
    let text = "  題𠮷📖  \r\nか\u{3099}👩‍👩‍👧‍👦\0終\n";
    let created = dispatch(&native, &callbacks, Event::CreatePaste(text.into()));
    assert_eq!(created.documents.len(), 1);
    assert_eq!(created.documents[0].title, "  題𠮷📖  ");
    assert_eq!(created.destination, Destination::Library);
    assert!(created.loaded_document.is_none()); // storage does not start a reader
    let id = created.documents[0].document_id;
    let expected = dispatch(&native, &callbacks, Event::LoadPaste(id))
        .loaded_document
        .unwrap();
    assert_eq!(expected.chapter.text, text);
    assert_eq!(
        dispatch(&native, &callbacks, Event::OpenReader(id)).reader_analysis,
        Some(ReaderAnalysis::Missing)
    );
    assert_eq!(
        dispatch(&native, &callbacks, Event::CreatePaste(" \n\t".into())).document_error,
        Some(DocumentError::EmptyPaste)
    );
    assert_eq!(
        dispatch(&native, &callbacks, Event::RefreshLibrary)
            .documents
            .len(),
        1
    );
    assert_eq!(
        dispatch(
            &native,
            &callbacks,
            Event::LoadPaste(DocumentId::try_from(i64::MAX).unwrap())
        )
        .document_error,
        Some(DocumentError::NotFound)
    );
    drop(native);
    // Wait for the worker to drop its last callback capture before asserting lifetime.
    assert!(callbacks.recv_timeout(Duration::from_secs(10)).is_err());
    assert!(life.upgrade().is_none());
    let (reopened, callbacks, _) = bridge(&db.path());
    assert_eq!(
        dispatch(&reopened, &callbacks, Event::RefreshLibrary).documents,
        created.documents
    );
    assert_eq!(
        dispatch(&reopened, &callbacks, Event::LoadPaste(id)).loaded_document,
        Some(expected)
    );
    let (independent, callbacks, _) = bridge(":memory:");
    assert!(dispatch(&independent, &callbacks, Event::RefreshLibrary)
        .documents
        .is_empty());
}

#[test]
fn async_open_failure_is_typed_retryable_and_preserves_invalid_files() {
    let db = Database::new();
    let (native, callbacks, _) = bridge(&db.path());
    assert_eq!(
        dispatch(&native, &callbacks, Event::RefreshLibrary).document_error,
        Some(DocumentError::Unavailable)
    );
    assert_eq!(
        dispatch(&native, &callbacks, Event::CreatePaste(" \n".into())).document_error,
        Some(DocumentError::EmptyPaste)
    );
    assert!(!db.0.exists());
    std::fs::create_dir(&db.0).unwrap();
    assert!(dispatch(&native, &callbacks, Event::RefreshLibrary)
        .document_error
        .is_none());
    drop(native);
    assert!(callbacks.recv_timeout(Duration::from_secs(10)).is_err());
    std::fs::write(db.path(), b"not a database").unwrap();
    let (native, callbacks, _) = bridge(&db.path());
    assert_eq!(
        dispatch(&native, &callbacks, Event::RefreshLibrary).document_error,
        Some(DocumentError::Unavailable)
    );
    assert_eq!(std::fs::read(db.path()).unwrap(), b"not a database");
}

#[test]
fn internal_storage_results_cannot_be_injected_over_ffi() {
    let (native, _, _) = bridge(":memory:");
    // StorageFinished is skipped by Serde and by Facet generation.
    let forged = Event::StorageFinished(StorageResult::Failed(DocumentError::Unavailable));
    assert!(BincodeFfiFormat::serialize(&mut vec![], &forged).is_err());
    // The skipped internal variant follows the final public variant. Derive its
    // discriminant so adding a public event cannot silently weaken this check.
    let mut last_public = Vec::new();
    BincodeFfiFormat::serialize(
        &mut last_public,
        &Event::DismissBookCompletionNotice(
            yomoku_core::document::CompletionCommandId::try_from(1).unwrap(),
        ),
    )
    .unwrap();
    let internal_tag = u32::from_le_bytes(last_public[..4].try_into().unwrap()) + 1;
    assert_eq!(
        native.update(&internal_tag.to_le_bytes()),
        Err(FfiError::InvalidEvent)
    );
    assert!(view(&native).documents.is_empty());
}

#[test]
fn blocked_sqlite_write_does_not_block_dispatch_view_or_navigation() {
    let db = Database::new();
    std::fs::create_dir(&db.0).unwrap();
    let (native, callbacks, _) = bridge(&db.path());
    dispatch(&native, &callbacks, Event::RefreshLibrary);
    let blocker = rusqlite::Connection::open(db.path()).unwrap();
    blocker.execute_batch("BEGIN IMMEDIATE").unwrap();
    let mut bytes = vec![];
    BincodeFfiFormat::serialize(&mut bytes, &Event::CreatePaste("blocked paste".into())).unwrap();
    let started = std::time::Instant::now();
    native.update(&bytes).unwrap();
    assert!(started.elapsed() < Duration::from_secs(1));
    assert!(callbacks.recv_timeout(Duration::from_millis(100)).is_err());
    assert!(view(&native).storage_busy);
    bytes.clear();
    BincodeFfiFormat::serialize(
        &mut bytes,
        &Event::SelectDestination(Destination::Vocabulary),
    )
    .unwrap();
    native.update(&bytes).unwrap();
    assert_eq!(view(&native).destination, Destination::Vocabulary);
    blocker.execute_batch("COMMIT").unwrap();
    callbacks.recv_timeout(Duration::from_secs(10)).unwrap();
    assert_eq!(view(&native).documents.len(), 1);
    assert!(!view(&native).storage_busy);
}

#[test]
fn monotonic_clock_final_save_survives_unrelated_busy_work_close_and_fresh_epoch_reopen() {
    use yomoku_core::{
        active_time::{ReaderClockAction, ReaderClockSample},
        app::ContentRoute,
        pagination::PageLength,
    };
    let db = Database::new();
    std::fs::create_dir(&db.0).unwrap();
    let (native, callbacks, _) = bridge(&db.path());
    let created = dispatch(&native, &callbacks, Event::CreatePaste("𠮷猫。".into()));
    let id = created.documents[0].document_id;
    dispatch(&native, &callbacks, Event::OpenReader(id));
    let saved = dispatch(&native, &callbacks, Event::RestoreReadingTime)
        .reader_time
        .unwrap();
    let sample = ReaderClockSample {
        document_id: id,
        position: saved.position,
        monotonic_ms: 500,
        action: ReaderClockAction::Resume,
    };
    let send = |event: Event| {
        let mut bytes = Vec::new();
        BincodeFfiFormat::serialize(&mut bytes, &event).unwrap();
        native.update(&bytes).unwrap();
    };
    send(Event::ObserveReaderClock(sample));
    send(Event::OpenSettings);
    let sql = rusqlite::Connection::open(db.path()).unwrap();
    sql.execute_batch("BEGIN IMMEDIATE").unwrap();
    send(Event::SetPageLength(PageLength::Characters250));
    send(Event::ObserveReaderClock(ReaderClockSample {
        monotonic_ms: 1_500,
        action: ReaderClockAction::Pause,
        ..sample
    }));
    send(Event::CloseDocument);
    assert!(view(&native).storage_busy && view(&native).reading_time_save_pending);
    assert_eq!(view(&native).content_route, ContentRoute::Reader);
    assert!(callbacks.recv_timeout(Duration::from_millis(100)).is_err());
    sql.execute_batch("COMMIT").unwrap();
    for _ in 0..2 {
        callbacks.recv_timeout(Duration::from_secs(10)).unwrap();
    }
    assert!(!view(&native).storage_busy && !view(&native).reading_time_save_pending);
    assert_eq!(view(&native).content_route, ContentRoute::Main);
    drop(native);
    let (native, callbacks, _) = bridge(&db.path());
    let opened = dispatch(&native, &callbacks, Event::OpenReader(id));
    assert_eq!(opened.page_length, Some(PageLength::Characters250));
    assert!(opened.reader_time.is_none());
    let saved = dispatch(&native, &callbacks, Event::RestoreReadingTime)
        .reader_time
        .unwrap();
    assert_eq!(saved.pending.milliseconds(), 1_000);
    let fresh = ReaderClockSample {
        monotonic_ms: 0,
        ..sample
    };
    let mut bytes = Vec::new();
    BincodeFfiFormat::serialize(&mut bytes, &Event::ObserveReaderClock(fresh)).unwrap();
    native.update(&bytes).unwrap();
    sql.execute_batch("CREATE TRIGGER fail_clock_save BEFORE UPDATE ON pending_reading_time BEGIN SELECT RAISE(ABORT, 'injected'); END").unwrap();
    let failed = dispatch(
        &native,
        &callbacks,
        Event::ObserveReaderClock(ReaderClockSample {
            monotonic_ms: 100,
            action: ReaderClockAction::Pause,
            ..fresh
        }),
    );
    assert!(failed.reading_time_save_pending);
    assert_eq!(failed.document_error, Some(DocumentError::Unavailable));
    assert_eq!(failed.reader_time, Some(saved));
    sql.execute_batch("DROP TRIGGER fail_clock_save").unwrap();
    assert_eq!(
        dispatch(&native, &callbacks, Event::RetryReadingTimeCheckpoint)
            .reader_time
            .unwrap()
            .pending
            .milliseconds(),
        1_100
    );
    assert_eq!(
        sql.query_row("SELECT milliseconds FROM pending_reading_time", [], |row| {
            row.get::<_, i64>(0)
        })
        .unwrap(),
        1_100
    );
    for table in [
        "reading_completions",
        "reading_occurrences",
        "vocabulary_status_events",
    ] {
        assert_eq!(
            sql.query_row(&format!("SELECT count(*) FROM {table}"), [], |row| row
                .get::<_, i64>(0))
                .unwrap(),
            0
        );
    }
}

#[test]
fn async_time_checkpoints_retry_exact_payload_and_restore_each_offset_after_reopen() {
    use yomoku_core::{
        active_time::PendingReadingTime,
        pagination::{PageLength, ReadingPosition},
    };
    let db = Database::new();
    std::fs::create_dir(&db.0).unwrap();
    let (native, callbacks, _) = bridge(&db.path());
    let text = format!("{}\n次の段落。", "𠮷".repeat(500));
    let created = dispatch(&native, &callbacks, Event::CreatePaste(text));
    let id = created.documents[0].document_id;
    let opened = dispatch(&native, &callbacks, Event::OpenReader(id));
    assert!(opened.reader_time.is_none());
    let sql = rusqlite::Connection::open(db.path()).unwrap();
    assert_eq!(
        sql.query_row("SELECT count(*) FROM pending_reading_time", [], |row| row
            .get::<_, i64>(
            0
        ))
        .unwrap(),
        0
    );
    sql.execute_batch("CREATE TRIGGER fail_time_restore BEFORE INSERT ON pending_reading_time BEGIN SELECT RAISE(ABORT, 'injected'); END").unwrap();
    let failed = dispatch(&native, &callbacks, Event::RestoreReadingTime);
    assert_eq!(failed.document_error, Some(DocumentError::Unavailable));
    assert!(failed.reader_time.is_none());
    sql.execute_batch("DROP TRIGGER fail_time_restore").unwrap();
    let restored = dispatch(&native, &callbacks, Event::RestoreReadingTime);
    let mut checkpoint = restored.reader_time.unwrap();
    assert_eq!(checkpoint.pending.milliseconds(), 0);
    checkpoint.pending = PendingReadingTime::from_millis(500).unwrap();
    sql.execute_batch("CREATE TRIGGER fail_time_save BEFORE UPDATE ON pending_reading_time BEGIN SELECT RAISE(ABORT, 'injected'); END").unwrap();
    let failed = dispatch(
        &native,
        &callbacks,
        Event::CheckpointReadingTime(checkpoint),
    );
    assert_eq!(failed.document_error, Some(DocumentError::Unavailable));
    assert!(failed.reading_time_save_pending);
    assert_eq!(failed.reader_time.unwrap().pending.milliseconds(), 0);
    sql.execute_batch("DROP TRIGGER fail_time_save").unwrap();
    let retried = dispatch(&native, &callbacks, Event::RetryReadingTimeCheckpoint);
    assert_eq!(retried.reader_time, Some(checkpoint));
    assert!(!retried.reading_time_save_pending);
    // Repeated absolute checkpoint delivery cannot add time twice.
    let repeated = dispatch(
        &native,
        &callbacks,
        Event::CheckpointReadingTime(checkpoint),
    );
    assert_eq!(repeated.reader_time, Some(checkpoint));
    let first = opened.reader.unwrap();
    let second_position = ReadingPosition {
        offset: first.anchor.range.end,
        ..first.position
    };
    let moved = dispatch(
        &native,
        &callbacks,
        Event::SaveReadingPosition(second_position),
    );
    assert!(moved.reader_time.is_none());
    let mut second = dispatch(&native, &callbacks, Event::RestoreReadingTime)
        .reader_time
        .unwrap();
    assert_eq!(second.pending.milliseconds(), 0);
    second.pending = PendingReadingTime::from_millis(700).unwrap();
    dispatch(&native, &callbacks, Event::CheckpointReadingTime(second));
    dispatch(
        &native,
        &callbacks,
        Event::SaveReadingPosition(first.position),
    );
    assert_eq!(
        dispatch(&native, &callbacks, Event::RestoreReadingTime).reader_time,
        Some(checkpoint)
    );
    // Reflow changes page membership without changing either anchor's duration.
    let mut bytes = Vec::new();
    BincodeFfiFormat::serialize(&mut bytes, &Event::OpenSettings).unwrap();
    native.update(&bytes).unwrap();
    assert_eq!(
        dispatch(
            &native,
            &callbacks,
            Event::SetPageLength(PageLength::Characters1500)
        )
        .reader_time,
        Some(checkpoint)
    );
    drop(native);
    assert!(callbacks.recv_timeout(Duration::from_secs(10)).is_err());
    let (reopened, callbacks, _) = bridge(&db.path());
    let opened = dispatch(&reopened, &callbacks, Event::OpenReader(id));
    assert!(opened.reader_time.is_none());
    assert_eq!(
        dispatch(&reopened, &callbacks, Event::RestoreReadingTime).reader_time,
        Some(checkpoint)
    );
    assert_eq!(
        sql.query_row(
            "SELECT sum(milliseconds) FROM pending_reading_time",
            [],
            |row| row.get::<_, i64>(0)
        )
        .unwrap(),
        1200
    );
    for table in [
        "reading_completions",
        "reading_occurrences",
        "vocabulary_status_events",
    ] {
        assert_eq!(
            sql.query_row(&format!("SELECT count(*) FROM {table}"), [], |row| row
                .get::<_, i64>(0))
                .unwrap(),
            0
        );
    }
}

#[test]
fn busy_time_updates_flush_before_position_and_close_through_async_ffi() {
    use yomoku_core::{
        active_time::PendingReadingTime, app::ContentRoute, pagination::ReadingPosition,
    };
    let db = Database::new();
    std::fs::create_dir(&db.0).unwrap();
    let (native, callbacks, _) = bridge(&db.path());
    let created = dispatch(
        &native,
        &callbacks,
        Event::CreatePaste(format!("{}\n次。", "𠮷".repeat(500))),
    );
    let id = created.documents[0].document_id;
    let opened = dispatch(&native, &callbacks, Event::OpenReader(id));
    let first = opened.reader.unwrap();
    let destination = ReadingPosition {
        offset: first.anchor.range.end,
        ..first.position
    };
    let mut checkpoint = dispatch(&native, &callbacks, Event::RestoreReadingTime)
        .reader_time
        .unwrap();
    let sql = rusqlite::Connection::open(db.path()).unwrap();
    let send = |event: Event| {
        let mut bytes = Vec::new();
        BincodeFfiFormat::serialize(&mut bytes, &event).unwrap();
        native.update(&bytes).unwrap();
    };
    let await_callbacks = |count| {
        for _ in 0..count {
            let (worker, bytes) = callbacks.recv_timeout(Duration::from_secs(10)).unwrap();
            assert_ne!(worker, thread::current().id());
            let requests: Requests<ShellEffectFfi> = BincodeFfiFormat::deserialize(&bytes).unwrap();
            assert_eq!(requests.0.len(), 1);
            assert!(matches!(requests.0[0].effect, ShellEffectFfi::Render(_)));
        }
        assert!(!view(&native).storage_busy);
        assert!(!view(&native).reading_time_save_pending);
    };
    sql.execute_batch("BEGIN IMMEDIATE").unwrap();
    for ms in [100, 200, 400, 300, 400] {
        checkpoint.pending = PendingReadingTime::from_millis(ms).unwrap();
        send(Event::CheckpointReadingTime(checkpoint));
    }
    send(Event::SaveReadingPosition(destination));
    send(Event::SaveReadingPosition(destination));
    assert_eq!(view(&native).reader.unwrap().position, first.position);
    assert!(callbacks.recv_timeout(Duration::from_millis(100)).is_err());
    sql.execute_batch("COMMIT").unwrap();
    await_callbacks(3); // First save, maximum coalesced save, then position.
    assert_eq!(view(&native).reader.unwrap().position, destination);
    assert!(view(&native).reader_time.is_none());
    let mut second = dispatch(&native, &callbacks, Event::RestoreReadingTime)
        .reader_time
        .unwrap();
    sql.execute_batch("BEGIN IMMEDIATE").unwrap();
    for ms in [500, 900, 700] {
        second.pending = PendingReadingTime::from_millis(ms).unwrap();
        send(Event::CheckpointReadingTime(second));
    }
    send(Event::CloseDocument);
    assert_eq!(view(&native).content_route, ContentRoute::Reader);
    sql.execute_batch("COMMIT").unwrap();
    await_callbacks(2); // Close follows the second save without another I/O.
    assert_eq!(view(&native).content_route, ContentRoute::Main);
    assert!(view(&native).reader.is_none());
    drop(native);
    assert!(callbacks.recv_timeout(Duration::from_secs(10)).is_err());
    let (native, callbacks, _) = bridge(&db.path());
    assert_eq!(
        dispatch(&native, &callbacks, Event::OpenReader(id))
            .reader
            .unwrap()
            .position,
        destination
    );
    assert_eq!(
        dispatch(&native, &callbacks, Event::RestoreReadingTime)
            .reader_time
            .unwrap()
            .pending
            .milliseconds(),
        900
    );
    dispatch(
        &native,
        &callbacks,
        Event::SaveReadingPosition(first.position),
    );
    assert_eq!(
        dispatch(&native, &callbacks, Event::RestoreReadingTime)
            .reader_time
            .unwrap()
            .pending
            .milliseconds(),
        400
    );
    let durable: Vec<(i64, i64)> = sql
        .prepare("SELECT milliseconds, revision FROM pending_reading_time ORDER BY byte_offset")
        .unwrap()
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();
    assert_eq!(durable, vec![(400, 3), (900, 3)]);
    for table in [
        "reading_completions",
        "reading_occurrences",
        "vocabulary_status_events",
    ] {
        assert_eq!(
            sql.query_row(&format!("SELECT count(*) FROM {table}"), [], |row| row
                .get::<_, i64>(0))
                .unwrap(),
            0
        );
    }
}

#[test]
fn analyzed_completion_flushes_freezes_retries_and_rereads_through_async_ffi() {
    use yomoku_core::{
        active_time::PendingReadingTime,
        analysis::{AnalysisProfile, AnalyzedToken, ChapterAnalysis, SplitMode},
        document::{PasteText, TextAnchor, TextOffset, TextRange},
        pagination::PageLength,
    };
    let db = Database::new();
    std::fs::create_dir(&db.0).unwrap();
    let mut storage = yomoku_storage::Storage::open(db.path()).unwrap();
    storage.save_page_length(PageLength::Characters250).unwrap();
    let imported = storage
        .create_analyzed_paste(
            &PasteText::new(format!("{}English 。\n犬 犬", "猫 ".repeat(250))).unwrap(),
            1,
            |chapter| {
                let mut cursor = 0;
                let tokens = chapter
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
                            is_japanese: matches!(surface, "猫" | "犬"),
                            reading: None,
                            dictionary_form: None,
                        }
                    })
                    .collect();
                Ok::<_, std::convert::Infallible>(ChapterAnalysis {
                    profile: AnalysisProfile {
                        engine_revision: "fixture-engine".into(),
                        dictionary_revision: "fixture-dictionary".into(),
                    },
                    mode: SplitMode::B,
                    tokens,
                })
            },
        )
        .unwrap();
    drop(storage);
    let (native, callbacks, _) = bridge(&db.path());
    dispatch(&native, &callbacks, Event::RefreshLibrary);
    let opened = dispatch(
        &native,
        &callbacks,
        Event::OpenReader(imported.document.document_id),
    );
    let first = opened.reader.unwrap();
    let request = Event::CompleteReaderPage {
        document_id: first.document_id,
        page: first.anchor,
        position: first.position,
        completed_at_unix_ms: 1234,
    };
    let mut checkpoint = dispatch(&native, &callbacks, Event::RestoreReadingTime)
        .reader_time
        .unwrap();
    let sql = rusqlite::Connection::open(db.path()).unwrap();
    let send = |event: Event| {
        let mut bytes = Vec::new();
        BincodeFfiFormat::serialize(&mut bytes, &event).unwrap();
        native.update(&bytes).unwrap();
    };
    let await_callbacks = |count| {
        for _ in 0..count {
            let (worker, bytes) = callbacks.recv_timeout(Duration::from_secs(10)).unwrap();
            assert_ne!(worker, thread::current().id());
            let requests: Requests<ShellEffectFfi> = BincodeFfiFormat::deserialize(&bytes).unwrap();
            assert!(requests
                .0
                .iter()
                .all(|request| matches!(request.effect, ShellEffectFfi::Render(_))));
        }
        assert!(!view(&native).storage_busy);
    };
    sql.execute_batch("BEGIN IMMEDIATE; CREATE TRIGGER fail BEFORE INSERT ON reading_occurrences BEGIN SELECT RAISE(ABORT, 'fixture'); END;").unwrap();
    for ms in [100, 400, 200] {
        checkpoint.pending = PendingReadingTime::from_millis(ms).unwrap();
        send(Event::CheckpointReadingTime(checkpoint));
    }
    send(request.clone());
    send(Event::CompleteReaderPage {
        completed_at_unix_ms: 9999,
        document_id: first.document_id,
        page: first.anchor,
        position: first.position,
    });
    assert!(view(&native).completion_save_pending);
    assert!(callbacks.recv_timeout(Duration::from_millis(100)).is_err());
    sql.execute_batch("COMMIT").unwrap();
    await_callbacks(4); // Two checkpoints, preparation, failed completion.
    assert_eq!(
        view(&native).document_error,
        Some(DocumentError::Unavailable)
    );
    assert!(view(&native).completion_save_pending);
    assert_eq!(view(&native).reader.unwrap(), first);
    let count = |table: &str| {
        sql.query_row(&format!("SELECT count(*) FROM {table}"), [], |row| {
            row.get::<_, i64>(0)
        })
        .unwrap()
    };
    assert_eq!(count("completion_commands"), 1);
    assert_eq!(count("reading_completions"), 0);
    assert_eq!(count("reading_occurrences"), 0);
    assert!(view(&native).completion_notice.is_none());
    sql.execute_batch("DROP TRIGGER fail").unwrap();
    let completed = dispatch(&native, &callbacks, Event::RetryReaderCompletion);
    assert!(!completed.completion_save_pending && !completed.reading_time_save_pending);
    assert!(completed.reader_time.is_none());
    assert_eq!(
        completed.reader_analysis,
        Some(ReaderAnalysis::Available(imported.analysis_id))
    );
    assert_eq!(
        completed.reader.as_ref().unwrap().position.offset,
        first.anchor.range.end
    );
    let notice = completed.completion_notice.unwrap();
    assert_eq!(notice.newly_known, 1);
    assert_eq!(i64::from(notice.command_id), 1);
    assert_eq!(count("completion_commands"), 1);
    assert_eq!(count("reading_occurrences"), 250);
    let frozen: (i64, i64, i64) = sql.query_row("SELECT completed_at_unix_ms, active_time_ms, (SELECT analysis_id FROM completion_analyses) FROM reading_completions", [], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?))).unwrap();
    assert_eq!(frozen, (1234, 400, i64::from(imported.analysis_id)));
    let states: Vec<(String, String)> = sql
        .prepare("SELECT surface, status FROM vocabulary ORDER BY surface")
        .unwrap()
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();
    assert!(states.contains(&("猫".into(), "known".into())));
    assert!(states.contains(&("犬".into(), "new".into())));
    let before = view(&native);
    send(request);
    send(Event::RetryReaderCompletion);
    assert_eq!(view(&native), before);
    assert!(callbacks.try_recv().is_err());
    dispatch(
        &native,
        &callbacks,
        Event::SaveReadingPosition(first.position),
    );
    assert_eq!(count("reading_completions"), 1); // Backward movement counts nothing.
    assert_eq!(
        dispatch(&native, &callbacks, Event::RestoreReadingTime)
            .reader_time
            .unwrap()
            .pending
            .milliseconds(),
        0
    );
    send(Event::CompleteReaderPage {
        document_id: first.document_id,
        page: first.anchor,
        position: first.position,
        completed_at_unix_ms: 5678,
    });
    await_callbacks(2);
    assert!(view(&native).completion_notice.is_none()); // Reread gets a new ID, no New words.
    assert_eq!(count("completion_commands"), 2);
    assert_eq!(count("reading_completions"), 2);
    assert_eq!(count("reading_occurrences"), 500);
    send(Event::CloseDocument);
    drop(native);
    assert!(callbacks.recv_timeout(Duration::from_secs(10)).is_err());
    let (native, callbacks, _) = bridge(&db.path());
    let reopened = dispatch(&native, &callbacks, Event::OpenReader(first.document_id));
    assert_eq!(
        reopened.reader.unwrap().position.offset,
        first.anchor.range.end
    );
    assert_eq!(
        reopened.reader_analysis,
        Some(ReaderAnalysis::Available(imported.analysis_id))
    );
    assert!(reopened.completion_notice.is_none() && !reopened.completion_save_pending);
}
