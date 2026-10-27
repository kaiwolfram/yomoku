//! Explicit data tests: tools/test-sudachi.sh. Ordinary workspace checks need no
//! 315 MiB dictionary and report these as ignored, never silently skip them.

use std::{path::PathBuf, sync::OnceLock};
use yomoku_analysis::{Analyzer, DICTIONARY_REVISION, ENGINE_REVISION};
use yomoku_core::{
    analysis::{ChapterAnalysis, SplitMode},
    document::ChapterText,
};

#[path = "../../yomoku-import/tests/support/epub.rs"]
mod epub_fixture;

fn analyzer() -> &'static Analyzer {
    static ANALYZER: OnceLock<Analyzer> = OnceLock::new();
    ANALYZER.get_or_init(|| {
        let path = std::env::var_os("YOMOKU_SUDACHI_DICTIONARY")
            .map(PathBuf::from)
            .unwrap_or_else(|| {
                PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                    .join("../../.local/sudachi/full-20261015/system_full.dic")
            });
        match std::env::var("YOMOKU_SUDACHI_STORAGE").as_deref() {
            Ok("mapped") => {
                let file = std::fs::File::open(path).unwrap();
                // SAFETY: the explicit test runner requires an immutable prepared
                // dictionary throughout this process. No fixture modifies it.
                unsafe { Analyzer::open_full_mapped(&file) }
            }
            Ok("owned") | Err(_) => Analyzer::open_full(path),
            Ok(other) => panic!("unknown YOMOKU_SUDACHI_STORAGE: {other}"),
        }
        .expect("prepare the pinned Full dictionary first")
    })
}

fn chapter(text: &str) -> ChapterText {
    ChapterText {
        source_id: 17.try_into().unwrap(),
        chapter_id: 29.try_into().unwrap(),
        text: text.into(),
    }
}

fn analyze(text: &str, mode: SplitMode) -> ChapterAnalysis {
    let chapter = chapter(text);
    let result = analyzer().analyze_mode(&chapter, mode).unwrap();
    result.validate(&chapter).unwrap();
    assert_eq!(result.profile.engine_revision, ENGINE_REVISION);
    assert_eq!(result.profile.dictionary_revision, DICTIONARY_REVISION);
    assert_eq!(result.mode, mode);
    for token in &result.tokens {
        assert_eq!(token.anchor.slice(&chapter).unwrap(), token.surface);
    }
    result
}

fn surfaces(result: &ChapterAnalysis) -> Vec<&str> {
    result
        .tokens
        .iter()
        .map(|token| token.surface.as_str())
        .collect()
}

#[test]
#[ignore = "requires pinned Sudachi Full; run tools/test-sudachi.sh"]
fn mapped_and_owned_outputs_are_identical_including_profiles_and_anchors() {
    let path = std::env::var_os("YOMOKU_SUDACHI_DICTIONARY")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../.local/sudachi/full-20261015/system_full.dic")
        });
    let owned = Analyzer::open_full(&path).unwrap();
    let file = std::fs::File::open(path).unwrap();
    // SAFETY: the explicit test runner keeps the prepared file immutable for
    // this process. Closing the read handle below does not unmap its storage.
    let mapped = unsafe { Analyzer::open_full_mapped(&file) }.unwrap();
    drop(file);
    for text in [
        "国家公務員。東京都と夏目漱石。食べる。食べた。".into(),
        "English 123 📖 日本語。𰻞".into(),
        "ｶﾞｯﾂﾎﾟｰｽﾞ ＡＢＣと㍿ か\u{3099}くせい e\u{301}と𠮷📖\0猫".into(),
        "\r\n猫。\r\n　犬。\tモーニング娘。が好き。".into(),
        "".into(),
        "猫と犬".repeat(8000),
    ] {
        let input = chapter(&text);
        for mode in [SplitMode::A, SplitMode::B, SplitMode::C] {
            let expected = owned.analyze_mode(&input, mode).unwrap();
            let actual = mapped.analyze_mode(&input, mode).unwrap();
            expected.validate(&input).unwrap();
            actual.validate(&input).unwrap();
            assert_eq!(actual, expected);
        }
    }
}

#[test]
#[ignore = "requires pinned Sudachi Full; run tools/test-sudachi.sh"]
fn default_b_and_all_three_real_split_modes() {
    let text = "国家公務員";
    let a = analyze(text, SplitMode::A);
    let b = analyze(text, SplitMode::B);
    let c = analyze(text, SplitMode::C);
    assert_eq!(surfaces(&a), ["国家", "公務", "員"]);
    assert_eq!(surfaces(&b), ["国家", "公務員"]);
    assert_eq!(surfaces(&c), ["国家公務員"]);
    assert_eq!(analyzer().analyze(&chapter(text)).unwrap(), b);
}

