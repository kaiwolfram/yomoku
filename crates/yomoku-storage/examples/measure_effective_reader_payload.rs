//! Reproducible synthetic JSON size review before defining an effective bridge.
//! These are domain/storage JSON sizes, not JNI wire sizes or performance claims.

use yomoku_core::{
    analysis::{AnalysisProfile, AnalyzedToken, ChapterAnalysis, SplitMode},
    document::{ChapterText, PastedDocument, TextAnchor, TextOffset, TextRange},
    effective_reader::{
        EffectiveReaderChapter, EffectiveReaderTokenKey, EffectiveReaderTokenSnapshot,
    },
    effective_session::{EffectiveReaderSelection, EffectiveReaderSession},
    pagination::PageLength,
    reader::{ReaderAnalysis, ReaderPage, ReaderTokenKey},
    vocabulary::Surface,
};

fn main() {
    println!("characters,segments,source_utf8_bytes,page_characters,key_json_bytes,page_token_json_bytes,compact_page_json_bytes,compact_identity_json_bytes,compact_selection_json_bytes");
    for repeats in [250, 2_500, 25_000] {
        let chapter = ChapterText {
            source_id: 1.try_into().unwrap(),
            chapter_id: 2.try_into().unwrap(),
            text: "日本語。".repeat(repeats),
        };
        let mut tokens = Vec::with_capacity(repeats * 2);
        for offset in (0..chapter.text.len()).step_by(12) {
            for (start, end, japanese) in
                [(offset, offset + 9, true), (offset + 9, offset + 12, false)]
            {
                tokens.push(AnalyzedToken {
                    anchor: TextAnchor {
                        source_id: chapter.source_id,
                        chapter_id: chapter.chapter_id,
                        range: TextRange {
                            start: TextOffset::at(&chapter.text, start).unwrap(),
                            end: TextOffset::at(&chapter.text, end).unwrap(),
                        },
                    },
                    surface: chapter.text[start..end].into(),
                    is_japanese: japanese,
                    reading: japanese.then(|| "ニホンゴ".into()),
                    dictionary_form: japanese.then(|| Surface::new("日本語").unwrap()),
                });
            }
        }
        let original = ChapterAnalysis {
            profile: AnalysisProfile {
                engine_revision: "synthetic-engine@1".into(),
                dictionary_revision: "synthetic-full@1".into(),
            },
            mode: SplitMode::C,
            tokens,
        };
        let projection =
            EffectiveReaderChapter::resolve(&chapter, 10.try_into().unwrap(), &original).unwrap();
        let document = PastedDocument {
            document_id: 1.try_into().unwrap(),
            title: "Synthetic payload fixture".into(),
            finished_at_least_once: false,
            chapter: chapter.clone(),
            reading_position: None,
        };
        let page = ReaderPage::open(&document, PageLength::default()).unwrap();
        let key = EffectiveReaderTokenKey {
            document_id: document.document_id,
            page: page.anchor,
            segmentation: projection.segmentation().clone(),
        };
        let tokens = projection.tokens_on_page(page.anchor).unwrap();
        let mut session = EffectiveReaderSession::default();
        let request = session
            .begin_load(ReaderTokenKey {
                document_id: document.document_id,
                page: page.anchor,
                analysis: ReaderAnalysis::Available(key.segmentation.original_analysis_id),
            })
            .unwrap();
        assert!(session.accept(
            request,
            EffectiveReaderTokenSnapshot {
                document_id: document.document_id,
                page: page.anchor,
                segmentation: Some(key.segmentation.clone()),
                tokens: tokens.clone(),
            },
        ));
        let compact = session.page().unwrap();
        let selection = EffectiveReaderSelection {
            page: compact.identity,
            segment_index: compact.tokens[0].segment_index,
        };
        println!(
            "{},{},{},{},{},{},{},{},{}",
            chapter.text.chars().count(),
            key.segmentation.segments.len(),
            chapter.text.len(),
            page.text.chars().count(),
            serde_json::to_vec(&key).unwrap().len(),
            serde_json::to_vec(&tokens).unwrap().len(),
            serde_json::to_vec(compact).unwrap().len(),
            serde_json::to_vec(&compact.identity).unwrap().len(),
            serde_json::to_vec(&selection).unwrap().len(),
        );
    }
}
