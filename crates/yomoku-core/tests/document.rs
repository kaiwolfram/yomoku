use yomoku_core::document::{
    ChapterId, ChapterText, DocumentId, PasteText, SourceId, TextAnchor, TextOffset, TextRange,
};

#[test]
fn paste_title_uses_the_literal_first_line_and_preserves_canonical_text() {
    for (text, title) in [
        ("  日本語𠮷📖  \r\nか\u{3099}\n👩‍👩‍👧‍👦\0", "  日本語𠮷📖  "),
        ("第一章\n\n次の行", "第一章"),
        ("\n本文", ""),
        ("\r\n本文", ""),
        ("文\r", "文\r"),
        ("一行", "一行"),
        (
            "title\0'\"; DROP TABLE documents;--\n本文",
            "title\0'\"; DROP TABLE documents;--",
        ),
    ] {
        let paste = PasteText::new(text.to_owned()).unwrap();
        assert_eq!(paste.initial_title(), title);
        assert_eq!(paste.as_str(), text);
    }
}

#[test]
fn empty_and_whitespace_only_pastes_are_rejected_without_trimming_valid_text() {
    for text in ["", " ", "\t\r\n", "\u{3000}\u{a0}"] {
        assert!(PasteText::new(text.to_owned()).is_err());
    }
    assert_eq!(
        PasteText::new(" 本文 \n".into()).unwrap().as_str(),
        " 本文 \n"
    );
}

#[test]
fn identities_are_positive_sqlite_integers_without_floating_point_loss() {
    for value in [1, (1 << 53) + 1, i64::MAX] {
        assert_eq!(i64::from(DocumentId::try_from(value).unwrap()), value);
        assert_eq!(i64::from(SourceId::try_from(value).unwrap()), value);
        assert_eq!(i64::from(ChapterId::try_from(value).unwrap()), value);
    }
    for value in [i64::MIN, -1, 0] {
        assert!(DocumentId::try_from(value).is_err());
        assert!(SourceId::try_from(value).is_err());
        assert!(ChapterId::try_from(value).is_err());
    }
}

#[test]
fn every_unicode_scalar_boundary_round_trips_to_utf16_without_normalization() {
    for text in [
        "",
        "ASCII\0\r\n",
        "日本語。",
        "𠮷野家😀",
        "か\u{3099}é e\u{301}",
        "👩‍👩‍👧‍👦\n終",
    ] {
        let mut utf16_index = 0;
        for (byte_index, scalar) in text.char_indices() {
            let offset = TextOffset::at(text, byte_index).unwrap();
            assert_eq!(offset.to_utf16(text).unwrap(), utf16_index);
            assert_eq!(TextOffset::from_utf16(text, utf16_index).unwrap(), offset);
            if scalar.len_utf16() == 2 {
                assert!(TextOffset::from_utf16(text, utf16_index + 1).is_err());
            }
            utf16_index += scalar.len_utf16();
        }
        let eof = TextOffset::at(text, text.len()).unwrap();
        assert_eq!(TextOffset::from_utf16(text, utf16_index).unwrap(), eof);
        assert_eq!(eof.to_utf16(text).unwrap(), utf16_index);
        assert!(TextOffset::from_utf16(text, utf16_index + 1).is_err());
        assert!(TextOffset::from_utf16(text, usize::MAX).is_err());
        for byte_index in 0..=text.len() + 1 {
            assert_eq!(
                TextOffset::at(text, byte_index).is_ok(),
                text.is_char_boundary(byte_index)
            );
        }
    }
}

#[test]
fn ranges_reject_reversed_out_of_bounds_and_partial_utf8_positions() {
    let text = "A𠮷か\u{3099}\r\n終";
    let slice = |start, end| {
        TextRange {
            start: TextOffset::try_from(start).unwrap(),
            end: TextOffset::try_from(end).unwrap(),
        }
        .slice(text)
    };
    assert_eq!(slice(1, 5).unwrap(), "𠮷");
    assert_eq!(slice(5, 8).unwrap(), "か"); // No implicit grapheme snapping.
    assert_eq!(slice(8, 11).unwrap(), "\u{3099}");
    assert_eq!(slice(0, text.len() as i64).unwrap(), text);
    assert_eq!(slice(text.len() as i64, text.len() as i64).unwrap(), "");
    for (start, end) in [(5, 1), (0, 2), (2, 5), (0, i64::MAX)] {
        assert!(slice(start, end).is_err());
    }
    assert!(TextOffset::try_from(-1).is_err());
    assert!(TextOffset::try_from(i64::MAX)
        .unwrap()
        .to_utf16(text)
        .is_err());
    assert_eq!(TextOffset::START.byte_index(text).unwrap(), 0);
}

#[test]
fn anchors_cannot_be_applied_to_a_different_source_or_chapter() {
    let chapter = ChapterText {
        source_id: SourceId::try_from(1).unwrap(),
        chapter_id: ChapterId::try_from(1).unwrap(),
        text: "日本語".into(),
    };
    let anchor = TextAnchor {
        source_id: chapter.source_id,
        chapter_id: chapter.chapter_id,
        range: TextRange {
            start: TextOffset::START,
            end: TextOffset::at(&chapter.text, 6).unwrap(),
        },
    };
    assert_eq!(anchor.slice(&chapter).unwrap(), "日本");
    assert!(TextAnchor {
        source_id: SourceId::try_from(2).unwrap(),
        ..anchor
    }
    .slice(&chapter)
    .is_err());
    assert!(TextAnchor {
        chapter_id: ChapterId::try_from(2).unwrap(),
        ..anchor
    }
    .slice(&chapter)
    .is_err());
}
