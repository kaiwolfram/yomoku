use yomoku_core::{
    document::{ChapterId, ChapterText, SourceId, TextOffset},
    pagination::{LogicalPages, PageLength, ReadingPosition},
};
use yomoku_import::srt::{parse, EmptySrt, SrtDocument};

fn cues(document: &SrtDocument) -> Vec<&str> {
    let mut previous = 0;
    let cues: Vec<_> = document
        .cues
        .iter()
        .map(|range| {
            assert!(range.start >= previous);
            assert!(range.start < range.end);
            let separator = if previous == 0 { "" } else { "\n\n" };
            assert_eq!(&document.text[previous..range.start], separator);
            previous = range.end;
            let text = document.text.get(range.clone()).unwrap();
            assert!(!text.trim().is_empty());
            text
        })
        .collect();
    assert_eq!(previous, document.text.len());
    assert_eq!(cues.join("\n\n"), document.text);
    cues
}

#[test]
fn fixture_strips_metadata_and_retains_cues_breaks_and_speakers() {
    let source = include_str!("fixtures/subtitles.srt");
    let document = parse(source).unwrap();
    assert_eq!(
        document.text,
        include_str!("fixtures/subtitles-expected.txt").trim_end_matches('\n')
    );
    assert_eq!(
        cues(&document),
        [
            "（太郎）今日は。\n- 花子：こんにちは。",
            "東京 & 京都\n続く行。",
            "𠮷野家😀で会う。"
        ]
    );
    assert!(source.contains("00:00:01,000"));
}

#[test]
fn bom_and_all_line_endings_produce_the_same_canonical_text() {
    let source = include_str!("fixtures/subtitles.srt");
    let expected = parse(source).unwrap();
    for ending in ["\n", "\r\n", "\r"] {
        let source = format!("\u{feff}{}", source.replace('\n', ending));
        assert_eq!(parse(&source).unwrap(), expected);
    }
    assert_eq!(parse("本文\u{feff}続き").unwrap().text, "本文\u{feff}続き");
}

#[test]
fn missing_counters_blank_separators_and_nonsequential_counters_recover() {
    let source = "00:00:01,000 --> 00:00:02,000\n最初\n8\n00:00:03,000 --> 00:00:04,000\n次\n00:00:05,000 --> 00:00:06,000\n最後";
    let document = parse(source).unwrap();
    assert_eq!(cues(&document), ["最初", "次", "最後"]);
}

#[test]
fn tolerant_timing_syntax_does_not_require_chronology_or_numeric_arithmetic() {
    let source = " 0009 \n 0:01.5-->0:02.75 X1:0 X2:100\n短縮\n\n3\n00:02:03 --> 00:01:01\n逆順\n\n3\n999999999999999999999999:99:99,999 --> 999999999999999999999999:99:99,999\n長時間";
    assert_eq!(cues(&parse(source).unwrap()), ["短縮", "逆順", "長時間"]);
}

#[test]
fn isolated_numbers_timestamps_and_dialogue_arrows_remain_visible() {
    let source = "1\n00:00:01,000 --> 00:00:02,000\n2026\n00:00:05,000\nA --> B\n10:30 --> 帰宅\n-123\n\n42\n答え\n";
    assert_eq!(
        cues(&parse(source).unwrap()),
        [
            "2026\n00:00:05,000\nA --> B\n10:30 --> 帰宅\n-123",
            "42\n答え"
        ]
    );
}

#[test]
fn unrecognized_or_damaged_timing_remains_readable_without_hiding_next_cues() {
    let source = "1\n00:00:bad --> 00:00:02,000\n壊れた開始\n\n2\n00:00:03,000 -->\n終了なし\n\n3\n00:00:04,000 --> 00:00:05,000\n回復";
    assert_eq!(
        cues(&parse(source).unwrap()),
        [
            "1\n00:00:bad --> 00:00:02,000\n壊れた開始",
            "2\n00:00:03,000 -->\n終了なし",
            "回復"
        ]
    );
}

#[test]
fn formatting_is_removed_without_merging_words_and_break_tags_keep_lines() {
    let source = "<I>one</I> <b>two</b>\n<font color=\"red\"><u>三</u></font><br />四<BR>五\n<s>六</s><strike>七</strike><span>八</span>";
    let document = parse(source).unwrap();
    assert_eq!(cues(&document), ["one two\n三\n四\n五\n六七八"]);
}

