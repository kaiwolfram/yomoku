use rusqlite::Connection;

use yomoku_core::{
    document::{DocumentId, TextAnchor},
    effective_reader::EffectiveReaderTokenSnapshot,
    reader::ReaderVocabulary,
    vocabulary::Surface,
};

use crate::{
    analysis::read_effective_reader_chapter_in, reader_documents::load_reader_chapter_in,
    reader_integrity::ReaderIntegrity, vocabulary::read_entry_by_surface, Storage, StorageError,
};

impl Storage {
    /// Read effective visible spans, exact metadata and live exact-surface statuses
    /// in one transaction without registering surfaces. None means deleted document;
    /// a snapshot with absent segmentation means missing original Mode C.
    pub fn load_effective_reader_tokens(
        &mut self,
        document_id: DocumentId,
        page: TextAnchor,
    ) -> Result<Option<EffectiveReaderTokenSnapshot>, StorageError> {
        let transaction = self.connection.transaction()?;
        let snapshot = load_effective_reader_tokens_in(
            &transaction,
            document_id,
            page,
            &self.reader_integrity,
        )?;
        transaction.commit()?;
        Ok(snapshot)
    }
}

pub(crate) fn load_effective_reader_tokens_in(
    connection: &Connection,
    document_id: DocumentId,
    page: TextAnchor,
    integrity: &ReaderIntegrity,
) -> Result<Option<EffectiveReaderTokenSnapshot>, StorageError> {
    let Some(chapter) =
        load_reader_chapter_in(connection, document_id, page.chapter_id, integrity)?
    else {
        return Ok(None);
    };
    if page
        .slice(&chapter)
        .map_err(|_| StorageError::InvalidAnalysis)?
        .is_empty()
    {
        return Err(StorageError::InvalidAnalysis);
    }
    let effective = read_effective_reader_chapter_in(connection, &chapter)?;
    let mut tokens = effective
        .as_ref()
        .map(|effective| effective.tokens_on_page(page))
        .transpose()
        .map_err(|_| StorageError::InvalidAnalysis)?
        .unwrap_or_default();
    // Repeated visible surfaces share one indexed lookup. Missing entries stay
    // absent; immutable analyzer tokens have no vocabulary foreign key.
    let mut entries = std::collections::BTreeMap::new();
    for token in &mut tokens {
        let surface = Surface::new(&token.surface).map_err(|_| StorageError::InvalidAnalysis)?;
        let entry = match entries.entry(surface) {
            std::collections::btree_map::Entry::Occupied(entry) => *entry.get(),
            std::collections::btree_map::Entry::Vacant(entry) => {
                let vocabulary =
                    read_entry_by_surface(connection, entry.key())?.map(|entry| ReaderVocabulary {
                        id: entry.id,
                        status: entry.status,
                    });
                *entry.insert(vocabulary)
            }
        };
        token.vocabulary = entry;
    }
    let snapshot = EffectiveReaderTokenSnapshot {
        document_id,
        page,
        segmentation: effective.map(|chapter| chapter.into_segmentation()),
        tokens,
    };
    Ok(Some(snapshot))
}
