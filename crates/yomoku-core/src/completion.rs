//! Page-completion intent values. No clocks, allocation or persistence live here.

use crate::{
    document::{CompletionCommandId, DocumentId, InvalidTextValue, PastedDocument, TextAnchor},
    pagination::{LogicalPages, PageLength, ReadingPosition},
};

/// Freeze this payload once per forward action, including its timestamp, and
/// retain the command ID across duplicate callbacks/retries. A reread gets a new ID.
/// Repository-only foundation: not yet an application event or FFI payload.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PageCompletion {
    pub command_id: CompletionCommandId,
    pub document_id: DocumentId,
    pub page: TextAnchor,
    pub page_length: PageLength,
    pub departing_position: ReadingPosition,
    pub completed_at_unix_ms: i64,
}

impl PageCompletion {
    /// Validate the exact departing page/position against a single-unit paste.
    /// The final page advances to EOF; finished-book behavior is a separate task.
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
