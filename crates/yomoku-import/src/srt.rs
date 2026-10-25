//! Tolerant SRT extraction from decoded text, without clocks, I/O or durable IDs.
//!
//! Callers validate the file type and decode original bytes before this boundary.
//! This is not a WebVTT or ASS parser. Encoding and original-byte identity belong
//! to the eventual file-import adapter, not to canonical subtitle text.

use std::ops::Range;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SrtDocument {
    /// LF within cues, one blank line between cues, no added trailing newline.
    pub text: String,
    /// Nonempty cue ranges in canonical UTF-8 bytes, excluding cue separators.
    /// All cues belong to one text unit; cues are not chapter boundaries.
    pub cues: Vec<Range<usize>>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EmptySrt;

impl std::fmt::Display for EmptySrt {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SRT must contain non-whitespace subtitle text")
    }
}

impl std::error::Error for EmptySrt {}

/// Recover missing counters, missing blank separators and dot/short timestamps.
/// Unrecognized metadata stays visible rather than consuming possible dialogue.
/// A numeric line is a counter only immediately before an identifiable time range.
pub fn parse(decoded: &str) -> Result<SrtDocument, EmptySrt> {
    let normalized = decoded
        .strip_prefix('\u{feff}')
        .unwrap_or(decoded)
        .replace("\r\n", "\n")
        .replace('\r', "\n");
    let lines: Vec<_> = normalized.split('\n').collect();
    let mut document = SrtDocument {
        text: String::new(),
        cues: Vec::new(),
    };
    let mut cue = Vec::new();
    for (index, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        let counter = ascii_digits(trimmed)
            && lines
                .get(index + 1)
                .is_some_and(|next| is_timing(next.trim()));
        if trimmed.is_empty() || counter || is_timing(trimmed) {
            finish_cue(&mut document, &mut cue);
        } else {
            cue.push(strip_formatting(line));
        }
    }
    finish_cue(&mut document, &mut cue);
    if document.cues.is_empty() {
        return Err(EmptySrt);
    }
    Ok(document)
}

fn finish_cue(document: &mut SrtDocument, lines: &mut Vec<String>) {
    let text = lines.join("\n");
    lines.clear();
    let text = text.trim_matches('\n');
    if text.trim().is_empty() {
        return;
    }
    if !document.cues.is_empty() {
        document.text.push_str("\n\n");
    }
    let start = document.text.len();
    document.text.push_str(text);
    document.cues.push(start..document.text.len());
}

fn ascii_digits(text: &str) -> bool {
    !text.is_empty() && text.bytes().all(|byte| byte.is_ascii_digit())
}

fn is_timing(line: &str) -> bool {
    let Some((start, end)) = line.split_once("-->") else {
        return false;
    };
    // Accept trailing positioning fields, but never strip arbitrary arrow text.
    timestamp(start.trim()) && end.split_whitespace().next().is_some_and(timestamp)
}

fn timestamp(text: &str) -> bool {
    let (whole, fraction) = text
        .split_once([',', '.'])
        .map_or((text, None), |(whole, fraction)| (whole, Some(fraction)));
    if fraction.is_some_and(|value| value.len() > 3 || !ascii_digits(value)) {
        return false;
    }
    let mut fields = whole.split(':');
    let Some(first) = fields.next() else {
        return false;
    };
    let Some(second) = fields.next() else {
        return false;
    };
    let third = fields.next();
    ascii_digits(first)
        && ascii_digits(second)
        && third.is_none_or(ascii_digits)
        && fields.next().is_none()
}

/// Strip common subtitle HTML formatting, retaining unknown/speaker markup and
/// malformed brackets. A single scan prevents quadratic work on repeated '<'.
fn strip_formatting(line: &str) -> String {
    let mut output = String::new();
    let mut opening = None;
    for (index, scalar) in line.char_indices() {
        if scalar == '<' {
            if let Some(start) = opening {
                output.push_str(&line[start..index]);
            }
            opening = Some(index);
        } else if scalar == '>' && opening.is_some() {
            let start = opening.take().unwrap();
            let body = line[start + 1..index].trim();
            let body = body.strip_prefix('/').unwrap_or(body).trim_start();
            let name = body.split([' ', '\t', '/']).next().unwrap_or_default();
            if name.eq_ignore_ascii_case("br") {
                output.push('\n');
            } else if ![
                "i", "b", "u", "s", "strike", "font", "span", "small", "big", "sub", "sup",
            ]
            .iter()
            .any(|tag| name.eq_ignore_ascii_case(tag))
            {
                output.push_str(&line[start..=index]);
            }
        } else if opening.is_none() {
            output.push(scalar);
        }
    }
    if let Some(start) = opening {
        output.push_str(&line[start..]);
    }
    decode_entities(&output)
}

/// Decode once, after tag removal, so escaped dialogue never becomes a tag.
fn decode_entities(text: &str) -> String {
    let mut output = String::new();
    let mut opening = None;
    for (index, scalar) in text.char_indices() {
        if scalar == '&' {
            if let Some(start) = opening {
                output.push_str(&text[start..index]);
            }
            opening = Some(index);
        } else if scalar == ';' && opening.is_some() {
            let start = opening.take().unwrap();
            let entity = &text[start + 1..index];
            let decoded = match entity {
                "amp" => Some('&'),
                "lt" => Some('<'),
                "gt" => Some('>'),
                "quot" => Some('"'),
                "apos" => Some('\''),
                "nbsp" => Some('\u{a0}'),
                _ => entity.strip_prefix('#').and_then(|number| {
                    let value = if let Some(hex) = number
                        .strip_prefix('x')
                        .or_else(|| number.strip_prefix('X'))
                    {
                        if !hex.bytes().all(|byte| byte.is_ascii_hexdigit()) {
                            return None;
                        }
                        u32::from_str_radix(hex, 16).ok()
                    } else {
                        if !ascii_digits(number) {
                            return None;
                        }
                        number.parse().ok()
                    }?;
                    // Invalid/control references remain visible, never replaced.
                    char::from_u32(value).filter(|scalar| !scalar.is_control())
                }),
            };
            if let Some(decoded) = decoded {
                output.push(decoded);
            } else {
                output.push_str(&text[start..=index]);
            }
        } else if opening.is_none() {
            output.push(scalar);
        }
    }
    if let Some(start) = opening {
        output.push_str(&text[start..]);
    }
    output
}
