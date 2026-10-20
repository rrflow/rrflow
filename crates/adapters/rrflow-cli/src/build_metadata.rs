//! Build-input wire record shared by the build script and primary executable.
//! This is provenance supplied by the repository builder, not a signature.
use serde::{Deserialize, Serialize};

pub const MAX_BUILD_MANIFEST_BYTES: u64 = 256 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CompilerIdentity {
    pub rustc: String,
    pub target: String,
    /// Cargo's root profile; custom profile names come from declared provenance.
    pub root_profile: String,
    pub opt_level: String,
    pub panic: String,
    pub debug_assertions: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BuildMetadata {
    pub compiler: CompilerIdentity,
    pub provenance: Option<BuildProvenance>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BuildProvenance {
    pub format_version: u16,
    pub source_commit: String,
    pub source_tree: String,
    pub source_content_sha256: String,
    pub dirty: bool,
    pub cargo_lock_sha256: String,
    pub rustc: String,
    pub target: String,
    pub cargo_profile: String,
    /// Hash of build-affecting environment values; raw values can contain secrets.
    pub build_environment_sha256: String,
    pub first_party_units: Vec<CompiledUnit>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CompiledUnit {
    pub package: String,
    pub version: String,
    pub target: String,
    pub kind: Vec<String>,
    pub features: Vec<String>,
    pub opt_level: String,
    pub debug_assertions: bool,
    pub overflow_checks: bool,
}

impl BuildProvenance {
    pub fn validate(&self) -> Result<(), &'static str> {
        fn hex(value: &str, lengths: &[usize]) -> bool {
            lengths.contains(&value.len())
                && value
                    .bytes()
                    .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
        }
        fn atom(value: &str) -> bool {
            !value.is_empty()
                && value.len() <= 128
                && value
                    .bytes()
                    .all(|c| c.is_ascii_alphanumeric() || b"_-+.".contains(&c))
        }
        if self.format_version != 1
            || !hex(&self.source_commit, &[40, 64])
            || !hex(&self.source_tree, &[40, 64])
            || !hex(&self.source_content_sha256, &[64])
            || !hex(&self.cargo_lock_sha256, &[64])
            || !hex(&self.build_environment_sha256, &[64])
            || !atom(&self.target)
            || !matches!(
                self.cargo_profile.as_str(),
                "dev" | "release" | "diagnostic"
            )
            || !self.rustc.starts_with("rustc ")
            || self.rustc.len() > 256
            || self.rustc.contains(['\n', '\r'])
        {
            return Err("invalid build provenance coordinates");
        }
        if self.first_party_units.is_empty() || self.first_party_units.len() > 128 {
            return Err("invalid first-party compilation closure size");
        }
        for unit in &self.first_party_units {
            if !atom(&unit.package)
                || !atom(&unit.version)
                || !atom(&unit.target)
                || unit.kind.is_empty()
                || unit.kind.len() > 8
                || unit.kind.iter().any(|kind| !atom(kind))
                || unit.features.len() > 128
                || unit.features.iter().any(|feature| !atom(feature))
                || unit.features.windows(2).any(|pair| pair[0] >= pair[1])
                || !matches!(unit.opt_level.as_str(), "0" | "1" | "2" | "3" | "s" | "z")
            {
                return Err("invalid first-party compilation unit");
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn provenance() -> BuildProvenance {
        BuildProvenance {
            format_version: 1,
            source_commit: "a".repeat(40),
            source_tree: "b".repeat(40),
            source_content_sha256: "c".repeat(64),
            dirty: false,
            cargo_lock_sha256: "d".repeat(64),
            rustc: "rustc fixture".into(),
            target: "x86_64-pc-windows-msvc".into(),
            cargo_profile: "dev".into(),
            build_environment_sha256: "e".repeat(64),
            first_party_units: vec![CompiledUnit {
                package: "rrflow-cli".into(),
                version: "1.0.0".into(),
                target: "rrflow".into(),
                kind: vec!["bin".into()],
                features: vec![],
                opt_level: "0".into(),
                debug_assertions: true,
                overflow_checks: true,
            }],
        }
    }

    #[test]
    fn malformed_or_unbounded_provenance_is_rejected() {
        let valid = provenance();
        valid.validate().unwrap();
        let mut malformed = valid.clone();
        malformed.source_commit = "unknown".into();
        assert!(malformed.validate().is_err());
        malformed = valid.clone();
        malformed.first_party_units.clear();
        assert!(malformed.validate().is_err());
        malformed = valid.clone();
        malformed.first_party_units[0].features = vec!["full".into(), "full".into()];
        assert!(malformed.validate().is_err());
        malformed = valid.clone();
        malformed.rustc = "rustc fixture\ncredential".into();
        assert!(malformed.validate().is_err());
        malformed = valid;
        malformed.first_party_units[0].package = "../outside".into();
        assert!(malformed.validate().is_err());
    }

    #[test]
    fn unknown_manifest_fields_are_rejected() {
        let mut document = serde_json::to_value(provenance()).unwrap();
        document["trusted"] = true.into();
        assert!(serde_json::from_value::<BuildProvenance>(document).is_err());
    }
}
