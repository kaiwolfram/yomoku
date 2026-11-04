//! Exercise the real application rather than the isolated retained component.
use super::*;
use yomoku_core::app::{Event as AppEvent, Model, ViewModel, Yomoku};

type AppRouter = HandleEffectLayer<Core<Yomoku>, StorageEffects>;

mod library;

fn router(fixture: &Fixture) -> AppRouter {
    let (effects, _) = StorageEffects::new(
        fixture.path(),
        Box::new(|| panic!("application reads/completion must not load Sudachi")),
        Box::new(|| panic!("application reads/completion must not load word dictionaries")),
    )
    .unwrap();
    Core::new_with(Yomoku, Model::default()).handle_effects_using(effects)
}

fn dispatch(router: &AppRouter, event: AppEvent) -> ViewModel {
    let (sender, receiver) = mpsc::channel();
    let effects = router.update(event, move |effects| {
        assert!(effects
            .iter()
            .all(|effect| matches!(effect, Effect::Render(_))));
        sender.send(()).unwrap();
    });
    assert!(effects
        .iter()
        .all(|effect| matches!(effect, Effect::Render(_))));
    let deadline = std::time::Instant::now() + TIMEOUT;
    loop {
        let view = router.view();
        if !view.storage_busy {
            return view;
        }
        receiver
            .recv_timeout(deadline.saturating_duration_since(std::time::Instant::now()))
            .unwrap();
    }
}
