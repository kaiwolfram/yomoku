//! Compact effective-reader identities backed by one retained Rust page.
//!
//! This is a prerequisite for the coordinated live-reader switch. It is not yet
//! wired into the app. Full chapter provenance never belongs in shell callbacks.

use serde::{Deserialize, Serialize};
use std::sync::Arc;

mod statuses;
pub use statuses::{EffectiveStatusRequest, EffectiveStatusState};
mod words;
pub use words::{EffectiveReaderWordState, EffectiveWordDetails, EffectiveWordRequest};
mod completions;
pub use completions::{
    EffectiveCompletionProgress, EffectiveCompletionRequest, EffectiveCompletionState,
};

use crate::{
    document::{TextAnchor, TextRange},
    effective_reader::{
        EffectiveReaderToken, EffectiveReaderTokenKey, EffectiveReaderTokenSnapshot,
        EffectiveTokenEligibility,
    },
    reader::{ReaderAnalysis, ReaderTokenKey, ReaderVocabulary},
};

/// Valid only in the owning core session. Revisions are never reused there,
/// including when revisiting the exact same page or retrying a failed load.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "facet_typegen", derive(facet::Facet))]
pub struct EffectivePageIdentity {
    pub revision: u64,
    pub base: ReaderTokenKey,
}

/// An effective whole-chapter segment index, never an original B token index.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "facet_typegen", derive(facet::Facet))]
pub struct EffectiveReaderSelection {
    pub page: EffectivePageIdentity,
    pub segment_index: u64,
}

/// Only visible data crosses FFI. Metadata source IDs and rule provenance remain
/// in the retained key and are revalidated by the existing storage APIs.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "facet_typegen", derive(facet::Facet))]
pub struct EffectivePageToken {
    pub segment_index: u64,
    pub anchor: TextAnchor,
    pub display_range: TextRange,
    pub surface: String,
    pub eligibility: EffectiveTokenEligibility,
    pub reading: Option<String>,
    pub dictionary_form: Option<String>,
    pub vocabulary: Option<ReaderVocabulary>,
}

impl From<EffectiveReaderToken> for EffectivePageToken {
    fn from(token: EffectiveReaderToken) -> Self {
        let (reading, dictionary_form) = token
            .metadata
            .map(|metadata| {
                (
                    metadata.reading,
                    metadata
                        .dictionary_form
                        .map(|form| form.as_str().to_owned()),
                )
            })
            .unwrap_or_default();
        Self {
            segment_index: token.segment_index,
            anchor: token.anchor,
            display_range: token.display_range,
            surface: token.surface,
            eligibility: token.eligibility,
            reading,
            dictionary_form,
            vocabulary: token.vocabulary,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "facet_typegen", derive(facet::Facet))]
pub struct EffectiveReaderPage {
    pub identity: EffectivePageIdentity,
    pub tokens: Vec<EffectivePageToken>,
}

/// Rust-only attempt token, issued before dispatch. It cannot be reconstructed
/// from a shell callback or mistaken for a completed page load.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EffectivePageRequest(EffectivePageIdentity);

impl EffectivePageRequest {
    pub fn base(self) -> ReaderTokenKey {
        self.0.base
    }
}

struct RetainedPage {
    key: Option<Arc<EffectiveReaderTokenKey>>,
    view: EffectiveReaderPage,
}

/// Rust-only accepted page command. Sharing the immutable full key makes retry
/// clones cheap and keeps it alive after movement invalidates shell callbacks.
/// Construct through the owning session, before dispatch or checkpoint drain.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EffectivePageCommand {
    identity: EffectivePageIdentity,
    key: Arc<EffectiveReaderTokenKey>,
}

impl EffectivePageCommand {
    pub fn identity(&self) -> EffectivePageIdentity {
        self.identity
    }

    pub fn key(&self) -> &EffectiveReaderTokenKey {
        &self.key
    }
}

/// A visible eligible selection authenticated before accepting a word/custom
/// command. Storage still revalidates this full key in its own transaction.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EffectiveSelectionCommand {
    page: EffectivePageCommand,
    selection: EffectiveReaderSelection,
}

impl EffectiveSelectionCommand {
    pub fn selection(&self) -> EffectiveReaderSelection {
        self.selection
    }

    pub fn key(&self) -> &EffectiveReaderTokenKey {
        self.page.key()
    }
}

