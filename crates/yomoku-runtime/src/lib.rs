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
mod imports;
pub use imports::{AnalyzerLifecycle, AnalyzerLoader, PasteAnalyzer};

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

/// Analyzed, deduplicating imports on the existing SQLite worker. The shell
/// supplies a lazy loader, not an analyzer created on the UI thread. The legacy
/// router is retained only for low-level storage fixtures; NativeBridge requires
/// this analyzed router and a verified-descriptor publisher.
pub fn effect_router_with_analyzed_imports(
    database_path: std::path::PathBuf,
    loader: AnalyzerLoader,
) -> std::io::Result<(EffectRouter, AnalyzerLifecycle)> {
    let (effects, lifecycle) = StorageEffects::with_analyzed_imports(database_path, loader)?;
    Ok((
        Core::<Yomoku>::new()
            .handle_effects_using(effects)
            .map_effect(),
        lifecycle,
    ))
}
