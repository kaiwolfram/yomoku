//! Versioned backup metadata. See docs/BACKUP_FORMAT.md for the complete contract.
//!
//! This prerequisite performs no cryptography, randomness, SQLite or file I/O.
//! Parsed metadata is untrusted: it never authorizes restoration. Authentication
//! of the entire stream and validation of the staged database must happen first.

use std::{error::Error, fmt};

pub const HEADER_LEN: usize = 72;
pub const MANIFEST_LEN: usize = 32;
pub const FRAME_HEADER_LEN: usize = 5;
pub const SALT_LEN: usize = 16;
pub const STREAM_NONCE_LEN: usize = 19;
pub const CHUNK_LEN: u32 = 65_536;
pub const TAG_LEN: u32 = 16;
pub const ARGON2_MEMORY_KIB: u32 = 65_536;
pub const ARGON2_ITERATIONS: u32 = 3;
pub const ARGON2_LANES: u32 = 4;

// BE32 permits next at counters 0..u32::MAX and last at counter u32::MAX.
// Canonical framing always ends with a short (possibly empty) last segment.
pub const MAX_PLAINTEXT_LEN: u64 = (u32::MAX as u64 + 1) * CHUNK_LEN as u64 - 1;

const HEADER_MAGIC: &[u8; 8] = b"YOMOKUBK";
const PAYLOAD_MAGIC: &[u8; 8] = b"YOMOKUDB";
const FORMAT_VERSION: u16 = 1;
const PAYLOAD_VERSION: u16 = 1;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FormatError {
    InvalidLength,
    InvalidMagic,
    UnsupportedVersion,
    UnsupportedProfile,
    NonzeroReserved,
    InvalidManifest,
    InvalidFrame,
}

impl fmt::Display for FormatError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::InvalidLength => "invalid backup metadata length",
            Self::InvalidMagic => "unrecognized backup metadata",
            Self::UnsupportedVersion => "unsupported backup version",
            Self::UnsupportedProfile => "unsupported backup encryption profile",
            Self::NonzeroReserved => "unsupported backup reserved fields",
            Self::InvalidManifest => "invalid backup manifest",
            Self::InvalidFrame => "invalid backup frame",
        })
    }
}

impl Error for FormatError {}

/// Cleartext metadata; authenticate its exact encoded bytes as AAD on every frame.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Header {
    salt: [u8; SALT_LEN],
    stream_nonce: [u8; STREAM_NONCE_LEN],
}

impl Header {
    /// The future encryption caller must generate fresh independent OS randomness
    /// for both arrays on every export, including retries. Parsing cannot prove it.
    pub fn new(salt: [u8; SALT_LEN], stream_nonce: [u8; STREAM_NONCE_LEN]) -> Self {
        Self { salt, stream_nonce }
    }

    pub fn salt(&self) -> &[u8; SALT_LEN] {
        &self.salt
    }

    pub fn stream_nonce(&self) -> &[u8; STREAM_NONCE_LEN] {
        &self.stream_nonce
    }

    pub fn encode(&self) -> [u8; HEADER_LEN] {
        let mut bytes = [0; HEADER_LEN];
        bytes[..8].copy_from_slice(HEADER_MAGIC);
        bytes[8..10].copy_from_slice(&FORMAT_VERSION.to_le_bytes());
        bytes[10..12].copy_from_slice(&(HEADER_LEN as u16).to_le_bytes());
        // Argon2id, Argon2 v1.3, XChaCha20Poly1305, STREAM BE32.
        bytes[12..16].copy_from_slice(&[1, 0x13, 1, 1]);
        bytes[16..20].copy_from_slice(&ARGON2_MEMORY_KIB.to_le_bytes());
        bytes[20..24].copy_from_slice(&ARGON2_ITERATIONS.to_le_bytes());
        bytes[24..28].copy_from_slice(&ARGON2_LANES.to_le_bytes());
        bytes[28..32].copy_from_slice(&CHUNK_LEN.to_le_bytes());
        bytes[32..48].copy_from_slice(&self.salt);
        bytes[48..67].copy_from_slice(&self.stream_nonce);
        bytes
    }

    /// Accept exactly one header, before invoking any KDF or allocating chunks.
    /// Version 1 accepts only its fixed profile, never arbitrary attacker costs.
    pub fn parse(bytes: &[u8]) -> Result<Self, FormatError> {
        let bytes: &[u8; HEADER_LEN] = bytes.try_into().map_err(|_| FormatError::InvalidLength)?;
        if &bytes[..8] != HEADER_MAGIC {
            return Err(FormatError::InvalidMagic);
        }
        if bytes[8..10] != FORMAT_VERSION.to_le_bytes() {
            return Err(FormatError::UnsupportedVersion);
        }
        if bytes[10..12] != (HEADER_LEN as u16).to_le_bytes() {
            return Err(FormatError::InvalidLength);
        }
        if bytes[67..].iter().any(|byte| *byte != 0) {
            return Err(FormatError::NonzeroReserved);
        }
        let header = Self::new(
            bytes[32..48].try_into().expect("fixed salt range"),
            bytes[48..67].try_into().expect("fixed nonce range"),
        );
        if bytes[12..32] != header.encode()[12..32] {
            return Err(FormatError::UnsupportedProfile);
        }
        Ok(header)
    }
}

