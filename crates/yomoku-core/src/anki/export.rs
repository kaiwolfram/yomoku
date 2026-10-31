//! Retain accepted content independently of subsequent reader changes.

use std::sync::Arc;

use super::{AnkiModelLayout, AnkiNote};
use crate::{
    document::{ChapterText, InvalidTextValue},
    effective_session::{EffectiveReaderPage, EffectiveReaderSelection, EffectiveWordDetails},
    reader::{ReaderTokenSnapshot, ReaderWordDetails, ReaderWordSelection},
};

macro_rules! provider_id {
    ($name:ident, $description:literal) => {
        #[doc = $description]
        #[derive(Clone, Copy, Debug, Eq, PartialEq)]
        pub struct $name(i64);

        impl TryFrom<i64> for $name {
            type Error = InvalidTextValue;

            fn try_from(value: i64) -> Result<Self, Self::Error> {
                (value > 0).then_some(Self(value)).ok_or(InvalidTextValue)
            }
        }

        impl From<$name> for i64 {
            fn from(value: $name) -> Self {
                value.0
            }
        }
    };
}

provider_id!(
    AnkiDeckId,
    "An external AnkiDroid deck ID, never a Yomoku ID."
);
provider_id!(
    AnkiNoteId,
    "An external AnkiDroid note ID, never a Yomoku ID."
);
provider_id!(
    AnkiModelId,
    "An external AnkiDroid model ID, never a Yomoku ID."
);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AnkiSelection {
    Original(ReaderWordSelection),
    Effective(EffectiveReaderSelection),
}

/// Frozen before dispatch. The adapter must use this payload, not the current
/// popup, dictionary result or subsequently configured default deck.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AnkiExport {
    selection: AnkiSelection,
    deck: AnkiDeckId,
    note: AnkiNote,
    layout: AnkiModelLayout,
}

impl AnkiExport {
    pub fn selection(&self) -> AnkiSelection {
        self.selection
    }

    pub fn deck(&self) -> AnkiDeckId {
        self.deck
    }

    pub fn note(&self) -> &AnkiNote {
        &self.note
    }

    pub fn layout(&self) -> &AnkiModelLayout {
        &self.layout
    }

    pub(crate) fn from_original_selection(
        deck: AnkiDeckId,
        chapter: &ChapterText,
        current: &ReaderTokenSnapshot,
        selection: ReaderWordSelection,
        details: &ReaderWordDetails,
    ) -> Result<Self, InvalidTextValue> {
        Ok(Self {
            selection: AnkiSelection::Original(selection),
            deck,
            note: AnkiNote::from_original_selection(chapter, current, selection, details)?,
            layout: AnkiModelLayout::yomoku(),
        })
    }

