#![allow(dead_code)]

use yomoku_core::{
    analysis::{AnalysisProfile, AnalyzedToken, ChapterAnalysis, SplitMode},
    document::{PasteText, PastedDocument, TextAnchor, TextOffset, TextRange},
};
use yomoku_storage::{PasteImportOutcome, Storage};

pub fn unanalyzed(
    storage: &mut Storage,
    path: impl AsRef<std::path::Path>,
    text: &PasteText,
) -> PastedDocument {
    let imported = storage
        .import_analyzed_paste(text, 0, |chapter| {
            Ok::<_, std::convert::Infallible>(ChapterAnalysis {
                profile: AnalysisProfile {
                    engine_revision: "fixture@1".into(),
                    dictionary_revision: "fixture@1".into(),
                },
                mode: SplitMode::C,
                tokens: vec![AnalyzedToken {
                    anchor: TextAnchor {
                        source_id: chapter.source_id,
                        chapter_id: chapter.chapter_id,
                        range: TextRange {
                            start: TextOffset::START,
                            end: TextOffset::at(&chapter.text, chapter.text.len()).unwrap(),
                        },
                    },
                    surface: chapter.text.clone(),
                    is_japanese: false,
                    reading: None,
                    dictionary_form: None,
                }],
            })
        })
        .unwrap();
    let document = match imported {
        PasteImportOutcome::Created(value) => value.document,
        _ => panic!("fresh fixture"),
    };
    let connection = rusqlite::Connection::open(path).unwrap();
    connection
        .pragma_update(None, "foreign_keys", true)
        .unwrap();
    connection
        .execute(
            "DELETE FROM chapter_analyses WHERE chapter_id = ?1",
            [i64::from(document.chapter.chapter_id)],
        )
        .unwrap();
    document
}

pub fn prepare(
    storage: &mut Storage,
    intent: yomoku_core::completion::PageCompletionIntent,
) -> Result<yomoku_core::effective_reader::EffectivePageCompletion, yomoku_storage::StorageError> {
    let key = storage
        .load_effective_reader_tokens(intent.document_id, intent.page)?
        .and_then(|s| s.key())
        .ok_or(yomoku_storage::StorageError::InvalidAnalysis)?;
    storage.prepare_effective_page_completion(intent, &key)
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SrtFile {
    pub original_bytes: Vec<u8>,
    pub encoding: String,
}
pub fn srt_file(
    connection: &rusqlite::Connection,
    id: yomoku_core::document::DocumentId,
) -> rusqlite::Result<Option<SrtFile>> {
    use rusqlite::OptionalExtension;
    connection.query_row("SELECT f.original_bytes, f.text_encoding FROM imported_files f JOIN documents d ON d.id = f.document_id JOIN sources s ON s.id = d.source_id WHERE d.id = ?1 AND s.kind = 'srt'", [i64::from(id)], |row| {
        let encoding: String = row.get(1)?;
        Ok(SrtFile { original_bytes: row.get(0)?, encoding })
    }).optional()
}
