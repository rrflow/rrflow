//! Provider-neutral bootstrap knowledge-package contracts.
//!
//! These types freeze a deterministic package for later authorized import.
//! They do not read Markdown, inspect a checkout, open storage, or mutate an
//! RRFlow engram. KB-03 owns export and KB-06 owns import through `RrdEngine`.

use crate::{invalid, sha256_bytes, validate_sha256, CanonicalId, Result};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub const KNOWLEDGE_PACKAGE_CONTRACT_VERSION: u16 = 1;
pub const MAX_KNOWLEDGE_RECORDS: usize = 100_000;
pub const MAX_KNOWLEDGE_EXCLUSIONS: usize = 100_000;
pub const MAX_KNOWLEDGE_SOURCE_PATH_BYTES: usize = 4_096;
pub const MAX_KNOWLEDGE_COORDINATE_BYTES: usize = 1_024;
pub const MAX_KNOWLEDGE_BODY_BYTES: usize = 4 * 1024 * 1024;
pub const MAX_KNOWLEDGE_PACKAGE_BODY_BYTES: usize = 512 * 1024 * 1024;
pub const MAX_KNOWLEDGE_REVISION_BYTES: usize = 256;
pub const MAX_KNOWLEDGE_EXCLUSION_REASON_BYTES: usize = 4_096;

/// The owning documentation taxonomy, independent of lifecycle wording in a
/// record's human-readable status line.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum KnowledgeClassification {
    Portal,
    Architecture,
    Decision,
    Objective,
    Roadmap,
    Poam,
    Reference,
    Guide,
    Operations,
    Research,
    Evidence,
    History,
}

impl KnowledgeClassification {
    const fn wire_name(self) -> &'static str {
        match self {
            Self::Portal => "portal",
            Self::Architecture => "architecture",
            Self::Decision => "decision",
            Self::Objective => "objective",
            Self::Roadmap => "roadmap",
            Self::Poam => "poam",
            Self::Reference => "reference",
            Self::Guide => "guide",
            Self::Operations => "operations",
            Self::Research => "research",
            Self::Evidence => "evidence",
            Self::History => "history",
        }
    }
}

/// Reproducible source identity for one package and every included record.
/// Repository and exporter are logical identifiers, never provider names,
/// credentials, checkout roots, or host-specific absolute paths.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct KnowledgeProvenance {
    pub repository: CanonicalId,
    pub revision: String,
    pub exporter: CanonicalId,
    pub exporter_version: u16,
}

impl KnowledgeProvenance {
    pub fn validate(&self) -> Result<()> {
        validate_ascii_token(
            &self.revision,
            "knowledge provenance revision",
            MAX_KNOWLEDGE_REVISION_BYTES,
        )?;
        if self.exporter_version == 0 {
            return invalid("knowledge exporter_version must be greater than zero");
        }
        Ok(())
    }
}

/// One normalized Markdown record ready for later authorized import.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct KnowledgeRecordV1 {
    pub contract_version: u16,
    pub coordinate: String,
    pub source_path: String,
    pub classification: KnowledgeClassification,
    pub owner_coordinate: String,
    pub body: String,
    pub body_sha256: String,
    pub provenance: KnowledgeProvenance,
    pub record_sha256: String,
}

impl KnowledgeRecordV1 {
    pub fn validate(&self) -> Result<()> {
        validate_version(self.contract_version)?;
        validate_coordinate(&self.coordinate, "knowledge record coordinate")?;
        validate_coordinate(&self.owner_coordinate, "knowledge record owner_coordinate")?;
        validate_source_path(&self.source_path)?;
        validate_normalized_body(&self.body)?;
        validate_sha256(&self.body_sha256, "knowledge record body_sha256")?;
        if sha256_bytes(self.body.as_bytes()) != self.body_sha256 {
            return invalid("knowledge record body digest does not match normalized body");
        }
        self.provenance.validate()?;
        validate_sha256(&self.record_sha256, "knowledge record_sha256")?;
        if knowledge_record_sha256(self)? != self.record_sha256 {
            return invalid("knowledge record digest does not match its content");
        }
        Ok(())
    }
}

/// Manifest disposition for every discovered source. This inventory is what
/// makes a silently omitted eligible document detectable.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "disposition", rename_all = "snake_case", deny_unknown_fields)]
pub enum KnowledgeManifestDispositionV1 {
    Included {
        coordinate: String,
        classification: KnowledgeClassification,
        record_sha256: String,
    },
    Excluded {
        reason_code: CanonicalId,
    },
}