/// Own this alongside the core model for its entire lifetime, never per page.
/// At most one full chapter key is retained; replacing/invalidation drops it.
/// Pending completion/custom commands must separately retain their accepted key
/// across retries; clearing the visible page must not discard accepted commands.
#[derive(Default)]
pub struct EffectiveReaderSession {
    revision: u64,
    pending: Option<EffectivePageRequest>,
    current: Option<RetainedPage>,
    words: words::WordState,
    completion: completions::CompletionState,
    status: statuses::StatusState,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EffectiveRevisionExhausted;

impl EffectiveReaderSession {
    /// Movement, reflow, exit, refresh and override edits invalidate immediately,
    /// before any asynchronous read/write. Even an identical later result gets
    /// a fresh revision. Counter exhaustion fails closed rather than wrapping.
    pub fn begin_load(
        &mut self,
        base: ReaderTokenKey,
    ) -> Result<EffectivePageRequest, EffectiveRevisionExhausted> {
        self.invalidate();
        self.revision = self
            .revision
            .checked_add(1)
            .ok_or(EffectiveRevisionExhausted)?;
        let request = EffectivePageRequest(EffectivePageIdentity {
            revision: self.revision,
            base,
        });
        self.pending = Some(request);
        Ok(request)
    }

    pub fn invalidate(&mut self) {
        self.pending = None;
        self.current = None;
        self.dismiss_word();
    }

    /// Accept only the outstanding request and its document/page/original-B
    /// identity. Snapshots must come from the validated repository projection.
    /// A mismatched callback cannot consume a newer attempt or resurrect a page.
    pub fn accept(
        &mut self,
        request: EffectivePageRequest,
        snapshot: EffectiveReaderTokenSnapshot,
    ) -> bool {
        if self.pending != Some(request) {
            return false;
        }
        let analysis = snapshot
            .segmentation
            .as_ref()
            .map_or(ReaderAnalysis::Missing, |segmentation| {
                ReaderAnalysis::Available(segmentation.original_analysis_id)
            });
        if snapshot.document_id != request.0.base.document_id
            || snapshot.page != request.0.base.page
            || analysis != request.0.base.analysis
            || (snapshot.segmentation.is_none() && !snapshot.tokens.is_empty())
        {
            return false;
        }
        // Move the full resolution, avoiding snapshot.key()'s chapter-sized clone.
        let key = snapshot.segmentation.map(|segmentation| {
            Arc::new(EffectiveReaderTokenKey {
                document_id: snapshot.document_id,
                page: snapshot.page,
                segmentation,
            })
        });
        self.current = Some(RetainedPage {
            key,
            view: EffectiveReaderPage {
                identity: request.0,
                tokens: snapshot.tokens.into_iter().map(Into::into).collect(),
            },
        });
        self.pending = None;
        true
    }

    /// No automatic retry. An explicit retry calls begin_load for a new revision.
    pub fn fail(&mut self, request: EffectivePageRequest) -> bool {
        if self.pending != Some(request) {
            return false;
        }
        self.pending = None;
        true
    }

    pub fn page(&self) -> Option<&EffectiveReaderPage> {
        self.current.as_ref().map(|page| &page.view)
    }

    /// Authenticate a page callback before freezing a storage command. Vocabulary
    /// changes do not modify this full key; storage still checks live provenance
    /// transactionally. This reference never crosses the shell boundary.
    pub fn resolve(&self, identity: EffectivePageIdentity) -> Option<&EffectiveReaderTokenKey> {
        let page = self.current.as_ref()?;
        (page.view.identity == identity)
            .then_some(page.key.as_deref())
            .flatten()
    }

    pub fn freeze_page(&self, identity: EffectivePageIdentity) -> Option<EffectivePageCommand> {
        let page = self.current.as_ref()?;
        (page.view.identity == identity).then_some(())?;
        Some(EffectivePageCommand {
            identity,
            key: Arc::clone(page.key.as_ref()?),
        })
    }

    pub fn freeze_selection(
        &self,
        selection: EffectiveReaderSelection,
    ) -> Option<EffectiveSelectionCommand> {
        self.resolve_selection(selection)?;
        Some(EffectiveSelectionCommand {
            page: self.freeze_page(selection.page)?,
            selection,
        })
    }

    /// Only Japanese segments actually visible on the retained page are valid.
    /// Crossing segments keep their whole-chapter identity on either page.
    pub fn resolve_selection(
        &self,
        selection: EffectiveReaderSelection,
    ) -> Option<&EffectiveReaderTokenKey> {
        let key = self.resolve(selection.page)?;
        self.page()?
            .tokens
            .iter()
            .any(|token| token.segment_index == selection.segment_index)
            .then_some(key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exhausted_revision_invalidates_without_reuse() {
        let mut session = EffectiveReaderSession {
            revision: u64::MAX,
            ..Default::default()
        };
        let base = ReaderTokenKey {
            document_id: 1.try_into().unwrap(),
            page: TextAnchor {
                source_id: 1.try_into().unwrap(),
                chapter_id: 1.try_into().unwrap(),
                range: TextRange {
                    start: 0.try_into().unwrap(),
                    end: 3.try_into().unwrap(),
                },
            },
            analysis: ReaderAnalysis::Missing,
        };
        session.pending = Some(EffectivePageRequest(EffectivePageIdentity {
            revision: u64::MAX,
            base,
        }));
        for _ in 0..2 {
            assert_eq!(session.begin_load(base), Err(EffectiveRevisionExhausted));
            assert!(session.pending.is_none());
            assert!(session.page().is_none());
        }
    }
}
