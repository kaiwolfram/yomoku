//! Rust-side effect routing between the pure application and the shell.

use crux_core::{
    macros::effect,
    middleware::{HandleEffectLayer, Layer, MapEffectLayer},
    render::RenderOperation,
    Core,
};
use yomoku_core::app::{Effect, Model, Yomoku};

mod epub_assets;
mod storage;
#[cfg(feature = "storage-measurements")]
pub mod storage_measurements;
use storage::StorageEffects;
pub use storage::WordDictionaryLoader;
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

/// Production reader word lookups and imports share one lazy FIFO worker.
pub fn effect_router_with_dictionaries(
    database_path: std::path::PathBuf,
    loader: AnalyzerLoader,
    word_loader: WordDictionaryLoader,
) -> std::io::Result<(EffectRouter, AnalyzerLifecycle)> {
    let (effects, lifecycle) =
        StorageEffects::with_dictionaries(database_path, loader, word_loader)?;
    Ok((
        Core::<Yomoku>::new()
            .handle_effects_using(effects)
            .map_effect(),
        lifecycle,
    ))
}

/// Production effective reader, sharing the import and dictionary
/// loaders, SQLite FIFO and lifecycle. NativeBridge selects this router; legacy
/// factories remain available for isolated original-B regressions.
pub fn effect_router_with_effective_reader(
    database_path: std::path::PathBuf,
    loader: AnalyzerLoader,
    word_loader: WordDictionaryLoader,
) -> std::io::Result<(EffectRouter, AnalyzerLifecycle)> {
    let (effects, lifecycle) =
        StorageEffects::with_dictionaries(database_path, loader, word_loader)?;
    Ok((
        Core::new_with(Yomoku, Model::with_effective_reader())
            .handle_effects_using(effects)
            .map_effect(),
        lifecycle,
    ))
}

/// Opt-in host diagnostics on the same effective-reader worker and core.
/// Not exported through the Android binding or included in default builds.
#[cfg(feature = "storage-measurements")]
pub fn effect_router_with_measured_effective_reader(
    database_path: std::path::PathBuf,
    loader: AnalyzerLoader,
    word_loader: WordDictionaryLoader,
    observer: storage_measurements::StorageObserver,
) -> std::io::Result<(EffectRouter, AnalyzerLifecycle)> {
    let (mut effects, lifecycle) =
        StorageEffects::with_dictionaries(database_path, loader, word_loader)?;
    effects.measurements = Some(observer);
    Ok((
        Core::new_with(Yomoku, Model::with_effective_reader())
            .handle_effects_using(effects)
            .map_effect(),
        lifecycle,
    ))
}