/// One candidate source discovered by the exporter before inclusion rules are
/// applied. The caller supplies this independently so dropping a source from
/// both the package and its manifest cannot validate silently.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct KnowledgeSourceInventoryEntryV1 {
    pub source_path: String,
    pub source_sha256: String,
}

impl KnowledgeSourceInventoryEntryV1 {
    fn validate(&self) -> Result<()> {
        validate_source_path(&self.source_path)?;
        validate_sha256(
            &self.source_sha256,
            "knowledge source inventory source_sha256",
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct KnowledgeManifestEntryV1 {
    pub source_path: String,
    pub source_sha256: String,
    pub disposition: KnowledgeManifestDispositionV1,
}

impl KnowledgeManifestEntryV1 {
    pub fn included(record: &KnowledgeRecordV1) -> Self {
        Self {
            source_path: record.source_path.clone(),
            source_sha256: record.body_sha256.clone(),
            disposition: KnowledgeManifestDispositionV1::Included {
                coordinate: record.coordinate.clone(),
                classification: record.classification,
                record_sha256: record.record_sha256.clone(),
            },
        }
    }

    pub fn excluded(exclusion: &KnowledgeExclusionV1) -> Self {
        Self {
            source_path: exclusion.source_path.clone(),
            source_sha256: exclusion.source_sha256.clone(),
            disposition: KnowledgeManifestDispositionV1::Excluded {
                reason_code: exclusion.reason_code.clone(),
            },
        }
    }

    fn validate(&self) -> Result<()> {
        validate_source_path(&self.source_path)?;
        validate_sha256(&self.source_sha256, "knowledge manifest source_sha256")?;
        match &self.disposition {
            KnowledgeManifestDispositionV1::Included {
                coordinate,
                record_sha256,
                ..
            } => {
                validate_coordinate(coordinate, "knowledge manifest coordinate")?;
                validate_sha256(record_sha256, "knowledge manifest record_sha256")
            }
            KnowledgeManifestDispositionV1::Excluded { .. } => Ok(()),
        }
    }
}

/// One explicit reason why a discovered source is outside the package.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct KnowledgeExclusionV1 {
    pub source_path: String,
    pub source_sha256: String,
    pub reason_code: CanonicalId,
    pub reason: String,
}

impl KnowledgeExclusionV1 {
    pub fn validate(&self) -> Result<()> {
        validate_source_path(&self.source_path)?;
        validate_sha256(&self.source_sha256, "knowledge exclusion source_sha256")?;
        validate_text(
            &self.reason,
            "knowledge exclusion reason",
            MAX_KNOWLEDGE_EXCLUSION_REASON_BYTES,
        )
    }
}

/// A complete deterministic bootstrap package. `manifest` is the discovered
/// source inventory; every entry resolves to exactly one record or exclusion.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct KnowledgePackageV1 {
    pub contract_version: u16,
    pub id: CanonicalId,
    pub provenance: KnowledgeProvenance,
    pub manifest: Vec<KnowledgeManifestEntryV1>,
    pub records: Vec<KnowledgeRecordV1>,
    pub exclusions: Vec<KnowledgeExclusionV1>,
    pub package_sha256: String,
}

impl KnowledgePackageV1 {
    pub fn validate(&self) -> Result<()> {
        validate_version(self.contract_version)?;
        self.provenance.validate()?;
        if self.records.is_empty() || self.records.len() > MAX_KNOWLEDGE_RECORDS {
            return invalid("knowledge package record count is outside its bound");
        }
        if self.exclusions.len() > MAX_KNOWLEDGE_EXCLUSIONS {
            return invalid("knowledge package exclusion count exceeds its bound");
        }
        if self.manifest.len() != self.records.len() + self.exclusions.len() {
            return invalid(
                "knowledge manifest must cover every included and excluded source exactly once",
            );
        }

        let mut body_bytes = 0_usize;
        let mut records_by_path = BTreeMap::new();
        let mut record_coordinates = BTreeSet::new();
        let mut previous_record_key: Option<(&str, &str)> = None;
        for record in &self.records {
            record.validate()?;
            if record.contract_version != self.contract_version
                || record.provenance != self.provenance
            {
                return invalid("knowledge record contract/provenance differs from its package");
            }
            let key = (record.coordinate.as_str(), record.source_path.as_str());
            if previous_record_key.is_some_and(|previous| previous >= key) {
                return invalid("knowledge records must be unique and sorted by coordinate/path");
            }
            previous_record_key = Some(key);
            if !record_coordinates.insert(record.coordinate.as_str())
                || records_by_path
                    .insert(record.source_path.as_str(), record)
                    .is_some()
            {
                return invalid("knowledge record coordinates and source paths must be unique");
            }
            body_bytes = body_bytes
                .checked_add(record.body.len())
                .ok_or_else(|| crate::ContractError("knowledge body size overflowed".into()))?;
        }
        if body_bytes > MAX_KNOWLEDGE_PACKAGE_BODY_BYTES {
            return invalid("knowledge package bodies exceed the package byte bound");
        }

        let mut exclusions_by_path = BTreeMap::new();
        let mut previous_exclusion_path: Option<&str> = None;
        for exclusion in &self.exclusions {
            exclusion.validate()?;
            if previous_exclusion_path
                .is_some_and(|previous| previous >= exclusion.source_path.as_str())
            {
                return invalid("knowledge exclusions must be unique and sorted by source path");
            }
            previous_exclusion_path = Some(&exclusion.source_path);
            if records_by_path.contains_key(exclusion.source_path.as_str())
                || exclusions_by_path
                    .insert(exclusion.source_path.as_str(), exclusion)
                    .is_some()
            {
                return invalid(
                    "knowledge sources cannot be duplicated or both included and excluded",
                );
            }
        }

        let mut previous_manifest_path: Option<&str> = None;
        for entry in &self.manifest {
            entry.validate()?;
            if previous_manifest_path.is_some_and(|previous| previous >= entry.source_path.as_str())
            {
                return invalid("knowledge manifest must be unique and sorted by source path");
            }
            previous_manifest_path = Some(&entry.source_path);
            match &entry.disposition {
                KnowledgeManifestDispositionV1::Included {
                    coordinate,
                    classification,
                    record_sha256,
                } => {
                    let Some(record) = records_by_path.get(entry.source_path.as_str()) else {
                        return invalid("knowledge manifest includes a source without its record");
                    };
                    if entry.source_sha256 != record.body_sha256
                        || coordinate != &record.coordinate
                        || classification != &record.classification
                        || record_sha256 != &record.record_sha256
                    {
                        return invalid("knowledge manifest inclusion differs from its record");
                    }
                }
                KnowledgeManifestDispositionV1::Excluded { reason_code } => {
                    let Some(exclusion) = exclusions_by_path.get(entry.source_path.as_str()) else {
                        return invalid("knowledge manifest excludes a source without a reason");
                    };
                    if entry.source_sha256 != exclusion.source_sha256
                        || reason_code != &exclusion.reason_code
                    {
                        return invalid("knowledge manifest exclusion differs from its ledger");
                    }
                }
            }
        }

        validate_sha256(&self.package_sha256, "knowledge package_sha256")?;
        if knowledge_package_sha256(self)? != self.package_sha256 {
            return invalid("knowledge package digest does not match its content");
        }
        Ok(())
    }

