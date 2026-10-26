//! Typed page and current-progress projections over immutable canonical chapters.

use serde::{Deserialize, Serialize};

use crate::{
    analysis::{ChapterAnalysis, InvalidAnalysis, SplitMode},
    document::{
        AnalysisId, ChapterId, ChapterText, DocumentId, InvalidTextValue, LibraryDocument,
        PastedDocument, ReaderDocument, TextAnchor, TextOffset, TextRange, VocabularyId,
    },
    pagination::{LogicalPages, PageLength, ReadingPosition},
    vocabulary::VocabularyStatus,
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

/// Ordered whole-document text and every chapter's original Mode B identity.
/// Used by the application and worker; the legacy ReaderSnapshot remains only
/// for callers of the historical single-text storage APIs.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "facet_typegen", derive(facet::Facet))]
pub struct DocumentReaderSnapshot {
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

/// One read-only page lookup. Echoed identities let asynchronous callers discard
/// results for a departed page or a different base analysis.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "facet_typegen", derive(facet::Facet))]
pub struct ReaderTokenSnapshot {
    pub document_id: DocumentId,
    pub page: TextAnchor,
    pub analysis: ReaderAnalysis,
    pub tokens: Vec<ReaderToken>,
}

impl ReaderTokenSnapshot {
    pub fn key(&self) -> ReaderTokenKey {
        ReaderTokenKey {
            document_id: self.document_id,
            page: self.page,
            analysis: self.analysis,
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "facet_typegen", derive(facet::Facet))]
pub struct ReaderVocabulary {
    pub id: VocabularyId,
    pub status: VocabularyStatus,
}

/// An explicit choice for an existing visible global entry. Lookup itself never
/// creates a choice; the page/base identity rejects stale UI callbacks.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "facet_typegen", derive(facet::Facet))]
pub struct ReaderStatusChange {
    pub key: ReaderTokenKey,
    pub vocabulary_id: VocabularyId,
    pub status: VocabularyStatus,
    pub occurred_at_unix_ms: i64,
}

/// A selectable Japanese occurrence, retaining its whole original surface even
/// when only part is visible. display_range counts UTF-8 bytes in the page text;
/// anchor counts UTF-8 bytes in the chapter. Neither uses Compose UTF-16 indices.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "facet_typegen", derive(facet::Facet))]
pub struct ReaderToken {
    pub token_index: u64,
    pub anchor: TextAnchor,
    pub display_range: TextRange,
    pub surface: String,
    pub reading: Option<String>,
    pub dictionary_form: Option<String>,
    /// Missing/deleted entries stay absent; lookup must never register them.
    pub vocabulary: Option<ReaderVocabulary>,
}

/// Project visible Japanese spans without changing analysis or vocabulary.
/// Storage attaches current global entries in the same read transaction.
/// Unlike completion's start-based ownership, visibility includes a crossing
/// token on both pages. Selecting either fragment identifies the same full token.
pub fn reader_tokens_on_page(
    chapter: &ChapterText,
    analysis: &ChapterAnalysis,
    page: TextAnchor,
) -> Result<Vec<ReaderToken>, InvalidAnalysis> {
    analysis.validate(chapter)?;
    if analysis.mode != SplitMode::B || page.slice(chapter).map_err(|_| InvalidAnalysis)?.is_empty()
    {
        return Err(InvalidAnalysis);
    }
    analysis
        .tokens
        .iter()
        .enumerate()
        .filter(|(_, token)| {
            token.is_japanese
                && token.anchor.range.start < page.range.end
                && token.anchor.range.end > page.range.start
        })
        .map(|(index, token)| {
            let start = token.anchor.range.start.max(page.range.start);
            let end = token.anchor.range.end.min(page.range.end);
            Ok(ReaderToken {
                token_index: u64::try_from(index).map_err(|_| InvalidAnalysis)?,
                anchor: token.anchor,
                display_range: TextRange {
                    start: TextOffset::try_from(i64::from(start) - i64::from(page.range.start))
                        .map_err(|_| InvalidAnalysis)?,
                    end: TextOffset::try_from(i64::from(end) - i64::from(page.range.start))
                        .map_err(|_| InvalidAnalysis)?,
                },
                surface: token.surface.clone(),
                reading: token.reading.clone(),
                dictionary_form: token
                    .dictionary_form
                    .as_ref()
                    .map(|s| s.as_str().to_owned()),
                vocabulary: None,
            })
        })
        .collect()
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
        })
    }

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
            formatting: Vec::new(),
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
            document_id: self.document_id,
            title: self.title.clone(),
            progress_percent: progress_percent(offset, total)?,
            finished_at_least_once: self.finished_at_least_once,
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

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[cfg_attr(feature = "facet_typegen", derive(facet::Facet))]
pub struct ReaderWordDetails {
    /// Echo the page/base identity and complete selected token for cancellation.
    pub key: ReaderTokenKey,
    pub token: ReaderToken,
    /// Lifetime completed-reading occurrences for this exact surface only.
    pub encountered: u64,
    /// Complete source data plus applicable indices, separately for words/names.
    /// Callers must respect the applicable views when displaying definitions.
    pub candidates: crate::dictionary::DictionaryCandidates,
}

/// A transient selection within the loaded original Mode B token snapshot.
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[cfg_attr(feature = "facet_typegen", derive(facet::Facet))]
pub struct ReaderWordSelection {
    pub key: ReaderTokenKey,
    pub token_index: u64,
}

impl ReaderWordDetails {
    pub fn selection(&self) -> ReaderWordSelection {
        ReaderWordSelection {
            key: self.key,
            token_index: self.token.token_index,
        }
    }
}
