//! Ordered bounded overlay over the canonical physical projected reader.
use super::Transaction;
use crate::{
    Database, Error, ProjectedReadBatch, ProjectedReadBudget, ProjectedReadProjection,
    ProjectedReadRange, ProjectedReadRequest, ProjectedReadResource, ProjectedReadStream, Result,
};
use std::cmp::Ordering;
use std::ops::Bound::{Excluded, Included};

struct BaseRows {
    stream: ProjectedReadStream,
    batch: Option<ProjectedReadBatch>,
    row: usize,
}
impl BaseRows {
    fn peek(&mut self) -> Result<Option<(&[u8], &[u8])>> {
        if self
            .batch
            .as_ref()
            .is_none_or(|batch| self.row >= batch.len())
        {
            // Release the old output reservation before requesting another batch.
            self.batch = None;
            self.batch = self.stream.next_batch()?;
            self.row = 0;
        }
        self.batch
            .as_ref()
            .map(|batch| {
                Ok((
                    batch.key(self.row).ok_or_else(|| {
                        Error::InvalidProjectedRead("projected scan omitted its key".into())
                    })?,
                    batch.value(self.row).ok_or_else(|| {
                        Error::InvalidProjectedRead("projected scan omitted its value".into())
                    })?,
                ))
            })
            .transpose()
    }
    fn advance(&mut self) {
        self.row += 1;
    }
}

/// The returned Vec is caller-owned. This bounds its payload and vector
/// representations before copying; source batch reservations remain separate.
struct OutputRows {
    rows: Vec<(Vec<u8>, Vec<u8>)>,
    payload: usize,
    bytes: usize,
    limit: usize,
}
impl OutputRows {
    fn push(&mut self, key: &[u8], value: &[u8]) -> Result<()> {
        let payload = self
            .payload
            .checked_add(key.len())
            .and_then(|n| n.checked_add(value.len()))
            .ok_or_else(|| {
                Error::InvalidProjectedRead("transaction scan output overflow".into())
            })?;
        let row_bytes = std::mem::size_of::<(Vec<u8>, Vec<u8>)>();
        let minimum = self
            .rows
            .len()
            .checked_add(1)
            .and_then(|n| n.checked_mul(row_bytes))
            .and_then(|n| n.checked_add(payload))
            .ok_or_else(|| Error::InvalidProjectedRead("transaction scan row overflow".into()))?;
        if minimum > self.bytes {
            return Err(Error::ProjectedReadLimit {
                resource: ProjectedReadResource::OutputBufferBytes,
                limit: self.bytes as u64,
                observed: minimum as u64,
            });
        }
        if self.rows.len() == self.rows.capacity() {
            let capacity = self
                .rows
                .capacity()
                .saturating_mul(2)
                .max(4)
                .min(self.limit)
                .min((self.bytes - payload) / row_bytes);
            self.rows
                .try_reserve_exact(capacity - self.rows.len())
                .map_err(|_| {
                    Error::InvalidProjectedRead("bounded scan output allocation failed".into())
                })?;
        }
        let actual = self
            .rows
            .capacity()
            .checked_mul(row_bytes)
            .and_then(|n| n.checked_add(payload))
            .ok_or_else(|| {
                Error::InvalidProjectedRead("transaction scan capacity overflow".into())
            })?;
        if actual > self.bytes {
            return Err(Error::ProjectedReadLimit {
                resource: ProjectedReadResource::OutputBufferBytes,
                limit: self.bytes as u64,
                observed: actual as u64,
            });
        }
        self.rows.push((key.to_vec(), value.to_vec()));
        self.payload = payload;
        Ok(())
    }
}

pub(super) fn bounded(
    transaction: &Transaction,
    database: &Database,
    start: &[u8],
    end: &[u8],
    limit: usize,
) -> Result<Vec<(Vec<u8>, Vec<u8>)>> {
    if start >= end || limit == 0 {
        return Err(Error::InvalidTransaction(
            "scan requires a non-empty half-open range and positive limit".into(),
        ));
    }
    database.validate_transaction_owner(transaction)?;
    // Bound range copies before creating the owned projected request. An empty
    // lower bound remains the existing way to start at the beginning.
    for bound in [start, end].into_iter().filter(|bound| !bound.is_empty()) {
        crate::batch::validate_key(bound)
            .map_err(|error| Error::InvalidTransaction(format!("scan range: {error}")))?;
    }
    let bytes = database.projected_output_memory().stats().limit_bytes;
    let defaults = ProjectedReadBudget::default();
    let stream = database.begin_projected_read(ProjectedReadRequest {
        ranges: vec![ProjectedReadRange::new(start.to_vec(), Some(end.to_vec()))?],
        projection: ProjectedReadProjection::KeyValue,
        snapshot: transaction.snapshot,
        budget: ProjectedReadBudget {
            max_batch_rows: limit.min(defaults.max_batch_rows),
            max_batch_allocated_bytes: bytes.min(defaults.max_batch_allocated_bytes),
            max_output_buffer_bytes: bytes as u64,
            ..defaults
        },
    })?;
    let mut base = BaseRows {
        stream,
        batch: None,
        row: 0,
    };
    let mut pending = transaction
        .writes
        .range::<[u8], _>((Included(start), Excluded(end)))
        .peekable();
    let mut output = OutputRows {
        rows: Vec::new(),
        payload: 0,
        bytes,
        limit,
    };
    while output.rows.len() < limit {
        let row = base.peek()?;
        let comparison = match (row, pending.peek()) {
            (Some((key, _)), Some((pending_key, _))) => key.cmp(pending_key.as_slice()),
            (Some(_), None) => Ordering::Less,
            (None, Some(_)) => Ordering::Greater,
            (None, None) => break,
        };
        if comparison == Ordering::Less {
            let (key, value) = row.expect("ordered base row exists");
            output.push(key, value)?;
            base.advance();
        } else {
            let (key, value) = pending.next().expect("ordered pending row exists");
            if let Some(value) = value {
                output.push(key, value)?;
            }
            if comparison == Ordering::Equal {
                base.advance();
            }
        }
    }
    base.stream.cancel()?;
    Ok(output.rows)
}
