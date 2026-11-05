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
    storage::DocumentError,
};
use yomoku_ffi::YomokuBridge as NativeBridge;
mod support;
use yomoku_runtime::ShellEffectFfi;

type Callbacks = mpsc::Receiver<(ThreadId, Vec<u8>)>;

fn bridge(path: &str) -> (NativeBridge, Callbacks, Weak<()>) {
    let (sender, receiver) = mpsc::channel();
    let life = Arc::new(());
    let weak = Arc::downgrade(&life);
    (
        support::bridge(path.into(), move |result| {
            let _keep_alive = &life;
            let _ = sender.send((thread::current().id(), result.unwrap()));
        })
        .unwrap()
        .0,
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
