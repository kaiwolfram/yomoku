use rusqlite::{named_params, OptionalExtension, TransactionBehavior};
use yomoku_core::{
    completion::{CompletionOutcome, PageCompletion},
    document::CompletionCommandId,
};

use crate::{
    documents::{load_paste_in, write_position_in},
    settings::read_page_length,
    Storage, StorageError,
};

impl Storage {
    /// Allocate once for a forward intent, not once per callback or retry.
    /// Unused IDs are harmless and never automatically completed on reopen.
    pub fn new_completion_command_id(&mut self) -> Result<CompletionCommandId, StorageError> {
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        transaction.execute("INSERT INTO completion_commands DEFAULT VALUES", [])?;
        let id = CompletionCommandId::try_from(transaction.last_insert_rowid())
            .map_err(|_| StorageError::InvalidCompletion)?;
        transaction.commit()?;
        Ok(id)
    }

    /// Repository-only foundation. Before exposing forward navigation, extend
    /// this transaction with analyzed encounters, New→Known events and time.
    /// Retries must supply the same ID and frozen payload; they never move progress.
    pub fn record_page_completion(
        &mut self,
        completion: PageCompletion,
    ) -> Result<CompletionOutcome, StorageError> {
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let values = named_params! {
            ":command": i64::from(completion.command_id),
            ":document": i64::from(completion.document_id),
            ":source": i64::from(completion.page.source_id),
            ":chapter": i64::from(completion.page.chapter_id),
            ":start": i64::from(completion.page.range.start),
            ":end": i64::from(completion.page.range.end),
            ":departing": i64::from(completion.departing_position.offset),
            ":length": completion.page_length.characters() as i64,
            ":date": completion.completed_at_unix_ms,
        };
        // Replay precedes live-content validation: history remains retry-safe
        // after progress changes, page reflow and future document deletion.
        let existing = transaction
            .query_row(
                "SELECT semantics_version = 1 AND document_id = :document
                AND source_id = :source AND chapter_id = :chapter
                AND start_byte_offset = :start AND end_byte_offset = :end
                AND departing_byte_offset = :departing AND page_length = :length
                AND completed_at_unix_ms = :date
             FROM reading_completions WHERE command_id = :command",
                values,
                |row| row.get::<_, bool>(0),
            )
            .optional()?;
        // These position identities are redundant with the anchor and must
        // match on retries as well as on first submission.
        if completion.departing_position.source_id != completion.page.source_id
            || completion.departing_position.chapter_id != completion.page.chapter_id
        {
            return Err(StorageError::InvalidCompletion);
        }
        if let Some(matches) = existing {
            if !matches {
                return Err(StorageError::InvalidCompletion);
            }
            transaction.commit()?;
            return Ok(CompletionOutcome::AlreadyRecorded);
        }
        let issued: bool = transaction.query_row(
            "SELECT EXISTS(SELECT 1 FROM completion_commands WHERE id = ?1)",
            [i64::from(completion.command_id)],
            |row| row.get(0),
        )?;
        if !issued || read_page_length(&transaction)? != completion.page_length {
            return Err(StorageError::InvalidCompletion);
        }
        let document = load_paste_in(&transaction, completion.document_id)?
            .ok_or(StorageError::InvalidCompletion)?;
        let next = completion
            .next_position(&document)
            .map_err(|_| StorageError::InvalidCompletion)?;
        transaction.execute(
            "INSERT INTO reading_completions
             (command_id, semantics_version, document_id, source_id, chapter_id,
              start_byte_offset, end_byte_offset, departing_byte_offset, page_length, completed_at_unix_ms)
             VALUES (:command, 1, :document, :source, :chapter, :start, :end, :departing, :length, :date)",
            values,
        )?;
        write_position_in(&transaction, completion.document_id, next)?;
        transaction.commit()?;
        Ok(CompletionOutcome::Recorded)
    }
}
