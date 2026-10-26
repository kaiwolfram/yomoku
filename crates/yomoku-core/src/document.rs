//! Stable identities and chapter-local anchors into immutable canonical text.
//!
//! Offsets count UTF-8 bytes, not characters, graphemes or UTF-16 code units.
//! Constructors/Serde check numeric bounds; operations check the actual text.

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InvalidTextValue;

impl std::fmt::Display for InvalidTextValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("invalid identity, text offset or range")
    }
}

impl std::error::Error for InvalidTextValue {}

macro_rules! identity {
    ($name:ident, $description:literal) => {
        #[doc = $description]
        #[derive(
            Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize,
        )]
        #[cfg_attr(feature = "facet_typegen", derive(facet::Facet))]
        #[serde(try_from = "i64", into = "i64")]
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

identity!(
    DocumentId,
    "A library document instance; reimport gets a fresh identity."
);
identity!(
    SourceId,
    "Historical provenance retained after its document is deleted."
);
identity!(
    ChapterId,
    "An immutable canonical text unit, independent of logical pages."
);
identity!(
    CompletionCommandId,
    "A database-issued completion intent; retries reuse it, rereads allocate another."
);
identity!(
    VocabularyId,
    "A global surface entry; deletion and later registration receive a fresh identity."
);
identity!(
    AnalysisId,
    "An immutable, versioned chapter analysis snapshot."
);
identity!(
    OverrideId,
    "A reversible segmentation override, independent of vocabulary identity."
);

/// Validated paste input. Keep the exact text so durable anchors remain valid.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PasteText(String);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EmptyPaste;

impl std::fmt::Display for EmptyPaste {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("paste text must contain non-whitespace content")
    }
}

impl std::error::Error for EmptyPaste {}

impl PasteText {
    pub fn new(text: String) -> Result<Self, EmptyPaste> {
        if text.trim().is_empty() {
            return Err(EmptyPaste);
        }
        Ok(Self(text))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// The literal first line, excluding LF/CRLF; no trim, fallback or truncation.
    pub fn initial_title(&self) -> &str {
        self.0.lines().next().unwrap_or_default()
    }
}

/// A persisted single-text document (paste, TXT or SRT), with an independent title.
/// Retained for the historical single-text repository APIs.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "facet_typegen", derive(facet::Facet))]
pub struct PastedDocument {
    pub document_id: DocumentId,
    pub title: String,
    pub finished_at_least_once: bool,
    pub chapter: ChapterText,
    pub reading_position: Option<crate::pagination::ReadingPosition>,
}

/// Complete canonical text in source order, including single-chapter documents.
/// Chapter-local offsets retain their identity across page reflow and movement.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "facet_typegen", derive(facet::Facet))]
pub struct ReaderDocument {
    pub document_id: DocumentId,
    pub title: String,
    pub finished_at_least_once: bool,
    pub chapters: Vec<ChapterText>,
    pub formatting: Vec<crate::reader_formatting::ReaderChapterFormatting>,
    pub reading_position: Option<crate::pagination::ReadingPosition>,
}

impl From<PastedDocument> for ReaderDocument {
    fn from(document: PastedDocument) -> Self {
        Self {
            document_id: document.document_id,
            title: document.title,
            finished_at_least_once: document.finished_at_least_once,
            chapters: vec![document.chapter],
            formatting: Vec::new(),
            reading_position: document.reading_position,
        }
    }
}

impl ReaderDocument {
    /// Validate source ownership, unique chapter identities and the saved scalar
    /// boundary. Empty text units are allowed; an empty chapter list is invalid.
    pub fn validate(&self) -> Result<(), InvalidTextValue> {
        let first = self.chapters.first().ok_or(InvalidTextValue)?;
        let mut seen = std::collections::HashSet::new();
        for chapter in &self.chapters {
            if chapter.source_id != first.source_id || !seen.insert(chapter.chapter_id) {
                return Err(InvalidTextValue);
            }
            i64::try_from(chapter.text.len()).map_err(|_| InvalidTextValue)?;
        }
        let mut formatted = std::collections::HashSet::new();
        for formatting in &self.formatting {
            if !formatted.insert(formatting.chapter_id) {
                return Err(InvalidTextValue);
            }
            formatting.validate(self.chapter(formatting.chapter_id)?)?;
        }
        if let Some(position) = self.reading_position {
            position.validate(self.chapter(position.chapter_id)?)?;
        }
        Ok(())
    }

