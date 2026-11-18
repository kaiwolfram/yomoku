use rusqlite::{named_params, OptionalExtension, TransactionBehavior};
use sha2::{Digest, Sha256};
use yomoku_core::{
    completion::{AnalyzedCompletionOutcome, PageCompletion, PageCompletionIntent},
    document::{ChapterId, CompletionCommandId, DocumentId, ReaderDocument, SourceId, TextOffset},
    effective_reader::{EffectivePageCompletion, EffectiveReaderTokenKey},
    pagination::ReadingPosition,
};

use crate::{
    documents::write_position_in, encounters::record_effective_occurrences_in,
    reader_documents::load_reader_document_in, settings::read_page_length, Storage, StorageError,
};

impl Storage {
    /// Validate the rendered page and reserve a new command ID with a fingerprint
    /// of the accepted completion. The session retains the frozen occurrences.
    /// Preparation never registers words or records encounters.
    pub fn prepare_effective_page_completion(
        &mut self,
        intent: PageCompletionIntent,
        key: &EffectiveReaderTokenKey,
    ) -> Result<EffectivePageCompletion, StorageError> {
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        if key.document_id != intent.document_id
            || key.page != intent.page
            || key.segmentation.original_analysis_id != intent.analysis_id
            || read_page_length(&transaction)? != intent.page_length
        {
            return Err(StorageError::InvalidCompletion);
        }
        let revision = self.reader_integrity.revision(&transaction)?;
        let layout = match self.reader_integrity.cached_layout(
            intent.document_id,
            intent.page_length,
            revision,
        ) {
            Some(layout) => layout,
            None => {
                let document = load_reader_document_in(
                    &transaction,
                    intent.document_id,
                    &self.reader_integrity,
                )?
                .ok_or(StorageError::InvalidCompletion)?;
                self.reader_integrity
                    .layout(&document, intent.page_length, revision)?
            }
        };
        let chapter = self
            .reader_integrity
            .chapter(&transaction, intent.document_id, intent.page.chapter_id)?
            .ok_or(StorageError::InvalidCompletion)?;
        let effective = chapter
            .effective
            .as_ref()
            .ok_or(StorageError::InvalidAnalysis)?;
        if effective.segmentation() != &key.segmentation {
            return Err(StorageError::InvalidCompletion);
        }
        let first = layout
            .pages()
            .first()
            .ok_or(StorageError::InvalidCompletion)?;
        let current = saved_position(&transaction, intent.document_id, first.source_id)?.unwrap_or(
            ReadingPosition {
                source_id: first.source_id,
                chapter_id: first.chapter_id,
                offset: first.range.start,
            },
        );
        transaction.execute("INSERT INTO completion_commands DEFAULT VALUES", [])?;
        let completion = PageCompletion {
            command_id: CompletionCommandId::try_from(transaction.last_insert_rowid())
                .map_err(|_| StorageError::InvalidCompletion)?,
            document_id: intent.document_id,
            page: intent.page,
            page_length: intent.page_length,
            departing_position: intent.departing_position,
            completed_at_unix_ms: intent.completed_at_unix_ms,
        };
        completion
            .next_position_with_layout(intent.document_id, current, &chapter.text, &layout)
            .map_err(|_| StorageError::InvalidCompletion)?;
        let frozen = effective
            .freeze_completion(completion)
            .map_err(|_| StorageError::InvalidAnalysis)?;
        transaction.execute(
            "UPDATE completion_commands SET pending_fingerprint = ?2 WHERE id = ?1",
            (
                i64::from(completion.command_id),
                completion_fingerprint(&frozen)?,
            ),
        )?;
        transaction.commit()?;
        Ok(frozen)
    }

    /// Commit the verified reservation atomically. Committed retries use the
    /// receipt and remain safe after document deletion or reflow.
    pub fn record_effective_page_completion(
        &mut self,
        frozen: &EffectivePageCompletion,
    ) -> Result<AnalyzedCompletionOutcome, StorageError> {
        self.commit_effective_page_completion(frozen)
            .map(|(outcome, _)| outcome)
    }

