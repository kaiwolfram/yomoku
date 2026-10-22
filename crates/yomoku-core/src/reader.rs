//! Typed first-page projection over immutable pasted text.

use serde::{Deserialize, Serialize};

use crate::{
    document::{DocumentId, InvalidTextValue, PastedDocument, TextAnchor},
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
        let pages = LogicalPages::new(
            std::slice::from_ref(&document.chapter),
            PageLength::default(),
        )?;
        let anchor = *pages.pages().first().ok_or(InvalidTextValue)?;
        Ok(Self {
            document_id: document.document_id,
            title: document.title.clone(),
            text: anchor.slice(&document.chapter)?.to_owned(),
            anchor,
            position: ReadingPosition {
                source_id: anchor.source_id,
                chapter_id: anchor.chapter_id,
                offset: anchor.range.start,
            },
            page_number: 1,
            total_pages: pages.pages().len() as u64,
        })
    }
}
