//! Connection-local reader validation, page layout and active chapter.

use std::{
    cell::{Ref, RefCell},
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
};

use rusqlite::Connection;
use sha2::{Digest, Sha256};
use yomoku_core::{
    analysis::ChapterAnalysis,
    document::{ChapterId, ChapterText, DocumentId, ReaderDocument, SourceId},
    effective_reader::EffectiveReaderChapter,
    pagination::{PageLayout, PageLength},
};

use crate::{reader_words::SelectionRevision, StorageError};

#[derive(Clone, Copy, PartialEq, Eq)]
struct BinaryRevision {
    local: u64,
    unobserved: u64,
    external: i64,
    schema: i64,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) struct ReaderRevision {
    binary: BinaryRevision,
    canonical: BinaryRevision,
    analysis: u64,
}

pub(crate) struct ActiveChapter {
    pub text: Arc<ChapterText>,
    pub effective: Option<EffectiveReaderChapter>,
}

#[derive(Default)]
pub(crate) struct ReaderIntegrity {
    binary_epoch: Arc<AtomicU64>,
    content_epoch: Arc<AtomicU64>,
    analysis_epoch: Arc<AtomicU64>,
    observed_changes: Arc<AtomicU64>,
    binaries: RefCell<Option<(DocumentId, BinaryRevision)>>,
    document: RefCell<Option<(DocumentId, BinaryRevision, SourceId)>>,
    layout: RefCell<Option<(DocumentId, BinaryRevision, Arc<PageLayout>)>>,
    chapter: RefCell<Option<(DocumentId, ReaderRevision, Arc<ActiveChapter>)>>,
    components: RefCell<Option<Vec<ChapterAnalysis>>>,
}

impl ReaderIntegrity {
    pub fn install(connection: &mut Connection) -> Result<Self, StorageError> {
        let integrity = Self::default();
        let binary = integrity.binary_epoch.clone();
        let content = integrity.content_epoch.clone();
        let analysis = integrity.analysis_epoch.clone();
        let observed = integrity.observed_changes.clone();
        // epub_assets is a rowid table. Trigger/cascade writes also fire the
        // hook. Metadata/count checks still detect deleted rows on every load.
        connection.update_hook(Some(move |_, database: &str, table: &str, _| {
            if database != "main" {
                return;
            }
            observed.fetch_add(1, Ordering::Relaxed);
            match table {
                "epub_assets" => {
                    binary.fetch_add(1, Ordering::Relaxed);
                    content.fetch_add(1, Ordering::Relaxed);
                }
                "documents"
                | "sources"
                | "chapters"
                | "chapter_texts"
                | "paste_content_hashes"
                | "imported_files"
                | "epub_documents"
                | "epub_chapters"
                | "epub_spans"
                | "epub_inline_images" => {
                    content.fetch_add(1, Ordering::Relaxed);
                }
                "chapter_analyses" | "analyzed_tokens" => {
                    analysis.fetch_add(1, Ordering::Relaxed);
                }
                _ => {}
            }
        }))?;
        let binary = integrity.binary_epoch.clone();
        let content = integrity.content_epoch.clone();
        let analysis = integrity.analysis_epoch.clone();
        // A validation made against uncommitted bytes must not survive rollback.
        connection.rollback_hook(Some(move || {
            binary.fetch_add(1, Ordering::Relaxed);
            content.fetch_add(1, Ordering::Relaxed);
            analysis.fetch_add(1, Ordering::Relaxed);
        }))?;
        Ok(integrity)
    }

    /// Read at transaction entry, before content establishes a SQLite snapshot.
    /// Retain this revision rather than sampling again after COMMIT.
    pub fn revision(&self, connection: &Connection) -> Result<ReaderRevision, StorageError> {
        let database = SelectionRevision::read(connection)?;
        // The truncate optimization can skip the row update hook.
        let unobserved = connection
            .total_changes()
            .wrapping_sub(self.observed_changes.load(Ordering::Relaxed));
        let revision = |epoch: &AtomicU64| BinaryRevision {
            local: epoch.load(Ordering::Relaxed),
            unobserved,
            external: database.data_version,
            schema: database.schema_version,
        };
        Ok(ReaderRevision {
            binary: revision(&self.binary_epoch),
            canonical: revision(&self.content_epoch),
            analysis: self.analysis_epoch.load(Ordering::Relaxed),
        })
    }

