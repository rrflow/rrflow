use super::*;
use std::collections::BTreeMap;

fn fixture(keys: usize, value_bytes: usize) -> BTreeMap<Vec<u8>, Vec<VersionedValue>> {
    let mut output = BTreeMap::new();
    for key in 0..keys {
        let mut values = Vec::new();
        for generation in 0..3 {
            let value = if generation == 1 && key % 5 == 0 {
                None
            } else {
                let mut bytes = vec![b'a' + (key % 20) as u8; value_bytes + key % 97];
                let mut random = (key as u64 + 1) * (generation + 1);
                for byte in bytes.iter_mut().step_by(7) {
                    random = random.wrapping_mul(6364136223846793005).wrapping_add(1);
                    *byte = (random >> 32) as u8;
                }
                Some(bytes.into_boxed_slice())
            };
            values.push(VersionedValue {
                sequence: generation + 1,
                value,
            });
        }
        output.insert(format!("key-{key:08}").into_bytes(), values);
    }
    output
}

#[test]
fn segment_v6_encoding_matches_pre_reuse_bytes() {
    let values = fixture(80, 311);
    // Captured from the materialized v6 encoder before buffer reuse. These bind
    // full bytes, boundaries, statistics, raw fallback, tombstones and history.
    let expected = [
        (
            158176,
            "f4901ea81e3f7e6119c3d979e22e6e3263518a9d88defd4bff39eee2e9c7a030",
        ),
        (
            123168,
            "4d4e5cd9e44c98ce1a049342b0969107a8e504b0d63d5dc356defb8fee68d275",
        ),
        (
            158176,
            "c95a56a0d3d0cde89a50e3c5899fc7a2943568fc9f5f6cf41fcb3b90ec05c728",
        ),
        (
            121120,
            "1559b20eb507e3caef9fc92a6a8429e83734f69f615ecac140ba9c8cd138aac9",
        ),
        (
            83488,
            "b4d8203998b66ec3c64e617013802fe522cc33f294f3e5258e048b0688299d30",
        ),
        (
            121120,
            "00327dad72df699b98c125411c5b24c973b06bfd3d89512a122ef7383c6e7898",
        ),
    ];
    let mut expected = expected.into_iter();
    for target_bytes in [1024, 65536] {
        for policy in [
            SegmentCompressionPolicy::None,
            SegmentCompressionPolicy::default(),
            SegmentCompressionPolicy::AdaptiveLz4 {
                minimum_savings_basis_points: 9999,
                maximum_page_logical_bytes: 512,
            },
        ] {
            let encoded = encode_versions(
                values
                    .iter()
                    .map(|(key, versions)| (key.as_slice(), versions.as_slice())),
                SegmentRowGroupBudget {
                    target_bytes,
                    max_rows: 7,
                },
                policy,
            )
            .unwrap();
            let (bytes, checksum) = expected.next().unwrap();
            assert_eq!(encoded.bytes.len(), bytes);
            assert_eq!(encoded.checksum, checksum);
        }
    }
    assert!(expected.next().is_none());
}

#[test]
#[ignore = "explicit native encoder characterization, not capacity qualification"]
fn encoding_working_set_profile() {
    let values = fixture(2048, 4096);
    let started = std::time::Instant::now();
    let mut bytes = 0usize;
    let mut checksum = String::new();
    for _ in 0..32 {
        let encoded = encode_versions(
            values
                .iter()
                .map(|(key, versions)| (key.as_slice(), versions.as_slice())),
            SegmentRowGroupBudget::default(),
            SegmentCompressionPolicy::default(),
        )
        .unwrap();
        bytes += std::hint::black_box(encoded.bytes.len());
        checksum = encoded.checksum;
    }
    println!("encoder_profile={{\"iterations\":32,\"keys\":2048,\"versions_per_key\":3,\"bytes\":{bytes},\"elapsed_ms\":{},\"checksum\":\"{checksum}\"}}", started.elapsed().as_millis());
}
