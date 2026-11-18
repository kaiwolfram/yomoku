use std::collections::BTreeSet;

use rusqlite::Connection;
use yomoku_core::{
    completion::PageCompletion,
    effective_reader::EffectivePageCompletion,
    vocabulary::{status_after_completion, Surface},
};

use crate::{
    vocabulary::{encode_status, read_entry_by_surface, register_vocabulary_in},
    StorageError,
};

/// Shares exact-surface registration and New-only transitions across reader projections.
fn register_completed_surfaces_in(
    connection: &Connection,
    surfaces: &BTreeSet<Surface>,
    completion: PageCompletion,
) -> Result<usize, StorageError> {
    // Only this actual completed page recreates deleted/new spellings. No rescan.
    register_vocabulary_in(connection, surfaces, completion.completed_at_unix_ms)?;
    let mut newly_known = 0;
    for surface in surfaces {
        let entry =
            read_entry_by_surface(connection, surface)?.ok_or(StorageError::InvalidVocabulary)?;
        let status = status_after_completion(entry.status);
        if status != entry.status {
            connection.execute(
                "UPDATE vocabulary SET status = ?1 WHERE id = ?2",
                (encode_status(status), i64::from(entry.id)),
            )?;
            connection.execute(
                "INSERT INTO vocabulary_status_events
                 (semantics_version, vocabulary_id, old_status, new_status, cause, occurred_at_unix_ms, completion_command_id)
                 VALUES (1, ?1, ?2, ?3, 'completion', ?4, ?5)",
                (i64::from(entry.id), encode_status(entry.status), encode_status(status),
                 completion.completed_at_unix_ms, i64::from(completion.command_id)),
            )?;
            newly_known += 1;
        }
    }
    Ok(newly_known)
}

pub(crate) fn record_effective_occurrences_in(
    connection: &Connection,
    frozen: &EffectivePageCompletion,
) -> Result<usize, StorageError> {
    let completion = frozen.completion();
    let segmentation = &frozen.key().segmentation;
    // Keep the immutable Mode C provenance in the receipt even after deletion.
    connection.execute(
        "INSERT INTO completion_analyses
         (command_id, semantics_version, analysis_id, engine_revision, dictionary_revision, split_mode)
         VALUES (?1, 1, ?2, ?3, ?4, 'c')",
        (i64::from(completion.command_id), i64::from(segmentation.original_analysis_id),
         &segmentation.profile.engine_revision, &segmentation.profile.dictionary_revision),
    )?;
    let surfaces = frozen
        .occurrences()
        .iter()
        .map(|token| Surface::new(&token.surface))
        .collect::<Result<BTreeSet<_>, _>>()
        .map_err(|_| StorageError::InvalidAnalysis)?;
    let newly_known = register_completed_surfaces_in(connection, &surfaces, completion)?;
    let mut insert = connection.prepare(
        "INSERT INTO reading_occurrences
         (command_id, token_index, vocabulary_id, start_byte, end_byte, surface, reading, dictionary_form)
         SELECT ?1, ?2, id, ?3, ?4, ?5, ?6, ?7 FROM vocabulary WHERE surface = ?5",
    )?;
    for token in frozen.occurrences() {
        let metadata = &token.metadata;
        insert.execute((
            i64::from(completion.command_id),
            i64::try_from(token.segment_index).map_err(|_| StorageError::InvalidAnalysis)?,
            i64::from(token.anchor.range.start),
            i64::from(token.anchor.range.end),
            &token.surface,
            metadata.reading.as_deref(),
            metadata.dictionary_form.as_ref().map(Surface::as_str),
        ))?;
    }
    Ok(newly_known)
}
