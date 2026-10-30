//! Indexed literal surface matching, without reading content or analyzer data.

use rusqlite::{params_from_iter, types::Value};
use yomoku_core::{
    document::VocabularyId,
    vocabulary::{Surface, VocabularyPage, VocabularyQuery, VocabularyRow},
};

use crate::{
    vocabulary::{decode_status, encode_status},
    Storage, StorageError,
};

mod page_sql;
pub(super) use page_sql::page_sql;

impl Storage {
    /// Capture every matching ID in one read snapshot, including unloaded rows.
    /// Sort affects presentation only; IDs are returned in ascending order.
    /// Freeze this selection before confirmation and reuse it for explicit Retry.
    /// Never rerun the query when applying a confirmed operation.
    pub fn vocabulary_matching_ids(
        &self,
        query: &VocabularyQuery,
    ) -> Result<Vec<VocabularyId>, StorageError> {
        let (predicate, parameters) = matching(query);
        let mut statement = self.connection.prepare(&format!(
            "SELECT v.id FROM vocabulary v WHERE {predicate} ORDER BY v.id"
        ))?;
        let rows =
            statement.query_map(params_from_iter(&parameters), |row| row.get::<_, i64>(0))?;
        rows.map(|id| VocabularyId::try_from(id?).map_err(|_| StorageError::InvalidVocabulary))
            .collect()
    }

    /// Read 100 matching entries and the full matching count in one snapshot.
    /// The existing BINARY unique surface index bounds prefix searches; existing
    /// occurrence indexes supply lifetime counts and last-completion dates.
    /// Never registers entries, writes history/counters or loads content/dictionaries.
    pub fn vocabulary_page(
        &self,
        query: &VocabularyQuery,
        offset: u64,
    ) -> Result<VocabularyPage, StorageError> {
        let sql_offset = i64::try_from(offset).map_err(|_| StorageError::InvalidVocabulary)?;
        let (predicate, mut parameters) = matching(query);
        let transaction = self.connection.unchecked_transaction()?;
        let count: i64 = transaction.query_row(
            &format!("SELECT COUNT(*) FROM vocabulary v WHERE {predicate}"),
            params_from_iter(&parameters),
            |row| row.get(0),
        )?;
        let matching_count = u64::try_from(count).map_err(|_| StorageError::InvalidVocabulary)?;
        let mut result = VocabularyPage {
            matching_count,
            offset,
            rows: Vec::new(),
            next_offset: None,
        };
        if offset < matching_count {
            parameters.push(Value::Integer(sql_offset));
            let mut statement = transaction.prepare(&page_sql(query.sort, &predicate))?;
            let mut rows = statement.query(params_from_iter(&parameters))?;
            while let Some(row) = rows.next()? {
                let registered_at_unix_ms: i64 = row.get(3)?;
                let count: i64 = row.get(4)?;
                let last_seen_unix_ms: Option<i64> = row.get(5)?;
                if registered_at_unix_ms < 0 {
                    return Err(StorageError::InvalidVocabulary);
                }
                if !row.get::<_, bool>(6)?
                    || last_seen_unix_ms.is_some_and(|date| date < 0)
                    || (count == 0) != last_seen_unix_ms.is_none()
                {
                    return Err(StorageError::InvalidCompletion);
                }
                result.rows.push(VocabularyRow {
                    id: VocabularyId::try_from(row.get::<_, i64>(0)?)
                        .map_err(|_| StorageError::InvalidVocabulary)?,
                    surface: Surface::new(row.get::<_, String>(1)?)
                        .map_err(|_| StorageError::InvalidVocabulary)?
                        .as_str()
                        .to_owned(),
                    status: decode_status(&row.get::<_, String>(2)?)?,
                    registered_at_unix_ms,
                    encountered_count: u64::try_from(count)
                        .map_err(|_| StorageError::InvalidCompletion)?,
                    last_seen_unix_ms,
                });
            }
            let next = offset + result.rows.len() as u64;
            result.next_offset = (next < matching_count).then_some(next);
        }
        transaction.commit()?;
        Ok(result)
    }
}

pub(super) fn matching(query: &VocabularyQuery) -> (String, Vec<Value>) {
    // A scalar-successor upper bound keeps literal %, _, backslash and NUL
    // searchable without LIKE's wildcard/case/collation behavior. UTF-8 BINARY
    // order agrees with Unicode scalar order, including supplementary scalars.
    let mut predicate = "v.surface >= ?".to_owned();
    let mut values = vec![Value::Text(query.prefix.clone())];
    if let Some(upper) = prefix_end(&query.prefix) {
        predicate.push_str(" AND v.surface < ?");
        values.push(Value::Text(upper));
    }
    let statuses: Vec<_> = VocabularyQuery::default()
        .statuses
        .into_iter()
        .filter(|status| query.statuses.contains(status))
        .collect();
    if statuses.is_empty() {
        predicate.push_str(" AND 0");
    } else if statuses.len() < 5 {
        predicate.push_str(" AND v.status IN (");
        // Canonicalize duplicates and bound the number of SQL parameters to five.
        let mut separator = "";
        for status in statuses {
            predicate.push_str(separator);
            predicate.push('?');
            separator = ", ";
            values.push(Value::Text(encode_status(status).to_owned()));
        }
        predicate.push(')');
    }
    (predicate, values)
}

fn prefix_end(prefix: &str) -> Option<String> {
    for (index, scalar) in prefix.char_indices().rev() {
        if scalar != char::MAX {
            // The scalar after U+D7FF is U+E000, skipping the surrogate range.
            let next = char::from_u32(u32::from(scalar) + 1).unwrap_or('\u{e000}');
            let mut upper = prefix[..index].to_owned();
            upper.push(next);
            return Some(upper);
        }
    }
    None
}
