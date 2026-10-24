//! Typed page and current-progress projections over immutable pasted text.

use serde::{Deserialize, Serialize};

use crate::{
    document::{
        AnalysisId, DocumentId, InvalidTextValue, LibraryDocument, PastedDocument, TextAnchor,
        TextOffset,
    },
    pagination::{LogicalPages, PageLength, ReadingPosition},
};

/// The original stored Mode B base snapshot, independent of the installed engine.
/// Missing is an explicit legacy state: opening never analyzes or registers words.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "facet_typegen", derive(facet::Facet))]
#[repr(C)]
pub enum ReaderAnalysis {
    Available(AnalysisId),
    Missing,
}

/// Document/finish state, settings and analysis identity from one consistent snapshot.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "facet_typegen", derive(facet::Facet))]
pub struct ReaderSnapshot {
    pub document: PastedDocument,
    pub page_length: PageLength,
    pub analysis: ReaderAnalysis,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "facet_typegen", derive(facet::Facet))]
pub struct ReaderPage {
    pub document_id: DocumentId,
    pub title: String,
    pub finished_at_least_once: bool,
    pub text: String,
    pub anchor: TextAnchor,
    pub position: ReadingPosition,
    pub page_number: u64,
    pub total_pages: u64,
}

impl ReaderPage {
    /// Project the supplied exact position. Explicit finished-book reopening is
    /// a separate storage operation; reflow/completion reloads must not reset it.
    pub fn open(document: &PastedDocument, length: PageLength) -> Result<Self, InvalidTextValue> {
        Self::at(
            document,
            document.reading_position.unwrap_or(ReadingPosition {
                source_id: document.chapter.source_id,
                chapter_id: document.chapter.chapter_id,
                offset: TextOffset::START,
            }),
            length,
        )
    }

    pub fn at(
        document: &PastedDocument,
        position: ReadingPosition,
        length: PageLength,
    ) -> Result<Self, InvalidTextValue> {
        let pages = LogicalPages::new(std::slice::from_ref(&document.chapter), length)?;
        let page_index = pages.page_at(position)?;
        let anchor = pages.pages()[page_index];
        Ok(Self {
            document_id: document.document_id,
            title: document.title.clone(),
            finished_at_least_once: document.finished_at_least_once,
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
            finished_at_least_once: self.finished_at_least_once,
            progress_percent: progress_percent(
                self.reading_position
                    .map_or(0, |position| position.offset.into()),
                i64::try_from(self.chapter.text.len()).map_err(|_| InvalidTextValue)?,
            )?,
        })
    }
}
