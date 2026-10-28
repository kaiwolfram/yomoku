//! Host-only, opt-in measurements over real TXT/EPUB inputs in a disposable DB.
//! Usage: measure_effective_reader DICTIONARY ITERATIONS BOOK.txt [BOOK.epub ...]
//! Inputs are read only. No user database, JNI, device or cold-cache claim.

use std::{
    error::Error,
    fs,
    hint::black_box,
    path::{Path, PathBuf},
    time::{Instant, SystemTime, UNIX_EPOCH},
};

use sha2::{Digest, Sha256};
use yomoku_analysis::{Analyzer, DICTIONARY_REVISION, ENGINE_REVISION};
use yomoku_core::{
    analysis::{ChapterAnalysis, OverrideScope, SplitMode, StoredOverride},
    document::{AnalysisId, ChapterText, DocumentId},
    effective_session::EffectiveReaderSession,
    pagination::{LogicalPages, PageLength},
    reader::{ReaderAnalysis, ReaderTokenKey},
    segmentation::{resolve_effective_segmentation, segmentation_from_snapshot},
};
use yomoku_storage::Storage;

type Result<T> = std::result::Result<T, Box<dyn Error>>;

struct Scratch(PathBuf);

impl Scratch {
    fn new() -> Result<Self> {
        let directory = std::env::temp_dir().join(format!(
            "yomoku-effective-measure-{}-{}",
            std::process::id(),
            SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos()
        ));
        fs::create_dir(&directory)?;
        Ok(Self(directory))
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        if let Err(error) = fs::remove_dir_all(&self.0) {
            eprintln!(
                "could not remove scratch directory {}: {error}",
                self.0.display()
            );
        }
    }
}

struct Chapter {
    document_id: DocumentId,
    text: ChapterText,
    original_id: AnalysisId,
    original: ChapterAnalysis,
    alternates: Vec<(AnalysisId, ChapterAnalysis)>,
}

fn measure(
    metric: &str,
    phase: &str,
    iterations: usize,
    mut operation: impl FnMut() -> Result<()>,
) -> Result<()> {
    // Explicit warmup. Timings include construction and destruction, exclude
    // assertions/fixture setup, and use the same connection and OS cache.
    operation()?;
    let mut times = Vec::with_capacity(iterations);
    for _ in 0..iterations {
        let start = Instant::now();
        operation()?;
        times.push(start.elapsed().as_secs_f64() * 1000.0);
    }
    times.sort_by(f64::total_cmp);
    let median = (times[(iterations - 1) / 2] + times[iterations / 2]) / 2.0;
    println!(
        "{metric},{phase},{iterations},{:.3},{:.3},{:.3}",
        times[0],
        median,
        times[iterations - 1]
    );
    Ok(())
}

fn scan(
    storage: &mut Storage,
    chapters: &[Chapter],
    rules: &[StoredOverride],
    iterations: usize,
    phase: &str,
) -> Result<()> {
    measure("resolve_all_chapters", phase, iterations, || {
        for chapter in chapters {
            black_box(resolve_effective_segmentation(
                &chapter.text,
                chapter.original_id,
                &chapter.original,
                &chapter.alternates,
                rules,
            )?);
        }
        Ok(())
    })?;
    measure("original_library", phase, iterations, || {
        black_box(storage.library_word_distributions()?);
        Ok(())
    })?;
    measure("effective_library", phase, iterations, || {
        black_box(storage.effective_library_word_distributions()?);
        Ok(())
    })?;

    let largest = chapters
        .iter()
        .max_by_key(|chapter| chapter.text.text.len())
        .ok_or("no chapters")?;
    let document_id = largest.document_id;
    let pages = LogicalPages::new(std::slice::from_ref(&largest.text), PageLength::default())?;
    let page = *pages
        .pages()
        .first()
        .ok_or("largest chapter has no pages")?;
    eprintln!(
        "{phase} page fixture: {} chapter characters, {} page characters",
        largest.text.text.chars().count(),
        page.slice(&largest.text)?.chars().count()
    );
    let base = ReaderTokenKey {
        document_id,
        page,
        analysis: ReaderAnalysis::Available(largest.original_id),
    };
    let mut session = EffectiveReaderSession::default();
    measure(
        "largest_chapter_page_load_accept",
        phase,
        iterations,
        || {
            let request = session.begin_load(base).map_err(|_| "revision exhausted")?;
            let tokens = storage
                .load_effective_reader_tokens(document_id, page)?
                .ok_or("document missing")?;
            if !session.accept(request, tokens) {
                return Err("page was rejected".into());
            }
            black_box(session.page());
            Ok(())
        },
    )?;
    Ok(())
}

fn import(storage: &mut Storage, analyzer: &Analyzer, path: &Path) -> Result<()> {
    let bytes = fs::read(path)?;
    let filename = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or("invalid filename")?;
    let start = Instant::now();
    match path
        .extension()
        .and_then(|extension| extension.to_str())
        .map(str::to_ascii_lowercase)
        .as_deref()
    {
        Some("txt") => {
            storage.import_analyzed_txt(&bytes, filename, None, 0, |chapter| {
                analyzer.analyze(chapter)
            })?;
        }
        Some("epub") => {
            storage
                .import_analyzed_epub(&bytes, filename, 0, |chapter| analyzer.analyze(chapter))?;
        }
        _ => return Err("expected .txt or .epub input".into()),
    }
    eprintln!(
        "import {}: {} source bytes, {:.3} ms, sha256={}",
        path.display(),
        bytes.len(),
        start.elapsed().as_secs_f64() * 1000.0,
        Sha256::digest(&bytes)
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>(),
    );
    Ok(())
}

