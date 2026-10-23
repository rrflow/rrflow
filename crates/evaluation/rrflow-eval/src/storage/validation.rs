use super::*;
use rrd_contract::{endpoint_catalogue, CapabilityStatus};
use schemars::JsonSchema;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use sha2::{Digest as _, Sha256};
use std::collections::{BTreeMap, BTreeSet};

/// Domain-separated digest of canonical typed JSON, independent of input whitespace.
pub fn digest<T: Serialize>(domain: &str, value: &T) -> Result<Digest> {
    let mut hasher = Sha256::new();
    hasher.update(domain.as_bytes());
    hasher.update([0]);
    hasher.update(serde_json::to_vec(value).map_err(|e| e.to_string())?);
    Digest::try_from(
        hasher
            .finalize()
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>(),
    )
}

pub fn catalogue_digest() -> Result<Digest> {
    digest("rrflow-eval-public-catalogue-v1", &endpoint_catalogue())
}
pub fn manifest_digest(manifest: &StorageManifest) -> Result<Digest> {
    digest("rrflow-eval-storage-manifest-v1", manifest)
}
pub fn expected_digest(expected: &[ExpectedOperation]) -> Result<Digest> {
    digest("rrflow-eval-expected-ledger-v1", &expected)
}

pub fn decode<T: DeserializeOwned>(bytes: &[u8]) -> Result<T> {
    require(
        bytes.len() as u64 <= MAX_DOCUMENT_BYTES,
        "document exceeds 8 MiB bound",
    )?;
    // Serde's display can echo an attacker-sized field or credential. Keep
    // location/category, never the rejected input, in the CLI diagnostic.
    serde_json::from_slice(bytes).map_err(|e| {
        format!(
            "invalid evidence JSON at line {} column {} ({:?})",
            e.line(),
            e.column(),
            e.classify()
        )
    })
}

pub fn schema<T: JsonSchema>() -> serde_json::Value {
    serde_json::to_value(
        schemars::generate::SchemaSettings::draft2020_12()
            .into_generator()
            .into_root_schema_for::<T>(),
    )
    .expect("schema serializes")
}

