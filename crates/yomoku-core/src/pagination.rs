//! Logical pages over immutable chapter text, independent of display dimensions.

use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use crate::document::{
    ChapterId, ChapterText, InvalidTextValue, SourceId, TextAnchor, TextOffset, TextRange,
};

/// Counts non-whitespace Unicode scalars, including punctuation and Latin text.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum PageLength {
    Characters250,
    #[default]
    Characters500,
    Characters750,
    Characters1000,
    Characters1500,
}

impl PageLength {
    pub const ALL: [Self; 5] = [
        Self::Characters250,
        Self::Characters500,
        Self::Characters750,
        Self::Characters1000,
        Self::Characters1500,
    ];

    pub const fn characters(self) -> usize {
        match self {
            Self::Characters250 => 250,
            Self::Characters500 => 500,
            Self::Characters750 => 750,
            Self::Characters1000 => 1000,
            Self::Characters1500 => 1500,
        }
    }
}

/// Durable reading location; reflow must not replace it with a page's start.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "facet_typegen", derive(facet::Facet))]
pub struct ReadingPosition {
    pub source_id: SourceId,
    pub chapter_id: ChapterId,
    pub offset: TextOffset,
}

impl ReadingPosition {
    pub fn validate(self, chapter: &ChapterText) -> Result<(), InvalidTextValue> {
        if self.source_id != chapter.source_id || self.chapter_id != chapter.chapter_id {
            return Err(InvalidTextValue);
        }
        self.offset.byte_index(&chapter.text)?;
        Ok(())
    }
}

/// Derived ranges only. Canonical text and the reading offset remain separate.
#[derive(Debug)]
pub struct LogicalPages<'a> {
    chapters: &'a [ChapterText],
    pages: Vec<TextAnchor>,
    length: PageLength,
}

#[derive(Debug)]
pub struct Reflow<'a> {
    pub pages: LogicalPages<'a>,
    pub position: ReadingPosition,
    pub page_index: usize,
}

impl<'a> LogicalPages<'a> {
    /// Input order is authoritative. Only supplied chapters create boundaries;
    /// titles, headings and blank lines never infer additional chapters.
    pub fn new(chapters: &'a [ChapterText], length: PageLength) -> Result<Self, InvalidTextValue> {
        let mut seen = HashSet::new();
        let mut pages = Vec::new();
        for chapter in chapters {
            if chapter.source_id != chapters[0].source_id || !seen.insert(chapter.chapter_id) {
                return Err(InvalidTextValue);
            }
            paginate_chapter(chapter, length, &mut pages)?;
        }
        Ok(Self {
            chapters,
            pages,
            length,
        })
    }

    pub fn pages(&self) -> &[TextAnchor] {
        &self.pages
    }

    pub fn length(&self) -> PageLength {
        self.length
    }

    /// A boundary belongs to the following page; chapter EOF to its final page.
    pub fn page_at(&self, position: ReadingPosition) -> Result<usize, InvalidTextValue> {
        let chapter = self
            .chapters
            .iter()
            .find(|chapter| {
                chapter.source_id == position.source_id && chapter.chapter_id == position.chapter_id
            })
            .ok_or(InvalidTextValue)?;
        let index = position.offset.byte_index(&chapter.text)?;
        self.pages
            .iter()
            .position(|page| {
                page.chapter_id == position.chapter_id
                    && page.range.start <= position.offset
                    && (position.offset < page.range.end
                        || (index == chapter.text.len() && position.offset == page.range.end))
            })
            .ok_or(InvalidTextValue)
    }

    pub fn reflow(
        &self,
        length: PageLength,
        position: ReadingPosition,
    ) -> Result<Reflow<'a>, InvalidTextValue> {
        // Validate against the original layout before deriving a replacement.
        self.page_at(position)?;
        let pages = Self::new(self.chapters, length)?;
        let page_index = pages.page_at(position)?;
        Ok(Reflow {
            pages,
            position,
            page_index,
        })
    }
}

fn paginate_chapter(
    chapter: &ChapterText,
    length: PageLength,
    pages: &mut Vec<TextAnchor>,
) -> Result<(), InvalidTextValue> {
    let text = &chapter.text;
    let target = length.characters();
    let first_page = pages.len();
    let mut start = 0;
    let mut line_start = 0;
    let mut count = 0;
    for paragraph in text.split_inclusive('\n') {
        let paragraph_count = paragraph.chars().filter(|c| !c.is_whitespace()).count();
        // Prefer the next hard-line paragraph boundary. If that would exceed
        // twice the preset, use the first following sentence at the threshold.
        if count + paragraph_count > target * 2 {
            let mut chars = paragraph.char_indices().peekable();
            while let Some((index, scalar)) = chars.next() {
                count += usize::from(!scalar.is_whitespace());
                let sentence_end = matches!(scalar, '。' | '！' | '？' | '!' | '?')
                    || (scalar == '.'
                        && chars.peek().is_none_or(|(_, next)| {
                            next.is_whitespace() || is_sentence_tail(*next)
                        }));
                if count >= target && sentence_end {
                    let mut end = line_start + index + scalar.len_utf8();
                    // Keep punctuation runs and closing quotes with the sentence.
                    while let Some(&(index, scalar)) = chars.peek() {
                        if !is_sentence_tail(scalar) {
                            break;
                        }
                        chars.next();
                        end = line_start + index + scalar.len_utf8();
                    }
                    push_page(chapter, start, end, pages)?;
                    start = end;
                    count = 0;
                }
            }
        } else {
            count += paragraph_count;
        }
        line_start += paragraph.len();
        if count >= target {
            push_page(chapter, start, line_start, pages)?;
            start = line_start;
            count = 0;
        }
    }
    if start < text.len() || pages.len() == first_page {
        if count == 0 && pages.len() > first_page {
            // A trailing newline/blank block must not create a whitespace page.
            pages.last_mut().unwrap().range.end = TextOffset::at(text, text.len())?;
        } else {
            push_page(chapter, start, text.len(), pages)?;
        }
    }
    Ok(())
}

fn push_page(
    chapter: &ChapterText,
    start: usize,
    end: usize,
    pages: &mut Vec<TextAnchor>,
) -> Result<(), InvalidTextValue> {
    pages.push(TextAnchor {
        source_id: chapter.source_id,
        chapter_id: chapter.chapter_id,
        range: TextRange {
            start: TextOffset::at(&chapter.text, start)?,
            end: TextOffset::at(&chapter.text, end)?,
        },
    });
    Ok(())
}

fn is_sentence_tail(scalar: char) -> bool {
    matches!(
        scalar,
        '。' | '！'
            | '？'
            | '!'
            | '?'
            | '.'
            | '」'
            | '』'
            | '）'
            | ')'
            | '］'
            | ']'
            | '｝'
            | '}'
            | '】'
            | '〉'
            | '》'
            | '"'
            | '\''
            | '”'
            | '’'
    )
}
