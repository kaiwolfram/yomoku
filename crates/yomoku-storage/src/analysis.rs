use rusqlite::{Connection, OptionalExtension, TransactionBehavior};
use yomoku_core::{
    analysis::{
        AnalysisProfile, AnalyzedToken, ChapterAnalysis, OverrideScope, SegmentationOverride,
        SplitMode, StoredOverride,
    },
    document::{
        AnalysisId, ChapterId, ChapterText, OverrideId, SourceId, TextAnchor, TextOffset, TextRange,
    },
    vocabulary::Surface,
};

use crate::{Storage, StorageError};

impl Storage {
    /// Persist a validated immutable base snapshot. Identical retries reuse its ID;
    /// different output under the same engine/dictionary/mode identity is an error.
    /// This prerequisite does not register vocabulary or record reading encounters.
    pub fn store_chapter_analysis(
        &mut self,
        chapter_id: ChapterId,
        analysis: &ChapterAnalysis,
    ) -> Result<AnalysisId, StorageError> {
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let id = store_chapter_analysis_in(&transaction, chapter_id, analysis)?;
        transaction.commit()?;
        Ok(id)
    }

    pub fn chapter_analysis(
        &mut self,
        id: AnalysisId,
    ) -> Result<Option<ChapterAnalysis>, StorageError> {
        let transaction = self.connection.transaction()?;
        let analysis = read_analysis(&transaction, id)?;
        transaction.commit()?;
        Ok(analysis)
    }

    /// Replace the active edit for this exact key, preserving base analysis and
    /// inactive edits. AllOccurrences is keyed by exact surface; ThisOccurrence
    /// by source/chapter/range. Overlap/precedence resolution is not performed here.
    pub fn save_segmentation_override(
        &mut self,
        edit: &SegmentationOverride,
    ) -> Result<OverrideId, StorageError> {
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let chapter = read_chapter(&transaction, edit.anchor.chapter_id)?;
        edit.replacement_anchors(&chapter)
            .map_err(|_| StorageError::InvalidAnalysis)?;
        let existing: Option<i64> = match edit.scope {
            OverrideScope::AllOccurrences => transaction.query_row(
                "SELECT id FROM segmentation_overrides WHERE scope = 'all' AND active = 1 AND surface = ?1",
                [edit.surface.as_str()], |row| row.get(0),
            ).optional()?,
            OverrideScope::ThisOccurrence => transaction.query_row(
                "SELECT id FROM segmentation_overrides WHERE scope = 'this' AND active = 1
                 AND source_id = ?1 AND chapter_id = ?2 AND start_byte = ?3 AND end_byte = ?4",
                (i64::from(edit.anchor.source_id), i64::from(edit.anchor.chapter_id),
                 i64::from(edit.anchor.range.start), i64::from(edit.anchor.range.end)), |row| row.get(0),
            ).optional()?,
        };
        if let Some(id) = existing {
            let stored = read_override(&transaction, override_id(id)?)?
                .ok_or(StorageError::InvalidAnalysis)?;
            // A global rule made at a different occurrence has the same meaning.
            if stored.edit.parts == edit.parts && stored.edit.surface == edit.surface {
                transaction.commit()?;
                return Ok(stored.id);
            }
            transaction.execute(
                "UPDATE segmentation_overrides SET active = 0 WHERE id = ?1",
                [id],
            )?;
        }
        transaction.execute(
            "INSERT INTO segmentation_overrides (semantics_version, source_id, chapter_id,
             start_byte, end_byte, surface, scope, active) VALUES (1, ?1, ?2, ?3, ?4, ?5, ?6, 1)",
            (
                i64::from(edit.anchor.source_id),
                i64::from(edit.anchor.chapter_id),
                i64::from(edit.anchor.range.start),
                i64::from(edit.anchor.range.end),
                edit.surface.as_str(),
                match edit.scope {
                    OverrideScope::AllOccurrences => "all",
                    OverrideScope::ThisOccurrence => "this",
                },
            ),
        )?;
        let id = override_id(transaction.last_insert_rowid())?;
        for (index, part) in edit.parts.iter().enumerate() {
            transaction.execute(
                "INSERT INTO segmentation_override_parts (override_id, part_index, surface) VALUES (?1, ?2, ?3)",
                (i64::from(id), index_value(index)?, part.as_str()),
            )?;
        }
        transaction.commit()?;
        Ok(id)
    }

    pub fn segmentation_override(
        &mut self,
        id: OverrideId,
    ) -> Result<Option<StoredOverride>, StorageError> {
        let transaction = self.connection.transaction()?;
        let edit = read_override(&transaction, id)?;
        transaction.commit()?;
        Ok(edit)
    }

    /// Reversal disables this exact edit; it does not resurrect a superseded one,
    /// delete vocabulary, rescan documents, or rewrite any completed-reading data.
    pub fn reverse_segmentation_override(&mut self, id: OverrideId) -> Result<bool, StorageError> {
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let Some(stored) = read_override(&transaction, id)? else {
            return Ok(false);
        };
        if !stored.active {
            return Ok(false);
        }
        transaction.execute(
            "UPDATE segmentation_overrides SET active = 0 WHERE id = ?1",
            [i64::from(id)],
        )?;
        transaction.commit()?;
        Ok(true)
    }
}

