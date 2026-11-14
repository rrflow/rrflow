//! Driving a projected read stream across the memtable and segments.

use super::*;

impl ProjectedReadStream {
    pub(crate) fn new(
        view: ReadView,
        ranges: Vec<ProjectedReadRange>,
        projection: ProjectedReadProjection,
        budget: ProjectedReadBudget,
        cache_access: PageCacheAccess,
        lease: ReadViewLease,
        output_memory: ProjectedOutputMemory,
    ) -> Self {
        let ranges: Arc<[ProjectedReadRange]> = ranges.into();
        let mut cursors = Vec::with_capacity(
            view.segments
                .len()
                .saturating_add(usize::from(view.memtable.version_count() != 0)),
        );
        if view.memtable.version_count() != 0 {
            cursors.push(RunCursor::Memtable(MemtableProjectedCursor {
                table: Arc::clone(&view.memtable),
                ranges: Arc::clone(&ranges),
                read_sequence: view.sequence,
                after: None,
            }));
        }
        cursors.extend(view.segments.into_iter().map(|segment| {
            RunCursor::Segment(SegmentProjectedCursor::new(
                segment,
                Arc::clone(&ranges),
                view.sequence,
                cache_access,
            ))
        }));
        let evidence = ProjectedReadEvidence {
            contract_version: PROJECTED_READ_CONTRACT_VERSION,
            manifest: view.manifest,
            snapshot_sequence: view.sequence,
            projection: Some(projection),
            selected_ranges: ranges.len() as u64,
            selected_runs: cursors.len() as u64,
            pinned_bytes: view.pinned_bytes,
            outcome: ProjectedReadOutcome::Creating,
            ..ProjectedReadEvidence::default()
        };
        let heads = (0..cursors.len()).map(|_| None).collect();
        Self {
            projection,
            budget,
            meter: ProjectedReadMeter { evidence, budget },
            cursors,
            heads,
            heap: BinaryHeap::new(),
            unopened: BinaryHeap::new(),
            pending: None,
            initialized: false,
            terminal: false,
            lease: Some(lease),
            output_memory,
        }
    }

    pub fn evidence(&self) -> &ProjectedReadEvidence {
        &self.meter.evidence
    }

    pub fn cancel(&mut self) -> Result<()> {
        if self.terminal {
            return Ok(());
        }
        self.finish(ProjectedReadOutcome::Cancelled)
    }

    pub fn next_batch(&mut self) -> Result<Option<ProjectedReadBatch>> {
        if self.terminal {
            return Ok(None);
        }
        self.meter.evidence.outcome = ProjectedReadOutcome::Running;
        let result = self.next_batch_inner();
        match result {
            Ok(Some(batch)) => Ok(Some(batch)),
            Ok(None) => {
                self.finish(ProjectedReadOutcome::Completed)?;
                Ok(None)
            }
            Err(error) => {
                let _ = self.finish(ProjectedReadOutcome::Failed);
                Err(error)
            }
        }
    }

    fn next_batch_inner(&mut self) -> Result<Option<ProjectedReadBatch>> {
        self.initialize()?;
        let mut rows = Vec::new();
        let mut key_bytes = 0usize;
        let mut value_bytes = 0usize;
        loop {
            let prepared = match self.pending.take() {
                Some(row) => Some(row),
                None => self.next_row()?,
            };
            let Some(prepared) = prepared else {
                break;
            };
            let next_rows = rows.len().checked_add(1).ok_or_else(|| {
                Error::InvalidProjectedRead("projected batch row count overflow".into())
            })?;
            let next_key_bytes = key_bytes.checked_add(prepared.key.len()).ok_or_else(|| {
                Error::InvalidProjectedRead("projected key byte count overflow".into())
            })?;
            let next_value_bytes =
                value_bytes.checked_add(prepared.value_len).ok_or_else(|| {
                    Error::InvalidProjectedRead("projected value byte count overflow".into())
                })?;
            let allocation = batch_allocation_bytes(
                next_rows,
                next_key_bytes,
                next_value_bytes,
                self.projection,
            )?;
            if next_rows > self.budget.max_batch_rows
                || allocation > self.budget.max_batch_allocated_bytes
            {
                if rows.is_empty() {
                    let (resource, limit, observed) = if next_rows > self.budget.max_batch_rows {
                        (
                            ProjectedReadResource::BatchRows,
                            self.budget.max_batch_rows as u64,
                            next_rows as u64,
                        )
                    } else {
                        (
                            ProjectedReadResource::BatchAllocatedBytes,
                            self.budget.max_batch_allocated_bytes as u64,
                            allocation as u64,
                        )
                    };
                    return Err(Error::ProjectedReadLimit {
                        resource,
                        limit,
                        observed,
                    });
                }
                self.pending = Some(prepared);
                break;
            }
            let projected_rows = self
                .meter
                .evidence
                .emitted_rows
                .checked_add(next_rows as u64)
                .ok_or_else(|| {
                    Error::InvalidProjectedRead("projected output row count overflow".into())
                })?;
            check_limit(
                ProjectedReadResource::OutputRows,
                self.budget.max_output_rows,
                projected_rows,
            )?;
            let projected_bytes = self
                .meter
                .evidence
                .emitted_buffer_bytes
                .checked_add(allocation as u64)
                .ok_or_else(|| {
                    Error::InvalidProjectedRead("projected output byte count overflow".into())
                })?;
            check_limit(
                ProjectedReadResource::OutputBufferBytes,
                self.budget.max_output_buffer_bytes,
                projected_bytes,
            )?;
            key_bytes = next_key_bytes;
            value_bytes = next_value_bytes;
            rows.push(prepared);
            if rows.len() == self.budget.max_batch_rows {
                break;
            }
        }
        if rows.is_empty() {
            return Ok(None);
        }
        self.build_batch(rows, key_bytes, value_bytes).map(Some)
    }

