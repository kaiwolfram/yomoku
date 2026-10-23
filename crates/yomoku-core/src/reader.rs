//! Typed page and current-progress projections over immutable pasted text.

use serde::{Deserialize, Serialize};

use crate::{
    document::{
        DocumentId, InvalidTextValue, LibraryDocument, PastedDocument, TextAnchor, TextOffset,
    },
    pagination::{LogicalPages, PageLength, ReadingPosition},
};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "facet_typegen", derive(facet::Facet))]
pub struct ReaderPage {
    pub document_id: DocumentId,
    pub title: String,
    pub text: String,
    pub anchor: TextAnchor,
    pub position: ReadingPosition,
    pub page_number: u64,
    pub total_pages: u64,
}

impl ReaderPage {
    pub fn open(document: &PastedDocument) -> Result<Self, InvalidTextValue> {
        Self::at(
            document,
            document.reading_position.unwrap_or(ReadingPosition {
                source_id: document.chapter.source_id,
                chapter_id: document.chapter.chapter_id,
                offset: TextOffset::START,
            }),
        )
    }

    pub fn at(
        document: &PastedDocument,
        position: ReadingPosition,
    ) -> Result<Self, InvalidTextValue> {
        let pages = LogicalPages::new(
            std::slice::from_ref(&document.chapter),
            PageLength::default(),
        )?;
        let page_index = pages.page_at(position)?;
        let anchor = pages.pages()[page_index];
        Ok(Self {
            document_id: document.document_id,
            title: document.title.clone(),
            text: anchor.slice(&document.chapter)?.to_owned(),
            anchor,
            position,
            page_number: page_index as u64 + 1,
            total_pages: pages.pages().len() as u64,
        })
    }
}

/// Floor to whole percent using the canonical UTF-8 offset and total byte length.
/// u128 arithmetic avoids overflow even at the signed 64-bit storage limit.
pub fn progress_percent(offset: i64, total_bytes: i64) -> Result<u64, InvalidTextValue> {
    if offset < 0 || total_bytes < 0 || offset > total_bytes {
        return Err(InvalidTextValue);
    }
    if total_bytes == 0 {
        return Ok(0);
    }
    Ok((offset as u128 * 100 / total_bytes as u128) as u64)
}

impl PastedDocument {
    pub fn library_summary(&self) -> Result<LibraryDocument, InvalidTextValue> {
        if let Some(position) = self.reading_position {
            position.validate(&self.chapter)?;
        }
        Ok(LibraryDocument {
            document_id: self.document_id,
            title: self.title.clone(),
            progress_percent: progress_percent(
                self.reading_position
                    .map_or(0, |position| position.offset.into()),
                i64::try_from(self.chapter.text.len()).map_err(|_| InvalidTextValue)?,
            )?,
        })
    }
}
