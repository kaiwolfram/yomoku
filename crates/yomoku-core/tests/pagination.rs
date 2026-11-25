use yomoku_core::{
    document::{ChapterId, ChapterText, SourceId, TextOffset},
    pagination::{LogicalPages, PageLength, ReadingPosition},
};

fn chapter(id: i64, text: impl Into<String>) -> ChapterText {
    ChapterText {
        source_id: SourceId::try_from(1).unwrap(),
        chapter_id: ChapterId::try_from(id).unwrap(),
        text: text.into(),
    }
}

fn position(chapter: &ChapterText, byte_index: usize) -> ReadingPosition {
    ReadingPosition {
        source_id: chapter.source_id,
        chapter_id: chapter.chapter_id,
        offset: TextOffset::at(&chapter.text, byte_index).unwrap(),
    }
}

fn slices<'a>(pages: &LogicalPages<'_>, chapter: &'a ChapterText) -> Vec<&'a str> {
    pages
        .pages()
        .iter()
        .map(|page| page.slice(chapter).unwrap())
        .collect()
}

#[test]
fn presets_and_default_match_the_reader_setting() {
    assert_eq!(PageLength::default().characters(), 250);
    assert_eq!(
        PageLength::ALL.map(PageLength::characters),
        [250, 500, 750, 1000]
    );
}

#[test]
fn short_empty_and_whitespace_text_have_one_exact_page() {
    for text in ["", "短い文。", " \r\n\t\u{3000}", "𠮷😀e\u{301}\0\r\n"] {
        let chapters = [chapter(1, text)];
        let pages = LogicalPages::new(&chapters, PageLength::default()).unwrap();
        assert_eq!(slices(&pages, &chapters[0]), [text]);
        assert_eq!(
            pages.page_at(position(&chapters[0], text.len())).unwrap(),
            0
        );
    }
    let pages = LogicalPages::new(&[], PageLength::default()).unwrap();
    assert!(pages.pages().is_empty());
    assert!(pages.page_at(position(&chapter(1, ""), 0)).is_err());
}

#[test]
fn exact_threshold_uses_the_paragraph_end_without_an_empty_final_page() {
    for length in PageLength::ALL {
        let paragraph = format!("{}\r\n", "日".repeat(length.characters()));
        let chapters = [chapter(1, format!("{paragraph}続き"))];
        let pages = LogicalPages::new(&chapters, length).unwrap();
        assert_eq!(slices(&pages, &chapters[0]), [paragraph.as_str(), "続き"]);
        assert_eq!(
            pages
                .page_at(position(&chapters[0], paragraph.len()))
                .unwrap(),
            1
        );
        let chapters = [chapter(1, paragraph.clone())];
        let pages = LogicalPages::new(&chapters, length).unwrap();
        assert_eq!(slices(&pages, &chapters[0]), [paragraph]);
    }
}

#[test]
fn waits_for_a_paragraph_after_threshold_even_with_earlier_sentences() {
    let first = format!("{}。\n", "日".repeat(298));
    let second = format!("{}。{}。\n", "語".repeat(249), "文".repeat(49));
    let chapters = [chapter(1, format!("{first}{second}末尾"))];
    let pages = LogicalPages::new(&chapters, PageLength::Characters500).unwrap();
    assert_eq!(
        slices(&pages, &chapters[0]),
        [format!("{first}{second}"), "末尾".into()]
    );
}

#[test]
fn long_paragraph_uses_the_first_following_sentence_and_keeps_closing_quotes() {
    let sentence = format!("{}。！？』」", "あ".repeat(510));
    let chapters = [chapter(
        1,
        format!("{}{sentence}終。\r\n\r\n", sentence.repeat(2)),
    )];
    let pages = LogicalPages::new(&chapters, PageLength::Characters500).unwrap();
    assert_eq!(
        slices(&pages, &chapters[0]),
        [
            sentence.as_str(),
            sentence.as_str(),
            sentence.as_str(),
            "終。\r\n\r\n"
        ]
    );
}

