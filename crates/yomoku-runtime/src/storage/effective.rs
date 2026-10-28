//! Effective commands use the same connection/queue/dictionary as legacy reads.

use std::sync::Arc;

use yomoku_core::{
    effective_overrides::EffectiveOverrideFailure,
    effective_storage::{EffectiveStorageOperation, EffectiveStorageResult},
    reader::ReaderAnalysis,
    storage::DocumentError,
};
use yomoku_storage::effective_overrides::EffectiveOverrideError;
use yomoku_storage::{dictionary::Dictionary, Storage};

use super::{map_custom_error, map_error, map_word_error, word_dictionary, WordDictionaryLoader};

pub(super) fn execute(
    storage: &mut Storage,
    dictionary: &mut Option<Dictionary>,
    word_loader: &mut Option<WordDictionaryLoader>,
    operation: EffectiveStorageOperation,
) -> Result<EffectiveStorageResult, DocumentError> {
    match operation {
        EffectiveStorageOperation::LoadOverrideContext { command, anchor } => Ok(
            match storage.load_effective_override_context(command.shared_key(), anchor) {
                Ok(context) => EffectiveStorageResult::OverrideContextLoaded(Arc::new(context)),
                Err(error) => override_error(error),
            },
        ),
        EffectiveStorageOperation::SaveOverride {
            context,
            scope,
            parts,
            registered_at_unix_ms,
        } => Ok(
            match storage.save_effective_override(&context, scope, parts, registered_at_unix_ms) {
                Ok((id, registered)) => EffectiveStorageResult::OverrideSaved { id, registered },
                Err(error) => override_error(error),
            },
        ),
        EffectiveStorageOperation::ReverseOverride { context, id } => {
            Ok(match storage.reverse_effective_override(&context, id) {
                Ok(()) => EffectiveStorageResult::OverrideReversed(id),
                Err(error) => override_error(error),
            })
        }
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
        EffectiveStorageOperation::SaveCustomEntry {
            command,
            english_meaning,
            reading,
        } => {
            let selection = command.selection();
            let result = storage.save_effective_reader_custom_entry(
                word_dictionary(dictionary, word_loader)?,
                command.key(),
                selection.segment_index,
                english_meaning,
                reading,
            );
            let entry = result
                .map_err(|error| map_custom_error(error, dictionary))?
                .ok_or(DocumentError::NotFound)?;
            Ok(EffectiveStorageResult::CustomEntryChanged(
                selection,
                Some(entry.into()),
            ))
        }
        EffectiveStorageOperation::DeleteCustomEntry(command) => {
            let selection = command.selection();
            let result = storage.delete_effective_reader_custom_entry(
                word_dictionary(dictionary, word_loader)?,
                command.key(),
                selection.segment_index,
            );
            result.map_err(|error| map_custom_error(error, dictionary))?;
            Ok(EffectiveStorageResult::CustomEntryChanged(selection, None))
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

fn override_error(error: EffectiveOverrideError) -> EffectiveStorageResult {
    EffectiveStorageResult::OverrideFailed(match error {
        EffectiveOverrideError::Storage(error) => {
            EffectiveOverrideFailure::Storage(map_error(error))
        }
        EffectiveOverrideError::InvalidSelection => EffectiveOverrideFailure::InvalidSelection,
        EffectiveOverrideError::StaleContext => EffectiveOverrideFailure::StaleContext,
    })
}
