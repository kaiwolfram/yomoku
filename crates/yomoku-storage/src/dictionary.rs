//! Read-only access to the independently published dictionary asset. This never
//! opens/migrates the user database or records vocabulary/reading history.

use std::{collections::BTreeMap, error::Error, fmt, path::Path};

use rusqlite::{Connection, OpenFlags};
use yomoku_core::dictionary::{
    matching_readings, NameEntry, Reading, Spelling, Translation, WordEntry,
};

/// Exact immutable index identity supplied by the verified platform publisher.
pub const IMPORT_DICTIONARY_SHA256: &str =
    "0be9d4493aa3fa4474dbb27291820a63a27748dbc63e5517f67be39ffea8e152";

const APPLICATION_ID: i64 = 0x594f4449;
const SCHEMA_VERSION: i64 = 1;
const LOOKUP_SQL: &str = "SELECT e.source, e.entry_id, e.payload
    FROM forms f JOIN entries e ON e.source = f.source AND e.entry_id = f.entry_id
    WHERE f.surface = ?1 COLLATE BINARY ORDER BY e.source, e.entry_id";
const FIRST_FORM_SQL: &str =
    "SELECT surface FROM forms WHERE surface >= ?1 COLLATE BINARY ORDER BY surface LIMIT 1";

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum IndexedForm {
    Missing,
    Prefix,
    Exact,
}

pub struct FormIndex<'a>(rusqlite::Statement<'a>);

impl FormIndex<'_> {
    /// One ordered index seek answers exact membership and prefix viability.
    fn first(&mut self, prefix: &str) -> Result<Option<String>, DictionaryError> {
        use rusqlite::OptionalExtension;
        Ok(self.0.query_row([prefix], |row| row.get(0)).optional()?)
    }

    pub fn find(&mut self, form: &str) -> Result<IndexedForm, DictionaryError> {
        if form.trim().is_empty() {
            return Err(DictionaryError::InvalidQuery);
        }
        Ok(match self.first(form)?.as_deref() {
            Some(first) if first == form => IndexedForm::Exact,
            Some(first) if first.starts_with(form) => IndexedForm::Prefix,
            _ => IndexedForm::Missing,
        })
    }
}

#[derive(Debug)]
pub enum DictionaryError {
    Sqlite(rusqlite::Error),
    Payload(serde_json::Error),
    UnrecognizedIndex,
    InvalidEntry,
    InvalidQuery,
}

impl fmt::Display for DictionaryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Sqlite(_) => "dictionary SQLite read failed",
            Self::Payload(_) => "invalid dictionary payload",
            Self::UnrecognizedIndex => "unsupported dictionary index",
            Self::InvalidEntry => "inconsistent dictionary entry",
            Self::InvalidQuery => "dictionary forms must not be blank",
        })
    }
}

impl Error for DictionaryError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Sqlite(error) => Some(error),
            Self::Payload(error) => Some(error),
            _ => None,
        }
    }
}

impl From<rusqlite::Error> for DictionaryError {
    fn from(value: rusqlite::Error) -> Self {
        Self::Sqlite(value)
    }
}

impl From<serde_json::Error> for DictionaryError {
    fn from(value: serde_json::Error) -> Self {
        Self::Payload(value)
    }
}

pub use yomoku_core::dictionary::{
    DictionaryCandidates, NameCandidate, NameFormMatch, WordCandidate, WordFormMatch,
};

/// One connection for the I/O worker, separate from mutable `Storage`.
/// The caller supplies a published immutable asset path and owns verification/
/// publication. Opening validates format identity, not a cryptographic checksum;
/// Android installation must verify bytes before providing this path.
pub struct Dictionary {
    connection: Connection,
}

impl Dictionary {
    /// Exact form membership in either source, without reading definition payloads.
    pub fn contains(&self, form: &str) -> Result<bool, DictionaryError> {
        if form.trim().is_empty() {
            return Err(DictionaryError::InvalidQuery);
        }
        Ok(self.connection.query_row(
            "SELECT EXISTS(SELECT 1 FROM forms WHERE surface = ?1 COLLATE BINARY)",
            [form],
            |row| row.get(0),
        )?)
    }

