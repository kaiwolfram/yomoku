//! Source ruby over canonical UTF-8 text, independent of analysis and vocabulary.

use serde::{Deserialize, Serialize};

use crate::document::{ChapterId, ChapterText, InvalidTextValue, TextOffset, TextRange};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "facet_typegen", derive(facet::Facet))]
pub struct ReaderChapterRuby {
    pub chapter_id: ChapterId,
    pub ruby: Vec<ReaderRuby>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "facet_typegen", derive(facet::Facet))]
pub struct ReaderRuby {
    /// Chapter-local in ReaderDocument; clipped page-local in ReaderPage.
    pub range: TextRange,
    /// Always the complete source reading, even when the base crosses a page.
    /// Readings cannot be split proportionally to the base's UTF-8 bytes.
    pub reading: String,
}

impl ReaderChapterRuby {
    pub fn validate(&self, chapter: &ChapterText) -> Result<(), InvalidTextValue> {
        if self.chapter_id != chapter.chapter_id {
            return Err(InvalidTextValue);
        }
        let mut end = TextOffset::START;
        for ruby in &self.ruby {
            if ruby.range.start < end
                || ruby.range.slice(&chapter.text)?.is_empty()
                || ruby.reading.is_empty()
            {
                return Err(InvalidTextValue);
            }
            end = ruby.range.end;
        }
        Ok(())
    }

    /// Clip the visible base without changing canonical text or guessing how
    /// a source reading aligns with individual characters of a compound.
    pub fn on_page(
        &self,
        chapter: &ChapterText,
        page: TextRange,
    ) -> Result<Vec<ReaderRuby>, InvalidTextValue> {
        self.validate(chapter)?;
        page.slice(&chapter.text)?;
        self.ruby
            .iter()
            .filter_map(|ruby| {
                let start = ruby.range.start.max(page.start);
                let end = ruby.range.end.min(page.end);
                (start < end).then_some((ruby, start, end))
            })
            .map(|(ruby, start, end)| {
                Ok(ReaderRuby {
                    range: TextRange {
                        start: TextOffset::try_from(i64::from(start) - i64::from(page.start))?,
                        end: TextOffset::try_from(i64::from(end) - i64::from(page.start))?,
                    },
                    reading: ruby.reading.clone(),
                })
            })
            .collect()
    }
}
