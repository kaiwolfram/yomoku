//! Test-only fixture exchange through the same generated codecs as the FFI.
//! No diagnostic command or text echo method is added to the production bridge.

use std::{env, error::Error, fs, path::Path};

use crux_core::bridge::{BincodeFfiFormat, FfiFormat};
use yomoku_core::document::{
    ChapterId, ChapterText, DocumentId, SourceId, TextAnchor, TextOffset, TextRange,
};

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
