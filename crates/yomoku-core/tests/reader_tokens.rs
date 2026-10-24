use yomoku_core::{
    analysis::{AnalysisProfile, AnalyzedToken, ChapterAnalysis, SplitMode},
    document::{ChapterText, TextAnchor, TextOffset, TextRange},
    pagination::{LogicalPages, PageLength},
    reader::reader_tokens_on_page,
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
    let mut cursor = 0;
    ChapterAnalysis {
        profile: AnalysisProfile {
            engine_revision: "reader-fixture@1".into(),
            dictionary_revision: "reader-fixture-full@1".into(),
        },
        mode: SplitMode::B,
        tokens: chapter
            .text
            .split_whitespace()
            .map(|surface| {
                let start = cursor + chapter.text[cursor..].find(surface).unwrap();
                cursor = start + surface.len();
                AnalyzedToken {
                    anchor: anchor(chapter, start, cursor),
                    surface: surface.into(),
                    is_japanese: surface != "English📖\0e\u{301}",
                    reading: Some("フィクスチャ".into()),
                    dictionary_form: Some(Surface::new("語").unwrap()),
                }
            })
            .collect(),
    }
}

#[test]
fn visible_tokens_preserve_exact_surfaces_metadata_indices_and_page_local_unicode_ranges() {
    let chapter = chapter("English📖\0e\u{301} \r\n𠮷野家 が か\u{3099} が ");
    let analysis = analysis(&chapter);
    let start = chapter.text.find('𠮷').unwrap();
    let page = anchor(&chapter, start, chapter.text.len());
    let before = analysis.clone();
    let tokens = reader_tokens_on_page(&chapter, &analysis, page).unwrap();
    assert_eq!(tokens.len(), 4);
    assert_eq!(tokens[0].token_index, 1); // the Latin/emoji token remains unselectable
    assert_eq!(tokens[0].surface, "𠮷野家");
    assert_eq!(tokens[0].reading.as_deref(), Some("フィクスチャ"));
    assert_eq!(tokens[0].dictionary_form.as_deref(), Some("語"));
    assert_ne!(tokens[1].surface, tokens[2].surface); // no normalization
    assert_eq!(tokens[1].surface, tokens[3].surface); // retain repetitions
    assert_ne!(tokens[1].token_index, tokens[3].token_index);
    for token in tokens {
        assert_eq!(token.anchor.slice(&chapter).unwrap(), token.surface);
        assert_eq!(
            token
                .display_range
                .slice(page.slice(&chapter).unwrap())
                .unwrap(),
            token.surface
        );
        assert!(token.vocabulary.is_none());
    }
    assert_eq!(analysis, before);
}

#[test]
fn crossing_tokens_are_visible_on_each_page_but_keep_one_full_identity_for_all_presets() {
    let chapter = chapter(&format!(
        "{}。\n{}。\n猫",
        "𠮷".repeat(600),
        "犬".repeat(1000)
    ));
    let mut analysis = analysis(&chapter);
    // Deliberately synthetic whole-chapter token straddles every logical page.
    analysis.tokens.truncate(1);
    analysis.tokens[0].anchor = anchor(&chapter, 0, chapter.text.len());
    analysis.tokens[0].surface = chapter.text.clone();
    let full = analysis.tokens[0].anchor;
    for length in PageLength::ALL {
        let pages = LogicalPages::new(std::slice::from_ref(&chapter), length).unwrap();
        let mut visible = String::new();
        for page in pages.pages() {
            let tokens = reader_tokens_on_page(&chapter, &analysis, *page).unwrap();
            assert_eq!(tokens.len(), 1);
            assert_eq!(tokens[0].anchor, full);
            assert_eq!(tokens[0].surface, chapter.text);
            assert_eq!(tokens[0].token_index, 0);
            visible.push_str(
                tokens[0]
                    .display_range
                    .slice(page.slice(&chapter).unwrap())
                    .unwrap(),
            );
        }
        assert_eq!(visible, chapter.text);
        // Display overlap does not alter start-based completion ownership.
        assert_eq!(
            pages
                .pages()
                .iter()
                .map(|page| analysis
                    .japanese_tokens_on_page(&chapter, *page)
                    .unwrap()
                    .len())
                .sum::<usize>(),
            1
        );
    }
}

#[test]
fn whitespace_and_touching_boundaries_have_no_phantom_tokens() {
    let chapter = chapter("猫 \r\n犬");
    let analysis = analysis(&chapter);
    for (start, end, expected) in [(0, 3, vec![0]), (3, 6, vec![]), (6, 9, vec![1])] {
        let tokens =
            reader_tokens_on_page(&chapter, &analysis, anchor(&chapter, start, end)).unwrap();
        assert_eq!(
            tokens.iter().map(|t| t.token_index).collect::<Vec<_>>(),
            expected
        );
    }
}

#[test]
fn rejects_foreign_empty_reversed_split_scalar_and_non_b_or_corrupt_analysis() {
    let chapter = chapter("猫𠮷");
    let analysis = analysis(&chapter);
    let page = anchor(&chapter, 0, chapter.text.len());
    let mut invalid = vec![
        anchor(&chapter, 0, 0),
        anchor(&chapter, chapter.text.len(), chapter.text.len()),
    ];
    invalid.push(TextAnchor {
        source_id: 99.try_into().unwrap(),
        ..page
    });
    invalid.push(TextAnchor {
        chapter_id: 99.try_into().unwrap(),
        ..page
    });
    invalid.push(TextAnchor {
        range: TextRange {
            start: 1.try_into().unwrap(),
            ..page.range
        },
        ..page
    });
    invalid.push(TextAnchor {
        range: TextRange {
            start: page.range.end,
            end: page.range.start,
        },
        ..page
    });
    for page in invalid {
        assert!(reader_tokens_on_page(&chapter, &analysis, page).is_err());
    }
    for mode in [SplitMode::A, SplitMode::C] {
        let mut bad = analysis.clone();
        bad.mode = mode;
        assert!(reader_tokens_on_page(&chapter, &bad, page).is_err());
    }
    let mut bad = analysis;
    bad.tokens[0].surface = "猫吉".into();
    assert!(reader_tokens_on_page(&chapter, &bad, page).is_err());
}
