//! Generate shell payloads from Rust types, never from the core's private effects.

use std::{env, error::Error, fs, path::PathBuf};

use crux_core::type_generation::facet::{Config, Export, TypeRegistry};
use yomoku_core::app::{Event, ViewModel};
use yomoku_core::document::{ChapterText, DocumentId, TextAnchor};
use yomoku_core::effective_session::{
    EffectiveReaderPage, EffectiveReaderSelection, EffectiveReaderWordState,
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
        .build()?
        .kotlin(&Config::builder("app.yomoku.shared", &output).build())?;
    let runtime = output.join("com/novi/serde/BinaryDeserializer.kt");
    fs::write(&runtime, scalar_loops(&fs::read_to_string(&runtime)?)?)?;
    Ok(())
}

// facet_generate 0.19.1 constructs two progressions per scalar read on Android.
// Keep byte-at-a-time reads (including partial-read error offsets) unchanged.
// Fail closed if an upstream runtime change invalidates this narrow adaptation.
fn scalar_loops(source: &str) -> Result<String, Box<dyn Error>> {
    let mut result = source.to_owned();
    for (ty, mask) in [("ULong", "0xffuL"), ("Long", "0xffL")] {
        let legacy = format!(
            r"        for (shift in 0 until 64 step 8) {{
            val byteValue = getByte().to{ty}() and {mask}
            value = value or (byteValue shl shift)
        }}"
        );
        if result.matches(&legacy).count() != 1 {
            return Err(format!("Expected exactly one pinned {ty} scalar decoder loop").into());
        }
        let replacement = format!(
            r"        var shift = 0
        while (shift < 64) {{
            val byteValue = getByte().to{ty}() and {mask}
            value = value or (byteValue shl shift)
            shift += 8
        }}"
        );
        result = result.replace(&legacy, &replacement);
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::scalar_loops;

    #[test]
    fn adapts_only_both_pinned_scalar_loops() {
        let source = ["ULong", "Long"].map(|ty| {
            let mask = if ty == "ULong" { "0xffuL" } else { "0xffL" };
            format!(
                "        for (shift in 0 until 64 step 8) {{\n            val byteValue = getByte().to{ty}() and {mask}\n            value = value or (byteValue shl shift)\n        }}"
            )
        }).join("\n");
        let adapted = scalar_loops(&source).unwrap();
        assert_eq!(adapted.matches("while (shift < 64)").count(), 2);
        assert_eq!(adapted.matches("shift += 8").count(), 2);
        assert!(!adapted.contains("step 8"));
        assert!(scalar_loops(&adapted).is_err());
        assert!(scalar_loops(&source.replace("0xffL", "255L")).is_err());
        assert!(scalar_loops(&format!("{source}\n{source}")).is_err());
    }
}