    pub fn form_index(&self) -> Result<FormIndex<'_>, DictionaryError> {
        Ok(FormIndex(self.connection.prepare(FIRST_FORM_SQL)?))
    }

    /// Exact JMdict membership for derived verb forms, without loading entries.
    pub fn contains_word(&self, form: &str) -> Result<bool, DictionaryError> {
        Ok(self.connection.query_row(
            "SELECT EXISTS(SELECT 1 FROM forms WHERE surface = ?1 COLLATE BINARY AND source = 'JMdict')",
            [form],
            |row| row.get(0),
        )?)
    }

    pub fn parts(
        &self,
        surface: &str,
        components: Vec<yomoku_core::dictionary::DictionaryPart>,
    ) -> Result<Vec<yomoku_core::dictionary::DictionaryPart>, DictionaryError> {
        use yomoku_core::{
            dictionary::DictionaryPart,
            document::{TextOffset, TextRange},
        };
        let mut index = self.form_index()?;
        let mut parts: Vec<DictionaryPart> = Vec::new();
        let mut retain = |part: DictionaryPart| {
            if let Some(existing) = parts.iter_mut().find(|old| old.surface == part.surface) {
                if part.range.start < existing.range.start {
                    *existing = part;
                }
            } else {
                parts.push(part);
            }
        };
        for component in components {
            if component.surface == surface
                || component.surface.trim().is_empty()
                || component.range.slice(surface) != Ok(component.surface.as_str())
            {
                continue;
            }
            let exact = index.find(&component.surface)? == IndexedForm::Exact;
            let base = match component
                .dictionary_form
                .as_deref()
                .filter(|base| !base.trim().is_empty())
            {
                Some(base) => index.find(base)? == IndexedForm::Exact,
                None => false,
            };
            if exact || base {
                retain(component);
            }
        }
        for (start, _) in surface.char_indices() {
            for (offset, scalar) in surface[start..].char_indices() {
                let end = start + offset + scalar.len_utf8();
                let part = &surface[start..end];
                let Some(first) = index.first(part)? else {
                    break;
                };
                if !first.starts_with(part) {
                    break;
                }
                if first == part && part != surface && !part.trim().is_empty() {
                    retain(DictionaryPart {
                        surface: part.to_owned(),
                        dictionary_form: None,
                        range: TextRange {
                            start: TextOffset::at(surface, start)
                                .map_err(|_| DictionaryError::InvalidQuery)?,
                            end: TextOffset::at(surface, end)
                                .map_err(|_| DictionaryError::InvalidQuery)?,
                        },
                        primary: false,
                    });
                }
            }
        }
        parts.sort_by_key(|part| {
            (
                std::cmp::Reverse(part.surface.chars().count()),
                part.range.start,
            )
        });
        for index in 0..parts.len() {
            let range = parts[index].range;
            parts[index].primary = !parts[..index]
                .iter()
                .any(|larger| larger.range.start <= range.start && larger.range.end >= range.end);
        }
        Ok(parts)
    }

    /// Never creates a missing file, follows SQLite URI options, or migrates.
    pub fn open(path: impl AsRef<Path>) -> Result<Self, DictionaryError> {
        let connection = Connection::open_with_flags(
            path,
            OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )?;
        let application: i64 =
            connection.pragma_query_value(None, "application_id", |r| r.get(0))?;
        let version: i64 = connection.pragma_query_value(None, "user_version", |r| r.get(0))?;
        if application != APPLICATION_ID || version != SCHEMA_VERSION {
            return Err(DictionaryError::UnrecognizedIndex);
        }
        connection.prepare(LOOKUP_SQL)?;
        Ok(Self { connection })
    }

    /// Union every exact surface/base candidate, separately for words/names,
    /// deduplicated by source/ID and ordered by ID. Keep both form provenances
    /// when they match one entry. No analyzer-reading filter or fuzzy fallback.
    pub fn lookup(
        &self,
        surface: &str,
        dictionary_form: Option<&str>,
    ) -> Result<DictionaryCandidates, DictionaryError> {
        self.lookup_forms(surface, dictionary_form, &[])
    }

    /// Search surface/base first, followed by ordered component forms. Retain
    /// applicable per-form matches, deduplicate entries and rank by first match.
    pub fn lookup_forms(
        &self,
        surface: &str,
        dictionary_form: Option<&str>,
        components: &[String],
    ) -> Result<DictionaryCandidates, DictionaryError> {
        if surface.trim().is_empty() || dictionary_form.is_some_and(|s| s.trim().is_empty()) {
            return Err(DictionaryError::InvalidQuery);
        }
        let mut words = BTreeMap::<i64, WordCandidate>::new();
        let mut names = BTreeMap::<i64, NameCandidate>::new();
        let mut forms = vec![surface];
        if let Some(base) = dictionary_form.filter(|s| *s != surface) {
            forms.push(base);
        }
        for component in components {
            if !component.trim().is_empty() && !forms.contains(&component.as_str()) {
                forms.push(component);
            }
        }
        let query_order = forms.clone();
        let mut statement = self.connection.prepare(LOOKUP_SQL)?;
        for form in forms {
            let mut rows = statement.query([form])?;
            while let Some(row) = rows.next()? {
                let source: String = row.get(0)?;
                let entry_id: i64 = row.get(1)?;
                let payload: String = row.get(2)?;
                if entry_id <= 0 {
                    return Err(DictionaryError::InvalidEntry);
                }
                match source.as_str() {
                    "JMdict" => {
                        let entry: WordEntry = serde_json::from_str(&payload)?;
                        validate_word(&entry, form)?;
                        let matches = entry.matches(form);
                        words
                            .entry(entry_id)
                            .or_insert_with(|| WordCandidate {
                                entry_id,
                                entry,
                                forms: Vec::new(),
                            })
                            .forms
                            .push(WordFormMatch {
                                form: form.to_owned(),
                                matches,
                            });
                    }
                    "JMnedict" => {
                        let entry: NameEntry = serde_json::from_str(&payload)?;
                        validate_name(&entry, form)?;
                        let matches = matching_readings(&entry.kanji, &entry.readings, form);
                        names
                            .entry(entry_id)
                            .or_insert_with(|| NameCandidate {
                                entry_id,
                                entry,
                                forms: Vec::new(),
                            })
                            .forms
                            .push(NameFormMatch {
                                form: form.to_owned(),
                                matches,
                            });
                    }
                    _ => return Err(DictionaryError::InvalidEntry),
                }
            }
        }
        let rank = |form: &str| {
            query_order
                .iter()
                .position(|query| *query == form)
                .unwrap_or(usize::MAX)
        };
        let mut words: Vec<_> = words.into_values().collect();
        let mut names: Vec<_> = names.into_values().collect();
        words.sort_by_key(|entry| {
            (
                entry.forms.iter().map(|form| rank(&form.form)).min(),
                entry.entry_id,
            )
        });
        names.sort_by_key(|entry| {
            (
                entry.forms.iter().map(|form| rank(&form.form)).min(),
                entry.entry_id,
            )
        });
        Ok(DictionaryCandidates {
            surface: surface.to_owned(),
            dictionary_form: dictionary_form.map(str::to_owned),
            words,
            names,
        })
    }
}

