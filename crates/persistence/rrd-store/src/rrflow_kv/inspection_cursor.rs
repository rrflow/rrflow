//! Bounded physical inspection; only hashes/counters cross the store boundary.
use super::*;
use rrd_lsm::{
    ProjectedReadBatch, ProjectedReadBudget, ProjectedReadProjection, ProjectedReadRange,
    ProjectedReadRequest, ProjectedReadStream,
};
use sha2::{Digest, Sha256};

pub struct StorageInspectionCursor {
    stream: Option<ProjectedReadStream>,
    batch: Option<ProjectedReadBatch>,
    sequence: u64,
    ordinal: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StorageInspectionPage {
    pub sequence: u64,
    pub ordinal: u64,
    pub rows: u64,
    pub allocated_bytes: u64,
    pub sha256: String,
    pub source_open: bool,
}

impl RrflowKvStore {
    pub fn inspect_storage(&self, rows: usize, bytes: usize) -> Result<StorageInspectionCursor> {
        if rows == 0 || rows > 4096 || bytes == 0 || bytes > 4 * 1024 * 1024 {
            return Err(Error::Substrate(
                "invalid storage inspection batch budget".into(),
            ));
        }
        let database = self.lock()?;
        let snapshot = database.snapshot();
        let bytes = bytes.min(database.projected_output_memory().stats().limit_bytes);
        let stream = database.begin_projected_read(ProjectedReadRequest {
            ranges: vec![ProjectedReadRange::all()],
            projection: ProjectedReadProjection::KeyValue,
            snapshot,
            budget: ProjectedReadBudget {
                max_batch_rows: rows,
                max_batch_allocated_bytes: bytes,
                max_output_buffer_bytes: bytes as u64,
                ..Default::default()
            },
        })?;
        // No store lock is held during retained lifetime or subsequent reads.
        drop(database);
        let mut cursor = StorageInspectionCursor {
            stream: Some(stream),
            batch: None,
            sequence: snapshot.sequence,
            ordinal: 0,
        };
        cursor.read_next()?;
        Ok(cursor)
    }
}

impl StorageInspectionCursor {
    /// Recomputes the digest from the still-owned buffers, not a cached hash.
    pub fn page(&self) -> Result<StorageInspectionPage> {
        let mut digest = Sha256::new();
        digest.update(b"rrflow-storage-inspection-page-v1");
        digest.update(self.sequence.to_be_bytes());
        digest.update(self.ordinal.to_be_bytes());
        let rows = self.batch.as_ref().map_or(0, |batch| batch.len() as u64);
        digest.update(rows.to_be_bytes());
        if let Some(batch) = &self.batch {
            for row in 0..batch.len() {
                for bytes in [batch.key(row), batch.value(row)] {
                    let bytes = bytes
                        .ok_or_else(|| Error::Substrate("inspection column missing".into()))?;
                    digest.update((bytes.len() as u64).to_be_bytes());
                    digest.update(bytes);
                }
            }
        }
        Ok(StorageInspectionPage {
            sequence: self.sequence,
            ordinal: self.ordinal,
            rows,
            allocated_bytes: self
                .batch
                .as_ref()
                .map_or(0, |b| b.allocated_bytes() as u64),
            sha256: digest
                .finalize()
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect(),
            source_open: self.stream.is_some(),
        })
    }
    fn read_next(&mut self) -> Result<()> {
        self.batch = None;
        if let Some(stream) = &mut self.stream {
            self.batch = stream.next_batch()?;
            if self.batch.is_none()
                || stream.evidence().outcome == rrd_lsm::ProjectedReadOutcome::Completed
            {
                self.stream = None;
            }
        }
        Ok(())
    }
    pub fn advance(&mut self, expected: u64) -> Result<()> {
        if expected.checked_add(1) == Some(self.ordinal) {
            return Ok(());
        }
        if expected != self.ordinal || self.stream.is_none() {
            return Err(Error::Substrate(
                "stale or terminal storage inspection page".into(),
            ));
        }
        let next = self.ordinal.checked_add(1).ok_or(Error::SequenceOverflow)?;
        self.read_next()?;
        self.ordinal = next;
        Ok(())
    }
    /// Releases native source leases while retaining the copied batch.
    pub fn cancel_source(&mut self) -> Result<()> {
        if let Some(mut stream) = self.stream.take() {
            stream.cancel()?;
        }
        Ok(())
    }
}
