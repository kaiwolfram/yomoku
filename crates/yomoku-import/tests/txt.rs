use yomoku_core::{
    document::{ChapterId, ChapterText, SourceId, TextOffset},
    pagination::{LogicalPages, PageLength, ReadingPosition},
};
use yomoku_import::txt::{parse, SourceRuby, TextEncoding, TxtDocument, TxtError};

fn utf8(text: &str) -> TxtDocument {
    parse(text.as_bytes(), Some(TextEncoding::Utf8)).unwrap()
}

fn readings(document: &TxtDocument) -> Vec<(&str, &str)> {
    let mut last_end = 0;
    document
        .ruby
        .iter()
        .map(|ruby| {
            assert!(ruby.range.start >= last_end);
            assert!(ruby.range.start < ruby.range.end);
            last_end = ruby.range.end;
            // All ranges must be exact UTF-8 boundaries in the canonical text.
            (
                document.text.get(ruby.range.clone()).unwrap(),
                ruby.reading.as_str(),
            )
        })
        .collect()
}

#[test]
fn detects_unambiguous_utf8_shift_jis_and_equivalent_ascii() {
    for (bytes, encoding, text) in [
        ("あ".as_bytes(), TextEncoding::Utf8, "あ"),
        (&[0x82, 0xA0], TextEncoding::ShiftJis, "あ"),
        (
            b"one\r\ntwo\n\rthree",
            TextEncoding::Utf8,
            "one\r\ntwo\n\rthree",
        ),
        (&[0x87, 0x40], TextEncoding::ShiftJis, "①"),
        (&[0xB6, 0xC0, 0xB6, 0xC5], TextEncoding::ShiftJis, "ｶﾀｶﾅ"),
    ] {
        let document = parse(bytes, None).unwrap();
        assert_eq!(document.encoding, encoding);
        assert_eq!(document.text, text);
        assert!(document.ruby.is_empty());
    }
}

#[test]
fn ambiguous_bytes_require_selection_and_each_choice_is_reproducible() {
    // C2 A9 is © in UTF-8, but two half-width kana in Shift-JIS.
    let bytes = [0xC2, 0xA9];
    assert_eq!(parse(&bytes, None), Err(TxtError::EncodingRequired));
    let unicode = parse(&bytes, Some(TextEncoding::Utf8)).unwrap();
    let legacy = parse(&bytes, Some(TextEncoding::ShiftJis)).unwrap();
    assert_eq!(unicode.text, "©");
    assert_eq!(legacy.text, "ﾂｩ");
    assert_ne!(unicode, legacy);
    assert_eq!(parse(&bytes, Some(TextEncoding::Utf8)).unwrap(), unicode);
    assert_eq!(bytes, [0xC2, 0xA9]);
}

#[test]
fn utf8_bom_is_authoritative_and_only_leading_bom_is_removed() {
    let document = parse(b"\xEF\xBB\xBF\xC2\xA9", None).unwrap();
    assert_eq!(document.encoding, TextEncoding::Utf8);
    assert_eq!(document.text, "©");
    assert_eq!(utf8("\u{FEFF}本文\u{FEFF}").text, "本文\u{FEFF}");
    assert_eq!(
        parse(b"\xEF\xBB\xBF\x82\xA0", None),
        Err(TxtError::InvalidEncoding(TextEncoding::Utf8))
    );
}

#[test]
fn malformed_bytes_and_wrong_manual_choices_never_insert_replacements() {
    for bytes in [&[0x82][..], &[0x82, 0x20], &[0xFF], &[0xC0, 0xAF, 0x82]] {
        assert_eq!(parse(bytes, None), Err(TxtError::Undecodable));
        for encoding in [TextEncoding::Utf8, TextEncoding::ShiftJis] {
            assert_eq!(
                parse(bytes, Some(encoding)),
                Err(TxtError::InvalidEncoding(encoding))
            );
        }
    }
    assert_eq!(
        parse(&[0x82, 0xA0], Some(TextEncoding::Utf8)),
        Err(TxtError::InvalidEncoding(TextEncoding::Utf8))
    );
    // A literal replacement character in a valid source is not a decoder error.
    assert_eq!(utf8("元の�文字").text, "元の�文字");
}

#[test]
fn unsupported_boms_are_rejected_even_with_manual_selection() {
    for bytes in [
        &[0xFF, 0xFE, 0x41, 0][..],
        &[0xFE, 0xFF, 0, 0x41],
        &[0xFF, 0xFE, 0, 0, 0x41, 0, 0, 0],
        &[0, 0, 0xFE, 0xFF, 0, 0, 0, 0x41],
    ] {
        for selection in [None, Some(TextEncoding::Utf8), Some(TextEncoding::ShiftJis)] {
            assert_eq!(parse(bytes, selection), Err(TxtError::UnsupportedEncoding));
        }
    }
}

#[test]
fn rejects_empty_whitespace_bom_and_directive_only_text() {
    for text in [
        "",
        " \t\r\n\u{3000}",
        "\u{FEFF}",
        "［＃改ページ］\r\n［＃３字下げ］",
    ] {
        assert_eq!(
            parse(text.as_bytes(), Some(TextEncoding::Utf8)),
            Err(TxtError::EmptyText)
        );
    }
    for bytes in [b"".as_slice(), b" \t\r\n", b"\xEF\xBB\xBF"] {
        assert_eq!(parse(bytes, None), Err(TxtError::EmptyText));
    }
}