#[test]
#[ignore = "requires pinned Sudachi Full; run tools/test-sudachi.sh"]
fn real_segmentation_choices_resolve_scopes_merges_and_reversal_without_replacing_b() {
    use yomoku_core::{
        analysis::{OverrideScope, SegmentationOverride},
        document::{PasteText, TextAnchor, TextOffset, TextRange},
        segmentation::segmentation_from_snapshot,
        vocabulary::Surface,
    };
    use yomoku_storage::Storage;

    let directory = std::env::temp_dir().join(format!(
        "yomoku-real-segmentation-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir(&directory).unwrap();
    let path = directory.join("segmentation.sqlite3");
    let mut storage = Storage::open(&path).unwrap();
    let imported = storage
        .create_analyzed_paste(
            &PasteText::new("国家公務員。国家公務員。食べた。English 𰻞📖\0".into()).unwrap(),
            100,
            |chapter| analyzer().analyze(chapter),
        )
        .unwrap();
    let chapter = &imported.document.chapter;
    let original = storage
        .chapter_analysis(imported.analysis_id)
        .unwrap()
        .unwrap();
    let selected = TextAnchor {
        source_id: chapter.source_id,
        chapter_id: chapter.chapter_id,
        range: TextRange {
            start: TextOffset::START,
            end: TextOffset::at(&chapter.text, "国家公務員".len()).unwrap(),
        },
    };
    let mut saved = Vec::new();
    for mode in [SplitMode::A, SplitMode::B, SplitMode::C] {
        let snapshot = analyzer().analyze_mode(chapter, mode).unwrap();
        let id = storage
            .store_chapter_analysis(chapter.chapter_id, &snapshot)
            .unwrap();
        saved.push((id, snapshot.clone()));
        let edit = segmentation_from_snapshot(
            chapter,
            &original,
            &snapshot,
            selected,
            OverrideScope::default(),
        )
        .unwrap();
        let override_id = storage.save_segmentation_override(&edit).unwrap();
        let resolved = storage
            .load_chapter_segmentation(chapter.chapter_id)
            .unwrap()
            .unwrap();
        assert_eq!(resolved.original_analysis_id, imported.analysis_id);
        let changed = resolved
            .segments
            .iter()
            .filter(|s| s.override_id == Some(override_id))
            .collect::<Vec<_>>();
        assert_eq!(changed.len(), edit.parts.len() * 2);
        for (index, segment) in changed.iter().enumerate() {
            assert_eq!(
                segment.surface,
                edit.parts[index % edit.parts.len()].as_str()
            );
            assert_eq!(segment.anchor.slice(chapter).unwrap(), segment.surface);
        }
        assert_eq!(
            storage
                .chapter_analysis(imported.analysis_id)
                .unwrap()
                .unwrap(),
            original
        );
        storage.reverse_segmentation_override(override_id).unwrap();
    }
    // Merge the real inflected B sequence 食べ + た into a selectable surface.
    let start = chapter.text.find("食べた").unwrap();
    let merged = SegmentationOverride {
        scope: OverrideScope::AllOccurrences,
        anchor: TextAnchor {
            range: TextRange {
                start: TextOffset::at(&chapter.text, start).unwrap(),
                end: TextOffset::at(&chapter.text, start + "食べた".len()).unwrap(),
            },
            ..selected
        },
        surface: Surface::new("食べた").unwrap(),
        parts: vec![Surface::new("食べた").unwrap()],
    };
    let global_id = storage.save_segmentation_override(&merged).unwrap();
    let local = segmentation_from_snapshot(
        chapter,
        &original,
        &original,
        merged.anchor,
        OverrideScope::ThisOccurrence,
    )
    .unwrap();
    let local_id = storage.save_segmentation_override(&local).unwrap();
    let resolved = storage
        .load_chapter_segmentation(chapter.chapter_id)
        .unwrap()
        .unwrap();
    assert_eq!(
        resolved
            .segments
            .iter()
            .filter(|s| s.override_id == Some(local_id))
            .map(|s| s.surface.as_str())
            .collect::<Vec<_>>(),
        ["食べ", "た"]
    );
    storage.reverse_segmentation_override(local_id).unwrap();
    let expected = storage
        .load_chapter_segmentation(chapter.chapter_id)
        .unwrap()
        .unwrap();
    assert!(expected.segments.iter().any(|s| s.surface == "食べた"
        && s.override_id == Some(global_id)
        && s.original_token_index.is_none()));
    drop(storage);
    let mut reopened = Storage::open(&path).unwrap();
    assert_eq!(
        reopened
            .load_chapter_segmentation(chapter.chapter_id)
            .unwrap()
            .unwrap(),
        expected
    );
    for (id, snapshot) in saved {
        assert_eq!(reopened.chapter_analysis(id).unwrap().unwrap(), snapshot);
    }
    reopened.reverse_segmentation_override(global_id).unwrap();
    let restored = reopened
        .load_chapter_segmentation(chapter.chapter_id)
        .unwrap()
        .unwrap();
    assert!(restored.segments.iter().all(|s| s.override_id.is_none()));
    assert_eq!(
        restored
            .segments
            .iter()
            .map(|s| s.surface.as_str())
            .collect::<Vec<_>>(),
        surfaces(&original)
    );
    assert_eq!(
        reopened
            .chapter_analysis(imported.analysis_id)
            .unwrap()
            .unwrap(),
        original
    );
    drop(reopened);
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
#[ignore = "requires pinned Sudachi Full; run tools/test-sudachi.sh"]
fn inflected_surfaces_stay_distinct_from_analyzer_forms() {
    let result = analyze("食べる。食べた。食べます。", SplitMode::B);
    assert_eq!(
        surfaces(&result),
        ["食べる", "。", "食べ", "た", "。", "食べ", "ます", "。"]
    );
    let verb = &result.tokens[2];
    assert!(verb.is_japanese);
    assert_eq!(verb.dictionary_form.as_ref().unwrap().as_str(), "食べる");
    assert_eq!(verb.reading.as_deref(), Some("タベ"));
    // Raw analyzer boundaries are retained; merging conjugations is separate work.
    assert_ne!(
        verb.surface,
        verb.dictionary_form.as_ref().unwrap().as_str()
    );
}

#[test]
#[ignore = "requires pinned Sudachi Full; run tools/test-sudachi.sh"]
fn full_dictionary_kanji_only_proper_nouns_are_words() {
    let result = analyze("東京都と夏目漱石。", SplitMode::B);
    assert_eq!(surfaces(&result), ["東京都", "と", "夏目", "漱石", "。"]);
    for token in &result.tokens[..4] {
        assert!(token.is_japanese);
        assert!(token.dictionary_form.is_some());
        assert!(token.reading.is_some());
    }
    assert!(!result.tokens.last().unwrap().is_japanese);
}

#[test]
#[ignore = "requires pinned Sudachi Full; run tools/test-sudachi.sh"]
fn mixed_scripts_punctuation_and_unknown_kanji() {
    let result = analyze("English 123 📖 日本語。𰻞", SplitMode::B);
    for token in &result.tokens {
        if ["English", "123", "📖", "。"].contains(&token.surface.as_str()) {
            assert!(!token.is_japanese, "{}", token.surface);
            assert!(token.dictionary_form.is_none());
        }
    }
    let unknown = result.tokens.last().unwrap();
    assert_eq!(unknown.surface, "𰻞");
    assert!(unknown.is_japanese);
    assert!(unknown.dictionary_form.is_none());
    assert!(unknown.reading.is_none());
}

#[test]
#[ignore = "requires pinned Sudachi Full; run tools/test-sudachi.sh"]
fn normalization_keeps_original_byte_surfaces_in_every_mode() {
    for mode in [SplitMode::A, SplitMode::B, SplitMode::C] {
        for text in [
            "ｶﾞｯﾂﾎﾟｰｽﾞ",
            "ＡＢＣと㍿",
            "か\u{3099}くせい",
            "e\u{301}と𠮷📖\0猫",
        ] {
            let result = analyze(text, mode);
            assert_eq!(
                result
                    .tokens
                    .iter()
                    .map(|token| token.surface.as_str())
                    .collect::<String>(),
                text
            );
        }
    }
}

#[test]
#[ignore = "requires pinned Sudachi Full; run tools/test-sudachi.sh"]
fn whitespace_empty_input_and_repeated_analysis() {
    for mode in [SplitMode::A, SplitMode::B, SplitMode::C] {
        assert!(analyze("", mode).tokens.is_empty());
        assert!(analyze(" \t\r\n　", mode).tokens.is_empty());
        let text = "\r\n猫。\r\n　犬。\t";
        let first = analyze(text, mode);
        assert_eq!(first, analyze(text, mode));
        assert_eq!(surfaces(&first), ["猫", "。", "犬", "。"]);
    }
}

#[test]
#[ignore = "requires pinned Sudachi Full; run tools/test-sudachi.sh"]
fn long_unbroken_chapter_exceeds_the_engine_input_limit() {
    let text = "猫と犬".repeat(8000);
    assert!(text.len() > 65_535);
    let result = analyze(&text, SplitMode::B);
    assert_eq!(
        result
            .tokens
            .iter()
            .map(|token| token.surface.as_str())
            .collect::<String>(),
        text
    );
    assert_eq!(
        i64::from(result.tokens.last().unwrap().anchor.range.end),
        text.len() as i64
    );
}

#[test]
#[ignore = "requires pinned Sudachi Full; run tools/test-sudachi.sh"]
fn sentence_boundaries_preserve_dictionary_names_ending_in_punctuation() {
    let result = analyze("モーニング娘。が好き。猫。犬。", SplitMode::C);
    assert_eq!(result.tokens[0].surface, "モーニング娘。");
    assert!(result.tokens[0].is_japanese);
    assert!(result.tokens.iter().any(|token| token.surface == "猫"));
    assert!(result.tokens.iter().any(|token| token.surface == "犬"));
}

#[test]
#[ignore = "requires pinned Sudachi Full; run tools/test-sudachi.sh"]
fn real_snapshots_persist_all_modes_and_reopen_without_source_changes() {
    use yomoku_core::document::PasteText;
    use yomoku_storage::Storage;

    let directory = std::env::temp_dir().join(format!(
        "yomoku-real-analysis-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir(&directory).unwrap();
    let path = directory.join("analysis.sqlite3");
    let mut storage = Storage::open(&path).unwrap();
    let text = "国家公務員。\r\nｶﾞｯﾂﾎﾟｰｽﾞと𰻞📖\0猫";
    let document = storage
        .create_paste(&PasteText::new(text.into()).unwrap())
        .unwrap();
    let mut saved = Vec::new();
    for mode in [SplitMode::A, SplitMode::B, SplitMode::C] {
        let result = analyzer().analyze_mode(&document.chapter, mode).unwrap();
        let id = storage
            .store_chapter_analysis(document.chapter.chapter_id, &result)
            .unwrap();
        assert_eq!(
            storage
                .store_chapter_analysis(document.chapter.chapter_id, &result)
                .unwrap(),
            id
        );
        saved.push((id, result));
    }
    drop(storage);
    let mut reopened = Storage::open(&path).unwrap();
    for (id, expected) in saved {
        assert_eq!(reopened.chapter_analysis(id).unwrap().unwrap(), expected);
    }
    assert_eq!(
        reopened
            .load_paste(document.document_id)
            .unwrap()
            .unwrap()
            .chapter
            .text,
        text
    );
    drop(reopened);
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
#[ignore = "requires pinned Sudachi Full; run tools/test-sudachi.sh"]
fn real_mode_b_import_and_completion_preserve_surfaces_and_duplicate_identity() {
    use std::collections::BTreeSet;
    use yomoku_analysis::AnalysisError;
    use yomoku_core::{
        completion::{AnalyzedCompletionOutcome, PageCompletion},
        document::PasteText,
        pagination::{LogicalPages, PageLength, ReadingPosition},
        reader::ReaderAnalysis,
        vocabulary::{Surface, VocabularyStatus},
    };
    use yomoku_storage::{PasteImportError, PasteImportOutcome, Storage};

    let directory = std::env::temp_dir().join(format!(
        "yomoku-real-import-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir(&directory).unwrap();
    let path = directory.join("import.sqlite3");
    let mut storage = Storage::open(&path).unwrap();
    let cat = Surface::new("猫").unwrap();
    storage
        .register_vocabulary(std::slice::from_ref(&cat), 10)
        .unwrap();
    let entry = storage.vocabulary_entry(&cat).unwrap().unwrap();
    let known = storage
        .set_vocabulary_status(entry.id, VocabularyStatus::Known, 20)
        .unwrap()
        .unwrap();
    let text =
        PasteText::new("国家公務員。猫と猫が食べた。\r\nｶﾞｯﾂﾎﾟｰｽﾞと𰻞 English 123 📖\0猫".into())
            .unwrap();
    assert!(matches!(
        storage.import_analyzed_paste(&text, 30, |_| Err(AnalysisError::InvalidOutput)),
        Err(PasteImportError::Analysis(AnalysisError::InvalidOutput))
    ));
    assert!(storage.list_documents().unwrap().is_empty());
    // Reuse the preloaded analyzer; no dictionary load occurs inside the transaction.
    let engine = analyzer();
    let PasteImportOutcome::Created(imported) = storage
        .import_analyzed_paste(&text, 30, |chapter| engine.analyze(chapter))
        .unwrap()
    else {
        panic!("expected a new import")
    };
    let expected = engine.analyze(&imported.document.chapter).unwrap();
    assert_eq!(expected.mode, SplitMode::B);
    let surfaces: BTreeSet<_> = expected
        .tokens
        .iter()
        .filter(|token| token.is_japanese)
        .map(|token| Surface::new(&token.surface).unwrap())
        .collect();
    assert_eq!(imported.registered_surfaces, surfaces.len() - 1);
    assert!(surfaces.contains(&Surface::new("𰻞").unwrap()));
    let halfwidth_start = text.as_str().find("ｶﾞｯﾂﾎﾟｰｽﾞ").unwrap();
    let halfwidth_end = halfwidth_start + "ｶﾞｯﾂﾎﾟｰｽﾞ".len();
    let halfwidth_tokens: Vec<_> = expected
        .tokens
        .iter()
        .filter(|token| {
            let start = usize::try_from(i64::from(token.anchor.range.start)).unwrap();
            (halfwidth_start..halfwidth_end).contains(&start)
        })
        .collect();
    assert!(halfwidth_tokens.iter().all(|token| token.is_japanese));
    assert_eq!(
        halfwidth_tokens
            .iter()
            .map(|token| token.surface.as_str())
            .collect::<String>(),
        "ｶﾞｯﾂﾎﾟｰｽﾞ"
    );
    assert!(
        expected
            .tokens
            .iter()
            .filter(|token| token.surface == "猫")
            .count()
            >= 3
    );
    drop(storage);
    let mut storage = Storage::open(&path).unwrap();
    let duplicate = storage
        .import_analyzed_paste(&text, 40, |_| -> Result<_, AnalysisError> {
            panic!("duplicate must not tokenize again")
        })
        .unwrap();
    let PasteImportOutcome::Existing(duplicate) = duplicate else {
        panic!("expected an existing document")
    };
    assert_eq!(duplicate, imported.document);
    assert_eq!(storage.list_documents().unwrap().len(), 1);
    assert_eq!(
        storage.load_paste(imported.document.document_id).unwrap(),
        Some(imported.document.clone())
    );
    assert_eq!(
        storage.chapter_analysis(imported.analysis_id).unwrap(),
        Some(expected.clone())
    );
    assert_eq!(storage.vocabulary_entry(&cat).unwrap(), Some(known));
    for surface in surfaces.iter().filter(|surface| **surface != cat) {
        let entry = storage.vocabulary_entry(surface).unwrap().unwrap();
        assert_eq!(entry.status, VocabularyStatus::New);
        assert_eq!(entry.registered_at_unix_ms, 30);
    }
    for surface in ["English", "123", "。", "📖", "\0"] {
        assert!(storage
            .vocabulary_entry(&Surface::new(surface).unwrap())
            .unwrap()
            .is_none());
    }
    let page_length = PageLength::default();
    let layout = LogicalPages::new(
        std::slice::from_ref(&imported.document.chapter),
        page_length,
    )
    .unwrap();
    assert_eq!(layout.pages().len(), 1);
    let page = layout.pages()[0];
    // Use the persisted original output and current global status, including
    // halfwidth/rare-kanji forms, without changing import state.
    let before_lookup = std::fs::read(&path).unwrap();
    let lookup = storage
        .load_reader_tokens(imported.document.document_id, page)
        .unwrap()
        .unwrap();
    assert_eq!(
        lookup.analysis,
        ReaderAnalysis::Available(imported.analysis_id)
    );
    let japanese: Vec<_> = expected
        .tokens
        .iter()
        .enumerate()
        .filter(|(_, token)| token.is_japanese)
        .collect();
    assert_eq!(lookup.tokens.len(), japanese.len());
    for (actual, (index, token)) in lookup.tokens.iter().zip(japanese) {
        assert_eq!(actual.token_index, index as u64);
        assert_eq!(actual.anchor, token.anchor);
        assert_eq!(actual.surface, token.surface);
        assert_eq!(actual.reading, token.reading);
        assert_eq!(
            actual.dictionary_form.as_deref(),
            token.dictionary_form.as_ref().map(Surface::as_str)
        );
        assert_eq!(
            actual.vocabulary.unwrap().status,
            if token.surface == "猫" {
                VocabularyStatus::Known
            } else {
                VocabularyStatus::New
            }
        );
    }
    assert_eq!(std::fs::read(&path).unwrap(), before_lookup);
    let departing_position = ReadingPosition {
        source_id: page.source_id,
        chapter_id: page.chapter_id,
        offset: page.range.start,
    };
    let completion = PageCompletion {
        command_id: storage.new_completion_command_id().unwrap(),
        document_id: imported.document.document_id,
        page,
        page_length,
        departing_position,
        completed_at_unix_ms: 50,
        pending_time: Default::default(),
    };
    assert_eq!(
        storage
            .record_analyzed_page_completion(completion, imported.analysis_id)
            .unwrap(),
        AnalyzedCompletionOutcome::Recorded {
            newly_known: (surfaces.len() - 1) as u64
        }
    );
    assert_eq!(
        storage
            .record_analyzed_page_completion(completion, imported.analysis_id)
            .unwrap(),
        AnalyzedCompletionOutcome::AlreadyRecorded
    );
    for surface in &surfaces {
        assert_eq!(
            storage.vocabulary_entry(surface).unwrap().unwrap().status,
            VocabularyStatus::Known
        );
    }
    let completed_document = storage
        .load_paste(imported.document.document_id)
        .unwrap()
        .unwrap();
    let before_lookup = std::fs::read(&path).unwrap();
    let lookup = storage
        .load_reader_tokens(imported.document.document_id, page)
        .unwrap()
        .unwrap();
    assert!(lookup
        .tokens
        .iter()
        .all(|token| token.vocabulary.unwrap().status == VocabularyStatus::Known));
    // EOF lookup preserves the marker/position. Only explicit OpenReader resets.
    assert_eq!(
        storage.load_paste(imported.document.document_id).unwrap(),
        Some(completed_document)
    );
    assert_eq!(std::fs::read(&path).unwrap(), before_lookup);
    storage
        .save_reading_position(imported.document.document_id, departing_position)
        .unwrap();
    let reread = PageCompletion {
        command_id: storage.new_completion_command_id().unwrap(),
        completed_at_unix_ms: 60,
        ..completion
    };
    assert_eq!(
        storage
            .record_analyzed_page_completion(reread, imported.analysis_id)
            .unwrap(),
        AnalyzedCompletionOutcome::Recorded { newly_known: 0 }
    );
    drop(storage);
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
#[ignore = "requires pinned Sudachi Full; run tools/test-sudachi.sh"]
fn real_txt_import_analyzes_only_canonical_text_and_completes_with_original_anchors() {
    use std::collections::BTreeSet;
    use yomoku_core::{
        completion::{AnalyzedCompletionOutcome, PageCompletion},
        pagination::{LogicalPages, PageLength, ReadingPosition},
        reader::ReaderAnalysis,
        vocabulary::{Surface, VocabularyStatus},
    };
    use yomoku_storage::{Storage, TxtImportOutcome};

    let directory = std::env::temp_dir().join(format!(
        "yomoku-real-txt-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir(&directory).unwrap();
    let path = directory.join("txt.sqlite3");
    let mut storage = Storage::open(&path).unwrap();
    let engine = analyzer();
    let bytes = "\u{feff}｜猫《ねこ》と猫が食べた。\r\n［＃大見出し］第一章［＃大見出し終わり］\n国家公務員。𠮷 English 📖\0".as_bytes();
    let canonical = "猫と猫が食べた。\r\n第一章\n国家公務員。𠮷 English 📖\0";
    let TxtImportOutcome::Created(imported) = storage
        .import_analyzed_txt(bytes, "吾輩.txt", None, 100, |chapter| {
            assert_eq!(chapter.text, canonical);
            engine.analyze(chapter)
        })
        .unwrap()
    else {
        panic!("expected new TXT")
    };
    let document = &imported.document;
    let expected = engine.analyze(&document.chapter).unwrap();
    assert_eq!(expected.mode, SplitMode::B);
    assert_eq!(
        storage.chapter_analysis(imported.analysis_id).unwrap(),
        Some(expected.clone())
    );
    let unique: BTreeSet<_> = expected
        .tokens
        .iter()
        .filter(|token| token.is_japanese)
        .map(|token| Surface::new(&token.surface).unwrap())
        .collect();
    assert_eq!(imported.registered_surfaces, unique.len());
    let source = storage
        .load_txt_source(document.document_id)
        .unwrap()
        .unwrap();
    assert_eq!(source.original_bytes, bytes);
    assert_eq!(source.ruby.len(), 1);
    assert_eq!(&canonical[source.ruby[0].range.clone()], "猫");
    assert_eq!(source.ruby[0].reading, "ねこ");
    for surface in &unique {
        assert_eq!(
            storage.vocabulary_entry(surface).unwrap().unwrap().status,
            VocabularyStatus::New
        );
    }
    let page_length = PageLength::default();
    let pages = LogicalPages::new(std::slice::from_ref(&document.chapter), page_length).unwrap();
    let page = pages.pages()[0];
    let tokens = storage
        .load_reader_tokens(document.document_id, page)
        .unwrap()
        .unwrap();
    assert_eq!(
        tokens.analysis,
        ReaderAnalysis::Available(imported.analysis_id)
    );
    assert_eq!(
        tokens
            .tokens
            .iter()
            .filter(|token| token.surface == "猫")
            .count(),
        2
    );
    let completion = PageCompletion {
        command_id: storage.new_completion_command_id().unwrap(),
        document_id: document.document_id,
        page,
        page_length,
        departing_position: ReadingPosition {
            source_id: page.source_id,
            chapter_id: page.chapter_id,
            offset: page.range.start,
        },
        completed_at_unix_ms: 200,
        pending_time: Default::default(),
    };
    assert_eq!(
        storage
            .record_analyzed_page_completion(completion, imported.analysis_id)
            .unwrap(),
        AnalyzedCompletionOutcome::Recorded {
            newly_known: unique.len() as u64
        }
    );
    let completed = storage.load_paste(document.document_id).unwrap().unwrap();
    assert!(completed.finished_at_least_once);
    assert_eq!(storage.list_documents().unwrap()[0].progress_percent, 100);
    drop(storage);
    let mut storage = Storage::open(&path).unwrap();
    assert_eq!(
        storage.load_txt_source(document.document_id).unwrap(),
        Some(source)
    );
    let TxtImportOutcome::Existing(existing) = storage
        .import_analyzed_txt(
            bytes,
            "renamed.txt",
            None,
            300,
            |_| -> Result<ChapterAnalysis, yomoku_analysis::AnalysisError> {
                panic!("duplicate must not analyze")
            },
        )
        .unwrap()
    else {
        panic!("expected existing TXT")
    };
    assert_eq!(existing, completed.library_summary().unwrap());
    assert_eq!(
        storage
            .record_analyzed_page_completion(completion, imported.analysis_id)
            .unwrap(),
        AnalyzedCompletionOutcome::AlreadyRecorded
    );
    assert_eq!(
        storage
            .open_reader_snapshot(document.document_id)
            .unwrap()
            .unwrap()
            .document
            .reading_position
            .unwrap()
            .offset,
        page.range.start
    );
    drop(storage);
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
#[ignore = "requires pinned Sudachi Full; run tools/test-sudachi.sh"]
fn real_srt_import_analyzes_only_canonical_text_and_completes_with_original_anchors() {
    use std::collections::BTreeSet;
    use yomoku_core::{
        completion::{AnalyzedCompletionOutcome, PageCompletion},
        pagination::{LogicalPages, PageLength, ReadingPosition},
        reader::ReaderAnalysis,
        vocabulary::{Surface, VocabularyStatus},
    };
    use yomoku_storage::{SrtImportOutcome, Storage};

    let directory = std::env::temp_dir().join(format!(
        "yomoku-real-srt-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir(&directory).unwrap();
    let path = directory.join("srt.sqlite3");
    let mut storage = Storage::open(&path).unwrap();
    let engine = analyzer();
    let bytes = "\u{feff}1\r\n00:00:01,000 --> 00:00:02,000\r\n<i>猫と猫が食べた。</i>\r\n\r\n2\r\n00:00:03,000 --> 00:00:04,000\r\n国家公務員。𠮷 English 📖\0".as_bytes();
    let canonical = "猫と猫が食べた。\n\n国家公務員。𠮷 English 📖\0";
    let SrtImportOutcome::Created(imported) = storage
        .import_analyzed_srt(bytes, "吾輩.srt", None, 100, |chapter| {
            assert_eq!(chapter.text, canonical);
            engine.analyze(chapter)
        })
        .unwrap()
    else {
        panic!("expected new SRT")
    };
    let document = &imported.document;
    let expected = engine.analyze(&document.chapter).unwrap();
    assert_eq!(expected.mode, SplitMode::B);
    assert_eq!(
        storage.chapter_analysis(imported.analysis_id).unwrap(),
        Some(expected.clone())
    );
    let unique: BTreeSet<_> = expected
        .tokens
        .iter()
        .filter(|token| token.is_japanese)
        .map(|token| Surface::new(&token.surface).unwrap())
        .collect();
    assert_eq!(imported.registered_surfaces, unique.len());
    let source = storage
        .load_srt_source(document.document_id)
        .unwrap()
        .unwrap();
    assert_eq!(source.original_bytes, bytes);
    assert_eq!(source.cues.len(), 2);
    assert_eq!(&canonical[source.cues[0].clone()], "猫と猫が食べた。");
    assert_eq!(
        &canonical[source.cues[1].clone()],
        "国家公務員。𠮷 English 📖\0"
    );
    for surface in &unique {
        assert_eq!(
            storage.vocabulary_entry(surface).unwrap().unwrap().status,
            VocabularyStatus::New
        );
    }
    let page_length = PageLength::default();
    let pages = LogicalPages::new(std::slice::from_ref(&document.chapter), page_length).unwrap();
    let page = pages.pages()[0];
    let tokens = storage
        .load_reader_tokens(document.document_id, page)
        .unwrap()
        .unwrap();
    assert_eq!(
        tokens.analysis,
        ReaderAnalysis::Available(imported.analysis_id)
    );
    assert_eq!(
        tokens
            .tokens
            .iter()
            .filter(|token| token.surface == "猫")
            .count(),
        2
    );
    let completion = PageCompletion {
        command_id: storage.new_completion_command_id().unwrap(),
        document_id: document.document_id,
        page,
        page_length,
        departing_position: ReadingPosition {
            source_id: page.source_id,
            chapter_id: page.chapter_id,
            offset: page.range.start,
        },
        completed_at_unix_ms: 200,
        pending_time: Default::default(),
    };
    assert_eq!(
        storage
            .record_analyzed_page_completion(completion, imported.analysis_id)
            .unwrap(),
        AnalyzedCompletionOutcome::Recorded {
            newly_known: unique.len() as u64
        }
    );
    let completed = storage.load_paste(document.document_id).unwrap().unwrap();
    assert!(completed.finished_at_least_once);
    assert_eq!(storage.list_documents().unwrap()[0].progress_percent, 100);
    drop(storage);
    let mut storage = Storage::open(&path).unwrap();
    assert_eq!(
        storage.load_srt_source(document.document_id).unwrap(),
        Some(source)
    );
    let SrtImportOutcome::Existing(existing) = storage
        .import_analyzed_srt(
            bytes,
            "renamed.srt",
            None,
            300,
            |_| -> Result<ChapterAnalysis, yomoku_analysis::AnalysisError> {
                panic!("duplicate must not analyze")
            },
        )
        .unwrap()
    else {
        panic!("expected existing SRT")
    };
    assert_eq!(existing, completed.library_summary().unwrap());
    assert_eq!(
        storage
            .record_analyzed_page_completion(completion, imported.analysis_id)
            .unwrap(),
        AnalyzedCompletionOutcome::AlreadyRecorded
    );
    assert_eq!(
        storage
            .open_reader_snapshot(document.document_id)
            .unwrap()
            .unwrap()
            .document
            .reading_position
            .unwrap()
            .offset,
        page.range.start
    );
    drop(storage);
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
#[ignore = "requires pinned Sudachi Full; run tools/test-sudachi.sh"]
fn real_epub_import_analyzes_every_canonical_chapter_and_reopens_without_rescan() {
    use std::collections::BTreeSet;
    use yomoku_core::{
        pagination::{LogicalPages, PageLength},
        vocabulary::{Surface, VocabularyStatus},
    };
    use yomoku_storage::{EpubImportOutcome, Storage};

    let directory = std::env::temp_dir().join(format!(
        "yomoku-real-epub-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir(&directory).unwrap();
    let path = directory.join("epub.sqlite3");
    let mut storage = Storage::open(&path).unwrap();
    let cat = Surface::new("猫").unwrap();
    storage
        .register_vocabulary(std::slice::from_ref(&cat), 1)
        .unwrap();
    let entry = storage.vocabulary_entry(&cat).unwrap().unwrap();
    storage
        .set_vocabulary_status(entry.id, VocabularyStatus::Learning, 2)
        .unwrap();
    let bytes = epub_fixture::book(
        true,
        "<p>猫と猫が食べた。</p><p><ruby>東京<rt>ソースルビ</rt></ruby> 𠮷 English 📖</p>",
    );
    let engine = analyzer();
    let mut analyses = Vec::new();
    let EpubImportOutcome::Created(imported) = storage
        .import_analyzed_epub(&bytes, "本.epub", 100, |chapter| {
            assert!(!chapter.text.contains("ソースルビ"));
            let analysis = engine.analyze(chapter)?;
            analyses.push(analysis.clone());
            Ok::<_, yomoku_analysis::AnalysisError>(analysis)
        })
        .unwrap()
    else {
        panic!("expected new EPUB")
    };
    assert_eq!(analyses.len(), 2);
    let mut unique = BTreeSet::new();
    for ((chapter, id), expected) in imported
        .document
        .chapters
        .iter()
        .zip(&imported.analysis_ids)
        .zip(&analyses)
    {
        expected.validate(&chapter.chapter).unwrap();
        assert_eq!(expected.mode, SplitMode::B);
        assert_eq!(
            storage.chapter_analysis(*id).unwrap(),
            Some(expected.clone())
        );
        unique.extend(
            expected
                .tokens
                .iter()
                .filter(|token| token.is_japanese)
                .map(|token| Surface::new(&token.surface).unwrap()),
        );
    }
    assert!(unique.contains(&cat));
    assert_eq!(imported.registered_surfaces, unique.len() - 1);
    for surface in &unique {
        assert_eq!(
            storage.vocabulary_entry(surface).unwrap().unwrap().status,
            if surface == &cat {
                VocabularyStatus::Learning
            } else {
                VocabularyStatus::New
            }
        );
    }
    assert!(storage
        .vocabulary_entry(&Surface::new("ソースルビ").unwrap())
        .unwrap()
        .is_none());
    let chapters: Vec<_> = imported
        .document
        .chapters
        .iter()
        .map(|chapter| chapter.chapter.clone())
        .collect();
    for preset in PageLength::ALL {
        assert_eq!(
            LogicalPages::new(&chapters, preset).unwrap().pages().len(),
            2
        );
    }
    assert_eq!(storage.list_documents().unwrap()[0].progress_percent, 0);
    drop(storage);
    let mut storage = Storage::open(&path).unwrap();
    assert_eq!(
        storage
            .load_epub_document(imported.document.document_id)
            .unwrap(),
        Some(imported.document.clone())
    );
    assert!(
        matches!(storage.import_analyzed_epub(&bytes, "renamed.epub", 200, |_| -> Result<ChapterAnalysis, yomoku_analysis::AnalysisError> { panic!("duplicate must not analyze") }).unwrap(), EpubImportOutcome::Existing(id) if id.document_id == imported.document.document_id)
    );
    for (id, expected) in imported.analysis_ids.iter().zip(&analyses) {
        assert_eq!(
            storage.chapter_analysis(*id).unwrap().as_ref(),
            Some(expected)
        );
    }
    // Exercise the same original real Mode B chapters through the generic reader
    // repositories, without parsing again or keeping an analyzer in the reader.
    use yomoku_core::{
        active_time::PendingReadingTime,
        completion::{AnalyzedCompletionOutcome, PageCompletionIntent},
        reader::{ReaderAnalysis, ReaderPage},
    };
    let id = imported.document.document_id;
    for (index, expected) in analyses.iter().enumerate() {
        let snapshot = storage.load_document_reader_snapshot(id).unwrap().unwrap();
        let page = ReaderPage::open_document(&snapshot.document, snapshot.page_length).unwrap();
        assert_eq!(page.page_number, index as u64 + 1);
        assert_eq!(page.total_pages, 2);
        let tokens = storage
            .load_reader_tokens(id, page.anchor)
            .unwrap()
            .unwrap();
        assert_eq!(
            tokens.analysis,
            ReaderAnalysis::Available(imported.analysis_ids[index])
        );
        assert_eq!(
            tokens
                .tokens
                .iter()
                .map(|token| token.surface.as_str())
                .collect::<Vec<_>>(),
            expected
                .tokens
                .iter()
                .filter(|token| token.is_japanese)
                .map(|token| token.surface.as_str())
                .collect::<Vec<_>>()
        );
        let saved = storage
            .load_pending_reading_time(id, page.position)
            .unwrap();
        storage
            .save_pending_reading_time(saved, PendingReadingTime::from_millis(75).unwrap())
            .unwrap();
        let command = storage
            .prepare_page_completion(PageCompletionIntent {
                document_id: id,
                page: page.anchor,
                page_length: snapshot.page_length,
                departing_position: page.position,
                completed_at_unix_ms: 300,
                analysis_id: imported.analysis_ids[index],
            })
            .unwrap();
        assert_eq!(command.completion.pending_time.milliseconds(), 75);
        assert!(matches!(
            storage
                .record_analyzed_page_completion(command.completion, command.analysis_id)
                .unwrap(),
            AnalyzedCompletionOutcome::Recorded { .. }
        ));
        assert_eq!(
            storage.finished_at_least_once(id).unwrap(),
            Some(index == 1)
        );
        let after = storage.load_document_reader_snapshot(id).unwrap();
        assert_eq!(
            storage
                .record_analyzed_page_completion(command.completion, command.analysis_id)
                .unwrap(),
            AnalyzedCompletionOutcome::AlreadyRecorded
        );
        assert_eq!(storage.load_document_reader_snapshot(id).unwrap(), after);
    }
    assert_eq!(storage.list_documents().unwrap()[0].progress_percent, 100);
    assert_eq!(
        storage.vocabulary_entry(&cat).unwrap().unwrap().status,
        VocabularyStatus::Learning
    );
    drop(storage);
    let mut storage = Storage::open(&path).unwrap();
    let opened = storage.open_document_reader_snapshot(id).unwrap().unwrap();
    assert_eq!(
        ReaderPage::open_document(&opened.document, opened.page_length)
            .unwrap()
            .page_number,
        1
    );
    assert!(opened.document.finished_at_least_once);
    assert_eq!(storage.list_documents().unwrap()[0].progress_percent, 0);
    drop(storage);
    std::fs::remove_dir_all(directory).unwrap();
}
