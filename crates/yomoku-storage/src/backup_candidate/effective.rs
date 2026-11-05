//! Read-only decoder for version-2 frozen effective completions. These private
//! wire records cannot become a live EffectivePageCompletion or mutation permit.

use std::collections::BTreeMap;

use rusqlite::{Connection, OptionalExtension};
use serde::Deserialize;
use yomoku_core::{
    analysis::SplitMode,
    document::{
        AnalysisId, ChapterId, ChapterText, CompletionCommandId, DocumentId, SourceId, TextAnchor,
        TextOffset, TextRange,
    },
    effective_reader::EffectiveTokenEligibility,
    pagination::{LogicalPages, PageLength, ReadingPosition},
    vocabulary::Surface,
};

use super::{check_cancelled, CandidateError};

// Mirror the existing serialized shape without making live command constructors
// deserializable. Reject unknown fields at every object depth; numeric/domain
// leaves use the ordinary core decoders. No JSON is rewritten or replayed.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Record {
    completion: Completion,
    analysis_id: AnalysisId,
    profile: Profile,
    mode: SplitMode,
    occurrences: Vec<Token>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Completion {
    command_id: CompletionCommandId,
    document_id: DocumentId,
    page: Anchor,
    page_length: PageLength,
    departing_position: Position,
    completed_at_unix_ms: i64,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Profile {
    engine_revision: String,
    dictionary_revision: String,
}

#[derive(Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
struct Metadata {
    analysis_id: AnalysisId,
    mode: SplitMode,
    token_index: usize,
    is_japanese: bool,
    #[serde(deserialize_with = "present_option")]
    reading: Option<String>,
    #[serde(deserialize_with = "present_option")]
    dictionary_form: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Token {
    segment_index: u64,
    anchor: Anchor,
    display_range: Range,
    surface: String,
    eligibility: EffectiveTokenEligibility,
    #[serde(deserialize_with = "present_option")]
    metadata: Option<Metadata>,
    // Completion projection always freezes None, never live vocabulary status.
    #[serde(deserialize_with = "present_option")]
    vocabulary: Option<()>,
}

#[derive(Clone, Copy, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
struct Anchor {
    source_id: SourceId,
    chapter_id: ChapterId,
    range: Range,
}

#[derive(Clone, Copy, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
struct Range {
    start: TextOffset,
    end: TextOffset,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Position {
    source_id: SourceId,
    chapter_id: ChapterId,
    offset: TextOffset,
}

pub(super) fn check(
    connection: &Connection,
    cancelled: &mut impl FnMut() -> bool,
) -> Result<(), CandidateError> {
    let issued_ids: BTreeMap<String, i64> = connection
        .prepare("SELECT name, seq FROM main.sqlite_sequence WHERE name IN ('documents', 'chapter_analyses')")?
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
        .collect::<Result<_, _>>()?;
    let mut statement = connection.prepare(
        "SELECT command_id, semantics_version, payload
         FROM main.effective_completion_commands ORDER BY command_id",
    )?;
    let mut rows = statement.query([])?;
    while let Some(row) = rows.next()? {
        check_cancelled(cancelled)?;
        let record: Record =
            serde_json::from_str(text(row, 2)?).map_err(|_| CandidateError::InvalidData)?;
        require(row.get::<_, i64>(1)? == 2)?;
        require(i64::from(record.completion.command_id) == row.get::<_, i64>(0)?)?;
        check_record(connection, &issued_ids, &record, cancelled)?;
        check_surviving_content(connection, &record, cancelled)?;
        check_receipt(connection, &record, cancelled)?;
    }
    check_cancelled(cancelled)
}

fn check_record(
    connection: &Connection,
    issued_ids: &BTreeMap<String, i64>,
    record: &Record,
    cancelled: &mut impl FnMut() -> bool,
) -> Result<(), CandidateError> {
    let completion = &record.completion;
    let page = completion.page;
    let position = &completion.departing_position;
    require(
        page.range.start < page.range.end
            && position.source_id == page.source_id
            && position.chapter_id == page.chapter_id
            && position.offset >= page.range.start
            && position.offset < page.range.end
            && completion.completed_at_unix_ms >= 0
            && record.mode == SplitMode::C
            && !record.profile.engine_revision.trim().is_empty()
            && !record.profile.dictionary_revision.trim().is_empty(),
    )?;
    let source: Option<i64> = connection
        .query_row(
            "SELECT source_id FROM main.chapters WHERE id = ?1",
            [i64::from(page.chapter_id)],
            |r| r.get(0),
        )
        .optional()?;
    require(source == Some(i64::from(page.source_id)))?;
    let live_source: Option<i64> = connection
        .query_row(
            "SELECT source_id FROM main.documents WHERE id = ?1",
            [i64::from(completion.document_id)],
            |r| r.get(0),
        )
        .optional()?;
    require(live_source.is_none_or(|id| id == i64::from(page.source_id)))?;
    issued(issued_ids, "documents", i64::from(completion.document_id))?;
    issued(
        issued_ids,
        "chapter_analyses",
        i64::from(record.analysis_id),
    )?;
    let mut previous = None;
    let mut previous_end = page.range.start;
    for token in &record.occurrences {
        check_cancelled(cancelled)?;
        let anchor = token.anchor;
        require(
            i64::try_from(token.segment_index).is_ok()
                && previous.is_none_or(|index| token.segment_index > index)
                && anchor.source_id == page.source_id
                && anchor.chapter_id == page.chapter_id
                && anchor.range.start >= previous_end
                && anchor.range.start < page.range.end
                && anchor.range.end > anchor.range.start
                && i64::from(anchor.range.end) - i64::from(anchor.range.start)
                    == token.surface.len() as i64
                && token.vocabulary.is_none()
                && token.eligibility == EffectiveTokenEligibility::AnalyzerJapanese
                && i64::from(token.display_range.start)
                    == i64::from(anchor.range.start) - i64::from(page.range.start)
                && i64::from(token.display_range.end)
                    == i64::from(anchor.range.end.min(page.range.end))
                        - i64::from(page.range.start),
        )?;
        Surface::new(&token.surface).map_err(|_| CandidateError::InvalidData)?;
        let metadata = token.metadata.as_ref().ok_or(CandidateError::InvalidData)?;
        require(
            metadata.analysis_id == record.analysis_id
                && metadata.mode == record.mode
                && metadata.token_index as u64 == token.segment_index
                && metadata.is_japanese
                && metadata
                    .reading
                    .as_ref()
                    .is_none_or(|r| !r.trim().is_empty()),
        )?;
        if let Some(form) = &metadata.dictionary_form {
            Surface::new(form).map_err(|_| CandidateError::InvalidData)?;
        }
        previous = Some(token.segment_index);
        previous_end = anchor.range.end;
    }
    Ok(())
}

fn check_surviving_content(
    connection: &Connection,
    record: &Record,
    cancelled: &mut impl FnMut() -> bool,
) -> Result<(), CandidateError> {
    check_cancelled(cancelled)?;
    let c = &record.completion;
    // One chapter only, never load/reimport a whole book or resolve today's rules.
    let canonical: Option<String> = connection
        .query_row(
            "SELECT text FROM main.chapter_texts WHERE chapter_id = ?1",
            [i64::from(c.page.chapter_id)],
            |row| row.get(0),
        )
        .optional()?;
    let chapter = canonical.map(|text| ChapterText {
        source_id: c.page.source_id,
        chapter_id: c.page.chapter_id,
        text,
    });
    let canonical = chapter.as_ref().map(|chapter| chapter.text.as_str());
    let live = chapter.is_some();
    check_analysis_header(
        connection,
        record.analysis_id,
        SplitMode::C,
        c.page.chapter_id,
        &record.profile,
        live,
    )?;
    if let Some(chapter) = &chapter {
        let pages = LogicalPages::new(std::slice::from_ref(chapter), c.page_length)
            .map_err(|_| CandidateError::InvalidData)?;
        let index = pages
            .page_at(ReadingPosition {
                source_id: c.departing_position.source_id,
                chapter_id: c.departing_position.chapter_id,
                offset: c.departing_position.offset,
            })
            .map_err(|_| CandidateError::InvalidData)?;
        require(
            pages.pages()[index]
                == TextAnchor {
                    source_id: c.page.source_id,
                    chapter_id: c.page.chapter_id,
                    range: TextRange {
                        start: c.page.range.start,
                        end: c.page.range.end,
                    },
                },
        )?;
    }
    if live {
        let mut statement = connection.prepare(
            "SELECT token_index, start_byte, end_byte, surface, reading, dictionary_form
             FROM main.analyzed_tokens WHERE analysis_id = ?1 AND is_japanese = 1
             AND start_byte >= ?2 AND start_byte < ?3 ORDER BY token_index",
        )?;
        let mut rows = statement.query((
            i64::from(record.analysis_id),
            i64::from(c.page.range.start),
            i64::from(c.page.range.end),
        ))?;
        for token in &record.occurrences {
            check_cancelled(cancelled)?;
            let row = rows.next()?.ok_or(CandidateError::InvalidData)?;
            let metadata = token.metadata.as_ref().ok_or(CandidateError::InvalidData)?;
            require(
                row.get::<_, i64>(0)? == token.segment_index as i64
                    && row.get::<_, i64>(1)? == i64::from(token.anchor.range.start)
                    && row.get::<_, i64>(2)? == i64::from(token.anchor.range.end)
                    && text(row, 3)? == token.surface
                    && row.get::<_, Option<String>>(4)? == metadata.reading
                    && row.get::<_, Option<String>>(5)? == metadata.dictionary_form,
            )?;
            require(
                super::analyses::slice(
                    canonical.as_ref().expect("live"),
                    i64::from(token.anchor.range.start),
                    i64::from(token.anchor.range.end),
                )? == token.surface,
            )?;
        }
        require(rows.next()?.is_none())?;
    }
    Ok(())
}

fn check_analysis_header(
    connection: &Connection,
    id: AnalysisId,
    mode: SplitMode,
    chapter: ChapterId,
    profile: &Profile,
    required: bool,
) -> Result<bool, CandidateError> {
    let mode = match mode {
        SplitMode::A => "a",
        SplitMode::B => "b",
        SplitMode::C => "c",
    };
    let matches = connection
        .query_row(
            "SELECT chapter_id, engine_revision, dictionary_revision, split_mode
             FROM main.chapter_analyses WHERE id = ?1",
            [i64::from(id)],
            |row| {
                Ok(row.get::<_, i64>(0)? == i64::from(chapter)
                    && row.get::<_, String>(1)? == profile.engine_revision
                    && row.get::<_, String>(2)? == profile.dictionary_revision
                    && row.get::<_, String>(3)? == mode)
            },
        )
        .optional()?;
    require(matches == Some(true) || !required && matches.is_none())?;
    Ok(matches.is_some())
}

fn check_receipt(
    connection: &Connection,
    record: &Record,
    cancelled: &mut impl FnMut() -> bool,
) -> Result<(), CandidateError> {
    let c = &record.completion;
    let command = i64::from(c.command_id);
    let receipt = connection
        .query_row(
            "SELECT document_id, source_id, chapter_id, start_byte_offset, end_byte_offset,
                departing_byte_offset, page_length, completed_at_unix_ms
         FROM main.reading_completions WHERE command_id = ?1",
            [command],
            |row| {
                (0..8)
                    .map(|i| row.get::<_, i64>(i))
                    .collect::<Result<Vec<_>, _>>()
            },
        )
        .optional()?;
    let Some(receipt) = receipt else {
        return Ok(()); // Unfinished reservation: never complete or replay it.
    };
    require(
        receipt
            == [
                i64::from(c.document_id),
                i64::from(c.page.source_id),
                i64::from(c.page.chapter_id),
                i64::from(c.page.range.start),
                i64::from(c.page.range.end),
                i64::from(c.departing_position.offset),
                c.page_length.characters() as i64,
                c.completed_at_unix_ms,
            ],
    )?;
    let header: Option<(i64, String, String, String)> = connection
        .query_row(
            "SELECT analysis_id, engine_revision, dictionary_revision, split_mode
         FROM main.completion_analyses WHERE command_id = ?1",
            [command],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .optional()?;
    require(
        header
            == Some((
                i64::from(record.analysis_id),
                record.profile.engine_revision.clone(),
                record.profile.dictionary_revision.clone(),
                "c".into(),
            )),
    )?;
    let mut statement = connection.prepare(
        "SELECT o.token_index, o.start_byte, o.end_byte, o.surface, o.reading, o.dictionary_form, v.surface
         FROM main.reading_occurrences o JOIN main.vocabulary v ON v.id = o.vocabulary_id
         WHERE o.command_id = ?1 ORDER BY o.token_index",
    )?;
    let mut rows = statement.query([command])?;
    while let Some(row) = rows.next()? {
        check_cancelled(cancelled)?;
        let index =
            u64::try_from(row.get::<_, i64>(0)?).map_err(|_| CandidateError::InvalidData)?;
        let token = record
            .occurrences
            .binary_search_by_key(&index, |token| token.segment_index)
            .ok()
            .map(|index| &record.occurrences[index])
            .ok_or(CandidateError::InvalidData)?;
        require(
            i64::from(token.anchor.range.start) == row.get::<_, i64>(1)?
                && i64::from(token.anchor.range.end) == row.get::<_, i64>(2)?
                && token.surface == text(row, 3)?
                && token.surface == text(row, 6)?
                && token.metadata.as_ref().and_then(|m| m.reading.as_deref())
                    == row.get::<_, Option<String>>(4)?.as_deref()
                && token
                    .metadata
                    .as_ref()
                    .and_then(|m| m.dictionary_form.as_ref())
                    .map(String::as_str)
                    == row.get::<_, Option<String>>(5)?.as_deref(),
        )?;
    }
    // Missing occurrences are valid after vocabulary erasure/re-registration.
    Ok(())
}

fn issued(issued_ids: &BTreeMap<String, i64>, table: &str, id: i64) -> Result<(), CandidateError> {
    // Names/typed counters have already passed the control-record checks.
    require(
        issued_ids
            .get(table)
            .is_some_and(|sequence| id <= *sequence),
    )
}

fn require(valid: bool) -> Result<(), CandidateError> {
    if valid {
        Ok(())
    } else {
        Err(CandidateError::InvalidData)
    }
}

fn text<'row>(row: &'row rusqlite::Row<'_>, column: usize) -> Result<&'row str, CandidateError> {
    row.get_ref(column)?
        .as_str()
        .map_err(|_| CandidateError::InvalidData)
}

// Saved Option fields are required keys whose values may be null. Missing keys
// must not silently manufacture defaults in a supposedly complete frozen record.
fn present_option<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::deserialize(deserializer)
}
