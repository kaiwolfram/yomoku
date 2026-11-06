//! Page-completion intent values. No clocks or persistence live here.

use serde::{Deserialize, Serialize};

use crate::{
    document::{
        AnalysisId, CompletionCommandId, DocumentId, InvalidTextValue, ReaderDocument, TextAnchor,
    },
    pagination::{LogicalPages, PageLength, ReadingPosition},
};

/// Freeze this payload once per forward action, including its timestamp, and
/// retain the command ID across duplicate callbacks/retries. A reread gets a new ID.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "facet_typegen", derive(facet::Facet))]
pub struct PageCompletion {
    pub command_id: CompletionCommandId,
    pub document_id: DocumentId,
    pub page: TextAnchor,
    pub page_length: PageLength,
    pub departing_position: ReadingPosition,
    pub completed_at_unix_ms: i64,
}

/// Captured from the visible reader; the FIFO reserves an ID for its completion.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "facet_typegen", derive(facet::Facet))]
pub struct PageCompletionIntent {
    pub document_id: DocumentId,
    pub page: TextAnchor,
    pub page_length: PageLength,
    pub departing_position: ReadingPosition,
    pub completed_at_unix_ms: i64,
    pub analysis_id: AnalysisId,
}

/// A dismissible notification for one newly committed command, only when > 0.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "facet_typegen", derive(facet::Facet))]
pub struct CompletionNotice {
    pub command_id: CompletionCommandId,
    pub newly_known: u64,
}

impl PageCompletion {
    /// A chapter's final page advances to the next chapter's start. Only the last
    /// page of the whole document advances to its own chapter EOF.
    pub fn next_document_position(
        self,
        document: &ReaderDocument,
    ) -> Result<ReadingPosition, InvalidTextValue> {
        if self.document_id != document.document_id || self.completed_at_unix_ms < 0 {
            return Err(InvalidTextValue);
        }
        let current = document.current_position()?;
        if self.departing_position != current {
            return Err(InvalidTextValue);
        }
        let pages = LogicalPages::new(&document.chapters, self.page_length)?;
        let index = pages.page_at(current)?;
        if pages.pages()[index] != self.page {
            return Err(InvalidTextValue);
        }
        let next = pages.pages().get(index + 1);
        Ok(ReadingPosition {
            source_id: self.page.source_id,
            chapter_id: next.map_or(self.page.chapter_id, |page| page.chapter_id),
            offset: next.map_or(self.page.range.end, |page| page.range.start),
        })
    }
}

/// Result of recording one reserved effective completion.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "facet_typegen", derive(facet::Facet))]
#[repr(C)]
pub enum AnalyzedCompletionOutcome {
    Recorded {
        newly_known: u64,
    },
    /// Do not show another status-change toast or repeat any side effects.
    AlreadyRecorded,
}
