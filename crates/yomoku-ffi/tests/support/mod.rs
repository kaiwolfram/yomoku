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
                is_japanese: true,
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
    YomokuBridge::new(
        path,
        Box::new(|| Ok(Box::new(TestAnalyzer))),
        Box::new(|| Err(DocumentError::Unavailable)),
        callback,
    )
}
