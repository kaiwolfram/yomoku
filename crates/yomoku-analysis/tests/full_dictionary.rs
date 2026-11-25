#[path = "../../../tests/support/storage.rs"]
mod storage_fixture;
// Explicit data tests: tools/test-sudachi.sh. Ordinary workspace checks need no
// 315 MiB dictionary and report these as ignored, never silently skip them.

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
fn import_grammar_repairs_small_tsu_and_retains_particle_and_source_boundaries() {
    for (text, expected) in [
        (
            "すごっ。すごッ。すごｯ。",
            vec!["すごっ", "。", "すごッ", "。", "すごｯ", "。"],
        ),
        (
            "端っこ。猫っぽかった。猫っぽくない。",
            vec!["端っこ", "。", "猫っぽかった", "。", "猫っぽくない", "。"],
        ),
        (
            "走って帰る。走っている。",
            vec!["走って", "帰る", "。", "走っている", "。"],
        ),
        (
            "それって何だっけ。",
            vec!["それ", "って", "何", "だ", "っけ", "。"],
        ),
        (
            "猫 っ。猫、っ。猫。っ。猫\nっ。",
            vec![
                "猫", "っ", "。", "猫", "、", "っ", "。", "猫", "。", "っ", "。", "猫", "っ", "。",
            ],
        ),
        (
            "走っ て。走っ、て。",
            vec!["走っ", "て", "。", "走っ", "、", "て", "。"],
        ),
    ] {
        let chapter = chapter(text);
        let result = analyzer().analyze_for_import(&chapter).unwrap();
        result.analysis.validate(&chapter).unwrap();
        assert_eq!(surfaces(&result.analysis), expected, "{text}");
        for (token, context) in result.analysis.tokens.iter().zip(result.context) {
            assert_eq!(token.anchor.slice(&chapter).unwrap(), token.surface);
            match token.surface.as_str() {
                "すごっ" | "すごッ" | "すごｯ" => {
                    assert!(context.preserve_join);
                    assert_eq!(token.dictionary_form.as_ref().unwrap().as_str(), "すごい");
                    assert_eq!(token.reading.as_deref(), Some("スゴッ"));
                }
                "猫っぽかった" | "猫っぽくない" => {
                    assert!(context.preserve_join && context.inflected);
                    assert_eq!(token.dictionary_form.as_ref().unwrap().as_str(), "猫っぽい");
                }
                "走って" => {
                    assert!(context.preserve_join && context.inflected);
                    assert_eq!(token.dictionary_form.as_ref().unwrap().as_str(), "走る");
                }
                "って" | "っけ" => assert!(context.standalone_grammar),
                _ => {}
            }
        }
    }
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
        for mode in [SplitMode::A, SplitMode::C, SplitMode::B] {
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
fn default_c_and_all_three_real_split_modes() {
    let text = "国家公務員";
    let a = analyze(text, SplitMode::A);
    let b = analyze(text, SplitMode::B);
    let c = analyze(text, SplitMode::C);
    assert_eq!(surfaces(&a), ["国家", "公務", "員"]);
    assert_eq!(surfaces(&b), ["国家", "公務員"]);
    assert_eq!(surfaces(&c), ["国家公務員"]);
    assert_eq!(analyzer().analyze(&chapter(text)).unwrap(), c);
}

#[test]
#[ignore = "requires pinned Sudachi Full; run tools/test-sudachi.sh"]
fn inflected_surfaces_stay_distinct_from_analyzer_forms() {
    let result = analyze("食べる。食べた。食べます。", SplitMode::C);
    assert_eq!(
        surfaces(&result),
        ["食べる", "。", "食べた", "。", "食べます", "。"]
    );
    let verb = &result.tokens[2];
    assert!(verb.is_japanese);
    assert_eq!(verb.dictionary_form.as_ref().unwrap().as_str(), "食べる");
    assert_eq!(verb.reading.as_deref(), Some("タベタ"));
    // Written conjugations remain distinct entries with the head’s base form.
    assert_ne!(
        verb.surface,
        verb.dictionary_form.as_ref().unwrap().as_str()
    );
}

#[test]
#[ignore = "requires pinned Sudachi Full; run tools/test-sudachi.sh"]
fn full_dictionary_kanji_only_proper_nouns_are_words() {
    let result = analyze("東京都と夏目漱石。", SplitMode::C);
    assert_eq!(surfaces(&result), ["東京都", "と", "夏目漱石", "。"]);
    for token in &result.tokens[..3] {
        assert!(token.is_japanese);
        assert!(token.dictionary_form.is_some());
        assert!(token.reading.is_some());
    }
    assert!(!result.tokens.last().unwrap().is_japanese);
}

#[test]
#[ignore = "requires pinned Sudachi Full; run tools/test-sudachi.sh"]
fn mixed_scripts_punctuation_and_unknown_kanji() {
    let result = analyze("English 123 📖 日本語。𰻞", SplitMode::C);
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
    for mode in [SplitMode::A, SplitMode::C, SplitMode::B] {
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
    for mode in [SplitMode::A, SplitMode::C, SplitMode::B] {
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
    let result = analyze(&text, SplitMode::C);
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
    let result = analyze("モーニング娘。が好き。猫。犬。", SplitMode::B);
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
    let document =
        storage_fixture::unanalyzed(&mut storage, &path, &PasteText::new(text.into()).unwrap());
    let mut saved = Vec::new();
    for mode in [SplitMode::A, SplitMode::C, SplitMode::B] {
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
fn real_mode_c_import_and_completion_preserve_surfaces_and_duplicate_identity() {
    use std::collections::BTreeSet;
    use yomoku_analysis::AnalysisError;
    use yomoku_core::{
        completion::{AnalyzedCompletionOutcome, PageCompletionIntent},
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
    assert_eq!(expected.mode, SplitMode::C);
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
        .load_effective_reader_tokens(imported.document.document_id, page)
        .unwrap()
        .unwrap();
    assert_eq!(
        lookup
            .segmentation
            .as_ref()
            .map(|s| s.original_analysis_id)
            .map_or(ReaderAnalysis::Missing, ReaderAnalysis::Available),
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
        assert_eq!(actual.segment_index, index as u64);
        assert_eq!(actual.anchor, token.anchor);
        assert_eq!(actual.surface, token.surface);
        assert_eq!(actual.metadata.reading.clone(), token.reading);
        assert_eq!(
            actual
                .metadata
                .dictionary_form
                .as_ref()
                .map(|s| s.as_str().to_owned())
                .as_deref(),
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
    let completion = storage_fixture::prepare(
        &mut storage,
        PageCompletionIntent {
            analysis_id: imported.analysis_id,
            document_id: imported.document.document_id,
            page,
            page_length,
            departing_position,
            completed_at_unix_ms: 50,
        },
    )
    .unwrap();
    assert_eq!(
        storage
            .record_effective_page_completion(&completion)
            .unwrap(),
        AnalyzedCompletionOutcome::Recorded {
            newly_known: (surfaces.len() - 1) as u64
        }
    );
    assert_eq!(
        storage
            .record_effective_page_completion(&completion)
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
        .load_effective_reader_tokens(imported.document.document_id, page)
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
        .save_document_reading_position(imported.document.document_id, departing_position)
        .unwrap();
    let reread = storage_fixture::prepare(
        &mut storage,
        PageCompletionIntent {
            analysis_id: imported.analysis_id,
            completed_at_unix_ms: 60,
            document_id: completion.completion().document_id,
            page: completion.completion().page,
            page_length: completion.completion().page_length,
            departing_position: completion.completion().departing_position,
        },
    )
    .unwrap();
    assert_eq!(
        storage.record_effective_page_completion(&reread).unwrap(),
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
        completion::{AnalyzedCompletionOutcome, PageCompletionIntent},
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
    assert_eq!(expected.mode, SplitMode::C);
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
        .load_txt_encoding(document.document_id)
        .unwrap()
        .unwrap();
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
        .load_effective_reader_tokens(document.document_id, page)
        .unwrap()
        .unwrap();
    assert_eq!(
        tokens
            .segmentation
            .as_ref()
            .map(|s| s.original_analysis_id)
            .map_or(ReaderAnalysis::Missing, ReaderAnalysis::Available),
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
    let completion = storage_fixture::prepare(
        &mut storage,
        PageCompletionIntent {
            analysis_id: imported.analysis_id,
            document_id: document.document_id,
            page,
            page_length,
            departing_position: ReadingPosition {
                source_id: page.source_id,
                chapter_id: page.chapter_id,
                offset: page.range.start,
            },
            completed_at_unix_ms: 200,
        },
    )
    .unwrap();
    assert_eq!(
        storage
            .record_effective_page_completion(&completion)
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
        storage.load_txt_encoding(document.document_id).unwrap(),
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
            .record_effective_page_completion(&completion)
            .unwrap(),
        AnalyzedCompletionOutcome::AlreadyRecorded
    );
    assert_eq!(
        storage
            .open_document_reader_snapshot(document.document_id)
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
        completion::{AnalyzedCompletionOutcome, PageCompletionIntent},
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
    assert_eq!(expected.mode, SplitMode::C);
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
    let source = storage_fixture::srt_file(
        &rusqlite::Connection::open(&path).unwrap(),
        document.document_id,
    )
    .unwrap()
    .unwrap();
    assert_eq!(source.encoding, "utf8");
    assert_eq!(
        canonical.split("\n\n").collect::<Vec<_>>(),
        ["猫と猫が食べた。", "国家公務員。𠮷 English 📖\0"]
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
        .load_effective_reader_tokens(document.document_id, page)
        .unwrap()
        .unwrap();
    assert_eq!(
        tokens
            .segmentation
            .as_ref()
            .map(|s| s.original_analysis_id)
            .map_or(ReaderAnalysis::Missing, ReaderAnalysis::Available),
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
    let completion = storage_fixture::prepare(
        &mut storage,
        PageCompletionIntent {
            analysis_id: imported.analysis_id,
            document_id: document.document_id,
            page,
            page_length,
            departing_position: ReadingPosition {
                source_id: page.source_id,
                chapter_id: page.chapter_id,
                offset: page.range.start,
            },
            completed_at_unix_ms: 200,
        },
    )
    .unwrap();
    assert_eq!(
        storage
            .record_effective_page_completion(&completion)
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
        storage_fixture::srt_file(
            &rusqlite::Connection::open(&path).unwrap(),
            document.document_id
        )
        .unwrap(),
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
            .record_effective_page_completion(&completion)
            .unwrap(),
        AnalyzedCompletionOutcome::AlreadyRecorded
    );
    assert_eq!(
        storage
            .open_document_reader_snapshot(document.document_id)
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
        assert_eq!(expected.mode, SplitMode::C);
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
    // Exercise the same original real Mode C chapters through the generic reader
    // repositories, without parsing again or keeping an analyzer in the reader.
    use yomoku_core::{
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
            .load_effective_reader_tokens(id, page.anchor)
            .unwrap()
            .unwrap();
        assert_eq!(
            tokens
                .segmentation
                .as_ref()
                .map(|s| s.original_analysis_id)
                .map_or(ReaderAnalysis::Missing, ReaderAnalysis::Available),
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

        let command = storage_fixture::prepare(
            &mut storage,
            PageCompletionIntent {
                document_id: id,
                page: page.anchor,
                page_length: snapshot.page_length,
                departing_position: page.position,
                completed_at_unix_ms: 300,
                analysis_id: imported.analysis_ids[index],
            },
        )
        .unwrap();

        assert!(matches!(
            storage.record_effective_page_completion(&command).unwrap(),
            AnalyzedCompletionOutcome::Recorded { .. }
        ));
        assert_eq!(
            storage.finished_at_least_once(id).unwrap(),
            Some(index == 1)
        );
        let after = storage.load_document_reader_snapshot(id).unwrap();
        assert_eq!(
            storage.record_effective_page_completion(&command).unwrap(),
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

#[test]
#[ignore = "requires pinned Sudachi Full; run tools/test-sudachi.sh"]
fn joined_conjugations_keep_head_forms_readings_and_raw_ab_components() {
    for (text, base, reading) in [
        ("できません", "できる", "デキマセン"),
        ("食べました", "食べる", "タベマシタ"),
        ("食べなかった", "食べる", "タベナカッタ"),
        ("高くない", "高い", "タカクナイ"),
        ("高くなかった", "高い", "タカクナカッタ"),
        ("高かった", "高い", "タカカッタ"),
        ("食べられた", "食べる", "タベラレタ"),
        ("書かせた", "書く", "カカセタ"),
        (
            "食べさせられませんでした",
            "食べる",
            "タベサセラレマセンデシタ",
        ),
        ("読んだ", "読む", "ヨンダ"),
        ("高いです", "高い", "タカイデス"),
        ("食べたい", "食べる", "タベタイ"),
        ("食べている", "食べる", "タベテイル"),
        ("食べていた", "食べる", "タベテイタ"),
        ("食べてしまった", "食べる", "タベテシマッタ"),
        (
            "食べておいてくれませんでした",
            "食べる",
            "タベテオイテクレマセンデシタ",
        ),
        ("読んでみる", "読む", "ヨンデミル"),
        ("読んである", "読む", "ヨンデアル"),
        ("泳いでいった", "泳ぐ", "オヨイデイッタ"),
        ("見て来た", "見る", "ミテキタ"),
        ("買ってあげた", "買う", "カッテアゲタ"),
        ("食べてもらった", "食べる", "タベテモラッタ"),
    ] {
        let result = analyze(text, SplitMode::C);
        assert_eq!(surfaces(&result), [text], "{text}");
        let token = &result.tokens[0];
        assert_eq!(
            token.dictionary_form.as_ref().unwrap().as_str(),
            base,
            "{text}"
        );
        assert_eq!(token.reading.as_deref(), Some(reading), "{text}");
        for mode in [SplitMode::A, SplitMode::B] {
            let raw = analyze(text, mode);
            assert!(raw.tokens.len() > 1, "{text}: {mode:?}");
            assert_eq!(
                raw.tokens
                    .iter()
                    .map(|t| t.surface.as_str())
                    .collect::<String>(),
                text
            );
        }
    }
}

#[test]
#[ignore = "requires pinned Sudachi Full; run tools/test-sudachi.sh"]
fn joining_stops_at_particles_independent_words_gaps_and_sentence_boundaries() {
    for (text, expected) in [
        ("食べるたび。", vec!["食べる", "たび", "。"]),
        ("食べてやる。", vec!["食べ", "て", "やる", "。"]),
        ("食べてほしい。", vec!["食べ", "て", "ほしい", "。"]),
        ("食べれば。", vec!["食べれ", "ば", "。"]),
        ("食べてはいる。", vec!["食べ", "ては", "いる", "。"]),
        ("食べて走った。", vec!["食べ", "て", "走った", "。"]),
        (
            "本がある。庭にいる。",
            vec!["本", "が", "ある", "。", "庭", "に", "いる", "。"],
        ),
        (
            "空を見る。東京へ行く。家に来る。",
            vec![
                "空", "を", "見る", "。", "東京", "へ", "行く", "。", "家", "に", "来る", "。",
            ],
        ),
        ("食べ ました", vec!["食べ", "まし", "た"]),
        ("食べ、ました", vec!["食べ", "、", "まし", "た"]),
        ("食べ。ました", vec!["食べ", "。", "まし", "た"]),
        ("食べ\nました", vec!["食べ", "まし", "た"]),
        ("食べて　いる", vec!["食べ", "て", "いる"]),
        ("高く\tない", vec!["高く", "ない"]),
        ("です。ます。", vec!["です", "。", "ます", "。"]),
    ] {
        assert_eq!(surfaces(&analyze(text, SplitMode::C)), expected, "{text}");
    }
    let text = "𠮷📖e\u{301}か\u{3099}\0\r\n食べました　高くない\t食べてしまった。";
    let result = analyze(text, SplitMode::C);
    for surface in ["食べました", "高くない", "食べてしまった"] {
        let token = result
            .tokens
            .iter()
            .find(|token| token.surface == surface)
            .unwrap();
        assert_eq!(
            i64::from(token.anchor.range.start),
            text.find(surface).unwrap() as i64
        );
        assert_eq!(
            i64::from(token.anchor.range.end),
            (text.find(surface).unwrap() + surface.len()) as i64
        );
    }
}

#[test]
#[ignore = "requires pinned Sudachi Full; run tools/test-sudachi.sh"]
fn previous_profile_supports_uncached_raw_components_but_cannot_be_retokenized() {
    use yomoku_analysis::{
        AnalysisError, CONJUGATION_ENGINE_REVISION, COPULA_ENGINE_REVISION, LEGACY_ENGINE_REVISION,
    };
    use yomoku_core::analysis::{
        AnalysisProfile, COMPOUND_IMPORT_POLICY_SUFFIX, IMPORT_POLICY_SUFFIX,
        IMPORT_POLICY_V3_SUFFIX, IMPORT_POLICY_V4_SUFFIX, IMPORT_POLICY_V5_SUFFIX,
        LEGACY_IMPORT_POLICY_SUFFIX, WORD_DICTIONARY_PREFIX,
    };
    let input = chapter("食べました。ひとつ。国家公務員。");
    let mut profile = AnalysisProfile {
        engine_revision: LEGACY_ENGINE_REVISION.into(),
        dictionary_revision: DICTIONARY_REVISION.into(),
    };
    for revision in [
        LEGACY_ENGINE_REVISION,
        CONJUGATION_ENGINE_REVISION,
        COPULA_ENGINE_REVISION,
        ENGINE_REVISION,
    ] {
        for suffix in [
            None,
            Some(LEGACY_IMPORT_POLICY_SUFFIX),
            Some(COMPOUND_IMPORT_POLICY_SUFFIX),
            Some(IMPORT_POLICY_V3_SUFFIX),
            Some(IMPORT_POLICY_V4_SUFFIX),
            Some(IMPORT_POLICY_V5_SUFFIX),
            Some(IMPORT_POLICY_SUFFIX),
        ] {
            profile.engine_revision = revision.into();
            profile.dictionary_revision = DICTIONARY_REVISION.into();
            if let Some(suffix) = suffix {
                profile.engine_revision.push_str(suffix);
                profile.dictionary_revision.push_str(&format!(
                    "{WORD_DICTIONARY_PREFIX}{}",
                    yomoku_storage::dictionary::IMPORT_DICTIONARY_SHA256
                ));
            }
            for mode in [SplitMode::A, SplitMode::B] {
                let legacy = analyzer()
                    .analyze_components(&input, mode, &profile)
                    .unwrap();
                assert_eq!(legacy.profile, profile);
                assert_eq!(
                    &surfaces(&legacy)[..7],
                    &["食べ", "まし", "た", "。", "ひと", "つ", "。"]
                );
                assert_eq!(
                    legacy.tokens,
                    analyzer().analyze_mode(&input, mode).unwrap().tokens
                );
            }
            assert!(matches!(
                analyzer().analyze_components(&input, SplitMode::C, &profile),
                Err(AnalysisError::UnsupportedProfile)
            ));
        }
    }
    profile.engine_revision = "unknown".into();
    assert!(matches!(
        analyzer().analyze_components(&input, SplitMode::A, &profile),
        Err(AnalysisError::UnsupportedProfile)
    ));
    profile.engine_revision = ENGINE_REVISION.into();
    profile.dictionary_revision = "unknown".into();
    assert!(matches!(
        analyzer().analyze_components(&input, SplitMode::B, &profile),
        Err(AnalysisError::UnsupportedProfile)
    ));
}

#[test]
#[ignore = "requires pinned Sudachi Full; run tools/test-sudachi.sh"]
fn joined_surfaces_register_once_retain_status_and_count_once_across_pages() {
    use yomoku_core::{
        completion::{AnalyzedCompletionOutcome, PageCompletionIntent},
        document::PasteText,
        library::WordStatusCounts,
        pagination::{LogicalPages, PageLength, ReadingPosition},
        vocabulary::{Surface, VocabularyStatus},
    };
    use yomoku_storage::{PasteImportOutcome, Storage};
    let directory = std::env::temp_dir().join(format!(
        "yomoku-joined-pages-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir(&directory).unwrap();
    let path = directory.join("reader.sqlite3");
    let mut storage = Storage::open(&path).unwrap();
    let surface = Surface::new("食べました").unwrap();
    storage
        .register_vocabulary(std::slice::from_ref(&surface), 10)
        .unwrap();
    let entry = storage.vocabulary_entry(&surface).unwrap().unwrap();
    let familiar = storage
        .set_vocabulary_status(entry.id, VocabularyStatus::Familiar, 20)
        .unwrap()
        .unwrap();
    let page_length = PageLength::default();
    // Place the first joined surface across the threshold for the current preset.
    let text = PasteText::new(format!(
        "{}. 食べました。\n食べました。",
        "x".repeat(page_length.characters() - 3)
    ))
    .unwrap();
    let PasteImportOutcome::Created(imported) = storage
        .import_analyzed_paste(&text, 30, |chapter| analyzer().analyze(chapter))
        .unwrap()
    else {
        panic!("expected new import")
    };
    assert_eq!(imported.registered_surfaces, 0);
    assert_eq!(
        storage.vocabulary_entry(&surface).unwrap(),
        Some(familiar.clone())
    );
    for component in ["食べ", "まし", "た", "食べる"] {
        assert!(storage
            .vocabulary_entry(&Surface::new(component).unwrap())
            .unwrap()
            .is_none());
    }
    let snapshot = storage
        .chapter_analysis(imported.analysis_id)
        .unwrap()
        .unwrap();
    assert_eq!(
        snapshot
            .tokens
            .iter()
            .filter(|token| token.is_japanese)
            .map(|token| token.surface.as_str())
            .collect::<Vec<_>>(),
        ["食べました", "食べました"]
    );
    let layout = LogicalPages::new(
        std::slice::from_ref(&imported.document.chapter),
        page_length,
    )
    .unwrap();
    assert_eq!(layout.pages().len(), 2);
    let first = snapshot
        .tokens
        .iter()
        .find(|token| token.is_japanese)
        .unwrap();
    assert!(first.anchor.range.start < layout.pages()[0].range.end);
    assert!(first.anchor.range.end <= layout.pages()[0].range.end);
    let sql = rusqlite::Connection::open(&path).unwrap();
    for (index, page) in layout.pages().iter().copied().enumerate() {
        let loaded = storage
            .load_effective_reader_tokens(imported.document.document_id, page)
            .unwrap()
            .unwrap();
        assert!(
            loaded
                .tokens
                .iter()
                .all(|token| token.surface == "食べました"
                    && token.vocabulary.unwrap().id == entry.id)
        );
        let completion = storage_fixture::prepare(
            &mut storage,
            PageCompletionIntent {
                analysis_id: imported.analysis_id,
                document_id: imported.document.document_id,
                page,
                page_length,
                departing_position: ReadingPosition {
                    source_id: page.source_id,
                    chapter_id: page.chapter_id,
                    offset: page.range.start,
                },
                completed_at_unix_ms: 40 + index as i64,
            },
        )
        .unwrap();
        assert_eq!(
            storage
                .record_effective_page_completion(&completion)
                .unwrap(),
            AnalyzedCompletionOutcome::Recorded { newly_known: 0 }
        );
        assert_eq!(
            storage
                .record_effective_page_completion(&completion)
                .unwrap(),
            AnalyzedCompletionOutcome::AlreadyRecorded
        );
        let count: i64 = sql
            .query_row(
                "SELECT COUNT(*) FROM reading_occurrences WHERE surface = '食べました'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, index as i64 + 1);
        let total: i64 = sql
            .query_row("SELECT COUNT(*) FROM reading_occurrences", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(total, count);
    }
    assert_eq!(
        storage.effective_library_word_distributions().unwrap()[0].counts,
        Some(WordStatusCounts {
            familiar: 1,
            ..Default::default()
        })
    );
    assert_eq!(storage.vocabulary_entry(&surface).unwrap(), Some(familiar));
    let duplicate = storage
        .import_analyzed_paste(
            &text,
            50,
            |_| -> Result<_, yomoku_analysis::AnalysisError> {
                panic!("duplicate must not reanalyze")
            },
        )
        .unwrap();
    assert!(matches!(duplicate, PasteImportOutcome::Existing(_)));
    drop(sql);
    drop(storage);
    let mut reopened = Storage::open(&path).unwrap();
    assert_eq!(
        reopened
            .chapter_analysis(imported.analysis_id)
            .unwrap()
            .unwrap(),
        snapshot
    );
    assert_eq!(
        reopened.vocabulary_entry(&surface).unwrap().unwrap().status,
        VocabularyStatus::Familiar
    );
    drop(reopened);
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
#[ignore = "requires pinned Sudachi Full; run tools/test-sudachi.sh"]
fn copula_past_keeps_nouns_separate_and_preserves_source_reading_and_base() {
    let result = analyze("学生だった。先生でした。", SplitMode::C);
    assert_eq!(
        surfaces(&result),
        ["学生", "だった", "。", "先生", "でした", "。"]
    );
    assert_eq!(result.tokens[1].reading.as_deref(), Some("ダッタ"));
    assert_eq!(
        result.tokens[1].dictionary_form.as_ref().unwrap().as_str(),
        "だ"
    );
    let raw = analyze("学生だった。", SplitMode::B);
    assert_eq!(surfaces(&raw), ["学生", "だっ", "た", "。"]);
    let chapter = chapter("学生だった。");
    for revision in [
        yomoku_analysis::LEGACY_ENGINE_REVISION,
        yomoku_analysis::CONJUGATION_ENGINE_REVISION,
    ] {
        let profile = yomoku_core::analysis::AnalysisProfile {
            engine_revision: revision.into(),
            dictionary_revision: DICTIONARY_REVISION.into(),
        };
        assert_eq!(
            analyzer()
                .analyze_components(&chapter, SplitMode::B, &profile)
                .unwrap()
                .tokens,
            raw.tokens
        );
    }
}

#[test]
#[ignore = "requires pinned Sudachi Full; run tools/test-sudachi.sh"]
fn kimi_no_na_wa_hitotsu_keeps_the_following_particle_separate() {
    let text = "失ったものは未だひとつもなく、とても甘やかな気持ちが、じんじんと体に満ちている。";
    let analysis = analyze(text, SplitMode::C);
    let index = analysis
        .tokens
        .iter()
        .position(|t| t.surface == "ひとつ")
        .unwrap();
    let counted = &analysis.tokens[index];
    assert_eq!(counted.dictionary_form.as_ref().unwrap().as_str(), "ひとつ");
    assert_eq!(counted.reading.as_deref(), Some("ヒトツ"));
    assert_eq!(analysis.tokens[index + 1].surface, "も");
    assert_eq!(
        i64::from(counted.anchor.range.start),
        text.find("ひとつ").unwrap() as i64
    );
    assert_eq!(
        counted.anchor.range.end,
        analysis.tokens[index + 1].anchor.range.start
    );
}

#[test]
#[ignore = "requires pinned Sudachi Full; run tools/test-sudachi.sh"]
fn numeral_counter_pairs_join_with_complete_dictionary_forms() {
    for (text, reading) in [
        ("ひとつ", Some("ヒトツ")),
        ("ふたつ", Some("フタツ")),
        ("みっつ", Some("ミッツ")),
        ("一つ", None),
        ("二個", None),
        ("三匹", None),
        ("2枚", None),
    ] {
        let joined = analyze(text, SplitMode::C);
        assert_eq!(surfaces(&joined), [text], "{text}");
        let token = &joined.tokens[0];
        assert!(token.is_japanese, "{text}");
        assert_eq!(token.dictionary_form.as_ref().unwrap().as_str(), text);
        assert_eq!(token.reading.as_deref(), reading, "{text}");
        for mode in [SplitMode::A, SplitMode::B] {
            let raw = analyze(text, mode);
            assert_eq!(raw.tokens.len(), 2, "{text}: {mode:?}");
            assert_eq!(surfaces(&raw).concat(), text);
        }
    }
}

#[test]
#[ignore = "requires pinned Sudachi Full; run tools/test-sudachi.sh"]
fn counter_joining_preserves_gaps_particles_nouns_and_other_suffixes() {
    for (text, expected) in [
        ("一 つ", vec!["一", "つ"]),
        ("一　つ", vec!["一", "つ"]),
        ("一\tつ", vec!["一", "つ"]),
        ("一\nつ", vec!["一", "つ"]),
        ("一、つ", vec!["一", "、", "つ"]),
        ("一。つ", vec!["一", "。", "つ"]),
        ("一猫", vec!["一", "猫"]),
        ("猫つ", vec!["猫", "つ"]),
        ("一つを", vec!["一つ", "を"]),
        ("三本の本", vec!["三本", "の", "本"]),
        // The pinned dictionary tags 冊 as a general suffix, not a counter.
        ("三冊", vec!["三", "冊"]),
    ] {
        assert_eq!(surfaces(&analyze(text, SplitMode::C)), expected, "{text}");
    }
}

#[test]
#[ignore = "requires pinned Sudachi Full and word dictionaries; run tools/test-sudachi.sh"]
fn counted_whole_word_lookup_excludes_numeral_only_entries() {
    let path = std::env::var_os("YOMOKU_WORD_DICTIONARY")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../.local/dictionaries/index-v1-20261025/dictionary.sqlite3")
        });
    let dictionary = yomoku_storage::dictionary::Dictionary::open(path).unwrap();
    for surface in ["ひとつ", "一つ"] {
        let analysis = analyze(surface, SplitMode::C);
        let token = &analysis.tokens[0];
        let results = dictionary
            .lookup(
                &token.surface,
                token.dictionary_form.as_ref().map(|form| form.as_str()),
            )
            .unwrap();
        let expected = dictionary.lookup(surface, None).unwrap();
        assert_eq!(results.words, expected.words);
        assert_eq!(results.names, expected.names);
        assert_eq!(results.words.len(), 1);
        assert_eq!(results.words[0].entry_id, 1160820);
        assert_eq!(results.words[0].forms[0].form, surface);
    }
}
