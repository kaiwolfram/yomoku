use yomoku_core::{
    document::{ChapterText, PastedDocument, ReaderDocument, TextOffset, TextRange},
    pagination::{LogicalPages, PageLength, ReadingPosition},
    reader::ReaderPage,
    reader_images::{ReaderChapterImages, ReaderImageAsset, ReaderInlineImage},
};

fn range(text: &str, start: usize, end: usize) -> TextRange {
    TextRange {
        start: TextOffset::at(text, start).unwrap(),
        end: TextOffset::at(text, end).unwrap(),
    }
}

fn document() -> ReaderDocument {
    let mut document: ReaderDocument = PastedDocument {
        document_id: 1.try_into().unwrap(),
        title: "本".into(),
        finished_at_least_once: false,
        read_override: None,
        chapter: ChapterText {
            source_id: 2.try_into().unwrap(),
            chapter_id: 3.try_into().unwrap(),
            // Paragraph boundaries put adjacent images on opposite pages for
            // the 250 preset. The tail also exercises nonzero page-local offsets.
            text: format!(
                "{}\u{fffc}\n\u{fffc}😀𠮷か\u{3099}\0{}\u{fffc}",
                "猫".repeat(249),
                "犬。".repeat(900)
            ),
        },
        reading_position: None,
    }
    .into();
    let first = &document.chapters[0];
    let images = first
        .text
        .match_indices('\u{fffc}')
        .map(|(offset, _)| ReaderInlineImage {
            range: range(&first.text, offset, offset + '\u{fffc}'.len_utf8()),
            asset: ReaderImageAsset {
                path: "OPS/images/挿絵😀.png".into(),
                media_type: "image/png".into(),
            },
            alt: "挿絵 𠮷か\u{3099}\0".into(),
        })
        .collect();
    document.images = vec![ReaderChapterImages {
        chapter_id: first.chapter_id,
        images,
    }];
    document
}

#[test]
fn page_images_preserve_positions_assets_and_text_for_every_preset() {
    let document = document();
    let mut plain = document.clone();
    plain.images.clear();
    for length in PageLength::ALL {
        let pages = LogicalPages::new(&document.chapters, length).unwrap();
        let mut images = Vec::new();
        let mut text = String::new();
        for anchor in pages.pages() {
            let position = ReadingPosition {
                source_id: anchor.source_id,
                chapter_id: anchor.chapter_id,
                offset: anchor.range.start,
            };
            let page = ReaderPage::at_document(&document, position, length).unwrap();
            let unillustrated = ReaderPage::at_document(&plain, position, length).unwrap();
            assert_eq!(page.text, unillustrated.text);
            assert_eq!(page.anchor, unillustrated.anchor);
            assert_eq!(page.position, unillustrated.position);
            assert_eq!(page.page_number, unillustrated.page_number);
            assert_eq!(page.total_pages, unillustrated.total_pages);
            for image in &page.images {
                assert_eq!(image.range.slice(&page.text).unwrap(), "\u{fffc}");
                let mut absolute = image.clone();
                absolute.range.start = (i64::from(image.range.start)
                    + i64::from(anchor.range.start))
                .try_into()
                .unwrap();
                absolute.range.end = (i64::from(image.range.end) + i64::from(anchor.range.start))
                    .try_into()
                    .unwrap();
                images.push(absolute);
            }
            text.push_str(&page.text);
        }
        assert_eq!(text, document.chapters[0].text);
        assert_eq!(images, document.images[0].images);
        assert_eq!(document.library_summary(), plain.library_summary());
        if length == PageLength::Characters250 {
            let first = ReaderPage::open_document(&document, length).unwrap();
            assert_eq!(first.images.len(), 1);
            assert!(first.text.ends_with("\u{fffc}\n"));
        }
    }
}

#[test]
fn exact_image_position_survives_all_reflow_pairs_and_eof() {
    let mut document = document();
    let chapter = &document.chapters[0];
    for offset in document.images[0]
        .images
        .iter()
        .map(|image| image.range.start)
        .chain([TextOffset::at(&chapter.text, chapter.text.len()).unwrap()])
        .collect::<Vec<_>>()
    {
        let position = ReadingPosition {
            source_id: chapter.source_id,
            chapter_id: chapter.chapter_id,
            offset,
        };
        document.reading_position = Some(position);
        for from in PageLength::ALL {
            for to in PageLength::ALL {
                let pages = LogicalPages::new(&document.chapters, from).unwrap();
                let reflow = pages.reflow(to, position).unwrap();
                let page = ReaderPage::open_document(&document, to).unwrap();
                assert_eq!(page.position, position);
                assert_eq!(page.anchor, reflow.pages.pages()[reflow.page_index]);
                if offset.byte_index(&chapter.text).unwrap() < chapter.text.len() {
                    assert!(page.images.iter().any(|image| i64::from(image.range.start)
                        + i64::from(page.anchor.range.start)
                        == i64::from(offset)));
                }
            }
        }
    }
}

