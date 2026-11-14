//! Canonical encoding of values, mutations, schemas and organisation.

use super::*;

pub(super) fn encode_ref(out: &mut Vec<u8>, reference: &RuntimeRef) {
    text(out, reference.kind.as_str());
    text(out, reference.id.as_str());
}

pub(super) fn encode_window(out: &mut Vec<u8>, valid_from: Millis, valid_to: Option<Millis>) {
    out.extend_from_slice(&valid_from.to_be_bytes());
    out.push(u8::from(valid_to.is_some()));
    if let Some(valid_to) = valid_to {
        out.extend_from_slice(&valid_to.to_be_bytes());
    }
}

pub(super) fn encode_properties(out: &mut Vec<u8>, properties: &RuntimeProperties) {
    out.extend_from_slice(&(properties.len() as u64).to_be_bytes());
    for (key, value) in properties {
        text(out, key);
        encode_value(out, value);
    }
}

pub(super) fn encode_value(out: &mut Vec<u8>, value: &RuntimeValue) {
    match value {
        RuntimeValue::Null => out.push(0),
        RuntimeValue::Bool(value) => {
            out.push(1);
            out.push(u8::from(*value));
        }
        RuntimeValue::Integer(value) => {
            out.push(2);
            out.extend_from_slice(&value.to_be_bytes());
        }
        RuntimeValue::Unsigned(value) => {
            out.push(3);
            out.extend_from_slice(&value.to_be_bytes());
        }
        RuntimeValue::Decimal(value) => {
            out.push(4);
            text(out, value);
        }
        RuntimeValue::String(value) => {
            out.push(5);
            text(out, value);
        }
        RuntimeValue::Digest(value) => {
            out.push(6);
            text(out, value);
        }
        RuntimeValue::List(values) => {
            out.push(7);
            out.extend_from_slice(&(values.len() as u64).to_be_bytes());
            for value in values {
                encode_value(out, value);
            }
        }
        RuntimeValue::Map(values) => {
            out.push(8);
            encode_properties(out, values);
        }
    }
}

