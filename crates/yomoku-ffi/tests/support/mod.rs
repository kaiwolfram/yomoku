use yomoku_core::{
    analysis::{AnalysisProfile, AnalyzedToken, ChapterAnalysis, SplitMode},
    document::{ChapterText, TextAnchor, TextOffset, TextRange},
    storage::DocumentError,
};
use yomoku_ffi::{FfiError, YomokuBridge};
use yomoku_runtime::{AnalyzerLifecycle, PasteAnalyzer};
struct TestAnalyzer;
impl PasteAnalyzer for TestAnalyzer {
    fn analyze(&mut self, chapter: &ChapterText) -> Result<ChapterAnalysis, DocumentError> {
        Ok(ChapterAnalysis {
            profile: AnalysisProfile {
                engine_revision: "ffi-test".into(),
                dictionary_revision: "ffi-test".into(),
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
    }
}
pub fn bridge(
    path: std::path::PathBuf,
    callback: impl Fn(Result<Vec<u8>, FfiError>) + Send + Sync + 'static,
) -> Result<(YomokuBridge, AnalyzerLifecycle), FfiError> {
    let index = path.with_extension("words.sqlite3");
    YomokuBridge::new(
        path,
        Box::new(|| Ok(Box::new(TestAnalyzer))),
        Box::new(move || {
            let sql = rusqlite::Connection::open(&index).map_err(|_| DocumentError::Unavailable)?;
            sql.execute_batch("PRAGMA application_id=0x594f4449; PRAGMA user_version=1;
                CREATE TABLE IF NOT EXISTS entries(source TEXT, entry_id INTEGER, payload TEXT);
                CREATE TABLE IF NOT EXISTS forms(surface TEXT COLLATE BINARY, source TEXT, entry_id INTEGER);").map_err(|_| DocumentError::Unavailable)?;
            Ok(index.clone())
        }),
        callback,
    )
}
