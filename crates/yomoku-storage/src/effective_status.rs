use rusqlite::TransactionBehavior;
use yomoku_core::{
    document::VocabularyId, effective_reader::EffectiveReaderTokenKey, reader::ReaderVocabulary,
    vocabulary::VocabularyStatus,
};

use crate::{
    reader_words::{selected_effective_token_in, ReaderWordError},
    vocabulary::set_vocabulary_status_in,
    Storage, StorageError,
};

impl Storage {
    /// Validate the complete rendered partition and visible spelling in the same
    /// transaction as the manual status/history write. Never register a missing
    /// vocabulary entry or apply an old ID to a replacement of that spelling.
    pub fn set_effective_reader_status(
        &mut self,
        key: &EffectiveReaderTokenKey,
        segment_index: u64,
        vocabulary_id: VocabularyId,
        status: VocabularyStatus,
        occurred_at_unix_ms: i64,
    ) -> Result<Option<ReaderVocabulary>, ReaderWordError> {
        if occurred_at_unix_ms < 0 {
            return Err(StorageError::InvalidVocabulary.into());
        }
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(StorageError::from)?;
        let token = selected_effective_token_in(&transaction, key, segment_index)?
            .ok_or(ReaderWordError::InvalidSelection)?;
        let Some(entry) = token.vocabulary else {
            return Ok(None);
        };
        if entry.id != vocabulary_id {
            return Err(ReaderWordError::InvalidSelection);
        }
        let entry =
            set_vocabulary_status_in(&transaction, vocabulary_id, status, occurred_at_unix_ms)?
                .map(|entry| ReaderVocabulary {
                    id: entry.id,
                    status: entry.status,
                });
        transaction.commit().map_err(StorageError::from)?;
        Ok(entry)
    }
}
