use rusqlite::{Connection, OptionalExtension, TransactionBehavior};
use yomoku_core::{
    analysis::{AnalysisProfile, AnalyzedToken, ChapterAnalysis, SplitMode},
    document::{AnalysisId, ChapterId, ChapterText, SourceId, TextAnchor, TextOffset, TextRange},
    effective_reader::EffectiveReaderChapter,
    reader::ReaderAnalysis,
    vocabulary::Surface,
};

use crate::{Storage, StorageError};

/// Resolve the pinned longest-token snapshot in one transaction.
pub(crate) fn read_effective_reader_chapter_in<'a>(
    connection: &Connection,
    chapter: &'a ChapterText,
) -> Result<Option<EffectiveReaderChapter<'a>>, StorageError> {
    let ReaderAnalysis::Available(id) = read_reader_analysis(connection, chapter.chapter_id)?
    else {
        return Ok(None);
    };
    let original = read_analysis(connection, id)?.ok_or(StorageError::InvalidAnalysis)?;
    EffectiveReaderChapter::resolve(chapter, id, &original)
        .map(Some)
        .map_err(|_| StorageError::InvalidAnalysis)
}

/// Pin the first persisted Mode C snapshot, including its original profile.
/// Later engine/dictionary snapshots must not silently replace an import's base.
/// Full token/anchor validation still happens inside atomic analyzed completion.
pub(crate) fn read_reader_analysis(
    connection: &Connection,
    chapter_id: ChapterId,
) -> Result<ReaderAnalysis, StorageError> {
    let header = connection
        .query_row(
            "SELECT id, semantics_version, engine_revision, dictionary_revision
             FROM chapter_analyses WHERE chapter_id = ?1 AND split_mode = 'c'
             ORDER BY id LIMIT 1",
            [i64::from(chapter_id)],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                ))
            },
        )
        .optional()?;
    let Some((id, version, engine, dictionary)) = header else {
        return Ok(ReaderAnalysis::Missing);
    };
    if version != 1 || engine.trim().is_empty() || dictionary.trim().is_empty() {
        return Err(StorageError::InvalidAnalysis);
    }
    Ok(ReaderAnalysis::Available(analysis_id(id)?))
}

impl Storage {
    pub fn has_component_analysis(
        &self,
        chapter: ChapterId,
        profile: &AnalysisProfile,
        mode: SplitMode,
    ) -> Result<bool, StorageError> {
        Ok(self.connection.query_row("SELECT EXISTS(SELECT 1 FROM chapter_analyses WHERE chapter_id = ?1 AND engine_revision = ?2 AND dictionary_revision = ?3 AND split_mode = ?4)",
            (i64::from(chapter), &profile.engine_revision, &profile.dictionary_revision, encode_mode(mode)), |row| row.get(0))?)
    }

    /// Persist a validated immutable base snapshot. Identical retries reuse its ID;
    /// different output under the same engine/dictionary/mode identity is an error.
    /// New snapshots retain resolved dictionary-form evidence for existing entries;
    /// this does not register vocabulary or record reading encounters.
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
}

pub(crate) fn read_chapter(
    connection: &Connection,
    id: ChapterId,
) -> Result<ChapterText, StorageError> {
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

fn analysis_id(value: i64) -> Result<AnalysisId, StorageError> {
    AnalysisId::try_from(value).map_err(|_| StorageError::InvalidAnalysis)
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