fn read_chapter(connection: &Connection, id: ChapterId) -> Result<ChapterText, StorageError> {
    let (source, text) = connection.query_row(
        "SELECT c.source_id, t.text FROM chapters c JOIN chapter_texts t ON t.chapter_id = c.id WHERE c.id = ?1",
        [i64::from(id)], |row| Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?)),
    ).optional()?.ok_or(StorageError::InvalidAnalysis)?;
    Ok(ChapterText {
        source_id: SourceId::try_from(source).map_err(|_| StorageError::InvalidAnalysis)?,
        chapter_id: id,
        text,
    })
}

pub(crate) fn read_analysis(
    connection: &Connection,
    id: AnalysisId,
) -> Result<Option<ChapterAnalysis>, StorageError> {
    let header = connection.query_row(
        "SELECT semantics_version, chapter_id, engine_revision, dictionary_revision, split_mode FROM chapter_analyses WHERE id = ?1",
        [i64::from(id)], |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?, row.get::<_, String>(2)?, row.get::<_, String>(3)?, row.get::<_, String>(4)?)),
    ).optional()?;
    let Some((version, chapter, engine_revision, dictionary_revision, mode)) = header else {
        return Ok(None);
    };
    if version != 1 {
        return Err(StorageError::InvalidAnalysis);
    }
    let chapter = read_chapter(
        connection,
        ChapterId::try_from(chapter).map_err(|_| StorageError::InvalidAnalysis)?,
    )?;
    let mode = match mode.as_str() {
        "a" => SplitMode::A,
        "b" => SplitMode::B,
        "c" => SplitMode::C,
        _ => return Err(StorageError::InvalidAnalysis),
    };
    let mut statement = connection.prepare(
        "SELECT token_index, start_byte, end_byte, surface, is_japanese, reading, dictionary_form FROM analyzed_tokens WHERE analysis_id = ?1 ORDER BY token_index",
    )?;
    let mut rows = statement.query([i64::from(id)])?;
    let mut tokens = Vec::new();
    while let Some(row) = rows.next()? {
        if row.get::<_, i64>(0)? != index_value(tokens.len())? {
            return Err(StorageError::InvalidAnalysis);
        }
        let japanese = match row.get::<_, i64>(4)? {
            0 => false,
            1 => true,
            _ => return Err(StorageError::InvalidAnalysis),
        };
        tokens.push(AnalyzedToken {
            anchor: TextAnchor {
                source_id: chapter.source_id,
                chapter_id: chapter.chapter_id,
                range: TextRange {
                    start: offset(row.get(1)?)?,
                    end: offset(row.get(2)?)?,
                },
            },
            surface: row.get(3)?,
            is_japanese: japanese,
            reading: row.get(5)?,
            dictionary_form: row
                .get::<_, Option<String>>(6)?
                .map(Surface::new)
                .transpose()
                .map_err(|_| StorageError::InvalidAnalysis)?,
        });
    }
    let analysis = ChapterAnalysis {
        profile: AnalysisProfile {
            engine_revision,
            dictionary_revision,
        },
        mode,
        tokens,
    };
    analysis
        .validate(&chapter)
        .map_err(|_| StorageError::InvalidAnalysis)?;
    Ok(Some(analysis))
}

