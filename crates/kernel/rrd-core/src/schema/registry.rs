//! The runtime schema registry.

use super::*;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeSchemaRegistry {
    pub revision: u64,
    pub migration: String,
    #[serde(default, skip_serializing_if = "RuntimeEstateOrganisation::is_default")]
    pub organisation: RuntimeEstateOrganisation,
    /// Sole logical-model authority for every schema-bound table. Specialized
    /// maps below add family-specific constraints; they never create a table
    /// or infer its model.
    pub tables: BTreeMap<RuntimeType, RuntimeTableSchema>,
    #[serde(default)]
    pub records: BTreeMap<RuntimeType, RuntimeRecordSchema>,
    #[serde(default)]
    pub relations: BTreeMap<RuntimeType, RuntimeRelationSchema>,
    #[serde(default)]
    pub events: BTreeMap<RuntimeType, RuntimeEventSchema>,
}

impl RuntimeSchemaRegistry {
    pub fn empty(revision: u64, migration: impl Into<String>) -> Self {
        Self {
            revision,
            migration: migration.into(),
            organisation: RuntimeEstateOrganisation::default(),
            tables: BTreeMap::new(),
            records: BTreeMap::new(),
            relations: BTreeMap::new(),
            events: BTreeMap::new(),
        }
    }

    /// Defines one strict record-like table and its specialized constraints as
    /// one in-memory authoring operation. A new schema revision may replace an
    /// existing definition in the same family, but cannot silently change the
    /// mutation family represented by a table identity.
    pub fn define_record_table(
        &mut self,
        kind: RuntimeType,
        model: RuntimeLogicalModel,
        schema: RuntimeRecordSchema,
    ) -> Result<bool> {
        if !model.is_record_like() {
            return invalid(format!(
                "record table type {kind} cannot use non-record model {model:?}"
            ));
        }
        self.reject_family_conflict(&kind, RuntimeMutationFamily::Record)?;
        let table = RuntimeTableSchema::strict(model);
        let changed =
            self.tables.get(&kind) != Some(&table) || self.records.get(&kind) != Some(&schema);
        self.tables.insert(kind.clone(), table);
        self.records.insert(kind, schema);
        Ok(changed)
    }

    /// Defines one strict graph-relation table and its endpoint/cardinality
    /// constraints as one in-memory authoring operation.
    pub fn define_relation_table(
        &mut self,
        kind: RuntimeType,
        schema: RuntimeRelationSchema,
    ) -> Result<bool> {
        self.reject_family_conflict(&kind, RuntimeMutationFamily::Relation)?;
        let table = RuntimeTableSchema::strict(RuntimeLogicalModel::GraphRelation);
        let changed =
            self.tables.get(&kind) != Some(&table) || self.relations.get(&kind) != Some(&schema);
        self.tables.insert(kind.clone(), table);
        self.relations.insert(kind, schema);
        Ok(changed)
    }

    /// Defines one strict event-like table and its subject/property constraints
    /// as one in-memory authoring operation.
    pub fn define_event_table(
        &mut self,
        kind: RuntimeType,
        model: RuntimeLogicalModel,
        schema: RuntimeEventSchema,
    ) -> Result<bool> {
        if !model.is_event_like() {
            return invalid(format!(
                "event table type {kind} cannot use non-event model {model:?}"
            ));
        }
        self.reject_family_conflict(&kind, RuntimeMutationFamily::Event)?;
        let table = RuntimeTableSchema::strict(model);
        let changed =
            self.tables.get(&kind) != Some(&table) || self.events.get(&kind) != Some(&schema);
        self.tables.insert(kind.clone(), table);
        self.events.insert(kind, schema);
        Ok(changed)
    }

    fn reject_family_conflict(
        &self,
        kind: &RuntimeType,
        family: RuntimeMutationFamily,
    ) -> Result<()> {
        if self
            .tables
            .get(kind)
            .is_some_and(|table| table.model.family() != family)
            || (family != RuntimeMutationFamily::Record && self.records.contains_key(kind))
            || (family != RuntimeMutationFamily::Relation && self.relations.contains_key(kind))
            || (family != RuntimeMutationFamily::Event && self.events.contains_key(kind))
        {
            return invalid(format!(
                "table type {kind} already belongs to a different mutation family"
            ));
        }
        Ok(())
    }

