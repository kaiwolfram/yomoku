//! Native boundary prompt snapshots and typed failures, without storage I/O.

use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::{
    analysis::{SplitMode, StoredOverride},
    document::TextAnchor,
    effective_reader::EffectiveReaderTokenKey,
    storage::DocumentError,
    vocabulary::Surface,
};

/// Typed failures preserve the distinction between retryable I/O and a
/// retired prompt. A stale edit must never be automatically rebased/replayed.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "facet_typegen", derive(facet::Facet))]
#[repr(C)]
pub enum EffectiveOverrideFailure {
    Storage(DocumentError),
    InvalidSelection,
    StaleContext,
}

/// Repositories append immutable rules, deactivate only and retain rule rows
/// after document deletion. This pair changes on every supported rule mutation,
/// including split then reversal restoring original B. Not an authorization ID.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct OverrideRuleRevision {
    pub newest_id: i64,
    pub inactive: i64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StoredBoundaryChoice {
    pub mode: SplitMode,
    pub parts: Vec<Surface>,
}

/// Read-only prompt data plus its before-image, for retention by a native core
/// session. Shares the caller's full key: retries never duplicate the chapter.
/// Database scoped; discard on replacement/restore. No Serde/FFI codec.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EffectiveOverrideContext {
    key: Arc<EffectiveReaderTokenKey>,
    anchor: TextAnchor,
    surface: Surface,
    revision: OverrideRuleRevision,
    choices: Vec<StoredBoundaryChoice>,
    rules: Vec<StoredOverride>,
}

impl EffectiveOverrideContext {
    /// Trusted native repository output from one consistent read transaction.
    /// Transactional validation is still required before every mutation.
    pub fn from_snapshot(
        key: Arc<EffectiveReaderTokenKey>,
        anchor: TextAnchor,
        surface: Surface,
        revision: OverrideRuleRevision,
        choices: Vec<StoredBoundaryChoice>,
        rules: Vec<StoredOverride>,
    ) -> Self {
        Self {
            key,
            anchor,
            surface,
            revision,
            choices,
            rules,
        }
    }

    pub fn key(&self) -> &EffectiveReaderTokenKey {
        &self.key
    }

    pub fn rule_revision(&self) -> OverrideRuleRevision {
        self.revision
    }

    pub fn anchor(&self) -> TextAnchor {
        self.anchor
    }

    pub fn surface(&self) -> &Surface {
        &self.surface
    }

    /// Already stored same-profile partitions spanning the exact selection.
    /// Unavailable/crossing alternates are omitted, never guessed.
    pub fn choices(&self) -> &[StoredBoundaryChoice] {
        &self.choices
    }

    /// Rules winning within the selected span, including global rules whose
    /// originating document has since been deleted.
    pub fn rules(&self) -> &[StoredOverride] {
        &self.rules
    }
}
