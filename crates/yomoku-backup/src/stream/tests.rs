use super::*;
use chacha20poly1305::aead::{AeadInOut, KeyInit};

const KEY: [u8; 32] = [0x11; 32];

fn header() -> Header {
    Header::new([2; SALT_LEN], [0x22; STREAM_NONCE_LEN])
}

fn hex(bytes: &str) -> Vec<u8> {
    bytes
        .as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
        .collect()
}

fn database(len: usize) -> Vec<u8> {
    (0..len).map(|i| (i.wrapping_mul(37) % 251) as u8).collect()
}

fn seal(data: &[u8]) -> Vec<u8> {
    let mut archive = Vec::new();
    encrypt_with_key(
        &KEY,
        &header(),
        &Manifest::new(24, data.len() as u64).unwrap(),
        &mut &data[..],
        &mut archive,
        &mut || false,
    )
    .unwrap();
    archive
}

// Also allows authenticated hostile payloads without weakening the public API.
fn seal_payload(payload: &[u8]) -> Vec<u8> {
    let mut archive = Vec::new();
    encrypt_payload(
        &KEY,
        &header(),
        &mut &payload[..],
        payload.len() as u64,
        &mut archive,
        &mut || false,
    )
    .unwrap();
    archive
}

fn open(archive: &[u8], candidate: &mut impl Write) -> Result<Manifest, BackupError> {
    let encoded: &[u8; HEADER_LEN] = archive[..HEADER_LEN].try_into().unwrap();
    let parsed = Header::parse(encoded)?;
    decrypt_with_key(
        &KEY,
        encoded,
        &parsed,
        &mut &archive[HEADER_LEN..],
        candidate,
        &mut || false,
    )
}

#[test]
fn argon2_rfc9106_known_answer_uses_guarded_memory() {
    // RFC 9106 §5.3: test profile, secret and associated data, not archive costs.
    let params = argon2::ParamsBuilder::new()
        .m_cost(32)
        .t_cost(3)
        .p_cost(4)
        .output_len(32)
        .data(argon2::AssociatedData::new(&[4; 12]).unwrap())
        .build()
        .unwrap();
    let argon2 =
        Argon2::new_with_secret(&[3; 8], Algorithm::Argon2id, Version::V0x13, params).unwrap();
    assert_eq!(
        hash_with_memory(&argon2, &[1; 32], &[2; 16])
            .unwrap()
            .as_slice(),
        hex("0d640df58d78766c08c037a34a8b53c9d01ef0452d75b65eb52520e96b01e659")
    );
    assert_eq!(
        hash_with_memory(&argon2, b"password", b"short").unwrap_err(),
        BackupError::KeyDerivation
    );
}

#[test]
fn xchacha_draft_known_answer() {
    // draft-irtf-cfrg-xchacha-03 A.3.1 (also RustCrypto's published test vector).
    let key: [u8; 32] = std::array::from_fn(|i| 0x80 + i as u8);
    let nonce: [u8; 24] = std::array::from_fn(|i| 0x40 + i as u8);
    let associated = hex("50515253c0c1c2c3c4c5c6c7");
    let plaintext = b"Ladies and Gentlemen of the class of '99: If I could offer you only one tip for the future, sunscreen would be it.";
    let expected = hex(concat!(
        "bd6d179d3e83d43b9576579493c0e939572a1700252bfaccbed2902c21396cbb",
        "731c7f1b0b4aa6440bf3a82f4eda7e39ae64c6708c54c216cb96b72e1213b452",
        "2f8c9ba40db5d945b11b69b982c1bb9e3f3fac2bc369488f76b2383565d3fff9",
        "21f9664c97637da9768812f615c68b13b52ec0875924c1c7987947deafd8780acf49"
    ));
    let cipher = XChaCha20Poly1305::new((&key).into());
    let mut buffer = plaintext.to_vec();
    cipher
        .encrypt_in_place((&nonce).into(), &associated, &mut buffer)
        .unwrap();
    assert_eq!(buffer, expected);
    cipher
        .decrypt_in_place((&nonce).into(), &associated, &mut buffer)
        .unwrap();
    assert_eq!(buffer, plaintext);
}

