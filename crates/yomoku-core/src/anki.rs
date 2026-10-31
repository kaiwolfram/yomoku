//! Pure Anki note content and retained export commands. Provider access belongs
//! outside core; neither preparation nor export changes vocabulary/history.

use crate::{
    document::{InvalidTextValue, TextRange},
    vocabulary::Surface,
};

mod export;
mod projection;
pub use export::*;

/// One forward card, with the exact written surface as its first field.
/// These are original templates, not copied Yomitan assets. Android must verify
/// a stored model's complete layout before reusing it; never overwrite a user's
/// unrelated model based only on its name.
pub const MODEL_NAME: &str = "Yomoku v1";
pub const FIELD_NAMES: [&str; 4] = ["Surface", "Reading", "Meanings", "Sentence"];
pub const CARD_NAME: &str = "Japanese word";
pub const QUESTION_FORMAT: &str = r#"<div class="yomoku-surface">{{Surface}}</div>"#;
pub const ANSWER_FORMAT: &str = r#"{{FrontSide}}
<hr id="answer">
{{#Reading}}<div class="yomoku-reading">{{Reading}}</div>{{/Reading}}
{{#Meanings}}<div class="yomoku-meanings">{{Meanings}}</div>{{/Meanings}}
<div class="yomoku-sentence">{{Sentence}}</div>"#;
pub const CSS: &str = r#".card { font-family: sans-serif; font-size: 20px; text-align: left; }
.yomoku-surface { font-size: 1.8em; text-align: center; white-space: pre-wrap; }
.yomoku-reading, .yomoku-meanings, .yomoku-sentence { white-space: pre-wrap; }
.yomoku-sentence { margin-top: 1em; }
.yomoku-target { font-weight: bold; text-decoration: underline; }"#;

/// Immutable, HTML-escaped fields in FIELD_NAMES order. Keep the unescaped
/// surface separately for exact identity; HTML rendering is not an identity key.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AnkiNote {
    surface: Surface,
    fields: [String; 4],
}

impl AnkiNote {
    /// `sentence` is literal canonical text, with a sentence-local UTF-8 target
    /// range. The caller resolves sentence boundaries and applicable dictionary
    /// readings/English meanings. No lemma substitution, normalization, global
    /// string replacement, dictionary lookup or fallback text occurs here.
    /// Missing readings/meanings remain empty rather than being fabricated.
    pub fn new(
        surface: &Surface,
        reading: &str,
        meanings: &[&str],
        sentence: &str,
        target: TextRange,
    ) -> Result<Self, InvalidTextValue> {
        if target.slice(sentence)? != surface.as_str() {
            return Err(InvalidTextValue);
        }
        let start = target.start.byte_index(sentence)?;
        let end = target.end.byte_index(sentence)?;
        let sentence = format!(
            "{}<strong class=\"yomoku-target\">{}</strong>{}",
            escape(&sentence[..start]),
            escape(surface.as_str()),
            escape(&sentence[end..]),
        );
        let meanings = if meanings.is_empty() {
            String::new()
        } else {
            let mut html = String::from("<ol>");
            for meaning in meanings {
                html.push_str("<li>");
                html.push_str(&escape(meaning));
                html.push_str("</li>");
            }
            html.push_str("</ol>");
            html
        };
        Ok(Self {
            surface: surface.clone(),
            fields: [
                escape(surface.as_str()),
                escape(reading),
                meanings,
                sentence,
            ],
        })
    }

    pub fn surface(&self) -> &Surface {
        &self.surface
    }

    pub fn fields(&self) -> &[String; 4] {
        &self.fields
    }
}

fn escape(text: &str) -> String {
    let mut html = String::new();
    for scalar in text.chars() {
        match scalar {
            '&' => html.push_str("&amp;"),
            '<' => html.push_str("&lt;"),
            '>' => html.push_str("&gt;"),
            '"' => html.push_str("&quot;"),
            '\'' => html.push_str("&#39;"),
            // Keep literal Anki media syntax inert and provider field separators
            // inside their field, without altering the retained surface identity.
            '[' => html.push_str("&#91;"),
            ']' => html.push_str("&#93;"),
            '\u{001f}' => html.push_str("&#31;"),
            other => html.push(other),
        }
    }
    html
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::TextOffset;

    fn range(start: i64, end: i64) -> TextRange {
        TextRange {
            start: TextOffset::try_from(start).unwrap(),
            end: TextOffset::try_from(end).unwrap(),
        }
    }

    #[test]
    fn repeated_surface_highlights_only_the_selected_occurrence() {
        let surface = Surface::new("猫").unwrap();
        let note = AnkiNote::new(
            &surface,
            "ねこ",
            &["cat", "feline"],
            "猫と猫。",
            range(6, 9),
        )
        .unwrap();
        assert_eq!(note.surface(), &surface);
        assert_eq!(
            note.fields(),
            &[
                "猫",
                "ねこ",
                "<ol><li>cat</li><li>feline</li></ol>",
                "猫と<strong class=\"yomoku-target\">猫</strong>。",
            ]
        );
    }

    #[test]
    fn markup_entities_media_and_field_separators_are_literal_content() {
        let text = "<&\"'>[sound:猫.mp3]\u{001f}";
        let surface = Surface::new(text).unwrap();
        let note =
            AnkiNote::new(&surface, text, &[text], text, range(0, text.len() as i64)).unwrap();
        let escaped = "&lt;&amp;&quot;&#39;&gt;&#91;sound:猫.mp3&#93;&#31;";
        assert_eq!(note.surface().as_str(), text);
        assert_eq!(note.fields()[0], escaped);
        assert_eq!(note.fields()[1], escaped);
        assert_eq!(note.fields()[2], format!("<ol><li>{escaped}</li></ol>"));
        assert_eq!(
            note.fields()[3],
            format!("<strong class=\"yomoku-target\">{escaped}</strong>")
        );
        assert!(note
            .fields()
            .iter()
            .all(|field| !field.contains('\u{001f}')));
    }

    #[test]
    fn sentence_context_is_escaped_without_moving_the_target() {
        let surface = Surface::new("猫").unwrap();
        let note = AnkiNote::new(&surface, "", &[], "<猫>&猫", range(1, 4)).unwrap();
        assert_eq!(
            note.fields()[3],
            "&lt;<strong class=\"yomoku-target\">猫</strong>&gt;&amp;猫"
        );
        assert_eq!(note.fields()[1], "");
        assert_eq!(note.fields()[2], "");
    }

    #[test]
    fn exact_unicode_whitespace_inflection_and_missing_definitions_are_preserved() {
        for text in [
            "𠮷",
            "e\u{301}",
            "é",
            "ｶﾞ",
            "ガ",
            "食べた",
            " 猫 ",
            "猫\n\t犬",
            "猫\0",
        ] {
            let surface = Surface::new(text).unwrap();
            let note = AnkiNote::new(&surface, "", &[], text, range(0, text.len() as i64)).unwrap();
            assert_eq!(note.surface().as_str(), text);
            assert_eq!(note.fields()[0], text);
            assert_eq!(note.fields()[1], "");
            assert_eq!(note.fields()[2], "");
        }
    }

    #[test]
    fn invalid_stale_non_scalar_and_non_exact_targets_are_rejected() {
        let surface = Surface::new("猫").unwrap();
        for target in [
            range(0, 0),
            range(3, 0),
            range(1, 3),
            range(0, 2),
            range(0, 99),
            range(99, 100),
        ] {
            assert_eq!(
                AnkiNote::new(&surface, "", &[], "猫。", target),
                Err(InvalidTextValue)
            );
        }
        for (wanted, sentence) in [
            ("食べる", "食べた"),
            ("ガ", "ｶﾞ"),
            ("é", "e\u{301}"),
            ("猫", "犬"),
        ] {
            assert_eq!(
                AnkiNote::new(
                    &Surface::new(wanted).unwrap(),
                    "",
                    &[],
                    sentence,
                    range(0, sentence.len() as i64)
                ),
                Err(InvalidTextValue)
            );
        }
    }
}
