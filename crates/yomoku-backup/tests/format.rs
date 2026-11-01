use yomoku_backup::{
    FormatError, FrameHeader, Header, Manifest, CHUNK_LEN, FRAME_HEADER_LEN, HEADER_LEN,
    MANIFEST_LEN, MAX_PLAINTEXT_LEN, TAG_LEN,
};

fn unhex(hex: &str) -> Vec<u8> {
    let (pairs, remainder) = hex.as_bytes().as_chunks::<2>();
    assert!(remainder.is_empty());
    pairs
        .iter()
        .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
        .collect()
}

fn golden_header() -> Vec<u8> {
    unhex(concat!(
        "594f4d4f4b55424b0100480001130101",
        "00000100030000000400000000000100",
        "000102030405060708090a0b0c0d0e0f",
        "101112131415161718191a1b1c1d1e1f202122",
        "0000000000"
    ))
}

#[test]
fn header_matches_version_one_wire_fixture() {
    let header = Header::new(
        std::array::from_fn(|i| i as u8),
        std::array::from_fn(|i| (i + 16) as u8),
    );
    assert_eq!(header.encode().as_slice(), golden_header());
    let parsed = Header::parse(&golden_header()).unwrap();
    assert_eq!(parsed, header);
    assert_eq!(parsed.salt(), header.salt());
    assert_eq!(parsed.stream_nonce(), header.stream_nonce());
}

#[test]
fn header_rejects_every_truncation_trailing_bytes_and_wrong_declared_length() {
    let golden = golden_header();
    for end in 0..HEADER_LEN {
        assert_eq!(
            Header::parse(&golden[..end]),
            Err(FormatError::InvalidLength)
        );
    }
    let mut extra = golden.clone();
    extra.push(0);
    assert_eq!(Header::parse(&extra), Err(FormatError::InvalidLength));
    for length in [0_u16, 71, 73, u16::MAX] {
        let mut bytes = golden.clone();
        bytes[10..12].copy_from_slice(&length.to_le_bytes());
        assert_eq!(Header::parse(&bytes), Err(FormatError::InvalidLength));
    }
}

#[test]
fn header_rejects_unknown_versions_profiles_costs_and_reserved_bytes() {
    let golden = golden_header();
    for (range, error) in [
        (0..8, FormatError::InvalidMagic),
        (8..10, FormatError::UnsupportedVersion),
        (12..32, FormatError::UnsupportedProfile),
        (67..72, FormatError::NonzeroReserved),
    ] {
        for index in range {
            for replacement in 0..=u8::MAX {
                if replacement == golden[index] {
                    continue;
                }
                let mut bytes = golden.clone();
                bytes[index] = replacement;
                assert_eq!(Header::parse(&bytes), Err(error), "byte {index}");
            }
        }
    }
    // Huge costs must be rejected as metadata; no KDF may be invoked first.
    let mut hostile = golden;
    hostile[16..28].fill(u8::MAX);
    assert_eq!(
        Header::parse(&hostile),
        Err(FormatError::UnsupportedProfile)
    );
}

#[test]
fn salt_and_nonce_are_opaque_bytes_not_proof_of_randomness_or_authenticity() {
    for byte in 0..=u8::MAX {
        let header = Header::new([byte; 16], [byte; 19]);
        assert_eq!(Header::parse(&header.encode()).unwrap(), header);
    }
}

#[test]
fn encrypted_manifest_has_independent_version_and_exact_u64_length() {
    let fixture = unhex(concat!(
        "594f4d4f4b5544420100200018000000",
        "00100000010000000000000000000000"
    ));
    let manifest = Manifest::new(24, 0x1_0000_1000).unwrap();
    assert_eq!(manifest.encode().as_slice(), fixture);
    assert_eq!(Manifest::parse(&fixture).unwrap(), manifest);
    assert_eq!(manifest.schema_version(), 24);
    assert_eq!(manifest.sqlite_len(), 0x1_0000_1000);
    // Parsing is not a promise that this installation can migrate the schema.
    assert!(Manifest::new(u32::MAX, 512).is_ok());
}

