//! Test-only fixture exchange through the same generated codecs as the FFI.
//! No diagnostic command or text echo method is added to the production bridge.

use std::{env, error::Error, fs, path::Path};

use crux_core::bridge::{BincodeFfiFormat, FfiFormat};
use yomoku_core::document::{
    AnalysisId, ChapterId, ChapterText, DocumentId, ReaderDocument, SourceId, TextAnchor,
    TextOffset, TextRange,
};
use yomoku_core::{
    document::PastedDocument,
    effective_reader::EffectiveTokenEligibility,
    effective_session::{
        EffectivePageIdentity, EffectivePageToken, EffectiveReaderPage, EffectiveReaderSelection,
        EffectiveReaderWordState, EffectiveWordDetails,
    },
    pagination::PageLength,
    reader::ReaderPage,
    reader_images::{ReaderChapterImages, ReaderImageAsset, ReaderInlineImage},
};
use yomoku_core::{pagination::ReadingPosition, reader::ReaderAnalysis};

#[path = "unicode-fixtures/anki.rs"]
mod anki;
#[path = "unicode-fixtures/anki_settings.rs"]
mod anki_settings;
#[path = "unicode-fixtures/effective_overrides.rs"]
mod effective_overrides;
#[path = "unicode-fixtures/statistics.rs"]
mod statistics;
#[path = "unicode-fixtures/vocabulary.rs"]
mod vocabulary;

const CASES: &[(&str, &str)] = &[
    ("", ""),
    ("日本語の文章。", "日本語"),
    ("A𠮷野家😀Z", "𠮷野家😀"),
    ("か\u{3099} と が / é e\u{301}", "か\u{3099}"),
    ("前👩‍👩‍👧‍👦後", "👩‍👩‍👧‍👦"),
    ("first\r\n第二行\n\n終\0", "第二行\n\n"),
    ("終端", "端"),
];

