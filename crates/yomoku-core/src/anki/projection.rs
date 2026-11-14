//! Read-only preparation from a current page selection and its dictionary result.

use super::AnkiNote;
use crate::{
    dictionary::{matching_readings, DictionaryCandidates, Translation},
    document::{ChapterText, InvalidTextValue, TextAnchor, TextOffset, TextRange},
    effective_session::{EffectiveReaderPage, EffectiveReaderSelection, EffectiveWordDetails},
    pagination::{is_sentence_end, is_sentence_tail},
    reader::ReaderAnalysis,
    vocabulary::Surface,
};

impl AnkiNote {
    pub(crate) fn from_vocabulary_entry(
        candidates: &DictionaryCandidates,
        entry: super::AnkiEntryKey,
    ) -> Result<Self, InvalidTextValue> {
        let content = entry_content(candidates, entry)?;
        Ok(Self {
            surface: Surface::new(&content.headword).map_err(|_| InvalidTextValue)?,
            reading: content.reading.clone(),
            fields: [
                super::escape(&content.headword),
                super::escape(&content.reading),
                content.meanings,
                String::new(),
            ],
        })
    }

    /// Effective revisions distinguish changed segmentation and page revisits.
    /// Manual spans use only their own metadata, never old B data.
    pub fn from_effective_selection(
        chapter: &ChapterText,
        current: &EffectiveReaderPage,
        selection: EffectiveReaderSelection,
        details: &EffectiveWordDetails,
    ) -> Result<Self, InvalidTextValue> {
        Self::from_effective_entry(chapter, current, selection, details, None)
    }

