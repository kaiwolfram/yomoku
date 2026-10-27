//! Source-preferred ruby over canonical UTF-8 text, independent of vocabulary.

use serde::{Deserialize, Serialize};

use crate::document::{ChapterId, ChapterText, InvalidTextValue, TextOffset, TextRange};
use crate::reader::{ReaderAnalysis, ReaderPage, ReaderTokenKey, ReaderTokenSnapshot};

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
    /// Always the complete reading, even when the base crosses a page.
    /// Readings cannot be split proportionally to the base's UTF-8 bytes.
    pub reading: String,
}

fn validate_ruby(text: &str, ruby: &[ReaderRuby]) -> Result<(), InvalidTextValue> {
    let mut end = TextOffset::START;
    for ruby in ruby {
        if ruby.range.start < end || ruby.range.slice(text)?.is_empty() || ruby.reading.is_empty() {
            return Err(InvalidTextValue);
        }
        end = ruby.range.end;
    }
    Ok(())
}

impl ReaderChapterRuby {
    pub fn validate(&self, chapter: &ChapterText) -> Result<(), InvalidTextValue> {
        if self.chapter_id != chapter.chapter_id {
            return Err(InvalidTextValue);
        }
        validate_ruby(&chapter.text, &self.ruby)
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

impl ReaderPage {
    /// Source bases own every overlapping analyzer token. Never guess how to
    /// divide an analyzer reading around source ruby or across a logical page.
    /// Missing, stale or malformed analyzer snapshots leave source ruby usable.
    pub fn furigana(
        &self,
        analysis: ReaderAnalysis,
        snapshot: Option<&ReaderTokenSnapshot>,
    ) -> Result<Vec<ReaderRuby>, InvalidTextValue> {
        validate_ruby(&self.text, &self.source_ruby)?;
        let mut ruby = self.source_ruby.clone();
        let Some(snapshot) = snapshot.filter(|snapshot| self.valid_ruby_tokens(analysis, snapshot))
        else {
            return Ok(ruby);
        };
        let mut source_index = 0;
        for token in &snapshot.tokens {
            while self
                .source_ruby
                .get(source_index)
                .is_some_and(|source| source.range.end <= token.display_range.start)
            {
                source_index += 1;
            }
            let overlaps_source = self
                .source_ruby
                .get(source_index)
                .is_some_and(|source| source.range.start < token.display_range.end);
            if !overlaps_source {
                if let Some(reading) = &token.reading {
                    ruby.push(ReaderRuby {
                        range: token.display_range,
                        reading: reading.clone(),
                    });
                }
            }
        }
        ruby.sort_by_key(|ruby| ruby.range.start);
        Ok(ruby)
    }

    fn valid_ruby_tokens(&self, analysis: ReaderAnalysis, snapshot: &ReaderTokenSnapshot) -> bool {
        let key = ReaderTokenKey {
            document_id: self.document_id,
            page: self.anchor,
            analysis,
        };
        if snapshot.key() != key
            || (analysis == ReaderAnalysis::Missing && !snapshot.tokens.is_empty())
            || i64::from(self.anchor.range.end).checked_sub(self.anchor.range.start.into())
                != i64::try_from(self.text.len()).ok()
        {
            return false;
        }
        let mut previous_end = TextOffset::START;
        let mut previous_index = None;
        for token in &snapshot.tokens {
            let start = token.anchor.range.start.max(self.anchor.range.start);
            let end = token.anchor.range.end.min(self.anchor.range.end);
            let surface_start = i64::from(start) - i64::from(token.anchor.range.start);
            let surface_end = i64::from(end) - i64::from(token.anchor.range.start);
            let surface = usize::try_from(surface_start)
                .ok()
                .zip(usize::try_from(surface_end).ok())
                .and_then(|(start, end)| token.surface.get(start..end));
            if token.anchor.source_id != self.anchor.source_id
                || token.anchor.chapter_id != self.anchor.chapter_id
                || start >= end
                || i64::from(token.anchor.range.end) - i64::from(token.anchor.range.start)
                    != i64::try_from(token.surface.len()).unwrap_or(-1)
                || i64::from(token.display_range.start)
                    != i64::from(start) - i64::from(self.anchor.range.start)
                || i64::from(token.display_range.end)
                    != i64::from(end) - i64::from(self.anchor.range.start)
                || token.display_range.start < previous_end
                || previous_index.is_some_and(|index| index >= token.token_index)
                || token
                    .reading
                    .as_ref()
                    .is_some_and(|reading| reading.trim().is_empty())
                || surface.is_none()
                || token.display_range.slice(&self.text).ok() != surface
            {
                return false;
            }
            previous_end = token.display_range.end;
            previous_index = Some(token.token_index);
        }
        true
    }
}