#[test]
fn unknown_speaker_tags_braces_aozora_and_malformed_markup_stay_visible() {
    for source in [
        "<太郎>こんにちは\n[花子] やあ\n（話者）返事\n♪ 音楽",
        "<speaker name=\"太郎\">本文</speaker>",
        "2 < 3 と 5 > 4",
        "本文<i 未完",
        "<<不明>> &unknown; &unfinished",
        "{\\an8}字幕と{話者}",
        "｜漢字《かんじ》［＃太字］",
    ] {
        assert_eq!(parse(source).unwrap().text, source, "{source}");
    }
}

#[test]
fn entities_decode_once_after_tags_and_keep_invalid_references() {
    let source = "&lt;i&gt;本文&lt;/i&gt; &amp;lt; &quot;字&quot; &apos; &nbsp; &#x20BB7; &#128512; &#101;&#769;\n&#0; &#10; &#xD800; &#x110000; &#-1; &#+65; &#x+41; &#; &unknown;";
    let document = parse(source).unwrap();
    assert_eq!(
        document.text,
        "<i>本文</i> &lt; \"字\" ' \u{a0} 𠮷 😀 e\u{301}\n&#0; &#10; &#xD800; &#x110000; &#-1; &#+65; &#x+41; &#; &unknown;"
    );
    cues(&document);
}

#[test]
fn empty_metadata_and_formatting_only_cues_are_ignored_or_rejected() {
    for source in [
        "",
        " \t\r\n\u{3000}",
        "\u{feff}",
        "1\n00:00:01,000 --> 00:00:02,000\n\n2\n00:00:03,000 --> 00:00:04,000",
        "<i></i>\n<br>\n&nbsp;",
    ] {
        assert_eq!(parse(source), Err(EmptySrt), "{source}");
    }
    let document = parse("<i></i>\n\n本文\n\n<br>\n\n次\n\n").unwrap();
    assert_eq!(cues(&document), ["本文", "次"]);
}

#[test]
fn unicode_and_dialogue_whitespace_are_preserved_with_canonical_byte_ranges() {
    let document = parse("1\n00:00:01,000 --> 00:00:02,000\n  𠮷😀e\u{301}\t\n　次\n \t\n2\n00:00:03,000 --> 00:00:04,000\n結末").unwrap();
    assert_eq!(cues(&document), ["  𠮷😀e\u{301}\t\n　次", "結末"]);
    for range in &document.cues {
        for index in [range.start, range.end] {
            let offset = TextOffset::at(&document.text, index).unwrap();
            assert_eq!(
                TextOffset::from_utf16(&document.text, offset.to_utf16(&document.text).unwrap())
                    .unwrap(),
                offset
            );
        }
    }
}

#[test]
fn cue_paragraphs_paginate_as_one_text_unit_for_every_preset() {
    let source: String = (1..=300)
        .map(|index| {
            format!("{index}\n00:00:01,000 --> 00:00:02,000\n<i>字幕𠮷😀。</i>\n次の行。\n\n")
        })
        .collect();
    let document = parse(&source).unwrap();
    assert_eq!(cues(&document).len(), 300);
    let chapter = ChapterText {
        source_id: SourceId::try_from(1).unwrap(),
        chapter_id: ChapterId::try_from(1).unwrap(),
        text: document.text.clone(),
    };
    for length in PageLength::ALL {
        let pages = LogicalPages::new(std::slice::from_ref(&chapter), length).unwrap();
        assert_eq!(
            pages
                .pages()
                .iter()
                .map(|page| page.slice(&chapter).unwrap())
                .collect::<String>(),
            document.text
        );
        assert!(pages.pages().len() > 1);
        for range in &document.cues {
            let position = ReadingPosition {
                source_id: chapter.source_id,
                chapter_id: chapter.chapter_id,
                offset: TextOffset::at(&chapter.text, range.start).unwrap(),
            };
            assert!(pages.page_at(position).is_ok());
        }
    }
}

#[test]
fn long_malformed_tags_entities_and_many_cues_do_not_recurse_or_drop_text() {
    let source = format!(
        "{}\n{}\n{}",
        "<".repeat(50_000),
        "&".repeat(50_000),
        "𠮷".repeat(50_000)
    );
    assert_eq!(parse(&source).unwrap().text, source);
    let source = "1\n00:00:01,000 --> 00:00:02,000\n<i>本文。</i>\n\n".repeat(10_000);
    let document = parse(&source).unwrap();
    assert_eq!(cues(&document), vec!["本文。"; 10_000]);
}
