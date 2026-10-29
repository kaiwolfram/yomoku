use yomoku_core::{
    analysis::{
        AnalysisProfile, AnalyzedToken, ChapterAnalysis, OverrideScope, SegmentationOverride,
        SplitMode, StoredOverride,
    },
    document::{ChapterText, PastedDocument, TextAnchor, TextOffset, TextRange},
    effective_reader::{EffectiveReaderChapter, EffectiveReaderTokenSnapshot},
    effective_session::EffectiveReaderSession,
    pagination::{PageLength, ReadingPosition},
    reader::{ReaderAnalysis, ReaderPage, ReaderTokenKey},
    reader_ruby::{ReaderChapterRuby, ReaderRuby},
    vocabulary::Surface,
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

fn rule(chapter: &ChapterText, id: i64, surface: &str, parts: &[&str]) -> StoredOverride {
    let start = chapter.text.find(surface).unwrap();
    StoredOverride {
        id: id.try_into().unwrap(),
        active: true,
        edit: SegmentationOverride {
            scope: OverrideScope::AllOccurrences,
            anchor: anchor(chapter, start, start + surface.len()),
            surface: Surface::new(surface).unwrap(),
            parts: parts
                .iter()
                .map(|part| Surface::new(*part).unwrap())
                .collect(),
        },
    }
}

fn assert_retained_ruby(
    chapter: &ChapterText,
    effective: &EffectiveReaderChapter<'_>,
    anchor: TextAnchor,
    source: Option<&ReaderChapterRuby>,
    expected: &[ReaderRuby],
) {
    let page = ReaderPage {
        document_id: 1.try_into().unwrap(),
        title: "本".into(),
        finished_at_least_once: false,
        text: anchor.slice(chapter).unwrap().to_owned(),
        anchor,
        position: ReadingPosition {
            source_id: anchor.source_id,
            chapter_id: anchor.chapter_id,
            offset: anchor.range.start,
        },
        page_number: 1,
        total_pages: 1,
        formatting: vec![],
        images: vec![],
        source_ruby: source
            .map(|s| s.on_page(chapter, anchor.range).unwrap())
            .unwrap_or_default(),
    };
    let analysis = ReaderAnalysis::Available(effective.segmentation().original_analysis_id);
    let mut session = EffectiveReaderSession::default();
    let request = session
        .begin_load(ReaderTokenKey {
            document_id: page.document_id,
            page: anchor,
            analysis,
        })
        .unwrap();
    assert!(session.accept(
        request,
        EffectiveReaderTokenSnapshot {
            original_boundaries: vec![],
            document_id: page.document_id,
            page: anchor,
            segmentation: Some(effective.segmentation().clone()),
            tokens: effective.tokens_on_page(anchor).unwrap(),
        }
    ));
    let before = session.page().unwrap().clone();
    assert_eq!(session.furigana(&page, analysis).unwrap(), expected);
    assert_eq!(session.page(), Some(&before));
}

#[test]
fn exact_effective_b_a_c_readings_exclude_unresolved_manual_and_non_japanese_parts() {
    let chapter = chapter("国家公務員 食べた 猫犬 鳥。");
    let mut original = analysis(
        &chapter,
        SplitMode::B,
        &["国家", "公務員", "食べ", "た", "猫", "犬", "鳥", "。"],
    );
    original.tokens[6].reading = None;
    let alternates = [
        (
            11.try_into().unwrap(),
            analysis(
                &chapter,
                SplitMode::A,
                &["国家", "公務", "員", "食べ", "た", "猫", "犬", "鳥", "。"],
            ),
        ),
        (
            12.try_into().unwrap(),
            analysis(
                &chapter,
                SplitMode::C,
                &["国家公務員", "食べた", "猫", "犬", "鳥", "。"],
            ),
        ),
    ];
    let rules = [
        rule(&chapter, 1, "国家公務員", &["国家", "公務", "員"]),
        rule(&chapter, 2, "食べた", &["食べた"]),
        rule(&chapter, 3, "猫犬", &["猫犬"]),
    ];
    let before = (original.clone(), alternates.clone(), rules.clone());
    let effective = EffectiveReaderChapter::resolve(
        &chapter,
        10.try_into().unwrap(),
        &original,
        &alternates,
        &rules,
    )
    .unwrap();
    let ruby = effective
        .furigana_on_page(anchor(&chapter, 0, chapter.text.len()), None)
        .unwrap();
    assert_eq!(
        ruby.iter()
            .map(|r| r.range.slice(&chapter.text).unwrap())
            .collect::<Vec<_>>(),
        ["国家", "公務", "員", "食べた"]
    );
    assert_eq!(
        ruby.iter().map(|r| r.reading.as_str()).collect::<Vec<_>>(),
        [
            original.tokens[0].reading.as_deref().unwrap(),
            alternates[0].1.tokens[1].reading.as_deref().unwrap(),
            alternates[0].1.tokens[2].reading.as_deref().unwrap(),
            alternates[1].1.tokens[1].reading.as_deref().unwrap(),
        ]
    );
    assert_retained_ruby(
        &chapter,
        &effective,
        anchor(&chapter, 0, chapter.text.len()),
        None,
        &ruby,
    );
    assert_eq!((original, alternates, rules), before);
}

#[test]
fn source_suppresses_whole_overlapping_effective_tokens_and_covers_unresolved_spans() {
    let chapter = chapter("猫東京大学犬 食べた");
    let original = analysis(
        &chapter,
        SplitMode::B,
        &["猫", "東京", "大学", "犬", "食べ", "た"],
    );
    let rules = [rule(&chapter, 1, "食べた", &["食べた"])];
    let effective =
        EffectiveReaderChapter::resolve(&chapter, 10.try_into().unwrap(), &original, &[], &rules)
            .unwrap();
    let source = ReaderChapterRuby {
        chapter_id: chapter.chapter_id,
        ruby: vec![
            ReaderRuby {
                range: range(6, 12),
                reading: "部分ソース𠮷\0".into(),
            },
            ReaderRuby {
                range: range(19, 28),
                reading: "たべた".into(),
            },
        ],
    };
    let before = (effective.clone(), source.clone());
    let ruby = effective
        .furigana_on_page(anchor(&chapter, 0, chapter.text.len()), Some(&source))
        .unwrap();
    assert_eq!(
        ruby,
        vec![
            ReaderRuby {
                range: range(0, 3),
                reading: original.tokens[0].reading.clone().unwrap()
            },
            source.ruby[0].clone(),
            ReaderRuby {
                range: range(15, 18),
                reading: original.tokens[3].reading.clone().unwrap()
            },
            source.ruby[1].clone(),
        ]
    );
    assert_retained_ruby(
        &chapter,
        &effective,
        anchor(&chapter, 0, chapter.text.len()),
        Some(&source),
        &ruby,
    );
    assert_eq!((effective, source), before);
}

#[test]
fn repeated_spellings_use_each_occurrences_metadata_and_reversal_restores_b() {
    let chapter = chapter("猫犬 猫犬");
    let original = analysis(&chapter, SplitMode::B, &["猫犬", "猫犬"]);
    let alternate = analysis(&chapter, SplitMode::A, &["猫", "犬", "猫", "犬"]);
    let mut rules = [rule(&chapter, 1, "猫犬", &["猫", "犬"])];
    let alternates = [(11.try_into().unwrap(), alternate.clone())];
    let page = anchor(&chapter, 0, chapter.text.len());
    let effective = EffectiveReaderChapter::resolve(
        &chapter,
        10.try_into().unwrap(),
        &original,
        &alternates,
        &rules,
    )
    .unwrap();
    let split = effective.furigana_on_page(page, None).unwrap();
    assert_eq!(
        split
            .iter()
            .map(|r| Some(r.reading.clone()))
            .collect::<Vec<_>>(),
        alternate
            .tokens
            .iter()
            .map(|t| t.reading.clone())
            .collect::<Vec<_>>()
    );
    assert_retained_ruby(&chapter, &effective, page, None, &split);
    rules[0].active = false;
    let reversed = EffectiveReaderChapter::resolve(
        &chapter,
        10.try_into().unwrap(),
        &original,
        &alternates,
        &rules,
    )
    .unwrap();
    let ruby = reversed.furigana_on_page(page, None).unwrap();
    assert_eq!(
        ruby.iter()
            .map(|r| Some(r.reading.clone()))
            .collect::<Vec<_>>(),
        original
            .tokens
            .iter()
            .map(|t| t.reading.clone())
            .collect::<Vec<_>>()
    );
    assert_retained_ruby(&chapter, &reversed, page, None, &ruby);
    // The old validated resolution stays frozen until the caller refreshes it.
    assert_eq!(effective.furigana_on_page(page, None).unwrap(), split);
}

#[test]
fn crossing_effective_and_source_bases_keep_complete_readings_at_every_preset() {
    let half = "𠮷か\u{3099}\0猫。".repeat(400);
    let chapter = chapter(&half.repeat(2));
    let original = analysis(&chapter, SplitMode::B, &[&half, &half]);
    let alternate = analysis(&chapter, SplitMode::C, &[&chapter.text]);
    let rules = [rule(&chapter, 1, &chapter.text, &[&chapter.text])];
    let alternates = [(11.try_into().unwrap(), alternate.clone())];
    let effective = EffectiveReaderChapter::resolve(
        &chapter,
        10.try_into().unwrap(),
        &original,
        &alternates,
        &rules,
    )
    .unwrap();
    let source = ReaderChapterRuby {
        chapter_id: chapter.chapter_id,
        ruby: vec![ReaderRuby {
            range: range(0, chapter.text.len()),
            reading: "完全ソース𠮷か\u{3099}\0".into(),
        }],
    };
    let document = PastedDocument {
        document_id: 1.try_into().unwrap(),
        title: "本".into(),
        finished_at_least_once: false,
        chapter: chapter.clone(),
        reading_position: None,
    };
    for length in PageLength::ALL {
        for source in [None, Some(&source)] {
            let mut page = ReaderPage::open(&document, length).unwrap();
            let mut text = String::new();
            loop {
                let ruby = effective.furigana_on_page(page.anchor, source).unwrap();
                assert_retained_ruby(&chapter, &effective, page.anchor, source, &ruby);
                assert_eq!(ruby.len(), 1);
                assert_eq!(
                    ruby[0].reading,
                    source
                        .map(|s| &s.ruby[0].reading)
                        .unwrap_or_else(|| alternate.tokens[0].reading.as_ref().unwrap())
                        .as_str()
                );
                assert_eq!(ruby[0].range, range(0, page.text.len()));
                for offset in [ruby[0].range.start, ruby[0].range.end] {
                    let utf16 = offset.to_utf16(&page.text).unwrap();
                    assert_eq!(TextOffset::from_utf16(&page.text, utf16).unwrap(), offset);
                }
                text.push_str(ruby[0].range.slice(&page.text).unwrap());
                if page.page_number == page.total_pages {
                    break;
                }
                let mut position = page.position;
                position.offset = page.anchor.range.end;
                page = ReaderPage::at(&document, position, length).unwrap();
            }
            assert_eq!(text, chapter.text);
        }
    }
}

#[test]
fn invalid_pages_and_whole_source_ruby_fail_without_partial_output() {
    let chapter = chapter("𠮷猫犬。");
    let original = analysis(&chapter, SplitMode::B, &["𠮷", "猫", "犬", "。"]);
    let effective =
        EffectiveReaderChapter::resolve(&chapter, 10.try_into().unwrap(), &original, &[], &[])
            .unwrap();
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
