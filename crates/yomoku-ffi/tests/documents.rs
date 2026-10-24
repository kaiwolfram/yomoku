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
    BincodeFfiFormat::serialize(&mut last_public, &Event::RetryReadingTimeCheckpoint).unwrap();
    let internal_tag = u32::from_le_bytes(last_public.try_into().unwrap()) + 1;
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
