//! Read-only preparation from a current page selection and its dictionary result.

use super::AnkiNote;
use crate::{
    dictionary::{matching_readings, DictionaryCandidates, Translation},
    document::{ChapterText, InvalidTextValue, TextAnchor, TextOffset, TextRange},
    effective_session::{EffectiveReaderPage, EffectiveReaderSelection, EffectiveWordDetails},
    pagination::{is_sentence_end, is_sentence_tail},
    reader::{
        ReaderAnalysis, ReaderCustomEntry, ReaderTokenSnapshot, ReaderWordDetails,
        ReaderWordSelection,
    },
    vocabulary::Surface,
};

impl AnkiNote {
    /// The owner supplies its CURRENT selection/snapshot, not values reconstructed
    /// from an export callback. Status and encounter observations may have changed;
    /// all text, metadata and page/analysis identities must still agree.
    pub fn from_original_selection(
        chapter: &ChapterText,
        current: &ReaderTokenSnapshot,
        selection: ReaderWordSelection,
        details: &ReaderWordDetails,
    ) -> Result<Self, InvalidTextValue> {
        if current.key() != selection.key
            || details.selection() != selection
            || current.analysis == ReaderAnalysis::Missing
        {
            return Err(InvalidTextValue);
        }
        let token = current
            .tokens
            .iter()
            .find(|token| token.token_index == selection.token_index)
            .ok_or(InvalidTextValue)?;
        let mut observed = details.token.clone();
        observed.vocabulary = token.vocabulary;
        if &observed != token {
            return Err(InvalidTextValue);
        }
        prepare(
            chapter,
            current.page,
            token.anchor,
            token.display_range,
            &token.surface,
            token.reading.as_deref(),
            token.dictionary_form.as_deref(),
            &details.candidates,
            details.custom_entry.as_ref(),
        )
    }

    /// Effective revisions distinguish changed segmentation and page revisits.
    /// Manual spans use only their own metadata/custom entry, never old B data.
    pub fn from_effective_selection(
        chapter: &ChapterText,
        current: &EffectiveReaderPage,
        selection: EffectiveReaderSelection,
        details: &EffectiveWordDetails,
    ) -> Result<Self, InvalidTextValue> {
        if current.identity != selection.page
            || details.selection != selection
            || current.identity.base.analysis == ReaderAnalysis::Missing
        {
            return Err(InvalidTextValue);
        }
        let token = current
            .tokens
            .iter()
            .find(|token| token.segment_index == selection.segment_index)
            .ok_or(InvalidTextValue)?;
        let mut observed = details.token.clone();
        observed.vocabulary = token.vocabulary;
        if &observed != token {
            return Err(InvalidTextValue);
        }
        prepare(
            chapter,
            current.identity.base.page,
            token.anchor,
            token.display_range,
            &token.surface,
            token.reading.as_deref(),
            token.dictionary_form.as_deref(),
            &details.candidates,
            details.custom_entry.as_ref(),
        )
    }
}

#[allow(clippy::too_many_arguments)]
fn prepare(
    chapter: &ChapterText,
    page: TextAnchor,
    target: TextAnchor,
    display: TextRange,
    surface: &str,
    analyzer_reading: Option<&str>,
    dictionary_form: Option<&str>,
    candidates: &DictionaryCandidates,
    custom: Option<&ReaderCustomEntry>,
) -> Result<AnkiNote, InvalidTextValue> {
    page.slice(chapter)?;
    if target.slice(chapter)? != surface
        || candidates.surface != surface
        || candidates.dictionary_form.as_deref() != dictionary_form
        || target.range.start >= page.range.end
        || target.range.end <= page.range.start
    {
        return Err(InvalidTextValue);
    }
    let expected_display = TextRange {
        start: (i64::from(target.range.start.max(page.range.start)) - i64::from(page.range.start))
            .try_into()?,
        end: (i64::from(target.range.end.min(page.range.end)) - i64::from(page.range.start))
            .try_into()?,
    };
    if display != expected_display {
        return Err(InvalidTextValue);
    }
    let (reading, meanings) = definitions(candidates, analyzer_reading, custom)?;
    let sentence = sentence_range(&chapter.text, target.range)?;
    let text = sentence.slice(&chapter.text)?;
    let local = TextRange {
        start: (i64::from(target.range.start) - i64::from(sentence.start)).try_into()?,
        end: (i64::from(target.range.end) - i64::from(sentence.start)).try_into()?,
    };
    AnkiNote::new(
        &Surface::new(surface).map_err(|_| InvalidTextValue)?,
        &reading,
        &meanings.iter().map(String::as_str).collect::<Vec<_>>(),
        text,
        local,
    )
}

