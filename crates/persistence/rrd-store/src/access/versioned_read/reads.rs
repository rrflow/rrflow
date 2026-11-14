//! Versioned reads, vector-delta reads and authenticated change steps.

use super::*;

pub(crate) fn read_versioned(
    transaction: &(impl AccessRead + ?Sized),
    read: &ReadStamp,
    sources: &[RuntimeVersionedSource],
    budget: RuntimeReadBudget,
) -> Result<RuntimeVersionedRead> {
    read_versioned_expanding(transaction, read, sources, budget, &mut |_| Vec::new())
}

/// Reads typed ranges in steps under one stamp validation, key budget, and
/// accumulator. Every step's changes are key-validated and authenticated
/// before `expand` sees them in global cursor order; `expand` returns the
/// next step's ranges. A range already read is not read again, a range that
/// overlaps a different read range is rejected, and an empty step ends the
/// read.
pub(crate) fn read_versioned_expanding(
    transaction: &(impl AccessRead + ?Sized),
    read: &ReadStamp,
    sources: &[RuntimeVersionedSource],
    budget: RuntimeReadBudget,
    expand: &mut dyn FnMut(&[RuntimeChange]) -> Vec<RuntimeVersionedSource>,
) -> Result<RuntimeVersionedRead> {
    budget.validate()?;
    if sources.is_empty() {
        return Err(Error::Substrate(
            "runtime versioned read requires at least one typed source".into(),
        ));
    }
    let mut ranges = SourceRanges::default();
    let step = ranges.admit(sources, &read.scope)?;
    if read.accumulator_root.is_none() {
        return Err(Error::ReadStampMismatch(read.manifest_id.clone()));
    }

    let reader = MeteredRead::new(transaction, budget);
    let stamp_validation = reader.with_path(RuntimeReadAccessPath::ReadStamp, || {
        validate_read_stamp(&reader, read)
    });
    let stamp_validation = reader.preserve_budget_error(stamp_validation)?;
    let changes = authenticated_steps(&reader, read, ranges, step, expand)?;
    let evidence = reader.finish(stamp_validation)?;
    let result = RuntimeVersionedRead {
        read: read.clone(),
        changes: changes.into_values().collect(),
        evidence,
    };
    result.evidence.validate()?;
    Ok(result)
}

/// Reads one vector source's committed deltas in (`after`, read cursor] and
/// the complete history of every identity they name, under one stamp
/// validation, key budget, and accumulator. The deltas are a derived index:
/// each before and after pointer must match the authenticated version it
/// names, and every authenticated version that entered the source in range
/// must have its delta, or the read fails closed.
pub(crate) fn read_vector_deltas(
    transaction: &(impl AccessRead + ?Sized),
    read: &ReadStamp,
    source: &VectorSourceAddress,
    after: u64,
    budget: RuntimeReadBudget,
) -> Result<RuntimeVectorDeltaRead> {
    budget.validate()?;
    source.validate()?;
    if after > read.commit_cursor {
        return Err(Error::Substrate(
            "vector delta read starts after its read stamp".into(),
        ));
    }
    if read.accumulator_root.is_none() {
        return Err(Error::ReadStampMismatch(read.manifest_id.clone()));
    }

    let reader = MeteredRead::new(transaction, budget);
    let stamp_validation = reader.with_path(RuntimeReadAccessPath::ReadStamp, || {
        validate_read_stamp(&reader, read)
    });
    let stamp_validation = reader.preserve_budget_error(stamp_validation)?;
    let scanned = reader.with_path(RuntimeReadAccessPath::VectorSourceDeltas, || {
        scan_vector_deltas(&reader, read, source, after)
    });
    let deltas = reader.preserve_budget_error(scanned)?;
    let identities = deltas
        .iter()
        .map(|delta| delta.reference.clone())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .map(|reference| RuntimeVersionedSource::Identity {
            model: RuntimeLogicalModel::Vector,
            reference,
        })
        .collect::<Vec<_>>();
    let mut ranges = SourceRanges::default();
    let step = ranges.admit(&identities, &read.scope)?;
    let changes = authenticated_steps(&reader, read, ranges, step, &mut |_| Vec::new())?;
    verify_vector_deltas(source, after, &deltas, &changes)?;
    let evidence = reader.finish(stamp_validation)?;
    let result = RuntimeVectorDeltaRead {
        read: read.clone(),
        source: source.clone(),
        after,
        deltas,
        changes: changes.into_values().collect(),
        evidence,
    };
    result.evidence.validate()?;
    Ok(result)
}

