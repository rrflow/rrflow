use super::*;

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[test]
fn application_and_family_tags_are_frozen() {
    assert_eq!(APPLICATION_FORMAT.to_be_bytes(), *b"RRKV0001");
    let families = [
        KeyFamily::Current,
        KeyFamily::Temporal,
        KeyFamily::OutgoingEdge,
        KeyFamily::IncomingEdge,
        KeyFamily::Scalar,
        KeyFamily::Unique,
        KeyFamily::TermDictionary,
        KeyFamily::TermStatistic,
        KeyFamily::TermPosting,
        KeyFamily::Vector,
        KeyFamily::ProjectionDelta,
        KeyFamily::Catalogue,
        KeyFamily::RuntimeCommit,
        KeyFamily::Outbox,
        KeyFamily::Audit,
        KeyFamily::EngineEvent,
        KeyFamily::System,
    ];
    assert_eq!(
        families.map(|family| family as u8),
        [
            0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18, 0x19, 0x1a, 0x1b, 0x1c, 0x1d,
            0x1e, 0x1f, 0x20,
        ]
    );
    assert_eq!(
        [
            CatalogueSubfamily::FunctionArtifact,
            CatalogueSubfamily::FunctionDefinition,
            CatalogueSubfamily::TransactionFunctionBinding,
            CatalogueSubfamily::Membership,
            CatalogueSubfamily::Head,
            CatalogueSubfamily::InvocationReceipt,
        ]
        .map(|subfamily| subfamily as u8),
        [1, 2, 3, 4, 5, 6]
    );
    assert_eq!(KeyCodec::from_application_format(None), None);
    assert_eq!(
        KeyCodec::from_application_format(Some(APPLICATION_FORMAT)),
        Some(KeyCodec)
    );
    assert_eq!(KeyCodec::from_application_format(Some(7)), None);
}

#[test]
fn every_family_and_field_type_round_trips() {
    let codec = KeyCodec;
    let address = KeyAddress {
        tenant: Some("tenant/a"),
        scope: Some("estate:\0-safe"),
    };
    let parts = [
        KeyPart::Bytes(&[0, 1, 0xff, b'/']),
        KeyPart::Text("src/\0is-data"),
        KeyPart::Bool(true),
        KeyPart::U8(7),
        KeyPart::U64(u64::MAX),
        KeyPart::I64(i64::MIN),
        KeyPart::DescU64(42),
    ];
    for family in [
        KeyFamily::Current,
        KeyFamily::Temporal,
        KeyFamily::OutgoingEdge,
        KeyFamily::IncomingEdge,
        KeyFamily::Scalar,
        KeyFamily::Unique,
        KeyFamily::TermDictionary,
        KeyFamily::TermStatistic,
        KeyFamily::TermPosting,
        KeyFamily::Vector,
        KeyFamily::ProjectionDelta,
        KeyFamily::Catalogue,
        KeyFamily::RuntimeCommit,
        KeyFamily::Outbox,
        KeyFamily::Audit,
        KeyFamily::EngineEvent,
        KeyFamily::System,
    ] {
        let encoded = codec.encode(address, family, &parts);
        let decoded = codec.decode(&encoded).unwrap();
        assert_eq!(decoded.tenant.as_deref(), address.tenant);
        assert_eq!(decoded.scope.as_deref(), address.scope);
        assert_eq!(decoded.family, family);
        assert_eq!(
            decoded.parts,
            vec![
                DecodedKeyPart::Bytes(vec![0, 1, 0xff, b'/']),
                DecodedKeyPart::Text("src/\0is-data".into()),
                DecodedKeyPart::Bool(true),
                DecodedKeyPart::U8(7),
                DecodedKeyPart::U64(u64::MAX),
                DecodedKeyPart::I64(i64::MIN),
                DecodedKeyPart::DescU64(42),
            ]
        );
    }
}

#[test]
fn address_and_component_boundaries_are_isolated() {
    let codec = KeyCodec;
    let key = |tenant, scope, subject, predicate| {
        codec.encode(
            KeyAddress { tenant, scope },
            KeyFamily::Temporal,
            &[
                KeyPart::Text(subject),
                KeyPart::Text(predicate),
                KeyPart::DescU64(10),
            ],
        )
    };
    assert_ne!(
        key(Some("tenant-a"), Some("estate"), "a/b", "c"),
        key(Some("tenant-b"), Some("estate"), "a/b", "c")
    );
    assert_ne!(
        key(Some("tenant-a"), Some("estate-a"), "a/b", "c"),
        key(Some("tenant-a"), Some("estate-b"), "a/b", "c")
    );
    assert_ne!(
        key(Some("tenant-a"), Some("estate"), "a/b", "c"),
        key(Some("tenant-a"), Some("estate"), "a", "b/c")
    );
    assert_ne!(
        key(None, Some("estate"), "a", "b"),
        key(Some(""), Some("estate"), "a", "b")
    );
}