    /// A fresh commit can return its validated document. An exact retry or a
    /// content write within the transaction requires the caller to reload.
    pub fn commit_effective_page_completion(
        &mut self,
        frozen: &EffectivePageCompletion,
    ) -> Result<(AnalyzedCompletionOutcome, Option<ReaderDocument>), StorageError> {
        let completion = frozen.completion();
        let segmentation = &frozen.key().segmentation;
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        if completion.departing_position.source_id != completion.page.source_id
            || completion.departing_position.chapter_id != completion.page.chapter_id
        {
            return Err(StorageError::InvalidCompletion);
        }
        let pending: Option<[u8; 32]> = transaction
            .query_row(
                "SELECT pending_fingerprint FROM completion_commands WHERE id = ?1",
                [i64::from(completion.command_id)],
                |row| row.get(0),
            )
            .optional()?
            .ok_or(StorageError::InvalidCompletion)?;
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
        let Some(pending) = pending else {
            let matches = transaction
                .query_row(
                    "SELECT c.semantics_version = 1 AND a.semantics_version = 1
                         AND c.document_id = :document AND c.source_id = :source
                         AND c.chapter_id = :chapter AND c.start_byte_offset = :start
                         AND c.end_byte_offset = :end AND c.departing_byte_offset = :departing
                         AND c.page_length = :length AND c.completed_at_unix_ms = :date
                         AND a.split_mode = 'c',
                         a.analysis_id, a.engine_revision, a.dictionary_revision
                     FROM reading_completions c JOIN completion_analyses a USING (command_id)
                     WHERE c.command_id = :command",
                    values,
                    |row| {
                        Ok(row.get::<_, bool>(0)?
                            && row.get::<_, i64>(1)?
                                == i64::from(segmentation.original_analysis_id)
                            && row.get_ref(2)?.as_str().ok()
                                == Some(segmentation.profile.engine_revision.as_str())
                            && row.get_ref(3)?.as_str().ok()
                                == Some(segmentation.profile.dictionary_revision.as_str()))
                    },
                )
                .optional()?;
            if matches != Some(true) {
                return Err(StorageError::InvalidCompletion);
            }
            transaction.commit()?;
            return Ok((AnalyzedCompletionOutcome::AlreadyRecorded, None));
        };
        if pending != completion_fingerprint(frozen)?
            || read_page_length(&transaction)? != completion.page_length
        {
            return Err(StorageError::InvalidCompletion);
        }
        let revision = self.reader_integrity.revision(&transaction)?;
        let mut document =
            load_reader_document_in(&transaction, completion.document_id, &self.reader_integrity)?
                .ok_or(StorageError::InvalidCompletion)?;
        let layout = self
            .reader_integrity
            .layout(&document, completion.page_length, revision)?;
        let current = document
            .current_position()
            .map_err(|_| StorageError::InvalidCompletion)?;
        let chapter = document
            .chapter(current.chapter_id)
            .map_err(|_| StorageError::InvalidCompletion)?;
        let next = completion
            .next_position_with_layout(document.document_id, current, chapter, &layout)
            .map_err(|_| StorageError::InvalidCompletion)?;
        transaction.execute(
            "INSERT INTO reading_completions
             (command_id, semantics_version, document_id, source_id, chapter_id,
              start_byte_offset, end_byte_offset, departing_byte_offset, page_length, completed_at_unix_ms)
             VALUES (:command, 1, :document, :source, :chapter, :start, :end, :departing, :length, :date)",
            values,
        )?;
        let newly_known = record_effective_occurrences_in(&transaction, frozen)?;
        write_position_in(&transaction, completion.document_id, next)?;
        document.reading_position = Some(next);
        // A new final-page completion sets the finish marker and clears the Library override.
        let last = document
            .chapters
            .last()
            .ok_or(StorageError::InvalidCompletion)?;
        let mut document_writes = 0;
        if completion.page.chapter_id == last.chapter_id
            && next.chapter_id == last.chapter_id
            && next
                .offset
                .byte_index(&last.text)
                .map_err(|_| StorageError::InvalidCompletion)?
                == last.text.len()
        {
            document_writes = transaction.execute(
                "UPDATE documents SET finished_at_least_once = 1, read_override = NULL
                 WHERE id = ?1 AND (finished_at_least_once = 0 OR read_override IS NOT NULL)",
                [i64::from(completion.document_id)],
            )? as u64;
            document.finished_at_least_once = true;
            document.read_override = None;
        }
        transaction.execute(
            "UPDATE completion_commands SET pending_fingerprint = NULL WHERE id = ?1",
            [i64::from(completion.command_id)],
        )?;
        let expected_position = saved_position(&transaction, document.document_id, next.source_id)
            .is_ok_and(|position| position == Some(next));
        transaction.commit()?;
        let reusable = expected_position
            && self.reader_integrity.snapshot_unchanged(
                &self.connection,
                revision,
                document_writes,
            )?;
        Ok((
            AnalyzedCompletionOutcome::Recorded {
                newly_known: newly_known as u64,
            },
            reusable.then_some(document),
        ))
    }
}

fn saved_position(
    connection: &rusqlite::Connection,
    document_id: DocumentId,
    source_id: SourceId,
) -> Result<Option<ReadingPosition>, StorageError> {
    let stored = connection
        .query_row(
            "SELECT chapter_id, byte_offset FROM reading_positions WHERE document_id = ?1",
            [i64::from(document_id)],
            |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)),
        )
        .optional()?;
    stored
        .map(|(chapter, offset)| {
            Ok(ReadingPosition {
                source_id,
                chapter_id: ChapterId::try_from(chapter)
                    .map_err(|_| StorageError::InvalidCompletion)?,
                offset: TextOffset::try_from(offset)
                    .map_err(|_| StorageError::InvalidCompletion)?,
            })
        })
        .transpose()
}

fn completion_fingerprint(frozen: &EffectivePageCompletion) -> Result<[u8; 32], StorageError> {
    let segmentation = &frozen.key().segmentation;
    // Only the page command, its immutable Mode C provenance and the accepted
    // occurrences contribute. The encoding is transient.
    let encoded = serde_json::to_vec(&(
        frozen.completion(),
        segmentation.original_analysis_id,
        &segmentation.profile,
        frozen.occurrences(),
    ))
    .map_err(|_| StorageError::InvalidCompletion)?;
    Ok(Sha256::digest(encoded).into())
}
