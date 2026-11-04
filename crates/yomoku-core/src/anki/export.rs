//! Retain accepted content independently of subsequent reader changes.

use super::{AnkiModelLayout, AnkiNote};
use crate::{
    document::{ChapterText, InvalidTextValue},
    effective_session::{EffectiveReaderPage, EffectiveReaderSelection, EffectiveWordDetails},
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

#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "facet_typegen", derive(facet::Facet))]
#[repr(C)]
pub enum AnkiSelection {
    Effective(EffectiveReaderSelection),
}

/// Frozen before dispatch. The adapter must use this payload, not the current
/// popup, dictionary result or subsequently configured default deck.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AnkiExport {
    selection: AnkiSelection,
    pub(crate) entry: Option<super::AnkiEntryKey>,
    deck: AnkiDeckId,
    note: AnkiNote,
    layout: AnkiModelLayout,
}

impl AnkiExport {
    pub(crate) fn for_entry(
        selection: AnkiSelection,
        entry: super::AnkiEntryKey,
        deck: AnkiDeckId,
        note: AnkiNote,
    ) -> Self {
        Self {
            selection,
            entry: Some(entry),
            deck,
            note,
            layout: AnkiModelLayout::yomoku(),
        }
    }

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

    pub(crate) fn from_effective_selection(
        deck: AnkiDeckId,
        chapter: &ChapterText,
        current: &EffectiveReaderPage,
        selection: EffectiveReaderSelection,
        details: &EffectiveWordDetails,
    ) -> Result<Self, InvalidTextValue> {
        Ok(Self {
            selection: AnkiSelection::Effective(selection),
            entry: None,
            deck,
            note: AnkiNote::from_effective_selection(chapter, current, selection, details)?,
            layout: AnkiModelLayout::yomoku(),
        })
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

#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "facet_typegen", derive(facet::Facet))]
#[repr(C)]
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
