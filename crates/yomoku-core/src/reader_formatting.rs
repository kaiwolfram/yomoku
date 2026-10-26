//! Source formatting over canonical UTF-8 text; independent of EPUB CSS/layout.

use serde::{Deserialize, Serialize};

use crate::document::{ChapterId, ChapterText, InvalidTextValue, TextOffset, TextRange};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "facet_typegen", derive(facet::Facet))]
pub struct ReaderChapterFormatting {
    pub chapter_id: ChapterId,
    pub spans: Vec<ReaderTextSpan>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "facet_typegen", derive(facet::Facet))]
pub struct ReaderTextSpan {
    /// Chapter-local in ReaderDocument; page-local in ReaderPage.
    pub range: TextRange,
    pub style: ReaderTextStyle,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "facet_typegen", derive(facet::Facet))]
#[repr(C)]
pub enum ReaderTextStyle {
    Heading(u8),
    Emphasis,
    Strong,
    ExternalLink(String),
}

impl ReaderChapterFormatting {
    pub fn validate(&self, chapter: &ChapterText) -> Result<(), InvalidTextValue> {
        if self.chapter_id != chapter.chapter_id {
            return Err(InvalidTextValue);
        }
        for span in &self.spans {
            if span.range.slice(&chapter.text)?.is_empty() {
                return Err(InvalidTextValue);
            }
            match &span.style {
                ReaderTextStyle::Heading(level) if !(1..=6).contains(level) => {
                    return Err(InvalidTextValue);
                }
                ReaderTextStyle::ExternalLink(target) if !website(target) => {
                    return Err(InvalidTextValue);
                }
                _ => {}
            }
        }
        Ok(())
    }

    /// Clip crossing spans without moving any canonical text or reading anchor.
    pub fn on_page(
        &self,
        chapter: &ChapterText,
        page: TextRange,
    ) -> Result<Vec<ReaderTextSpan>, InvalidTextValue> {
        self.validate(chapter)?;
        page.slice(&chapter.text)?;
        self.spans
            .iter()
            .filter_map(|span| {
                let start = span.range.start.max(page.start);
                let end = span.range.end.min(page.end);
                (start < end).then_some((span, start, end))
            })
            .map(|(span, start, end)| {
                Ok(ReaderTextSpan {
                    range: TextRange {
                        start: TextOffset::try_from(i64::from(start) - i64::from(page.start))?,
                        end: TextOffset::try_from(i64::from(end) - i64::from(page.start))?,
                    },
                    style: span.style.clone(),
                })
            })
            .collect()
    }
}

fn website(target: &str) -> bool {
    target.split_once("://").is_some_and(|(scheme, rest)| {
        (scheme.eq_ignore_ascii_case("http") || scheme.eq_ignore_ascii_case("https"))
            && !rest.is_empty()
            && !rest.starts_with('/')
            && !target.chars().any(|c| c.is_whitespace() || c.is_control())
    })
}