fn database_digest(path: &Path) -> Result<[u8; 32]> {
    Ok(Sha256::digest(fs::read(path)?).into())
}

fn main() -> Result<()> {
    let mut arguments = std::env::args_os().skip(1);
    let dictionary = arguments
        .next()
        .ok_or("usage: measure_effective_reader DICTIONARY ITERATIONS BOOK.txt [BOOK.epub ...]")?;
    let iterations: usize = arguments
        .next()
        .ok_or("missing iterations")?
        .to_str()
        .ok_or("invalid iterations")?
        .parse()?;
    if !(1..=101).contains(&iterations) {
        return Err("iterations must be 1..=101".into());
    }
    let inputs: Vec<PathBuf> = arguments.map(PathBuf::from).collect();
    if inputs.is_empty() {
        return Err("at least one TXT/EPUB is required".into());
    }

    eprintln!("arch={} debug_assertions={} sqlite={}\nengine={ENGINE_REVISION}\ndictionary={DICTIONARY_REVISION}", std::env::consts::ARCH, cfg!(debug_assertions), rusqlite::version());

    let start = Instant::now();
    let analyzer = Analyzer::open_full(dictionary)?;
    eprintln!(
        "verified owned Full load: {:.3} ms",
        start.elapsed().as_secs_f64() * 1000.0
    );
    let scratch = Scratch::new()?;
    let database = scratch.0.join("measurement.sqlite3");
    let mut storage = Storage::open(&database)?;
    for input in inputs {
        import(&mut storage, &analyzer, &input)?;
    }
    let documents = storage.list_documents()?;
    let mut chapters = Vec::new();
    for document in &documents {
        let snapshot = storage
            .load_document_reader_snapshot(document.document_id)?
            .ok_or("document missing")?;
        for text in snapshot.document.chapters {
            let original_id = snapshot
                .analyses
                .iter()
                .find_map(|entry| match entry.analysis {
                    ReaderAnalysis::Available(id) if entry.chapter_id == text.chapter_id => {
                        Some(id)
                    }
                    _ => None,
                })
                .ok_or("missing original B")?;
            let original = storage
                .chapter_analysis(original_id)?
                .ok_or("missing B snapshot")?;
            chapters.push(Chapter {
                document_id: document.document_id,
                text,
                original_id,
                original,
                alternates: Vec::new(),
            });
        }
    }
    eprintln!(
        "fixture: {} documents, {} chapters, {} characters, {} B tokens, {} database bytes",
        documents.len(),
        chapters.len(),
        chapters
            .iter()
            .map(|c| c.text.text.chars().count())
            .sum::<usize>(),
        chapters
            .iter()
            .map(|c| c.original.tokens.len())
            .sum::<usize>(),
        fs::metadata(&database)?.len()
    );
    println!("metric,phase,iterations,min_ms,median_ms,max_ms");
    let before = database_digest(&database)?;
    assert_eq!(
        storage.library_word_distributions()?,
        storage.effective_library_word_distributions()?
    );
    scan(&mut storage, &chapters, &[], iterations, "b_only")?;
    assert_eq!(
        database_digest(&database)?,
        before,
        "measurement changed database bytes"
    );

    for mode in [SplitMode::A, SplitMode::C] {
        let start = Instant::now();
        let analyses = chapters
            .iter()
            .map(|chapter| analyzer.analyze_mode(&chapter.text, mode))
            .collect::<std::result::Result<Vec<_>, _>>()?;
        eprintln!(
            "whole-chapter {mode:?} analysis: {:.3} ms",
            start.elapsed().as_secs_f64() * 1000.0
        );
        let start = Instant::now();
        for (chapter, analysis) in chapters.iter_mut().zip(analyses) {
            let id = storage.store_chapter_analysis(chapter.text.chapter_id, &analysis)?;
            chapter.alternates.push((id, analysis));
        }
        eprintln!(
            "store {mode:?} snapshots: {:.3} ms",
            start.elapsed().as_secs_f64() * 1000.0
        );
    }
    let before = database_digest(&database)?;
    assert_eq!(
        storage.library_word_distributions()?,
        storage.effective_library_word_distributions()?
    );
    scan(&mut storage, &chapters, &[], iterations, "stored_abc")?;
    assert_eq!(
        database_digest(&database)?,
        before,
        "measurement changed database bytes"
    );

    let edit = chapters.iter().find_map(|chapter| {
        chapter
            .original
            .tokens
            .iter()
            .filter(|token| token.is_japanese)
            .find_map(|token| {
                chapter.alternates.iter().find_map(|(_, alternate)| {
                    let edit = segmentation_from_snapshot(
                        &chapter.text,
                        &chapter.original,
                        alternate,
                        token.anchor,
                        OverrideScope::AllOccurrences,
                    )
                    .ok()?;
                    (edit.parts.as_slice() != std::slice::from_ref(&edit.surface)).then_some(edit)
                })
            })
    });
    if let Some(edit) = edit {
        let (id, registered) = storage.save_registered_segmentation_override(&edit, 1)?;
        eprintln!(
            "global split fixture: {} parts, {registered} registrations",
            edit.parts.len()
        );
        let before = database_digest(&database)?;
        scan(
            &mut storage,
            &chapters,
            &[StoredOverride {
                id,
                edit,
                active: true,
            }],
            iterations,
            "global_split",
        )?;
        assert_eq!(
            database_digest(&database)?,
            before,
            "measurement changed database bytes"
        );
    } else {
        eprintln!("no aligned alternate split; global_split phase omitted");
    }
    Ok(())
}