#[test]
fn stream_be32_matches_independent_libsodium_fixture() {
    // Fixture provenance/reproduction: tools/backup-reference-vectors.py.
    let mut encryptor =
        EncryptorBE32::<XChaCha20Poly1305>::new((&KEY).into(), header().stream_nonce().into());
    let mut first = b"abc".to_vec();
    encryptor
        .encrypt_next_in_place(b"first", &mut first)
        .unwrap();
    assert_eq!(first, hex("9affe98e47e430435f2c75eb7f95a2ddf1cd78"));
    let mut last = b"def".to_vec();
    encryptor
        .encrypt_last_in_place(b"second", &mut last)
        .unwrap();
    assert_eq!(last, hex("503623bb3c0b9bc8dfb1f29bc8c758f1a4c8bc"));
    let mut decryptor =
        DecryptorBE32::<XChaCha20Poly1305>::new((&KEY).into(), header().stream_nonce().into());
    decryptor
        .decrypt_next_in_place(b"first", &mut first)
        .unwrap();
    decryptor
        .decrypt_last_in_place(b"second", &mut last)
        .unwrap();
    assert_eq!(first, b"abc");
    assert_eq!(last, b"def");
}

#[test]
fn public_password_profile_round_trip_freshness_and_authentication_errors() {
    let password = || Password::new(" 日本語🔑 e\u{301} ".as_bytes().to_vec());
    let key = derive_key(password(), &header()).unwrap();
    // Independent reference libargon2 result; literal whitespace/decomposed UTF-8.
    assert_eq!(
        key.as_slice(),
        hex("cbae8d1f2bd793b9a25168e77dc1318d0e9f9f852575bf3b20ce9a604c996c4e")
    );
    let data = database(CHUNK_LEN as usize * 2 + 512);
    let manifest = Manifest::new(24, data.len() as u64).unwrap();
    let mut first = Vec::new();
    encrypt_snapshot(
        password(),
        &manifest,
        &mut data.as_slice(),
        &mut first,
        || false,
    )
    .unwrap();
    let mut second = Vec::new();
    encrypt_snapshot(
        password(),
        &manifest,
        &mut data.as_slice(),
        &mut second,
        || false,
    )
    .unwrap();
    let first_header = Header::parse(&first[..HEADER_LEN]).unwrap();
    let second_header = Header::parse(&second[..HEADER_LEN]).unwrap();
    assert_ne!(first_header.salt(), second_header.salt());
    assert_ne!(first_header.stream_nonce(), second_header.stream_nonce());
    assert_ne!(first, second);
    let mut restored = Vec::new();
    assert_eq!(
        decrypt_snapshot(password(), &mut first.as_slice(), &mut restored, || false).unwrap(),
        manifest
    );
    assert_eq!(restored, data);
    let wrong = Password::new("日本語🔑 e\u{301}".as_bytes().to_vec());
    let mut candidate = Vec::new();
    assert_eq!(
        decrypt_snapshot(wrong, &mut first.as_slice(), &mut candidate, || false),
        Err(BackupError::Authentication)
    );
    assert!(candidate.is_empty());
    first[HEADER_LEN + FRAME_HEADER_LEN] ^= 1;
    assert_eq!(
        decrypt_snapshot(password(), &mut first.as_slice(), &mut candidate, || false),
        Err(BackupError::Authentication)
    );
    assert!(candidate.is_empty());
}

#[test]
fn entropy_failures_stop_both_randomness_requests() {
    for fail_at in [1, 2] {
        let mut calls = 0;
        let result = fresh_header(|bytes| {
            calls += 1;
            if calls == fail_at {
                return Err(getrandom::Error::new_custom(1));
            }
            bytes.fill(9);
            Ok(())
        });
        assert_eq!(result, Err(BackupError::Entropy));
        assert_eq!(calls, fail_at);
    }
    let mut sizes = Vec::new();
    let generated = fresh_header(|bytes| {
        sizes.push(bytes.len());
        bytes.fill(sizes.len() as u8);
        Ok(())
    })
    .unwrap();
    assert_eq!(sizes, [16, 19]);
    assert_eq!(generated, Header::new([1; 16], [2; 19]));
}

