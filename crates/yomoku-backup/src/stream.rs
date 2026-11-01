use std::{error::Error, fmt, io, io::Read, io::Write};

use aead_stream::{DecryptorBE32, EncryptorBE32};
use argon2::{Algorithm, Argon2, Block, Params, Version};
use chacha20poly1305::XChaCha20Poly1305;
use zeroize::{Zeroize, Zeroizing};

use crate::{
    FormatError, FrameHeader, Header, Manifest, ARGON2_ITERATIONS, ARGON2_LANES, ARGON2_MEMORY_KIB,
    CHUNK_LEN, FRAME_HEADER_LEN, HEADER_LEN, MANIFEST_LEN, MAX_PLAINTEXT_LEN, SALT_LEN,
    STREAM_NONCE_LEN, TAG_LEN,
};

/// Owned literal password bytes, cleared on drop. No Debug/Clone/serialization.
/// Callers must separately clear any UI/FFI copies; this cannot erase those.
pub struct Password(Zeroizing<Vec<u8>>);

impl Password {
    pub fn new(bytes: Vec<u8>) -> Self {
        Self(Zeroizing::new(bytes))
    }
}

/// Deliberately contains no caller-supplied I/O messages, passwords or content.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BackupError {
    Format(FormatError),
    Authentication,
    Io(io::ErrorKind),
    Entropy,
    KeyDerivation,
    Resource,
    Cancelled,
    LengthMismatch,
    StreamLimit,
    TrailingData,
}

impl fmt::Display for BackupError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Format(_) => "invalid or unsupported backup format",
            Self::Authentication => "backup authentication failed",
            Self::Io(_) => "backup I/O failed",
            Self::Entropy => "backup randomness unavailable",
            Self::KeyDerivation => "backup key derivation failed",
            Self::Resource => "backup memory unavailable",
            Self::Cancelled => "backup cancelled",
            Self::LengthMismatch => "backup payload length mismatch",
            Self::StreamLimit => "backup stream limit exceeded",
            Self::TrailingData => "unexpected data after backup",
        })
    }
}

impl Error for BackupError {}

impl From<FormatError> for BackupError {
    fn from(error: FormatError) -> Self {
        Self::Format(error)
    }
}

impl From<io::Error> for BackupError {
    fn from(error: io::Error) -> Self {
        Self::Io(error.kind())
    }
}

/// Encrypt an already validated, coherent private SQLite snapshot.
///
/// Generates fresh independent salt/nonce bytes on every call, including retries.
/// Checks the exact declared length and flushes output before returning success.
/// This does not validate SQLite, sync/close files or manage staging/publishing.
/// On *any* error the caller must discard the partial output.
///
/// Cancellation is polled around KDF and bounded I/O; it cannot interrupt an
/// in-progress KDF or blocking Read/Write. Do not call on the Android UI thread.
pub fn encrypt_snapshot(
    password: Password,
    manifest: &Manifest,
    database: &mut impl Read,
    output: &mut impl Write,
    mut cancelled: impl FnMut() -> bool,
) -> Result<(), BackupError> {
    check_cancelled(&mut cancelled)?;
    let header = fresh_header(getrandom::fill)?;
    check_cancelled(&mut cancelled)?;
    let key = derive_key(password, &header)?;
    check_cancelled(&mut cancelled)?;
    encrypt_with_key(&key, &header, manifest, database, output, &mut cancelled)
}

/// Authenticate/decrypt into a **new private disposable candidate**, never a live
/// database or external plaintext destination. Returns the manifest only after
/// Last, EOF, exact payload length and output flush all succeed.
///
/// Earlier authenticated chunks may already be written if a later check fails.
/// The caller must delete failed/cancelled candidates and handle crash cleanup.
/// Success still requires independent hardened SQLite/schema/data validation
/// before any confirmation or replacement. No SQL is inspected/executed here.
/// Cancellation has the same synchronous limits as `encrypt_snapshot`.
pub fn decrypt_snapshot(
    password: Password,
    archive: &mut impl Read,
    candidate: &mut impl Write,
    mut cancelled: impl FnMut() -> bool,
) -> Result<Manifest, BackupError> {
    check_cancelled(&mut cancelled)?;
    let mut encoded_header = [0; HEADER_LEN];
    read_exact(archive, &mut encoded_header, &mut cancelled)?;
    // Reject hostile work factors before allocating KDF scratch or deriving.
    let header = Header::parse(&encoded_header)?;
    check_cancelled(&mut cancelled)?;
    let key = derive_key(password, &header)?;
    check_cancelled(&mut cancelled)?;
    decrypt_with_key(
        &key,
        &encoded_header,
        &header,
        archive,
        candidate,
        &mut cancelled,
    )
}