#[test]
fn byte_text_numeric_and_version_order_is_memcomparable() {
    let codec = KeyCodec;
    let base = KeyAddress::GLOBAL;
    let encoded = |part| codec.encode(base, KeyFamily::Scalar, &[part]);

    assert!(encoded(KeyPart::Text("")) < encoded(KeyPart::Text("\0")));
    assert!(encoded(KeyPart::Text("a")) < encoded(KeyPart::Text("aa")));
    assert!(encoded(KeyPart::Text("aa")) < encoded(KeyPart::Text("b")));
    assert!(encoded(KeyPart::Bytes(&[0xff])) < encoded(KeyPart::Bytes(&[0xff, 0])));
    assert!(encoded(KeyPart::I64(i64::MIN)) < encoded(KeyPart::I64(-1)));
    assert!(encoded(KeyPart::I64(-1)) < encoded(KeyPart::I64(0)));
    assert!(encoded(KeyPart::I64(0)) < encoded(KeyPart::I64(i64::MAX)));
    assert!(encoded(KeyPart::U64(9)) < encoded(KeyPart::U64(10)));
    assert!(encoded(KeyPart::DescU64(10)) < encoded(KeyPart::DescU64(9)));
}

#[test]
fn component_prefix_has_an_exact_exclusive_range() {
    let codec = KeyCodec;
    let address = KeyAddress {
        tenant: Some("tenant-a"),
        scope: Some("estate-a"),
    };
    let prefix = codec.encode(
        address,
        KeyFamily::OutgoingEdge,
        &[KeyPart::Text("module"), KeyPart::Text("src/lib.rs")],
    );
    let end = prefix_end(&prefix).unwrap();
    let inside = codec.encode(
        address,
        KeyFamily::OutgoingEdge,
        &[
            KeyPart::Text("module"),
            KeyPart::Text("src/lib.rs"),
            KeyPart::Text("imports"),
            KeyPart::Text("src/key.rs"),
        ],
    );
    let neighbour = codec.encode(
        address,
        KeyFamily::OutgoingEdge,
        &[KeyPart::Text("module"), KeyPart::Text("src/lib.rsx")],
    );
    assert!(inside >= prefix && inside < end);
    assert!(neighbour < prefix || neighbour >= end);
}

#[test]
fn prefix_end_handles_every_final_byte_and_trailing_ff() {
    for byte in 0_u8..=u8::MAX {
        let prefix = [0x42, byte];
        let end = prefix_end(&prefix);
        if byte == u8::MAX {
            assert_eq!(end, Some(vec![0x43]));
        } else {
            assert_eq!(end, Some(vec![0x42, byte + 1]));
        }
    }
    for penultimate in 0_u8..=u8::MAX {
        let prefix = [penultimate, 0xff, 0xff];
        let end = prefix_end(&prefix);
        if penultimate == u8::MAX {
            assert_eq!(end, None);
        } else {
            assert_eq!(end, Some(vec![penultimate + 1]));
        }
    }
    assert_eq!(prefix_end(&[]), None);
}

#[test]
fn malformed_keys_fail_closed() {
    let codec = KeyCodec;
    let valid = codec.encode(
        KeyAddress::GLOBAL,
        KeyFamily::Current,
        &[KeyPart::Text("record")],
    );
    let component_start = codec
        .encode(KeyAddress::GLOBAL, KeyFamily::Current, &[])
        .len();
    for truncated in component_start + 1..valid.len() {
        assert!(codec.decode(&valid[..truncated]).is_err());
    }

    let mut wrong_format = valid.clone();
    wrong_format[8] ^= 1;
    assert!(codec.decode(&wrong_format).is_err());

    let mut unknown_family = codec.encode(KeyAddress::GLOBAL, KeyFamily::Current, &[]);
    *unknown_family.last_mut().unwrap() = 0xff;
    assert!(codec.decode(&unknown_family).is_err());

    let mut invalid_bool = codec.encode(
        KeyAddress::GLOBAL,
        KeyFamily::Current,
        &[KeyPart::Bool(true)],
    );
    *invalid_bool.last_mut().unwrap() = 2;
    assert!(codec.decode(&invalid_bool).is_err());

    let mut invalid_padding = codec.encode(
        KeyAddress::GLOBAL,
        KeyFamily::Current,
        &[KeyPart::Bytes(b"a")],
    );
    let padding_byte = invalid_padding.len() - 2;
    invalid_padding[padding_byte] = 1;
    assert!(codec.decode(&invalid_padding).is_err());

    let mut unknown_type = codec.encode(KeyAddress::GLOBAL, KeyFamily::Current, &[]);
    unknown_type.push(0xee);
    assert!(codec.decode(&unknown_type).is_err());
}

