//! Source-preferred ruby from the retained effective page, without re-resolution.

use super::EffectiveReaderSession;
use crate::{
    document::InvalidTextValue,
    reader::{ReaderAnalysis, ReaderPage, ReaderTokenKey},
    reader_ruby::{source_preferred_ruby, valid_page_tokens, validate_ruby, ReaderRuby, RubyToken},
};

impl EffectiveReaderSession {
    /// Project only the current page's exact effective readings. Source ruby
    /// remains usable before load, on failure/Missing B and after invalidation.
    /// Manual parts without exact analyzer metadata never borrow old B readings
    /// or user dictionary readings. No chapter key is cloned or sent to the UI.
    pub fn furigana(
        &self,
        page: &ReaderPage,
        analysis: ReaderAnalysis,
    ) -> Result<Vec<ReaderRuby>, InvalidTextValue> {
        validate_ruby(&page.text, &page.source_ruby)?;
        let base = ReaderTokenKey {
            document_id: page.document_id,
            page: page.anchor,
            analysis,
        };
        let Some(effective) = self.page().filter(|effective| {
            effective.identity.base == base
                && valid_page_tokens(
                    page,
                    effective.tokens.iter().map(|token| RubyToken {
                        index: token.segment_index,
                        anchor: token.anchor,
                        display_range: token.display_range,
                        surface: &token.surface,
                        reading: token.reading.as_deref(),
                    }),
                )
        }) else {
            return Ok(page.source_ruby.clone());
        };
        Ok(source_preferred_ruby(
            &page.source_ruby,
            effective.tokens.iter().filter_map(|token| {
                token.reading.as_ref().map(|reading| ReaderRuby {
                    range: token.display_range,
                    reading: reading.clone(),
                })
            }),
        ))
    }
}