fn fresh_header(
    mut fill: impl FnMut(&mut [u8]) -> Result<(), getrandom::Error>,
) -> Result<Header, BackupError> {
    let mut salt = [0; SALT_LEN];
    let mut nonce = [0; STREAM_NONCE_LEN];
    fill(&mut salt).map_err(|_| BackupError::Entropy)?;
    fill(&mut nonce).map_err(|_| BackupError::Entropy)?;
    Ok(Header::new(salt, nonce))
}

fn derive_key(password: Password, header: &Header) -> Result<Zeroizing<[u8; 32]>, BackupError> {
    let params = Params::new(ARGON2_MEMORY_KIB, ARGON2_ITERATIONS, ARGON2_LANES, Some(32))
        .map_err(|_| BackupError::KeyDerivation)?;
    let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);
    hash_with_memory(&argon2, &password.0, header.salt())
}

fn hash_with_memory(
    argon2: &Argon2<'_>,
    password: &[u8],
    salt: &[u8],
) -> Result<Zeroizing<[u8; 32]>, BackupError> {
    // Own the scratch so every exit clears all 64 MiB, including KDF failures.
    // Do not use the library's convenience allocator, whose Vec isn't guarded.
    let mut memory = Zeroizing::new(Vec::new());
    let count = argon2.params().block_count();
    memory
        .try_reserve_exact(count)
        .map_err(|_| BackupError::Resource)?;
    memory.resize(count, Block::default());
    let mut key = Zeroizing::new([0; 32]);
    argon2
        .hash_password_into_with_memory(password, salt, key.as_mut(), &mut memory)
        .map_err(|_| BackupError::KeyDerivation)?;
    Ok(key)
}

#[derive(Default)]
struct StreamLength {
    next_frames: u32,
    plaintext: u64,
}

impl StreamLength {
    fn add(&mut self, frame: &FrameHeader) -> Result<(), BackupError> {
        if !frame.is_last() {
            self.next_frames = self
                .next_frames
                .checked_add(1)
                .ok_or(BackupError::StreamLimit)?;
        }
        self.plaintext = self
            .plaintext
            .checked_add(u64::from(frame.ciphertext_len() - TAG_LEN))
            .filter(|total| *total <= MAX_PLAINTEXT_LEN)
            .ok_or(BackupError::StreamLimit)?;
        Ok(())
    }
}

fn chunk_buffer() -> Result<Zeroizing<Vec<u8>>, BackupError> {
    let mut buffer = Zeroizing::new(Vec::new());
    buffer
        .try_reserve_exact((CHUNK_LEN + TAG_LEN) as usize)
        .map_err(|_| BackupError::Resource)?;
    Ok(buffer)
}

fn aad(
    header: &[u8; HEADER_LEN],
    frame: &[u8; FRAME_HEADER_LEN],
) -> [u8; HEADER_LEN + FRAME_HEADER_LEN] {
    let mut bytes = [0; HEADER_LEN + FRAME_HEADER_LEN];
    bytes[..HEADER_LEN].copy_from_slice(header);
    bytes[HEADER_LEN..].copy_from_slice(frame);
    bytes
}

fn encrypt_with_key(
    key: &[u8; 32],
    header: &Header,
    manifest: &Manifest,
    database: &mut impl Read,
    output: &mut impl Write,
    cancelled: &mut impl FnMut() -> bool,
) -> Result<(), BackupError> {
    let mut input = io::Cursor::new(manifest.encode()).chain(database);
    let expected_len = MANIFEST_LEN as u64 + manifest.sqlite_len();
    encrypt_payload(key, header, &mut input, expected_len, output, cancelled)
}

fn encrypt_payload(
    key: &[u8; 32],
    header: &Header,
    input: &mut impl Read,
    expected_len: u64,
    output: &mut impl Write,
    cancelled: &mut impl FnMut() -> bool,
) -> Result<(), BackupError> {
    let encoded_header = header.encode();
    let mut encryptor = Some(EncryptorBE32::<XChaCha20Poly1305>::new(
        key.into(),
        header.stream_nonce().into(),
    ));
    let mut buffer = chunk_buffer()?;
    let mut length = StreamLength::default();
    write_all(output, &encoded_header, cancelled)?;
    loop {
        buffer.zeroize();
        buffer.resize(CHUNK_LEN as usize, 0);
        let size = read_chunk(input, &mut buffer, cancelled)?;
        buffer.truncate(size);
        let last = size < CHUNK_LEN as usize;
        let frame = if last {
            FrameHeader::last(size as u32)?
        } else {
            FrameHeader::next()
        };
        length.add(&frame)?;
        if length.plaintext > expected_len || (last && length.plaintext != expected_len) {
            return Err(BackupError::LengthMismatch);
        }
        let encoded_frame = frame.encode();
        let associated = aad(&encoded_header, &encoded_frame);
        check_cancelled(cancelled)?;
        if last {
            encryptor
                .take()
                .expect("Last ends the loop")
                .encrypt_last_in_place(&associated, &mut *buffer)
                .map_err(|_| BackupError::Authentication)?;
        } else {
            encryptor
                .as_mut()
                .expect("Last ends the loop")
                .encrypt_next_in_place(&associated, &mut *buffer)
                .map_err(|_| BackupError::Authentication)?;
        }
        write_all(output, &encoded_frame, cancelled)?;
        write_all(output, &buffer, cancelled)?;
        if last {
            check_cancelled(cancelled)?;
            output.flush()?;
            return check_cancelled(cancelled);
        }
    }
}