    pub(crate) fn from_effective_selection(
        deck: AnkiDeckId,
        chapter: &ChapterText,
        current: &EffectiveReaderPage,
        selection: EffectiveReaderSelection,
        details: &EffectiveWordDetails,
    ) -> Result<Self, InvalidTextValue> {
        Ok(Self {
            selection: AnkiSelection::Effective(selection),
            deck,
            note: AnkiNote::from_effective_selection(chapter, current, selection, details)?,
            layout: AnkiModelLayout::yomoku(),
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AnkiExportOperation {
    /// Check availability/spec/permission, deck and complete model layout. Check
    /// exact-surface duplicates across models/decks immediately before insertion.
    /// A failed/partial duplicate query must not proceed to insert. Never edit or
    /// delete a discovered note or overwrite an incompatible user model.
    CheckAndAdd,
    /// Read-only exact-surface reconciliation after a possibly committed insert.
    /// An empty query cannot prove the earlier insertion will never commit.
    Reconcile,
}

/// Capture this Rust-only token in the callback. Each dispatch has a fresh
/// attempt; the shared allocation also distinguishes otherwise identical sessions.
#[derive(Clone, Debug)]
pub struct AnkiExportRequest {
    attempt: u64,
    export: Arc<AnkiExport>,
    operation: AnkiExportOperation,
}

impl PartialEq for AnkiExportRequest {
    fn eq(&self, other: &Self) -> bool {
        self.attempt == other.attempt
            && self.operation == other.operation
            && Arc::ptr_eq(&self.export, &other.export)
    }
}

impl Eq for AnkiExportRequest {}

impl AnkiExportRequest {
    pub fn export(&self) -> &AnkiExport {
        &self.export
    }

    pub fn operation(&self) -> AnkiExportOperation {
        self.operation
    }
}

/// All returned IDs must have verified exact first-field surface identity. More
/// than one is a valid existing-note observation, never a reason to add another.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AnkiExistingNotes(Vec<AnkiNoteId>);

impl AnkiExistingNotes {
    pub fn new(ids: Vec<AnkiNoteId>) -> Result<Self, InvalidTextValue> {
        if ids.is_empty() {
            return Err(InvalidTextValue);
        }
        let mut unique = Vec::new();
        for id in ids {
            if !unique.contains(&id) {
                unique.push(id);
            }
        }
        Ok(Self(unique))
    }

    pub fn ids(&self) -> &[AnkiNoteId] {
        &self.0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AnkiExportError {
    ProviderUnavailable,
    UnsupportedProvider,
    PermissionDenied,
    CollectionUnavailable,
    DeckUnavailable,
    IncompatibleModel,
    DuplicateLookupFailed,
    ProviderFailure,
    InvalidResponse,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AnkiAddOutcome {
    Added(AnkiNoteId),
    Existing(AnkiExistingNotes),
    /// Only when the adapter KNOWS no note insertion was submitted/committed.
    /// Exceptions, null IDs and transport loss after submission are Uncertain.
    NotInserted(AnkiExportError),
    Uncertain,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AnkiReconciliation {
    Existing(AnkiExistingNotes),
    /// Includes an empty successful query. No supported atomic idempotency or
    /// provider completion fence has been established; keep insertion blocked.
    Unresolved,
    Failed(AnkiExportError),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AnkiExportResult {
    Add(AnkiAddOutcome),
    Reconcile(AnkiReconciliation),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AnkiExportProgress {
    Pending(AnkiExportOperation),
    Failed(AnkiExportError),
    /// Retry issues only a read, even after permission/query failures or an
    /// empty reconciliation. Clear/replacement cannot erase this obligation.
    Uncertain(Option<AnkiExportError>),
    Added(AnkiNoteId),
    Existing(AnkiExistingNotes),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AnkiBeginError {
    Busy,
    UnresolvedInsertion,
    InvalidSelection,
    AttemptsExhausted,
}

struct RetainedExport {
    export: Arc<AnkiExport>,
    progress: AnkiExportProgress,
}

/// Own for the entire core lifetime, never per popup/page. Dismissing or changing
/// the reader does not cancel provider I/O or discard an uncertain insertion.
/// This is an in-memory prerequisite, not yet app/FFI wiring. Before shipping,
/// the host must also preserve unresolved insert obligations across process death.
#[derive(Default)]
pub struct AnkiExportSession {
    attempt: u64,
    retained: Option<RetainedExport>,
    pending: Option<AnkiExportRequest>,
}

impl AnkiExportSession {
    pub fn export(&self) -> Option<&AnkiExport> {
        self.retained.as_ref().map(|retained| &*retained.export)
    }

    pub fn progress(&self) -> Option<&AnkiExportProgress> {
        self.retained.as_ref().map(|retained| &retained.progress)
    }

    /// Supply the owner's actual CURRENT selection and loaded details, never a
    /// selection reconstructed from a stale shell event. Preparation authenticates
    /// it against the current snapshot and canonical chapter before accepting I/O.
    pub fn begin_original(
        &mut self,
        deck: AnkiDeckId,
        chapter: &ChapterText,
        current: &ReaderTokenSnapshot,
        selection: ReaderWordSelection,
        details: &ReaderWordDetails,
    ) -> Result<AnkiExportRequest, AnkiBeginError> {
        self.can_begin()?;
        let export =
            AnkiExport::from_original_selection(deck, chapter, current, selection, details)
                .map_err(|_| AnkiBeginError::InvalidSelection)?;
        self.dispatch(Arc::new(export), AnkiExportOperation::CheckAndAdd)
    }

    /// Same current-owner requirement as begin_original; effective revisions and
    /// segment indices must match, including after a page revisit or manual edit.
    pub fn begin_effective(
        &mut self,
        deck: AnkiDeckId,
        chapter: &ChapterText,
        current: &EffectiveReaderPage,
        selection: EffectiveReaderSelection,
        details: &EffectiveWordDetails,
    ) -> Result<AnkiExportRequest, AnkiBeginError> {
        self.can_begin()?;
        let export =
            AnkiExport::from_effective_selection(deck, chapter, current, selection, details)
                .map_err(|_| AnkiBeginError::InvalidSelection)?;
        self.dispatch(Arc::new(export), AnkiExportOperation::CheckAndAdd)
    }

    /// Only explicit Retry dispatches again. A safe failure repeats CheckAndAdd
    /// with the exact frozen payload; an uncertain insert can only Reconcile.
    pub fn retry(&mut self) -> Option<AnkiExportRequest> {
        if self.pending.is_some() {
            return None;
        }
        let retained = self.retained.as_ref()?;
        let operation = match retained.progress {
            AnkiExportProgress::Failed(_) => AnkiExportOperation::CheckAndAdd,
            AnkiExportProgress::Uncertain(_) => AnkiExportOperation::Reconcile,
            _ => return None,
        };
        let export = Arc::clone(&retained.export);
        self.dispatch(export, operation).ok()
    }

    /// Accept only the callback token captured at dispatch. Stale/repeated or
    /// other-session results cannot consume a newer request. Wrong result kinds
    /// are conservative: insertion may have happened, so require reconciliation.
    pub fn finish(&mut self, request: &AnkiExportRequest, result: AnkiExportResult) -> bool {
        if self.pending.as_ref() != Some(request) {
            return false;
        }
        let progress = match (request.operation, result) {
            (AnkiExportOperation::CheckAndAdd, AnkiExportResult::Add(outcome)) => match outcome {
                AnkiAddOutcome::Added(id) => AnkiExportProgress::Added(id),
                AnkiAddOutcome::Existing(ids) => AnkiExportProgress::Existing(ids),
                AnkiAddOutcome::NotInserted(error) => AnkiExportProgress::Failed(error),
                AnkiAddOutcome::Uncertain => AnkiExportProgress::Uncertain(None),
            },
            (AnkiExportOperation::Reconcile, AnkiExportResult::Reconcile(outcome)) => match outcome
            {
                AnkiReconciliation::Existing(ids) => AnkiExportProgress::Existing(ids),
                AnkiReconciliation::Unresolved => AnkiExportProgress::Uncertain(None),
                AnkiReconciliation::Failed(error) => AnkiExportProgress::Uncertain(Some(error)),
            },
            _ => AnkiExportProgress::Uncertain(Some(AnkiExportError::InvalidResponse)),
        };
        self.pending = None;
        self.retained.as_mut().expect("pending export").progress = progress;
        true
    }

    /// Cancel a known pre-insert failure or dismiss a completed result. A pending
    /// call or unresolved insertion survives popup dismissal and page navigation.
    pub fn clear(&mut self) -> bool {
        if self.can_begin().is_err() {
            return false;
        }
        self.retained = None;
        true
    }

    fn can_begin(&self) -> Result<(), AnkiBeginError> {
        if self.pending.is_some() {
            return Err(AnkiBeginError::Busy);
        }
        if matches!(self.progress(), Some(AnkiExportProgress::Uncertain(_))) {
            return Err(AnkiBeginError::UnresolvedInsertion);
        }
        Ok(())
    }

    fn dispatch(
        &mut self,
        export: Arc<AnkiExport>,
        operation: AnkiExportOperation,
    ) -> Result<AnkiExportRequest, AnkiBeginError> {
        let attempt = self
            .attempt
            .checked_add(1)
            .ok_or(AnkiBeginError::AttemptsExhausted)?;
        let request = AnkiExportRequest {
            attempt,
            export: Arc::clone(&export),
            operation,
        };
        self.attempt = attempt;
        self.retained = Some(RetainedExport {
            export,
            progress: AnkiExportProgress::Pending(operation),
        });
        self.pending = Some(request.clone());
        Ok(request)
    }
}