fn read_override(
    connection: &Connection,
    id: OverrideId,
) -> Result<Option<StoredOverride>, StorageError> {
    let header = connection
        .query_row(
            "SELECT o.semantics_version, o.source_id, o.chapter_id, o.start_byte, o.end_byte,
         o.surface, o.scope, o.active, c.source_id FROM segmentation_overrides o
         LEFT JOIN chapters c ON c.id = o.chapter_id WHERE o.id = ?1",
            [i64::from(id)],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, i64>(3)?,
                    row.get::<_, i64>(4)?,
                    row.get::<_, String>(5)?,
                    row.get::<_, String>(6)?,
                    row.get::<_, i64>(7)?,
                    row.get::<_, Option<i64>>(8)?,
                ))
            },
        )
        .optional()?;
    let Some((version, source, chapter, start, end, surface, scope, active, owner)) = header else {
        return Ok(None);
    };
    if version != 1 || Some(source) != owner {
        return Err(StorageError::InvalidAnalysis);
    }
    let scope = match scope.as_str() {
        "all" => OverrideScope::AllOccurrences,
        "this" => OverrideScope::ThisOccurrence,
        _ => return Err(StorageError::InvalidAnalysis),
    };
    let active = match active {
        0 => false,
        1 => true,
        _ => return Err(StorageError::InvalidAnalysis),
    };
    let mut statement = connection.prepare("SELECT part_index, surface FROM segmentation_override_parts WHERE override_id = ?1 ORDER BY part_index")?;
    let mut rows = statement.query([i64::from(id)])?;
    let mut parts = Vec::new();
    while let Some(row) = rows.next()? {
        if row.get::<_, i64>(0)? != index_value(parts.len())? {
            return Err(StorageError::InvalidAnalysis);
        }
        parts.push(
            Surface::new(row.get::<_, String>(1)?).map_err(|_| StorageError::InvalidAnalysis)?,
        );
    }
    let edit = SegmentationOverride {
        scope,
        anchor: TextAnchor {
            source_id: SourceId::try_from(source).map_err(|_| StorageError::InvalidAnalysis)?,
            chapter_id: ChapterId::try_from(chapter).map_err(|_| StorageError::InvalidAnalysis)?,
            range: TextRange {
                start: offset(start)?,
                end: offset(end)?,
            },
        },
        surface: Surface::new(surface).map_err(|_| StorageError::InvalidAnalysis)?,
        parts,
    };
    edit.validate_shape()
        .map_err(|_| StorageError::InvalidAnalysis)?;
    // Validate against canonical text while it exists; retained global rules can
    // still be inspected/reversed after their originating content is deleted.
    let text_exists: bool = connection.query_row(
        "SELECT EXISTS(SELECT 1 FROM chapter_texts WHERE chapter_id = ?1)",
        [chapter],
        |row| row.get(0),
    )?;
    if text_exists {
        edit.replacement_anchors(&read_chapter(connection, edit.anchor.chapter_id)?)
            .map_err(|_| StorageError::InvalidAnalysis)?;
    }
    Ok(Some(StoredOverride { id, edit, active }))
}

fn analysis_id(value: i64) -> Result<AnalysisId, StorageError> {
    AnalysisId::try_from(value).map_err(|_| StorageError::InvalidAnalysis)
}
fn override_id(value: i64) -> Result<OverrideId, StorageError> {
    OverrideId::try_from(value).map_err(|_| StorageError::InvalidAnalysis)
}
fn offset(value: i64) -> Result<TextOffset, StorageError> {
    TextOffset::try_from(value).map_err(|_| StorageError::InvalidAnalysis)
}
fn index_value(value: usize) -> Result<i64, StorageError> {
    i64::try_from(value).map_err(|_| StorageError::InvalidAnalysis)
}
fn encode_mode(mode: SplitMode) -> &'static str {
    match mode {
        SplitMode::A => "a",
        SplitMode::B => "b",
        SplitMode::C => "c",
    }
}

pub(crate) fn store_chapter_analysis_in(
    connection: &Connection,
    chapter_id: ChapterId,
    analysis: &ChapterAnalysis,
) -> Result<AnalysisId, StorageError> {
    let chapter = read_chapter(connection, chapter_id)?;
    analysis
        .validate(&chapter)
        .map_err(|_| StorageError::InvalidAnalysis)?;
    let existing: Option<i64> = connection
        .query_row(
            "SELECT id FROM chapter_analyses WHERE chapter_id = ?1 AND engine_revision = ?2
         AND dictionary_revision = ?3 AND split_mode = ?4",
            (
                i64::from(chapter_id),
                &analysis.profile.engine_revision,
                &analysis.profile.dictionary_revision,
                encode_mode(analysis.mode),
            ),
            |row| row.get(0),
        )
        .optional()?;
    if let Some(id) = existing {
        let id = analysis_id(id)?;
        if read_analysis(connection, id)?.as_ref() != Some(analysis) {
            return Err(StorageError::InvalidAnalysis);
        }
        return Ok(id);
    }
    connection.execute(
        "INSERT INTO chapter_analyses
         (semantics_version, chapter_id, engine_revision, dictionary_revision, split_mode)
         VALUES (1, ?1, ?2, ?3, ?4)",
        (
            i64::from(chapter_id),
            &analysis.profile.engine_revision,
            &analysis.profile.dictionary_revision,
            encode_mode(analysis.mode),
        ),
    )?;
    let id = analysis_id(connection.last_insert_rowid())?;
    for (index, token) in analysis.tokens.iter().enumerate() {
        connection.execute(
            "INSERT INTO analyzed_tokens (analysis_id, token_index, start_byte, end_byte,
             surface, is_japanese, reading, dictionary_form) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            (i64::from(id), index_value(index)?, i64::from(token.anchor.range.start),
             i64::from(token.anchor.range.end), &token.surface, token.is_japanese, &token.reading,
             token.dictionary_form.as_ref().map(Surface::as_str)),
        )?;
    }
    Ok(id)
}