    pub(crate) fn from_effective_entry(
        chapter: &ChapterText,
        current: &EffectiveReaderPage,
        selection: EffectiveReaderSelection,
        details: &EffectiveWordDetails,
        entry: Option<super::AnkiEntryKey>,
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
            entry,
            &details.dictionary_sources,
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
    entry: Option<super::AnkiEntryKey>,
    sources: &[crate::dictionary::DictionarySource],
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
    let content = entry
        .map(|key| entry_content(candidates, key))
        .transpose()?;
    let (headword, reading, meanings, form) = if let Some(content) = &content {
        (
            content.headword.clone(),
            content.reading.clone(),
            Vec::new(),
            content.form.clone(),
        )
    } else {
        let (reading, meanings) = definitions(candidates, analyzer_reading)?;
        (surface.to_owned(), reading, meanings, surface.to_owned())
    };
    let occurrence = if form == surface || Some(form.as_str()) == dictionary_form {
        target
    } else {
        let source = sources
            .iter()
            .find(|source| source.form == form)
            .ok_or(InvalidTextValue)?;
        if source.anchor.chapter_id != target.chapter_id
            || source.anchor.range.start < target.range.start
            || source.anchor.range.end > target.range.end
        {
            return Err(InvalidTextValue);
        }
        source.anchor.slice(chapter)?;
        source.anchor
    };
    let target = occurrence;
    let sentence = sentence_range(&chapter.text, target.range)?;
    let text = sentence.slice(&chapter.text)?;
    let local = TextRange {
        start: (i64::from(target.range.start) - i64::from(sentence.start)).try_into()?,
        end: (i64::from(target.range.end) - i64::from(sentence.start)).try_into()?,
    };
    let mut note = AnkiNote::with_occurrence(
        &Surface::new(&headword).map_err(|_| InvalidTextValue)?,
        &reading,
        &meanings.iter().map(String::as_str).collect::<Vec<_>>(),
        text,
        local,
    )?;
    if let Some(content) = content {
        note.fields[2] = content.meanings;
    }
    Ok(note)
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
) -> Result<(String, Vec<String>), InvalidTextValue> {
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

struct EntryContent {
    headword: String,
    reading: String,
    meanings: String,
    form: String,
}

fn entry_content(
    candidates: &DictionaryCandidates,
    key: super::AnkiEntryKey,
) -> Result<EntryContent, InvalidTextValue> {
    use super::AnkiEntryKey;
    let (entry_id, pair) = match key {
        AnkiEntryKey::Word { entry_id, reading } | AnkiEntryKey::Name { entry_id, reading } => {
            (entry_id, reading)
        }
    };
    if entry_id <= 0 {
        return Err(InvalidTextValue);
    }
    let mut senses = String::new();
    let (kanji, readings, form, dictionary) = match key {
        AnkiEntryKey::Word { .. } => {
            let candidate = candidates
                .words
                .iter()
                .find(|c| c.entry_id == entry_id)
                .ok_or(InvalidTextValue)?;
            for form in &candidate.forms {
                if form.matches.is_empty() || form.matches != candidate.entry.matches(&form.form) {
                    return Err(InvalidTextValue);
                }
            }
            let form = candidate
                .forms
                .iter()
                .find(|f| f.matches.iter().any(|m| m.reading == pair))
                .ok_or(InvalidTextValue)?;
            for (i, sense) in candidate.entry.senses.iter().enumerate() {
                if candidate
                    .forms
                    .iter()
                    .flat_map(|f| &f.matches)
                    .any(|m| m.reading == pair && m.sense_indices.contains(&i))
                {
                    let labels: Vec<_> = sense
                        .pos
                        .iter()
                        .chain(&sense.field)
                        .chain(&sense.misc)
                        .chain(&sense.dial)
                        .cloned()
                        .collect();
                    senses.push_str(&sense_html(&labels, &sense.glosses, &sense.s_inf));
                }
            }
            (
                &candidate.entry.kanji,
                &candidate.entry.readings,
                &form.form,
                "JMdict",
            )
        }
        AnkiEntryKey::Name { .. } => {
            let candidate = candidates
                .names
                .iter()
                .find(|c| c.entry_id == entry_id)
                .ok_or(InvalidTextValue)?;
            for form in &candidate.forms {
                if form.matches.is_empty()
                    || form.matches
                        != matching_readings(
                            &candidate.entry.kanji,
                            &candidate.entry.readings,
                            &form.form,
                        )
                {
                    return Err(InvalidTextValue);
                }
            }
            let form = candidate
                .forms
                .iter()
                .find(|f| f.matches.contains(&pair))
                .ok_or(InvalidTextValue)?;
            for group in &candidate.entry.names {
                senses.push_str(&sense_html(&group.name_type, &group.translations, &[]));
            }
            (
                &candidate.entry.kanji,
                &candidate.entry.readings,
                &form.form,
                "JMnedict",
            )
        }
    };
    // The pair must occur in validated matches before indexing the immutable payload.
    let reading = readings
        .get(pair.reading_index)
        .ok_or(InvalidTextValue)?
        .text
        .clone();
    let headword = match pair.spelling_index {
        Some(index) => kanji.get(index).ok_or(InvalidTextValue)?.text.clone(),
        None => reading.clone(),
    };
    Ok(EntryContent {
        headword,
        reading,
        meanings: format!("<div class=\"yomoku-dictionary\">{dictionary}</div><ol>{senses}</ol>"),
        form: form.clone(),
    })
}

fn sense_html(labels: &[String], glosses: &[Translation], notes: &[String]) -> String {
    let mut meanings = Vec::new();
    push_meaning(&mut meanings, glosses);
    if meanings.is_empty() {
        return String::new();
    }
    let mut html = String::from("<li>");
    let mut unique_labels = Vec::new();
    for label in labels {
        push_unique(&mut unique_labels, label);
    }
    if !unique_labels.is_empty() {
        html.push_str(&format!(
            "<i class=\"yomoku-labels\">({})</i> ",
            super::escape(&unique_labels.join(", "))
        ));
    }
    html.push_str(&super::escape(&meanings.join("; ")));
    for note in notes {
        html.push_str(&format!(
            "<div class=\"yomoku-note\">{}</div>",
            super::escape(note)
        ));
    }
    html.push_str("</li>");
    html
}