fn encode<T: serde::Serialize>(value: &T) -> Vec<u8> {
    let mut bytes = Vec::new();
    BincodeFfiFormat::serialize(&mut bytes, value).unwrap();
    bytes
}

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<_> = env::args().skip(1).collect();
    if args.len() != 2 || !matches!(args[0].as_str(), "write" | "verify") {
        return Err("usage: unicode-fixtures <write|verify> <fixture-directory>".into());
    }
    let dir = Path::new(&args[1]);
    let writing = args[0] == "write";
    statistics::exchange(dir, writing)?;
    vocabulary::exchange(dir, writing)?;
    anki::exchange(dir, writing)?;
    anki_settings::exchange(dir, writing)?;
    if writing {
        fs::create_dir_all(dir)?;
        fs::write(
            dir.join("document.bin"),
            encode(&DocumentId::try_from(i64::MAX)?),
        )?;
    } else {
        let document: DocumentId =
            BincodeFfiFormat::deserialize(&fs::read(dir.join("document-return.bin"))?)?;
        assert_eq!(i64::from(document), i64::MAX);
        for name in ["zero", "negative"] {
            assert!(BincodeFfiFormat::deserialize::<DocumentId>(&fs::read(
                dir.join(format!("invalid-id-{name}.bin"))
            )?)
            .is_err());
        }
        assert!(BincodeFfiFormat::deserialize::<TextOffset>(&fs::read(
            dir.join("invalid-offset.bin")
        )?)
        .is_err());
    }
    for (index, analysis) in [
        ReaderAnalysis::Available(AnalysisId::try_from(i64::MAX)?),
        ReaderAnalysis::Missing,
    ]
    .into_iter()
    .enumerate()
    {
        if writing {
            fs::write(dir.join(format!("analysis-{index}.bin")), encode(&analysis))?;
        } else {
            let returned: ReaderAnalysis = BincodeFfiFormat::deserialize(&fs::read(
                dir.join(format!("analysis-return-{index}.bin")),
            )?)?;
            assert_eq!(returned, analysis);
        }
    }
    let ordered = ReaderDocument {
        document_id: DocumentId::try_from(i64::MAX)?,
        title: "章 📖".into(),
        images: Vec::new(),
        source_ruby: Vec::new(),
        formatting: vec![yomoku_core::reader_formatting::ReaderChapterFormatting {
            chapter_id: ChapterId::try_from(i64::MAX - 2)?,
            spans: [
                yomoku_core::reader_formatting::ReaderTextStyle::Heading(2),
                yomoku_core::reader_formatting::ReaderTextStyle::Emphasis,
                yomoku_core::reader_formatting::ReaderTextStyle::Strong,
                yomoku_core::reader_formatting::ReaderTextStyle::ExternalLink(
                    "https://example.org/本".into(),
                ),
            ]
            .into_iter()
            .map(|style| yomoku_core::reader_formatting::ReaderTextSpan {
                range: yomoku_core::document::TextRange {
                    start: TextOffset::START,
                    end: TextOffset::at(CASES[2].0, CASES[2].0.len()).unwrap(),
                },
                style,
            })
            .collect(),
        }],
        finished_at_least_once: true,
        chapters: CASES
            .iter()
            .enumerate()
            .map(|(index, (text, _))| ChapterText {
                source_id: SourceId::try_from((1 << 53) + 1).unwrap(),
                chapter_id: ChapterId::try_from(i64::MAX - index as i64).unwrap(),
                text: (*text).into(),
            })
            .collect(),
        reading_position: Some(ReadingPosition {
            source_id: SourceId::try_from((1 << 53) + 1)?,
            chapter_id: ChapterId::try_from(i64::MAX - 2)?,
            offset: TextOffset::at(CASES[2].0, "A𠮷".len())?,
        }),
    };
    ordered.validate()?;
    if writing {
        fs::write(dir.join("ordered.bin"), encode(&ordered))?;
    } else {
        let returned: ReaderDocument =
            BincodeFfiFormat::deserialize(&fs::read(dir.join("ordered-return.bin"))?)?;
        assert_eq!(returned, ordered);
        returned.validate()?;
    }
    let image_text = "😀𠮷か\u{3099}\0\u{fffc}後";
    let mut illustrated: ReaderDocument = PastedDocument {
        document_id: DocumentId::try_from(i64::MAX)?,
        title: "挿絵 📖".into(),
        finished_at_least_once: false,
        chapter: ChapterText {
            source_id: SourceId::try_from((1 << 53) + 1)?,
            chapter_id: ChapterId::try_from(i64::MAX)?,
            text: image_text.into(),
        },
        reading_position: None,
    }
    .into();
    let start = image_text.find('\u{fffc}').unwrap();
    illustrated.images = vec![ReaderChapterImages {
        chapter_id: illustrated.chapters[0].chapter_id,
        images: vec![ReaderInlineImage {
            range: TextRange {
                start: TextOffset::at(image_text, start)?,
                end: TextOffset::at(image_text, start + 3)?,
            },
            asset: ReaderImageAsset {
                path: "OPS/images/挿絵😀.png".into(),
                media_type: "image/png".into(),
            },
            alt: "説明𠮷か\u{3099}\0".into(),
        }],
    }];
    illustrated.source_ruby = vec![yomoku_core::reader_ruby::ReaderChapterRuby {
        chapter_id: illustrated.chapters[0].chapter_id,
        ruby: vec![yomoku_core::reader_ruby::ReaderRuby {
            range: TextRange {
                start: TextOffset::at(image_text, "😀".len())?,
                end: TextOffset::at(image_text, "😀𠮷か\u{3099}".len())?,
            },
            reading: "よし𠮷か\u{3099}\0".into(),
        }],
    }];
    illustrated.validate()?;
    let illustrated_page = ReaderPage::open_document(&illustrated, PageLength::default())?;
    // Standalone prerequisite schemas use the actual generated shell codec.
    // Maximum revisions/indices and literal unresolved metadata stay lossless.
    let effective_page = EffectiveReaderPage {
        original_boundaries: vec![yomoku_core::effective_reader::OriginalBoundaryToken {
            anchor: illustrated_page.anchor,
            display_range: illustrated_page.anchor.range,
            surface: image_text.into(),
        }],
        identity: EffectivePageIdentity {
            revision: u64::MAX,
            base: yomoku_core::reader::ReaderTokenKey {
                document_id: illustrated.document_id,
                page: illustrated_page.anchor,
                analysis: ReaderAnalysis::Available(AnalysisId::try_from(i64::MAX)?),
            },
        },
        tokens: [
            EffectiveTokenEligibility::AnalyzerJapanese,
            EffectiveTokenEligibility::ManualJapaneseSpan,
        ]
        .into_iter()
        .enumerate()
        .map(|(index, eligibility)| EffectivePageToken {
            segment_index: index as u64,
            anchor: illustrated_page.anchor,
            display_range: illustrated_page.anchor.range,
            surface: image_text.into(),
            eligibility,
            reading: (index == 0).then(|| "読み𠮷か\u{3099}\0".into()),
            dictionary_form: (index == 0).then(|| "原形𠮷か\u{3099}\0".into()),
            vocabulary: (index == 0).then(|| yomoku_core::reader::ReaderVocabulary {
                id: i64::MAX.try_into().unwrap(),
                status: yomoku_core::vocabulary::VocabularyStatus::Known,
            }),
        })
        .collect(),
    };
    let effective_selection = EffectiveReaderSelection {
        page: effective_page.identity,
        segment_index: u64::MAX,
    };
    effective_overrides::exchange(dir, writing, effective_page.identity)?;
    let effective_word = EffectiveReaderWordState {
        selection: Some(effective_selection),
        details: Some(EffectiveWordDetails {
            selection: effective_selection,
            token: effective_page.tokens[1].clone(),
            encountered: u64::MAX,
            candidates: yomoku_core::dictionary::DictionaryCandidates {
                surface: image_text.into(),
                dictionary_form: None,
                words: vec![],
                names: vec![],
            },
            custom_entry: Some(yomoku_core::reader::ReaderCustomEntry {
                surface: image_text.into(),
                english_meaning: " literal\0meaning 📖 ".into(),
                reading: " よみか\u{3099}\0 ".into(),
            }),
        }),
        error: None,
        custom_entry_error: Some(yomoku_core::storage::DocumentError::Unavailable),
        busy: false,
        custom_entry_pending: true,
    };
    let identity_bytes = encode(&effective_page.identity);
    let selection_bytes = encode(&effective_selection);
    assert_eq!(identity_bytes.len(), 60);
    assert_eq!(selection_bytes.len(), 68);
    let effective_completion = yomoku_core::app::Event::CompleteEffectiveReaderPage {
        identity: effective_page.identity,
        position: illustrated_page.position,
        completed_at_unix_ms: i64::MAX,
    };
    let mut effective_view = crux_core::Core::<yomoku_core::app::Yomoku>::new().view();
    effective_view.effective_reader_tokens = Some(effective_page.clone());
    effective_view.effective_reader_word = Some(effective_word.clone());
    effective_view.effective_reader_status =
        Some(yomoku_core::effective_session::EffectiveStatusState {
            pending: true,
            busy: false,
            error: Some(yomoku_core::storage::DocumentError::Unavailable),
        });
    effective_view.completion_save_pending = true;
    use yomoku_core::app::Event as AppEvent;
    for (index, event) in [
        AppEvent::LoadEffectiveReaderWord(effective_selection),
        AppEvent::SaveEffectiveReaderCustomEntry {
            selection: effective_selection,
            english_meaning: " literal\0meaning 📖 ".into(),
            reading: " よみか\u{3099}\0 ".into(),
        },
        AppEvent::DeleteEffectiveReaderCustomEntry(effective_selection),
        AppEvent::RetryEffectiveReaderCustomEntry,
        AppEvent::CancelEffectiveReaderCustomEntry,
        AppEvent::SetEffectiveReaderStatus {
            selection: effective_selection,
            vocabulary_id: i64::MAX.try_into()?,
            status: yomoku_core::vocabulary::VocabularyStatus::Ignored,
            occurred_at_unix_ms: i64::MAX,
        },
        AppEvent::RetryEffectiveReaderStatus,
        AppEvent::CancelEffectiveReaderStatus,
    ]
    .into_iter()
    .enumerate()
    {
        if writing {
            fs::write(
                dir.join(format!("effective-word-event-{index}.bin")),
                encode(&event),
            )?;
        } else {
            let returned: AppEvent = BincodeFfiFormat::deserialize(&fs::read(
                dir.join(format!("effective-word-event-{index}-return.bin")),
            )?)?;
            assert_eq!(returned, event);
        }
    }
    if writing {
        fs::write(
            dir.join("effective-completion-event.bin"),
            encode(&effective_completion),
        )?;
        fs::write(dir.join("effective-app-view.bin"), encode(&effective_view))?;
        fs::write(dir.join("effective-page.bin"), encode(&effective_page))?;
        fs::write(dir.join("effective-selection.bin"), &selection_bytes)?;
        fs::write(dir.join("effective-word.bin"), encode(&effective_word))?;
        println!(
            "Effective codec: identity={} bytes, selection={} bytes",
            identity_bytes.len(),
            selection_bytes.len()
        );
    } else {
        let returned: yomoku_core::app::Event = BincodeFfiFormat::deserialize(&fs::read(
            dir.join("effective-completion-event-return.bin"),
        )?)?;
        assert_eq!(returned, effective_completion);
        let returned: yomoku_core::app::ViewModel =
            BincodeFfiFormat::deserialize(&fs::read(dir.join("effective-app-view-return.bin"))?)?;
        assert_eq!(returned, effective_view);
        let returned: EffectiveReaderPage =
            BincodeFfiFormat::deserialize(&fs::read(dir.join("effective-page-return.bin"))?)?;
        assert_eq!(returned, effective_page);
        let returned: EffectiveReaderSelection =
            BincodeFfiFormat::deserialize(&fs::read(dir.join("effective-selection-return.bin"))?)?;
        assert_eq!(returned, effective_selection);
        let returned: EffectiveReaderWordState =
            BincodeFfiFormat::deserialize(&fs::read(dir.join("effective-word-return.bin"))?)?;
        assert_eq!(returned, effective_word);
    }
    if writing {
        fs::write(dir.join("illustrated.bin"), encode(&illustrated))?;
        fs::write(dir.join("illustrated-page.bin"), encode(&illustrated_page))?;
    } else {
        let returned: ReaderDocument =
            BincodeFfiFormat::deserialize(&fs::read(dir.join("illustrated-return.bin"))?)?;
        assert_eq!(returned, illustrated);
        returned.validate()?;
        let returned: ReaderPage =
            BincodeFfiFormat::deserialize(&fs::read(dir.join("illustrated-page-return.bin"))?)?;
        assert_eq!(returned, illustrated_page);
    }
    for (index, &(text, selected)) in CASES.iter().enumerate() {
        let chapter = ChapterText {
            source_id: SourceId::try_from((1 << 53) + 1)?,
            chapter_id: ChapterId::try_from(i64::MAX - index as i64)?,
            text: text.into(),
        };
        let start = text.find(selected).unwrap();
        let end = start + selected.len();
        let anchor = TextAnchor {
            source_id: chapter.source_id,
            chapter_id: chapter.chapter_id,
            range: TextRange {
                start: TextOffset::at(text, start)?,
                end: TextOffset::at(text, end)?,
            },
        };
        if writing {
            fs::write(dir.join(format!("chapter-{index}.bin")), encode(&chapter))?;
            fs::write(dir.join(format!("anchor-{index}.bin")), encode(&anchor))?;
        } else {
            let returned: ChapterText = BincodeFfiFormat::deserialize(&fs::read(
                dir.join(format!("chapter-return-{index}.bin")),
            )?)?;
            assert_eq!(returned, chapter);
            let returned: TextAnchor = BincodeFfiFormat::deserialize(&fs::read(
                dir.join(format!("anchor-return-{index}.bin")),
            )?)?;
            assert_eq!(returned, anchor);
            let tail: TextAnchor =
                BincodeFfiFormat::deserialize(&fs::read(dir.join(format!("tail-{index}.bin")))?)?;
            assert_eq!(tail.range.start, anchor.range.end);
            assert_eq!(tail.range.end, TextOffset::at(text, text.len())?);
            assert_eq!(tail.slice(&chapter)?, &text[end..]);
        }
    }
    println!(
        "PASS: {} Rust/Kotlin Unicode fixtures ({})",
        CASES.len(),
        args[0]
    );
    Ok(())
}