    fn initialize(&mut self) -> Result<()> {
        if self.initialized {
            return Ok(());
        }
        self.initialized = true;
        for run in 0..self.cursors.len() {
            match &self.cursors[run] {
                RunCursor::Memtable(_) => self.advance(run)?,
                RunCursor::Segment(cursor) => self.unopened.push(HeapItem {
                    // Descriptor bounds already govern read-view selection.
                    // This lower bound cannot exceed any visible selected key.
                    key: cursor.segment.descriptor.first_key.clone(),
                    run,
                }),
            }
        }
        Ok(())
    }

    fn open_competing_runs(&mut self) -> Result<()> {
        // Open every run that could precede or tie the smallest loaded key.
        // Opening may move that key; compare again before selecting a winner.
        // Disjoint future runs remain pinned/admitted but need no page I/O yet.
        while self.unopened.peek().is_some_and(|candidate| {
            self.heap
                .peek()
                .is_none_or(|head| candidate.key <= head.key)
        }) {
            let candidate = self.unopened.pop().expect("unopened run was checked");
            self.advance(candidate.run)?;
        }
        Ok(())
    }

    fn advance(&mut self, run: usize) -> Result<()> {
        let entry = self.cursors[run].next_candidate(&mut self.meter)?;
        if let Some(entry) = entry {
            self.heap.push(HeapItem {
                key: entry.key.clone(),
                run,
            });
            self.heads[run] = Some(entry);
        }
        Ok(())
    }

    fn next_row(&mut self) -> Result<Option<PreparedRow>> {
        loop {
            self.open_competing_runs()?;
            let Some(first) = self.heap.pop() else {
                return Ok(None);
            };
            let key = first.key;
            let mut runs = vec![first.run];
            while self.heap.peek().is_some_and(|item| item.key == key) {
                runs.push(self.heap.pop().expect("heap item was checked").run);
            }
            let mut entries = Vec::with_capacity(runs.len());
            let mut sequences = BTreeMap::new();
            for run in &runs {
                let entry = self.heads[*run].take().ok_or_else(|| {
                    Error::InvalidProjectedRead("projected merge heap lost its run head".into())
                })?;
                if entry.key != key {
                    return Err(Error::InvalidProjectedRead(
                        "projected merge heap key differs from its run head".into(),
                    ));
                }
                if sequences.insert(entry.sequence, *run).is_some() {
                    return Err(Error::DuplicateProjectedSequence {
                        manifest: self.meter.evidence.manifest.clone(),
                        sequence: entry.sequence,
                    });
                }
                entries.push((*run, entry));
            }
            let winner = entries
                .iter()
                .max_by_key(|(_, entry)| entry.sequence)
                .expect("one heap item creates one entry");
            let mut prepared = self.cursors[winner.0].prepare_value(
                winner.1.clone(),
                self.projection,
                &mut self.meter,
            )?;
            if let Some(prepared) = prepared.as_mut() {
                prepared.run = winner.0;
            }
            for run in runs {
                self.advance(run)?;
            }
            if let Some(prepared) = prepared {
                return Ok(Some(prepared));
            }
        }
    }

