use rusqlite::{named_params, OptionalExtension, TransactionBehavior};
use yomoku_core::{
    completion::{
        AnalyzedCompletionOutcome, AnalyzedPageCompletion, CompletionOutcome, PageCompletion,
        PageCompletionIntent,
    },
    document::{AnalysisId, CompletionCommandId},
    effective_reader::{EffectivePageCompletion, EffectiveReaderTokenKey},
};

use crate::{
    analysis::read_effective_reader_chapter_in,
    documents::write_position_in,
    encounters::{record_effective_occurrences_in, record_occurrences_in},
    reader_documents::load_reader_document_in,
    settings::read_page_length,
    Storage, StorageError,
};

impl Storage {
    /// Reserve a command and freeze the page identity in one transaction.
    /// Retrying preparation after failure cannot complete or consume anything.
    pub fn prepare_page_completion(
        &mut self,
        intent: PageCompletionIntent,
    ) -> Result<AnalyzedPageCompletion, StorageError> {
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let document = load_reader_document_in(&transaction, intent.document_id)?
            .ok_or(StorageError::InvalidCompletion)?;
        if read_page_length(&transaction)? != intent.page_length {
            return Err(StorageError::InvalidCompletion);
        }
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
            .next_document_position(&document)
            .map_err(|_| StorageError::InvalidCompletion)?;
        transaction.commit()?;
        Ok(AnalyzedPageCompletion {
            completion,
            analysis_id: intent.analysis_id,
        })
    }

