//! Exact written surfaces are identities: never normalize or substitute lemmas.

use serde::{Deserialize, Serialize};

use crate::document::VocabularyId;

/// Persisted global entry. Registration time is distinct from reading encounters.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VocabularyEntry {
    pub id: VocabularyId,
    pub surface: Surface,
    pub status: VocabularyStatus,
    pub registered_at_unix_ms: i64,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "facet_typegen", derive(facet::Facet))]
#[repr(C)]
pub enum VocabularyStatus {
    #[default]
    New,
    Learning,
    Familiar,
    Known,
    Ignored,
}

/// A validated written surface supplied by the Japanese analyzer.
///
/// This type rejects blank values; it does not attempt Japanese detection.
/// Reading and dictionary form are deliberately not part of identity.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Surface(String);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EmptySurface;

impl std::fmt::Display for EmptySurface {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("a vocabulary surface must not be blank")
    }
}

impl std::error::Error for EmptySurface {}

impl Surface {
    pub fn new(written: impl Into<String>) -> Result<Self, EmptySurface> {
        let written = written.into();
        if written.trim().is_empty() {
            return Err(EmptySurface);
        }
        Ok(Self(written))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Registration policy shared by import and first-display commands.
///
/// This decision does not record a reading encounter or mutate storage.
/// The application must persist a registration event only for a new entry.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Registration {
    CreateNew,
    AlreadyRegistered(VocabularyStatus),
}

pub fn register(existing: Option<VocabularyStatus>) -> Registration {
    match existing {
        None => Registration::CreateNew,
        Some(status) => Registration::AlreadyRegistered(status),
    }
}

/// Status transition for a successfully completed page/post, never for lookup,
/// registration, Previous, or Skip. Transaction/idempotency handling is external.
pub fn status_after_completion(status: VocabularyStatus) -> VocabularyStatus {
    match status {
        VocabularyStatus::New => VocabularyStatus::Known,
        other => other,
    }
}
