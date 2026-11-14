//! Worker packets, source captures, authority and readback context.

use super::*;

pub(super) const ACTIONS: [SecurityAction; 4] = [
    SecurityAction::WorkerExecute,
    SecurityAction::TaskRead,
    SecurityAction::ReasoningRead,
    SecurityAction::ReasoningWrite,
];

#[derive(Serialize)]
pub(super) struct WorkerPacket {
    pub(super) contract_version: u16,
    pub(super) task: EndpointSelectionTask,
    pub(super) selection_sha256: String,
    pub(super) choice: EndpointSelectionChoice,
    pub(super) configuration_sha256: String,
    pub(super) read: ContextReadStamp,
    pub(super) context: Option<ContextPacket>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) source: Option<WorkerSourcePacket>,
}

#[derive(Clone, Serialize)]
pub(in super::super) struct WorkerSourcePacket {
    pub(in super::super) content: String,
    pub(in super::super) provenance: TaskWorkerSourceProvenance,
}

pub(in super::super) struct WorkerSourceCapture {
    pub(in super::super) packet: WorkerSourcePacket,
    pub(in super::super) head_key: String,
    pub(in super::super) head_bytes: Vec<u8>,
}

impl WorkerSourceCapture {
    pub(in super::super) fn provenance(&self) -> &TaskWorkerSourceProvenance {
        &self.packet.provenance
    }
}

pub(super) struct WorkerAuthority {
    pub(super) guards: Vec<(AttemptAuthorityGuard, TaskAttemptInvalidation)>,
}

pub(super) struct SelectedSourceReadbackContext<'a> {
    pub(super) session: &'a CorrelationId,
    pub(super) token: &'a CorrelationId,
    pub(super) now: u64,
    pub(super) request_id: &'a str,
    pub(super) operation_id: &'a str,
}
