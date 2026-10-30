//! Decode and validate one production status event before pure history replay.

use yomoku_core::{
    document::VocabularyId, statistics::VocabularyStatusChange, vocabulary::VocabularyStatus,
};

use super::{decode_status, StorageError};

pub(super) fn decode(row: &rusqlite::Row<'_>) -> Result<VocabularyStatusChange, StorageError> {
    let version: i64 = row.get(2)?;
    // These fixed enum spellings are needed only during this row's validation.
    // Borrow SQLite text instead of allocating up to three Strings per event.
    // ValueRef enforces TEXT/NULL and UTF-8; no coercion, trimming or lossy decode.
    let old = row
        .get_ref(3)?
        .as_str_or_null()
        .map_err(|_| StorageError::InvalidVocabulary)?
        .map(decode_status)
        .transpose()?;
    let new = decode_status(
        row.get_ref(4)?
            .as_str()
            .map_err(|_| StorageError::InvalidVocabulary)?,
    )?;
    let cause = row
        .get_ref(5)?
        .as_str()
        .map_err(|_| StorageError::InvalidVocabulary)?;
    let command: Option<i64> = row.get(7)?;
    let completion_version: Option<i64> = row.get(8)?;
    let analysis_version: Option<i64> = row.get(9)?;
    let effective_version: Option<i64> = row.get(10)?;
    let valid_cause = match cause {
        "registration" => old.is_none() && new == VocabularyStatus::New && command.is_none(),
        "manual" => old.is_some() && old != Some(new) && command.is_none(),
        "completion" => {
            old == Some(VocabularyStatus::New)
                && new == VocabularyStatus::Known
                && command.is_some()
                && completion_version == Some(1)
                && analysis_version == Some(1)
                && effective_version.is_none_or(|version| version == 1)
        }
        _ => false,
    };
    if version != 1 || !valid_cause {
        return Err(StorageError::InvalidVocabulary);
    }
    Ok(VocabularyStatusChange {
        event_id: u64::try_from(row.get::<_, i64>(0)?)
            .map_err(|_| StorageError::InvalidVocabulary)?,
        vocabulary_id: VocabularyId::try_from(row.get::<_, i64>(1)?)
            .map_err(|_| StorageError::InvalidVocabulary)?,
        occurred_at_unix_ms: row.get(6)?,
        old_status: old,
        new_status: new,
    })
}

#[cfg(test)]
mod tests {
    use rusqlite::{types::Value, Connection};

    use super::*;

    #[test]
    fn enum_text_requires_sqlite_text_and_only_old_status_can_be_null() {
        // Normal STRICT tables disallow BLOBs. Use raw SQLite rows to verify
        // the borrowed decoder itself never coerces them or accepts NULL text.
        let connection = Connection::open_in_memory().unwrap();
        let mut statement = connection
            .prepare("SELECT 1, 1, 1, ?1, ?2, ?3, 100, NULL, NULL, NULL, NULL")
            .unwrap();
        for (index, spelling) in ["new", "known", "manual"].into_iter().enumerate() {
            for invalid in [
                Value::Blob(spelling.as_bytes().to_vec()),
                Value::Integer(1),
                Value::Null,
            ] {
                let mut values = [
                    Value::Text("new".into()),
                    Value::Text("known".into()),
                    Value::Text("manual".into()),
                ];
                values[index] = invalid;
                let mut rows = statement
                    .query(rusqlite::params_from_iter(&values))
                    .unwrap();
                assert!(decode(rows.next().unwrap().unwrap()).is_err(), "{values:?}");
            }
        }
        let mut rows = statement
            .query((None::<String>, "new", "registration"))
            .unwrap();
        assert_eq!(
            decode(rows.next().unwrap().unwrap()).unwrap(),
            VocabularyStatusChange {
                event_id: 1,
                vocabulary_id: VocabularyId::try_from(1).unwrap(),
                occurred_at_unix_ms: 100,
                old_status: None,
                new_status: VocabularyStatus::New,
            }
        );
    }
}
