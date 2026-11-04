use yomoku_core::{
    analysis::{AnalysisProfile, AnalyzedToken, ChapterAnalysis, SplitMode},
    document::{ChapterText, TextAnchor, TextRange},
    effective_reader::EffectiveReaderChapter,
    reader_ruby::{ReaderChapterRuby, ReaderRuby},
};

fn chapter(text: &str) -> ChapterText {
    ChapterText {
        source_id: 1.try_into().unwrap(),
        chapter_id: 2.try_into().unwrap(),
        text: text.into(),
    }
}

fn range(start: usize, end: usize) -> TextRange {
    TextRange {
        start: (start as i64).try_into().unwrap(),
        end: (end as i64).try_into().unwrap(),
    }
}

fn anchor(chapter: &ChapterText, start: usize, end: usize) -> TextAnchor {
    TextAnchor {
        source_id: chapter.source_id,
        chapter_id: chapter.chapter_id,
        range: range(start, end),
    }
}

fn analysis(chapter: &ChapterText, mode: SplitMode, parts: &[&str]) -> ChapterAnalysis {
    let mut end = 0;
    ChapterAnalysis {
        profile: AnalysisProfile {
            engine_revision: "fixture@1".into(),
            dictionary_revision: "full@1".into(),
        },
        mode,
        tokens: parts
            .iter()
            .enumerate()
            .map(|(index, part)| {
                let start = end + chapter.text[end..].find(part).unwrap();
                end = start + part.len();
                AnalyzedToken {
                    anchor: anchor(chapter, start, end),
                    surface: (*part).into(),
                    is_japanese: *part != "。",
                    // Deliberately give punctuation a reading too.
                    reading: Some(format!("{mode:?}{index}𠮷か\u{3099}\0")),
                    dictionary_form: None,
                }
            })
            .collect(),
    }
}

#[test]
fn invalid_pages_and_whole_source_ruby_fail_without_partial_output() {
    let chapter = chapter("𠮷猫犬。");
    let original = analysis(&chapter, SplitMode::C, &["𠮷", "猫", "犬", "。"]);
    let effective =
        EffectiveReaderChapter::resolve(&chapter, 10.try_into().unwrap(), &original).unwrap();
    let page = anchor(&chapter, 0, 4);
    let source = ReaderChapterRuby {
        chapter_id: chapter.chapter_id,
        ruby: vec![
            ReaderRuby {
                range: range(0, 4),
                reading: "よし".into(),
            },
            ReaderRuby {
                range: range(7, 10),
                reading: "いぬ".into(),
            },
        ],
    };
    assert!(effective.furigana_on_page(page, Some(&source)).is_ok());
    let mut bad_pages = vec![
        anchor(&chapter, 0, 0),
        anchor(&chapter, chapter.text.len(), chapter.text.len()),
    ];
    for field in 0..5 {
        let mut bad = page;
        match field {
            0 => bad.source_id = 99.try_into().unwrap(),
            1 => bad.chapter_id = 99.try_into().unwrap(),
            2 => bad.range.end = 2.try_into().unwrap(),
            3 => bad.range.end = 99.try_into().unwrap(),
            _ => bad.range = range(4, 0),
        }
        bad_pages.push(bad);
    }
    for page in bad_pages {
        assert!(effective.furigana_on_page(page, None).is_err());
        assert!(effective.furigana_on_page(page, Some(&source)).is_err());
    }
    for field in 0..6 {
        let mut bad = source.clone();
        match field {
            0 => bad.chapter_id = 99.try_into().unwrap(),
            1 => bad.ruby[1].range = range(8, 10), // off-page invalid UTF-8
            2 => bad.ruby[1].reading.clear(),
            3 => bad.ruby[1].range = range(0, 4),
            4 => bad.ruby.reverse(),
            _ => bad.ruby[1].range = range(7, 99),
        }
        assert!(
            effective.furigana_on_page(page, Some(&bad)).is_err(),
            "{bad:?}"
        );
    }
}
