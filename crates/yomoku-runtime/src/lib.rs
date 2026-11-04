//! Rust-side effect routing between the pure application and the shell.

use crux_core::{
    macros::effect,
    middleware::{HandleEffectLayer, Layer, MapEffectLayer},
    render::RenderOperation,
    Core,
};
use yomoku_core::app::{Effect, Yomoku};

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

    AnkiExportPreflight(yomoku_core::anki::AnkiPreflightOperation),
    AnkiLive(yomoku_core::anki::AnkiLiveOperation),
}

impl From<Effect> for ShellEffect {
    fn from(effect: Effect) -> Self {
        // Deliberately exhaustive: adding a core operation requires deciding
        // its owner here. Rust I/O must be handled below this mapping layer.
        match effect {
            Effect::Render(request) => Self::Render(request),
            Effect::AnkiExportPreflight(request) => Self::AnkiExportPreflight(request),
            Effect::AnkiLive(request) => Self::AnkiLive(request),
            Effect::Storage(_) => unreachable!("storage effects are consumed by Rust middleware"),
        }
    }
}

/// SQLite effects stay native; render and typed Anki provider requests cross FFI.
pub type EffectRouter =
    MapEffectLayer<HandleEffectLayer<Core<Yomoku>, StorageEffects>, ShellEffect>;

/// Production effective reader, sharing the import and dictionary
/// loaders, SQLite FIFO and lifecycle. NativeBridge selects this router; every
/// session uses the same reader implementation.
pub fn effect_router(
    database_path: std::path::PathBuf,
    loader: AnalyzerLoader,
    word_loader: WordDictionaryLoader,
) -> std::io::Result<(EffectRouter, AnalyzerLifecycle)> {
    let (effects, lifecycle) = StorageEffects::new(database_path, loader, word_loader)?;
    Ok((
        Core::<Yomoku>::new()
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
    let (mut effects, lifecycle) = StorageEffects::new(database_path, loader, word_loader)?;
    effects.measurements = Some(observer);
    Ok((
        Core::<Yomoku>::new()
            .handle_effects_using(effects)
            .map_effect(),
        lifecycle,
    ))
}
