//! Standalone measurement tool, deliberately outside the app/runtime/FFI.
//! Run with tools/measure-sudachi-android.py; also runs on a Linux host.

use std::{error::Error, fs, hint::black_box, time::Instant};
use yomoku_analysis::{Analyzer, DICTIONARY_REVISION, ENGINE_REVISION};
use yomoku_core::{analysis::SplitMode, document::ChapterText};

fn memory(stage: &str) -> Result<(), Box<dyn Error>> {
    // Kernel snapshots, in KiB. HWM is a process high-water mark, not PSS.
    let status = fs::read_to_string("/proc/self/status")?;
    for key in ["VmRSS", "VmHWM", "RssAnon", "RssFile", "VmSwap"] {
        let line = status
            .lines()
            .find(|line| line.starts_with(&format!("{key}:")))
            .ok_or_else(|| format!("missing /proc/self/status field {key}"))?;
        println!("memory stage={stage} {line}");
    }
    Ok(())
}

fn chapter(text: String) -> ChapterText {
    ChapterText {
        source_id: 17.try_into().unwrap(),
        chapter_id: 29.try_into().unwrap(),
        text,
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = std::env::args_os().skip(1);
    let usage = "usage: android_probe DICTIONARY [owned|mapped]";
    let dictionary = args.next().ok_or(usage)?;
    let storage = args.next().unwrap_or_else(|| "owned".into());
    let storage = storage.to_str().ok_or(usage)?;
    if !matches!(storage, "owned" | "mapped") {
        return Err(usage.into());
    }
    if args.next().is_some() {
        return Err(usage.into());
    }
    println!(
        "probe_version=2 arch={} debug_assertions={} storage={storage}",
        std::env::consts::ARCH,
        cfg!(debug_assertions)
    );
    println!("engine={ENGINE_REVISION}\ndictionary={DICTIONARY_REVISION}");
    memory("baseline")?;
    let started = Instant::now();
    let analyzer = if storage == "mapped" {
        let file = fs::File::open(dictionary)?;
        // SAFETY: the measurement driver owns an isolated directory, publishes
        // read-only bytes before starting, and never writes/removes them until
        // all probe processes exit. Manual invocations must obey the same rule.
        unsafe { Analyzer::open_full_mapped(&file)? }
    } else {
        Analyzer::open_full(dictionary)?
    };
    println!(
        "initialization_ms={:.3}",
        started.elapsed().as_secs_f64() * 1000.0
    );
    memory("initialized")?;

    // Golden modes verify that this is the pinned real engine/data, with exact
    // original byte anchors. This first call includes lazy regex initialization.
    let modes = chapter("国家公務員".into());
    for (mode, expected) in [
        (SplitMode::B, vec!["国家", "公務員"]),
        (SplitMode::A, vec!["国家", "公務", "員"]),
        (SplitMode::C, vec!["国家公務員"]),
    ] {
        let started = Instant::now();
        let result = analyzer.analyze_mode(&modes, mode)?;
        let elapsed = started.elapsed();
        assert_eq!(
            result
                .tokens
                .iter()
                .map(|t| t.surface.as_str())
                .collect::<Vec<_>>(),
            expected
        );
        result.validate(&modes)?;
        println!(
            "golden mode={mode:?} elapsed_ms={:.3} tokens={}",
            elapsed.as_secs_f64() * 1000.0,
            result.tokens.len()
        );
    }

    let mixed = chapter("English 123 📖 日本語。𰻞".into());
    let result = analyzer.analyze(&mixed)?;
    result.validate(&mixed)?;
    assert_eq!(
        result
            .tokens
            .iter()
            .map(|t| t.surface.as_str())
            .collect::<String>(),
        mixed.text.replace(' ', "")
    );
    let rare = result
        .tokens
        .iter()
        .find(|t| t.surface == "𰻞")
        .ok_or("missing rare kanji")?;
    assert!(rare.is_japanese && rare.dictionary_form.is_none() && rare.reading.is_none());
    for surface in ["English", "123", "📖", "。"] {
        let token = result
            .tokens
            .iter()
            .find(|t| t.surface == surface)
            .ok_or("missing non-Japanese token")?;
        assert!(!token.is_japanese);
    }
    drop(result);
    for text in ["ｶﾞｯﾂﾎﾟｰｽﾞ", "e\u{301}と𠮷📖\0猫"] {
        let input = chapter(text.into());
        let result = analyzer.analyze(&input)?;
        assert_eq!(
            result
                .tokens
                .iter()
                .map(|t| t.surface.as_str())
                .collect::<String>(),
            text
        );
    }
    println!("mixed_unicode=PASS");
    memory("first_analysis")?;

    // Fixed synthetic workloads, not representative book-performance claims.
    // Every timed call includes creation of tokenizer, owned output and validation.
    let page = chapter("猫と犬。東京都と夏目漱石。食べる。".repeat(30));
    let long = chapter("猫と犬".repeat(8000));
    for (name, input, runs) in [("page", &page, 20), ("long_unbroken", &long, 3)] {
        let mut timings = Vec::new();
        let mut token_count = 0;
        for _ in 0..runs {
            let started = Instant::now();
            let result = analyzer.analyze(black_box(input))?;
            timings.push(started.elapsed().as_secs_f64() * 1000.0);
            token_count = result.tokens.len();
            black_box(&result);
        }
        timings.sort_by(f64::total_cmp);
        println!("workload={name} bytes={} scalars={} tokens={token_count} runs={runs} min_ms={:.3} median_ms={:.3} max_ms={:.3}", input.text.len(), input.text.chars().count(), timings[0], timings[runs / 2], timings[runs - 1]);
        memory(name)?;
    }
    drop(analyzer);
    memory("dropped")?;
    println!("validation=PASS");
    Ok(())
}
