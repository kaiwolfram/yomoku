//! In-memory EPUB extraction. No filesystem, network, analysis or durable IDs.

use std::{collections::BTreeMap, ops::Range};

use roxmltree::{Document, Node, ParsingOptions};

use crate::txt::SourceRuby;

mod archive;
mod content;

const OPF: &str = "http://www.idpf.org/2007/opf";
const DC: &str = "http://purl.org/dc/elements/1.1/";
const XHTML: &str = "http://www.w3.org/1999/xhtml";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EpubDocument {
    /// The import adapter supplies the filename when metadata has no title.
    pub title: Option<String>,
    /// First-reference spine order, including auxiliary items; repeated items and
    /// empty text-only items are omitted.
    pub chapters: Vec<EpubChapter>,
    /// Only referenced images and the cover, keyed by container-relative path.
    pub images: BTreeMap<String, EpubImage>,
    pub cover: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EpubChapter {
    pub path: String,
    pub text: String,
    pub ruby: Vec<SourceRuby>,
    pub spans: Vec<EpubSpan>,
    /// Each image occupies one U+FFFC scalar in canonical text, never its alt text.
    pub images: Vec<InlineImage>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EpubSpan {
    pub range: Range<usize>,
    pub kind: SpanKind,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SpanKind {
    Heading(u8),
    Emphasis,
    Strong,
    /// Only HTTP(S) website links can later become interactive.
    ExternalLink(String),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InlineImage {
    pub range: Range<usize>,
    pub path: String,
    pub alt: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EpubImage {
    pub media_type: String,
    pub bytes: Vec<u8>,
}

/// Operational parser budgets, not persisted product settings. Adapters may tune
/// them after measurement. All byte limits apply before allocating entry output.
#[derive(Clone, Copy, Debug)]
pub struct EpubLimits {
    pub source_bytes: usize,
    pub entries: usize,
    pub entry_bytes: usize,
    pub expanded_bytes: usize,
    pub xml_nodes: u32,
    /// Element nesting; capped at 64 even when a caller supplies a larger budget.
    pub depth: usize,
}

impl Default for EpubLimits {
    fn default() -> Self {
        Self {
            source_bytes: 256 * 1024 * 1024,
            entries: 20_000,
            entry_bytes: 32 * 1024 * 1024,
            expanded_bytes: 256 * 1024 * 1024,
            xml_nodes: 500_000,
            depth: 64,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EpubError {
    InvalidArchive,
    InvalidPackage,
    InvalidXml,
    InvalidPath,
    MissingResource,
    UnsupportedResource,
    EncryptedResource,
    LimitExceeded,
    EmptyContent,
}

impl std::fmt::Display for EpubError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::InvalidArchive => "invalid EPUB ZIP archive",
            Self::InvalidPackage => "invalid EPUB container, manifest or spine",
            Self::InvalidXml => "invalid or unsupported EPUB XML",
            Self::InvalidPath => "invalid EPUB resource path",
            Self::MissingResource => "missing EPUB resource",
            Self::UnsupportedResource => "unsupported EPUB resource type",
            Self::EncryptedResource => "encrypted EPUB content is unsupported",
            Self::LimitExceeded => "EPUB parser resource limit exceeded",
            Self::EmptyContent => "EPUB has no readable content",
        })
    }
}

impl std::error::Error for EpubError {}

type Result<T> = std::result::Result<T, EpubError>;

pub fn parse(bytes: &[u8]) -> Result<EpubDocument> {
    parse_with_limits(bytes, EpubLimits::default())
}

pub fn parse_with_limits(bytes: &[u8], limits: EpubLimits) -> Result<EpubDocument> {
    let mut archive = archive::Archive::new(bytes, limits)?;
    if archive.read("mimetype")? != b"application/epub+zip" {
        return Err(EpubError::InvalidPackage);
    }
    let container_text = xml_text(&archive.read("META-INF/container.xml")?)?;
    let container = xml(&container_text, limits)?;
    if !container.root_element().has_tag_name((
        "urn:oasis:names:tc:opendocument:xmlns:container",
        "container",
    )) {
        return Err(EpubError::InvalidPackage);
    }
    let package_path = container
        .descendants()
        .find(|node| {
            node.has_tag_name((
                "urn:oasis:names:tc:opendocument:xmlns:container",
                "rootfile",
            )) && node.attribute("media-type") == Some("application/oebps-package+xml")
        })
        .and_then(|node| node.attribute("full-path"))
        .ok_or(EpubError::InvalidPackage)?;
    let package_path = archive::resolve("", package_path)?;
    archive.load_encryption(limits)?;
    let package_text = xml_text(&archive.read(&package_path)?)?;
    let package = xml(&package_text, limits)?;
    let root = package.root_element();
    if !root.has_tag_name((OPF, "package")) {
        return Err(EpubError::InvalidPackage);
    }
    let metadata = child(root, "metadata")?;
    let title = metadata
        .children()
        .filter(|n| n.has_tag_name((DC, "title")))
        .map(plain_text)
        .find(|text| !text.is_empty());
    let manifest = child(root, "manifest")?;
    let mut items = BTreeMap::new();
    let mut manifest_paths = std::collections::BTreeSet::new();
    let mut image_types = BTreeMap::new();
    let mut cover = None;
    for node in manifest
        .children()
        .filter(|n| n.has_tag_name((OPF, "item")))
    {
        let id = required(node, "id")?;
        let media_type = required(node, "media-type")?;
        let href = required(node, "href")?;
        // Unused stylesheets, fonts and scripts are not fetched or interpreted.
        let path = if media_type == "application/xhtml+xml" || media_type.starts_with("image/") {
            archive::resolve(&package_path, href)?
        } else {
            href.to_owned()
        };
        if items.insert(id, (path.clone(), media_type)).is_some() {
            return Err(EpubError::InvalidPackage);
        }
        if !manifest_paths.insert(path.clone()) {
            return Err(EpubError::InvalidPackage);
        }
        if media_type.starts_with("image/") {
            image_types.insert(path.clone(), media_type.to_owned());
        }
        if node
            .attribute("properties")
            .is_some_and(|v| v.split_whitespace().any(|p| p == "cover-image"))
        {
            cover = Some(path);
        }
    }
    if cover.is_none() {
        if let Some(id) = metadata
            .children()
            .find(|n| n.has_tag_name((OPF, "meta")) && n.attribute("name") == Some("cover"))
            .and_then(|n| n.attribute("content"))
        {
            cover = Some(items.get(id).ok_or(EpubError::InvalidPackage)?.0.clone());
        }
    }
    let mut document = EpubDocument {
        title,
        chapters: Vec::new(),
        images: BTreeMap::new(),
        cover,
    };
    let mut spine_ids = std::collections::BTreeSet::new();
    for item in child(root, "spine")?
        .children()
        .filter(|n| n.has_tag_name((OPF, "itemref")))
    {
        let id = required(item, "idref")?;
        if !spine_ids.insert(id) {
            // Some books repeat their cover or other spine items. Keep the
            // first occurrence without duplicating chapters or analysis.
            continue;
        }
        let (path, media_type) = items.get(id).ok_or(EpubError::InvalidPackage)?;
        let chapter = match *media_type {
            "application/xhtml+xml" => {
                let text = xml_text(&archive.read(path)?)?;
                content::extract(path, &xml(&text, limits)?, limits)?
            }
            "image/svg+xml" => content::image_chapter(path),
            _ => return Err(EpubError::UnsupportedResource),
        };
        if !chapter.text.trim().is_empty() {
            document.chapters.push(chapter);
        }
    }
    if document.chapters.is_empty() {
        return Err(EpubError::EmptyContent);
    }
    let paths: std::collections::BTreeSet<_> = document
        .chapters
        .iter()
        .flat_map(|c| c.images.iter().map(|image| image.path.clone()))
        .chain(document.cover.iter().cloned())
        .collect();
    for path in paths {
        let media_type = image_types
            .get(&path)
            .ok_or(EpubError::MissingResource)?
            .clone();
        document.images.insert(
            path.clone(),
            EpubImage {
                media_type,
                bytes: archive.read(&path)?,
            },
        );
    }
    Ok(document)
}

fn child<'a, 'input>(node: Node<'a, 'input>, name: &str) -> Result<Node<'a, 'input>> {
    node.children()
        .find(|n| n.has_tag_name((OPF, name)))
        .ok_or(EpubError::InvalidPackage)
}

fn required<'a>(node: Node<'a, '_>, name: &str) -> Result<&'a str> {
    node.attribute(name)
        .filter(|v| !v.is_empty())
        .ok_or(EpubError::InvalidPackage)
}

fn plain_text(node: Node<'_, '_>) -> String {
    let text: String = node
        .descendants()
        .filter(|n| n.is_text())
        .filter_map(|n| n.text())
        .collect();
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn xml(text: &str, limits: EpubLimits) -> Result<Document<'_>> {
    // roxmltree's tokenizer recurses. Preflight with a nonrecursive tokenizer
    // before building the tree, not after untrusted nesting reaches that parser.
    let mut depth = 0usize;
    for token in xmlparser::Tokenizer::from(text) {
        use xmlparser::{ElementEnd, Token};
        match token.map_err(|_| EpubError::InvalidXml)? {
            Token::ElementStart { .. } => {
                depth += 1;
                if depth > limits.depth.min(64) {
                    return Err(EpubError::LimitExceeded);
                }
            }
            Token::ElementEnd {
                end: ElementEnd::Empty | ElementEnd::Close(..),
                ..
            } => {
                depth = depth.checked_sub(1).ok_or(EpubError::InvalidXml)?;
            }
            Token::Text { text } => validate_references(text.as_str())?,
            Token::Attribute { value, .. } => validate_references(value.as_str())?,
            // Entity definitions could inject nesting after the preflight and
            // silently alter source text. A plain external DOCTYPE is accepted.
            Token::EntityDeclaration { .. } => return Err(EpubError::InvalidXml),
            _ => {}
        }
    }
    // No external resolver: a DOCTYPE declaration never causes network/file I/O.
    Document::parse_with_options(
        text,
        ParsingOptions {
            allow_dtd: true,
            nodes_limit: limits.xml_nodes,
            ..ParsingOptions::default()
        },
    )
    .map_err(|_| EpubError::InvalidXml)
}

fn validate_references(text: &str) -> Result<()> {
    for reference in text.split('&').skip(1) {
        let name = reference.split_once(';').ok_or(EpubError::InvalidXml)?.0;
        if let Some(number) = name.strip_prefix('#') {
            let (digits, radix) = number
                .strip_prefix('x')
                .map_or((number, 10), |hex| (hex, 16));
            if digits.is_empty() || !digits.chars().all(|c| c.is_digit(radix)) {
                return Err(EpubError::InvalidXml);
            }
            let value = u32::from_str_radix(digits, radix).map_err(|_| EpubError::InvalidXml)?;
            if !matches!(value, 0x9 | 0xA | 0xD | 0x20..=0xD7FF | 0xE000..=0xFFFD | 0x10000..=0x10FFFF)
            {
                return Err(EpubError::InvalidXml);
            }
        }
    }
    Ok(())
}

fn xml_text(bytes: &[u8]) -> Result<String> {
    let (encoding, bytes) = if let Some(bytes) = bytes.strip_prefix(&[0xFF, 0xFE]) {
        ("utf-16le", bytes)
    } else if let Some(bytes) = bytes.strip_prefix(&[0xFE, 0xFF]) {
        ("utf-16be", bytes)
    } else if bytes.starts_with(&[0, b'<', 0, b'?']) {
        ("utf-16be", bytes)
    } else if bytes.starts_with(&[b'<', 0, b'?', 0]) {
        ("utf-16le", bytes)
    } else {
        (
            "utf-8",
            bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]).unwrap_or(bytes),
        )
    };
    let decoded = match encoding {
        "utf-16le" => {
            encoding_rs::UTF_16LE.decode_without_bom_handling_and_without_replacement(bytes)
        }
        "utf-16be" => {
            encoding_rs::UTF_16BE.decode_without_bom_handling_and_without_replacement(bytes)
        }
        _ => std::str::from_utf8(bytes)
            .ok()
            .map(std::borrow::Cow::Borrowed),
    }
    .ok_or(EpubError::InvalidXml)?;
    if let Some(Ok(xmlparser::Token::Declaration {
        encoding: Some(declared),
        ..
    })) = xmlparser::Tokenizer::from(decoded.as_ref()).next()
    {
        if !declared.as_str().eq_ignore_ascii_case(encoding)
            && !(encoding.starts_with("utf-16") && declared.as_str().eq_ignore_ascii_case("utf-16"))
        {
            return Err(EpubError::InvalidXml);
        }
    }
    Ok(decoded.into_owned())
}
