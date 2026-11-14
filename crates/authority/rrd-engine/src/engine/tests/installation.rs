mod restore;

use super::*;

use crate::engine::clock::FixedClock;

use crate::{
    ClockAssessmentStatus, ClockObservationFailure, ClockSourceKind, ClockTrustLevel,
    InstallationPreview, InstallationVerificationReport, InstallationVerificationStatus,
};

use rrd_contract::{installation_plan_sha256, InstallationTargetKind, SessionLease};

use std::collections::BTreeMap;

use std::path::{Path, PathBuf};

fn inventory(root: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    fn visit(root: &Path, current: &Path, output: &mut BTreeMap<PathBuf, Vec<u8>>) {
        let mut entries = std::fs::read_dir(current)
            .unwrap()
            .collect::<std::io::Result<Vec<_>>>()
            .unwrap();
        entries.sort_by_key(std::fs::DirEntry::file_name);
        for entry in entries {
            let path = entry.path();
            if path.is_dir() {
                visit(root, &path, output);
            } else {
                output.insert(
                    path.strip_prefix(root).unwrap().to_owned(),
                    std::fs::read(path).unwrap(),
                );
            }
        }
    }
    let mut output = BTreeMap::new();
    visit(root, root, &mut output);
    output
}

fn plan(project: &Path) -> InstallationPreview {
    RrdEngine::plan_installation(
        project,
        InstallationTargetKind::ExistingProject,
        &"1".repeat(64),
        "default",
        None,
        &test_executable(),
    )
    .unwrap()
}

fn plan_with_source_read(project: &Path, enabled: bool) -> InstallationPreview {
    RrdEngine::plan_installation_with_options(
        project,
        InstallationTargetKind::ExistingProject,
        &"1".repeat(64),
        "default",
        None,
        crate::InstallationPlanningOptions {
            attunement_follow: None,
            attunement_proposal: None,
            allow_project_source_read: enabled,
        },
        &test_executable(),
    )
    .unwrap()
}

fn test_executable() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml")
}

fn open_at(project: &Path, executable: &Path, at_unix_ms: u64) -> crate::Result<RrdEngine> {
    RrdEngine::open_installed_with_clock(project, executable, &FixedClock::at(at_unix_ms))
}

fn inspect_at(
    project: &Path,
    executable: &Path,
    at_unix_ms: u64,
) -> crate::Result<InstallationVerificationReport> {
    RrdEngine::inspect_installed_with_clock(project, executable, &FixedClock::at(at_unix_ms))
}

mod apply_and_open;
mod configuration_and_adoption;
mod defaults_and_identity;
