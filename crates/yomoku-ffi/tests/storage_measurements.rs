#![cfg(feature = "storage-measurements")]

use std::{
    path::PathBuf,
    sync::{
        atomic::{AtomicUsize, Ordering},
        mpsc, Arc,
    },
    thread::{self, ThreadId},
    time::{Duration, Instant},
};

use crux_core::bridge::{BincodeFfiFormat, FfiFormat, Requests};
use yomoku_core::{
    app::{Destination, Event, ViewModel},
    vocabulary::{Surface, VocabularyQuery},
};
use yomoku_ffi::YomokuBridge;
use yomoku_runtime::{
    storage_measurements::{StorageMeasurement, StorageRead},
    AnalyzerLifecycle, ShellEffectFfi,
};
use yomoku_storage::Storage;

const TIMEOUT: Duration = Duration::from_secs(10);

struct Database(PathBuf);

impl Database {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let path = std::env::temp_dir().join(format!(
            "yomoku-worker-phases-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&path).unwrap();
        Self(path)
    }

    fn path(&self) -> PathBuf {
        self.0.join("scratch.sqlite")
    }

    fn seed(&self) {
        let mut storage = Storage::open(self.path()).unwrap();
        let surfaces: Vec<_> = (0..205)
            .map(|index| Surface::new(format!("語{index:03}")).unwrap())
            .collect();
        storage.register_vocabulary(&surfaces, 1).unwrap();
    }
}

impl Drop for Database {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).unwrap();
    }
}

enum Delivery {
    Callback(Vec<u8>, ThreadId),
    Measurement(StorageMeasurement, ThreadId),
}

struct Session {
    bridge: YomokuBridge,
    lifecycle: AnalyzerLifecycle,
    deliveries: mpsc::Receiver<Delivery>,
}

impl Session {
    fn new(database: &Database) -> Self {
        let (sender, deliveries) = mpsc::channel();
        let callback_sender = sender.clone();
        let (bridge, lifecycle) = YomokuBridge::with_measured_effective_reader(
            database.path(),
            Box::new(|| panic!("must not load analyzer")),
            Box::new(|| panic!("must not load dictionary")),
            Arc::new(move |sample| {
                sender
                    .send(Delivery::Measurement(sample, thread::current().id()))
                    .unwrap();
            }),
            move |effects| {
                callback_sender
                    .send(Delivery::Callback(effects.unwrap(), thread::current().id()))
                    .unwrap();
            },
        )
        .unwrap();
        let session = Self {
            bridge,
            lifecycle,
            deliveries,
        };
        session.update(Event::SelectDestination(Destination::Vocabulary));
        session
    }

    fn update(&self, event: Event) {
        let mut bytes = Vec::new();
        BincodeFfiFormat::serialize(&mut bytes, &event).unwrap();
        let effects = self.bridge.update(&bytes).unwrap();
        let _: Requests<ShellEffectFfi> = BincodeFfiFormat::deserialize(&effects).unwrap();
    }

    fn completed(&self, operation: StorageRead, failed: bool) -> StorageMeasurement {
        let Delivery::Callback(effects, callback_thread) =
            self.deliveries.recv_timeout(TIMEOUT).unwrap()
        else {
            panic!("measurement must follow response serialization and host callback");
        };
        let _: Requests<ShellEffectFfi> = BincodeFfiFormat::deserialize(&effects).unwrap();
        let Delivery::Measurement(sample, observer_thread) =
            self.deliveries.recv_timeout(TIMEOUT).unwrap()
        else {
            panic!("expected exactly one measurement");
        };
        assert_eq!(callback_thread, observer_thread);
        assert_ne!(observer_thread, thread::current().id());
        assert_eq!(sample.operation, operation);
        assert_eq!(sample.failed, failed);
        assert!(self.deliveries.try_recv().is_err());
        sample
    }

    fn view(&self) -> ViewModel {
        BincodeFfiFormat::deserialize(&self.bridge.view().unwrap()).unwrap()
    }
}

#[test]
fn measured_reads_preserve_full_ids_and_include_wait_behind_an_existing_fifo_job() {
    let database = Database::new();
    database.seed();
    let storage = Storage::open(database.path()).unwrap();
    let expected = storage
        .vocabulary_page(&VocabularyQuery::default(), 0)
        .unwrap();
    let expected_ids = storage
        .vocabulary_matching_ids(&VocabularyQuery::default())
        .unwrap();
    let before = std::fs::read(database.path()).unwrap();
    let session = Session::new(&database);

    // Block an earlier FIFO callback, enqueue the read, then release the worker.
    // Assert causal interval bounds without sleeps or performance thresholds.
    let (entered, entry) = mpsc::channel();
    let (release, released) = mpsc::channel();
    session
        .lifecycle
        .load_epub_cover_reference(1.try_into().unwrap(), move |_| {
            entered.send(()).unwrap();
            released.recv_timeout(TIMEOUT).unwrap();
        })
        .unwrap();
    entry.recv_timeout(TIMEOUT).unwrap();
    session.update(Event::LoadVocabulary);
    let submitted = Instant::now();
    let resumed = Instant::now();
    release.send(()).unwrap();
    let sample = session.completed(StorageRead::VocabularyPage, false);
    assert!(sample.queue >= resumed.duration_since(submitted));
    assert_eq!(session.view().vocabulary.rows, expected.rows);

    session.update(Event::SelectAllVocabulary);
    session.completed(StorageRead::VocabularyMatchingIds, false);
    assert_eq!(
        session.view().vocabulary.selection.as_ref(),
        Some(&expected_ids)
    );
    session.update(Event::RequestVocabularyBulkAction(
        yomoku_core::vocabulary::VocabularyBulkAction::SetStatus {
            status: yomoku_core::vocabulary::VocabularyStatus::Known,
            occurred_at_unix_ms: 999,
        },
    ));
    assert_eq!(
        session.view().vocabulary.confirmation.unwrap().ids,
        expected_ids
    );
    session.update(Event::SelectAllVocabulary);
    // Rejected/replayed/synchronous events must not manufacture worker samples.
    assert!(session.deliveries.try_recv().is_err());
    assert_eq!(std::fs::read(database.path()).unwrap(), before);
}

#[test]
fn failed_page_and_exact_retry_each_report_once_without_altering_error_behavior() {
    let database = Database::new();
    database.seed();
    let connection = rusqlite::Connection::open(database.path()).unwrap();
    connection
        .execute_batch("PRAGMA ignore_check_constraints=ON; UPDATE vocabulary SET status='bad';")
        .unwrap();
    let before = std::fs::read(database.path()).unwrap();
    let session = Session::new(&database);
    session.update(Event::LoadVocabulary);
    session.completed(StorageRead::VocabularyPage, true);
    let failed = session.view();
    assert!(failed.vocabulary.error.is_some());
    assert!(!failed.storage_busy);
    assert_eq!(std::fs::read(database.path()).unwrap(), before);
    session.update(Event::LoadVocabulary);
    assert!(session.deliveries.try_recv().is_err());
    assert_eq!(session.view(), failed);

    connection
        .execute_batch("UPDATE vocabulary SET status='new';")
        .unwrap();
    let repaired = std::fs::read(database.path()).unwrap();
    session.update(Event::RetryVocabulary);
    session.completed(StorageRead::VocabularyPage, false);
    let retried = session.view();
    assert!(retried.vocabulary.error.is_none());
    assert_eq!(retried.vocabulary.rows.len(), 100);
    assert_eq!(retried.vocabulary.matching_count, Some(205));
    assert_eq!(std::fs::read(database.path()).unwrap(), repaired);
}
