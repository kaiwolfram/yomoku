//! Typed page and current-progress projections over immutable canonical chapters.

use serde::{Deserialize, Serialize};

use crate::{
    document::{
        AnalysisId, ChapterId, DocumentId, InvalidTextValue, LibraryDocument, PastedDocument,
        ReaderDocument, TextAnchor, VocabularyId,
    },
    pagination::{LogicalPages, PageLength, ReadingPosition},
    vocabulary::VocabularyStatus,
};

/// The original stored Mode C base snapshot, independent of the installed engine.
/// Missing is explicit: opening never analyzes or registers words.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "facet_typegen", derive(facet::Facet))]
#[repr(C)]
pub enum ReaderAnalysis {
    Available(AnalysisId),
    Missing,
}

/// Ordered whole-document text and every chapter's original Mode C identity.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "facet_typegen", derive(facet::Facet))]
pub struct DocumentReaderSnapshot {
    pub last_read_sequence: Option<i64>,
    pub typography: crate::typography::ReaderTypography,
    pub document: ReaderDocument,
    pub page_length: PageLength,
    pub analyses: Vec<ReaderChapterAnalysis>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "facet_typegen", derive(facet::Facet))]
pub struct ReaderChapterAnalysis {
    pub chapter_id: ChapterId,
    pub analysis: ReaderAnalysis,
}

impl DocumentReaderSnapshot {
    /// Every chapter must have exactly one explicit original-analysis state.
    /// Entries are keyed by chapter identity, not by their vector order.
    pub fn validate(&self) -> Result<(), InvalidTextValue> {
        self.document.validate()?;
        if self.last_read_sequence.is_some_and(|value| value <= 0) {
            return Err(InvalidTextValue);
        }
        if self.analyses.len() != self.document.chapters.len() {
            return Err(InvalidTextValue);
        }
        let mut seen = std::collections::HashSet::new();
        for entry in &self.analyses {
            self.document.chapter(entry.chapter_id)?;
            if !seen.insert(entry.chapter_id) {
                return Err(InvalidTextValue);
            }
        }
        Ok(())
    }
}

/// Identity of a displayed token lookup, independent of the exact reading offset.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "facet_typegen", derive(facet::Facet))]
pub struct ReaderTokenKey {
    pub document_id: DocumentId,
    pub page: TextAnchor,
    pub analysis: ReaderAnalysis,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "facet_typegen", derive(facet::Facet))]
pub struct ReaderVocabulary {
    pub id: VocabularyId,
    pub status: VocabularyStatus,
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
    pub formatting: Vec<crate::reader_formatting::ReaderTextSpan>,
    pub images: Vec<crate::reader_images::ReaderInlineImage>,
}

impl ReaderPage {
    pub fn open_document(
        document: &ReaderDocument,
        length: PageLength,
    ) -> Result<Self, InvalidTextValue> {
        Self::at_document(document, document.current_position()?, length)
    }

    pub fn at_document(
        document: &ReaderDocument,
        position: ReadingPosition,
        length: PageLength,
    ) -> Result<Self, InvalidTextValue> {
        document.validate()?;
        let pages = LogicalPages::new(&document.chapters, length)?;
        let page_index = pages.page_at(position)?;
        let anchor = pages.pages()[page_index];
        Ok(Self {
            document_id: document.document_id,
            title: document.title.clone(),
            finished_at_least_once: document.finished_at_least_once,
            text: anchor
                .slice(document.chapter(anchor.chapter_id)?)?
                .to_owned(),
            anchor,
            position,
            page_number: page_index as u64 + 1,
            total_pages: pages.pages().len() as u64,
            formatting: document
                .formatting
                .iter()
                .find(|item| item.chapter_id == anchor.chapter_id)
                .map(|item| item.on_page(document.chapter(anchor.chapter_id)?, anchor.range))
                .transpose()?
                .unwrap_or_default(),
            images: document
                .images
                .iter()
                .find(|item| item.chapter_id == anchor.chapter_id)
                .map(|item| item.on_page(document.chapter(anchor.chapter_id)?, anchor.range))
                .transpose()?
                .unwrap_or_default(),
        })
    }
}

impl ReaderDocument {
    pub fn library_summary(&self) -> Result<LibraryDocument, InvalidTextValue> {
        let position = self.current_position()?;
        let mut total = 0_i64;
        let mut preceding = 0_i64;
        for chapter in &self.chapters {
            if chapter.chapter_id == position.chapter_id {
                preceding = total;
            }
            total = total
                .checked_add(i64::try_from(chapter.text.len()).map_err(|_| InvalidTextValue)?)
                .ok_or(InvalidTextValue)?;
        }
        let offset = preceding
            .checked_add(position.offset.into())
            .ok_or(InvalidTextValue)?;
        Ok(LibraryDocument {
            last_read_sequence: None,
            document_id: self.document_id,
            title: self.title.clone(),
            progress_percent: progress_percent(offset, total)?,
            is_read: self.read_override.unwrap_or(self.finished_at_least_once),
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
            last_read_sequence: None,
            document_id: self.document_id,
            title: self.title.clone(),
            is_read: self.read_override.unwrap_or(self.finished_at_least_once),
            progress_percent: progress_percent(
                self.reading_position
                    .map_or(0, |position| position.offset.into()),
                i64::try_from(self.chapter.text.len()).map_err(|_| InvalidTextValue)?,
            )?,
        })
    }
}
