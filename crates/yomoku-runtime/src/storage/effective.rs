//! Effective commands use the same connection/queue/dictionary as legacy reads.

use std::sync::Arc;

use yomoku_core::{
    effective_storage::{EffectiveStorageOperation, EffectiveStorageResult},
    reader::ReaderAnalysis,
    storage::DocumentError,
};
use yomoku_storage::{dictionary::Dictionary, Storage};

use crate::imports::Imports;

use super::{map_error, map_word_error, word_dictionary, WordDictionaryLoader};

pub(super) fn execute(
    storage: &mut Storage,
    imports: &mut Option<Imports>,
    dictionary: &mut Option<Dictionary>,
    word_loader: &mut Option<WordDictionaryLoader>,
    operation: EffectiveStorageOperation,
) -> Result<EffectiveStorageResult, DocumentError> {
    match operation {
        EffectiveStorageOperation::LoadTokens(request) => {
            let base = request.base();
            let snapshot = storage
                .load_effective_reader_tokens(base.document_id, base.page)
                .map_err(map_error)?
                .ok_or(DocumentError::NotFound)?;
            let analysis = snapshot
                .segmentation
                .as_ref()
                .map_or(ReaderAnalysis::Missing, |segmentation| {
                    ReaderAnalysis::Available(segmentation.original_analysis_id)
                });
            if analysis != base.analysis {
                return Err(DocumentError::InvalidDocument);
            }
            Ok(EffectiveStorageResult::TokensLoaded(request, snapshot))
        }
        EffectiveStorageOperation::SetStatus {
            command,
            vocabulary_id,
            status,
            occurred_at_unix_ms,
        } => storage
            .set_effective_reader_status(
                command.key(),
                command.selection().segment_index,
                vocabulary_id,
                status,
                occurred_at_unix_ms,
            )
            .map(|entry| EffectiveStorageResult::StatusChanged(command.selection(), entry))
            .map_err(|error| map_word_error(error, dictionary)),
        EffectiveStorageOperation::LoadWord(command) => {
            let selection = command.selection();
            let key = command.key();
            let mut component_document = None;
            for mode in [
                yomoku_core::analysis::SplitMode::A,
                yomoku_core::analysis::SplitMode::B,
            ] {
                if !storage
                    .has_component_analysis(key.page.chapter_id, &key.segmentation.profile, mode)
                    .map_err(map_error)?
                {
                    if component_document.is_none() {
                        component_document = Some(
                            storage
                                .load_document_reader_snapshot(key.document_id)
                                .map_err(map_error)?
                                .ok_or(DocumentError::NotFound)?
                                .document,
                        );
                    }
                    let chapter = component_document
                        .as_ref()
                        .expect("loaded document")
                        .chapter(key.page.chapter_id)
                        .map_err(|_| DocumentError::InvalidDocument)?;
                    let analysis = imports
                        .as_mut()
                        .ok_or(DocumentError::Unavailable)?
                        .analyze_mode(chapter, mode, &key.segmentation.profile)?;
                    analysis
                        .validate(chapter)
                        .map_err(|_| DocumentError::InvalidDocument)?;
                    if analysis.mode != mode || analysis.profile != key.segmentation.profile {
                        return Err(DocumentError::InvalidDocument);
                    }
                    storage
                        .store_chapter_analysis(chapter.chapter_id, &analysis)
                        .map_err(map_error)?;
                }
            }
            let result = storage.load_effective_reader_word(
                word_dictionary(dictionary, word_loader)?,
                command.key(),
                selection.segment_index,
            );
            let details = result
                .map_err(|error| map_word_error(error, dictionary))?
                .ok_or(DocumentError::NotFound)?;
            Ok(EffectiveStorageResult::WordLoaded(
                selection,
                Box::new(details),
            ))
        }

        EffectiveStorageOperation::PrepareCompletion { intent, command } => storage
            .prepare_effective_page_completion(intent, command.key())
            .map(|prepared| EffectiveStorageResult::CompletionPrepared(Arc::new(prepared)))
            .map_err(map_error),
        EffectiveStorageOperation::CompletePage(prepared) => {
            let outcome = storage
                .record_effective_page_completion(&prepared)
                .map_err(map_error)?;
            // Preserve the reservation for an exact retry if the follow-up read
            // fails. The repository prevents duplicate history/status changes.
            let document = storage
                .load_document_reader_snapshot(prepared.completion().document_id)
                .map_err(map_error)?
                .ok_or(DocumentError::NotFound)?
                .document;
            Ok(EffectiveStorageResult::PageCompleted(document, outcome))
        }
    }
}
