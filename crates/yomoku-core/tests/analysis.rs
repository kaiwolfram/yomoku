use yomoku_core::{
    analysis::{
        AnalysisProfile, AnalyzedToken, ChapterAnalysis, OverrideScope, SegmentationOverride,
        SplitMode,
    },
    document::{ChapterText, TextAnchor, TextOffset, TextRange},
    vocabulary::Surface,
};

fn chapter(text: &str) -> ChapterText {
    ChapterText {
        source_id: 1.try_into().unwrap(),
        chapter_id: 2.try_into().unwrap(),
        text: text.into(),
    }
}

fn anchor(chapter: &ChapterText, start: usize, end: usize) -> TextAnchor {
    TextAnchor {
        source_id: chapter.source_id,
        chapter_id: chapter.chapter_id,
        range: TextRange {
            start: TextOffset::at(&chapter.text, start).unwrap(),
            end: TextOffset::at(&chapter.text, end).unwrap(),
        },
    }
}

fn analysis(chapter: &ChapterText) -> ChapterAnalysis {
    ChapterAnalysis {
        profile: AnalysisProfile {
            engine_revision: "fixture-engine@1".into(),
            dictionary_revision: "fixture-dictionary@1".into(),
        },
        mode: SplitMode::default(),
        tokens: vec![AnalyzedToken {
            anchor: anchor(chapter, 0, chapter.text.len()),
            surface: chapter.text.clone(),
            is_japanese: true,
            reading: None,
            dictionary_form: None,
        }],
    }
}

#[test]
fn japanese_occurrences_belong_once_to_the_page_containing_their_start() {
    use yomoku_core::pagination::{LogicalPages, PageLength};

    let source = chapter(&format!(
        "{}。\n{}。\n猫",
        "猫".repeat(600),
        "犬".repeat(1000)
    ));
    // A synthetic whole-chapter token deliberately crosses every page boundary.
    // Its original anchor must remain intact and count once for every preset.
    let snapshot = analysis(&source);
    for length in PageLength::ALL {
        let pages = LogicalPages::new(std::slice::from_ref(&source), length).unwrap();
        let occurrences = pages
            .pages()
            .iter()
            .flat_map(|page| snapshot.japanese_tokens_on_page(&source, *page).unwrap())
            .collect::<Vec<_>>();
        assert_eq!(occurrences, vec![(0, &snapshot.tokens[0])]);
    }

    let source = chapter("猫𠮷");
    let mut snapshot = analysis(&source);
    let mut second = snapshot.tokens[0].clone();
    snapshot.tokens[0].anchor = anchor(&source, 0, 3);
    snapshot.tokens[0].surface = "猫".into();
    second.anchor = anchor(&source, 3, source.text.len());
    second.surface = "𠮷".into();
    snapshot.tokens.push(second);
    let first = anchor(&source, 0, 3);
    let next = anchor(&source, 3, source.text.len());
    assert_eq!(
        snapshot.japanese_tokens_on_page(&source, first).unwrap(),
        vec![(0, &snapshot.tokens[0])]
    );
    assert_eq!(
        snapshot.japanese_tokens_on_page(&source, next).unwrap(),
        vec![(1, &snapshot.tokens[1])]
    );
    snapshot.tokens[1].is_japanese = false;
    assert!(snapshot
        .japanese_tokens_on_page(&source, next)
        .unwrap()
        .is_empty());
}

#[test]
fn page_occurrence_projection_rejects_foreign_split_scalar_and_invalid_analysis() {
    let chapter = chapter("猫𠮷");
    let mut analysis = analysis(&chapter);
    let mut page = anchor(&chapter, 0, chapter.text.len());
    page.chapter_id = 99.try_into().unwrap();
    assert!(analysis.japanese_tokens_on_page(&chapter, page).is_err());
    page = anchor(&chapter, 0, chapter.text.len());
    page.range.start = 1.try_into().unwrap();
    assert!(analysis.japanese_tokens_on_page(&chapter, page).is_err());
    analysis.tokens[0].surface = "犬".into();
    assert!(analysis
        .japanese_tokens_on_page(&chapter, anchor(&chapter, 0, chapter.text.len()))
        .is_err());
}

#[test]
fn exact_original_unicode_surface_and_metadata_are_separate() {
    let chapter = chapter("𠮷が食べた\0e\u{301}📖");
    let mut analysis = analysis(&chapter);
    analysis.tokens[0].dictionary_form = Some(Surface::new("食べる").unwrap());
    analysis.tokens[0].reading = Some("タベタ".into());
    assert_eq!(analysis.mode, SplitMode::B);
    assert!(analysis.validate(&chapter).is_ok());
    assert_eq!(
        analysis.tokens[0].anchor.slice(&chapter).unwrap(),
        chapter.text
    );
    analysis.tokens[0].surface = "𠮷が食べる\0é📖".into();
    assert!(analysis.validate(&chapter).is_err());
}

