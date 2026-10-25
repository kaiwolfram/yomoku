//! Strict UTF-8/Shift-JIS TXT decoding and common Aozora markup.

use std::{borrow::Cow, ops::Range};

use encoding_rs::SHIFT_JIS;

mod aozora;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TextEncoding {
    Utf8,
    ShiftJis,
}

/// Ruby refers to the base text in `TxtDocument::text`, never source-file bytes.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SourceRuby {
    pub range: Range<usize>,
    pub reading: String,
}

/// One canonical text unit: headings and page directives never create chapters.
/// The filename supplies the title at the storage boundary.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TxtDocument {
    pub encoding: TextEncoding,
    pub text: String,
    pub ruby: Vec<SourceRuby>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TxtError {
    /// Both supported encodings are valid but produce different text.
    EncodingRequired,
    InvalidEncoding(TextEncoding),
    UnsupportedEncoding,
    Undecodable,
    EmptyText,
}

impl std::fmt::Display for TxtError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::EncodingRequired => "select UTF-8 or Shift-JIS for ambiguous TXT bytes",
            Self::InvalidEncoding(TextEncoding::Utf8) => "TXT bytes are not valid UTF-8",
            Self::InvalidEncoding(TextEncoding::ShiftJis) => "TXT bytes are not valid Shift-JIS",
            Self::UnsupportedEncoding => "TXT has an unsupported UTF-16/UTF-32 byte order mark",
            Self::Undecodable => "TXT bytes are neither valid UTF-8 nor valid Shift-JIS",
            Self::EmptyText => "TXT must contain non-whitespace content",
        })
    }
}

impl std::error::Error for TxtError {}

/// `None` detects conservatively; `Some` resolves uncertainty without replacing
/// malformed bytes. Only a leading UTF-8 BOM is removed; all hard breaks and
/// Unicode scalars outside recognized markup remain unchanged.
pub fn parse(bytes: &[u8], selection: Option<TextEncoding>) -> Result<TxtDocument, TxtError> {
    let (encoding, decoded) = decode(bytes, selection)?;
    let (text, ruby) = aozora::parse(&decoded);
    if text.trim().is_empty() {
        return Err(TxtError::EmptyText);
    }
    Ok(TxtDocument {
        encoding,
        text,
        ruby,
    })
}

/// Decode a supported text file without applying TXT/Aozora markup rules.
/// SRT adapters can reuse the same conservative encoding choice and strict errors.
pub fn decode(
    bytes: &[u8],
    selection: Option<TextEncoding>,
) -> Result<(TextEncoding, Cow<'_, str>), TxtError> {
    if bytes.starts_with(&[0xFF, 0xFE])
        || bytes.starts_with(&[0xFE, 0xFF])
        || bytes.starts_with(&[0, 0, 0xFE, 0xFF])
    {
        return Err(TxtError::UnsupportedEncoding);
    }
    let utf8_bom = bytes.starts_with(&[0xEF, 0xBB, 0xBF]);
    let selection = selection.or(utf8_bom.then_some(TextEncoding::Utf8));
    let utf8_bytes = if utf8_bom { &bytes[3..] } else { bytes };
    let utf8 = || std::str::from_utf8(utf8_bytes).ok().map(Cow::Borrowed);
    let shift_jis = || SHIFT_JIS.decode_without_bom_handling_and_without_replacement(bytes);
    if let Some(encoding) = selection {
        let text = match encoding {
            TextEncoding::Utf8 => utf8(),
            TextEncoding::ShiftJis => shift_jis(),
        };
        return text
            .map(|text| (encoding, text))
            .ok_or(TxtError::InvalidEncoding(encoding));
    }
    match (utf8(), shift_jis()) {
        (Some(text), None) => Ok((TextEncoding::Utf8, text)),
        (None, Some(text)) => Ok((TextEncoding::ShiftJis, text)),
        (Some(utf8), Some(shift_jis)) if utf8 == shift_jis => Ok((TextEncoding::Utf8, utf8)),
        (Some(_), Some(_)) => Err(TxtError::EncodingRequired),
        (None, None) => Err(TxtError::Undecodable),
    }
}
