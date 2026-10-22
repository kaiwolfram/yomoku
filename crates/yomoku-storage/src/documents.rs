use rusqlite::{OptionalExtension, TransactionBehavior};
use yomoku_core::document::{
    ChapterId, ChapterText, DocumentId, LibraryDocument, PasteText, PastedDocument, SourceId,
};

use crate::{Storage, StorageError};

impl Storage {
    /// Stable allocation order until Library sorting is implemented.
    pub fn list_documents(&self) -> Result<Vec<LibraryDocument>, StorageError> {
        let mut statement = self
            .connection
            .prepare("SELECT id, title FROM documents ORDER BY id")?;
        let rows = statement.query_map([], |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
        })?;
        rows.map(|row| {
            let (id, title) = row?;
            Ok(LibraryDocument {
                document_id: DocumentId::try_from(id).map_err(|_| StorageError::InvalidDocument)?,
                title,
            })
        })
        .collect()
    }

    /// Persist one paste atomically. This does not tokenize/register vocabulary,
    /// deduplicate imports or start reading; the runtime import pipeline adds those.
    /// There is deliberately no content-edit operation.
    pub fn create_paste(&mut self, text: &PasteText) -> Result<PastedDocument, StorageError> {
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let title = text.initial_title();
        transaction.execute(
            "INSERT INTO sources (kind, original_title) VALUES ('paste', ?1)",
            [title],
        )?;
        let source_id = SourceId::try_from(transaction.last_insert_rowid())
            .map_err(|_| StorageError::InvalidDocument)?;
        transaction.execute(
            "INSERT INTO chapters (source_id, chapter_index) VALUES (?1, 0)",
            [i64::from(source_id)],
        )?;
        let chapter_id = ChapterId::try_from(transaction.last_insert_rowid())
            .map_err(|_| StorageError::InvalidDocument)?;
        transaction.execute(
            "INSERT INTO documents (source_id, title) VALUES (?1, ?2)",
            (i64::from(source_id), title),
        )?;
        let document_id = DocumentId::try_from(transaction.last_insert_rowid())
            .map_err(|_| StorageError::InvalidDocument)?;
        transaction.execute(
            "INSERT INTO chapter_texts (chapter_id, text) VALUES (?1, ?2)",
            (i64::from(chapter_id), text.as_str()),
        )?;
        transaction.commit()?;
        Ok(PastedDocument {
            document_id,
            title: title.to_owned(),
            chapter: ChapterText {
                source_id,
                chapter_id,
                text: text.as_str().to_owned(),
            },
        })
    }

    /// Read a consistent snapshot. None means the Library document is absent;
    /// missing provenance/text for an existing document is an error.
    pub fn load_paste(
        &mut self,
        document_id: DocumentId,
    ) -> Result<Option<PastedDocument>, StorageError> {
        let transaction = self.connection.transaction()?;
        let document = transaction
            .query_row(
                "SELECT source_id, title FROM documents WHERE id = ?1",
                [i64::from(document_id)],
                |row| Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?)),
            )
            .optional()?;
        let Some((source_id, title)) = document else {
            transaction.commit()?;
            return Ok(None);
        };
        let source_id = SourceId::try_from(source_id).map_err(|_| StorageError::InvalidDocument)?;
        let chapter = transaction
            .query_row(
                "SELECT chapters.id, chapter_texts.text
                 FROM sources
                 JOIN chapters ON chapters.source_id = sources.id
                 JOIN chapter_texts ON chapter_texts.chapter_id = chapters.id
                 WHERE sources.id = ?1 AND sources.kind = 'paste' AND chapters.chapter_index = 0",
                [i64::from(source_id)],
                |row| Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?)),
            )
            .optional()?
            .ok_or(StorageError::InvalidDocument)?;
        let chapter_id =
            ChapterId::try_from(chapter.0).map_err(|_| StorageError::InvalidDocument)?;
        let text = PasteText::new(chapter.1).map_err(|_| StorageError::InvalidDocument)?;
        transaction.commit()?;
        Ok(Some(PastedDocument {
            document_id,
            title,
            chapter: ChapterText {
                source_id,
                chapter_id,
                text: text.as_str().to_owned(),
            },
        }))
    }
}