pub(super) fn encode_mutation(out: &mut Vec<u8>, mutation: &RuntimeMutation) {
    match mutation {
        RuntimeMutation::Claim { claim } => {
            out.push(0);
            let bytes = claim.canonical_bytes();
            out.extend_from_slice(&(bytes.len() as u64).to_be_bytes());
            out.extend_from_slice(&bytes);
        }
        RuntimeMutation::Schema { registry } => {
            out.push(4);
            encode_schema(out, registry);
        }
        RuntimeMutation::Record { record } => {
            out.push(1);
            encode_ref(out, &record.reference);
            encode_window(out, record.valid_from, record.valid_to);
            encode_properties(out, &record.properties);
        }
        RuntimeMutation::Relation { relation } => {
            out.push(2);
            encode_ref(out, &relation.reference);
            encode_ref(out, &relation.from);
            encode_ref(out, &relation.to);
            encode_window(out, relation.valid_from, relation.valid_to);
            encode_properties(out, &relation.properties);
        }
        RuntimeMutation::Event { event } => {
            out.push(3);
            text(out, event.kind.as_str());
            out.push(u8::from(event.subject.is_some()));
            if let Some(subject) = &event.subject {
                encode_ref(out, subject);
            }
            encode_properties(out, &event.properties);
        }
        RuntimeMutation::Vector { vector } => {
            out.push(5);
            encode_ref(out, &vector.reference);
            encode_ref(out, &vector.subject);
            text(out, &vector.collection.collection_id);
            text(out, &vector.collection.vector_name);
            text(out, &vector.field);
            encode_window(out, vector.valid_from, vector.valid_to);
            encode_vector_value(out, &vector.value);
            out.push(u8::from(vector.provenance.is_some()));
            if let Some(provenance) = &vector.provenance {
                text(out, &provenance.source_digest);
                text(out, &provenance.model);
                text(out, &provenance.model_digest);
                out.extend_from_slice(&provenance.dimensions.to_be_bytes());
                out.push(match provenance.normalization {
                    VectorNormalization::None => 0,
                    VectorNormalization::UnitL2 => 1,
                });
                encode_properties(out, &provenance.generation_parameters);
            }
            encode_properties(out, &vector.properties);
        }
        RuntimeMutation::SeriesSample { sample } => {
            out.push(6);
            encode_ref(out, &sample.reference);
            encode_ref(out, &sample.series);
            out.extend_from_slice(&sample.observed_at.to_be_bytes());
            match &sample.value {
                SeriesValue::Integer(value) => {
                    out.push(0);
                    out.extend_from_slice(&value.to_be_bytes());
                }
                SeriesValue::Unsigned(value) => {
                    out.push(1);
                    out.extend_from_slice(&value.to_be_bytes());
                }
                SeriesValue::Decimal(value) => {
                    out.push(2);
                    text(out, value);
                }
                SeriesValue::Bool(value) => {
                    out.push(3);
                    out.push(u8::from(*value));
                }
                SeriesValue::String(value) => {
                    out.push(4);
                    text(out, value);
                }
            }
            encode_properties(out, &sample.properties);
        }
        RuntimeMutation::Geo { geo } => {
            out.push(7);
            encode_ref(out, &geo.reference);
            encode_ref(out, &geo.subject);
            text(out, &geo.field);
            encode_window(out, geo.valid_from, geo.valid_to);
            match geo.value {
                GeoValue::Point { point } => {
                    out.push(0);
                    encode_f64(out, point.longitude);
                    encode_f64(out, point.latitude);
                }
                GeoValue::BoundingBox {
                    southwest,
                    northeast,
                } => {
                    out.push(1);
                    encode_f64(out, southwest.longitude);
                    encode_f64(out, southwest.latitude);
                    encode_f64(out, northeast.longitude);
                    encode_f64(out, northeast.latitude);
                }
            }
            encode_properties(out, &geo.properties);
        }
        RuntimeMutation::Object { object } => {
            out.push(8);
            encode_ref(out, &object.reference);
            out.push(u8::from(object.subject.is_some()));
            if let Some(subject) = &object.subject {
                encode_ref(out, subject);
            }
            text(out, &object.sha256);
            out.extend_from_slice(&object.length.to_be_bytes());
            text(out, &object.media_type);
            text(out, &object.receipt.backend);
            text(out, &object.receipt.key);
            optional_text(out, object.receipt.version.as_deref());
            optional_text(out, object.receipt.etag.as_deref());
            encode_properties(out, &object.properties);
        }
        RuntimeMutation::Retire { retirement } => {
            out.push(9);
            out.push(runtime_logical_model_tag(retirement.model));
            encode_ref(out, &retirement.reference);
            out.extend_from_slice(&retirement.effective_at.to_be_bytes());
        }
    }
}

pub(super) fn runtime_logical_model_tag(model: crate::RuntimeLogicalModel) -> u8 {
    match model {
        crate::RuntimeLogicalModel::Document => 0,
        crate::RuntimeLogicalModel::Relational => 1,
        crate::RuntimeLogicalModel::GraphNode => 2,
        crate::RuntimeLogicalModel::GraphRelation => 3,
        crate::RuntimeLogicalModel::KeyValue => 4,
        crate::RuntimeLogicalModel::Vector => 5,
        crate::RuntimeLogicalModel::Event => 6,
        crate::RuntimeLogicalModel::TimeSeries => 7,
        crate::RuntimeLogicalModel::Geo => 8,
        crate::RuntimeLogicalModel::Object => 9,
        crate::RuntimeLogicalModel::ReasoningRecord => 10,
        crate::RuntimeLogicalModel::ReasoningEvent => 11,
        crate::RuntimeLogicalModel::LifecycleRecord => 12,
        crate::RuntimeLogicalModel::LifecycleEvent => 13,
        crate::RuntimeLogicalModel::ReasoningClaim => 14,
    }
}