    pub fn validate(&self) -> Result<()> {
        if self.revision == 0 {
            return invalid("runtime schema revision must be greater than zero");
        }
        if self.migration.trim().is_empty() {
            return invalid("runtime schema migration description must not be empty");
        }
        if self.tables.is_empty() {
            return invalid("runtime schema must declare an explicit canonical table map");
        }
        self.catalogue_tables()?;
        self.organisation.validate(&self.tables)?;
        for (kind, schema) in &self.records {
            validate_property_schema(kind, &schema.properties, schema.allow_additional_properties)?;
            for property in &schema.unique_properties {
                if !schema.properties.contains_key(property) {
                    return invalid(format!(
                        "record type {kind} declares unknown unique property {property:?}"
                    ));
                }
            }
        }
        for (kind, schema) in &self.relations {
            if schema.from.is_empty() || schema.to.is_empty() {
                return invalid(format!(
                    "relation type {kind} must declare at least one from and to endpoint type"
                ));
            }
            if schema.max_outgoing == Some(0) || schema.max_incoming == Some(0) {
                return invalid(format!(
                    "relation type {kind} cardinality limits must be greater than zero"
                ));
            }
            validate_property_schema(kind, &schema.properties, schema.allow_additional_properties)?;
        }
        for (kind, schema) in &self.events {
            validate_property_schema(kind, &schema.properties, schema.allow_additional_properties)?;
        }
        Ok(())
    }

    /// Validates and returns the exact authoritative table catalogue. No table
    /// identity or logical model is derived from specialized constraints.
    pub fn catalogue_tables(&self) -> Result<BTreeMap<RuntimeType, RuntimeTableSchema>> {
        for kind in self.records.keys() {
            validate_specialized_table(&self.tables, kind, RuntimeMutationFamily::Record)?;
        }
        for kind in self.relations.keys() {
            validate_specialized_table(&self.tables, kind, RuntimeMutationFamily::Relation)?;
        }
        for kind in self.events.keys() {
            validate_specialized_table(&self.tables, kind, RuntimeMutationFamily::Event)?;
        }
        for (kind, table) in &self.tables {
            validate_table(kind, table)?;
            let specialized = match table.model.family() {
                RuntimeMutationFamily::Record => self.records.contains_key(kind),
                RuntimeMutationFamily::Relation => self.relations.contains_key(kind),
                RuntimeMutationFamily::Event => self.events.contains_key(kind),
                RuntimeMutationFamily::Vector
                | RuntimeMutationFamily::Series
                | RuntimeMutationFamily::Geo
                | RuntimeMutationFamily::Object
                | RuntimeMutationFamily::Claim => false,
            };
            match (table.mode, table.model.family(), specialized) {
                (
                    RuntimeSchemaMode::Strict,
                    RuntimeMutationFamily::Record
                    | RuntimeMutationFamily::Relation
                    | RuntimeMutationFamily::Event,
                    false,
                ) => {
                    return invalid(format!(
                        "strict table type {kind} lacks its specialized {:?} schema",
                        table.model.family()
                    ));
                }
                (
                    RuntimeSchemaMode::Schemaless,
                    RuntimeMutationFamily::Record
                    | RuntimeMutationFamily::Relation
                    | RuntimeMutationFamily::Event,
                    true,
                ) => {
                    return invalid(format!(
                        "schemaless table type {kind} also declares a strict specialized schema"
                    ));
                }
                _ => {}
            }
        }
        Ok(self.tables.clone())
    }

