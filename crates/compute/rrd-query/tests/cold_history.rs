use bytes::Bytes;
use parquet::{arrow::arrow_reader::ParquetRecordBatchReaderBuilder, basic::Compression};
use rrd_core::{
    ReadStamp, RuntimeChange, RuntimeCommit, RuntimeLogicalModel, RuntimeMutation, RuntimeRecord,
    RuntimeRef, RuntimeRetirement, RuntimeSchemaRegistry, RuntimeTableSchema, RuntimeType,
    RuntimeValue, ScopeId,
};
use rrd_query::{
    decode_cold_history_page, encode_cold_history_page, record_batch_to_rows,
    ColdHistoryReadExpectation, ColdHistoryReadLimits, ParquetCompression, ParquetEncoding,
    ParquetExportLimits,
};
use rrd_store::{RrflowKvStore, StorageEngine};
use std::{
    collections::BTreeMap,
    time::{Duration, Instant},
};

fn limits() -> ParquetExportLimits {
    ParquetExportLimits {
        max_rows: 32,
        max_columns: 16,
        max_input_bytes: 1024 * 1024,
        max_output_bytes: 1024 * 1024,
        row_group_rows: 2,
        max_memory_bytes: 64 * 1024 * 1024,
        deadline: Instant::now() + Duration::from_secs(30),
    }
}

fn encoding() -> ParquetEncoding {
    ParquetEncoding {
        compression: ParquetCompression::Zstd { level: 3 },
        dictionary: true,
        statistics: true,
    }
}

