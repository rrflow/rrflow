use super::*;
use rrd_contract::{
    DiagnosticMaintenanceSnapshot, DiagnosticMemtableSnapshot, DiagnosticPageCacheSnapshot,
    DiagnosticRrflowCldSnapshot, DiagnosticRrflowKvSnapshot, DiagnosticSegmentIoMode,
    DiagnosticSegmentIoSnapshot, DiagnosticSegmentSetSnapshot, DiagnosticStorageAdmission,
    DiagnosticStorageEvidenceLevel, DiagnosticStorageSnapshot, DiagnosticWriteAheadLogSnapshot,
    MAX_MESSAGE_BYTES,
};
use rrd_store::PhysicalStoreEvidence;

mod assemble;
mod graph;
mod sections;
mod storage;

use assemble::*;
use graph::*;
use sections::*;
use storage::*;

#[cfg(test)]
mod storage_tests {
    use super::*;

    #[test]
    fn rrflow_kv_projection_fails_closed_on_a_missing_counter() {
        let evidence = PhysicalStoreEvidence::logical_only("rrflow_kv");
        let error = rrflow_kv_snapshot(
            &evidence,
            &crate::RrflowCldSnapshot {
                trigger_count: 0,
                cold_source_hits: 0,
                cold_mapped_hits: 0,
                cold_owned_hits: 0,
                cache_bypass_hits: 0,
                last_trigger_at_unix_ms: None,
            },
        )
        .unwrap_err();
        assert!(error.to_string().contains("physical_sequence"));
    }

    #[test]
    fn segment_io_modes_map_exactly() {
        for (text, mode) in [
            ("auto", DiagnosticSegmentIoMode::Auto),
            ("mmap", DiagnosticSegmentIoMode::Mmap),
            ("io_uring", DiagnosticSegmentIoMode::IoUring),
            ("bounded", DiagnosticSegmentIoMode::Bounded),
        ] {
            assert_eq!(segment_io_mode(Some(text)).unwrap(), mode);
        }
        assert!(segment_io_mode(Some("uring")).is_err());
        assert!(segment_io_mode(None).is_err());
    }

    #[test]
    fn fallback_reasons_are_bounded_contract_messages() {
        assert_eq!(bounded_reason(None), None);
        assert_eq!(bounded_reason(Some(&String::new())), None);
        assert_eq!(
            bounded_reason(Some(&"a\0b".to_owned())),
            Some("ab".to_owned())
        );
        let long = "é".repeat(MAX_MESSAGE_BYTES);
        let bounded = bounded_reason(Some(&long)).unwrap();
        assert!(bounded.len() <= MAX_MESSAGE_BYTES);
        assert!(bounded.chars().all(|character| character == 'é'));
    }
}

#[cfg(test)]
mod bounded_graph_tests {
    use super::*;

    fn reference(id: &str) -> RuntimeRef {
        RuntimeRef::new("document", id).unwrap()
    }

    fn graph(records: &[&str], relations: &[(&str, &str)]) -> RuntimeGraphSnapshot {
        RuntimeGraphSnapshot {
            scope: rrd_core::ScopeId::new("instance:test").unwrap(),
            valid_at: 1,
            known_at_cursor: 4,
            records: records
                .iter()
                .map(|id| RuntimeRecord {
                    reference: reference(id),
                    valid_from: 1,
                    valid_to: None,
                    properties: Default::default(),
                })
                .collect(),
            relations: relations
                .iter()
                .map(|(from, to)| RuntimeRelation {
                    reference: RuntimeRef::new("links", format!("{from}-{to}")).unwrap(),
                    from: reference(from),
                    to: reference(to),
                    valid_from: 1,
                    valid_to: None,
                    properties: Default::default(),
                })
                .collect(),
        }
    }

    fn ids(snapshot: &DiagnosticGraphSnapshot) -> Vec<String> {
        snapshot
            .records
            .iter()
            .map(|record| record.reference.id.clone())
            .collect()
    }

    #[test]
    fn a_bounded_read_returns_the_most_linked_records_and_counts_the_rest() {
        let after = graph(&["a", "b", "c", "d"], &[("a", "b"), ("a", "c")]);
        let keep = bounded_records(&after, Some(2));
        let public = public_graph_snapshot("instance:test", &after, keep.as_ref()).unwrap();
        assert_eq!(
            ids(&public),
            ["a", "b"],
            "a has two links; b wins the tie by reference"
        );
        assert_eq!(
            public.relations.len(),
            1,
            "only the relation between returned records"
        );
        assert_eq!((public.records_total, public.relations_total), (4, 2));

        let empty = graph(&[], &[]);
        let difference = public_graph_difference(1, &empty.diff(&after), keep.as_ref()).unwrap();
        assert_eq!(difference.added_records.len(), 2);
        assert_eq!(difference.added_relations.len(), 1);
        assert_eq!(difference.omitted_entries, 3);
    }

    #[test]
    fn an_unbounded_or_roomy_read_leaves_nothing_out() {
        let after = graph(&["a", "b"], &[("a", "b")]);
        assert!(bounded_records(&after, None).is_none());
        assert!(bounded_records(&after, Some(2)).is_none());
        let public = public_graph_snapshot("instance:test", &after, None).unwrap();
        assert_eq!((public.records.len(), public.records_total), (2, 2));
        let difference = public_graph_difference(1, &graph(&[], &[]).diff(&after), None).unwrap();
        assert_eq!(difference.omitted_entries, 0);
    }
}
