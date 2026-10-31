//! Schedule native recovery before the initial Library read. Constructors stay
//! free of database I/O; startup emits no provider reads and keeps journals native.

use crux_core::{middleware::Layer, Resolvable, ResolveError};
use yomoku_core::app::Event;

pub struct AnkiStartupLayer<L> {
    inner: L,
}

impl<L> AnkiStartupLayer<L> {
    pub(crate) fn new(inner: L) -> Self {
        Self { inner }
    }
}

impl<L: Layer<Event = Event>> Layer for AnkiStartupLayer<L> {
    type Event = Event;
    type Effect = L::Effect;
    type ViewModel = L::ViewModel;

    fn update<F>(&self, event: Event, effect_callback: F) -> Vec<Self::Effect>
    where
        F: Fn(Vec<Self::Effect>) + Sync + Send + 'static,
    {
        if !matches!(event, Event::RefreshLibrary) {
            return self.inner.update(event, effect_callback);
        }
        // The app coalesces repeated startup requests and never implicitly
        // retries a failure. Recovery has its own read slot on the same FIFO.
        let callback = std::sync::Arc::new(effect_callback);
        let startup_callback = callback.clone();
        let mut effects = self
            .inner
            .update(Event::RestoreAnkiRecovery, move |effects| {
                startup_callback(effects);
            });
        effects.extend(self.inner.update(event, move |effects| callback(effects)));
        effects
    }

    fn resolve<Output, F>(
        &self,
        request: &mut impl Resolvable<Output>,
        output: Output,
        effect_callback: F,
    ) -> Result<Vec<Self::Effect>, ResolveError>
    where
        F: Fn(Vec<Self::Effect>) + Sync + Send + 'static,
    {
        self.inner.resolve(request, output, effect_callback)
    }

    fn process_tasks<F>(&self, effect_callback: F) -> Vec<Self::Effect>
    where
        F: Fn(Vec<Self::Effect>) + Sync + Send + 'static,
    {
        self.inner.process_tasks(effect_callback)
    }

    fn view(&self) -> Self::ViewModel {
        self.inner.view()
    }
}

#[cfg(test)]
mod tests;