/// The source's deltas in (`after`, read cursor], each bound to its typed key.
pub(super) fn scan_vector_deltas(
    reader: &(impl AccessRead + ?Sized),
    read: &ReadStamp,
    source: &VectorSourceAddress,
    after: u64,
) -> Result<Vec<VectorSourceDelta>> {
    let key = |cursor: u64| {
        keyspaces::runtime_vector_source_delta_start(
            &read.scope,
            &source.collection.collection_id,
            &source.collection.vector_name,
            &source.field,
            cursor,
        )
    };
    let mut deltas = Vec::new();
    // A read starting at its own stamp has no cursor to scan.
    if after == read.commit_cursor {
        return Ok(deltas);
    }
    let start = key(after.checked_add(1).ok_or(Error::SequenceOverflow)?)?;
    let end = key(read
        .commit_cursor
        .checked_add(1)
        .ok_or(Error::SequenceOverflow)?)?;
    for (stored_key, bytes) in reader.scan_range(&start, &end, usize::MAX)? {
        let delta: VectorSourceDelta = serde_json::from_slice(&bytes)?;
        delta.validate()?;
        let expected = keyspaces::runtime_vector_source_delta_key(
            &read.scope,
            &source.collection.collection_id,
            &source.collection.vector_name,
            &source.field,
            delta.source_cursor,
            delta.commit_ordinal,
        )?;
        if delta.scope != read.scope
            || &delta.source != source
            || stored_key != expected
            || delta.source_cursor <= after
            || delta.source_cursor > read.commit_cursor
        {
            return Err(Error::IndexConstraint(format!(
                "vector source delta at cursor {} does not match its typed key or read range",
                delta.source_cursor
            )));
        }
        deltas.push(delta);
    }
    Ok(deltas)
}

/// Every delta names the authenticated commit that produced it and the
/// versions it moved between; every version that entered the source in range
/// is in the stream unless the same commit retired it.
pub(super) fn verify_vector_deltas(
    source: &VectorSourceAddress,
    after: u64,
    deltas: &[VectorSourceDelta],
    changes: &BTreeMap<u64, RuntimeChange>,
) -> Result<()> {
    let refused = |cursor: u64, reason: &str| {
        Err(Error::IndexConstraint(format!(
            "vector source delta at cursor {cursor} {reason}"
        )))
    };
    let in_source = |vector: &rrd_core::RuntimeVector| {
        vector.collection == source.collection && vector.field == source.field
    };
    let vector_at = |cursor: u64, reference: &RuntimeRef| match changes.get(&cursor) {
        Some(RuntimeChange {
            mutation: RuntimeMutation::Vector { vector },
            ..
        }) if &vector.reference == reference => Some(vector),
        _ => None,
    };
    let points_to = |pointer: &VectorSourceVersion, vector: &rrd_core::RuntimeVector| {
        let change = &changes[&pointer.source_cursor];
        Ok::<_, Error>(
            pointer.commit_id == change.commit_id
                && pointer.commit_ordinal == change.commit_ordinal
                && pointer.valid_from == vector.valid_from
                && pointer.valid_to == vector.valid_to
                && pointer.value_sha256
                    == rrd_core::digest::sha256_hex(&serde_json::to_vec(&vector.value)?),
        )
    };
    for delta in deltas {
        let Some(produced) = changes.get(&delta.source_cursor) else {
            return refused(delta.source_cursor, "names no authenticated version");
        };
        if produced.commit_id != delta.commit_id || produced.commit_ordinal != delta.commit_ordinal
        {
            return refused(delta.source_cursor, "names another commit");
        }
        match &delta.after {
            Some(pointer) => match vector_at(pointer.source_cursor, &delta.reference) {
                Some(vector) if in_source(vector) && points_to(pointer, vector)? => {}
                _ => return refused(delta.source_cursor, "does not match its after version"),
            },
            None => {
                let ended = match &produced.mutation {
                    RuntimeMutation::Retire { retirement } => {
                        retirement.model == RuntimeLogicalModel::Vector
                            && retirement.reference == delta.reference
                    }
                    RuntimeMutation::Vector { vector } => {
                        vector.reference == delta.reference && !in_source(vector)
                    }
                    _ => false,
                };
                if !ended {
                    return refused(
                        delta.source_cursor,
                        "does not name the retirement or move that ended its version",
                    );
                }
            }
        }
        if let Some(pointer) = &delta.before {
            match vector_at(pointer.source_cursor, &delta.reference) {
                Some(vector)
                    if pointer.source_cursor < delta.source_cursor
                        && in_source(vector)
                        && points_to(pointer, vector)? => {}
                _ => return refused(delta.source_cursor, "does not match its before version"),
            }
        }
    }
    let recorded = deltas
        .iter()
        .filter_map(|delta| delta.after.as_ref().map(|after| after.source_cursor))
        .collect::<BTreeSet<_>>();
    for change in changes.values().filter(|change| change.cursor > after) {
        let RuntimeMutation::Vector { vector } = &change.mutation else {
            continue;
        };
        // The kernel refuses two puts of one identity in a commit, but a put
        // and its retirement may share one; the commit records only its final
        // transition, the retirement.
        let retired_with_it = changes.values().any(|later| {
            later.cursor > change.cursor
                && later.commit_id == change.commit_id
                && matches!(&later.mutation, RuntimeMutation::Retire { retirement }
                    if retirement.reference == vector.reference)
        });
        if in_source(vector) && !recorded.contains(&change.cursor) && !retired_with_it {
            return refused(
                change.cursor,
                "is missing for a version that entered the source",
            );
        }
    }
    Ok(())
}