#[test]
fn stamped_native_history_encodes_compressed_rows_and_survives_reopen() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("engram");
    let store = RrflowKvStore::open(&path).unwrap();
    let scope = ScopeId::new("instance:cold").unwrap();
    let mut registry = RuntimeSchemaRegistry::empty(1, "cold history fixture");
    registry.tables.insert(
        RuntimeType::new("document").unwrap(),
        RuntimeTableSchema::schemaless(RuntimeLogicalModel::Relational),
    );
    let reference = RuntimeRef::new("document", "note-one").unwrap();
    let record = |value: &str| RuntimeMutation::Record {
        record: RuntimeRecord {
            reference: reference.clone(),
            valid_from: 10,
            valid_to: None,
            properties: BTreeMap::from([("body".into(), RuntimeValue::String(value.into()))]),
        },
    };
    let commit = |expected_cursor, mutations| {
        store
            .runtime()
            .commit(&RuntimeCommit {
                scope: scope.clone(),
                at: 100 + expected_cursor,
                actor: "cold-test".into(),
                expected_cursor,
                mutations,
            })
            .unwrap()
    };
    commit(
        0,
        vec![RuntimeMutation::Schema { registry }, record("first")],
    );
    commit(2, vec![record("updated")]);
    commit(
        3,
        vec![RuntimeMutation::Retire {
            retirement: RuntimeRetirement {
                model: RuntimeLogicalModel::Relational,
                reference,
                effective_at: 20,
            },
        }],
    );
    let snapshot = store
        .runtime()
        .open_snapshot(&scope, "cold-test", 100, 10_000)
        .unwrap();
    let page = store
        .runtime()
        .snapshot_changes(&snapshot, 0, 32, 101)
        .unwrap();
    assert_eq!(page.changes.len(), 4);
    let expected = page.changes.clone();
    let artifact = encode_cold_history_page(&snapshot.read, &page, limits(), encoding()).unwrap();
    assert_eq!(artifact.artifact.row_groups, 2);
    let bytes = artifact.artifact.bytes;
    let expected_read = ColdHistoryReadExpectation {
        stamp: snapshot.read.clone(),
        requested_after: page.requested_after,
        through_cursor: page.through_cursor,
        rows: page.changes.len() as u64,
        artifact_sha256: rrd_core::digest::sha256_hex(&bytes),
    };
    let read_limits = ColdHistoryReadLimits {
        max_rows: 32,
        max_encoded_bytes: 1024 * 1024,
        max_decoded_bytes: 1024 * 1024,
        max_memory_bytes: 64 * 1024 * 1024,
        max_batch_rows: 2,
        deadline: Instant::now() + Duration::from_secs(30),
    };
    let decoded = decode_cold_history_page(bytes.clone(), &expected_read, read_limits).unwrap();
    assert_eq!(decoded.row_groups, 2);
    assert_eq!(decoded.changes, expected);
    // A global cursor interval can contain no rows for the selected scope.
    // This exercises the codec's empty-page representation, not completeness.
    let mut empty = page.clone();
    empty.changes.clear();
    let empty_artifact =
        encode_cold_history_page(&snapshot.read, &empty, limits(), encoding()).unwrap();
    let empty_expectation = ColdHistoryReadExpectation {
        stamp: snapshot.read.clone(),
        requested_after: empty.requested_after,
        through_cursor: empty.through_cursor,
        rows: 0,
        artifact_sha256: rrd_core::digest::sha256_hex(&empty_artifact.artifact.bytes),
    };
    let empty_decoded = decode_cold_history_page(
        empty_artifact.artifact.bytes,
        &empty_expectation,
        read_limits,
    )
    .unwrap();
    assert!(empty_decoded.changes.is_empty());
    let mut tampered = bytes.clone();
    tampered[8] ^= 1;
    assert!(decode_cold_history_page(tampered, &expected_read, read_limits).is_err());
    assert!(decode_cold_history_page(
        bytes.clone(),
        &expected_read,
        ColdHistoryReadLimits {
            max_decoded_bytes: 1,
            ..read_limits
        }
    )
    .is_err());
    drop(store);
    let store = RrflowKvStore::open(&path).unwrap();
    assert_eq!(
        store
            .runtime()
            .snapshot_changes(&snapshot, 0, 32, 102)
            .unwrap()
            .changes,
        expected
    );
    let builder = ParquetRecordBatchReaderBuilder::try_new(Bytes::from(bytes)).unwrap();
    let metadata = builder.metadata();
    assert_eq!(metadata.num_row_groups(), 2);
    assert!(matches!(
        metadata.row_group(0).column(0).compression(),
        Compression::ZSTD(_)
    ));
    let fields = metadata.file_metadata().key_value_metadata().unwrap();
    let source = fields
        .iter()
        .find(|entry| entry.key == "rrflow.cold.read_stamp")
        .unwrap()
        .value
        .as_ref()
        .unwrap();
    assert_eq!(
        serde_json::from_str::<ReadStamp>(source).unwrap(),
        snapshot.read
    );
    let restored = builder
        .build()
        .unwrap()
        .flat_map(|batch| record_batch_to_rows(&batch.unwrap()).unwrap())
        .map(|row| {
            let RuntimeValue::String(json) = &row.values["change_json"] else {
                panic!("history value")
            };
            serde_json::from_str::<RuntimeChange>(json).unwrap()
        })
        .collect::<Vec<_>>();
    assert_eq!(restored, expected);

    let mut corrupt = page.clone();
    corrupt.changes[0].digest = "0".repeat(64);
    assert!(encode_cold_history_page(&snapshot.read, &corrupt, limits(), encoding()).is_err());
    let mut wrong_scope = page.clone();
    wrong_scope.changes[0].scope = ScopeId::new("instance:other").unwrap();
    assert!(encode_cold_history_page(&snapshot.read, &wrong_scope, limits(), encoding()).is_err());
    assert!(encode_cold_history_page(
        &snapshot.read,
        &page,
        ParquetExportLimits {
            max_input_bytes: 32,
            ..limits()
        },
        encoding()
    )
    .is_err());
    assert!(encode_cold_history_page(
        &snapshot.read,
        &page,
        limits(),
        ParquetEncoding {
            compression: ParquetCompression::Zstd { level: i32::MAX },
            ..encoding()
        }
    )
    .is_err());
}