#[test]
fn frozen_family_vectors_match_the_checked_in_fixture() {
    let codec = KeyCodec;
    let address = KeyAddress {
        tenant: Some("tenant-a"),
        scope: Some("estate-a"),
    };
    let vectors = [
        (
            "current",
            KeyFamily::Current,
            vec![
                KeyPart::U8(1),
                KeyPart::Text("document"),
                KeyPart::Text("readme"),
            ],
        ),
        (
            "temporal",
            KeyFamily::Temporal,
            vec![
                KeyPart::U8(1),
                KeyPart::Text("claim"),
                KeyPart::Text("status"),
                KeyPart::DescU64(100),
                KeyPart::DescU64(200),
            ],
        ),
        (
            "outgoing_edge",
            KeyFamily::OutgoingEdge,
            vec![
                KeyPart::Text("file:a"),
                KeyPart::Text("imports"),
                KeyPart::Text("file:b"),
            ],
        ),
        (
            "incoming_edge",
            KeyFamily::IncomingEdge,
            vec![
                KeyPart::Text("file:b"),
                KeyPart::Text("imports"),
                KeyPart::Text("file:a"),
            ],
        ),
        (
            "scalar",
            KeyFamily::Scalar,
            vec![
                KeyPart::Text("by_size"),
                KeyPart::I64(-7),
                KeyPart::Text("file:a"),
            ],
        ),
        (
            "unique",
            KeyFamily::Unique,
            vec![KeyPart::Text("by_path"), KeyPart::Text("src/lib.rs")],
        ),
        (
            "term_dictionary",
            KeyFamily::TermDictionary,
            vec![KeyPart::Text("source"), KeyPart::Text("arrow")],
        ),
        (
            "term_statistic",
            KeyFamily::TermStatistic,
            vec![KeyPart::Text("source"), KeyPart::U64(42)],
        ),
        (
            "term_posting",
            KeyFamily::TermPosting,
            vec![
                KeyPart::Text("source"),
                KeyPart::Text("arrow"),
                KeyPart::Text("file:a"),
            ],
        ),
        (
            "vector",
            KeyFamily::Vector,
            vec![KeyPart::Text("code"), KeyPart::Text("file:a")],
        ),
        (
            "projection_delta",
            KeyFamily::ProjectionDelta,
            vec![KeyPart::Text("hnsw:code"), KeyPart::U64(9), KeyPart::U64(1)],
        ),
        (
            "catalogue_artifact",
            KeyFamily::Catalogue,
            vec![
                KeyPart::U8(CatalogueSubfamily::FunctionArtifact as u8),
                KeyPart::Bytes(&[0xaa, 0xbb]),
            ],
        ),
        (
            "catalogue_definition",
            KeyFamily::Catalogue,
            vec![
                KeyPart::U8(CatalogueSubfamily::FunctionDefinition as u8),
                KeyPart::Text("lint"),
            ],
        ),
        (
            "catalogue_binding",
            KeyFamily::Catalogue,
            vec![
                KeyPart::U8(CatalogueSubfamily::TransactionFunctionBinding as u8),
                KeyPart::Text("on_commit"),
            ],
        ),
        (
            "catalogue_membership",
            KeyFamily::Catalogue,
            vec![
                KeyPart::U8(CatalogueSubfamily::Membership as u8),
                KeyPart::U64(3),
            ],
        ),
        (
            "catalogue_head",
            KeyFamily::Catalogue,
            vec![KeyPart::U8(CatalogueSubfamily::Head as u8)],
        ),
        (
            "catalogue_invocation",
            KeyFamily::Catalogue,
            vec![
                KeyPart::U8(CatalogueSubfamily::InvocationReceipt as u8),
                KeyPart::U64(5),
            ],
        ),
        (
            "runtime_commit",
            KeyFamily::RuntimeCommit,
            vec![KeyPart::Bytes(&[0xcc, 0xdd])],
        ),
        (
            "outbox",
            KeyFamily::Outbox,
            vec![KeyPart::U64(9), KeyPart::U64(1)],
        ),
        (
            "audit",
            KeyFamily::Audit,
            vec![KeyPart::Bytes(&[0xee, 0xff])],
        ),
    ];
    let computed = vectors
        .iter()
        .map(|(name, family, parts)| {
            format!("{name} {}", hex(&codec.encode(address, *family, parts)))
        })
        .collect::<Vec<_>>()
        .join("\n");
    let expected = include_str!("../../fixtures/rrflow-kv-key-codec-v1.hex").trim_end();
    assert_eq!(computed, expected);
}