#[test]
fn long_paragraph_fallback_accounts_for_preceding_short_paragraphs() {
    let prefix = format!("{}\n", "前".repeat(200));
    let sentence = format!("{}。", "あ".repeat(299));
    let rest = format!("{}。", "後".repeat(600));
    let chapters = [chapter(1, format!("{prefix}{sentence}{rest}"))];
    let pages = LogicalPages::new(&chapters, PageLength::Characters500).unwrap();
    assert_eq!(
        slices(&pages, &chapters[0]),
        [format!("{prefix}{sentence}"), rest]
    );
}

#[test]
fn unbroken_text_is_never_cut_at_an_arbitrary_character() {
    let text = "字".repeat(10_000);
    let chapters = [chapter(1, &text)];
    let pages = LogicalPages::new(&chapters, PageLength::Characters250).unwrap();
    assert_eq!(slices(&pages, &chapters[0]), [text]);
}

#[test]
fn paragraph_at_twice_the_preset_stays_whole_and_one_above_uses_sentences() {
    let first = format!("{}。", "日".repeat(249));
    let second = format!("{}。", "語".repeat(249));
    let chapters = [chapter(1, format!("{first}{second}\n残り"))];
    let pages = LogicalPages::new(&chapters, PageLength::Characters250).unwrap();
    assert_eq!(
        slices(&pages, &chapters[0]),
        [format!("{first}{second}\n"), "残り".into()]
    );
    let chapters = [chapter(1, format!("{first}{second}終\n残り"))];
    let pages = LogicalPages::new(&chapters, PageLength::Characters250).unwrap();
    assert_eq!(
        slices(&pages, &chapters[0]),
        [first, second, "終\n残り".into()]
    );
}

#[test]
fn mixed_script_counts_scalars_including_punctuation_but_excludes_whitespace() {
    // 10 scalars per unit; supplementary kanji/emoji count once, as do each
    // Latin letter, punctuation and the combining mark. Whitespace counts zero.
    let unit = "日かカ𠮷😀Ab、e\u{301} \t\u{3000}";
    let paragraph = format!("{}\r\n", unit.repeat(25));
    let chapters = [chapter(1, format!("{paragraph}残り"))];
    let pages = LogicalPages::new(&chapters, PageLength::Characters250).unwrap();
    assert_eq!(slices(&pages, &chapters[0]), [paragraph.as_str(), "残り"]);
}

#[test]
fn ascii_sentences_keep_quotes_and_do_not_treat_a_decimal_as_a_sentence_end() {
    let first = format!("{}3.14 is a value.\"", "A".repeat(250));
    let second = format!("{}!", "B".repeat(260));
    let chapters = [chapter(1, format!("{first} {second}"))];
    let pages = LogicalPages::new(&chapters, PageLength::Characters250).unwrap();
    assert_eq!(slices(&pages, &chapters[0]), [first, format!(" {second}")]);
}

#[test]
fn explicit_chapters_force_pages_and_txt_headings_never_infer_chapters() {
    let chapters = [
        chapter(2, "序章\n短い。"),
        chapter(1, "第一章\n短い。"),
        chapter(3, ""),
    ];
    let pages = LogicalPages::new(&chapters, PageLength::default()).unwrap();
    assert_eq!(pages.pages().len(), 3);
    for (index, chapter) in chapters.iter().enumerate() {
        assert_eq!(pages.pages()[index].slice(chapter).unwrap(), chapter.text);
        assert_eq!(
            pages
                .page_at(position(chapter, chapter.text.len()))
                .unwrap(),
            index
        );
    }
    let chapters = [chapter(1, "序章\n短い。\n\n第一章\n短い。")];
    let pages = LogicalPages::new(&chapters, PageLength::default()).unwrap();
    assert_eq!(pages.pages().len(), 1);
}

