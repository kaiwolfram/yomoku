use std::collections::BTreeSet;

use rusqlite::Connection;
use yomoku_core::{
    analysis::SplitMode,
    completion::PageCompletion,
    document::{AnalysisId, ChapterText},
    effective_reader::EffectivePageCompletion,
    vocabulary::{status_after_completion, Surface},
};

use crate::{
    analysis::read_analysis,
    vocabulary::{
        encode_status, link_dictionary_forms_in, read_entry_by_surface, register_vocabulary_in,
    },
    StorageError,
};

pub(crate) fn record_occurrences_in(
    connection: &Connection,
    chapter: &ChapterText,
    completion: PageCompletion,
    analysis_id: AnalysisId,
) -> Result<usize, StorageError> {
    let analysis = read_analysis(connection, analysis_id)?.ok_or(StorageError::InvalidAnalysis)?;
    let tokens = analysis
        .japanese_tokens_on_page(chapter, completion.page)
        .map_err(|_| StorageError::InvalidAnalysis)?;
    // No live-analysis FK: deleting a document must not erase historical identity.
    connection.execute(
        "INSERT INTO completion_analyses
         (command_id, semantics_version, analysis_id, engine_revision, dictionary_revision, split_mode)
         VALUES (?1, 1, ?2, ?3, ?4, ?5)",
        (i64::from(completion.command_id), i64::from(analysis_id),
         &analysis.profile.engine_revision, &analysis.profile.dictionary_revision,
         match analysis.mode { SplitMode::A => "a", SplitMode::B => "b", SplitMode::C => "c" }),
    )?;
    let surfaces = tokens
        .iter()
        .map(|(_, token)| Surface::new(&token.surface))
        .collect::<Result<BTreeSet<_>, _>>()
        .map_err(|_| StorageError::InvalidAnalysis)?;
    let newly_known = register_completed_surfaces_in(connection, &surfaces, completion)?;
    link_dictionary_forms_in(
        connection,
        tokens
            .iter()
            .map(|(_, token)| (token.surface.as_str(), token.dictionary_form.as_ref())),
    )?;
    let mut insert = connection.prepare(
        "INSERT INTO reading_occurrences
         (command_id, token_index, vocabulary_id, start_byte, end_byte, surface, reading, dictionary_form)
         SELECT ?1, ?2, id, ?3, ?4, ?5, ?6, ?7 FROM vocabulary WHERE surface = ?5",
    )?;
    // Enumerate the complete analysis so token_index retains original provenance,
    // including gaps for non-Japanese tokens and tokens belonging to other pages.
    for (index, token) in tokens {
        insert.execute((
            i64::from(completion.command_id),
            i64::try_from(index).map_err(|_| StorageError::InvalidAnalysis)?,
            i64::from(token.anchor.range.start),
            i64::from(token.anchor.range.end),
            &token.surface,
            &token.reading,
            token.dictionary_form.as_ref().map(Surface::as_str),
        ))?;
    }
    Ok(newly_known)
}

/// Shares exact-surface registration and New-only transitions with original B.
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
    // This header identifies the immutable B basis. The versioned effective
    // reservation distinguishes effective segment indices from analyzer indices
    // and preserves each exact metadata/rule source, including unresolved spans.
    connection.execute(
        "INSERT INTO completion_analyses
         (command_id, semantics_version, analysis_id, engine_revision, dictionary_revision, split_mode)
         VALUES (?1, 1, ?2, ?3, ?4, 'b')",
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
    link_dictionary_forms_in(
        connection,
        frozen.occurrences().iter().map(|token| {
            (
                token.surface.as_str(),
                token
                    .metadata
                    .as_ref()
                    .and_then(|metadata| metadata.dictionary_form.as_ref()),
            )
        }),
    )?;
    let mut insert = connection.prepare(
        "INSERT INTO reading_occurrences
         (command_id, token_index, vocabulary_id, start_byte, end_byte, surface, reading, dictionary_form)
         SELECT ?1, ?2, id, ?3, ?4, ?5, ?6, ?7 FROM vocabulary WHERE surface = ?5",
    )?;
    for token in frozen.occurrences() {
        let metadata = token.metadata.as_ref();
        insert.execute((
            i64::from(completion.command_id),
            i64::try_from(token.segment_index).map_err(|_| StorageError::InvalidAnalysis)?,
            i64::from(token.anchor.range.start),
            i64::from(token.anchor.range.end),
            &token.surface,
            metadata.and_then(|metadata| metadata.reading.as_deref()),
            metadata
                .and_then(|metadata| metadata.dictionary_form.as_ref())
                .map(Surface::as_str),
        ))?;
    }
    Ok(newly_known)
}