#[test]
fn public_rejects_header_profile_before_password_work_or_output() {
    let mut unsupported = header().encode();
    unsupported[16..20].copy_from_slice(&u32::MAX.to_le_bytes());
    let mut candidate = Vec::new();
    assert_eq!(
        decrypt_snapshot(
            Password::new(Vec::new()),
            &mut unsupported.as_slice(),
            &mut candidate,
            || false
        ),
        Err(BackupError::Format(FormatError::UnsupportedProfile))
    );
    assert!(candidate.is_empty());
    for length in 0..HEADER_LEN {
        assert_eq!(
            decrypt_snapshot(
                Password::new(Vec::new()),
                &mut &unsupported[..length],
                &mut candidate,
                || false
            ),
            Err(BackupError::Io(io::ErrorKind::UnexpectedEof))
        );
    }
}

#[test]
fn bounded_round_trips_across_chunk_boundaries_and_short_interrupted_io() {
    for len in [512, 65_024, 65_536, 66_048, 196_608] {
        let data = database(len);
        let manifest = Manifest::new(24, len as u64).unwrap();
        let mut source = TestReader::new(&data);
        let mut encrypted = TestWriter::default();
        encrypt_with_key(
            &KEY,
            &header(),
            &manifest,
            &mut source,
            &mut encrypted,
            &mut || false,
        )
        .unwrap();
        assert!(encrypted.flushed);
        let mut archive = TestReader::new(&encrypted.bytes[HEADER_LEN..]);
        let mut candidate = TestWriter::default();
        assert_eq!(
            decrypt_with_key(
                &KEY,
                &header().encode(),
                &header(),
                &mut archive,
                &mut candidate,
                &mut || false
            )
            .unwrap(),
            manifest
        );
        assert_eq!(candidate.bytes, data);
        assert!(candidate.flushed);
        assert_eq!(source.position, len);
        assert_eq!(archive.position + HEADER_LEN, encrypted.bytes.len());
    }
}

#[test]
fn canonical_stream_framing_including_authenticated_empty_last() {
    for len in [0, 1, 65_535, 65_536, 65_537, 131_072] {
        let payload = database(len);
        let encoded = seal_payload(&payload);
        let mut reader = &encoded[HEADER_LEN..];
        let mut decryptor = Some(DecryptorBE32::<XChaCha20Poly1305>::new(
            (&KEY).into(),
            header().stream_nonce().into(),
        ));
        let mut recovered = Vec::new();
        let mut nexts = 0;
        loop {
            let mut bytes = [0; FRAME_HEADER_LEN];
            reader.read_exact(&mut bytes).unwrap();
            let frame = FrameHeader::parse(&bytes).unwrap();
            let mut buffer = vec![0; frame.ciphertext_len() as usize];
            reader.read_exact(&mut buffer).unwrap();
            if frame.is_last() {
                assert_eq!(buffer.len(), len % CHUNK_LEN as usize + TAG_LEN as usize);
                decryptor
                    .take()
                    .unwrap()
                    .decrypt_last_in_place(&aad(&header().encode(), &bytes), &mut buffer)
                    .unwrap();
                recovered.extend(buffer);
                assert!(reader.is_empty());
                break;
            }
            nexts += 1;
            decryptor
                .as_mut()
                .unwrap()
                .decrypt_next_in_place(&aad(&header().encode(), &bytes), &mut buffer)
                .unwrap();
            recovered.extend(buffer);
        }
        assert_eq!(nexts, len / CHUNK_LEN as usize);
        assert_eq!(recovered, payload);
    }
}

