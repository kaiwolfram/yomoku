//! Read-only access to the independently published dictionary asset. This never
//! opens/migrates the user database or records vocabulary/reading history.

use std::{collections::BTreeMap, error::Error, fmt, path::Path};

use rusqlite::{Connection, OpenFlags};
use yomoku_core::dictionary::{
    matching_readings, NameEntry, Reading, ReadingMatch, Spelling, Translation, WordEntry,
    WordMatch,
};

const APPLICATION_ID: i64 = 0x594f4449;
const SCHEMA_VERSION: i64 = 1;
const LOOKUP_SQL: &str = "SELECT e.source, e.entry_id, e.payload
    FROM forms f JOIN entries e ON e.source = f.source AND e.entry_id = f.entry_id
    WHERE f.surface = ?1 COLLATE BINARY ORDER BY e.source, e.entry_id";

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

/// A candidate keeps every upstream field, plus an explicit applicable view for
/// each matching query form. An empty `matches` licenses no displayed senses.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Candidate<E, M> {
    /// Positive source-scoped ent_seq; word and name IDs may overlap.
    pub entry_id: i64,
    pub entry: E,
    pub forms: Vec<FormMatch<M>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FormMatch<M> {
    pub form: String,
    pub matches: Vec<M>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DictionaryCandidates {
    /// Exact vocabulary identity, never substituted with the base form.
    pub surface: String,
    pub dictionary_form: Option<String>,
    pub words: Vec<Candidate<WordEntry, WordMatch>>,
    pub names: Vec<Candidate<NameEntry, ReadingMatch>>,
}

/// One connection for the I/O worker, separate from mutable `Storage`.
/// The caller supplies a published immutable asset path and owns verification/
/// publication. Opening validates format identity, not a cryptographic checksum;
/// future Android installation must verify bytes before providing this path.
pub struct Dictionary {
    connection: Connection,
}

impl Dictionary {
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
        if surface.trim().is_empty() || dictionary_form.is_some_and(|s| s.trim().is_empty()) {
            return Err(DictionaryError::InvalidQuery);
        }
        let mut words = BTreeMap::<i64, Candidate<WordEntry, WordMatch>>::new();
        let mut names = BTreeMap::<i64, Candidate<NameEntry, ReadingMatch>>::new();
        let mut forms = vec![surface];
        if let Some(base) = dictionary_form.filter(|s| *s != surface) {
            forms.push(base);
        }
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
                            .or_insert_with(|| Candidate {
                                entry_id,
                                entry,
                                forms: Vec::new(),
                            })
                            .forms
                            .push(FormMatch {
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
                            .or_insert_with(|| Candidate {
                                entry_id,
                                entry,
                                forms: Vec::new(),
                            })
                            .forms
                            .push(FormMatch {
                                form: form.to_owned(),
                                matches,
                            });
                    }
                    _ => return Err(DictionaryError::InvalidEntry),
                }
            }
        }
        Ok(DictionaryCandidates {
            surface: surface.to_owned(),
            dictionary_form: dictionary_form.map(str::to_owned),
            words: words.into_values().collect(),
            names: names.into_values().collect(),
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