    /// Validate the package against the complete independently discovered
    /// source inventory. KB-03 must call this before emitting package bytes.
    pub fn validate_against_inventory(
        &self,
        inventory: &[KnowledgeSourceInventoryEntryV1],
    ) -> Result<()> {
        self.validate()?;
        if inventory.len() != self.manifest.len() {
            return invalid(
                "knowledge package omits or invents a discovered source inventory entry",
            );
        }
        let mut previous_path: Option<&str> = None;
        for (source, manifest) in inventory.iter().zip(&self.manifest) {
            source.validate()?;
            if previous_path.is_some_and(|previous| previous >= source.source_path.as_str()) {
                return invalid("knowledge source inventory must be unique and sorted by path");
            }
            previous_path = Some(&source.source_path);
            if source.source_path != manifest.source_path
                || source.source_sha256 != manifest.source_sha256
            {
                return invalid("knowledge package manifest differs from discovered sources");
            }
        }
        Ok(())
    }
}

/// SHA-256 over a versioned, length-framed record representation. Every field
/// boundary is an unsigned 64-bit big-endian length followed by exact bytes.
pub fn knowledge_record_sha256(record: &KnowledgeRecordV1) -> Result<String> {
    let mut encoded = domain_bytes(b"rrflow-knowledge-record-v1");
    push_frame(&mut encoded, &record.contract_version.to_be_bytes())?;
    push_frame(&mut encoded, record.coordinate.as_bytes())?;
    push_frame(&mut encoded, record.source_path.as_bytes())?;
    push_frame(&mut encoded, record.classification.wire_name().as_bytes())?;
    push_frame(&mut encoded, record.owner_coordinate.as_bytes())?;
    push_frame(&mut encoded, record.body.as_bytes())?;
    push_frame(&mut encoded, record.body_sha256.as_bytes())?;
    push_frame(&mut encoded, &provenance_bytes(&record.provenance)?)?;
    Ok(sha256_bytes(&encoded))
}

/// SHA-256 over the complete package, including source inventory, records,
/// and exclusions. Nested entries use the same unambiguous framing.
pub fn knowledge_package_sha256(package: &KnowledgePackageV1) -> Result<String> {
    let mut encoded = domain_bytes(b"rrflow-knowledge-package-v1");
    push_frame(&mut encoded, &package.contract_version.to_be_bytes())?;
    push_frame(&mut encoded, package.id.as_str().as_bytes())?;
    push_frame(&mut encoded, &provenance_bytes(&package.provenance)?)?;
    push_collection(&mut encoded, &package.manifest, manifest_entry_bytes)?;
    push_collection(&mut encoded, &package.records, record_bytes)?;
    push_collection(&mut encoded, &package.exclusions, exclusion_bytes)?;
    Ok(sha256_bytes(&encoded))
}

fn provenance_bytes(provenance: &KnowledgeProvenance) -> Result<Vec<u8>> {
    let mut encoded = Vec::new();
    push_frame(&mut encoded, provenance.repository.as_str().as_bytes())?;
    push_frame(&mut encoded, provenance.revision.as_bytes())?;
    push_frame(&mut encoded, provenance.exporter.as_str().as_bytes())?;
    push_frame(&mut encoded, &provenance.exporter_version.to_be_bytes())?;
    Ok(encoded)
}

fn manifest_entry_bytes(entry: &KnowledgeManifestEntryV1) -> Result<Vec<u8>> {
    let mut encoded = Vec::new();
    push_frame(&mut encoded, entry.source_path.as_bytes())?;
    push_frame(&mut encoded, entry.source_sha256.as_bytes())?;
    match &entry.disposition {
        KnowledgeManifestDispositionV1::Included {
            coordinate,
            classification,
            record_sha256,
        } => {
            push_frame(&mut encoded, b"included")?;
            push_frame(&mut encoded, coordinate.as_bytes())?;
            push_frame(&mut encoded, classification.wire_name().as_bytes())?;
            push_frame(&mut encoded, record_sha256.as_bytes())?;
        }
        KnowledgeManifestDispositionV1::Excluded { reason_code } => {
            push_frame(&mut encoded, b"excluded")?;
            push_frame(&mut encoded, reason_code.as_str().as_bytes())?;
        }
    }
    Ok(encoded)
}

fn record_bytes(record: &KnowledgeRecordV1) -> Result<Vec<u8>> {
    let mut encoded = Vec::new();
    push_frame(&mut encoded, &record.contract_version.to_be_bytes())?;
    push_frame(&mut encoded, record.coordinate.as_bytes())?;
    push_frame(&mut encoded, record.source_path.as_bytes())?;
    push_frame(&mut encoded, record.classification.wire_name().as_bytes())?;
    push_frame(&mut encoded, record.owner_coordinate.as_bytes())?;
    push_frame(&mut encoded, record.body.as_bytes())?;
    push_frame(&mut encoded, record.body_sha256.as_bytes())?;
    push_frame(&mut encoded, &provenance_bytes(&record.provenance)?)?;
    push_frame(&mut encoded, record.record_sha256.as_bytes())?;
    Ok(encoded)
}

fn exclusion_bytes(exclusion: &KnowledgeExclusionV1) -> Result<Vec<u8>> {
    let mut encoded = Vec::new();
    push_frame(&mut encoded, exclusion.source_path.as_bytes())?;
    push_frame(&mut encoded, exclusion.source_sha256.as_bytes())?;
    push_frame(&mut encoded, exclusion.reason_code.as_str().as_bytes())?;
    push_frame(&mut encoded, exclusion.reason.as_bytes())?;
    Ok(encoded)
}

fn push_collection<T>(
    output: &mut Vec<u8>,
    values: &[T],
    encode: fn(&T) -> Result<Vec<u8>>,
) -> Result<()> {
    let count = u64::try_from(values.len())
        .map_err(|_| crate::ContractError("knowledge collection length overflowed".into()))?;
    push_frame(output, &count.to_be_bytes())?;
    for value in values {
        push_frame(output, &encode(value)?)?;
    }
    Ok(())
}

fn domain_bytes(domain: &[u8]) -> Vec<u8> {
    let mut encoded = Vec::with_capacity(domain.len() + 1);
    encoded.extend_from_slice(domain);
    encoded.push(0);
    encoded
}

fn push_frame(output: &mut Vec<u8>, value: &[u8]) -> Result<()> {
    let length = u64::try_from(value.len())
        .map_err(|_| crate::ContractError("knowledge digest frame length overflowed".into()))?;
    output.extend_from_slice(&length.to_be_bytes());
    output.extend_from_slice(value);
    Ok(())
}

fn validate_version(version: u16) -> Result<()> {
    if version != KNOWLEDGE_PACKAGE_CONTRACT_VERSION {
        return invalid(format!(
            "unsupported knowledge package contract version {version}; expected {KNOWLEDGE_PACKAGE_CONTRACT_VERSION}"
        ));
    }
    Ok(())
}

fn validate_coordinate(value: &str, field: &str) -> Result<()> {
    if value.len() > MAX_KNOWLEDGE_COORDINATE_BYTES
        || value.contains(['\0', '\\', '?', '#'])
        || value.trim() != value
    {
        return invalid(format!(
            "{field} is not a bounded canonical RRFlow coordinate"
        ));
    }
    let Some(path) = value.strip_prefix("rrflow://") else {
        return invalid(format!("{field} must use the rrflow URI scheme"));
    };
    let segments = path.split('/').collect::<Vec<_>>();
    if segments.len() < 4 || segments[1] != "data" {
        return invalid(format!(
            "{field} must be rrflow://<instance>/data/<canonical-path>/<record-id>"
        ));
    }
    for segment in std::iter::once(segments[0]).chain(segments[2..].iter().copied()) {
        CanonicalId::new(segment).map_err(|_| {
            crate::ContractError(format!("{field} contains a non-canonical path segment"))
        })?;
    }
    Ok(())
}

pub(crate) fn validate_source_path(value: &str) -> Result<()> {
    if value.is_empty()
        || value.len() > MAX_KNOWLEDGE_SOURCE_PATH_BYTES
        || value.starts_with('/')
        || value.ends_with('/')
        || value.contains(['\0', '\\'])
        || value.chars().any(char::is_control)
    {
        return invalid("knowledge source_path is not a bounded normalized relative path");
    }
    let mut segments = value.split('/');
    if segments.any(|segment| segment.is_empty() || matches!(segment, "." | "..")) {
        return invalid("knowledge source_path contains an unsafe or non-normalized segment");
    }
    Ok(())
}

fn validate_normalized_body(value: &str) -> Result<()> {
    if value.is_empty()
        || value.len() > MAX_KNOWLEDGE_BODY_BYTES
        || !value.ends_with('\n')
        || value.contains(['\0', '\r'])
    {
        return invalid(
            "knowledge body must be non-empty normalized UTF-8/LF with a final newline and no NUL",
        );
    }
    Ok(())
}

fn validate_ascii_token(value: &str, field: &str, maximum: usize) -> Result<()> {
    if value.is_empty()
        || value.len() > maximum
        || value.trim() != value
        || !value.bytes().all(|byte| byte.is_ascii_graphic())
    {
        return invalid(format!(
            "{field} must be a non-empty bounded printable ASCII token"
        ));
    }
    Ok(())
}

fn validate_text(value: &str, field: &str, maximum: usize) -> Result<()> {
    if value.trim().is_empty() || value.len() > maximum || value.contains(['\0', '\r']) {
        return invalid(format!(
            "{field} must be non-empty normalized UTF-8/LF with at most {maximum} bytes"
        ));
    }
    Ok(())
}