#[test]
fn every_small_archive_truncation_and_every_ciphertext_byte_tamper_fails() {
    let archive = seal(&database(512));
    for end in HEADER_LEN..archive.len() {
        assert!(
            open(&archive[..end], &mut Vec::new()).is_err(),
            "truncation {end}"
        );
    }
    for offset in HEADER_LEN + FRAME_HEADER_LEN..archive.len() {
        let mut changed = archive.clone();
        changed[offset] ^= 1;
        let mut candidate = Vec::new();
        assert_eq!(
            open(&changed, &mut candidate),
            Err(BackupError::Authentication),
            "tamper {offset}"
        );
        assert!(candidate.is_empty());
    }
}

#[test]
fn every_header_byte_and_frame_metadata_are_bound() {
    let archive = seal(&database(512));
    for offset in 0..HEADER_LEN {
        let mut changed = archive.clone();
        changed[offset] ^= 1;
        let mut candidate = Vec::new();
        assert!(open(&changed, &mut candidate).is_err(), "header {offset}");
        assert!(candidate.is_empty());
    }
    for offset in HEADER_LEN..HEADER_LEN + FRAME_HEADER_LEN {
        let mut changed = archive.clone();
        changed[offset] ^= 1;
        assert!(open(&changed, &mut Vec::new()).is_err(), "frame {offset}");
    }
    let next_archive = seal(&database(CHUNK_LEN as usize));
    let mut changed = next_archive.clone();
    // Valid short Last metadata substituted for Next, with correspondingly short
    // ciphertext: fails authentication, rather than just structural parsing.
    changed[HEADER_LEN..HEADER_LEN + FRAME_HEADER_LEN]
        .copy_from_slice(&FrameHeader::last(CHUNK_LEN - 1).unwrap().encode());
    assert_eq!(
        open(&changed, &mut Vec::new()),
        Err(BackupError::Authentication)
    );
}

#[test]
fn reorder_repeat_splice_missing_last_and_trailing_frames_fail() {
    let archive = seal(&database(CHUNK_LEN as usize * 3));
    let frame_len = FRAME_HEADER_LEN + (CHUNK_LEN + TAG_LEN) as usize;
    let a = HEADER_LEN;
    let b = a + frame_len;
    let c = b + frame_len;
    let last = c + frame_len;
    let cases = [
        [&archive[..a], &archive[b..c], &archive[a..b], &archive[c..]].concat(),
        [&archive[..b], &archive[a..b], &archive[b..]].concat(),
        [&archive[..b], &archive[c..]].concat(),
        archive[..last].to_vec(),
        [&archive[..], &archive[last..]].concat(),
        [&archive[..], &[0]].concat(),
    ];
    for changed in cases {
        assert!(open(&changed, &mut Vec::new()).is_err());
    }
    let mut other_data = database(CHUNK_LEN as usize * 3);
    other_data[CHUNK_LEN as usize] ^= 1;
    // Fresh nonce on the foreign archive, even though the test key is reused.
    let foreign_header = Header::new([2; 16], [0x33; STREAM_NONCE_LEN]);
    let mut foreign = Vec::new();
    encrypt_with_key(
        &KEY,
        &foreign_header,
        &Manifest::new(24, other_data.len() as u64).unwrap(),
        &mut other_data.as_slice(),
        &mut foreign,
        &mut || false,
    )
    .unwrap();
    let spliced = [&archive[..b], &foreign[b..c], &archive[c..]].concat();
    assert_eq!(
        open(&spliced, &mut Vec::new()),
        Err(BackupError::Authentication)
    );
    // A late failed tag may leave earlier verified plaintext in private staging.
    let mut changed = archive.clone();
    *changed.last_mut().unwrap() ^= 1;
    let mut candidate = Vec::new();
    assert_eq!(
        open(&changed, &mut candidate),
        Err(BackupError::Authentication)
    );
    assert!(!candidate.is_empty());
}

#[test]
fn hostile_lengths_are_rejected_before_payload_read() {
    for size in [0, 15, 65_552, 65_553, u32::MAX] {
        let mut archive = header().encode().to_vec();
        archive.push(1);
        archive.extend(size.to_le_bytes());
        let mut reader = TestReader::new(&archive[HEADER_LEN..]);
        reader.fail_at = Some(FRAME_HEADER_LEN);
        assert_eq!(
            decrypt_with_key(
                &KEY,
                &header().encode(),
                &header(),
                &mut reader,
                &mut Vec::new(),
                &mut || false
            ),
            Err(BackupError::Format(FormatError::InvalidFrame))
        );
        assert_eq!(reader.position, FRAME_HEADER_LEN);
    }
}