pub(super) fn encode_vector_value(out: &mut Vec<u8>, value: &VectorValue) {
    match value {
        VectorValue::Dense { values } => {
            out.push(0);
            encode_f32s(out, values);
        }
        VectorValue::Sparse {
            dimensions,
            indices,
            values,
        } => {
            out.push(1);
            out.extend_from_slice(&dimensions.to_be_bytes());
            out.extend_from_slice(&(indices.len() as u64).to_be_bytes());
            for index in indices {
                out.extend_from_slice(&index.to_be_bytes());
            }
            encode_f32s(out, values);
        }
        VectorValue::MultiDense {
            dimensions,
            vectors,
        } => {
            out.push(2);
            out.extend_from_slice(&dimensions.to_be_bytes());
            out.extend_from_slice(&(vectors.len() as u64).to_be_bytes());
            for vector in vectors {
                encode_f32s(out, vector);
            }
        }
    }
}

pub(super) fn encode_f32s(out: &mut Vec<u8>, values: &[f32]) {
    out.extend_from_slice(&(values.len() as u64).to_be_bytes());
    for value in values {
        out.extend_from_slice(&value.to_bits().to_be_bytes());
    }
}

pub(super) fn encode_f64(out: &mut Vec<u8>, value: f64) {
    out.extend_from_slice(&value.to_bits().to_be_bytes());
}

pub(super) fn encode_schema(out: &mut Vec<u8>, registry: &RuntimeSchemaRegistry) {
    out.extend_from_slice(&registry.revision.to_be_bytes());
    text(out, &registry.migration);
    out.extend_from_slice(&(registry.records.len() as u64).to_be_bytes());
    for (kind, schema) in &registry.records {
        text(out, kind.as_str());
        encode_property_schemas(out, &schema.properties);
        out.push(u8::from(schema.allow_additional_properties));
        out.extend_from_slice(&(schema.unique_properties.len() as u64).to_be_bytes());
        for property in &schema.unique_properties {
            text(out, property);
        }
    }
    out.extend_from_slice(&(registry.relations.len() as u64).to_be_bytes());
    for (kind, schema) in &registry.relations {
        text(out, kind.as_str());
        encode_type_set(out, &schema.from);
        encode_type_set(out, &schema.to);
        encode_property_schemas(out, &schema.properties);
        out.push(u8::from(schema.allow_additional_properties));
        out.push(u8::from(schema.unique_pair));
        encode_optional_u64(out, schema.max_outgoing);
        encode_optional_u64(out, schema.max_incoming);
    }
    out.extend_from_slice(&(registry.events.len() as u64).to_be_bytes());
    for (kind, schema) in &registry.events {
        text(out, kind.as_str());
        out.push(u8::from(schema.subject_required));
        encode_type_set(out, &schema.subject_types);
        encode_property_schemas(out, &schema.properties);
        out.push(u8::from(schema.allow_additional_properties));
    }
    out.extend_from_slice(b"rrflow-unified-catalogue-v1\0");
    // The v1 encoding's former namespace/database slot. Estate organisation
    // retired that label, and the slot keeps its only value so every registry
    // already committed keeps its digest.
    text(out, "default");
    text(out, "default");
    out.extend_from_slice(&(registry.tables.len() as u64).to_be_bytes());
    for (kind, table) in &registry.tables {
        text(out, kind.as_str());
        out.push(match table.model {
            crate::RuntimeLogicalModel::Document => 0,
            crate::RuntimeLogicalModel::Relational => 1,
            crate::RuntimeLogicalModel::GraphNode => 2,
            crate::RuntimeLogicalModel::GraphRelation => 3,
            crate::RuntimeLogicalModel::KeyValue => 4,
            crate::RuntimeLogicalModel::Vector => 5,
            crate::RuntimeLogicalModel::Event => 6,
            crate::RuntimeLogicalModel::TimeSeries => 7,
            crate::RuntimeLogicalModel::Geo => 8,
            crate::RuntimeLogicalModel::Object => 9,
            crate::RuntimeLogicalModel::ReasoningRecord => 10,
            crate::RuntimeLogicalModel::ReasoningEvent => 11,
            crate::RuntimeLogicalModel::LifecycleRecord => 12,
            crate::RuntimeLogicalModel::LifecycleEvent => 13,
            crate::RuntimeLogicalModel::ReasoningClaim => 14,
        });
        out.push(match table.mode {
            crate::RuntimeSchemaMode::Strict => 0,
            crate::RuntimeSchemaMode::Schemaless => 1,
        });
        encode_property_schemas(out, &table.properties);
        out.push(u8::from(table.allow_additional_properties));
    }
    if !registry.organisation.is_default() {
        encode_organisation(out, &registry.organisation);
    }
}

