//! Each file's fact mutations, schema and index registration, retirement and summary.

use super::*;

/// The mutations that make `next` the file's current state: retire every fact
/// the previous state named that the new one does not, write the new facts with
/// names and text sliced from `source`, the bytes the engine read and verified
/// itself, and write the state. A worker's facts never supply content.
pub(in super::super) fn file_mutations(
    previous: Option<&FileState>,
    mut next: FileState,
    outcome: Outcome<'_>,
    source: &[u8],
    at: u64,
) -> Result<Vec<RuntimeMutation>> {
    let mut written = Vec::new();
    match outcome {
        Outcome::Parsed(facts) => {
            if facts.content_sha256 != next.content_sha256
                || facts.language != next.language
                || facts.parser_revision != next.parser_revision
                || digest::sha256_hex(source) != next.content_sha256
            {
                return Err(invalid());
            }
            let common = |path: &str| {
                serde_json::json!({
                    "path": path,
                    "content_sha256": next.content_sha256,
                    "language": next.language,
                })
            };
            let id = |index: usize| {
                fact_id(
                    &next.path,
                    &next.content_sha256,
                    &next.parser_revision,
                    next.chunk_bytes,
                    index as u64,
                )
            };
            for (index, symbol) in facts.symbols.iter().enumerate() {
                let mut properties = common(&next.path);
                properties["name"] = slice(source, symbol.name_start, symbol.name_end)?.into();
                properties["symbol_kind"] = symbol.kind.as_str().into();
                properties["role"] = match symbol.role {
                    rrd_attunement::facts::SymbolRole::Definition => "definition",
                    rrd_attunement::facts::SymbolRole::Reference => "reference",
                }
                .into();
                slice(source, symbol.start_byte, symbol.end_byte)?;
                properties["start_byte"] = symbol.start_byte.into();
                properties["end_byte"] = symbol.end_byte.into();
                properties["start_line"] = symbol.start_line.into();
                properties["count"] = symbol.count.into();
                written.push(record(SYMBOL, id(index), at, properties)?);
            }
            for (index, span) in facts.spans.iter().enumerate() {
                slice(source, span.start_byte, span.end_byte)?;
                let mut properties = common(&next.path);
                properties["span_kind"] = span.kind.as_str().into();
                properties["start_byte"] = span.start_byte.into();
                properties["end_byte"] = span.end_byte.into();
                properties["start_line"] = span.start_line.into();
                properties["end_line"] = span.end_line.into();
                written.push(record(SPAN, id(index), at, properties)?);
            }
            for (index, chunk) in facts.chunks.iter().enumerate() {
                if chunk.end_byte.saturating_sub(chunk.start_byte) > next.chunk_bytes {
                    return Err(invalid());
                }
                let mut properties = common(&next.path);
                properties["text"] = slice(source, chunk.start_byte, chunk.end_byte)?.into();
                properties["start_byte"] = chunk.start_byte.into();
                properties["end_byte"] = chunk.end_byte.into();
                properties["start_line"] = chunk.start_line.into();
                properties["end_line"] = chunk.end_line.into();
                written.push(record(CHUNK, id(index), at, properties)?);
            }
            next.status = Status::Parsed;
            next.reason = String::new();
            next.facts_sha256 = digest::sha256_hex(&serde_json::to_vec(facts).map_err(contract)?);
            next.symbols = facts.symbols.len() as u64;
            next.spans = facts.spans.len() as u64;
            next.chunks = facts.chunks.len() as u64;
            next.nodes = facts.nodes;
            next.error_nodes = facts.error_nodes;
        }
        Outcome::Rejected(reason) => {
            next.status = Status::Rejected;
            next.reason = reason;
            next.facts_sha256 = "0".repeat(64);
            (next.symbols, next.spans, next.chunks) = (0, 0, 0);
            (next.nodes, next.error_nodes) = (0, 0);
        }
        Outcome::Lost(reason) => {
            next.status = Status::Lost;
            next.reason = reason;
            next.facts_sha256 = "0".repeat(64);
            (next.symbols, next.spans, next.chunks) = (0, 0, 0);
            (next.nodes, next.error_nodes) = (0, 0);
        }
    }
    let mut mutations = Vec::new();
    if let Some(previous) = previous {
        let kept = fact_ids(&next);
        for ((kind, before), (_, after)) in fact_ids(previous).into_iter().zip(kept) {
            for id in before.difference(&after) {
                mutations.push(retire(kind, id.clone(), at)?);
            }
        }
    }
    mutations.extend(written);
    mutations.push(state_record(&next, at)?);
    Ok(mutations)
}

/// The parsed-source tables registered in the instance's schema, when they are
/// not there yet.
pub(in super::super) fn schema_mutation(
    engine: &RrdEngine,
    scope: &ScopeId,
    read: &ReadStamp,
) -> Result<Option<RuntimeMutation>> {
    let current = engine.storage.runtime().schema(scope)?;
    if current.as_ref().map(|schema| schema.revision) != read.schema_revision {
        return Err(ServiceError::StorageConflict("Parse schema changed".into()));
    }
    let mut registry = current
        .clone()
        .unwrap_or_else(|| RuntimeSchemaRegistry::empty(1, "parsed source tables"));
    if !register(&mut registry)? {
        return Ok(None);
    }
    if let Some(old) = current {
        registry.revision = old.revision.checked_add(1).ok_or_else(invalid)?;
        registry.migration = "parsed source tables".into();
    }
    Ok(Some(RuntimeMutation::Schema { registry }))
}

