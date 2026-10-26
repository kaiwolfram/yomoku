use std::sync::{Arc, Weak};

use crux_core::bridge::{BincodeFfiFormat, FfiFormat, Requests};
use yomoku_core::app::{Destination, Event, ViewModel};
use yomoku_ffi::{
    native::{DictionaryProvider, EffectsListener, NativeBridge},
    FfiError,
};
use yomoku_runtime::ShellEffectFfi;

struct Listener {
    _lifetime: Arc<()>,
}

impl EffectsListener for Listener {
    fn on_effects(&self, _: Vec<u8>) {
        panic!("navigation must return effects synchronously");
    }

    fn on_error(&self, _: FfiError) {
        panic!("navigation must return errors synchronously");
    }
}

struct NoDictionary;
impl DictionaryProvider for NoDictionary {
    fn verified_word_dictionary_path(&self) -> String {
        String::new()
    }
    fn open_verified_dictionary(&self) -> i64 {
        panic!("navigation must not load the dictionary");
    }
}

fn bridge() -> (NativeBridge, Weak<()>) {
    let lifetime = Arc::new(());
    let weak = Arc::downgrade(&lifetime);
    (
        NativeBridge::new(
            ":memory:".into(),
            Box::new(Listener {
                _lifetime: lifetime,
            }),
            Box::new(NoDictionary),
        )
        .unwrap(),
        weak,
    )
}

fn view(bridge: &NativeBridge) -> ViewModel {
    BincodeFfiFormat::deserialize(&bridge.view().unwrap()).unwrap()
}

#[test]
fn native_facade_preserves_payloads_errors_and_instance_isolation() {
    let (first, _) = bridge();
    let (second, _) = bridge();
    let mut event = Vec::new();
    BincodeFfiFormat::serialize(
        &mut event,
        &Event::SelectDestination(Destination::Vocabulary),
    )
    .unwrap();
    let requests: Requests<ShellEffectFfi> =
        BincodeFfiFormat::deserialize(&first.update(&event).unwrap()).unwrap();
    assert_eq!(requests.0.len(), 1);
    assert!(matches!(requests.0[0].effect, ShellEffectFfi::Render(_)));
    assert_eq!(first.update(&[]), Err(FfiError::InvalidEvent));
    assert_eq!(
        first.resolve(requests.0[0].id.0, &[]),
        Err(FfiError::InvalidResponse)
    );
    assert_eq!(view(&first).destination, Destination::Vocabulary);
    assert_eq!(view(&second).destination, Destination::Library);
}

#[test]
fn native_facade_can_be_shared_across_threads_and_releases_listener() {
    let (bridge, lifetime) = bridge();
    let bridge = Arc::new(bridge);
    let worker = Arc::clone(&bridge);
    std::thread::spawn(move || {
        assert_eq!(view(&worker).destination, Destination::Library);
    })
    .join()
    .unwrap();
    assert!(lifetime.upgrade().is_some());
    drop(bridge);
    assert!(lifetime.upgrade().is_none());
}

#[test]
fn native_imports_require_a_dictionary_and_never_fall_back_to_text_only() {
    use std::sync::{
        atomic::{AtomicUsize, Ordering},
        mpsc,
    };
    use yomoku_core::storage::DocumentError;
    struct Missing(Arc<AtomicUsize>);
    impl DictionaryProvider for Missing {
        fn verified_word_dictionary_path(&self) -> String {
            String::new()
        }
        fn open_verified_dictionary(&self) -> i64 {
            self.0.fetch_add(1, Ordering::SeqCst);
            0
        }
    }
    struct Channel(mpsc::Sender<Vec<u8>>);
    impl EffectsListener for Channel {
        fn on_effects(&self, bytes: Vec<u8>) {
            self.0.send(bytes).unwrap();
        }
        fn on_error(&self, error: FfiError) {
            panic!("unexpected {error:?}");
        }
    }
    let (send, receive) = mpsc::channel();
    let calls = Arc::new(AtomicUsize::new(0));
    let native = NativeBridge::new(
        ":memory:".into(),
        Box::new(Channel(send)),
        Box::new(Missing(calls.clone())),
    )
    .unwrap();
    for event in [
        Event::RefreshLibrary,
        Event::CreatePaste("猫".into()),
        Event::CreatePaste("猫".into()),
    ] {
        let mut bytes = vec![];
        BincodeFfiFormat::serialize(&mut bytes, &event).unwrap();
        native.update(&bytes).unwrap();
        receive
            .recv_timeout(std::time::Duration::from_secs(10))
            .unwrap();
        let view = view(&native);
        assert!(view.documents.is_empty());
        assert!(view.import_summary.is_none());
        if matches!(event, Event::CreatePaste(_)) {
            assert_eq!(view.document_error, Some(DocumentError::Unavailable));
        } else {
            assert_eq!(calls.load(Ordering::SeqCst), 0);
        }
    }
    assert_eq!(calls.load(Ordering::SeqCst), 2);
}

#[test]
fn native_asset_requests_validate_before_enqueue_and_release_one_shot_listeners() {
    use std::{sync::mpsc, thread, time::Duration};
    use yomoku_ffi::native::{EpubAssetError, EpubAssetListener};
    struct AssetListener {
        result: mpsc::Sender<(thread::ThreadId, EpubAssetError)>,
        dropped: mpsc::Sender<()>,
    }
    impl EpubAssetListener for AssetListener {
        fn on_asset(&self, _: String, _: Vec<u8>) {
            panic!("no saved image");
        }
        fn on_error(&self, error: EpubAssetError) {
            self.result.send((thread::current().id(), error)).unwrap();
        }
    }
    impl Drop for AssetListener {
        fn drop(&mut self) {
            self.dropped.send(()).unwrap();
        }
    }
    let (native, _) = bridge();
    for (id, path, media) in [
        (0, "OPS/image.png", "image/png"),
        (-1, "OPS/image.png", "image/png"),
        (1, "../image.png", "image/png"),
        (1, "OPS/image.png", "text/plain"),
    ] {
        let (send, receive) = mpsc::channel();
        let (dropped, drop_receive) = mpsc::channel();
        assert_eq!(
            native.load_epub_image_asset(
                id,
                path.into(),
                media.into(),
                Box::new(AssetListener {
                    result: send,
                    dropped
                })
            ),
            Err(EpubAssetError::InvalidRequest)
        );
        drop_receive.recv_timeout(Duration::from_secs(10)).unwrap();
        assert!(matches!(
            receive.try_recv(),
            Err(mpsc::TryRecvError::Disconnected)
        ));
    }
    let (send, receive) = mpsc::channel();
    let (dropped, drop_receive) = mpsc::channel();
    native
        .load_epub_image_asset(
            1,
            "OPS/image.png".into(),
            "image/png".into(),
            Box::new(AssetListener {
                result: send,
                dropped,
            }),
        )
        .unwrap();
    let (worker, error) = receive.recv_timeout(Duration::from_secs(10)).unwrap();
    assert_ne!(worker, thread::current().id());
    assert_eq!(error, EpubAssetError::NotFound);
    drop_receive.recv_timeout(Duration::from_secs(10)).unwrap();
    assert!(matches!(
        receive.try_recv(),
        Err(mpsc::TryRecvError::Disconnected)
    ));
}
