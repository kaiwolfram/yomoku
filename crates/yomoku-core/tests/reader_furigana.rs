use yomoku_core::{
    document::{ChapterText, PastedDocument, TextOffset, TextRange},
    pagination::PageLength,
    reader::{ReaderAnalysis, ReaderPage, ReaderToken, ReaderTokenSnapshot, ReaderVocabulary},
    reader_ruby::ReaderRuby,
    vocabulary::VocabularyStatus,
};

fn range(start: usize, end: usize) -> TextRange {
    TextRange {
        start: TextOffset::try_from(start as i64).unwrap(),
        end: TextOffset::try_from(end as i64).unwrap(),
    }
}

fn document(text: &str) -> PastedDocument {
    PastedDocument {
        document_id: 1.try_into().unwrap(),
        title: "本".into(),
        finished_at_least_once: false,
        chapter: ChapterText {
            source_id: 2.try_into().unwrap(),
            chapter_id: 3.try_into().unwrap(),
            text: text.into(),
        },
        reading_position: None,
    }
}

fn analysis() -> ReaderAnalysis {
    ReaderAnalysis::Available(4.try_into().unwrap())
}

fn snapshot(page: &ReaderPage) -> ReaderTokenSnapshot {
    ReaderTokenSnapshot {
        document_id: page.document_id,
        page: page.anchor,
        analysis: analysis(),
        tokens: Vec::new(),
    }
}

fn token(page: &ReaderPage, index: u64, start: usize, end: usize) -> ReaderToken {
    let mut anchor = page.anchor;
    anchor.range = range(
        i64::from(anchor.range.start) as usize + start,
        i64::from(anchor.range.start) as usize + end,
    );
    ReaderToken {
        token_index: index,
        anchor,
        display_range: range(start, end),
        surface: page.text[start..end].into(),
        reading: Some(format!("よみ{index}𠮷か\u{3099}\0")),
        dictionary_form: None,
        vocabulary: None,
    }
}

#[test]
fn source_wins_over_partial_and_multiple_tokens_and_status_never_filters_readings() {
    let mut page =
        ReaderPage::open(&document("猫東京大学犬かな終"), PageLength::default()).unwrap();
    let source = ReaderRuby {
        // Crosses two tokens, starting/ending inside each; do not split their readings.
        range: range(6, 12),
        reading: "ソースとうきょう\0𠮷".into(),
    };
    page.source_ruby = vec![source.clone()];
    let mut snapshot = snapshot(&page);
    snapshot.tokens = vec![
        token(&page, 0, 0, 3),
        token(&page, 1, 3, 9),
        token(&page, 2, 9, 15),
        token(&page, 3, 15, 18),
        token(&page, 4, 18, 24),
        token(&page, 5, 24, 27),
    ];
    snapshot.tokens[5].reading = None;
    let expected = vec![
        ReaderRuby {
            range: range(0, 3),
            reading: snapshot.tokens[0].reading.clone().unwrap(),
        },
        source,
        ReaderRuby {
            range: range(15, 18),
            reading: snapshot.tokens[3].reading.clone().unwrap(),
        },
        ReaderRuby {
            range: range(18, 24),
            reading: snapshot.tokens[4].reading.clone().unwrap(),
        },
    ];
    for status in [
        None,
        Some(VocabularyStatus::New),
        Some(VocabularyStatus::Learning),
        Some(VocabularyStatus::Familiar),
        Some(VocabularyStatus::Known),
        Some(VocabularyStatus::Ignored),
    ] {
        for token in &mut snapshot.tokens {
            token.vocabulary = status.map(|status| ReaderVocabulary {
                id: 7.try_into().unwrap(),
                status,
            });
        }
        let before = (page.clone(), snapshot.clone());
        assert_eq!(
            page.furigana(analysis(), Some(&snapshot)).unwrap(),
            expected
        );
        assert_eq!((page.clone(), snapshot.clone()), before);
    }
    assert_eq!(page.furigana(analysis(), None).unwrap(), page.source_ruby);
    assert_eq!(
        page.furigana(ReaderAnalysis::Missing, None).unwrap(),
        page.source_ruby
    );
    let missing = ReaderTokenSnapshot {
        analysis: ReaderAnalysis::Missing,
        tokens: vec![],
        ..snapshot
    };
    assert_eq!(
        page.furigana(ReaderAnalysis::Missing, Some(&missing))
            .unwrap(),
        page.source_ruby
    );
}

