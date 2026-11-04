//! Frozen in-memory provider payloads, separate from current reader selection.

use serde::{Deserialize, Serialize};

use super::{AnkiDeckId, AnkiModelId, AnkiNote};
use crate::document::InvalidTextValue;

/// Complete expected normal, sort-field-zero, single forward-card layout.
/// Keep the actual strings, so an app upgrade cannot change an unresolved call.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AnkiModelLayout {
    pub name: String,
    pub field_names: [String; 4],
    pub css: String,
    pub card_name: String,
    pub question_format: String,
    pub answer_format: String,
}

impl AnkiModelLayout {
    pub fn yomoku() -> Self {
        Self {
            name: super::MODEL_NAME.into(),
            field_names: super::FIELD_NAMES.map(String::from),
            css: super::CSS.into(),
            card_name: super::CARD_NAME.into(),
            question_format: super::QUESTION_FORMAT.into(),
            answer_format: super::ANSWER_FORMAT.into(),
        }
    }

    fn valid(&self) -> bool {
        !self.name.is_empty()
            && !self.card_name.is_empty()
            && self.field_names.iter().enumerate().all(|(index, name)| {
                !name.is_empty()
                    && !name.contains(['\0', '\u{1f}'])
                    && !self.field_names[..index].contains(name)
            })
            && [
                &self.name,
                &self.css,
                &self.card_name,
                &self.question_format,
                &self.answer_format,
            ]
            .iter()
            .all(|text| !text.contains('\0'))
    }
}

/// Frozen model-creation intent. Never update a partially created model.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AnkiModelCreation {
    layout: AnkiModelLayout,
}

impl AnkiModelCreation {
    pub fn new(layout: AnkiModelLayout) -> Result<Self, InvalidTextValue> {
        if !layout.valid() {
            return Err(InvalidTextValue);
        }
        Ok(Self { layout })
    }

    pub fn layout(&self) -> &AnkiModelLayout {
        &self.layout
    }
}

/// Validated immutable payload. The live owner authenticates the selection
/// before construction. This changes no vocabulary or reading history.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AnkiSubmission {
    note: AnkiNote,
    deck: AnkiDeckId,
    model: AnkiModelId,
    layout: AnkiModelLayout,
}

impl AnkiSubmission {
    pub fn new(
        note: AnkiNote,
        deck: AnkiDeckId,
        model: AnkiModelId,
        layout: AnkiModelLayout,
    ) -> Result<Self, InvalidTextValue> {
        if note.surface().as_str().contains('\0')
            || note
                .fields()
                .iter()
                .any(|text| text.contains(['\0', '\u{1f}']))
            || note.fields()[0] != super::escape(note.surface().as_str())
            || !layout.valid()
        {
            return Err(InvalidTextValue);
        }
        Ok(Self {
            note,
            deck,
            model,
            layout,
        })
    }

    pub fn note(&self) -> &AnkiNote {
        &self.note
    }
    pub fn deck(&self) -> AnkiDeckId {
        self.deck
    }
    pub fn model(&self) -> AnkiModelId {
        self.model
    }
    pub fn layout(&self) -> &AnkiModelLayout {
        &self.layout
    }
}
