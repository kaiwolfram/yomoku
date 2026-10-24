//! Page-completion intent values. No clocks or persistence live here.

use serde::{Deserialize, Serialize};

use crate::{
    active_time::PendingReadingTime,
    document::{
        AnalysisId, CompletionCommandId, DocumentId, InvalidTextValue, PastedDocument, TextAnchor,
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
    /// Checkpoint first, then freeze the total belonging to this departing page.
    pub pending_time: PendingReadingTime,
}

/// Captured from the visible reader before draining its final checkpoints.
/// The FIFO then reserves an ID and freezes the whole page's durable time.
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

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "facet_typegen", derive(facet::Facet))]
pub struct AnalyzedPageCompletion {
    pub completion: PageCompletion,
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
    /// Validate the exact departing page/position against a single-unit paste.
    /// The final page advances to EOF before the core returns to Library.
    pub fn next_position(
        self,
        document: &PastedDocument,
    ) -> Result<ReadingPosition, InvalidTextValue> {
        if self.document_id != document.document_id || self.completed_at_unix_ms < 0 {
            return Err(InvalidTextValue);
        }
        self.page.slice(&document.chapter)?;
        self.departing_position.validate(&document.chapter)?;
        let current = document.reading_position.unwrap_or(ReadingPosition {
            source_id: document.chapter.source_id,
            chapter_id: document.chapter.chapter_id,
            offset: crate::document::TextOffset::START,
        });
        if self.departing_position != current {
            return Err(InvalidTextValue);
        }
        let pages = LogicalPages::new(std::slice::from_ref(&document.chapter), self.page_length)?;
        if pages.pages()[pages.page_at(current)?] != self.page {
            return Err(InvalidTextValue);
        }
        Ok(ReadingPosition {
            source_id: self.page.source_id,
            chapter_id: self.page.chapter_id,
            offset: self.page.range.end,
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CompletionOutcome {
    Recorded,
    /// No effects, including no position write. A late retry cannot move a reader.
    AlreadyRecorded,
}

/// Analyzed completion is distinct from legacy position/time-only history.
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