fn valid_translations(values: &[Translation]) -> bool {
    !values.is_empty() && values.iter().all(|v| !v.text.is_empty() && v.lang == "eng")
}

fn validate_word(entry: &WordEntry, form: &str) -> Result<(), DictionaryError> {
    validate_forms(&entry.kanji, &entry.readings, form)?;
    if entry.senses.is_empty()
        || entry.senses.iter().any(|s| {
            !valid_translations(&s.glosses)
                || s.stagk
                    .iter()
                    .any(|t| !entry.kanji.iter().any(|k| &k.text == t))
                || s.stagr
                    .iter()
                    .any(|t| !entry.readings.iter().any(|r| &r.text == t))
        })
    {
        return Err(DictionaryError::InvalidEntry);
    }
    Ok(())
}

fn validate_name(entry: &NameEntry, form: &str) -> Result<(), DictionaryError> {
    validate_forms(&entry.kanji, &entry.readings, form)?;
    if entry.names.is_empty()
        || entry
            .names
            .iter()
            .any(|n| !valid_translations(&n.translations))
    {
        return Err(DictionaryError::InvalidEntry);
    }
    Ok(())
}

fn validate_forms(
    kanji: &[Spelling],
    readings: &[Reading],
    form: &str,
) -> Result<(), DictionaryError> {
    if readings.is_empty()
        || kanji.iter().any(|k| k.text.is_empty())
        || readings.iter().any(|r| {
            r.text.is_empty()
                || r.re_restr
                    .iter()
                    .any(|t| !kanji.iter().any(|k| &k.text == t))
        })
        || !kanji.iter().any(|k| k.text == form) && !readings.iter().any(|r| r.text == form)
    {
        return Err(DictionaryError::InvalidEntry);
    }
    Ok(())
}

#[cfg(test)]
mod tests;
