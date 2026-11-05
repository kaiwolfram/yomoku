use yomoku_core::{
    completion::PageCompletion,
    document::{ChapterText, PastedDocument, ReaderDocument, TextOffset},
    pagination::{LogicalPages, PageLength, ReadingPosition},
    reader::ReaderPage,
};

fn document() -> ReaderDocument {
    ReaderDocument {
        document_id: 7.try_into().unwrap(),
        title: "本".into(),
        finished_at_least_once: false,
        // Deliberately not ID-sorted: source order is authoritative.
        chapters: [
            (30, format!("{}\n𠮷 か\u{3099}\0終", "猫。".repeat(900))),
            (20, "犬 犬".into()),
        ]
        .map(|(id, text)| ChapterText {
            source_id: 4.try_into().unwrap(),
            chapter_id: id.try_into().unwrap(),
            text,
        })
        .into(),
        reading_position: None,
        formatting: Vec::new(),
        images: Vec::new(),
        source_ruby: Vec::new(),
    }
}

fn position(document: &ReaderDocument, chapter: usize, offset: usize) -> ReadingPosition {
    let chapter = &document.chapters[chapter];
    ReadingPosition {
        source_id: chapter.source_id,
        chapter_id: chapter.chapter_id,
        offset: TextOffset::at(&chapter.text, offset).unwrap(),
    }
}

fn completion(document: &ReaderDocument, length: PageLength) -> PageCompletion {
    let page = ReaderPage::open_document(document, length).unwrap();
    PageCompletion {
        command_id: 1.try_into().unwrap(),
        document_id: document.document_id,
        page: page.anchor,
        page_length: length,
        departing_position: page.position,
        completed_at_unix_ms: 100,
    }
}

#[test]
fn chapter_boundaries_global_counters_and_exact_reflow_work_for_all_presets() {
    let mut document = document();
    let exact = position(&document, 0, document.chapters[0].text.len() - "終".len());
    document.reading_position = Some(exact);
    let expected_summary = document.library_summary().unwrap();
    for from in PageLength::ALL {
        for to in PageLength::ALL {
            let pages = LogicalPages::new(&document.chapters, from).unwrap();
            let reflow = pages.reflow(to, exact).unwrap();
            let page = ReaderPage::open_document(&document, to).unwrap();
            assert_eq!(page.position, exact);
            assert_eq!(page.anchor, reflow.pages.pages()[reflow.page_index]);
            assert_eq!(page.page_number + 1, page.total_pages);
            assert_eq!(page.anchor.chapter_id, document.chapters[0].chapter_id);
            assert!(page.text.ends_with("𠮷 か\u{3099}\0終"));
            let next = completion(&document, to)
                .next_document_position(&document)
                .unwrap();
            assert_eq!(next, position(&document, 1, 0));
            let last = ReaderPage::at_document(&document, next, to).unwrap();
            assert_eq!(last.page_number, last.total_pages);
            assert_eq!(last.text, "犬 犬");
            assert_eq!(document.library_summary().unwrap(), expected_summary);
        }
    }
}

#[test]
fn forward_traversal_visits_every_page_and_only_whole_book_end_stays_at_eof() {
    for length in PageLength::ALL {
        let mut document = document();
        let expected = LogicalPages::new(&document.chapters, length)
            .unwrap()
            .pages()
            .to_vec();
        for (index, anchor) in expected.iter().enumerate() {
            let page = ReaderPage::open_document(&document, length).unwrap();
            assert_eq!(&page.anchor, anchor);
            assert_eq!(page.page_number, index as u64 + 1);
            let next = completion(&document, length)
                .next_document_position(&document)
                .unwrap();
            document.reading_position = Some(next);
        }
        assert_eq!(
            document.reading_position,
            Some(position(&document, 1, "犬 犬".len()))
        );
        assert_eq!(document.library_summary().unwrap().progress_percent, 100);
        document.reading_position = Some(position(&document, 0, 0));
        assert_eq!(document.library_summary().unwrap().progress_percent, 0);
    }
}

#[test]
fn invalid_ownership_units_offsets_and_stale_completion_are_rejected() {
    let original = document();
    for case in 0..6 {
        let mut bad = original.clone();
        match case {
            0 => bad.chapters.clear(),
            1 => bad.chapters[1].chapter_id = bad.chapters[0].chapter_id,
            2 => bad.chapters[1].source_id = 9.try_into().unwrap(),
            3 => {
                bad.reading_position = Some(ReadingPosition {
                    chapter_id: 99.try_into().unwrap(),
                    ..position(&bad, 0, 0)
                })
            }
            4 => {
                bad.reading_position = Some(ReadingPosition {
                    offset: 1.try_into().unwrap(),
                    ..position(&bad, 0, 0)
                })
            }
            _ => {
                bad.reading_position = Some(ReadingPosition {
                    source_id: 9.try_into().unwrap(),
                    ..position(&bad, 0, 0)
                })
            }
        }
        assert!(bad.validate().is_err());
        assert!(bad.library_summary().is_err());
        assert!(ReaderPage::open_document(&bad, PageLength::default()).is_err());
    }
    for case in 0..5 {
        let mut command = completion(&original, PageLength::default());
        match case {
            0 => command.document_id = 99.try_into().unwrap(),
            1 => command.page.chapter_id = original.chapters[1].chapter_id,
            2 => command.departing_position = position(&original, 1, 0),
            3 => command.completed_at_unix_ms = -1,
            _ => command.page.range.end = 1.try_into().unwrap(),
        }
        assert!(command.next_document_position(&original).is_err());
    }
}

#[test]
fn single_chapter_adapter_preserves_legacy_projection_and_completion() {
    let original = document();
    let paste = PastedDocument {
        document_id: original.document_id,
        title: original.title,
        finished_at_least_once: true,
        chapter: original.chapters[0].clone(),
        reading_position: None,
    };
    let mut document = ReaderDocument::from(paste.clone());
    for length in PageLength::ALL {
        let pages = LogicalPages::new(&document.chapters, length)
            .unwrap()
            .pages()
            .to_vec();
        for anchor in pages {
            let current = ReadingPosition {
                source_id: anchor.source_id,
                chapter_id: anchor.chapter_id,
                offset: anchor.range.start,
            };
            document.reading_position = Some(current);
            let paste = PastedDocument {
                reading_position: Some(current),
                ..paste.clone()
            };
            assert_eq!(
                ReaderPage::open_document(&document, length).unwrap(),
                ReaderPage::open(&paste, length).unwrap()
            );
            assert_eq!(
                document.library_summary().unwrap(),
                paste.library_summary().unwrap()
            );
            let command = completion(&document, length);
            assert_eq!(
                command.next_document_position(&document),
                command.next_position(&paste)
            );
        }
    }
}

#[test]
fn empty_final_unit_is_a_separate_page_after_previous_chapter_eof() {
    let mut document = document();
    document.chapters[1].text.clear();
    document.reading_position = Some(position(&document, 0, document.chapters[0].text.len()));
    let length = PageLength::default();
    let next = completion(&document, length)
        .next_document_position(&document)
        .unwrap();
    assert_eq!(next, position(&document, 1, 0));
    document.reading_position = Some(next);
    let page = ReaderPage::open_document(&document, length).unwrap();
    assert_eq!(page.page_number, page.total_pages);
    assert!(page.text.is_empty());
    assert_eq!(
        completion(&document, length)
            .next_document_position(&document)
            .unwrap(),
        next
    );
}