/// Appended only when the organisation is not the default. What can follow a
/// schema in a commit or change encoding is a mutation tag or an optional-text
/// flag, both below 0x20, so the 0xFF marker keeps the encoding unambiguous.
pub(super) fn encode_organisation(
    out: &mut Vec<u8>,
    organisation: &crate::RuntimeEstateOrganisation,
) {
    out.push(0xFF);
    out.extend_from_slice(b"rrflow-estate-organisation-v1\0");
    out.extend_from_slice(&(organisation.domains.len() as u64).to_be_bytes());
    for (id, domain) in &organisation.domains {
        text(out, id.as_str());
        text(out, &domain.title);
        text(out, &domain.description);
        optional_text(out, domain.parent.as_ref().map(RuntimeType::as_str));
    }
    out.extend_from_slice(&(organisation.categories.len() as u64).to_be_bytes());
    for (id, category) in &organisation.categories {
        text(out, id.as_str());
        text(out, &category.title);
        text(out, &category.description);
        out.push(match category.targets {
            crate::RuntimeCategoryTargets::Kinds => 0,
            crate::RuntimeCategoryTargets::Records => 1,
            crate::RuntimeCategoryTargets::KindsAndRecords => 2,
        });
        out.push(match category.cardinality {
            crate::RuntimeTagCardinality::Single => 0,
            crate::RuntimeTagCardinality::Multiple => 1,
        });
        out.push(u8::from(category.open));
        out.extend_from_slice(&(category.tags.len() as u64).to_be_bytes());
        for (tag, schema) in &category.tags {
            text(out, tag.as_str());
            text(out, &schema.title);
            text(out, &schema.description);
        }
    }
    for memberships in [&organisation.kind_domains, &organisation.collection_domains] {
        out.extend_from_slice(&(memberships.len() as u64).to_be_bytes());
        for (identity, domain) in memberships {
            text(out, identity.as_str());
            text(out, domain.as_str());
        }
    }
    out.extend_from_slice(&(organisation.kind_tags.len() as u64).to_be_bytes());
    for (kind, tags) in &organisation.kind_tags {
        text(out, kind.as_str());
        out.extend_from_slice(&(tags.len() as u64).to_be_bytes());
        for reference in tags {
            text(out, reference.category.as_str());
            text(out, reference.tag.as_str());
        }
    }
}

pub(super) fn encode_property_schemas(
    out: &mut Vec<u8>,
    properties: &BTreeMap<String, crate::RuntimePropertySchema>,
) {
    out.extend_from_slice(&(properties.len() as u64).to_be_bytes());
    for (name, schema) in properties {
        text(out, name);
        out.push(match schema.value_type {
            crate::RuntimeValueType::Null => 0,
            crate::RuntimeValueType::Bool => 1,
            crate::RuntimeValueType::Integer => 2,
            crate::RuntimeValueType::Unsigned => 3,
            crate::RuntimeValueType::Decimal => 4,
            crate::RuntimeValueType::String => 5,
            crate::RuntimeValueType::Digest => 6,
            crate::RuntimeValueType::List => 7,
            crate::RuntimeValueType::Map => 8,
        });
        out.push(u8::from(schema.required));
    }
}

pub(super) fn encode_type_set(out: &mut Vec<u8>, values: &BTreeSet<RuntimeType>) {
    out.extend_from_slice(&(values.len() as u64).to_be_bytes());
    for value in values {
        text(out, value.as_str());
    }
}

pub(super) fn encode_optional_u64(out: &mut Vec<u8>, value: Option<u64>) {
    out.push(u8::from(value.is_some()));
    if let Some(value) = value {
        out.extend_from_slice(&value.to_be_bytes());
    }
}