impl StorageManifest {
    pub fn validate(&self) -> Result<()> {
        require(
            self.format_version == 1,
            "unsupported storage manifest version",
        )?;
        require(
            !matches!(
                self.evidence_class,
                EvidenceClass::NativeProcess | EvidenceClass::NativeCapacity
            ) || self.durability == DurabilityClass::Authoritative,
            "native crash/capacity history requires authoritative durability",
        )?;
        for revision in [&self.candidate.commit, &self.candidate.tree] {
            require(
                revision.len() == 40
                    && revision
                        .bytes()
                        .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)),
                "source identity must be a lowercase Git SHA-1",
            )?;
        }
        require(
            self.candidate.clean,
            "qualification requires clean source identity",
        )?;
        require(
            self.candidate.public_catalogue_sha256 == catalogue_digest()?,
            "public catalogue identity differs from validator",
        )?;
        self.installed_configuration
            .validate()
            .map_err(|e| e.to_string())?;
        require(
            self.installed_configuration.configuration_sha256
                == self.candidate.configuration_sha256.as_str(),
            "installed configuration identity mismatch",
        )?;
        for text in [
            &self.environment.target,
            &self.environment.toolchain,
            &self.environment.cpu,
            &self.environment.os_version,
            &self.environment.filesystem_device,
            &self.environment.mount_options,
            &self.environment.flush_primitive,
            &self.environment.measurement_tool,
        ] {
            require(
                !text.trim().is_empty()
                    && text.len() <= 256
                    && text.bytes().all(|b| b.is_ascii_graphic() || b == b' '),
                "missing or invalid native environment identity",
            )?;
        }
        let target: Vec<_> = self.environment.target.split('-').collect();
        let target_matches = target.len() >= 3
            && target.iter().all(|part| {
                !part.is_empty()
                    && part
                        .bytes()
                        .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
            })
            && match self.environment.platform {
                NativePlatform::LinuxExt4 => target[2] == "linux",
                NativePlatform::MacosApfs => target[1] == "apple" && target[2] == "darwin",
                NativePlatform::WindowsNtfs => target[2] == "windows",
            };
        require(
            target_matches,
            "native target and filesystem profile disagree",
        )?;
        self.budgets.validate()?;
        require(
            self.budgets.recovery_reserve_bytes.get()
                == self.installed_configuration.storage.recovery_reserve_bytes,
            "declared recovery reserve differs from installed native admission",
        )?;
        require(
            self.budgets
                .run_ms
                .get()
                .checked_mul(1_000_000)
                .is_some_and(|n| n <= MAX_EXACT_INTEGER),
            "nanosecond run duration exceeds exact evidence integer",
        )?;
        let settings = self.effective_settings()?;
        require(
            settings.iter().all(|s| s.value <= MAX_EXACT_INTEGER),
            "observed setting exceeds exact evidence integer",
        )?;
        let kv = self
            .storage_observation
            .rrflow_kv
            .as_ref()
            .ok_or("missing physical storage observation")?;
        require(
            kv.write_ahead_log.payload_max_bytes > 0
                && kv.memtable.max_versions > 0
                && kv.segment_io.max_request_bytes > 0
                && kv.segment_set.l0_compaction_trigger >= 2
                && kv.segment_set.compaction_target_segment_bytes > 0,
            "invalid observed storage policy",
        )?;
        require(
            kv.page_cache.capacity_bytes <= self.budgets.cache_bytes.get(),
            "observed cache exceeds declared reservation",
        )?;
        require(
            kv.segment_io.max_request_bytes <= self.budgets.decoded_bytes.get(),
            "observed I/O request exceeds declared read reservation",
        )?;
        require(
            kv.write_ahead_log.payload_max_bytes < self.budgets.mutable_bytes.get()
                && kv.memtable.max_versions < self.budgets.mutable_versions.get(),
            "soft flush targets must be below declared hard ceilings",
        )?;
        require(
            !self.bindings.is_empty()
                && self.bindings.len() <= MAX_CASES
                && !self.required_cases.is_empty()
                && self.required_cases.len() <= MAX_CASES,
            "required bindings/cases missing or excessive",
        )?;
        let catalogue = endpoint_catalogue();
        let mut operations = BTreeSet::new();
        for binding in &self.bindings {
            require(
                operations.insert(&binding.operation),
                "duplicate operation binding",
            )?;
            require(
                catalogue
                    .endpoints
                    .iter()
                    .any(|endpoint| endpoint.operation.as_str() == binding.operation.as_str()),
                "unknown operation outside public catalogue",
            )?;
            require(
                binding.availability == CapabilityStatus::Available,
                "unsupported or experimental driver capability",
            )?;
        }
        let mut cases = BTreeSet::new();
        let mut used = BTreeSet::new();
        for case in &self.required_cases {
            require(cases.insert(&case.id), "duplicate required case")?;
            require(
                case.seed <= MAX_EXACT_INTEGER,
                "seed exceeds exact evidence integer",
            )?;
            require(
                operations.contains(&case.operation),
                "case lacks admitted operation binding",
            )?;
            used.insert(&case.operation);
        }
        require(used == operations, "unused operation binding")?;
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CaseReport {
    pub id: Id,
    pub status: CaseStatus,
    pub ledger: LedgerReport,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ValidationReport {
    #[schemars(range(min = 1, max = 1))]
    pub format_version: u16,
    pub manifest_sha256: Digest,
    pub result_sha256: Digest,
    pub evidence_class: EvidenceClass,
    pub durability: DurabilityClass,
    /// Structural + coverage + independent oracle success, NOT gate closure.
    pub contract_passed: bool,
    pub required: u64,
    pub passed: u64,
    pub failed: u64,
    pub skipped: u64,
    pub unimplemented: u64,
    pub attempts: u64,
    pub cases: Vec<CaseReport>,
    pub effective_settings: Vec<EffectiveSetting>,
}

pub fn validate_result(
    manifest: &StorageManifest,
    result: &StorageResult,
) -> Result<ValidationReport> {
    manifest.validate()?;
    require(
        result.format_version == 1,
        "unsupported storage result version",
    )?;
    require(
        result.manifest_sha256 == manifest_digest(manifest)?,
        "result manifest identity mismatch",
    )?;
    require(
        result.cases.len() == manifest.required_cases.len(),
        "missing or extra required case results",
    )?;
    let required: BTreeMap<_, _> = manifest
        .required_cases
        .iter()
        .map(|case| (&case.id, case))
        .collect();
    let mut seen = BTreeSet::new();
    let mut report = ValidationReport {
        format_version: 1,
        manifest_sha256: result.manifest_sha256.clone(),
        result_sha256: digest("rrflow-eval-storage-result-v1", result)?,
        evidence_class: manifest.evidence_class,
        durability: manifest.durability,
        contract_passed: false,
        required: required.len() as u64,
        passed: 0,
        failed: 0,
        skipped: 0,
        unimplemented: 0,
        attempts: 0,
        cases: Vec::new(),
        effective_settings: manifest.effective_settings()?,
    };
    for case in &result.cases {
        require(seen.insert(&case.id), "duplicate result case")?;
        let expected = required.get(&case.id).ok_or("unknown result case")?;
        require(
            case.seed == expected.seed,
            "result seed differs from required case",
        )?;
        require(
            expected_digest(&case.ledger.expected)? == expected.expected_ledger_sha256,
            "independent expected ledger changed after manifest freeze",
        )?;
        require(case.faults.len() <= MAX_CASES, "too many fault records")?;
        let ledger = case.ledger.reconcile(
            manifest.budgets.attempts.get(),
            manifest.budgets.run_ms.get() * 1_000_000,
        )?;
        report.attempts = report
            .attempts
            .checked_add(ledger.attempts)
            .ok_or("attempt sum overflow")?;
        require(
            report.attempts <= manifest.budgets.attempts.get(),
            "aggregate attempts exceed run bound",
        )?;
        match case.status {
            CaseStatus::Executed => {
                require(
                    case.limitation.is_none() && !case.ledger.expected.is_empty(),
                    "executed case must have a nonempty oracle and no unavailable limitation",
                )?;
                if ledger.violations.is_empty() && ledger.unresolved == 0 {
                    report.passed += 1;
                } else {
                    report.failed += 1;
                }
            }
            CaseStatus::Failed => {
                require(
                    !case.faults.is_empty(),
                    "failed case must retain failure provenance",
                )?;
                report.failed += 1;
            }
            CaseStatus::Skipped | CaseStatus::Unimplemented => {
                require(
                    case.limitation.is_some()
                        && case.ledger.attempts.is_empty()
                        && case.ledger.readback.is_empty(),
                    "unexecuted case needs limitation and cannot contain execution",
                )?;
                if case.status == CaseStatus::Skipped {
                    report.skipped += 1;
                } else {
                    report.unimplemented += 1;
                }
            }
        }
        report.cases.push(CaseReport {
            id: case.id.clone(),
            status: case.status,
            ledger,
        });
    }
    report.cases.sort_by(|a, b| a.id.cmp(&b.id));
    report.contract_passed = report.passed == report.required;
    Ok(report)
}