/// Encrypted payload prefix, followed by exactly `sqlite_len` database bytes.
/// Schema support, migration history and database integrity are checked separately.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Manifest {
    schema_version: u32,
    sqlite_len: u64,
}

impl Manifest {
    pub fn new(schema_version: u32, sqlite_len: u64) -> Result<Self, FormatError> {
        if schema_version == 0
            || sqlite_len < 512
            || !sqlite_len.is_multiple_of(512)
            || sqlite_len > MAX_PLAINTEXT_LEN - MANIFEST_LEN as u64
        {
            return Err(FormatError::InvalidManifest);
        }
        Ok(Self {
            schema_version,
            sqlite_len,
        })
    }

    pub fn schema_version(&self) -> u32 {
        self.schema_version
    }

    pub fn sqlite_len(&self) -> u64 {
        self.sqlite_len
    }

    pub fn encode(&self) -> [u8; MANIFEST_LEN] {
        let mut bytes = [0; MANIFEST_LEN];
        bytes[..8].copy_from_slice(PAYLOAD_MAGIC);
        bytes[8..10].copy_from_slice(&PAYLOAD_VERSION.to_le_bytes());
        bytes[10..12].copy_from_slice(&(MANIFEST_LEN as u16).to_le_bytes());
        bytes[12..16].copy_from_slice(&self.schema_version.to_le_bytes());
        bytes[16..24].copy_from_slice(&self.sqlite_len.to_le_bytes());
        bytes
    }

    pub fn parse(bytes: &[u8]) -> Result<Self, FormatError> {
        let bytes: &[u8; MANIFEST_LEN] =
            bytes.try_into().map_err(|_| FormatError::InvalidLength)?;
        if &bytes[..8] != PAYLOAD_MAGIC {
            return Err(FormatError::InvalidMagic);
        }
        if bytes[8..10] != PAYLOAD_VERSION.to_le_bytes() {
            return Err(FormatError::UnsupportedVersion);
        }
        if bytes[10..12] != (MANIFEST_LEN as u16).to_le_bytes() {
            return Err(FormatError::InvalidLength);
        }
        if bytes[24..].iter().any(|byte| *byte != 0) {
            return Err(FormatError::NonzeroReserved);
        }
        Self::new(
            u32::from_le_bytes(bytes[12..16].try_into().expect("fixed schema range")),
            u64::from_le_bytes(bytes[16..24].try_into().expect("fixed length range")),
        )
    }
}

/// Bounded public framing. Also authenticate these exact bytes as frame AAD.
/// Only the future STREAM decryptor can verify order, finality and authentication.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FrameHeader {
    is_last: bool,
    ciphertext_len: u32,
}

impl FrameHeader {
    pub fn next() -> Self {
        Self {
            is_last: false,
            ciphertext_len: CHUNK_LEN + TAG_LEN,
        }
    }

    pub fn last(plaintext_len: u32) -> Result<Self, FormatError> {
        if plaintext_len >= CHUNK_LEN {
            return Err(FormatError::InvalidFrame);
        }
        Ok(Self {
            is_last: true,
            ciphertext_len: plaintext_len + TAG_LEN,
        })
    }

    pub fn is_last(&self) -> bool {
        self.is_last
    }

    pub fn ciphertext_len(&self) -> u32 {
        self.ciphertext_len
    }

    pub fn encode(&self) -> [u8; FRAME_HEADER_LEN] {
        let mut bytes = [0; FRAME_HEADER_LEN];
        bytes[0] = u8::from(self.is_last);
        bytes[1..].copy_from_slice(&self.ciphertext_len.to_le_bytes());
        bytes
    }

    pub fn parse(bytes: &[u8]) -> Result<Self, FormatError> {
        let bytes: &[u8; FRAME_HEADER_LEN] =
            bytes.try_into().map_err(|_| FormatError::InvalidLength)?;
        let ciphertext_len = u32::from_le_bytes(bytes[1..].try_into().expect("fixed length range"));
        match bytes[0] {
            0 if ciphertext_len == CHUNK_LEN + TAG_LEN => Ok(Self::next()),
            1 => Self::last(
                ciphertext_len
                    .checked_sub(TAG_LEN)
                    .ok_or(FormatError::InvalidFrame)?,
            ),
            _ => Err(FormatError::InvalidFrame),
        }
    }
}
