//! Generate shell payloads from Rust types, never from the core's private effects.

use std::{env, error::Error, path::PathBuf};

use crux_core::type_generation::facet::{Config, Export, TypeRegistry};
use yomoku_core::app::{Event, ViewModel};
use yomoku_core::document::{ChapterText, DocumentId, TextAnchor};
use yomoku_core::effective_session::{
    EffectiveOverrideAction, EffectiveReaderOverrideState, EffectiveReaderPage,
    EffectiveReaderSelection, EffectiveReaderWordState,
};
use yomoku_runtime::ShellEffect;

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = env::args_os().skip(1);
    let output = args
        .next()
        .map(PathBuf::from)
        .ok_or("usage: generate-kotlin <generated-output-directory>")?;
    if args.next().is_some() {
        return Err("usage: generate-kotlin <generated-output-directory>".into());
    }

    let mut registry = TypeRegistry::new();
    ShellEffect::register_types(&mut registry)?;
    registry
        .register_type::<Event>()?
        .register_type::<ViewModel>()?
        .register_type::<DocumentId>()?
        .register_type::<TextAnchor>()?
        .register_type::<ChapterText>()?
        // Compact schemas also used by staged native application integration.
        // Production activation waits for the coordinated reader switch.
        .register_type::<EffectiveReaderPage>()?
        .register_type::<EffectiveReaderSelection>()?
        .register_type::<EffectiveReaderWordState>()?
        .register_type::<EffectiveReaderOverrideState>()?
        .register_type::<EffectiveOverrideAction>()?
        .build()?
        .kotlin(&Config::builder("app.yomoku.shared", output).build())?;
    Ok(())
}
