//! One selected model activity under the existing task attempt transaction.
//! Accepted output is a draft, never a project mutation or correctness label.
use super::clock::{observe_required, ClockSource, HostSystemClock};
use super::endpoint_proposal::{capacity::WorkerCapacityLease, endpoint_failure};
use super::reasoning_budget::{
    ActiveTaskCancellation, AttemptAuthorityGuard, TaskAttemptAdmission, TaskAttemptOutput,
    TaskCompletionAdmission,
};
use super::*;
use rrd_contract::{
    ContextPacket, ContextReadStamp, EndpointSelectionChoice, EndpointSelectionTask, ReadTask,
    ReadTaskAttempt, TaskAttemptInvalidation, TaskAttemptStatus, TaskWorkerContext,
    TaskWorkerOutput, TaskWorkerRequest, TaskWorkerSourceProvenance, TaskWorkerSourceRequest,
    ENDPOINT_WORKER_VERSION, TASK_WORKER_SOURCE_VERSION,
};
use rrd_inference::endpoint_chat::{ChatMessage, ChatRole, ModelChatRequest, PreparedModelChat};
use rrd_inference::{EmbeddingCancellation, NetworkPolicy};

mod execute;
mod packet;
mod support;

pub(super) use packet::*;
pub(super) use support::*;

#[cfg(test)]
mod legacy_fingerprint_tests {
    use super::*;

    #[test]
    fn no_source_worker_fingerprint_keeps_pre_source_tuple_bytes() {
        let packet = WorkerPacket {
            contract_version: ENDPOINT_WORKER_VERSION,
            task: EndpointSelectionTask {
                scope: "instance:test".into(),
                task_id: CanonicalId::new("task-a").unwrap(),
                title: "Review".into(),
                goal: "Inspect".into(),
                task_sha256: "1".repeat(64),
                known_at_cursor: 7,
            },
            selection_sha256: "2".repeat(64),
            choice: EndpointSelectionChoice::Select {
                binding_id: CanonicalId::new("binding-a").unwrap(),
                effort: None,
            },
            configuration_sha256: "3".repeat(64),
            read: ContextReadStamp {
                runtime_manifest_sha256: "4".repeat(64),
                runtime_cursor: 8,
                schema_revision: None,
                catalogue_revision: 1,
            },
            context: None,
            source: None,
        };
        let request = TaskWorkerRequest {
            scope: "instance:test".into(),
            task_id: CanonicalId::new("task-a").unwrap(),
            attempt_id: rrd_contract::CorrelationId::new("attempt-a").unwrap(),
            selection_attempt_id: rrd_contract::CorrelationId::new("selection-a").unwrap(),
            expected_selection_sha256: "2".repeat(64),
            expected_task_sha256: "1".repeat(64),
            expected_configuration_sha256: "3".repeat(64),
            max_storage_keys: 100,
            context: None,
            source: None,
        };
        let legacy = (
            "task-worker",
            &packet.task,
            &packet.configuration_sha256,
            &packet.choice,
            &request.context,
            None::<(&Vec<rrd_contract::ContextItem>, bool)>,
        );
        let bytes = serde_json::to_vec(&legacy).unwrap();
        let expected_bytes = br#"["task-worker",{"scope":"instance:test","task_id":"task-a","title":"Review","goal":"Inspect","task_sha256":"1111111111111111111111111111111111111111111111111111111111111111","known_at_cursor":7},"3333333333333333333333333333333333333333333333333333333333333333",{"kind":"select","binding_id":"binding-a","effort":null},null,null]"#;
        assert_eq!(bytes, expected_bytes);
        assert_eq!(
            equivalent_worker_work_digest(&packet, &request, None).unwrap(),
            digest::sha256_hex(expected_bytes)
        );
    }

    #[test]
    fn source_repetition_fingerprint_ignores_global_policy_and_snapshot_refreshes() {
        let packet = WorkerPacket {
            contract_version: ENDPOINT_WORKER_VERSION,
            task: EndpointSelectionTask {
                scope: "instance:test".into(),
                task_id: CanonicalId::new("task-a").unwrap(),
                title: "Review".into(),
                goal: "Inspect".into(),
                task_sha256: "1".repeat(64),
                known_at_cursor: 7,
            },
            selection_sha256: "2".repeat(64),
            choice: EndpointSelectionChoice::Select {
                binding_id: CanonicalId::new("binding-a").unwrap(),
                effort: None,
            },
            configuration_sha256: "3".repeat(64),
            read: ContextReadStamp {
                runtime_manifest_sha256: "4".repeat(64),
                runtime_cursor: 8,
                schema_revision: None,
                catalogue_revision: 1,
            },
            context: None,
            source: None,
        };
        let request = TaskWorkerRequest {
            scope: "instance:test".into(),
            task_id: CanonicalId::new("task-a").unwrap(),
            attempt_id: rrd_contract::CorrelationId::new("attempt-a").unwrap(),
            selection_attempt_id: rrd_contract::CorrelationId::new("selection-a").unwrap(),
            expected_selection_sha256: "2".repeat(64),
            expected_task_sha256: "1".repeat(64),
            expected_configuration_sha256: "3".repeat(64),
            max_storage_keys: 100,
            context: None,
            source: None,
        };
        let capture = |snapshot: &str, policy: &str, at| WorkerSourceCapture {
            packet: WorkerSourcePacket {
                content: "selected source bytes".into(),
                provenance: TaskWorkerSourceProvenance {
                    contract_version: TASK_WORKER_SOURCE_VERSION,
                    captured_snapshot_sha256: snapshot.into(),
                    captured_read: ContextReadStamp {
                        runtime_manifest_sha256: "5".repeat(64),
                        runtime_cursor: 12,
                        schema_revision: None,
                        catalogue_revision: 2,
                    },
                    project_id: CanonicalId::new("project-a").unwrap(),
                    entry_identity: "6".repeat(64),
                    path_sha256: "7".repeat(64),
                    content_sha256: digest::sha256_hex(b"selected source bytes"),
                    policy_sha256: policy.into(),
                    profile_sha256: "8".repeat(64),
                    captured_at_unix_ms: at,
                    validated_snapshot_sha256: "9".repeat(64),
                    validated_read: ContextReadStamp {
                        runtime_manifest_sha256: "a".repeat(64),
                        runtime_cursor: 12,
                        schema_revision: None,
                        catalogue_revision: 2,
                    },
                    validated_at_unix_ms: at + 1,
                },
            },
            head_key: "head".into(),
            head_bytes: vec![1],
        };
        let before = capture(&"b".repeat(64), &"c".repeat(64), 10);
        let after = capture(&"d".repeat(64), &"e".repeat(64), 20);
        assert_eq!(
            equivalent_worker_work_digest(&packet, &request, Some(&before)).unwrap(),
            equivalent_worker_work_digest(&packet, &request, Some(&after)).unwrap()
        );
    }
}