#[test]
fn authenticated_manifest_and_exact_payload_length_are_required() {
    for len in 0..MANIFEST_LEN {
        assert!(open(&seal_payload(&vec![0; len]), &mut Vec::new()).is_err());
    }
    let manifest = Manifest::new(24, 512).unwrap();
    for delta in [-1_i32, 1, 512] {
        let payload = [
            manifest.encode().as_slice(),
            &vec![0; (512 + delta) as usize],
        ]
        .concat();
        assert_eq!(
            open(&seal_payload(&payload), &mut Vec::new()),
            Err(BackupError::LengthMismatch)
        );
    }
    for offset in [0, 8, 10, 12, 16, 24, 31] {
        let mut payload = [manifest.encode().as_slice(), &database(512)].concat();
        payload[offset] = if offset == 12 { 0 } else { payload[offset] ^ 1 };
        assert!(
            open(&seal_payload(&payload), &mut Vec::new()).is_err(),
            "manifest {offset}"
        );
    }
    // Empty Last must authenticate even though this payload's total cannot be
    // valid manifest + aligned SQLite. Missing/tag-tampered Last never succeeds.
    let mut payload = vec![0; CHUNK_LEN as usize];
    payload[..MANIFEST_LEN].copy_from_slice(&Manifest::new(24, CHUNK_LEN as u64).unwrap().encode());
    let mut encoded = seal_payload(&payload);
    assert_eq!(
        open(&encoded, &mut Vec::new()),
        Err(BackupError::LengthMismatch)
    );
    *encoded.last_mut().unwrap() ^= 1;
    assert_eq!(
        open(&encoded, &mut Vec::new()),
        Err(BackupError::Authentication)
    );
}

#[test]
fn export_rejects_short_or_extra_snapshot_without_last() {
    let manifest = Manifest::new(24, 512).unwrap();
    for len in [0, 511, 513, 131_072] {
        let mut output = Vec::new();
        assert_eq!(
            encrypt_with_key(
                &KEY,
                &header(),
                &manifest,
                &mut database(len).as_slice(),
                &mut output,
                &mut || false
            ),
            Err(BackupError::LengthMismatch)
        );
        assert_eq!(output.len(), HEADER_LEN);
    }
}

#[test]
fn checked_counter_and_total_limits_allow_maximum_last_only() {
    let mut length = StreamLength {
        next_frames: u32::MAX - 1,
        plaintext: u64::from(u32::MAX - 1) * u64::from(CHUNK_LEN),
    };
    length.add(&FrameHeader::next()).unwrap();
    length
        .add(&FrameHeader::last(CHUNK_LEN - 1).unwrap())
        .unwrap();
    assert_eq!(length.plaintext, MAX_PLAINTEXT_LEN);
    assert_eq!(
        length.add(&FrameHeader::next()),
        Err(BackupError::StreamLimit)
    );
    let mut length = StreamLength {
        next_frames: 0,
        plaintext: MAX_PLAINTEXT_LEN,
    };
    length.add(&FrameHeader::last(0).unwrap()).unwrap();
    assert_eq!(
        length.add(&FrameHeader::last(1).unwrap()),
        Err(BackupError::StreamLimit)
    );
    let mut length = StreamLength {
        next_frames: 0,
        plaintext: u64::MAX,
    };
    assert_eq!(
        length.add(&FrameHeader::last(1).unwrap()),
        Err(BackupError::StreamLimit)
    );
}

