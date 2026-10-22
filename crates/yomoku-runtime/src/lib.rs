//! Rust-side effect routing between the pure application and the shell.

use crux_core::{
    macros::effect,
    middleware::{Layer, MapEffectLayer},
    render::RenderOperation,
    Core,
};
use yomoku_core::app::{Effect, Yomoku};

/// Only operations owned by the platform shell belong in this enum.
#[effect(typegen)]
pub enum ShellEffect {
    Render(RenderOperation),
}

impl From<Effect> for ShellEffect {
    fn from(effect: Effect) -> Self {
        // Deliberately exhaustive: adding a core operation requires deciding
        // its owner here. Rust I/O must be handled below this mapping layer.
        match effect {
            Effect::Render(request) => Self::Render(request),
        }
    }
}

/// Current routing stack. Add Rust I/O handlers before the shell mapping when
/// storage/analysis/network operations are introduced.
pub type EffectRouter = MapEffectLayer<Core<Yomoku>, ShellEffect>;

pub fn effect_router() -> EffectRouter {
    Core::<Yomoku>::new().map_effect()
}
