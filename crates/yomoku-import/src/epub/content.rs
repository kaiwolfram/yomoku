use roxmltree::{Document, Node};

use super::{
    archive::resolve, plain_text, EpubChapter, EpubError, EpubLimits, EpubSpan, InlineImage,
    Result, SourceRuby, SpanKind, XHTML,
};

pub(super) fn extract(
    path: &str,
    document: &Document<'_>,
    limits: EpubLimits,
) -> Result<EpubChapter> {
    if !document.root_element().has_tag_name((XHTML, "html")) {
        return Err(EpubError::InvalidXml);
    }
    let body = document
        .root_element()
        .children()
        .find(|n| n.has_tag_name((XHTML, "body")))
        .ok_or(EpubError::InvalidXml)?;
    let mut output = Output {
        chapter: EpubChapter {
            path: path.to_owned(),
            text: String::new(),
            ruby: Vec::new(),
            spans: Vec::new(),
            images: Vec::new(),
        },
        breaks: 0,
        space: false,
        limits,
    };
    output.walk(body, false)?;
    output
        .chapter
        .spans
        .sort_by_key(|span| (span.range.start, span.range.end));
    output
        .chapter
        .ruby
        .sort_by_key(|ruby| (ruby.range.start, ruby.range.end));
    Ok(output.chapter)
}

pub(super) fn image_chapter(path: &str) -> EpubChapter {
    EpubChapter {
        path: path.to_owned(),
        text: "\u{fffc}".to_owned(),
        ruby: Vec::new(),
        spans: Vec::new(),
        images: vec![InlineImage {
            range: 0..3,
            path: path.to_owned(),
            alt: String::new(),
        }],
    }
}

struct Output {
    chapter: EpubChapter,
    breaks: usize,
    space: bool,
    limits: EpubLimits,
}

impl Output {
    fn flush(&mut self) {
        if !self.chapter.text.is_empty() {
            if self.breaks > 0 {
                let trailing = self
                    .chapter
                    .text
                    .chars()
                    .rev()
                    .take_while(|c| *c == '\n')
                    .count();
                for _ in trailing..self.breaks {
                    self.chapter.text.push('\n');
                }
            } else if self.space && !self.chapter.text.ends_with([' ', '\n', '\t']) {
                self.chapter.text.push(' ');
            }
        }
        self.breaks = 0;
        self.space = false;
    }

    fn boundary(&mut self) {
        self.breaks = 2;
        self.space = false;
    }

    fn text(&mut self, text: &str, pre: bool) {
        if pre {
            self.flush();
            self.chapter.text.push_str(text);
            return;
        }
        for scalar in text.chars() {
            if matches!(scalar, ' ' | '\t' | '\n' | '\r' | '\u{c}') {
                self.space = true;
            } else {
                self.flush();
                self.chapter.text.push(scalar);
            }
        }
    }

    fn walk(&mut self, node: Node<'_, '_>, pre: bool) -> Result<()> {
        if node.is_text() {
            self.text(node.text().unwrap_or_default(), pre);
            return self.check_size();
        }
        if !node.is_element() {
            return Ok(());
        }
        let html = node.tag_name().namespace() == Some(XHTML);
        let name = node.tag_name().name();
        if matches!(name, "script" | "style")
            || (html
                && (matches!(name, "head" | "rt" | "rp" | "rtc") || node.has_attribute("hidden")))
        {
            return Ok(());
        }
        let block = html
            && matches!(
                name,
                "address"
                    | "article"
                    | "aside"
                    | "blockquote"
                    | "div"
                    | "dl"
                    | "dt"
                    | "dd"
                    | "figure"
                    | "figcaption"
                    | "footer"
                    | "header"
                    | "h1"
                    | "h2"
                    | "h3"
                    | "h4"
                    | "h5"
                    | "h6"
                    | "hr"
                    | "li"
                    | "main"
                    | "nav"
                    | "ol"
                    | "p"
                    | "pre"
                    | "section"
                    | "table"
                    | "tr"
                    | "ul"
            );
        if block {
            self.boundary();
        }
        let start = self.chapter.text.len();
        if html && name == "br" {
            self.flush();
            self.chapter.text.push('\n');
        } else if (html && name == "img")
            || node.has_tag_name(("http://www.w3.org/2000/svg", "image"))
        {
            let reference = node
                .attribute("src")
                .or_else(|| node.attribute("href"))
                .or_else(|| node.attribute(("http://www.w3.org/1999/xlink", "href")))
                .ok_or(EpubError::MissingResource)?;
            let path = resolve(&self.chapter.path, reference)?;
            self.flush();
            let start = self.chapter.text.len();
            self.chapter.text.push('\u{fffc}');
            self.chapter.images.push(InlineImage {
                range: start..self.chapter.text.len(),
                path,
                alt: node.attribute("alt").unwrap_or_default().to_owned(),
            });
        } else {
            for child in node.children() {
                self.walk(child, pre || (html && name == "pre"))?;
            }
        }
        // Deferred whitespace keeps all ranges stable: no later trimming or
        // insertion into canonical text, even for nested inline formatting.
        let raw = &self.chapter.text[start..];
        let left = raw.len()
            - raw
                .trim_start_matches(|c: char| c.is_ascii_whitespace())
                .len();
        let end = start
            + raw
                .trim_end_matches(|c: char| c.is_ascii_whitespace())
                .len();
        let range = start + left..end;
        if html && !range.is_empty() {
            let kind = match name {
                "h1" => Some(SpanKind::Heading(1)),
                "h2" => Some(SpanKind::Heading(2)),
                "h3" => Some(SpanKind::Heading(3)),
                "h4" => Some(SpanKind::Heading(4)),
                "h5" => Some(SpanKind::Heading(5)),
                "h6" => Some(SpanKind::Heading(6)),
                "em" | "i" => Some(SpanKind::Emphasis),
                "strong" | "b" => Some(SpanKind::Strong),
                "a" => node.attribute("href").map(|href| {
                    if website(href) {
                        SpanKind::ExternalLink(href.to_owned())
                    } else {
                        SpanKind::InertLink(href.to_owned())
                    }
                }),
                _ => None,
            };
            if let Some(kind) = kind {
                self.chapter.spans.push(EpubSpan {
                    range: range.clone(),
                    kind,
                });
            }
            if name == "ruby" {
                let reading: String = node
                    .descendants()
                    .filter(|n| n.has_tag_name((XHTML, "rt")))
                    .filter(|n| {
                        !n.ancestors()
                            .take_while(|ancestor| *ancestor != node)
                            .any(|a| a.has_tag_name((XHTML, "rtc")))
                    })
                    .map(plain_text)
                    .collect();
                if !reading.is_empty() {
                    self.chapter.ruby.push(SourceRuby { range, reading });
                }
            }
        }
        if block {
            self.boundary();
        }
        if html && matches!(name, "td" | "th") {
            self.space = true;
        }
        self.check_size()
    }

    fn check_size(&self) -> Result<()> {
        if self.chapter.text.len() > self.limits.entry_bytes {
            Err(EpubError::LimitExceeded)
        } else {
            Ok(())
        }
    }
}

fn website(href: &str) -> bool {
    let Some((scheme, rest)) = href.split_once("://") else {
        return false;
    };
    (scheme.eq_ignore_ascii_case("https") || scheme.eq_ignore_ascii_case("http"))
        && !rest.is_empty()
        && !rest.starts_with('/')
        && !href.chars().any(|c| c.is_whitespace() || c.is_control())
}