// Same lightweight punctuation rules as pagination, plus canonical hard lines.
// Boundaries inside a manually merged target cannot truncate that target.
fn sentence_range(text: &str, target: TextRange) -> Result<TextRange, InvalidTextValue> {
    if target.slice(text)?.is_empty() {
        return Err(InvalidTextValue);
    }
    let target_start = target.start.byte_index(text)?;
    let target_end = target.end.byte_index(text)?;
    let mut start = 0;
    let mut end = text.len();
    let mut chars = text.char_indices().peekable();
    while let Some((index, scalar)) = chars.next() {
        let mut boundary = index + scalar.len_utf8();
        if scalar == '\r' {
            if chars.peek().is_some_and(|(_, next)| *next == '\n') {
                chars.next();
                boundary += 1;
            }
        } else if scalar != '\n' {
            if !is_sentence_end(scalar, chars.peek().map(|(_, next)| *next)) {
                continue;
            }
            while let Some(&(index, tail)) = chars.peek() {
                if !is_sentence_tail(tail) {
                    break;
                }
                chars.next();
                boundary = index + tail.len_utf8();
            }
        }
        if boundary <= target_start {
            start = boundary;
        } else if boundary >= target_end {
            end = boundary;
            break;
        }
    }
    Ok(TextRange {
        start: TextOffset::at(text, start)?,
        end: TextOffset::at(text, end)?,
    })
}

fn definitions(
    candidates: &DictionaryCandidates,
    analyzer_reading: Option<&str>,
    custom: Option<&ReaderCustomEntry>,
) -> Result<(String, Vec<String>), InvalidTextValue> {
    if let Some(custom) = custom {
        if !candidates.words.is_empty()
            || !candidates.names.is_empty()
            || custom.surface != candidates.surface
            || custom.reading.trim().is_empty()
            || custom.english_meaning.trim().is_empty()
        {
            return Err(InvalidTextValue);
        }
        return Ok((custom.reading.clone(), vec![custom.english_meaning.clone()]));
    }
    let valid_form = |form: &str| {
        form == candidates.surface || Some(form) == candidates.dictionary_form.as_deref()
    };
    let mut readings = Vec::new();
    let mut meanings = Vec::new();
    for candidate in &candidates.words {
        if candidate.entry_id <= 0 || candidate.forms.is_empty() {
            return Err(InvalidTextValue);
        }
        let mut selected_readings = vec![false; candidate.entry.readings.len()];
        let mut senses = vec![false; candidate.entry.senses.len()];
        for form in &candidate.forms {
            if !valid_form(&form.form)
                || form.matches.is_empty()
                || form.matches != candidate.entry.matches(&form.form)
            {
                return Err(InvalidTextValue);
            }
            for found in &form.matches {
                selected_readings[found.reading.reading_index] = true;
                for &sense in &found.sense_indices {
                    senses[sense] = true;
                }
            }
        }
        for (reading, selected) in candidate.entry.readings.iter().zip(selected_readings) {
            if selected {
                push_unique(&mut readings, &reading.text);
            }
        }
        for (sense, selected) in candidate.entry.senses.iter().zip(senses) {
            if selected {
                push_meaning(&mut meanings, &sense.glosses);
            }
        }
    }
    for candidate in &candidates.names {
        if candidate.entry_id <= 0 || candidate.forms.is_empty() {
            return Err(InvalidTextValue);
        }
        let mut selected_readings = vec![false; candidate.entry.readings.len()];
        for form in &candidate.forms {
            if !valid_form(&form.form)
                || form.matches.is_empty()
                || form.matches
                    != matching_readings(
                        &candidate.entry.kanji,
                        &candidate.entry.readings,
                        &form.form,
                    )
            {
                return Err(InvalidTextValue);
            }
            for found in &form.matches {
                selected_readings[found.reading_index] = true;
            }
        }
        for (reading, selected) in candidate.entry.readings.iter().zip(selected_readings) {
            if selected {
                push_unique(&mut readings, &reading.text);
            }
        }
        for group in &candidate.entry.names {
            push_meaning(&mut meanings, &group.translations);
        }
    }
    if readings.is_empty() {
        if let Some(reading) = analyzer_reading {
            push_unique(&mut readings, reading);
        }
    }
    Ok((readings.join("、"), meanings))
}

fn push_unique(values: &mut Vec<String>, text: &str) {
    if !values.iter().any(|value| value == text) {
        values.push(text.into());
    }
}

fn push_meaning(values: &mut Vec<String>, translations: &[Translation]) {
    let english: Vec<_> = translations
        .iter()
        .filter(|translation| translation.lang == "eng")
        .map(|translation| {
            let qualifiers: Vec<_> = [translation.g_type.as_deref(), translation.g_gend.as_deref()]
                .into_iter()
                .flatten()
                .collect();
            if qualifiers.is_empty() {
                translation.text.clone()
            } else {
                format!("{} ({})", translation.text, qualifiers.join(", "))
            }
        })
        .collect();
    if !english.is_empty() {
        values.push(english.join("; "));
    }
}