#[test]
fn image_only_chapters_adjacent_images_and_shared_assets_keep_source_order() {
    let mut document = document();
    let chapter = ChapterText {
        chapter_id: 4.try_into().unwrap(),
        text: "\u{fffc}\u{fffc}".into(),
        ..document.chapters[0].clone()
    };
    let image = &document.images[0].images[0];
    document.images.insert(
        0,
        ReaderChapterImages {
            chapter_id: chapter.chapter_id,
            images: vec![
                ReaderInlineImage {
                    range: range(&chapter.text, 0, 3),
                    ..image.clone()
                },
                ReaderInlineImage {
                    range: range(&chapter.text, 3, 6),
                    alt: String::new(),
                    ..image.clone()
                },
            ],
        },
    );
    document.chapters.push(chapter.clone());
    document.chapters.push(ChapterText {
        chapter_id: 5.try_into().unwrap(),
        text: "画像なし".into(),
        ..chapter.clone()
    });
    document.validate().unwrap();
    let position = ReadingPosition {
        source_id: chapter.source_id,
        chapter_id: chapter.chapter_id,
        offset: TextOffset::START,
    };
    for length in PageLength::ALL {
        let page = ReaderPage::at_document(&document, position, length).unwrap();
        assert_eq!(page.text, chapter.text);
        assert_eq!(page.images, document.images[0].images);
        let final_page = ReaderPage::at_document(
            &document,
            ReadingPosition {
                chapter_id: 5.try_into().unwrap(),
                ..position
            },
            length,
        )
        .unwrap();
        assert!(final_page.images.is_empty());
    }
}

#[test]
fn invalid_image_keys_ranges_order_and_metadata_are_rejected() {
    let valid = document();
    let chapter = &valid.chapters[0];
    let image = &valid.images[0].images[0];
    let start = i64::from(image.range.start);
    for (start, end) in [
        (start, start),
        (start + 1, start + 3),
        (start, start + 2),
        (start + 3, start),
        (0, 3),
        (start, i64::MAX),
    ] {
        let mut invalid = valid.clone();
        invalid.images[0].images[0].range = TextRange {
            start: start.try_into().unwrap(),
            end: end.try_into().unwrap(),
        };
        assert!(invalid.validate().is_err());
        assert!(ReaderPage::open_document(&invalid, PageLength::default()).is_err());
    }
    for path in [
        "",
        "/root.png",
        "../image.png",
        "OPS/../image.png",
        "OPS/./image.png",
        "OPS//image.png",
        "OPS/image.png/",
        "C:\\image.png",
        "https://example.org/a.png",
        "OPS/\0.png",
    ] {
        let mut invalid = valid.clone();
        invalid.images[0].images[0].asset.path = path.into();
        assert!(invalid.validate().is_err(), "path {path:?}");
    }
    for media_type in ["", "image/", "text/html", "image/png\n", "image/ png"] {
        let mut invalid = valid.clone();
        invalid.images[0].images[0].asset.media_type = media_type.into();
        assert!(invalid.validate().is_err(), "media type {media_type:?}");
    }
    let mut invalid = valid.clone();
    invalid.images.push(valid.images[0].clone());
    assert!(invalid.validate().is_err());
    invalid = valid.clone();
    invalid.images[0].chapter_id = 4.try_into().unwrap();
    assert!(invalid.validate().is_err());
    invalid = valid.clone();
    invalid.images[0].images.insert(1, image.clone());
    assert!(invalid.validate().is_err());
    invalid = valid.clone();
    invalid.images[0].images.swap(0, 1);
    assert!(invalid.validate().is_err());
    invalid = valid.clone();
    invalid.images[0].images[1].asset.media_type = "image/jpeg".into();
    assert!(invalid.validate().is_err());
    assert!(valid.images[0]
        .validate(&ChapterText {
            chapter_id: 4.try_into().unwrap(),
            ..chapter.clone()
        })
        .is_err());
    assert!(valid.images[0]
        .on_page(
            chapter,
            TextRange {
                start: 1.try_into().unwrap(),
                end: 3.try_into().unwrap()
            }
        )
        .is_err());
}
