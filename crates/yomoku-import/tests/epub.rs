use std::{
    collections::BTreeMap,
    io::{Cursor, Write},
};

use yomoku_core::{
    document::{ChapterId, ChapterText, SourceId, TextOffset},
    pagination::{LogicalPages, PageLength},
};
use yomoku_import::epub::{parse, parse_with_limits, EpubError, EpubLimits, SpanKind};
use zip::{write::SimpleFileOptions, CompressionMethod, ZipWriter};

type Files = BTreeMap<String, Vec<u8>>;

fn fixture() -> Files {
    [
        ("mimetype", b"application/epub+zip".to_vec()),
        (
            "META-INF/container.xml",
            include_bytes!("fixtures/epub/container.xml").to_vec(),
        ),
        (
            "OPS/book.opf",
            include_bytes!("fixtures/epub/book.opf").to_vec(),
        ),
        (
            "OPS/text/z-first.xhtml",
            include_bytes!("fixtures/epub/first.xhtml").to_vec(),
        ),
        (
            "OPS/text/a-second.xhtml",
            xhtml("<h2>第二章</h2><p>別の章。</p>"),
        ),
        // Parser retains immutable bytes; decoding/dimensions belong to rendering.
        ("OPS/images/cover.png", b"fixture cover bytes".to_vec()),
        ("OPS/images/挿絵.png", b"fixture image bytes".to_vec()),
    ]
    .into_iter()
    .map(|(name, bytes)| (name.to_owned(), bytes))
    .collect()
}

