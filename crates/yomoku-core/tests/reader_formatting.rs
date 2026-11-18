use yomoku_core::{
    document::{ChapterText, PastedDocument, ReaderDocument, TextOffset, TextRange},
    pagination::{LogicalPages, PageLength, ReadingPosition},
    reader::ReaderPage,
    reader_formatting::{ReaderChapterFormatting, ReaderTextSpan, ReaderTextStyle},
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
        read_override: None,
        chapter: ChapterText {
            source_id: 2.try_into().unwrap(),
            chapter_id: 3.try_into().unwrap(),
            text: format!("{}\n😀𠮷か\u{3099}\0終", "猫。".repeat(900)),
        },
        reading_position: None,
    }
    .into()
}

#[test]
fn formatting_clips_across_all_pages_and_presets_without_changing_text_or_position() {
    let mut document = document();
    let chapter = &document.chapters[0];
    let whole = range(&chapter.text, 0, chapter.text.len());
    let tail_start = chapter.text.find('😀').unwrap();
    let tail = range(&chapter.text, tail_start, chapter.text.len());
    document.formatting = vec![ReaderChapterFormatting {
        chapter_id: chapter.chapter_id,
        spans: [
            (whole, ReaderTextStyle::Heading(1)),
            (whole, ReaderTextStyle::Emphasis),
            (tail, ReaderTextStyle::Strong),
            (
                whole,
                ReaderTextStyle::ExternalLink("HTTPS://example.org/本?q=1&x=2".into()),
            ),
        ]
        .map(|(range, style)| ReaderTextSpan { range, style })
        .into(),
    }];
    let mut plain = document.clone();
    plain.formatting.clear();
    for length in PageLength::ALL {
        let pages = LogicalPages::new(&document.chapters, length).unwrap();
        let mut reconstructed = String::new();
        for anchor in pages.pages() {
            let position = ReadingPosition {
                source_id: anchor.source_id,
                chapter_id: anchor.chapter_id,
                offset: anchor.range.start,
            };
            let page = ReaderPage::at_document(&document, position, length).unwrap();
            let unformatted = ReaderPage::at_document(&plain, position, length).unwrap();
            assert_eq!(page.text, unformatted.text);
            assert_eq!(page.anchor, unformatted.anchor);
            assert_eq!(page.position, unformatted.position);
            assert_eq!(page.page_number, unformatted.page_number);
            assert_eq!(page.total_pages, unformatted.total_pages);
            for span in &page.formatting {
                assert!(!span.range.slice(&page.text).unwrap().is_empty());
            }
            assert_eq!(
                page.formatting.len(),
                if anchor.range.end > tail.start { 4 } else { 3 }
            );
            assert_eq!(
                page.formatting[0].range,
                range(&page.text, 0, page.text.len())
            );
            reconstructed.push_str(&page.text);
        }
        assert_eq!(reconstructed, document.chapters[0].text);
        document.reading_position = Some(ReadingPosition {
            source_id: document.chapters[0].source_id,
            chapter_id: document.chapters[0].chapter_id,
            offset: tail.start,
        });
        assert_eq!(
            ReaderPage::open_document(&document, length)
                .unwrap()
                .position
                .offset,
            tail.start
        );
    }
}

#[test]
fn formatting_is_keyed_by_chapter_and_never_leaks_into_later_chapters() {
    let mut document = document();
    let first = &document.chapters[0];
    document.formatting = vec![ReaderChapterFormatting {
        chapter_id: first.chapter_id,
        spans: vec![ReaderTextSpan {
            range: range(&first.text, 0, first.text.len()),
            style: ReaderTextStyle::Strong,
        }],
    }];
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
    assert!(ReaderPage::open_document(&document, PageLength::default())
        .unwrap()
        .formatting
        .is_empty());
}

#[test]
fn invalid_formatting_rejects_foreign_duplicate_scalar_and_website_inputs() {
    let mut document = document();
    let chapter = &document.chapters[0];
    let valid = ReaderChapterFormatting {
        chapter_id: chapter.chapter_id,
        spans: vec![ReaderTextSpan {
            range: range(&chapter.text, 0, 3),
            style: ReaderTextStyle::Strong,
        }],
    };
    for style in [
        ReaderTextStyle::Heading(0),
        ReaderTextStyle::Heading(7),
        ReaderTextStyle::ExternalLink("file:///etc/passwd".into()),
        ReaderTextStyle::ExternalLink("javascript:alert(1)".into()),
        ReaderTextStyle::ExternalLink("#note".into()),
        ReaderTextStyle::ExternalLink("https:///book".into()),
        ReaderTextStyle::ExternalLink("https://".into()),
        ReaderTextStyle::ExternalLink("https://example.org/\n".into()),
    ] {
        let mut invalid = valid.clone();
        invalid.spans[0].style = style;
        document.formatting = vec![invalid];
        assert!(document.validate().is_err());
    }
    for (start, end) in [(0, 0), (1, 3), (0, 2), (3, 0), (0, i64::MAX)] {
        let mut invalid = valid.clone();
        invalid.spans[0].range = TextRange {
            start: start.try_into().unwrap(),
            end: end.try_into().unwrap(),
        };
        document.formatting = vec![invalid];
        assert!(document.validate().is_err());
    }
    document.formatting = vec![valid.clone(), valid.clone()];
    assert!(document.validate().is_err());
    document.formatting = vec![ReaderChapterFormatting {
        chapter_id: 4.try_into().unwrap(),
        ..valid
    }];
    assert!(document.validate().is_err());
}
