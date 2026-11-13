use rusqlite::Connection;
use yomoku_core::effective_reader::{EffectiveReaderToken, EffectiveReaderTokenKey};

use crate::StorageError;

/// Compare only on this owned connection. total_changes covers local DML
/// (including triggers/rollbacks), data_version covers other connections, and
/// schema_version covers local DDL, which does not increment total_changes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct SelectionRevision {
    total_changes: u64,
    data_version: i64,
    schema_version: i64,
}

impl SelectionRevision {
    pub fn with_status_changes(self, changes: u64) -> Self {
        Self {
            total_changes: self.total_changes + changes,
            ..self
        }
    }

    pub fn read(connection: &Connection) -> Result<Self, StorageError> {
        // Read data_version before schema_version establishes the read snapshot.
        // Keep this revision after COMMIT: sampling again could associate an old
        // selection with an external commit made after our transaction ended.
        let data_version = connection.pragma_query_value(None, "data_version", |r| r.get(0))?;
        let schema_version = connection.pragma_query_value(None, "schema_version", |r| r.get(0))?;
        Ok(Self {
            total_changes: connection.total_changes(),
            data_version,
            schema_version,
        })
    }
}

/// Exactly one fully validated selection, bounded independently of sheet use.
/// Vocabulary is deliberately absent; status and ID must be read live.
pub(crate) struct ValidatedSelection {
    key: EffectiveReaderTokenKey,
    pub token: EffectiveReaderToken,
    pub revision: SelectionRevision,
}

impl ValidatedSelection {
    pub fn new(
        key: &EffectiveReaderTokenKey,
        mut token: EffectiveReaderToken,
        revision: SelectionRevision,
    ) -> Self {
        token.vocabulary = None;
        Self {
            key: key.clone(),
            token,
            revision,
        }
    }

    pub fn matches(
        &self,
        key: &EffectiveReaderTokenKey,
        segment_index: u64,
        revision: SelectionRevision,
    ) -> bool {
        self.revision == revision && self.token.segment_index == segment_index && self.key == *key
    }
}
