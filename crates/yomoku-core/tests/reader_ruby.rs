use yomoku_core::{
    document::{ChapterText, PastedDocument, ReaderDocument, TextOffset, TextRange},
    pagination::{LogicalPages, PageLength, ReadingPosition},
    reader::ReaderPage,
    reader_ruby::{ReaderChapterRuby, ReaderRuby},
};

fn range(text: &str, start: usize, end: usize) -> TextRange {
    TextRange {
        start: TextOffset::at(text, start).unwrap(),
        end: TextOffset::at(text, end).unwrap(),
    }
}

fn document() -> ReaderDocument {
    PastedDocument {
        document_id: 1.try_into().unwrap(),
        title: "本".into(),
        finished_at_least_once: false,
        chapter: ChapterText {
            source_id: 2.try_into().unwrap(),
            chapter_id: 3.try_into().unwrap(),
            text: format!("😀{}𠮷か\u{3099}\0終", "漢字".repeat(1600)),
        },
        reading_position: None,
    }
    .into()
}

#[test]
fn crossing_source_ruby_clips_at_every_preset_without_moving_text_or_saved_offset() {
    let mut document = document();
    let chapter = &document.chapters[0];
    let base = range(&chapter.text, "😀".len(), chapter.text.find('\0').unwrap());
    document.source_ruby = vec![ReaderChapterRuby {
        chapter_id: chapter.chapter_id,
        ruby: vec![ReaderRuby {
            range: base,
            reading: "ソース𠮷か\u{3099}\0よみ".into(),
        }],
    }];
    let mut plain = document.clone();
    plain.source_ruby.clear();
    for length in PageLength::ALL {
        let pages = LogicalPages::new(&document.chapters, length).unwrap();
        let mut reconstructed = String::new();
        let mut annotated = String::new();
        for anchor in pages.pages() {
            let position = ReadingPosition {
                source_id: anchor.source_id,
                chapter_id: anchor.chapter_id,
                offset: anchor.range.start,
            };
            let page = ReaderPage::at_document(&document, position, length).unwrap();
            let unannotated = ReaderPage::at_document(&plain, position, length).unwrap();
            assert_eq!(page.text, unannotated.text);
            assert_eq!(page.anchor, unannotated.anchor);
            assert_eq!(page.position, unannotated.position);
            assert_eq!(page.page_number, unannotated.page_number);
            assert_eq!(page.total_pages, unannotated.total_pages);
            for ruby in &page.source_ruby {
                assert_eq!(ruby.reading, document.source_ruby[0].ruby[0].reading);
                annotated.push_str(ruby.range.slice(&page.text).unwrap());
            }
            reconstructed.push_str(&page.text);
        }
        assert_eq!(reconstructed, document.chapters[0].text);
        assert_eq!(annotated, base.slice(&document.chapters[0].text).unwrap());
        let exact = ReadingPosition {
            source_id: document.chapters[0].source_id,
            chapter_id: document.chapters[0].chapter_id,
            offset: base.end,
        };
        document.reading_position = Some(exact);
        assert_eq!(
            ReaderPage::open_document(&document, length)
                .unwrap()
                .position,
            exact
        );
    }
}

#[test]
fn adjacent_ruby_is_ordered_and_absent_from_other_chapters_and_boundary_only_pages() {
    let mut document = document();
    let first = &document.chapters[0];
    let start = first.text.find('漢').unwrap();
    let ruby = ReaderChapterRuby {
        chapter_id: first.chapter_id,
        ruby: vec![
            ReaderRuby {
                range: range(&first.text, start, start + 3),
                reading: "かん".into(),
            },
            ReaderRuby {
                range: range(&first.text, start + 3, start + 6),
                reading: "じ".into(),
            },
        ],
    };
    assert!(ruby
        .on_page(first, range(&first.text, 0, start))
        .unwrap()
        .is_empty());
    assert!(ruby
        .on_page(first, range(&first.text, start + 6, first.text.len()))
        .unwrap()
        .is_empty());
    let page = ruby
        .on_page(first, range(&first.text, start, start + 6))
        .unwrap();
    assert_eq!(
        page.iter()
            .map(|r| (r.range.slice("漢字").unwrap(), r.reading.as_str()))
            .collect::<Vec<_>>(),
        [("漢", "かん"), ("字", "じ")]
    );
    let second = ChapterText {
        chapter_id: 4.try_into().unwrap(),
        ..first.clone()
    };
    document.reading_position = Some(ReadingPosition {
        source_id: second.source_id,
        chapter_id: second.chapter_id,
        offset: TextOffset::START,
    });
    document.chapters.push(second);
    document.source_ruby = vec![ruby];
    assert!(ReaderPage::open_document(&document, PageLength::default())
        .unwrap()
        .source_ruby
        .is_empty());
}

#[test]
fn invalid_ruby_rejects_empty_overlapping_unordered_foreign_and_non_scalar_ranges() {
    let mut document = document();
    let chapter = &document.chapters[0];
    let valid = ReaderChapterRuby {
        chapter_id: chapter.chapter_id,
        ruby: vec![ReaderRuby {
            range: range(&chapter.text, 4, 7),
            reading: "かん".into(),
        }],
    };
    for (start, end) in [(0, 0), (1, 4), (4, 6), (7, 4), (0, i64::MAX)] {
        let mut invalid = valid.clone();
        invalid.ruby[0].range = TextRange {
            start: start.try_into().unwrap(),
            end: end.try_into().unwrap(),
        };
        document.source_ruby = vec![invalid];
        assert!(document.validate().is_err());
    }
    let mut empty = valid.clone();
    empty.ruby[0].reading.clear();
    document.source_ruby = vec![empty];
    assert!(document.validate().is_err());
    let mut overlapping = valid.clone();
    overlapping.ruby.push(overlapping.ruby[0].clone());
    document.source_ruby = vec![overlapping];
    assert!(document.validate().is_err());
    let mut unordered = valid.clone();
    unordered.ruby.insert(
        0,
        ReaderRuby {
            range: range(&document.chapters[0].text, 7, 10),
            reading: "じ".into(),
        },
    );
    document.source_ruby = vec![unordered];
    assert!(document.validate().is_err());
    document.source_ruby = vec![valid.clone(), valid.clone()];
    assert!(document.validate().is_err());
    document.source_ruby = vec![ReaderChapterRuby {
        chapter_id: 4.try_into().unwrap(),
        ..valid.clone()
    }];
    assert!(document.validate().is_err());
    assert!(valid
        .on_page(
            &document.chapters[0],
            TextRange {
                start: 1.try_into().unwrap(),
                end: 4.try_into().unwrap()
            }
        )
        .is_err());
    assert!(valid
        .on_page(
            &ChapterText {
                chapter_id: 4.try_into().unwrap(),
                ..document.chapters[0].clone()
            },
            range(&document.chapters[0].text, 0, 4)
        )
        .is_err());
}