#[test]
fn manifest_rejects_malformed_versions_lengths_and_reserved_fields() {
    let golden = Manifest::new(24, 4096).unwrap().encode();
    for end in 0..MANIFEST_LEN {
        assert_eq!(
            Manifest::parse(&golden[..end]),
            Err(FormatError::InvalidLength)
        );
    }
    let mut extra = golden.to_vec();
    extra.push(0);
    assert_eq!(Manifest::parse(&extra), Err(FormatError::InvalidLength));
    for (index, error) in [
        (0, FormatError::InvalidMagic),
        (8, FormatError::UnsupportedVersion),
        (10, FormatError::InvalidLength),
        (24, FormatError::NonzeroReserved),
        (31, FormatError::NonzeroReserved),
    ] {
        let mut bytes = golden;
        bytes[index] ^= 1;
        assert_eq!(Manifest::parse(&bytes), Err(error));
    }
}

#[test]
fn manifest_bounds_database_size_without_allocation_or_integer_wrap() {
    assert_eq!(MAX_PLAINTEXT_LEN, (1_u64 << 48) - 1);
    for length in [0, 1, 511, 513, 4095, MAX_PLAINTEXT_LEN, u64::MAX] {
        assert_eq!(Manifest::new(24, length), Err(FormatError::InvalidManifest));
        let mut bytes = Manifest::new(24, 512).unwrap().encode();
        bytes[16..24].copy_from_slice(&length.to_le_bytes());
        assert_eq!(Manifest::parse(&bytes), Err(FormatError::InvalidManifest));
    }
    assert_eq!(Manifest::new(0, 512), Err(FormatError::InvalidManifest));
    let maximum = (MAX_PLAINTEXT_LEN - MANIFEST_LEN as u64) / 512 * 512;
    assert!(Manifest::new(24, maximum).is_ok());
    assert_eq!(
        Manifest::new(24, maximum + 512),
        Err(FormatError::InvalidManifest)
    );
    for length in [512, 4096, 65_536, maximum] {
        let manifest = Manifest::new(24, length).unwrap();
        assert_eq!(Manifest::parse(&manifest.encode()).unwrap(), manifest);
    }
}

#[test]
fn frame_wire_fixtures_and_every_canonical_final_length_are_bounded() {
    let next = FrameHeader::next();
    assert_eq!(next.encode(), [0, 0x10, 0, 1, 0]);
    assert_eq!(FrameHeader::parse(&next.encode()).unwrap(), next);
    assert!(!next.is_last());
    assert_eq!(next.ciphertext_len(), CHUNK_LEN + TAG_LEN);
    assert_eq!(FrameHeader::last(0).unwrap().encode(), [1, 0x10, 0, 0, 0]);
    assert_eq!(
        FrameHeader::last(65_535).unwrap().encode(),
        [1, 0x0f, 0, 1, 0]
    );
    for length in 0..CHUNK_LEN {
        let last = FrameHeader::last(length).unwrap();
        assert!(last.is_last());
        assert_eq!(last.ciphertext_len(), length + TAG_LEN);
        assert_eq!(FrameHeader::parse(&last.encode()).unwrap(), last);
    }
}

#[test]
fn frames_reject_hostile_sizes_flags_truncation_and_noncanonical_boundaries() {
    for end in 0..FRAME_HEADER_LEN {
        assert_eq!(
            FrameHeader::parse(&[0; FRAME_HEADER_LEN][..end]),
            Err(FormatError::InvalidLength)
        );
    }
    assert_eq!(
        FrameHeader::parse(&[0; FRAME_HEADER_LEN + 1]),
        Err(FormatError::InvalidLength)
    );
    for length in [CHUNK_LEN, u32::MAX - TAG_LEN, u32::MAX] {
        assert_eq!(FrameHeader::last(length), Err(FormatError::InvalidFrame));
    }
    for flag in 0..=u8::MAX {
        for length in [0_u32, 15, 16, 17, 65_551, 65_552, 65_553, u32::MAX] {
            let mut bytes = [0; FRAME_HEADER_LEN];
            bytes[0] = flag;
            bytes[1..].copy_from_slice(&length.to_le_bytes());
            let valid =
                (flag == 0 && length == 65_552) || (flag == 1 && (16..65_552).contains(&length));
            assert_eq!(FrameHeader::parse(&bytes).is_ok(), valid);
        }
    }
}
