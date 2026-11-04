//! Streaming equivalent of ChapterAnalysis::validate over retained canonical text.
//! Missing legacy analyses are valid; validation never invokes an analyzer.

use rusqlite::{types::ValueRef, Connection, Row};
use yomoku_core::{document::AnalysisId, vocabulary::Surface};

use super::{check_cancelled, CandidateError};

pub(super) fn check(
    connection: &Connection,
    cancelled: &mut impl FnMut() -> bool,
) -> Result<(), CandidateError> {
    let mut headers = connection.prepare(
        "SELECT a.id, a.semantics_version, a.engine_revision, a.dictionary_revision,
                a.split_mode, t.text
         FROM main.chapter_analyses a JOIN main.chapter_texts t ON t.chapter_id = a.chapter_id
         ORDER BY a.id",
    )?;
    let mut tokens = connection.prepare(
        "SELECT token_index, start_byte, end_byte, surface, is_japanese, reading, dictionary_form
         FROM main.analyzed_tokens WHERE analysis_id = ?1 ORDER BY token_index",
    )?;
    let mut rows = headers.query([])?;
    while let Some(row) = rows.next()? {
        check_cancelled(cancelled)?;
        AnalysisId::try_from(row.get::<_, i64>(0)?).map_err(|_| CandidateError::InvalidData)?;
        require(
            row.get::<_, i64>(1)? == 1
                && !text(row, 2)?.trim().is_empty()
                && !text(row, 3)?.trim().is_empty()
                && matches!(text(row, 4)?, "a" | "b" | "c"),
        )?;
        let canonical = text(row, 5)?;
        let mut previous_end = 0;
        let mut index = 0_i64;
        let mut units = tokens.query([row.get::<_, i64>(0)?])?;
        while let Some(token) = units.next()? {
            check_cancelled(cancelled)?;
            let start = token.get::<_, i64>(1)?;
            let end = token.get::<_, i64>(2)?;
            let surface = slice(canonical, start, end)?;
            let japanese = token.get::<_, i64>(4)?;
            require(
                token.get::<_, i64>(0)? == index
                    && !surface.is_empty()
                    && surface == text(token, 3)?
                    && slice(canonical, previous_end, start)?
                        .chars()
                        .all(char::is_whitespace)
                    && matches!(japanese, 0 | 1),
            )?;
            if japanese == 1 {
                Surface::new(surface).map_err(|_| CandidateError::InvalidData)?;
            }
            if token.get_ref(5)? != ValueRef::Null {
                require(!text(token, 5)?.trim().is_empty())?;
            }
            if token.get_ref(6)? != ValueRef::Null {
                Surface::new(text(token, 6)?).map_err(|_| CandidateError::InvalidData)?;
            }
            previous_end = end;
            index = index.checked_add(1).ok_or(CandidateError::InvalidData)?;
        }
        let end = i64::try_from(canonical.len()).map_err(|_| CandidateError::InvalidData)?;
        require(
            slice(canonical, previous_end, end)?
                .chars()
                .all(char::is_whitespace),
        )?;
    }
    check_cancelled(cancelled)
}

pub(super) fn slice(text: &str, start: i64, end: i64) -> Result<&str, CandidateError> {
    let start = usize::try_from(start).map_err(|_| CandidateError::InvalidData)?;
    let end = usize::try_from(end).map_err(|_| CandidateError::InvalidData)?;
    text.get(start..end).ok_or(CandidateError::InvalidData)
}

fn text<'row>(row: &'row Row<'_>, column: usize) -> Result<&'row str, CandidateError> {
    row.get_ref(column)?
        .as_str()
        .map_err(|_| CandidateError::InvalidData)
}

fn require(valid: bool) -> Result<(), CandidateError> {
    if valid {
        Ok(())
    } else {
        Err(CandidateError::InvalidData)
    }
}
