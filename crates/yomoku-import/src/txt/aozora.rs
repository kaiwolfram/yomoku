pub(super) fn parse(source: &str) -> String {
    let mut text = String::with_capacity(source.len());
    let mut remaining = source;
    // A second reading must never overlap an already annotated base.
    let mut ruby_floor = 0;
    while let Some(ch) = remaining.chars().next() {
        if ch == '｜' {
            let after_marker = &remaining[ch.len_utf8()..];
            if let Some(open) = after_marker.find(['《', '｜', '》', '［', '\r', '\n']) {
                let base = &after_marker[..open];
                let suffix = &after_marker[open..];
                if !base.trim().is_empty() && suffix.starts_with('《') {
                    if let Some((_, consumed)) = reading(suffix) {
                        text.push_str(base);
                        ruby_floor = text.len();
                        remaining = &suffix[consumed..];
                        continue;
                    }
                }
            }
        } else if ch == '《' {
            if let Some((_, consumed)) = reading(remaining) {
                if text[ruby_floor..].chars().next_back().is_some_and(is_kanji) {
                    ruby_floor = text.len();
                    remaining = &remaining[consumed..];
                    continue;
                }
            }
            // Unrecognized/malformed quotation is literal as a unit. In
            // particular, do not strip apparent directives inside quoted text.
            let end = remaining
                .char_indices()
                .skip(1)
                .find_map(|(index, ch)| match ch {
                    '》' => Some(index + ch.len_utf8()),
                    '\r' | '\n' => Some(index),
                    _ => None,
                })
                .unwrap_or(remaining.len());
            text.push_str(&remaining[..end]);
            ruby_floor = text.len();
            remaining = &remaining[end..];
            continue;
        } else if let Some(after_prefix) = remaining.strip_prefix("［＃") {
            let mut depth = 1;
            let (end, complete) = after_prefix
                .char_indices()
                .find_map(|(index, ch)| {
                    match ch {
                        '\r' | '\n' => return Some((index, false)),
                        '［' => depth += 1,
                        '］' => depth -= 1,
                        _ => {}
                    }
                    (depth == 0).then_some((index + ch.len_utf8(), true))
                })
                .unwrap_or((after_prefix.len(), false));
            let consumed = "［＃".len() + end;
            // Gaiji descriptions may contain the only available spelling.
            // Consume unknown/nested/malformed notes as a unit, preserving them.
            if !complete
                || text.ends_with('※')
                || !is_typographic(&after_prefix[..end - '］'.len_utf8()])
            {
                text.push_str(&remaining[..consumed]);
            }
            ruby_floor = text.len();
            remaining = &remaining[consumed..];
            continue;
        }
        text.push(ch);
        remaining = &remaining[ch.len_utf8()..];
    }
    text
}

/// Returns the reading and total source bytes through the closing delimiter.
/// A malformed delimiter cannot consume another line or a subsequent ruby.
fn reading(source: &str) -> Option<(&str, usize)> {
    let after_open = source.strip_prefix('《')?;
    let end = after_open.find(['》', '《', '｜', '［', '\r', '\n'])?;
    let reading = &after_open[..end];
    (after_open[end..].starts_with('》') && !reading.trim().is_empty())
        .then_some((reading, '《'.len_utf8() + end + '》'.len_utf8()))
}

fn is_kanji(ch: char) -> bool {
    matches!(ch,
        '仝' | '々' | '〆' | '〇' | 'ヶ'
        | '\u{3400}'..='\u{4DBF}' | '\u{4E00}'..='\u{9FFF}'
        | '\u{F900}'..='\u{FAFF}' | '\u{20000}'..='\u{2FA1F}'
        | '\u{30000}'..='\u{3347F}'
    )
}

fn is_typographic(note: &str) -> bool {
    if matches!(note, "改ページ" | "改丁" | "改見開き" | "改段" | "地付き") {
        return true;
    }
    let style = note
        .strip_prefix("ここから")
        .or_else(|| note.strip_prefix("ここで"))
        .unwrap_or(note);
    let style = style.strip_suffix("終わり").unwrap_or(style);
    if is_style(style) || numbered_layout(style) {
        return true;
    }
    // Forward-reference annotations affect the preceding visible base only.
    note.strip_prefix('「')
        .and_then(|note| note.split_once("」は"))
        .is_some_and(|(base, style)| !base.is_empty() && is_style(style))
}

fn is_style(style: &str) -> bool {
    matches!(
        style,
        "太字"
            | "斜体"
            | "傍点"
            | "白ゴマ傍点"
            | "丸傍点"
            | "白丸傍点"
            | "黒三角傍点"
            | "白三角傍点"
            | "二重丸傍点"
            | "蛇の目傍点"
            | "ばつ傍点"
            | "傍線"
            | "二重傍線"
            | "鎖線"
            | "破線"
            | "波線"
            | "大見出し"
            | "中見出し"
            | "小見出し"
            | "同行大見出し"
            | "同行中見出し"
            | "同行小見出し"
            | "窓大見出し"
            | "窓中見出し"
            | "窓小見出し"
            | "字下げ"
            | "地付き"
            | "地上げ"
    )
}

fn numbered_layout(style: &str) -> bool {
    let number = style.strip_suffix("字下げ").or_else(|| {
        style
            .strip_prefix("地から")
            .and_then(|style| style.strip_suffix("字上げ"))
    });
    number.is_some_and(|number| {
        !number.is_empty()
            && number
                .chars()
                .all(|ch| ch.is_ascii_digit() || ('０'..='９').contains(&ch))
    })
}