/// Reads, key-validates, and authenticates typed version ranges in dependent
/// steps under the reader's stamp, budget, and one accumulator. Every step's
/// changes are authenticated before `expand` sees them in cursor order.
pub(super) fn authenticated_steps<R: AccessRead + ?Sized>(
    reader: &MeteredRead<'_, R>,
    read: &ReadStamp,
    mut ranges: SourceRanges,
    mut step: Vec<RuntimeVersionedSource>,
    expand: &mut dyn FnMut(&[RuntimeChange]) -> Vec<RuntimeVersionedSource>,
) -> Result<BTreeMap<u64, RuntimeChange>> {
    let mut accumulator = None;
    // Owned by this one immutable transaction/stamp, never by the store.
    let mut proof_nodes = ProofNodes::new();
    let mut changes = BTreeMap::<u64, RuntimeChange>::new();
    while !step.is_empty() {
        let mut selected = BTreeMap::<u64, RuntimeChange>::new();
        for source in &step {
            let path = source.path()?;
            reader.with_path(path, || {
                scan_source(reader, read, source, &mut |stored_key, bytes| {
                    let change: RuntimeChange = serde_json::from_slice(&bytes)?;
                    if change.cursor > read.commit_cursor {
                        return Ok(());
                    }
                    validate_version_entry(source, read, &stored_key, &change)?;
                    insert_version(&mut selected, change)
                })
            })?;
        }
        let authentication = reader.with_path(RuntimeReadAccessPath::AccumulatorProof, || {
            let accumulator = match &mut accumulator {
                Some(accumulator) => accumulator,
                empty => empty.insert(open_accumulator(reader, read, &mut proof_nodes)?),
            };
            authenticate_changes(
                reader,
                read,
                accumulator,
                &mut proof_nodes,
                selected
                    .values()
                    .filter(|change| !changes.contains_key(&change.cursor)),
            )
        });
        reader.preserve_budget_error(authentication)?;
        let selected = selected.into_values().collect::<Vec<_>>();
        let next = expand(&selected);
        for change in selected {
            insert_version(&mut changes, change)?;
        }
        step = ranges.admit(&next, &read.scope)?;
    }
    Ok(changes)
}

pub(super) fn insert_version(
    changes: &mut BTreeMap<u64, RuntimeChange>,
    change: RuntimeChange,
) -> Result<()> {
    match changes.entry(change.cursor) {
        std::collections::btree_map::Entry::Vacant(entry) => {
            entry.insert(change);
        }
        std::collections::btree_map::Entry::Occupied(entry) => {
            if entry.get() != &change {
                return Err(Error::Substrate(format!(
                    "runtime cursor {} resolved to conflicting semantic versions",
                    change.cursor
                )));
            }
        }
    }
    Ok(())
}

pub(crate) fn schema_at_read(
    read: &ReadStamp,
    changes: &[RuntimeChange],
) -> Result<rrd_core::RuntimeSchemaRegistry> {
    let expected_revision = read
        .schema_revision
        .ok_or_else(|| Error::RuntimeSchemaMissing(read.scope.to_string()))?;
    let schema = changes
        .iter()
        .filter(|change| change.scope == read.scope && change.cursor <= read.commit_cursor)
        .filter_map(|change| match &change.mutation {
            RuntimeMutation::Schema { registry } => Some(registry),
            _ => None,
        })
        .next_back()
        .cloned()
        .ok_or_else(|| Error::RuntimeSchemaMissing(read.scope.to_string()))?;
    if schema.revision != expected_revision {
        return Err(Error::ReadStampUnavailable(format!(
            "schema revision {} for scope {} at cursor {}",
            expected_revision, read.scope, read.commit_cursor
        )));
    }
    Ok(schema)
}
