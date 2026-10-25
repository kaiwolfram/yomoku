use std::{
    collections::{BTreeMap, BTreeSet},
    io::{Cursor, Read},
};

use super::{xml, xml_text, EpubError, EpubLimits, Result};

pub(super) struct Archive<'a> {
    zip: zip::ZipArchive<Cursor<&'a [u8]>>,
    paths: BTreeMap<String, usize>,
    encrypted: BTreeSet<String>,
    limits: EpubLimits,
    expanded: usize,
}

impl<'a> Archive<'a> {
    pub fn new(bytes: &'a [u8], limits: EpubLimits) -> Result<Self> {
        if bytes.len() > limits.source_bytes {
            return Err(EpubError::LimitExceeded);
        }
        let mut zip =
            zip::ZipArchive::new(Cursor::new(bytes)).map_err(|_| EpubError::InvalidArchive)?;
        if zip.len() > limits.entries {
            return Err(EpubError::LimitExceeded);
        }
        // zip-rs indexes duplicate raw names with last-wins semantics. Reject
        // these before trusting that index, rather than silently selecting text.
        let mut cursor = usize::try_from(zip.central_directory_start())
            .map_err(|_| EpubError::InvalidArchive)?;
        let mut names = BTreeSet::new();
        while bytes.get(cursor..cursor.saturating_add(4)) == Some(b"PK\x01\x02") {
            let header = bytes
                .get(cursor..cursor.saturating_add(46))
                .ok_or(EpubError::InvalidArchive)?;
            let field = |at| u16::from_le_bytes([header[at], header[at + 1]]) as usize;
            let start = cursor + 46;
            let end = start
                .checked_add(field(28))
                .ok_or(EpubError::InvalidArchive)?;
            let name = bytes.get(start..end).ok_or(EpubError::InvalidArchive)?;
            if !names.insert(name) {
                return Err(EpubError::InvalidArchive);
            }
            if names.len() > limits.entries {
                return Err(EpubError::LimitExceeded);
            }
            cursor = end
                .checked_add(field(30))
                .and_then(|v| v.checked_add(field(32)))
                .ok_or(EpubError::InvalidArchive)?;
        }
        if names.len() != zip.len() {
            return Err(EpubError::InvalidArchive);
        }
        let mut paths = BTreeMap::new();
        for index in 0..zip.len() {
            let file = zip
                .by_index_raw(index)
                .map_err(|_| EpubError::InvalidArchive)?;
            let raw = std::str::from_utf8(file.name_raw()).map_err(|_| EpubError::InvalidPath)?;
            let path = normalize(raw)?;
            if file.is_symlink() {
                return Err(EpubError::InvalidArchive);
            }
            if file.is_dir() {
                continue;
            }
            if paths.insert(path, index).is_some() {
                return Err(EpubError::InvalidArchive);
            }
        }
        Ok(Self {
            zip,
            paths,
            encrypted: BTreeSet::new(),
            limits,
            expanded: 0,
        })
    }

    pub fn read(&mut self, path: &str) -> Result<Vec<u8>> {
        if self.encrypted.contains(path) {
            return Err(EpubError::EncryptedResource);
        }
        let index = *self.paths.get(path).ok_or(EpubError::MissingResource)?;
        if self
            .zip
            .by_index_raw(index)
            .map_err(|_| EpubError::InvalidArchive)?
            .encrypted()
        {
            return Err(EpubError::EncryptedResource);
        }
        let mut file = self
            .zip
            .by_index(index)
            .map_err(|_| EpubError::InvalidArchive)?;
        let size = usize::try_from(file.size()).map_err(|_| EpubError::LimitExceeded)?;
        let remaining = self.limits.expanded_bytes.saturating_sub(self.expanded);
        if size > self.limits.entry_bytes || size > remaining {
            return Err(EpubError::LimitExceeded);
        }
        let mut bytes = Vec::new();
        (&mut file)
            .take(size as u64 + 1)
            .read_to_end(&mut bytes)
            .map_err(|_| EpubError::InvalidArchive)?;
        if bytes.len() != size {
            return Err(EpubError::InvalidArchive);
        }
        self.expanded += size;
        Ok(bytes)
    }

    pub fn load_encryption(&mut self, limits: EpubLimits) -> Result<()> {
        if !self.paths.contains_key("META-INF/encryption.xml") {
            return Ok(());
        }
        let text = xml_text(&self.read("META-INF/encryption.xml")?)?;
        let document = xml(&text, limits)?;
        for node in document
            .descendants()
            .filter(|n| n.has_tag_name(("http://www.w3.org/2001/04/xmlenc#", "CipherReference")))
        {
            let path = resolve("", node.attribute("URI").ok_or(EpubError::InvalidPackage)?)?;
            self.encrypted.insert(path);
        }
        Ok(())
    }
}

/// Resolve URI references once, against the referring file's directory. No
/// filesystem paths or fetching; encoded parent traversal cannot escape root.
pub(super) fn resolve(base: &str, reference: &str) -> Result<String> {
    if reference.contains(['?', '#']) {
        return Err(EpubError::InvalidPath);
    }
    let mut decoded = Vec::new();
    let mut bytes = reference.bytes();
    while let Some(byte) = bytes.next() {
        if byte == b'%' {
            let hex = |b: u8| (b as char).to_digit(16).map(|v| v as u8);
            let hi = bytes.next().and_then(hex).ok_or(EpubError::InvalidPath)?;
            let lo = bytes.next().and_then(hex).ok_or(EpubError::InvalidPath)?;
            decoded.push(hi * 16 + lo);
        } else {
            decoded.push(byte);
        }
    }
    let reference = std::str::from_utf8(&decoded).map_err(|_| EpubError::InvalidPath)?;
    if reference.is_empty()
        || reference.starts_with('/')
        || reference.contains([':', '\\'])
        || reference.chars().any(char::is_control)
    {
        return Err(EpubError::InvalidPath);
    }
    let directory = base.rsplit_once('/').map_or("", |(dir, _)| dir);
    normalize(format!("{directory}/{reference}").trim_start_matches('/'))
}

fn normalize(path: &str) -> Result<String> {
    if path.starts_with('/') || path.contains([':', '\\']) || path.chars().any(char::is_control) {
        return Err(EpubError::InvalidPath);
    }
    let mut parts = Vec::new();
    for part in path.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                if parts.pop().is_none() {
                    return Err(EpubError::InvalidPath);
                }
            }
            _ => parts.push(part),
        }
    }
    if parts.is_empty() {
        return Err(EpubError::InvalidPath);
    }
    Ok(parts.join("/"))
}