fn xhtml(body: &str) -> Vec<u8> {
    format!(r#"<html xmlns="http://www.w3.org/1999/xhtml"><head/><body>{body}</body></html>"#)
        .into_bytes()
}

fn archive(files: Files) -> Vec<u8> {
    compress(files, CompressionMethod::Deflated)
}

fn compress(files: Files, compression: CompressionMethod) -> Vec<u8> {
    let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
    for (name, bytes) in files {
        writer
            .start_file(
                name,
                SimpleFileOptions::default().compression_method(compression),
            )
            .unwrap();
        writer.write_all(&bytes).unwrap();
    }
    writer.finish().unwrap().into_inner()
}

fn with_body(body: &str) -> Vec<u8> {
    let mut files = fixture();
    files.insert("OPS/text/z-first.xhtml".to_owned(), xhtml(body));
    archive(files)
}

fn edit(files: &mut Files, path: &str, from: &str, to: &str) {
    let text = String::from_utf8(files[path].clone())
        .unwrap()
        .replace(from, to);
    files.insert(path.to_owned(), text.into_bytes());
}

#[test]
fn extracts_fixture_in_spine_order_with_metadata_and_canonical_ranges() {
    for compression in [CompressionMethod::Stored, CompressionMethod::Deflated] {
        let document = parse(&compress(fixture(), compression)).unwrap();
        assert_eq!(document.title.as_deref(), Some("日本語の本 & EPUB"));
        assert_eq!(document.chapters.len(), 2);
        let first = &document.chapters[0];
        assert_eq!(first.path, "OPS/text/z-first.xhtml");
        assert_eq!(
            first.text,
            include_str!("fixtures/epub/first-expected.txt").trim_end_matches('\n')
        );
        assert_eq!(document.chapters[1].text, "第二章\n\n別の章。");
        assert_eq!(document.cover.as_deref(), Some("OPS/images/cover.png"));
        assert_eq!(document.images.len(), 2);
        assert_eq!(
            document.images["OPS/images/挿絵.png"].bytes,
            b"fixture image bytes"
        );
    }
}

#[test]
fn missing_or_blank_metadata_title_leaves_filename_fallback_to_adapter() {
    for replacement in ["", "<dc:title> \n </dc:title>"] {
        let mut files = fixture();
        edit(
            &mut files,
            "OPS/book.opf",
            "<dc:title> 日本語の本 &amp; EPUB </dc:title>",
            replacement,
        );
        assert_eq!(parse(&archive(files)).unwrap().title, None);
    }
}

#[test]
fn nested_emphasis_headings_and_links_keep_exact_unicode_ranges() {
    let bytes = with_body(
        r##"<h6> 見出し </h6><p>前 <em>食<strong>べ𠮷😀</strong>る</em> 後</p><p><a href="#note">注</a><a href="chapter.xhtml#note">内部</a><a href="javascript:alert(1)">危険</a><a href="mailto:a@example.org">メール</a><a href="HTTPS://example.org/">外部</a></p>"##,
    );
    let document = parse(&bytes).unwrap();
    let chapter = &document.chapters[0];
    let spans: Vec<_> = chapter
        .spans
        .iter()
        .map(|span| (&chapter.text[span.range.clone()], &span.kind))
        .collect();
    assert_eq!(spans[0], ("見出し", &SpanKind::Heading(6)));
    assert!(spans.contains(&("食べ𠮷😀る", &SpanKind::Emphasis)));
    assert!(spans.contains(&("べ𠮷😀", &SpanKind::Strong)));
    assert!(chapter.text.contains("注内部危険メール外部"));
    assert_eq!(spans.len(), 4);
    assert!(spans.contains(&(
        "外部",
        &SpanKind::ExternalLink("HTTPS://example.org/".to_owned())
    )));
    for span in &chapter.spans {
        for index in [span.range.start, span.range.end] {
            let offset = TextOffset::at(&chapter.text, index).unwrap();
            assert_eq!(
                TextOffset::from_utf16(&chapter.text, offset.to_utf16(&chapter.text).unwrap())
                    .unwrap(),
                offset
            );
        }
    }
}

#[test]
fn source_ruby_readings_and_parentheses_never_enter_base_text() {
    let document = parse(&with_body("<p>𠮷😀<ruby><rb>東</rb><rt>とう</rt><rb>京</rb><rt>きょう</rt><rtc><rt>Tokyo</rt></rtc></ruby>と<ruby>漢字<rp>(</rp><rt>かんじ</rt><rp>)</rp></ruby>。</p>")).unwrap();
    let chapter = &document.chapters[0];
    assert_eq!(chapter.text, "𠮷😀東京と漢字。");
}

#[test]
fn css_is_ignored_html_whitespace_collapses_and_pre_and_breaks_survive() {
    let document = parse(&with_body("<style>p{display:none}</style><p style=\"display:none\"> A \n B&#160;　C <em> D </em> E<br/>F</p><p hidden=\"hidden\">非表示</p><pre>  生\n\t文  </pre><p>終</p>")).unwrap();
    assert_eq!(
        document.chapters[0].text,
        "A B\u{a0}　C D E\nF\n\n  生\n\t文  \n\n終"
    );
}

#[test]
fn image_positions_alt_text_deduplication_and_svg_wrapper_are_preserved() {
    let document = parse(&with_body(r#"<p>前<img src="../images/挿絵.png" alt="説明"/>後<img src="../images/挿絵.png"/></p><svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink"><image xlink:href="../images/cover.png"/></svg>"#)).unwrap();
    let chapter = &document.chapters[0];
    assert_eq!(chapter.text, "前\u{fffc}後\u{fffc}\n\n\u{fffc}");
    assert_eq!(chapter.images.len(), 3);
    assert_eq!(chapter.images[0].alt, "説明");
    assert_eq!(document.images.len(), 2);
    for image in &chapter.images {
        assert_eq!(&chapter.text[image.range.clone()], "\u{fffc}");
    }
    assert!(!chapter.text.contains("説明"));
}

#[test]
fn epub_two_cover_metadata_and_svg_spine_items_work_without_stylesheets() {
    let mut files = fixture();
    edit(&mut files, "OPS/book.opf", "properties=\"cover-image\"", "");
    edit(
        &mut files,
        "OPS/book.opf",
        "</metadata>",
        "<meta name=\"cover\" content=\"cover\"/></metadata>",
    );
    edit(
        &mut files,
        "OPS/book.opf",
        "<itemref idref=\"second\" linear=\"no\"/>",
        "<itemref idref=\"cover\"/>",
    );
    edit(
        &mut files,
        "OPS/book.opf",
        "images/cover.png",
        "images/cover.svg",
    );
    edit(
        &mut files,
        "OPS/book.opf",
        "media-type=\"image/png\"",
        "media-type=\"image/svg+xml\"",
    );
    let cover = files.remove("OPS/images/cover.png").unwrap();
    files.insert("OPS/images/cover.svg".to_owned(), cover);
    let document = parse(&archive(files)).unwrap();
    assert_eq!(document.cover.as_deref(), Some("OPS/images/cover.svg"));
    assert_eq!(document.chapters[1].text, "\u{fffc}");
    assert_eq!(document.chapters[1].images[0].path, "OPS/images/cover.svg");
}

#[test]
fn invalid_truncated_and_crc_damaged_archives_fail() {
    for bytes in [b"".as_slice(), b"not zip", b"PK\x03\x04"] {
        assert_eq!(parse(bytes), Err(EpubError::InvalidArchive));
    }
    let bytes = archive(fixture());
    assert_eq!(
        parse(&bytes[..bytes.len() / 2]),
        Err(EpubError::InvalidArchive)
    );
    let mut bytes = compress(fixture(), CompressionMethod::Stored);
    let at = bytes
        .windows(20)
        .position(|value| value == b"application/epub+zip")
        .unwrap();
    bytes[at] = b'X';
    assert_eq!(parse(&bytes), Err(EpubError::InvalidArchive));
}

#[test]
fn ambiguous_duplicate_names_and_aliases_fail() {
    let mut files = fixture();
    files.insert("OPS/./book.opf".to_owned(), files["OPS/book.opf"].clone());
    assert_eq!(parse(&archive(files)), Err(EpubError::InvalidArchive));
    let mut files = fixture();
    files.insert("OPS/boox.opf".to_owned(), files["OPS/book.opf"].clone());
    let mut bytes = archive(files);
    let name = b"OPS/boox.opf";
    let positions: Vec<_> = bytes
        .windows(name.len())
        .enumerate()
        .filter_map(|(i, value)| (value == name).then_some(i))
        .collect();
    for at in positions {
        bytes[at..at + name.len()].copy_from_slice(b"OPS/book.opf");
    }
    assert_eq!(parse(&bytes), Err(EpubError::InvalidArchive));
}

#[test]
fn unsafe_archive_names_and_resource_references_fail_without_extraction() {
    for path in [
        "../escape",
        "/absolute",
        "C:/file",
        "OPS/../../escape",
        "OPS\\file",
        "OPS/\0file",
    ] {
        let mut files = fixture();
        files.insert(path.to_owned(), vec![1]);
        assert_eq!(
            parse(&archive(files)),
            Err(EpubError::InvalidPath),
            "{path}"
        );
    }
    for href in [
        "../../../escape",
        "%2e%2e/%2e%2e/%2e%2e/file",
        "/absolute",
        "%2Fabsolute",
        "https://example.org/a.png",
        "file:///tmp/a",
        "bad%XY.png",
        "../images/cover.png?x",
        "../images/cover.png#x",
        "bad%00.png",
    ] {
        assert_eq!(
            parse(&with_body(&format!("<img src=\"{href}\"/>"))),
            Err(EpubError::InvalidPath),
            "{href}"
        );
    }
}

#[test]
fn missing_resources_and_broken_manifest_fail_the_entire_document() {
    for path in [
        "mimetype",
        "META-INF/container.xml",
        "OPS/book.opf",
        "OPS/text/z-first.xhtml",
        "OPS/text/a-second.xhtml",
        "OPS/images/cover.png",
        "OPS/images/挿絵.png",
    ] {
        let mut files = fixture();
        files.remove(path);
        assert_eq!(
            parse(&archive(files)),
            Err(EpubError::MissingResource),
            "{path}"
        );
    }
    for (from, to) in [
        ("idref=\"first\"", "idref=\"absent\""),
        ("id=\"second\"", "id=\"first\""),
        ("<spine>", "<not-spine>"),
        ("application/xhtml+xml", "text/plain"),
    ] {
        let mut files = fixture();
        edit(&mut files, "OPS/book.opf", from, to);
        assert!(parse(&archive(files)).is_err());
    }
    assert_eq!(parse(&with_body("<img/>")), Err(EpubError::MissingResource));
}

#[test]
fn percent_encoded_relative_paths_and_parent_directory_resolution_work_once() {
    let mut files = fixture();
    edit(
        &mut files,
        "OPS/book.opf",
        "text/z-first.xhtml",
        "./text/../text/z-first.xhtml",
    );
    let document = parse(&archive(files)).unwrap();
    assert_eq!(document.chapters[0].images[0].path, "OPS/images/挿絵.png");
    let mut files = fixture();
    files.insert("OPS/images/%20.png".to_owned(), vec![1]);
    edit(
        &mut files,
        "OPS/book.opf",
        "</manifest>",
        "<item id=\"escaped\" href=\"images/%2520.png\" media-type=\"image/png\"/></manifest>",
    );
    files.insert(
        "OPS/text/z-first.xhtml".to_owned(),
        xhtml("<img src=\"../images/%2520.png\"/>"),
    );
    assert!(parse(&archive(files))
        .unwrap()
        .images
        .contains_key("OPS/images/%20.png"));
}

#[test]
fn utf8_bom_and_bom_marked_utf16_xml_are_strictly_decoded() {
    for little in [true, false] {
        let mut files = fixture();
        let text = String::from_utf8(xhtml("<p>𠮷😀と日本語</p>")).unwrap();
        let bytes: Vec<u8> = if little {
            vec![0xff, 0xfe]
        } else {
            vec![0xfe, 0xff]
        }
        .into_iter()
        .chain(text.encode_utf16().flat_map(|unit| {
            if little {
                unit.to_le_bytes()
            } else {
                unit.to_be_bytes()
            }
        }))
        .collect();
        files.insert("OPS/text/z-first.xhtml".to_owned(), bytes);
        assert_eq!(
            parse(&archive(files)).unwrap().chapters[0].text,
            "𠮷😀と日本語"
        );
    }
    let mut files = fixture();
    let bytes = [vec![0xef, 0xbb, 0xbf], xhtml("<p>BOM</p>")].concat();
    files.insert("OPS/text/z-first.xhtml".to_owned(), bytes);
    assert_eq!(parse(&archive(files)).unwrap().chapters[0].text, "BOM");
    for bytes in [vec![0xff, 0xfe, 0], vec![0xff], vec![0xfe, 0xff, 0xd8, 0]] {
        let mut files = fixture();
        files.insert("OPS/text/z-first.xhtml".to_owned(), bytes);
        assert_eq!(parse(&archive(files)), Err(EpubError::InvalidXml));
    }
}

#[test]
fn xml_errors_and_external_entities_fail_without_a_resolver() {
    for body in [
        "<p>unclosed",
        "<p>&unknown;</p>",
        "<p>&#xD800;</p>",
        "<p><em>mismatch</p></em>",
    ] {
        assert_eq!(parse(&with_body(body)), Err(EpubError::InvalidXml));
    }
    let mut files = fixture();
    files.insert("OPS/text/z-first.xhtml".to_owned(), r#"<!DOCTYPE html SYSTEM "https://example.invalid/never-fetched.dtd"><html xmlns="http://www.w3.org/1999/xhtml"><body><p>本文</p></body></html>"#.as_bytes().to_vec());
    assert_eq!(
        parse(&archive(files.clone())).unwrap().chapters[0].text,
        "本文"
    );
    files.insert("OPS/text/z-first.xhtml".to_owned(), br#"<!DOCTYPE html [<!ENTITY secret SYSTEM "file:///etc/passwd">]><html xmlns="http://www.w3.org/1999/xhtml"><body>&secret;</body></html>"#.to_vec());
    assert_eq!(parse(&archive(files)), Err(EpubError::InvalidXml));
}

#[test]
fn resource_limits_cover_source_entries_expansion_nodes_and_depth() {
    let bytes = archive(fixture());
    let limits = EpubLimits {
        source_bytes: bytes.len() - 1,
        ..EpubLimits::default()
    };
    assert_eq!(
        parse_with_limits(&bytes, limits),
        Err(EpubError::LimitExceeded)
    );
    for limits in [
        EpubLimits {
            entries: 2,
            ..EpubLimits::default()
        },
        EpubLimits {
            entry_bytes: 10,
            ..EpubLimits::default()
        },
        EpubLimits {
            expanded_bytes: 1000,
            ..EpubLimits::default()
        },
        EpubLimits {
            depth: 3,
            ..EpubLimits::default()
        },
    ] {
        assert_eq!(
            parse_with_limits(&bytes, limits),
            Err(EpubError::LimitExceeded)
        );
    }
    assert_eq!(
        parse_with_limits(
            &bytes,
            EpubLimits {
                xml_nodes: 2,
                ..EpubLimits::default()
            }
        ),
        Err(EpubError::InvalidXml)
    );
    let bytes = with_body(&format!("<p>{}</p>", "日".repeat(100_000)));
    assert_eq!(
        parse_with_limits(
            &bytes,
            EpubLimits {
                entry_bytes: 10_000,
                ..EpubLimits::default()
            }
        ),
        Err(EpubError::LimitExceeded)
    );
    let bytes = with_body(&format!(
        "{}字{}",
        "<span>".repeat(500),
        "</span>".repeat(500)
    ));
    assert_eq!(parse(&bytes), Err(EpubError::LimitExceeded));
}

#[test]
fn encryption_of_used_content_fails_but_unused_obfuscated_fonts_do_not() {
    for (path, expected) in [
        ("OPS/font.otf", false),
        ("OPS/text/z-first.xhtml", true),
        ("OPS/images/cover.png", true),
        ("OPS/images/挿絵.png", true),
    ] {
        let mut files = fixture();
        files.insert("META-INF/encryption.xml".to_owned(), format!(r#"<encryption xmlns="urn:oasis:names:tc:opendocument:xmlns:container"><EncryptedData xmlns="http://www.w3.org/2001/04/xmlenc#"><CipherData><CipherReference URI="{path}"/></CipherData></EncryptedData></encryption>"#).into_bytes());
        let result = parse(&archive(files));
        if expected {
            assert_eq!(result, Err(EpubError::EncryptedResource));
        } else {
            assert!(result.is_ok());
        }
    }
}

#[test]
fn empty_chapters_are_omitted_image_only_chapters_remain_and_empty_book_fails() {
    let mut files = fixture();
    files.insert("OPS/text/z-first.xhtml".to_owned(), xhtml("<p> \n </p>"));
    assert_eq!(parse(&archive(files.clone())).unwrap().chapters.len(), 1);
    files.insert("OPS/text/a-second.xhtml".to_owned(), xhtml("<p/>"));
    assert_eq!(parse(&archive(files.clone())), Err(EpubError::EmptyContent));
    files.insert(
        "OPS/text/z-first.xhtml".to_owned(),
        xhtml("<img src=\"../images/cover.png\"/>"),
    );
    let document = parse(&archive(files)).unwrap();
    assert_eq!(document.chapters.len(), 1);
    assert_eq!(document.chapters[0].text, "\u{fffc}");
}

#[test]
fn all_presets_reconstruct_text_and_never_cross_epub_chapter_boundaries() {
    let document = parse(&with_body(&format!(
        "<h1>章</h1>{}",
        "<p>日本語𠮷😀と<ruby>漢字<rt>かんじ</rt></ruby>。</p>".repeat(300)
    )))
    .unwrap();
    let chapters: Vec<_> = document
        .chapters
        .iter()
        .enumerate()
        .map(|(index, chapter)| ChapterText {
            source_id: SourceId::try_from(1).unwrap(),
            chapter_id: ChapterId::try_from(index as i64 + 1).unwrap(),
            text: chapter.text.clone(),
        })
        .collect();
    for length in PageLength::ALL {
        let pages = LogicalPages::new(&chapters, length).unwrap();
        assert!(pages.pages().len() > 2);
        for chapter in &chapters {
            let text: String = pages
                .pages()
                .iter()
                .filter(|page| page.chapter_id == chapter.chapter_id)
                .map(|page| page.slice(chapter).unwrap())
                .collect();
            assert_eq!(text, chapter.text);
        }
    }
}

#[test]
fn many_paragraphs_and_nested_annotations_keep_valid_byte_ranges() {
    let document = parse(&with_body(
        &"<p>𠮷😀<em><ruby>漢字<rt>かんじ</rt></ruby></em>。</p>".repeat(10_000),
    ))
    .unwrap();
    let chapter = &document.chapters[0];
    assert_eq!(chapter.spans.len(), 10_000);
    for span in &chapter.spans {
        assert_eq!(&chapter.text[span.range.clone()], "漢字");
    }
}

#[test]
fn xml_preflight_counts_elements_without_counting_comments_cdata_or_attributes() {
    let body = format!(
        "{}<p data-value=\"&lt;tag&gt;\"><!-- <fake><fake> --><![CDATA[<字>&#xD800;]]></p>{}",
        "<span>".repeat(60),
        "</span>".repeat(60)
    );
    let document = parse(&with_body(&body)).unwrap();
    assert_eq!(document.chapters[0].text, "<字>&#xD800;");
    let body = format!("{}字{}", "<span>".repeat(10_000), "</span>".repeat(10_000));
    assert_eq!(
        parse_with_limits(
            &with_body(&body),
            EpubLimits {
                depth: usize::MAX,
                ..EpubLimits::default()
            }
        ),
        Err(EpubError::LimitExceeded)
    );
    assert_eq!(
        parse(&with_body(
            "<!DOCTYPE p [<!ENTITY inject '<p>text</p>'>]><p>&inject;</p>"
        )),
        Err(EpubError::InvalidXml)
    );
}

#[test]
fn xml_encoding_declarations_must_match_supported_original_bytes() {
    for declaration in ["utf-16", "Shift_JIS", "ISO-8859-1"] {
        let mut files = fixture();
        files.insert(
            "OPS/text/z-first.xhtml".to_owned(),
            [
                format!("<?xml version=\"1.0\" encoding=\"{declaration}\"?>").into_bytes(),
                xhtml("<p>本文</p>"),
            ]
            .concat(),
        );
        assert_eq!(parse(&archive(files)), Err(EpubError::InvalidXml));
    }
    for little in [true, false] {
        let mut files = fixture();
        let text = format!(
            "<?xml version=\"1.0\" encoding=\"UTF-16\"?>{}",
            String::from_utf8(xhtml("<p>本文𠮷</p>")).unwrap()
        );
        let bytes: Vec<_> = text
            .encode_utf16()
            .flat_map(|u| {
                if little {
                    u.to_le_bytes()
                } else {
                    u.to_be_bytes()
                }
            })
            .collect();
        files.insert("OPS/text/z-first.xhtml".to_owned(), bytes);
        assert_eq!(parse(&archive(files)).unwrap().chapters[0].text, "本文𠮷");
    }
}

#[test]
fn unused_remote_stylesheets_and_svg_scripts_never_enter_canonical_text() {
    let mut files = fixture();
    edit(
        &mut files,
        "OPS/book.opf",
        "href=\"style.css\"",
        "href=\"https://example.invalid/never-fetch.css\"",
    );
    files.insert("OPS/text/z-first.xhtml".to_owned(), xhtml("<p>本文</p><svg xmlns=\"http://www.w3.org/2000/svg\"><script>poison</script><style>poison</style></svg>"));
    assert_eq!(parse(&archive(files)).unwrap().chapters[0].text, "本文");
}

#[test]
fn symlink_and_zip_encrypted_resources_fail_explicitly() {
    let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
    for (name, bytes) in fixture() {
        writer
            .start_file(
                name,
                SimpleFileOptions::default().compression_method(CompressionMethod::Stored),
            )
            .unwrap();
        writer.write_all(&bytes).unwrap();
    }
    writer
        .add_symlink("link", "OPS/book.opf", SimpleFileOptions::default())
        .unwrap();
    assert_eq!(
        parse(&writer.finish().unwrap().into_inner()),
        Err(EpubError::InvalidArchive)
    );

    let mut bytes = compress(fixture(), CompressionMethod::Stored);
    // Mark the actual mimetype local/central headers as encrypted. No password
    // or encryption implementation is needed to prove it is rejected pre-read.
    for signature in [b"PK\x03\x04", b"PK\x01\x02"] {
        let header = if signature == b"PK\x03\x04" { 30 } else { 46 };
        let flag = if signature == b"PK\x03\x04" { 6 } else { 8 };
        let positions: Vec<_> = bytes
            .windows(4)
            .enumerate()
            .filter_map(|(at, value)| {
                (value == signature && bytes.get(at + header..at + header + 8) == Some(b"mimetype"))
                    .then_some(at)
            })
            .collect();
        for at in positions {
            bytes[at + flag] |= 1;
        }
    }
    assert_eq!(parse(&bytes), Err(EpubError::EncryptedResource));
}

#[test]
fn invalid_container_and_duplicate_manifest_paths_fail() {
    let mut files = fixture();
    edit(
        &mut files,
        "META-INF/container.xml",
        "<container ",
        "<invalid ",
    );
    edit(
        &mut files,
        "META-INF/container.xml",
        "</container>",
        "</invalid>",
    );
    assert_eq!(parse(&archive(files)), Err(EpubError::InvalidPackage));
    let mut files = fixture();
    edit(
        &mut files,
        "OPS/book.opf",
        "text/a-second.xhtml",
        "./text/z-first.xhtml",
    );
    assert_eq!(parse(&archive(files)), Err(EpubError::InvalidPackage));
}

#[test]
fn repeated_cover_and_text_spine_items_keep_first_occurrence_and_annotations_once() {
    for cover_only in [false, true] {
        let mut files = fixture();
        if cover_only {
            files.insert(
                "OPS/text/z-first.xhtml".to_owned(),
                xhtml("<p><img src='../images/cover.png' alt='表紙'/></p>"),
            );
        }
        let expected = parse(&archive(files.clone())).unwrap();
        for spine in [
            "<itemref idref='first'/><itemref idref='first'/><itemref idref='second'/>",
            "<itemref idref='first'/><itemref idref='second'/><itemref idref='first'/><itemref idref='second'/>",
            "<itemref idref='second'/><itemref idref='first'/><itemref idref='second'/>",
        ] {
            let mut repeated = files.clone();
            edit(
                &mut repeated,
                "OPS/book.opf",
                "<itemref idref=\"first\"/><itemref idref=\"second\" linear=\"no\"/>",
                spine,
            );
            let mut expected = expected.clone();
            if spine.starts_with("<itemref idref='second'") {
                expected.chapters.reverse();
            }
            assert_eq!(parse(&archive(repeated)).unwrap(), expected);
        }
    }
}