#[test]
fn reflow_between_every_preset_preserves_every_exact_unicode_offset_and_ownership() {
    let chapters = [
        chapter(1, format!("{}\r\n", "日本𠮷😀か\u{3099}Ab。\n".repeat(240))),
        chapter(2, format!("{}\n", "「次の文です！」".repeat(200))),
        chapter(3, ""),
    ];
    for from in PageLength::ALL {
        let original = LogicalPages::new(&chapters, from).unwrap();
        for to in PageLength::ALL {
            let mut result = original.reflow(to, position(&chapters[0], 0)).unwrap();
            assert_eq!(result.pages.length(), to);
            for chapter in &chapters {
                let chapter_pages: Vec<_> = result
                    .pages
                    .pages()
                    .iter()
                    .filter(|page| page.chapter_id == chapter.chapter_id)
                    .collect();
                let reconstructed: String = chapter_pages
                    .iter()
                    .map(|page| page.slice(chapter).unwrap())
                    .collect();
                assert_eq!(reconstructed, chapter.text);
                assert_eq!(
                    chapter_pages.first().unwrap().range.start,
                    TextOffset::START
                );
                for pair in chapter_pages.windows(2) {
                    assert_eq!(pair[0].range.end, pair[1].range.start);
                }
                for index in chapter
                    .text
                    .char_indices()
                    .map(|(index, _)| index)
                    .chain([chapter.text.len()])
                {
                    let position = position(chapter, index);
                    // Exercise actual reflow for positions both at and inside pages.
                    if index % 97 == 0 || index == chapter.text.len() {
                        result = original.reflow(to, position).unwrap();
                        assert_eq!(result.position, position);
                        let page = result.pages.pages()[result.page_index];
                        assert_eq!(page.chapter_id, chapter.chapter_id);
                        assert!(page.range.start <= position.offset);
                        assert!(position.offset < page.range.end || index == chapter.text.len());
                    }
                    let page = result.pages.pages()[result.pages.page_at(position).unwrap()];
                    assert_eq!(page.chapter_id, chapter.chapter_id);
                    assert!(
                        page.range.start <= position.offset && position.offset <= page.range.end
                    );
                }
            }
        }
    }
}

#[test]
fn invalid_ownership_and_offsets_are_rejected_without_changing_layout() {
    let chapters = [chapter(1, "𠮷日本語")];
    let pages = LogicalPages::new(&chapters, PageLength::default()).unwrap();
    for position in [
        ReadingPosition {
            source_id: SourceId::try_from(2).unwrap(),
            ..position(&chapters[0], 0)
        },
        ReadingPosition {
            chapter_id: ChapterId::try_from(2).unwrap(),
            ..position(&chapters[0], 0)
        },
        ReadingPosition {
            offset: TextOffset::try_from(1).unwrap(),
            ..position(&chapters[0], 0)
        },
        ReadingPosition {
            offset: TextOffset::try_from(i64::MAX).unwrap(),
            ..position(&chapters[0], 0)
        },
    ] {
        assert!(pages.page_at(position).is_err());
        assert!(pages.reflow(PageLength::Characters250, position).is_err());
    }
    assert_eq!(slices(&pages, &chapters[0]), ["𠮷日本語"]);
    assert!(LogicalPages::new(&[chapter(1, "a"), chapter(1, "b")], PageLength::default()).is_err());
    let mut other = chapter(2, "b");
    other.source_id = SourceId::try_from(2).unwrap();
    assert!(LogicalPages::new(&[chapter(1, "a"), other], PageLength::default()).is_err());
}

#[test]
fn trailing_blank_lines_after_sentence_fallback_extend_the_final_page() {
    let sentence = format!("{}。", "字".repeat(250));
    let text = format!("{} \r\n\r\n\u{3000}\n", sentence.repeat(3));
    let chapters = [chapter(1, text)];
    let pages = LogicalPages::new(&chapters, PageLength::Characters250).unwrap();
    assert_eq!(
        slices(&pages, &chapters[0]),
        [
            sentence.as_str(),
            sentence.as_str(),
            format!("{sentence} \r\n\r\n\u{3000}\n").as_str()
        ]
    );
}
