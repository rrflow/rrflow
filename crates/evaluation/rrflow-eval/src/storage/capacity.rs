//! Replaceable capacity coordinates projected from the owning J-04 table.
//! This admits a declared operating point; it never proves a measurement.
use super::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CapacityProfiles {
    pub format_version: u16,
    pub owner: String,
    pub owner_projection_sha256: Digest,
    pub record_payload_bytes: u64,
    pub profiles: Vec<CapacityProfile>,
    pub thresholds: CapacityThresholds,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CapacityProfile {
    pub id: String,
    pub name: String,
    pub vcpus: u64,
    pub host_bytes: u64,
    pub engine_bytes: u64,
    pub estates: u64,
    pub records_min: u64,
    pub edges_min: u64,
    pub vectors_min: u64,
    pub vector_dimensions: u64,
    pub logical_source_bytes_min: u64,
    pub clients: Vec<u64>,
    pub rates: Vec<u64>,
    pub requires_estate_interference: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CapacityThresholds {
    pub nominal_rates: Vec<NominalRate>,
    pub point_read: LatencyThreshold,
    pub mixed_write: LatencyThreshold,
    pub range_read: LatencyThreshold,
    pub maximum_error_basis_points: u16,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct NominalRate {
    pub profile: String,
    pub requests_per_second: u64,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LatencyThreshold {
    pub p95_ns: u64,
    pub p99_ns: u64,
}

pub fn capacity_profiles() -> Result<CapacityProfiles> {
    serde_json::from_str(include_str!(
        "../../../../../eval/capacity-profiles-v1.json"
    ))
    .map_err(|error| format!("invalid bundled J-04 projection: {error}"))
}

pub(super) fn validate_capacity_profile(campaign: &StorageCampaign) -> Result<()> {
    let projection = capacity_profiles()?;
    let allocation = &campaign.allocation;
    require(
        projection.format_version == 1
            && allocation.owner_projection_sha256 == projection.owner_projection_sha256,
        "qualification requires the current owning J-04 projection digest",
    )?;
    let profile = projection
        .profiles
        .iter()
        .find(|profile| profile.id == campaign.storage.profile.as_str())
        .ok_or("qualification requires a known J-04 capacity profile")?;
    require(
        allocation.vcpus.get() == profile.vcpus
            && allocation.host_bytes.get() == profile.host_bytes
            && allocation.engine_bytes.get() == profile.engine_bytes
            && allocation.estates.get() == profile.estates,
        "qualification allocation differs from the owning J-04 profile",
    )?;
    require(
        allocation.records.get() >= profile.records_min
            && allocation.edges.get() >= profile.edges_min
            && allocation.vectors.get() >= profile.vectors_min
            && allocation.vector_dimensions.get() == profile.vector_dimensions
            && allocation.logical_source_bytes.get() >= profile.logical_source_bytes_min,
        "qualification corpus is smaller than or incompatible with its owning J-04 profile",
    )?;
    for point in &campaign.points {
        require(
            (profile.clients.is_empty() || profile.clients.contains(&point.clients.get()))
                && (profile.rates.is_empty()
                    || profile.rates.contains(&point.requests_per_second.get())),
            "qualification operating point is outside the owning J-04 sweep",
        )?;
    }
    Ok(())
}