    pub fn chapter(&self, chapter_id: ChapterId) -> Result<&ChapterText, InvalidTextValue> {
        self.chapters
            .iter()
            .find(|chapter| chapter.chapter_id == chapter_id)
            .ok_or(InvalidTextValue)
    }

    pub fn current_position(&self) -> Result<crate::pagination::ReadingPosition, InvalidTextValue> {
        self.validate()?;
        Ok(self
            .reading_position
            .unwrap_or(crate::pagination::ReadingPosition {
                source_id: self.chapters[0].source_id,
                chapter_id: self.chapters[0].chapter_id,
                offset: TextOffset::START,
            }))
    }
}

/// Nonnegative UTF-8 byte offset within one canonical chapter, including EOF.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[cfg_attr(feature = "facet_typegen", derive(facet::Facet))]
#[serde(try_from = "i64", into = "i64")]
pub struct TextOffset(i64);

impl TryFrom<i64> for TextOffset {
    type Error = InvalidTextValue;

    fn try_from(value: i64) -> Result<Self, Self::Error> {
        (value >= 0).then_some(Self(value)).ok_or(InvalidTextValue)
    }
}

impl From<TextOffset> for i64 {
    fn from(value: TextOffset) -> Self {
        value.0
    }
}

impl TextOffset {
    pub const START: Self = Self(0);

    /// Validate an offset against this chapter's exact UTF-8 text.
    pub fn at(text: &str, byte_index: usize) -> Result<Self, InvalidTextValue> {
        if !text.is_char_boundary(byte_index) {
            return Err(InvalidTextValue);
        }
        Ok(Self(
            i64::try_from(byte_index).map_err(|_| InvalidTextValue)?,
        ))
    }

    pub fn byte_index(self, text: &str) -> Result<usize, InvalidTextValue> {
        let index = usize::try_from(self.0).map_err(|_| InvalidTextValue)?;
        Self::at(text, index)?;
        Ok(index)
    }

    /// Kotlin/Compose indices count UTF-16 units. Never accept half a surrogate.
    pub fn from_utf16(text: &str, utf16_index: usize) -> Result<Self, InvalidTextValue> {
        let mut units = 0;
        for (byte_index, scalar) in text.char_indices() {
            if units == utf16_index {
                return Self::at(text, byte_index);
            }
            units += scalar.len_utf16();
            if units > utf16_index {
                return Err(InvalidTextValue);
            }
        }
        if units == utf16_index {
            Self::at(text, text.len())
        } else {
            Err(InvalidTextValue)
        }
    }

    pub fn to_utf16(self, text: &str) -> Result<usize, InvalidTextValue> {
        Ok(text[..self.byte_index(text)?].encode_utf16().count())
    }
}

/// Half-open [start, end) range; scalar boundaries may bisect a grapheme.
/// Contextual validation is required after deserialization and before use.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "facet_typegen", derive(facet::Facet))]
pub struct TextRange {
    pub start: TextOffset,
    pub end: TextOffset,
}

impl TextRange {
    pub fn slice(self, text: &str) -> Result<&str, InvalidTextValue> {
        text.get(self.start.byte_index(text)?..self.end.byte_index(text)?)
            .ok_or(InvalidTextValue)
    }
}

/// Historical/token anchor. Chapter ownership must be checked by repositories.
/// It deliberately has no FK dependency on a live document or logical page.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "facet_typegen", derive(facet::Facet))]
pub struct TextAnchor {
    pub source_id: SourceId,
    pub chapter_id: ChapterId,
    pub range: TextRange,
}

/// Snapshot of canonical chapter text. Persistence must never update its text.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "facet_typegen", derive(facet::Facet))]
pub struct ChapterText {
    pub source_id: SourceId,
    pub chapter_id: ChapterId,
    pub text: String,
}

impl TextAnchor {
    pub fn slice(self, chapter: &ChapterText) -> Result<&str, InvalidTextValue> {
        if self.source_id != chapter.source_id || self.chapter_id != chapter.chapter_id {
            return Err(InvalidTextValue);
        }
        self.range.slice(&chapter.text)
    }
}

/// Current progress and a permanent finish marker that is independent of position.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "facet_typegen", derive(facet::Facet))]
pub struct LibraryDocument {
    pub document_id: DocumentId,
    pub title: String,
    pub progress_percent: u64,
    pub finished_at_least_once: bool,
}