    pub fn validate_objects<'a>(
        &self,
        mutations: impl IntoIterator<Item = &'a RuntimeMutation>,
        existing_records: impl IntoIterator<Item = &'a RuntimeRecord>,
        existing_relations: impl IntoIterator<Item = &'a RuntimeRelation>,
    ) -> Result<()> {
        self.validate()?;
        let tables = self.catalogue_tables()?;
        let mutations = mutations.into_iter().collect::<Vec<_>>();
        for mutation in &mutations {
            match mutation {
                RuntimeMutation::Claim { .. } | RuntimeMutation::Schema { .. } => {}
                RuntimeMutation::Retire { retirement } => {
                    let table = tables.get(&retirement.reference.kind).ok_or_else(|| {
                        Error::InvalidRuntime {
                            reason: format!(
                                "retirement table type {} is not registered in the catalogue",
                                retirement.reference.kind
                            ),
                        }
                    })?;
                    if table.model != retirement.model {
                        return invalid(format!(
                            "retirement target {}/{} is declared {:?}, not {:?}",
                            retirement.reference.kind,
                            retirement.reference.id,
                            table.model,
                            retirement.model
                        ));
                    }
                }
                RuntimeMutation::Record { record } => self.validate_record(record, &tables)?,
                RuntimeMutation::Relation { relation } => {
                    self.validate_relation(relation, &tables)?
                }
                RuntimeMutation::Event { event } => self.validate_event(event, &tables)?,
                RuntimeMutation::Vector { vector } => validate_structural_table(
                    "vector",
                    &vector.reference.kind,
                    &vector.properties,
                    RuntimeMutationFamily::Vector,
                    &tables,
                )?,
                RuntimeMutation::SeriesSample { sample } => validate_structural_table(
                    "time-series sample",
                    &sample.reference.kind,
                    &sample.properties,
                    RuntimeMutationFamily::Series,
                    &tables,
                )?,
                RuntimeMutation::Geo { geo } => validate_structural_table(
                    "geo value",
                    &geo.reference.kind,
                    &geo.properties,
                    RuntimeMutationFamily::Geo,
                    &tables,
                )?,
                RuntimeMutation::Object { object } => validate_structural_table(
                    "object",
                    &object.reference.kind,
                    &object.properties,
                    RuntimeMutationFamily::Object,
                    &tables,
                )?,
            }
        }

        let mut records = existing_records
            .into_iter()
            .map(|record| (record.reference.clone(), record.clone()))
            .collect::<BTreeMap<_, _>>();
        let mut relations = existing_relations
            .into_iter()
            .map(|relation| (relation.reference.clone(), relation.clone()))
            .collect::<BTreeMap<_, _>>();
        for mutation in mutations {
            match mutation {
                RuntimeMutation::Record { record } => {
                    records.insert(record.reference.clone(), record.clone());
                }
                RuntimeMutation::Relation { relation } => {
                    relations.insert(relation.reference.clone(), relation.clone());
                }
                RuntimeMutation::Claim { .. }
                | RuntimeMutation::Event { .. }
                | RuntimeMutation::Schema { .. }
                | RuntimeMutation::Vector { .. }
                | RuntimeMutation::SeriesSample { .. }
                | RuntimeMutation::Geo { .. }
                | RuntimeMutation::Object { .. }
                | RuntimeMutation::Retire { .. } => {}
            }
        }
        self.validate_record_uniqueness(records.values())?;
        self.validate_relation_cardinality(relations.values())?;
        Ok(())
    }

    fn validate_record(
        &self,
        record: &RuntimeRecord,
        tables: &BTreeMap<RuntimeType, RuntimeTableSchema>,
    ) -> Result<()> {
        let table = require_table(
            &record.reference.kind,
            RuntimeMutationFamily::Record,
            tables,
        )?;
        if table.mode == RuntimeSchemaMode::Schemaless {
            return Ok(());
        }
        let schema =
            self.records
                .get(&record.reference.kind)
                .ok_or_else(|| Error::InvalidRuntime {
                    reason: format!("record type {} is not registered", record.reference.kind),
                })?;
        validate_properties(
            "record",
            &record.reference.kind,
            &record.properties,
            &schema.properties,
            schema.allow_additional_properties,
        )
    }

    fn validate_relation(
        &self,
        relation: &RuntimeRelation,
        tables: &BTreeMap<RuntimeType, RuntimeTableSchema>,
    ) -> Result<()> {
        let table = require_table(
            &relation.reference.kind,
            RuntimeMutationFamily::Relation,
            tables,
        )?;
        if table.mode == RuntimeSchemaMode::Schemaless {
            return Ok(());
        }
        let schema = self
            .relations
            .get(&relation.reference.kind)
            .ok_or_else(|| Error::InvalidRuntime {
                reason: format!(
                    "relation type {} is not registered",
                    relation.reference.kind
                ),
            })?;
        if !schema.from.contains(&relation.from.kind) || !schema.to.contains(&relation.to.kind) {
            return invalid(format!(
                "relation type {} rejects endpoint {} -> {}",
                relation.reference.kind, relation.from.kind, relation.to.kind
            ));
        }
        validate_properties(
            "relation",
            &relation.reference.kind,
            &relation.properties,
            &schema.properties,
            schema.allow_additional_properties,
        )
    }

    fn validate_event(
        &self,
        event: &RuntimeEvent,
        tables: &BTreeMap<RuntimeType, RuntimeTableSchema>,
    ) -> Result<()> {
        let table = require_table(&event.kind, RuntimeMutationFamily::Event, tables)?;
        if table.mode == RuntimeSchemaMode::Schemaless {
            return Ok(());
        }
        let schema = self
            .events
            .get(&event.kind)
            .ok_or_else(|| Error::InvalidRuntime {
                reason: format!("event type {} is not registered", event.kind),
            })?;
        if schema.subject_required && event.subject.is_none() {
            return invalid(format!("event type {} requires a subject", event.kind));
        }
        if let Some(subject) = &event.subject {
            if !schema.subject_types.is_empty() && !schema.subject_types.contains(&subject.kind) {
                return invalid(format!(
                    "event type {} rejects subject type {}",
                    event.kind, subject.kind
                ));
            }
        }
        validate_properties(
            "event",
            &event.kind,
            &event.properties,
            &schema.properties,
            schema.allow_additional_properties,
        )
    }

    fn validate_record_uniqueness<'a>(
        &self,
        records: impl IntoIterator<Item = &'a RuntimeRecord>,
    ) -> Result<()> {
        let records = records.into_iter().collect::<Vec<_>>();
        for (kind, schema) in &self.records {
            for property in &schema.unique_properties {
                let candidates = records
                    .iter()
                    .filter(|record| &record.reference.kind == kind)
                    .filter_map(|record| {
                        record
                            .properties
                            .get(property)
                            .map(|value| (*record, value))
                    })
                    .collect::<Vec<_>>();
                for (index, (left, value)) in candidates.iter().enumerate() {
                    if candidates[index + 1..].iter().any(|(right, other)| {
                        *value == *other
                            && windows_overlap(
                                left.valid_from,
                                left.valid_to,
                                right.valid_from,
                                right.valid_to,
                            )
                    }) {
                        return invalid(format!(
                            "record type {kind} property {property:?} is not unique for overlapping validity windows"
                        ));
                    }
                }
            }
        }
        Ok(())
    }

    fn validate_relation_cardinality<'a>(
        &self,
        relations: impl IntoIterator<Item = &'a RuntimeRelation>,
    ) -> Result<()> {
        let relations = relations.into_iter().collect::<Vec<_>>();
        for (kind, schema) in &self.relations {
            let typed = relations
                .iter()
                .filter(|relation| &relation.reference.kind == kind)
                .copied()
                .collect::<Vec<_>>();
            if schema.unique_pair {
                let mut groups = BTreeMap::new();
                for relation in &typed {
                    groups
                        .entry((relation.from.clone(), relation.to.clone()))
                        .or_insert_with(Vec::new)
                        .push(*relation);
                }
                for ((from, to), group) in groups {
                    if max_overlap(&group) > 1 {
                        return invalid(format!(
                            "relation type {kind} requires a unique pair for {}/{} -> {}/{}",
                            from.kind, from.id, to.kind, to.id
                        ));
                    }
                }
            }
            if let Some(limit) = schema.max_outgoing {
                let mut groups = BTreeMap::new();
                for relation in &typed {
                    groups
                        .entry(relation.from.clone())
                        .or_insert_with(Vec::new)
                        .push(*relation);
                }
                for (from, group) in groups {
                    if max_overlap(&group) > limit {
                        return invalid(format!(
                            "relation type {kind} exceeds max_outgoing={limit} at {}/{}",
                            from.kind, from.id
                        ));
                    }
                }
            }
            if let Some(limit) = schema.max_incoming {
                let mut groups = BTreeMap::new();
                for relation in &typed {
                    groups
                        .entry(relation.to.clone())
                        .or_insert_with(Vec::new)
                        .push(*relation);
                }
                for (to, group) in groups {
                    if max_overlap(&group) > limit {
                        return invalid(format!(
                            "relation type {kind} exceeds max_incoming={limit} at {}/{}",
                            to.kind, to.id
                        ));
                    }
                }
            }
        }
        Ok(())
    }
}