    /// Validate the rendered effective identity and freeze
    /// its complete provenance/occurrences with a new command ID.
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
        let document = load_reader_document_in(&transaction, intent.document_id)?
            .ok_or(StorageError::InvalidCompletion)?;
        let chapter = document
            .chapter(intent.page.chapter_id)
            .map_err(|_| StorageError::InvalidCompletion)?;
        let effective = read_effective_reader_chapter_in(&transaction, chapter)?
            .ok_or(StorageError::InvalidAnalysis)?;
        if effective.segmentation() != &key.segmentation {
            return Err(StorageError::InvalidCompletion);
        }
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
            .next_document_position(&document)
            .map_err(|_| StorageError::InvalidCompletion)?;
        let frozen = effective
            .freeze_completion(completion)
            .map_err(|_| StorageError::InvalidAnalysis)?;
        transaction.execute(
            "INSERT INTO effective_completion_commands (command_id, semantics_version, payload) VALUES (?1, 2, ?2)",
            (i64::from(completion.command_id), effective_payload(&frozen)?),
        )?;
        transaction.commit()?;
        Ok(frozen)
    }

    /// Commit the authenticated reservation, including newly partitioned or
    /// deleted surfaces. Later rule/metadata changes cannot reinterpret a retry.
    pub fn record_effective_page_completion(
        &mut self,
        frozen: &EffectivePageCompletion,
    ) -> Result<AnalyzedCompletionOutcome, StorageError> {
        self.record_completion(
            frozen.completion(),
            Some(frozen.key().segmentation.original_analysis_id),
            Some(frozen),
        )
        .map(analyzed_outcome)
    }

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

    /// Legacy position-only repository foundation, not a reader forward action.
    /// An analyzed command cannot be replayed without its frozen analysis identity.
    pub fn record_page_completion(
        &mut self,
        completion: PageCompletion,
    ) -> Result<CompletionOutcome, StorageError> {
        self.record_completion(completion, None, None)
            .map(|(outcome, _)| outcome)
    }

    /// Freeze analysis_id with the command payload. Every Japanese occurrence,
    /// New→Known event and destination position commits together.
    /// Retries check identity before live content, so deletion/reflow is harmless.
    pub fn record_analyzed_page_completion(
        &mut self,
        completion: PageCompletion,
        analysis_id: AnalysisId,
    ) -> Result<AnalyzedCompletionOutcome, StorageError> {
        self.record_completion(completion, Some(analysis_id), None)
            .map(analyzed_outcome)
    }

    fn record_completion(
        &mut self,
        completion: PageCompletion,
        analysis_id: Option<AnalysisId>,
        effective: Option<&EffectivePageCompletion>,
    ) -> Result<(CompletionOutcome, usize), StorageError> {
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        // Authenticate the entire frozen command before replay/live checks. Legacy
        // commands cannot consume an effective ID or bypass its identity on retry.
        let reserved: Option<(i64, String)> = transaction.query_row(
            "SELECT semantics_version, payload FROM effective_completion_commands WHERE command_id = ?1",
            [i64::from(completion.command_id)], |row| Ok((row.get(0)?, row.get(1)?)),
        ).optional()?;
        match (effective, reserved) {
            (Some(frozen), Some((2, payload))) if effective_payload(frozen)? == payload => {}
            (None, None) => {}
            _ => return Err(StorageError::InvalidCompletion),
        }
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
                AND completed_at_unix_ms = :date,
                (SELECT analysis_id FROM completion_analyses WHERE command_id = :command)
             FROM reading_completions WHERE command_id = :command",
                values,
                |row| Ok((row.get::<_, bool>(0)?, row.get::<_, Option<i64>>(1)?)),
            )
            .optional()?;
        // These position identities are redundant with the anchor and must
        // match on retries as well as on first submission.
        if completion.departing_position.source_id != completion.page.source_id
            || completion.departing_position.chapter_id != completion.page.chapter_id
        {
            return Err(StorageError::InvalidCompletion);
        }
        if let Some((matches, stored_analysis)) = existing {
            if !matches || stored_analysis != analysis_id.map(i64::from) {
                return Err(StorageError::InvalidCompletion);
            }
            transaction.commit()?;
            return Ok((CompletionOutcome::AlreadyRecorded, 0));
        }
        let issued: bool = transaction.query_row(
            "SELECT EXISTS(SELECT 1 FROM completion_commands WHERE id = ?1)",
            [i64::from(completion.command_id)],
            |row| row.get(0),
        )?;
        if !issued || read_page_length(&transaction)? != completion.page_length {
            return Err(StorageError::InvalidCompletion);
        }
        let document = load_reader_document_in(&transaction, completion.document_id)?
            .ok_or(StorageError::InvalidCompletion)?;
        let next = completion
            .next_document_position(&document)
            .map_err(|_| StorageError::InvalidCompletion)?;
        transaction.execute(
            "INSERT INTO reading_completions
             (command_id, semantics_version, document_id, source_id, chapter_id,
              start_byte_offset, end_byte_offset, departing_byte_offset, page_length, completed_at_unix_ms)
             VALUES (:command, 1, :document, :source, :chapter, :start, :end, :departing, :length, :date)",
            values,
        )?;
        let newly_known = if let Some(frozen) = effective {
            record_effective_occurrences_in(&transaction, frozen)?
        } else if let Some(analysis_id) = analysis_id {
            record_occurrences_in(
                &transaction,
                document
                    .chapter(completion.page.chapter_id)
                    .map_err(|_| StorageError::InvalidCompletion)?,
                completion,
                analysis_id,
            )?
        } else {
            0
        };
        write_position_in(&transaction, completion.document_id, next)?;
        // Only the whole document's final page sets the permanent marker.
        // Intermediate chapter EOF, position-only movement and retries do not.
        let last = document
            .chapters
            .last()
            .ok_or(StorageError::InvalidCompletion)?;
        if completion.page.chapter_id == last.chapter_id
            && next.chapter_id == last.chapter_id
            && next
                .offset
                .byte_index(&last.text)
                .map_err(|_| StorageError::InvalidCompletion)?
                == last.text.len()
        {
            transaction.execute(
                "UPDATE documents SET finished_at_least_once = 1
                 WHERE id = ?1 AND finished_at_least_once = 0",
                [i64::from(completion.document_id)],
            )?;
        }
        transaction.commit()?;
        Ok((CompletionOutcome::Recorded, newly_known))
    }
}

fn effective_payload(frozen: &EffectivePageCompletion) -> Result<String, StorageError> {
    #[derive(serde::Serialize)]
    struct Record<'a> {
        completion: PageCompletion,
        analysis_id: AnalysisId,
        profile: &'a yomoku_core::analysis::AnalysisProfile,
        mode: yomoku_core::analysis::SplitMode,
        occurrences: &'a [yomoku_core::effective_reader::EffectiveReaderToken],
    }
    let segmentation = &frozen.key().segmentation;
    serde_json::to_string(&Record {
        completion: frozen.completion(),
        analysis_id: segmentation.original_analysis_id,
        profile: &segmentation.profile,
        mode: yomoku_core::analysis::SplitMode::C,
        occurrences: frozen.occurrences(),
    })
    .map_err(|_| StorageError::InvalidCompletion)
}

fn analyzed_outcome(
    (outcome, newly_known): (CompletionOutcome, usize),
) -> AnalyzedCompletionOutcome {
    match outcome {
        CompletionOutcome::Recorded => AnalyzedCompletionOutcome::Recorded {
            newly_known: newly_known as u64,
        },
        CompletionOutcome::AlreadyRecorded => AnalyzedCompletionOutcome::AlreadyRecorded,
    }
}
