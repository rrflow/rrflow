//! Choosing the physical plan.

use super::*;

pub fn plan(bound: &BoundQuery) -> Result<PhysicalPlan> {
    let mut operators = vec![
        LogicalOperator::Scan {
            source: bound.source.clone(),
        },
        LogicalOperator::Temporal {
            valid_at: bound.valid_at,
            known_at_cursor: bound.known_at_cursor,
        },
    ];
    if let Some(join) = &bound.join {
        operators.push(LogicalOperator::Join {
            source: join.source.clone(),
            left_field: join.left_field.clone(),
            right_field: join.right_field.clone(),
        });
    }
    if !bound.filters.is_empty() {
        operators.push(LogicalOperator::Filter {
            predicates: bound.filters.clone(),
        });
    }
    operators.push(LogicalOperator::Project {
        projection: bound.projection.clone(),
    });
    if let Some(rows) = bound.limit {
        operators.push(LogicalOperator::Limit { rows });
    }
    let logical = LogicalPlan {
        contract_version: bound.contract_version,
        result_identity_version: rrd_core::result_identity::RESULT_IDENTITY_VERSION,
        read: bound.read.clone(),
        schema_revision: bound.schema_revision,
        source_cursor: bound.source_cursor,
        version_sources: bound.version_sources.clone(),
        field_types: bound.field_types.clone(),
        operators,
        explain_analyze: bound.explain_analyze,
    };
    let event_cursor = event_cursor_filter(bound);
    let identity_read = matches!(
        bound.version_sources.as_slice(),
        [RuntimeVersionedSource::Identity { .. }]
    );
    let adjacency_read = matches!(
        bound.version_sources.as_slice(),
        [RuntimeVersionedSource::Adjacency { .. }]
    );
    let identity_range_read = matches!(
        bound.version_sources.as_slice(),
        [RuntimeVersionedSource::IdentityRange { .. }]
    );
    let traversal = std::iter::once(&bound.source)
        .chain(bound.join.as_ref().map(|join| &join.source))
        .any(|source| matches!(source, Source::Traversal { .. }));
    let match_field = bound
        .filters
        .iter()
        .find(|filter| filter.comparison.is_match())
        .map(|filter| filter.field.as_str());
    let selected_index = (!identity_read && !identity_range_read && !adjacency_read && !traversal)
        .then(|| {
            bound
                .index_candidates
                .iter()
                .filter(|candidate| {
                    candidate.state == ProjectionState::Ready
                        && candidate.source_cursor == bound.source_cursor
                        && candidate.source_cursor <= bound.known_at_cursor
                        && candidate.built_valid_at == Some(bound.valid_at)
                        && candidate.artifact_rows.is_some()
                        && match (&candidate.kind, match_field) {
                            (IndexKind::Scalar | IndexKind::Geo, None) => {
                                candidate.predicates.is_empty()
                            }
                            (IndexKind::MaterializedView, None) => {
                                candidate.predicates == bound.filters
                            }
                            (IndexKind::Bm25 { .. }, Some(field)) => {
                                candidate.fields.as_slice() == [field]
                            }
                            _ => false,
                        }
                })
                .max_by(|left, right| {
                    left.matched_prefix
                        .cmp(&right.matched_prefix)
                        .then_with(|| right.id.cmp(&left.id))
                })
        })
        .flatten();
    let commit_index = (selected_index.is_none()
        && !identity_read
        && !identity_range_read
        && !adjacency_read
        && !traversal
        && bound.join.is_none()
        && bound.known_at_cursor == bound.read.commit_cursor)
        .then(|| {
            select_commit_index(
                &bound.commit_indexes,
                &bound.filters,
                bound.limit,
                match_field,
            )
        })
        .flatten();
    let mut candidates = if traversal {
        vec![
            CandidatePath {
                name: "adjacency_traversal".into(),
                selected: true,
                exact: true,
                reason: "reads each start identity, then per breadth-first level the temporal adjacency of the frontier and the identity history of every relation it names".into(),
            },
            CandidatePath {
                name: "versioned_source_read".into(),
                selected: false,
                exact: true,
                reason: "rejected: relation and record family ranges grow with the engram, not with the visited neighbourhood".into(),
            },
            CandidatePath {
                name: "derived_projection".into(),
                selected: false,
                exact: false,
                reason: "rejected: no projection generation and grounding proof supplied".into(),
            },
        ]
    } else if identity_range_read {
        vec![
            CandidatePath {
                name: "versioned_identity_range_read".into(),
                selected: true,
                exact: true,
                reason: "reads complete identity histories inside the bound ID interval before temporal and residual predicate evaluation".into(),
            },
            CandidatePath {
                name: "versioned_source_read".into(),
                selected: false,
                exact: true,
                reason: "rejected: identities outside the ID interval cannot satisfy the query".into(),
            },
        ]
    } else if adjacency_read {
        vec![
            CandidatePath {
                name: "versioned_adjacency_read".into(),
                selected: true,
                exact: true,
                reason: "reads the bound endpoint adjacency and complete histories of its named relations before temporal and residual predicate evaluation".into(),
            },
            CandidatePath {
                name: "versioned_source_read".into(),
                selected: false,
                exact: true,
                reason: "rejected: unrelated relation histories do not belong to the addressed endpoint".into(),
            },
        ]
    } else if identity_read {
        vec![
            CandidatePath {
                name: "versioned_identity_read".into(),
                selected: selected_index.is_none(),
                exact: true,
                reason: event_cursor.map_or_else(
                    || "selects the typed record or relation identity for bound id equality at the captured read stamp; all predicates remain evaluated".into(),
                    |cursor| format!("selects the typed event identity for bound global cursor {cursor} at the captured read stamp"),
                ),
            },
            CandidatePath {
                name: "versioned_source_read".into(),
                selected: false,
                exact: true,
                reason: if event_cursor.is_some() {
                    "rejected: the typed identity range is narrower than the event-family range"
                } else {
                    "rejected: the bound identity range excludes unrelated family histories"
                }.into(),
            },
            CandidatePath {
                name: "derived_projection".into(),
                selected: false,
                exact: false,
                reason: "rejected: no projection generation and grounding proof supplied".into(),
            },
        ]
    } else if let Some((index, probe)) = &commit_index {
        vec![
            CandidatePath {
                name: format!("commit_index:{}", index.id),
                selected: true,
                exact: true,
                reason: match probe {
                    CommitIndexProbe::Scalar { values } => format!(
                        "selected: commit-maintained entries for {} leading value(s) name the identities to read at the head; every predicate is still evaluated",
                        values.len()
                    ),
                    CommitIndexProbe::Bm25 { top_k, .. } => format!(
                        "selected: commit-maintained postings rank every current document and name the top {top_k} identities to read at the head"
                    ),
                },
            },
            CandidatePath {
                name: "versioned_source_read".into(),
                selected: false,
                exact: true,
                reason: "fallback: read when the index cannot answer for this stamp or valid time"
                    .into(),
            },
        ]
    } else {
        vec![
            CandidatePath {
                name: "versioned_source_read".into(),
                selected: selected_index.is_none(),
                exact: true,
                reason: if selected_index.is_none() {
                    "selects only the typed semantic version ranges required by the bound sources"
                        .into()
                } else {
                    "rejected: a verified exact scalar index requests fewer materialized rows"
                        .into()
                },
            },
            CandidatePath {
                name: "derived_projection".into(),
                selected: false,
                exact: false,
                reason: "rejected: no projection generation and grounding proof supplied".into(),
            },
        ]
    };
    candidates.extend(bound.index_candidates.iter().map(|index| {
        let state = format!("{:?}", index.state).to_ascii_lowercase();
        let freshness = if index.source_cursor == bound.source_cursor {
            "fresh"
        } else if index.source_cursor > bound.known_at_cursor {
            "newer-than-requested"
        } else {
            "stale"
        };
        let selected = selected_index.is_some_and(|selected| selected.id == index.id);
        let time = index
            .built_valid_at
            .map_or_else(|| "unbuilt".into(), |value| format!("valid-time {value}"));
        CandidatePath {
            name: format!("index:{}", index.id),
            selected,
            exact: selected,
            reason: if selected {
                format!(
                    "selected: verified generation {} is ready and exact at cursor {} and valid-time {}; matched prefix {}/{}",
                    index.generation,
                    index.source_cursor,
                    bound.valid_at,
                    index.matched_prefix,
                    index.fields.len()
                )
            } else {
                format!(
                    "rejected: generation {} is {state}, {freshness} at cursor {}, and {time}; matched prefix {}/{}",
                    index.generation,
                    index.source_cursor,
                    index.matched_prefix,
                    index.fields.len()
                )
            },
        }
    }));
    let explanation = PlanExplanation {
        contract: ExecutionContract {
            read_manifest: bound.read.manifest_id.clone(),
            scope: bound.read.scope.to_string(),
            valid_at: bound.valid_at,
            known_at_cursor: bound.known_at_cursor,
            source_cursor: bound.source_cursor,
            schema_revision: bound.schema_revision,
            exact: true,
            deterministic_order: if match_field.is_some() {
                "bm25_score_desc_identity_ascending".into()
            } else {
                "identity_ascending".into()
            },
            read_authentication: "rfc9162_authenticated_semantic_versions".into(),
            read_budget_unit: "physical_keys_examined".into(),
            network_required: false,
            gpu_required: false,
            authorization_boundary: format!("scope:{}", bound.read.scope),
        },
        candidates,
    };
    let access = if let Some(index) = selected_index {
        PhysicalOperator::MaterializedIndex {
            id: index.id.clone(),
            kind: index.kind.clone(),
            generation: index.generation,
            source_cursor: index.source_cursor,
            schema_revision: bound.schema_revision,
            valid_at: bound.valid_at,
            config_digest: index.config_digest.clone(),
            artifact_digest: index.artifact_digest.clone(),
            artifact_rows: index
                .artifact_rows
                .expect("selected indexes have a row count"),
            exact: true,
            stable_order: if matches!(index.kind, IndexKind::Bm25 { .. }) {
                "bm25_score_desc_identity_ascending".into()
            } else {
                "identity_ascending".into()
            },
        }
    } else if let Some((index, probe)) = commit_index {
        PhysicalOperator::CommitIndexRead {
            id: index.id.clone(),
            stable_order: if matches!(probe, CommitIndexProbe::Bm25 { .. }) {
                "bm25_score_desc_identity_ascending".into()
            } else {
                "global_cursor".into()
            },
            probe,
            model: index.model,
            fallback: bound.version_sources.clone(),
            exact: true,
        }
    } else if traversal {
        PhysicalOperator::AdjacencyTraversal {
            sources: bound.version_sources.clone(),
            exact: true,
            stable_order: "global_cursor".into(),
        }
    } else {
        PhysicalOperator::VersionedRead {
            sources: bound.version_sources.clone(),
            exact: true,
            stable_order: "global_cursor".into(),
        }
    };
    let physical_operators = vec![access, PhysicalOperator::DataFusionEvaluate];
    let digest = plan_digest(&logical, &physical_operators, &explanation)?;
    Ok(PhysicalPlan {
        logical,
        operators: physical_operators,
        explanation,
        digest,
    })
}

pub(super) fn is_false(value: &bool) -> bool {
    !*value
}

pub(super) fn event_cursor_filter(bound: &BoundQuery) -> Option<u64> {
    if !matches!(&bound.source, Source::Event { .. }) {
        return None;
    }
    bound.filters.iter().find_map(|filter| {
        (filter.field == "cursor" && filter.comparison == ComparisonOperator::Equal)
            .then_some(&filter.value)
            .and_then(|value| match value {
                RuntimeValue::Unsigned(cursor) => Some(*cursor),
                _ => None,
            })
    })
}