/// The indexes parsed source is read through: exact lookups of a symbol by
/// name and of every fact of a file by path, and BM25 over chunk text that
/// indexes code identifiers whole and by their words. Each is maintained in
/// the commit that writes the facts, so a question reads current facts
/// without scanning them.
pub(in super::super) fn index_definitions() -> Result<Vec<rrd_query::IndexDefinition>> {
    let record = |kind: &str| -> Result<rrd_query::Source> {
        Ok(rrd_query::Source::Record {
            kind: RuntimeType::new(kind).map_err(contract)?,
        })
    };
    let definition = |id: &str, kind: &str, field: &str, index: rrd_query::IndexKind| {
        Ok(rrd_query::IndexDefinition {
            id: rrd_core::ProjectionId::new(id).map_err(contract)?,
            source: record(kind)?,
            fields: vec![field.into()],
            unique: false,
            kind: index,
            filters: Vec::new(),
        })
    };
    let code = rrd_query::Bm25Config {
        identifiers: rrd_query::Bm25Identifiers::WholeAndParts,
        ..rrd_query::Bm25Config::default()
    };
    [
        definition(
            "source-symbol-name",
            SYMBOL,
            "name",
            rrd_query::IndexKind::Scalar,
        ),
        definition(
            "source-symbol-path",
            SYMBOL,
            "path",
            rrd_query::IndexKind::Scalar,
        ),
        definition(
            "source-span-path",
            SPAN,
            "path",
            rrd_query::IndexKind::Scalar,
        ),
        definition(
            "source-chunk-path",
            CHUNK,
            "path",
            rrd_query::IndexKind::Scalar,
        ),
        definition(
            "source-chunk-text",
            CHUNK,
            "text",
            rrd_query::IndexKind::Bm25 { config: code },
        ),
    ]
    .into_iter()
    .collect()
}

/// Defines any of [`index_definitions`] the scope does not have yet, once the
/// parsed-source tables exist. The store backfills facts already published.
pub(in super::super) fn ensure_indexes(
    engine: &RrdEngine,
    scope: &ScopeId,
    context: &rrd_query::IndexMutationContext,
) -> Result<()> {
    let Some(schema) = engine.storage.runtime().schema(scope)? else {
        return Ok(());
    };
    if !schema
        .records
        .contains_key(&RuntimeType::new(CHUNK).map_err(contract)?)
    {
        return Ok(());
    }
    let repository = rrd_query::IndexCatalogueRepository::new(&engine.storage, scope.clone());
    let existing = repository
        .load()
        .map_err(|error| ServiceError::Query(error.to_string()))?;
    let missing = index_definitions()?
        .into_iter()
        .filter(|definition| !existing.entries.contains_key(&definition.id))
        .collect::<Vec<_>>();
    if missing.is_empty() {
        return Ok(());
    }
    let mut sources = Vec::new();
    for definition in &missing {
        if !sources.contains(&definition.source) {
            sources.push(definition.source.clone());
        }
    }
    let catalogue = rrd_query::Catalog::capture_for_sources(
        &engine.storage,
        scope,
        &sources,
        RuntimeReadBudget::new(engine.engram_configuration.query.max_storage_keys)?,
    )
    .map_err(|error| ServiceError::Query(error.to_string()))?;
    for definition in missing {
        repository
            .create(context, &catalogue, definition)
            .map_err(|error| ServiceError::Query(error.to_string()))?;
    }
    Ok(())
}

/// Retires a file that left the project or its parse bindings: its facts and
/// its state.
pub(in super::super) fn retire_file(state: &FileState, at: u64) -> Result<Vec<RuntimeMutation>> {
    let mut mutations = Vec::new();
    for (kind, ids) in fact_ids(state) {
        for id in ids {
            mutations.push(retire(kind, id, at)?);
        }
    }
    mutations.push(retire(STATE, state_id(&state.path), at)?);
    Ok(mutations)
}

/// The state a mutation writes, when it writes one.
pub(in super::super) fn state_of(mutation: &RuntimeMutation) -> Result<Option<FileState>> {
    match mutation {
        RuntimeMutation::Record { record } if record.reference.kind.as_str() == STATE => {
            Ok(Some(state_from(record)?))
        }
        _ => Ok(None),
    }
}

/// One digest of every file's current answer, recorded by the Parse checkpoint.
pub(in super::super) fn summary_sha256(states: &BTreeMap<String, FileState>) -> Result<String> {
    let rows: Vec<_> = states
        .values()
        .map(|state| {
            (
                &state.path,
                &state.content_sha256,
                &state.status,
                &state.facts_sha256,
            )
        })
        .collect();
    Ok(digest::sha256_hex(
        &serde_json::to_vec(&rows).map_err(contract)?,
    ))
}