#[test]
fn read_write_eof_probe_zero_write_and_flush_faults_never_succeed() {
    let data = database(CHUNK_LEN as usize + 512);
    let archive = seal(&data);
    let boundary = HEADER_LEN + FRAME_HEADER_LEN + (CHUNK_LEN + TAG_LEN) as usize;
    for fail_at in [
        0,
        1,
        FRAME_HEADER_LEN - 1,
        FRAME_HEADER_LEN,
        boundary - HEADER_LEN,
        archive.len() - HEADER_LEN - 1,
        archive.len() - HEADER_LEN,
    ] {
        let mut reader = TestReader::new(&archive[HEADER_LEN..]);
        reader.fail_at = Some(fail_at);
        assert_eq!(
            decrypt_with_key(
                &KEY,
                &header().encode(),
                &header(),
                &mut reader,
                &mut Vec::new(),
                &mut || false
            ),
            Err(BackupError::Io(io::ErrorKind::Other)),
            "read {fail_at}"
        );
    }
    let manifest = Manifest::new(24, data.len() as u64).unwrap();
    for fail_at in [
        0,
        1,
        HEADER_LEN - 1,
        HEADER_LEN,
        HEADER_LEN + FRAME_HEADER_LEN,
        boundary,
        archive.len() - 1,
    ] {
        let mut writer = TestWriter {
            fail_at: Some(fail_at),
            ..Default::default()
        };
        assert_eq!(
            encrypt_with_key(
                &KEY,
                &header(),
                &manifest,
                &mut data.as_slice(),
                &mut writer,
                &mut || false
            ),
            Err(BackupError::Io(io::ErrorKind::Other)),
            "write {fail_at}"
        );
        assert!(!writer.flushed);
    }
    for fail_at in [0, 1, CHUNK_LEN as usize - MANIFEST_LEN, data.len() - 1] {
        let mut writer = TestWriter {
            fail_at: Some(fail_at),
            ..Default::default()
        };
        assert_eq!(
            open(&archive, &mut writer),
            Err(BackupError::Io(io::ErrorKind::Other))
        );
    }
    let mut source = TestReader::new(&data);
    source.fail_at = Some(CHUNK_LEN as usize);
    assert_eq!(
        encrypt_with_key(
            &KEY,
            &header(),
            &manifest,
            &mut source,
            &mut Vec::new(),
            &mut || false
        ),
        Err(BackupError::Io(io::ErrorKind::Other))
    );
    for zero in [true, false] {
        let mut writer = TestWriter {
            zero,
            flush_error: !zero,
            ..Default::default()
        };
        let error = if zero {
            BackupError::Io(io::ErrorKind::WriteZero)
        } else {
            BackupError::Io(io::ErrorKind::Other)
        };
        assert_eq!(open(&archive, &mut writer), Err(error));
        let mut writer = TestWriter {
            zero,
            flush_error: !zero,
            ..Default::default()
        };
        assert_eq!(
            encrypt_with_key(
                &KEY,
                &header(),
                &manifest,
                &mut data.as_slice(),
                &mut writer,
                &mut || false
            ),
            Err(error)
        );
    }
}

