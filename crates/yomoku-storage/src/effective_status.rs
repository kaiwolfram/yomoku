use rusqlite::TransactionBehavior;
use yomoku_core::{
    document::VocabularyId,
    effective_reader::EffectiveReaderTokenKey,
    reader::ReaderVocabulary,
    vocabulary::{Surface, VocabularyStatus},
};

use crate::{
    reader_words::{
        selected_effective_token_in, ReaderWordError, SelectionRevision, ValidatedSelection,
    },
    vocabulary::{read_entry_by_surface, set_vocabulary_status_in},
    Storage, StorageError,
};

impl Storage {
    /// Validate the complete rendered partition and visible spelling, reusing
    /// prior validation only at the same database revision. Read vocabulary live
    /// in the manual status/history transaction; never register a missing entry
    /// or apply an old ID to a replacement of that spelling.
    pub fn set_effective_reader_status(
        &mut self,
        key: &EffectiveReaderTokenKey,
        segment_index: u64,
        vocabulary_id: VocabularyId,
        status: VocabularyStatus,
        occurred_at_unix_ms: i64,
    ) -> Result<Option<ReaderVocabulary>, ReaderWordError> {
        // Taking ownership makes every failed write/validation discard the cache.
        let cached = self.validated_reader_selection.take();
        if occurred_at_unix_ms < 0 {
            return Err(StorageError::InvalidVocabulary.into());
        }
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(StorageError::from)?;
        let revision = SelectionRevision::read(&transaction)?;
        let mut validated =
            match cached.filter(|cached| cached.matches(key, segment_index, revision)) {
                Some(cached) => cached,
                None => {
                    let (_, token) = selected_effective_token_in(
                        &transaction,
                        key,
                        segment_index,
                        &self.reader_integrity,
                    )?
                    .ok_or(ReaderWordError::InvalidSelection)?;
                    ValidatedSelection::new(key, token, revision)
                }
            };
        let surface =
            Surface::new(&validated.token.surface).map_err(|_| StorageError::InvalidAnalysis)?;
        let Some(entry) = read_entry_by_surface(&transaction, &surface)? else {
            return Ok(None);
        };
        if entry.id != vocabulary_id {
            return Err(ReaderWordError::InvalidSelection);
        }
        // One vocabulary UPDATE plus one history INSERT. Any additional trigger
        // or cascade work makes this validation unsafe to carry across COMMIT.
        let expected_changes = if entry.status == status { 0 } else { 2 };
        let entry =
            set_vocabulary_status_in(&transaction, vocabulary_id, status, occurred_at_unix_ms)?
                .map(|entry| ReaderVocabulary {
                    id: entry.id,
                    status: entry.status,
                });
        let after = SelectionRevision::read(&transaction)?;
        let expected = revision.with_status_changes(expected_changes);
        validated.revision = after;
        transaction.commit().map_err(StorageError::from)?;
        self.validated_reader_selection = (after == expected).then_some(validated);
        Ok(entry)
    }
}