fn decrypt_with_key(
    key: &[u8; 32],
    encoded_header: &[u8; HEADER_LEN],
    header: &Header,
    archive: &mut impl Read,
    candidate: &mut impl Write,
    cancelled: &mut impl FnMut() -> bool,
) -> Result<Manifest, BackupError> {
    let mut decryptor = Some(DecryptorBE32::<XChaCha20Poly1305>::new(
        key.into(),
        header.stream_nonce().into(),
    ));
    let mut buffer = chunk_buffer()?;
    let mut length = StreamLength::default();
    let mut manifest: Option<Manifest> = None;
    loop {
        let mut encoded_frame = [0; FRAME_HEADER_LEN];
        read_exact(archive, &mut encoded_frame, cancelled)?;
        let frame = FrameHeader::parse(&encoded_frame)?;
        length.add(&frame)?;
        buffer.zeroize();
        buffer.resize(frame.ciphertext_len() as usize, 0);
        read_exact(archive, &mut buffer, cancelled)?;
        let associated = aad(encoded_header, &encoded_frame);
        check_cancelled(cancelled)?;
        if frame.is_last() {
            decryptor
                .take()
                .expect("Last ends the loop")
                .decrypt_last_in_place(&associated, &mut *buffer)
                .map_err(|_| BackupError::Authentication)?;
        } else {
            decryptor
                .as_mut()
                .expect("Last ends the loop")
                .decrypt_next_in_place(&associated, &mut *buffer)
                .map_err(|_| BackupError::Authentication)?;
        }
        let data = if manifest.is_none() {
            let prefix = buffer
                .get(..MANIFEST_LEN)
                .ok_or(BackupError::LengthMismatch)?;
            manifest = Some(Manifest::parse(prefix)?);
            &buffer[MANIFEST_LEN..]
        } else {
            &buffer[..]
        };
        let current = manifest.as_ref().expect("parsed first frame manifest");
        let expected_len = MANIFEST_LEN as u64 + current.sqlite_len();
        if length.plaintext > expected_len || (frame.is_last() && length.plaintext != expected_len)
        {
            return Err(BackupError::LengthMismatch);
        }
        write_all(candidate, data, cancelled)?;
        if frame.is_last() {
            let mut extra = [0];
            if read_chunk(archive, &mut extra, cancelled)? != 0 {
                return Err(BackupError::TrailingData);
            }
            check_cancelled(cancelled)?;
            candidate.flush()?;
            check_cancelled(cancelled)?;
            return Ok(manifest.expect("parsed first frame manifest"));
        }
    }
}

fn check_cancelled(cancelled: &mut impl FnMut() -> bool) -> Result<(), BackupError> {
    if cancelled() {
        Err(BackupError::Cancelled)
    } else {
        Ok(())
    }
}

// Unlike a single read, these loops treat short reads correctly and retry EINTR.
// Poll on *every* call, including Interrupted, rather than inside Read::read_exact.
fn read_chunk(
    reader: &mut impl Read,
    mut bytes: &mut [u8],
    cancelled: &mut impl FnMut() -> bool,
) -> Result<usize, BackupError> {
    let capacity = bytes.len();
    while !bytes.is_empty() {
        check_cancelled(cancelled)?;
        match reader.read(bytes) {
            Ok(0) => break,
            Ok(size) => bytes = &mut bytes[size..],
            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
            Err(error) => return Err(error.into()),
        }
    }
    check_cancelled(cancelled)?;
    Ok(capacity - bytes.len())
}

fn read_exact(
    reader: &mut impl Read,
    bytes: &mut [u8],
    cancelled: &mut impl FnMut() -> bool,
) -> Result<(), BackupError> {
    if read_chunk(reader, bytes, cancelled)? != bytes.len() {
        return Err(BackupError::Io(io::ErrorKind::UnexpectedEof));
    }
    Ok(())
}

fn write_all(
    writer: &mut impl Write,
    mut bytes: &[u8],
    cancelled: &mut impl FnMut() -> bool,
) -> Result<(), BackupError> {
    while !bytes.is_empty() {
        check_cancelled(cancelled)?;
        match writer.write(bytes) {
            Ok(0) => return Err(BackupError::Io(io::ErrorKind::WriteZero)),
            Ok(size) => bytes = &bytes[size..],
            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
            Err(error) => return Err(error.into()),
        }
    }
    check_cancelled(cancelled)
}

#[cfg(test)]
mod tests;