#[test]
fn hard_breaks_whitespace_unicode_and_literal_markup_remain_exact() {
    let source = "  第一章\r\n続く行\r単独CR\n\n　𠮷😀e\u{301}\0\t <b>本文</b> | literal\n";
    let document = utf8(source);
    assert_eq!(document.text, source);
    assert!(document.ruby.is_empty());
    let shift_jis = [0x82, 0xA0, b'\r', b'\n', b'\r', 0x82, 0xA2, b'\n'];
    assert_eq!(parse(&shift_jis, None).unwrap().text, "あ\r\n\rい\n");
}

#[test]
fn aozora_fixture_keeps_base_text_and_all_source_ruby_in_order() {
    let source = include_str!("fixtures/aozora.txt");
    let document = utf8(source);
    assert_eq!(document.text, include_str!("fixtures/aozora-expected.txt"));
    assert_eq!(
        readings(&document),
        [
            ("漢字", "かんじ"),
            ("東京", "とうきょう"),
            ("食べた", "たべた"),
            ("𠮷野家", "よしのや"),
            ("市ヶ谷", "いちがや"),
            ("転々", "てんてん"),
        ]
    );
    let encoded = encoding_rs::SHIFT_JIS.encode(source);
    // This fixture contains supplementary kanji: it must not be encoded lossily.
    assert!(encoded.2);
    let source = source.replace('𠮷', "吉");
    let (encoded, _, replaced) = encoding_rs::SHIFT_JIS.encode(&source);
    assert!(!replaced);
    let legacy = parse(&encoded, None).unwrap();
    let unicode = utf8(&source);
    assert_eq!(legacy.text, unicode.text);
    assert_eq!(legacy.ruby, unicode.ruby);
}

#[test]
fn ruby_ranges_handle_mixed_scripts_combining_marks_and_adjacent_bases() {
    let document = utf8("😀｜e\u{301}と漢字《reading》東京《とうきょう》京都《きょうと》。お湯《ゆ》と〆切《しめきり》。");
    assert_eq!(document.text, "😀e\u{301}と漢字東京京都。お湯と〆切。");
    assert_eq!(
        readings(&document),
        [
            ("e\u{301}と漢字", "reading"),
            ("東京", "とうきょう"),
            ("京都", "きょうと"),
            ("湯", "ゆ"),
            ("〆切", "しめきり"),
        ]
    );
    assert_eq!(utf8("東京《とうきょう》《二つ目》").text, "東京《二つ目》");
    assert_eq!(utf8("東京《とうきょう》京都《きょうと》").ruby.len(), 2);
}

#[test]
fn common_typographic_directives_hide_without_removing_hard_breaks() {
    let source = "［＃ここから２字下げ］\r\n第一章［＃「第一章」は中見出し］\r\n［＃ここから斜体］本文［＃斜体終わり］\r\n［＃ここで字下げ終わり］\r\n［＃改丁］\r\n［＃地から３字上げ］末尾";
    assert_eq!(utf8(source).text, "\r\n第一章\r\n本文\r\n\r\n\r\n末尾");
}

#[test]
fn unknown_semantic_gaiji_and_malformed_notation_remain_visible() {
    for source in [
        "※［＃「木＋靈」、第3水準1-86-29］の文字",
        "※［＃太字］",
        "本文［＃未対応の注記］続き",
        "本文［＃「本文」は底本では「原文」］",
        "［＃未知［＃太字］］",
        "［＃未知［＃太字］",
        "［＃太字\n本文",
        "漢字《未完\r\n次の行",
        "｜漢字《未完",
        "｜《から》",
        "漢字《》",
        "漢字《\n》",
        "漢字《不正《読み》》",
        "漢字《［＃太字］》",
        "《引用符》と｜縦棒",
        "かな《未対応の暗黙ルビ》",
    ] {
        assert_eq!(utf8(source).text, source, "{source}");
    }
}

#[test]
fn canonicalization_precedes_pagination_and_never_infers_chapters() {
    let document = utf8(&format!(
        "第一章\r\n{}［＃改ページ］\r\n第二章\r\n{}",
        "漢字《かんじ》".repeat(260),
        "末尾。".repeat(200)
    ));
    let chapters = [ChapterText {
        source_id: SourceId::try_from(1).unwrap(),
        chapter_id: ChapterId::try_from(1).unwrap(),
        text: document.text.clone(),
    }];
    for length in PageLength::ALL {
        let pages = LogicalPages::new(&chapters, length).unwrap();
        let slices: Vec<_> = pages
            .pages()
            .iter()
            .map(|page| page.slice(&chapters[0]).unwrap())
            .collect();
        assert_eq!(slices.concat(), document.text);
        assert!(pages
            .pages()
            .iter()
            .all(|page| page.chapter_id == chapters[0].chapter_id));
        for SourceRuby { range, .. } in &document.ruby {
            assert_eq!(&document.text[range.clone()], "漢字");
            let offset = TextOffset::at(&chapters[0].text, range.start).unwrap();
            let position = ReadingPosition {
                source_id: chapters[0].source_id,
                chapter_id: chapters[0].chapter_id,
                offset,
            };
            assert!(pages.page_at(position).is_ok());
            assert_eq!(
                TextOffset::from_utf16(&document.text, offset.to_utf16(&document.text).unwrap())
                    .unwrap(),
                offset
            );
        }
    }
}

#[test]
fn large_and_repeated_malformed_markup_keeps_text_and_does_not_recurse() {
    let source = format!(
        "{}\n{}\n{}",
        "｜".repeat(20_000),
        "《".repeat(20_000),
        "［＃".repeat(20_000)
    );
    assert_eq!(utf8(&source).text, source);
    let source = "東京《とうきょう》。\r\n".repeat(10_000);
    let document = utf8(&source);
    assert_eq!(document.text, "東京。\r\n".repeat(10_000));
    assert_eq!(document.ruby.len(), 10_000);
    assert_eq!(readings(&document).len(), 10_000);
}