#[test]
fn cancellation_at_every_poll_discards_success_including_after_flush() {
    let data = database(CHUNK_LEN as usize + 512);
    let manifest = Manifest::new(24, data.len() as u64).unwrap();
    let archive = seal(&data);
    let mut encrypt_polls = 0;
    encrypt_with_key(
        &KEY,
        &header(),
        &manifest,
        &mut data.as_slice(),
        &mut Vec::new(),
        &mut || {
            encrypt_polls += 1;
            false
        },
    )
    .unwrap();
    let mut decrypt_polls = 0;
    decrypt_with_key(
        &KEY,
        &header().encode(),
        &header(),
        &mut &archive[HEADER_LEN..],
        &mut Vec::new(),
        &mut || {
            decrypt_polls += 1;
            false
        },
    )
    .unwrap();
    for stop in 1..=encrypt_polls {
        let mut count = 0;
        assert_eq!(
            encrypt_with_key(
                &KEY,
                &header(),
                &manifest,
                &mut data.as_slice(),
                &mut Vec::new(),
                &mut || {
                    count += 1;
                    count == stop
                }
            ),
            Err(BackupError::Cancelled)
        );
    }
    for stop in 1..=decrypt_polls {
        let mut count = 0;
        assert_eq!(
            decrypt_with_key(
                &KEY,
                &header().encode(),
                &header(),
                &mut &archive[HEADER_LEN..],
                &mut Vec::new(),
                &mut || {
                    count += 1;
                    count == stop
                }
            ),
            Err(BackupError::Cancelled)
        );
    }
    let mut output = Vec::new();
    assert_eq!(
        encrypt_snapshot(
            Password::new(Vec::new()),
            &manifest,
            &mut data.as_slice(),
            &mut output,
            || true
        ),
        Err(BackupError::Cancelled)
    );
    assert!(output.is_empty());
    assert_eq!(
        decrypt_snapshot(
            Password::new(Vec::new()),
            &mut archive.as_slice(),
            &mut output,
            || true
        ),
        Err(BackupError::Cancelled)
    );
    assert!(output.is_empty());
    // Repeated EINTR remains cancellable instead of spinning inside std helpers.
    struct Interrupted;
    impl Read for Interrupted {
        fn read(&mut self, _: &mut [u8]) -> io::Result<usize> {
            Err(io::ErrorKind::Interrupted.into())
        }
    }
    impl Write for Interrupted {
        fn write(&mut self, _: &[u8]) -> io::Result<usize> {
            Err(io::ErrorKind::Interrupted.into())
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }
    let mut count = 0;
    assert_eq!(
        read_chunk(&mut Interrupted, &mut [0], &mut || {
            count += 1;
            count == 10
        }),
        Err(BackupError::Cancelled)
    );
    let mut count = 0;
    assert_eq!(
        write_all(&mut Interrupted, &[0], &mut || {
            count += 1;
            count == 10
        }),
        Err(BackupError::Cancelled)
    );
}

#[test]
fn io_diagnostics_never_include_caller_content() {
    let error = BackupError::from(io::Error::other("password/private document"));
    assert_eq!(error, BackupError::Io(io::ErrorKind::Other));
    assert!(!format!("{error:?} {error}").contains("password/private document"));
}

struct TestReader<'a> {
    bytes: &'a [u8],
    position: usize,
    interrupt: bool,
    fail_at: Option<usize>,
}

impl<'a> TestReader<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self {
            bytes,
            position: 0,
            interrupt: true,
            fail_at: None,
        }
    }
}

impl Read for TestReader<'_> {
    fn read(&mut self, output: &mut [u8]) -> io::Result<usize> {
        assert!(output.len() <= (CHUNK_LEN + TAG_LEN) as usize);
        self.interrupt = !self.interrupt;
        if !self.interrupt {
            return Err(io::ErrorKind::Interrupted.into());
        }
        let remaining = self
            .fail_at
            .map_or(usize::MAX, |limit| limit - self.position);
        if remaining == 0 {
            return Err(io::Error::other("private source"));
        }
        let size = output
            .len()
            .min(97)
            .min(remaining)
            .min(self.bytes.len() - self.position);
        output[..size].copy_from_slice(&self.bytes[self.position..self.position + size]);
        self.position += size;
        Ok(size)
    }
}

#[derive(Default)]
struct TestWriter {
    bytes: Vec<u8>,
    interrupt: bool,
    flushed: bool,
    fail_at: Option<usize>,
    flush_error: bool,
    zero: bool,
}

impl Write for TestWriter {
    fn write(&mut self, input: &[u8]) -> io::Result<usize> {
        assert!(input.len() <= (CHUNK_LEN + TAG_LEN) as usize);
        self.interrupt = !self.interrupt;
        if self.interrupt {
            return Err(io::ErrorKind::Interrupted.into());
        }
        if self.zero {
            return Ok(0);
        }
        let remaining = self
            .fail_at
            .map_or(usize::MAX, |limit| limit - self.bytes.len());
        if remaining == 0 {
            return Err(io::Error::other("private destination"));
        }
        let size = input.len().min(83).min(remaining);
        self.bytes.extend_from_slice(&input[..size]);
        Ok(size)
    }
    fn flush(&mut self) -> io::Result<()> {
        if self.flush_error {
            return Err(io::Error::other("private destination"));
        }
        self.flushed = true;
        Ok(())
    }
}
