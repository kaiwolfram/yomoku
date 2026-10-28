//! Host-only, opt-in measurements over real TXT/EPUB inputs in a disposable DB.
//! Usage: measure_effective_reader DICTIONARY ITERATIONS [--epub-copies N] BOOK...
//! Inputs are read only. No user database, JNI, device or cold-cache claim.

use std::{
    error::Error,
    fs,
    hint::black_box,
    io::{Cursor, Read},
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
use yomoku_storage::{EpubImportOutcome, Storage};

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

fn import(storage: &mut Storage, analyzer: &Analyzer, path: &Path, copies: usize) -> Result<()> {
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
            for index in 1..copies {
                // Distinct source bytes exercise production dedup/registration
                // without changing any book member, text, ruby or image. This
                // deliberately measures repeated content, not corpus diversity.
                let copy = epub_copy(&bytes, index)?;
                let start = Instant::now();
                let result = storage.import_analyzed_epub(&copy, filename, 0, |chapter| {
                    analyzer.analyze(chapter)
                })?;
                if !matches!(result, EpubImportOutcome::Created(_)) {
                    return Err("generated EPUB copy unexpectedly deduplicated".into());
                }
                eprintln!(
                    "EPUB copy {}/{copies}: {:.3} ms",
                    index + 1,
                    start.elapsed().as_secs_f64() * 1000.0
                );
            }
        }
        _ => return Err("expected .txt or .epub input".into()),
    }
    eprintln!(
        "import {} ({copies} copies): {} original source bytes, {:.3} ms total, sha256={}",
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

fn epub_copy(bytes: &[u8], index: usize) -> Result<Vec<u8>> {
    let mut writer = zip::ZipWriter::new_append(Cursor::new(bytes.to_vec()))?;
    writer.set_comment(format!("yomoku host scaling fixture copy {index}"))?;
    Ok(writer.finish()?.into_inner())
}

fn database_digest(path: &Path) -> Result<[u8; 32]> {
    let mut file = fs::File::open(path)?;
    let mut digest = Sha256::new();
    let mut buffer = [0; 64 * 1024];
    loop {
        let count = file.read(&mut buffer)?;
        if count == 0 {
            return Ok(digest.finalize().into());
        }
        digest.update(&buffer[..count]);
    }
}

fn main() -> Result<()> {
    let mut arguments = std::env::args_os().skip(1);
    let dictionary = arguments
        .next()
        .ok_or("usage: measure_effective_reader DICTIONARY ITERATIONS [--epub-copies N] BOOK...")?;
    let iterations: usize = arguments
        .next()
        .ok_or("missing iterations")?
        .to_str()
        .ok_or("invalid iterations")?
        .parse()?;
    if !(1..=101).contains(&iterations) {
        return Err("iterations must be 1..=101".into());
    }
    let mut copy_count = None;
    let mut inputs = Vec::new();
    while let Some(argument) = arguments.next() {
        if argument == "--epub-copies" {
            if copy_count.is_some() {
                return Err("--epub-copies may be supplied only once".into());
            }
            let copies = arguments
                .next()
                .ok_or("missing EPUB copy count")?
                .to_str()
                .ok_or("invalid EPUB copy count")?
                .parse()?;
            if !(1..=1000).contains(&copies) {
                return Err("EPUB copies must be 1..=1000".into());
            }
            copy_count = Some(copies);
        } else {
            inputs.push(PathBuf::from(argument));
        }
    }
    if inputs.is_empty() {
        return Err("at least one TXT/EPUB is required".into());
    }
    let copies = copy_count.unwrap_or(1);
    if copy_count.is_some()
        && (inputs.len() != 1
            || !inputs[0]
                .extension()
                .is_some_and(|extension| extension.eq_ignore_ascii_case("epub")))
    {
        return Err("--epub-copies requires exactly one EPUB input".into());
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
        import(&mut storage, &analyzer, &input, copies)?;
    }
    let documents = storage.list_documents()?;
    if copy_count.is_some() && documents.len() != copies {
        return Err("EPUB scaling fixture has an unexpected document count".into());
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn scaling_copies_change_source_identity_without_changing_archive_members() {
        let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
        writer.set_comment("original comment").unwrap();
        for (name, text) in [
            ("mimetype", "application/epub+zip"),
            ("本文.xhtml", "猫𠮷\0"),
        ] {
            writer
                .start_file(name, zip::write::SimpleFileOptions::default())
                .unwrap();
            writer.write_all(text.as_bytes()).unwrap();
        }
        let original = writer.finish().unwrap().into_inner();
        let first = epub_copy(&original, 1).unwrap();
        let second = epub_copy(&original, 2).unwrap();
        assert_ne!(first, original);
        assert_ne!(first, second);
        assert_eq!(first, epub_copy(&original, 1).unwrap());
        for bytes in [&first, &second] {
            let mut before = zip::ZipArchive::new(Cursor::new(&original)).unwrap();
            let mut after = zip::ZipArchive::new(Cursor::new(bytes)).unwrap();
            assert_eq!(before.len(), after.len());
            for index in 0..before.len() {
                let mut left = before.by_index(index).unwrap();
                let mut right = after.by_index(index).unwrap();
                assert_eq!(left.name(), right.name());
                assert_eq!(left.compression(), right.compression());
                assert_eq!(left.crc32(), right.crc32());
                let mut left_bytes = Vec::new();
                let mut right_bytes = Vec::new();
                left.read_to_end(&mut left_bytes).unwrap();
                right.read_to_end(&mut right_bytes).unwrap();
                assert_eq!(left_bytes, right_bytes);
            }
        }
    }
}
