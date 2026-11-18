use super::*;
use crate::document::{ChapterText, TextOffset};

#[test]
fn applying_positions_reuses_the_layout_until_reflow_or_content_change() {
    let document = ReaderDocument {
        document_id: 1.try_into().unwrap(),
        title: "Book".into(),
        finished_at_least_once: false,
        read_override: None,
        chapters: vec![ChapterText {
            source_id: 1.try_into().unwrap(),
            chapter_id: 1.try_into().unwrap(),
            text: "猫と犬。\n".repeat(2_000),
        }],
        formatting: vec![],
        images: vec![],
        reading_position: None,
    };
    let mut model = Model {
        content_route: ContentRoute::Reader,
        page_length: Some(PageLength::Characters250),
        ..Default::default()
    };
    model.reader = Some(model.project_reader(&document).unwrap());
    let layout = model.reader_layout.as_ref().unwrap().pages();
    let original_layout = layout.as_ptr();
    let next = layout[1];
    let mut document = document;
    model.reader_document = Some(document.clone());
    document.reading_position = Some(ReadingPosition {
        source_id: next.source_id,
        chapter_id: next.chapter_id,
        offset: next.range.start,
    });
    model.apply_position(document.clone());
    assert_eq!(
        model.reader_layout.as_ref().unwrap().pages().as_ptr(),
        original_layout
    );
    assert_eq!(model.reader.as_ref().unwrap().page_number, 2);

    model.apply_page_length(PageLength::Characters500);
    let reflowed_layout = model.reader_layout.as_ref().unwrap().pages().as_ptr();
    assert_ne!(reflowed_layout, original_layout);
    assert_eq!(
        Some(model.reader.as_ref().unwrap().position),
        document.reading_position
    );

    document.chapters[0].text.push_str("続き。");
    document.reading_position.as_mut().unwrap().offset = TextOffset::START;
    model.apply_position(document);
    assert_ne!(
        model.reader_layout.as_ref().unwrap().pages().as_ptr(),
        reflowed_layout
    );
    assert_eq!(model.reader.as_ref().unwrap().page_number, 1);
}