    pub fn validate_binaries(
        &self,
        connection: &Connection,
        document_id: DocumentId,
        revision: ReaderRevision,
    ) -> Result<(), StorageError> {
        let revision = revision.binary;
        if *self.binaries.borrow() == Some((document_id, revision)) {
            return Ok(());
        }
        self.binaries.replace(None);
        let mut statement =
            connection.prepare("SELECT bytes, sha256 FROM epub_assets WHERE document_id = ?1")?;
        let mut rows = statement.query([i64::from(document_id)])?;
        while let Some(row) = rows.next()? {
            if !valid_binary(row)? {
                return Err(StorageError::InvalidDocument);
            }
        }
        self.binaries.replace(Some((document_id, revision)));
        Ok(())
    }

    pub fn validated_source(
        &self,
        document_id: DocumentId,
        revision: ReaderRevision,
    ) -> Option<SourceId> {
        self.document
            .borrow()
            .filter(|(id, saved, _)| *id == document_id && *saved == revision.canonical)
            .map(|(_, _, source)| source)
    }

    pub fn remember_document(
        &self,
        document_id: DocumentId,
        source_id: SourceId,
        revision: ReaderRevision,
    ) {
        self.document
            .replace(Some((document_id, revision.canonical, source_id)));
    }

    pub fn cached_layout(
        &self,
        document: DocumentId,
        length: PageLength,
        revision: ReaderRevision,
    ) -> Option<Arc<PageLayout>> {
        self.layout
            .borrow()
            .as_ref()
            .filter(|(id, saved, layout)| {
                *id == document && *saved == revision.canonical && layout.length() == length
            })
            .map(|(_, _, layout)| layout.clone())
    }

    pub fn layout(
        &self,
        document: &ReaderDocument,
        length: PageLength,
        revision: ReaderRevision,
    ) -> Result<Arc<PageLayout>, StorageError> {
        if let Some(layout) = self.cached_layout(document.document_id, length, revision) {
            return Ok(layout);
        }
        #[cfg(test)]
        crate::tests::reader_work::LAYOUTS.update(|count| count + 1);
        let layout = Arc::new(
            PageLayout::new(&document.chapters, length)
                .map_err(|_| StorageError::InvalidDocument)?,
        );
        self.layout.replace(Some((
            document.document_id,
            revision.canonical,
            layout.clone(),
        )));
        Ok(layout)
    }

    pub fn chapter(
        &self,
        connection: &Connection,
        document: DocumentId,
        chapter_id: ChapterId,
    ) -> Result<Option<Arc<ActiveChapter>>, StorageError> {
        let revision = self.revision(connection)?;
        if let Some((_, _, chapter)) =
            self.chapter
                .borrow()
                .as_ref()
                .filter(|(id, saved, chapter)| {
                    *id == document && *saved == revision && chapter.text.chapter_id == chapter_id
                })
        {
            return Ok(Some(chapter.clone()));
        }
        self.chapter.replace(None);
        self.components.replace(None);
        let Some(text) = crate::reader_documents::load_reader_chapter_in(
            connection, document, chapter_id, self,
        )?
        else {
            return Ok(None);
        };
        let text = Arc::new(text);
        let effective =
            crate::analysis::read_effective_reader_chapter_in(connection, text.clone())?;
        let chapter = Arc::new(ActiveChapter { text, effective });
        self.chapter
            .replace(Some((document, revision, chapter.clone())));
        Ok(Some(chapter))
    }

    /// Called only after validating the current active chapter in this transaction.
    pub fn components(
        &self,
        load: impl FnOnce() -> Result<Vec<ChapterAnalysis>, StorageError>,
    ) -> Result<Ref<'_, Vec<ChapterAnalysis>>, StorageError> {
        if self.components.borrow().is_none() {
            self.components.replace(Some(load()?));
        }
        Ok(Ref::map(self.components.borrow(), |components| {
            components.as_ref().unwrap()
        }))
    }

    pub fn snapshot_unchanged(
        &self,
        transaction: &Connection,
        before: ReaderRevision,
        expected_document_writes: u64,
    ) -> Result<bool, StorageError> {
        let mut expected = before;
        expected.canonical.local = expected
            .canonical
            .local
            .wrapping_add(expected_document_writes);
        Ok(self.revision(transaction)? == expected)
    }
}

fn valid_binary(row: &rusqlite::Row<'_>) -> rusqlite::Result<bool> {
    // Borrow SQLite's current row rather than allocating another image.
    let bytes = row.get_ref(0)?.as_blob()?;
    let digest = row.get_ref(1)?.as_blob()?;
    Ok(Sha256::digest(bytes).as_slice() == digest)
}
