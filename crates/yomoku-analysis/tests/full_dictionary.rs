//! Explicit data tests: tools/test-sudachi.sh. Ordinary workspace checks need no
//! 315 MiB dictionary and report these as ignored, never silently skip them.

use std::{path::PathBuf, sync::OnceLock};
use yomoku_analysis::{Analyzer, DICTIONARY_REVISION, ENGINE_REVISION};
use yomoku_core::{
    analysis::{ChapterAnalysis, SplitMode},
    document::ChapterText,
};

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
            newly_known: surfaces.len() - 1
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
