//! One owner for the shared storage slot. Preference requests are independent.

use super::*;

#[derive(Default)]
pub(super) enum PendingStorage {
    #[default]
    Idle,
    Document {
        attempt: u64,
        request: DocumentRequest,
    },
    Effective(EffectiveReaderRequest),
    Statistics(u64, crate::statistics::StatisticsQuery),
    Vocabulary(vocabulary::VocabularyRequest),
    VocabularyWord(vocabulary_word::VocabularyWordRequest),
    DictionaryLookup(
        crate::anki::AnkiSelection,
        crate::dictionary::DictionaryQuery,
    ),
}

pub(super) enum DocumentRequest {
    Library,
    LibraryWords,
    Reader(DocumentId),
    Delete(DocumentId),
    Rename(DocumentId, String),
    SetRead(DocumentId, bool),
    Import,
    PageLength,
    Typography,
    Position,
}

impl DocumentRequest {
    pub(super) fn capture(operation: &StorageOperation) -> Self {
        match operation {
            StorageOperation::List => Self::Library,
            StorageOperation::LoadEffectiveLibraryWordDistributions => Self::LibraryWords,
            StorageOperation::OpenReader(id) => Self::Reader(*id),
            StorageOperation::DeleteDocument(id) => Self::Delete(*id),
            StorageOperation::RenameDocument { document_id, title } => {
                Self::Rename(*document_id, title.clone())
            }
            StorageOperation::SetDocumentRead {
                document_id,
                is_read,
            } => Self::SetRead(*document_id, *is_read),
            StorageOperation::CreatePaste(_)
            | StorageOperation::ImportTxt(_)
            | StorageOperation::ImportSrt(_)
            | StorageOperation::ImportEpub(_) => Self::Import,
            StorageOperation::SavePageLength(_) => Self::PageLength,
            StorageOperation::SaveReaderTypography(_) => Self::Typography,
            StorageOperation::SaveReadingPosition { .. } => Self::Position,
            _ => unreachable!("operation has a dedicated callback owner"),
        }
    }
}

impl Model {
    pub(super) fn storage_busy(&self) -> bool {
        !matches!(self.pending_storage, PendingStorage::Idle)
    }

    pub(super) fn statistics_request(&self) -> Option<(u64, crate::statistics::StatisticsQuery)> {
        match self.pending_storage {
            PendingStorage::Statistics(id, query) => Some((id, query)),
            _ => None,
        }
    }

    pub(super) fn vocabulary_request(&self) -> Option<&vocabulary::VocabularyRequest> {
        match &self.pending_storage {
            PendingStorage::Vocabulary(request) => Some(request),
            _ => None,
        }
    }

    pub(super) fn vocabulary_request_mut(&mut self) -> Option<&mut vocabulary::VocabularyRequest> {
        match &mut self.pending_storage {
            PendingStorage::Vocabulary(request) => Some(request),
            _ => None,
        }
    }

    pub(super) fn vocabulary_word_request(
        &self,
    ) -> Option<&vocabulary_word::VocabularyWordRequest> {
        match &self.pending_storage {
            PendingStorage::VocabularyWord(request) => Some(request),
            _ => None,
        }
    }

    pub(super) fn vocabulary_word_request_mut(
        &mut self,
    ) -> Option<&mut vocabulary_word::VocabularyWordRequest> {
        match &mut self.pending_storage {
            PendingStorage::VocabularyWord(request) => Some(request),
            _ => None,
        }
    }
}