#[test]
fn crossing_analyzer_bases_keep_full_readings_and_exact_unicode_anchors_at_every_preset() {
    let document = document(&format!(
        "{}\n{}",
        "𠮷か\u{3099}猫。".repeat(400),
        "犬。".repeat(800)
    ));
    for length in PageLength::ALL {
        let mut position = ReaderPage::open(&document, length).unwrap().position;
        let mut reconstructed = String::new();
        loop {
            let page = ReaderPage::at(&document, position, length).unwrap();
            let mut snapshot = snapshot(&page);
            snapshot.tokens = vec![ReaderToken {
                anchor: yomoku_core::document::TextAnchor {
                    range: range(0, document.chapter.text.len()),
                    ..page.anchor
                },
                surface: document.chapter.text.clone(),
                ..token(&page, 0, 0, page.text.len())
            }];
            let before = page.clone();
            let ruby = page.furigana(analysis(), Some(&snapshot)).unwrap();
            assert_eq!(ruby.len(), 1);
            assert_eq!(ruby[0].reading, snapshot.tokens[0].reading.clone().unwrap());
            reconstructed.push_str(ruby[0].range.slice(&page.text).unwrap());
            assert_eq!(page, before);
            if page.page_number == page.total_pages {
                break;
            }
            position.offset = page.anchor.range.end;
        }
        assert_eq!(reconstructed, document.chapter.text);
    }
}

#[test]
fn stale_and_malformed_analyzer_snapshots_fall_back_to_source_without_partial_results() {
    let mut page = ReaderPage::open(&document("😀猫𠮷犬終"), PageLength::default()).unwrap();
    page.source_ruby = vec![ReaderRuby {
        range: range(14, 17),
        reading: "おわり".into(),
    }];
    let mut valid = snapshot(&page);
    valid.tokens = vec![token(&page, 1, 4, 7), token(&page, 3, 7, 11)];
    assert_eq!(page.furigana(analysis(), Some(&valid)).unwrap().len(), 3);
    let mut invalid = Vec::new();
    let mut bad = valid.clone();
    bad.document_id = 99.try_into().unwrap();
    invalid.push(bad);
    let mut bad = valid.clone();
    bad.page.chapter_id = 99.try_into().unwrap();
    invalid.push(bad);
    let mut bad = valid.clone();
    bad.page.source_id = 99.try_into().unwrap();
    invalid.push(bad);
    let mut bad = valid.clone();
    bad.page.range.start = 4.try_into().unwrap();
    invalid.push(bad);
    let mut bad = valid.clone();
    bad.analysis = ReaderAnalysis::Missing;
    invalid.push(bad);
    let mut bad = valid.clone();
    bad.analysis = ReaderAnalysis::Available(99.try_into().unwrap());
    invalid.push(bad);
    for mutate in [
        (|token: &mut ReaderToken| token.anchor.chapter_id = 99.try_into().unwrap())
            as fn(&mut ReaderToken),
        |token| token.anchor.source_id = 99.try_into().unwrap(),
        |token| token.display_range = range(8, 11), // inside supplementary scalar
        |token| token.display_range = range(7, 10),
        |token| token.display_range = range(7, 7),
        |token| token.display_range = range(7, 100),
        |token| token.display_range = range(11, 7),
        |token| token.anchor.range = range(7, 10),
        |token| token.anchor.range = range(11, 7),
        |token| token.surface = "😀".into(), // valid length, wrong canonical text
        |token| token.reading = Some(" \n".into()),
        |token| token.token_index = 1,
    ] {
        let mut bad = valid.clone();
        mutate(&mut bad.tokens[1]);
        invalid.push(bad);
    }
    let mut bad = valid.clone();
    bad.tokens.reverse();
    invalid.push(bad);
    let mut bad = valid.clone();
    bad.tokens.push(bad.tokens[1].clone());
    invalid.push(bad);
    for bad in invalid {
        assert_eq!(
            page.furigana(analysis(), Some(&bad)).unwrap(),
            page.source_ruby,
            "{bad:?}"
        );
    }
    assert_eq!(
        page.furigana(ReaderAnalysis::Missing, Some(&valid))
            .unwrap(),
        page.source_ruby
    );
    page.source_ruby[0].range = range(8, 11);
    assert!(page.furigana(analysis(), Some(&valid)).is_err());
}
