//! Read-only decoder for version-1 frozen effective completions. These private
//! wire records cannot become a live EffectivePageCompletion or mutation permit.

use std::collections::BTreeMap;

use rusqlite::{Connection, OptionalExtension};
use serde::Deserialize;
use yomoku_core::{
    active_time::PendingReadingTime,
    analysis::SplitMode,
    document::{
        AnalysisId, ChapterId, CompletionCommandId, DocumentId, OverrideId, SourceId, TextOffset,
    },
    effective_reader::EffectiveTokenEligibility,
    pagination::PageLength,
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
    key: Key,
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
    pending_time: PendingReadingTime,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Key {
    document_id: DocumentId,
    page: Anchor,
    segmentation: Segmentation,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Segmentation {
    original_analysis_id: AnalysisId,
    profile: Profile,
    segments: Vec<Segment>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Profile {
    engine_revision: String,
    dictionary_revision: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Segment {
    boundary: Boundary,
    #[serde(deserialize_with = "present_option")]
    metadata: Option<Metadata>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Boundary {
    anchor: Anchor,
    surface: String,
    #[serde(deserialize_with = "present_option")]
    original_token_index: Option<usize>,
    #[serde(deserialize_with = "present_option")]
    override_id: Option<OverrideId>,
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
    version: usize,
    cancelled: &mut impl FnMut() -> bool,
) -> Result<(), CandidateError> {
    if version < 18 {
        return check_cancelled(cancelled);
    }
    let issued_ids: BTreeMap<String, i64> = connection
        .prepare("SELECT name, seq FROM main.sqlite_sequence WHERE name IN ('documents', 'chapter_analyses', 'segmentation_overrides')")?
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
        require(row.get::<_, i64>(1)? == 1)?;
        require(i64::from(record.completion.command_id) == row.get::<_, i64>(0)?)?;
        check_record(connection, &issued_ids, &record, cancelled)?;
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
    let segmentation = &record.key.segmentation;
    require(
        completion.document_id == record.key.document_id
            && page == record.key.page
            && page.range.start < page.range.end
            && position.source_id == page.source_id
            && position.chapter_id == page.chapter_id
            && position.offset >= page.range.start
            && position.offset <= page.range.end
            && completion.completed_at_unix_ms >= 0
            && !segmentation.profile.engine_revision.trim().is_empty()
            && !segmentation.profile.dictionary_revision.trim().is_empty(),
    )?;
    // Sources/chapters survive document deletion, while document/analysis/rule
    // IDs can exist only in history. Require issued bounds, never live content.
    let source: Option<i64> = connection
        .query_row(
            "SELECT source_id FROM main.chapters WHERE id = ?1",
            [i64::from(page.chapter_id)],
            |row| row.get(0),
        )
        .optional()?;
    require(source == Some(i64::from(page.source_id)))?;
    let live_source: Option<i64> = connection
        .query_row(
            "SELECT source_id FROM main.documents WHERE id = ?1",
            [i64::from(completion.document_id)],
            |row| row.get(0),
        )
        .optional()?;
    require(live_source.is_none_or(|source| source == i64::from(page.source_id)))?;
    issued(issued_ids, "documents", i64::from(completion.document_id))?;
    issued(
        issued_ids,
        "chapter_analyses",
        i64::from(segmentation.original_analysis_id),
    )?;

    let mut previous_end = TextOffset::START;
    let mut original_index = None;
    // One immutable analysis identity per A/B/C mode; exact matches retain
    // increasing source token indices even when intervening tokens were replaced.
    let mut metadata_sources = BTreeMap::new();
    let mut occurrences = record.occurrences.iter();
    for (index, segment) in segmentation.segments.iter().enumerate() {
        check_cancelled(cancelled)?;
        let boundary = &segment.boundary;
        let anchor = boundary.anchor;
        let length = i64::from(anchor.range.end) - i64::from(anchor.range.start);
        require(
            anchor.source_id == page.source_id
                && anchor.chapter_id == page.chapter_id
                && anchor.range.start >= previous_end
                && length > 0
                && usize::try_from(length).ok() == Some(boundary.surface.len()),
        )?;
        previous_end = anchor.range.end;
        for offset in [page.range.start, page.range.end, position.offset] {
            if offset >= anchor.range.start && offset <= anchor.range.end {
                let relative = usize::try_from(i64::from(offset) - i64::from(anchor.range.start))
                    .map_err(|_| CandidateError::InvalidData)?;
                require(boundary.surface.is_char_boundary(relative))?;
            }
        }
        if let Some(id) = boundary.override_id {
            issued(issued_ids, "segmentation_overrides", i64::from(id))?;
        }
        if let Some(index) = boundary.original_token_index {
            require(original_index.is_none_or(|previous| index > previous))?;
            original_index = Some(index);
            let metadata = segment
                .metadata
                .as_ref()
                .ok_or(CandidateError::InvalidData)?;
            require(
                metadata.analysis_id == segmentation.original_analysis_id
                    && metadata.mode == SplitMode::B
                    && metadata.token_index == index,
            )?;
        } else {
            require(boundary.override_id.is_some())?;
            require(
                segment
                    .metadata
                    .as_ref()
                    .is_none_or(|m| m.mode != SplitMode::B),
            )?;
        }
        if let Some(metadata) = &segment.metadata {
            require(i64::try_from(metadata.token_index).is_ok())?;
            if let Some(form) = &metadata.dictionary_form {
                Surface::new(form).map_err(|_| CandidateError::InvalidData)?;
            }
            require(
                metadata
                    .reading
                    .as_ref()
                    .is_none_or(|s| !s.trim().is_empty()),
            )?;
            issued(
                issued_ids,
                "chapter_analyses",
                i64::from(metadata.analysis_id),
            )?;
            let mode = match metadata.mode {
                SplitMode::A => 0,
                SplitMode::B => 1,
                SplitMode::C => 2,
            };
            require((metadata.analysis_id == segmentation.original_analysis_id) == (mode == 1))?;
            if let Some((id, previous)) =
                metadata_sources.insert(mode, (metadata.analysis_id, metadata.token_index))
            {
                require(id == metadata.analysis_id && previous < metadata.token_index)?;
            }
            require(
                !metadata_sources
                    .iter()
                    .any(|(other, (id, _))| *other != mode && *id == metadata.analysis_id),
            )?;
        }
        let eligibility = match &segment.metadata {
            Some(metadata) if metadata.is_japanese => {
                Some(EffectiveTokenEligibility::AnalyzerJapanese)
            }
            Some(_) => None,
            None => Some(EffectiveTokenEligibility::ManualJapaneseSpan),
        };
        if eligibility.is_some() {
            Surface::new(&boundary.surface).map_err(|_| CandidateError::InvalidData)?;
        }
        if let Some(eligibility) = eligibility.filter(|_| {
            anchor.range.start >= page.range.start && anchor.range.start < page.range.end
        }) {
            let token = occurrences.next().ok_or(CandidateError::InvalidData)?;
            let display_start = i64::from(anchor.range.start) - i64::from(page.range.start);
            let display_end =
                i64::from(anchor.range.end.min(page.range.end)) - i64::from(page.range.start);
            require(
                token.segment_index == index as u64
                    && token.anchor == anchor
                    && token.surface == boundary.surface
                    && token.eligibility == eligibility
                    && token.metadata == segment.metadata
                    && token.vocabulary.is_none()
                    && i64::from(token.display_range.start) == display_start
                    && i64::from(token.display_range.end) == display_end,
            )?;
        }
    }
    require(occurrences.next().is_none())
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
                departing_byte_offset, page_length, completed_at_unix_ms, active_time_ms
         FROM main.reading_completions WHERE command_id = ?1",
            [command],
            |row| {
                (0..9)
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
                c.pending_time.milliseconds(),
            ],
    )?;
    let segmentation = &record.key.segmentation;
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
                i64::from(segmentation.original_analysis_id),
                segmentation.profile.engine_revision.clone(),
                segmentation.profile.dictionary_revision.clone(),
                "b".into(),
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
