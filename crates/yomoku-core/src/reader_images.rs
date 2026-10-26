//! Image references over immutable canonical text. Asset I/O/decoding stays outside core.

use serde::{Deserialize, Serialize};

use crate::document::{ChapterId, ChapterText, InvalidTextValue, TextOffset, TextRange};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "facet_typegen", derive(facet::Facet))]
pub struct ReaderChapterImages {
    pub chapter_id: ChapterId,
    pub images: Vec<ReaderInlineImage>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "facet_typegen", derive(facet::Facet))]
pub struct ReaderInlineImage {
    /// Exactly one U+FFFC scalar; chapter-local in documents, page-local in pages.
    pub range: TextRange,
    pub asset: ReaderImageAsset,
    pub alt: String,
}

/// Container-relative asset key, scoped by the enclosing document's identity.
/// Never interpret this path as a platform filename or a URL.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "facet_typegen", derive(facet::Facet))]
pub struct ReaderImageAsset {
    pub path: String,
    pub media_type: String,
}

impl ReaderImageAsset {
    pub fn validate(&self) -> Result<(), InvalidTextValue> {
        if self
            .path
            .split('/')
            .any(|part| matches!(part, "" | "." | ".."))
            || self.path.contains([':', '\\'])
            || self.path.chars().any(char::is_control)
            || !self
                .media_type
                .strip_prefix("image/")
                .is_some_and(|subtype| !subtype.is_empty())
            || self
                .media_type
                .chars()
                .any(|c| c.is_whitespace() || c.is_control())
        {
            return Err(InvalidTextValue);
        }
        Ok(())
    }
}

impl ReaderChapterImages {
    pub fn validate(&self, chapter: &ChapterText) -> Result<(), InvalidTextValue> {
        if self.chapter_id != chapter.chapter_id {
            return Err(InvalidTextValue);
        }
        let mut end = TextOffset::START;
        for image in &self.images {
            if image.range.start < end || image.range.slice(&chapter.text)? != "\u{fffc}" {
                return Err(InvalidTextValue);
            }
            image.asset.validate()?;
            end = image.range.end;
        }
        Ok(())
    }

    /// Select whole placeholders, preserving source order and asset identity.
    pub fn on_page(
        &self,
        chapter: &ChapterText,
        page: TextRange,
    ) -> Result<Vec<ReaderInlineImage>, InvalidTextValue> {
        self.validate(chapter)?;
        page.slice(&chapter.text)?;
        self.images
            .iter()
            .filter(|image| image.range.start >= page.start && image.range.end <= page.end)
            .map(|image| {
                Ok(ReaderInlineImage {
                    range: TextRange {
                        start: TextOffset::try_from(
                            i64::from(image.range.start) - i64::from(page.start),
                        )?,
                        end: TextOffset::try_from(
                            i64::from(image.range.end) - i64::from(page.start),
                        )?,
                    },
                    asset: image.asset.clone(),
                    alt: image.alt.clone(),
                })
            })
            .collect()
    }
}