#[test]
fn rejects_wrong_ownership_split_scalars_overlap_and_uncovered_content() {
    let chapter = chapter("日本語");
    let original = analysis(&chapter);
    let mut bad = original.clone();
    bad.tokens[0].anchor.source_id = 9.try_into().unwrap();
    assert!(bad.validate(&chapter).is_err());
    bad = original.clone();
    bad.tokens[0].anchor.range.start = 1.try_into().unwrap();
    assert!(bad.validate(&chapter).is_err());
    bad = original.clone();
    bad.tokens.push(bad.tokens[0].clone());
    assert!(bad.validate(&chapter).is_err());
    bad = original.clone();
    bad.tokens.clear();
    assert!(bad.validate(&chapter).is_err());
    bad = original.clone();
    bad.profile.dictionary_revision = " \n".into();
    assert!(bad.validate(&chapter).is_err());
    bad = original;
    bad.tokens[0].reading = Some(" ".into());
    assert!(bad.validate(&chapter).is_err());
}

#[test]
fn whitespace_gaps_are_allowed_but_punctuation_and_latin_cannot_disappear() {
    let chapter = chapter(" \r\n日本語\t");
    let mut valid = analysis(&chapter);
    valid.tokens[0].anchor = anchor(&chapter, 3, 12);
    valid.tokens[0].surface = "日本語".into();
    assert!(valid.validate(&chapter).is_ok());
    for text in [" ", "", "\r\n\t"] {
        let chapter = self::chapter(text);
        let mut empty = analysis(&chapter);
        empty.tokens.clear();
        assert!(empty.validate(&chapter).is_ok());
    }
    for text in ["a日本語", "。日本語"] {
        let chapter = self::chapter(text);
        let mut missing = analysis(&chapter);
        let start = text.find('日').unwrap();
        missing.tokens[0].anchor = anchor(&chapter, start, text.len());
        missing.tokens[0].surface = "日本語".into();
        assert!(missing.validate(&chapter).is_err());
    }
}

#[test]
fn split_and_merge_partition_exact_source_without_mutating_base_analysis() {
    let chapter = chapter("📖𠮷野家で食べた。");
    let base = analysis(&chapter);
    let selected = "𠮷野家で食べた";
    let anchor = anchor(&chapter, 4, 4 + selected.len());
    let mut edit = SegmentationOverride {
        scope: OverrideScope::default(),
        anchor,
        surface: Surface::new(selected).unwrap(),
        parts: ["𠮷野家", "で", "食べた"]
            .map(|s| Surface::new(s).unwrap())
            .to_vec(),
    };
    assert_eq!(edit.scope, OverrideScope::AllOccurrences);
    let parts = edit.replacement_anchors(&chapter).unwrap();
    assert_eq!(
        parts
            .iter()
            .map(|a| a.slice(&chapter).unwrap())
            .collect::<Vec<_>>(),
        ["𠮷野家", "で", "食べた"]
    );
    assert_eq!(parts[0].range.start, anchor.range.start);
    assert_eq!(parts.last().unwrap().range.end, anchor.range.end);
    edit.scope = OverrideScope::ThisOccurrence;
    edit.parts = vec![edit.surface.clone()];
    assert_eq!(edit.replacement_anchors(&chapter).unwrap(), [anchor]);
    assert_eq!(base, analysis(&chapter));
    assert!(base.validate(&chapter).is_ok());
}

#[test]
fn overrides_reject_normalization_missing_parts_bad_ranges_and_wrong_source() {
    let chapter = chapter("が𠮷\0食べた");
    let original = SegmentationOverride {
        scope: OverrideScope::ThisOccurrence,
        anchor: anchor(&chapter, 0, chapter.text.len()),
        surface: Surface::new(&chapter.text).unwrap(),
        parts: ["が", "𠮷\0", "食べた"]
            .map(|s| Surface::new(s).unwrap())
            .to_vec(),
    };
    assert!(original.replacement_anchors(&chapter).is_ok());
    let mut bad = original.clone();
    bad.parts[0] = Surface::new("か\u{3099}").unwrap();
    assert!(bad.replacement_anchors(&chapter).is_err());
    bad = original.clone();
    bad.parts.clear();
    assert!(bad.replacement_anchors(&chapter).is_err());
    bad = original.clone();
    bad.anchor.range.end = TextOffset::START;
    assert!(bad.replacement_anchors(&chapter).is_err());
    bad = original;
    bad.anchor.chapter_id = 99.try_into().unwrap();
    assert!(bad.replacement_anchors(&chapter).is_err());
}