    fn build_batch(
        &mut self,
        rows: Vec<PreparedRow>,
        key_bytes: usize,
        value_bytes: usize,
    ) -> Result<ProjectedReadBatch> {
        let allocation =
            batch_allocation_bytes(rows.len(), key_bytes, value_bytes, self.projection)?;
        let mut reservation = self.output_memory.reserve(allocation)?;
        self.meter.record_batch_allocation(allocation)?;
        let mut key_offsets = MutableBuffer::from_len_zeroed((rows.len() + 1) * 8);
        let mut key_data = MutableBuffer::from_len_zeroed(key_bytes);
        let mut value_offsets = (self.projection == ProjectedReadProjection::KeyValue)
            .then(|| MutableBuffer::from_len_zeroed((rows.len() + 1) * 8));
        let mut value_data = (self.projection == ProjectedReadProjection::KeyValue)
            .then(|| MutableBuffer::from_len_zeroed(value_bytes));
        let mut key_cursor = 0usize;
        let mut value_cursor = 0usize;
        write_offset(key_offsets.as_slice_mut(), 0, 0)?;
        if let Some(offsets) = value_offsets.as_mut() {
            write_offset(offsets.as_slice_mut(), 0, 0)?;
        }
        for (index, row) in rows.iter().enumerate() {
            let key_end = key_cursor.checked_add(row.key.len()).ok_or_else(|| {
                Error::InvalidProjectedRead("projected key output overflow".into())
            })?;
            key_data.as_slice_mut()[key_cursor..key_end].copy_from_slice(&row.key);
            key_cursor = key_end;
            write_offset(key_offsets.as_slice_mut(), index + 1, key_cursor)?;
            if self.projection == ProjectedReadProjection::KeyValue {
                let value_end = value_cursor.checked_add(row.value_len).ok_or_else(|| {
                    Error::InvalidProjectedRead("projected value output overflow".into())
                })?;
                let output = value_data
                    .as_mut()
                    .expect("value projection allocated value data")
                    .as_slice_mut()
                    .get_mut(value_cursor..value_end)
                    .ok_or_else(|| {
                        Error::InvalidProjectedRead("projected value output is absent".into())
                    })?;
                self.cursors[row.run].copy_value(row, output, &mut self.meter)?;
                value_cursor = value_end;
                write_offset(
                    value_offsets
                        .as_mut()
                        .expect("value projection allocated offsets")
                        .as_slice_mut(),
                    index + 1,
                    value_cursor,
                )?;
            }
        }
        let batch = ProjectedReadBatch {
            projection: self.projection,
            rows: rows.len(),
            key_offsets: reservation.wrap(key_offsets)?,
            key_data: reservation.wrap(key_data)?,
            value_offsets: value_offsets
                .map(|buffer| reservation.wrap(buffer))
                .transpose()?,
            value_data: value_data
                .map(|buffer| reservation.wrap(buffer))
                .transpose()?,
        };
        self.meter.record_emitted_batch(
            rows.len() as u64,
            key_bytes as u64,
            value_bytes as u64,
            batch.allocated_bytes() as u64,
        )?;
        Ok(batch)
    }

    fn finish(&mut self, outcome: ProjectedReadOutcome) -> Result<()> {
        if self.terminal {
            return Ok(());
        }
        self.terminal = true;
        self.meter.evidence.outcome = outcome;
        // Release captured generations and temporary buffers before the pin.
        // Returned batches own copied buffers independently of these sources.
        self.pending = None;
        self.heads = Vec::new();
        self.heap = BinaryHeap::new();
        self.unopened = BinaryHeap::new();
        self.cursors = Vec::new();
        let release = self
            .lease
            .take()
            .map_or(Ok(()), |mut lease| lease.release());
        tracing::debug!(
            target: "rrd_lsm::projected_read",
            manifest = %self.meter.evidence.manifest,
            snapshot_sequence = self.meter.evidence.snapshot_sequence,
            projection = ?self.projection,
            selected_ranges = self.meter.evidence.selected_ranges,
            selected_runs = self.meter.evidence.selected_runs,
            selected_row_groups = self.meter.evidence.selected_row_groups,
            versions_examined = self.meter.evidence.versions_examined,
            page_requests = self.meter.evidence.page_requests,
            physical_bytes_read = self.meter.evidence.physical_bytes_read,
            emitted_rows = self.meter.evidence.emitted_rows,
            emitted_buffer_bytes = self.meter.evidence.emitted_buffer_bytes,
            outcome = ?outcome,
            "projected physical read reached a terminal state"
        );
        release
    }
}

impl Drop for ProjectedReadStream {
    fn drop(&mut self) {
        if !self.terminal {
            let _ = self.finish(ProjectedReadOutcome::Cancelled);
        }
    }
}
