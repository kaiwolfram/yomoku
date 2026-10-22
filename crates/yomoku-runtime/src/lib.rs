//! Rust-side effect routing between the pure application and the shell.

use crux_core::{
    macros::effect,
    middleware::{HandleEffectLayer, Layer, MapEffectLayer},
    render::RenderOperation,
    Core,
};
use yomoku_core::app::{Effect, Yomoku};

mod storage;
use storage::StorageEffects;

/// Only operations owned by the platform shell belong in this enum.
#[effect(facet_typegen)]
pub enum ShellEffect {
    Render(RenderOperation),
}

impl From<Effect> for ShellEffect {
    fn from(effect: Effect) -> Self {
        // Deliberately exhaustive: adding a core operation requires deciding
        // its owner here. Rust I/O must be handled below this mapping layer.
        match effect {
            Effect::Render(request) => Self::Render(request),
            Effect::Storage(_) => unreachable!("storage effects are consumed by Rust middleware"),
        }
    }
}

/// SQLite effects are consumed before the shell mapping; only Render crosses FFI.
pub type EffectRouter =
    MapEffectLayer<HandleEffectLayer<Core<Yomoku>, StorageEffects>, ShellEffect>;

pub fn effect_router(database_path: std::path::PathBuf) -> std::io::Result<EffectRouter> {
    Ok(Core::<Yomoku>::new()
        .handle_effects_using(StorageEffects::new(database_path)?)
        .map_effect())
}
