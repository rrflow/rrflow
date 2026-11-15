//! The project map: what it describes, what it keeps, and that the installed
//! layout admits it.

use super::*;

fn install_demo_project() -> (tempfile::TempDir, InstallationPreview) {
    let project = tempfile::tempdir().unwrap();
    let write = |relative: &str, bytes: &[u8]| {
        let path = project.path().join(relative);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, bytes).unwrap();
    };
    write(".gitignore", b"node_modules/\n");
    write("README.md", b"# demo\n");
    write("package.json", b"{}\n");
    write("src/app.ts", &[b'x'; 300]);
    write("src/util.ts", &[b'x'; 100]);
    write("docs/guide.md", &[b'x'; 50]);
    write("node_modules/pkg/index.js", b"ignored\n");
    let preview = plan(project.path());
    RrdEngine::apply_installation_at(
        project.path(),
        InstallationTargetKind::ExistingProject,
        &preview,
        &preview.installation.plan_sha256,
        1_800_000_000_000,
        &test_executable(),
    )
    .unwrap();
    (project, preview)
}

#[test]
pub(super) fn the_project_map_describes_the_project_and_keeps_the_owners_notes() {
    let (project, _) = install_demo_project();

    let first = RrdEngine::refresh_project_map(project.path()).unwrap();
    assert!(first.changed);
    assert_eq!(first.files, 6);
    let path = project.path().join(crate::PROJECT_MAP_RELATIVE_PATH);
    let map = std::fs::read_to_string(&path).unwrap();
    let name = project.path().file_name().unwrap().to_str().unwrap();
    assert!(map.starts_with(&format!("# Project map: {name}\n")));
    assert!(map.contains("- **Kind:** JavaScript/TypeScript (npm)"));
    assert!(map.contains("- `README.md`: the project's own introduction"));
    assert!(map.contains("| `src/` | 2 | 400 B | TypeScript |"));
    assert!(map.contains("| `docs/` | 1 | 50 B | Markdown |"));
    assert!(!map.contains("node_modules"));

    // The owner's notes survive; an unchanged project is not rewritten.
    let edited = map.replace(
        "RRFlow keeps this section exactly as you leave it.",
        "Ship the checkout flow first.",
    );
    assert_ne!(edited, map, "the default notes are in a fresh map");
    std::fs::write(&path, &edited).unwrap();
    let unchanged = RrdEngine::refresh_project_map(project.path()).unwrap();
    assert!(!unchanged.changed);
    assert_eq!(std::fs::read_to_string(&path).unwrap(), edited);

    // The map evolves with the project and still keeps the notes.
    std::fs::write(project.path().join("src/cart.ts"), [b'x'; 100]).unwrap();
    let evolved = RrdEngine::refresh_project_map(project.path()).unwrap();
    assert!(evolved.changed);
    let map = std::fs::read_to_string(&path).unwrap();
    assert!(map.contains("| `src/` | 3 | 500 B | TypeScript |"));
    assert!(map.contains("Ship the checkout flow first."));
    assert!(!project.path().join(".rrflow/.project-map.pending").exists());
}

#[test]
pub(super) fn an_installed_project_that_holds_its_map_still_reopens_replays_and_recovers() {
    use crate::engine::estate_layout::{validate_recovery_estate, EstateFilesystem, EstateLayout};

    let (project, preview) = install_demo_project();
    let executable = test_executable();
    let source_before = RrdEngine::plan_installation(
        project.path(),
        InstallationTargetKind::ExistingProject,
        &"2".repeat(64),
        "default",
        None,
        &executable,
    )
    .map(|plan| plan.installation.project_precondition_sha256);

    RrdEngine::refresh_project_map(project.path()).unwrap();
    // A refresh interrupted before its rename leaves the pending file behind.
    std::fs::write(project.path().join(".rrflow/.project-map.pending"), b"x").unwrap();

    let layout = EstateLayout::new(
        &preview.installation.storage_root_id,
        &preview.installation.initial_principal_id,
    )
    .unwrap();
    validate_recovery_estate(
        &EstateFilesystem::admit(
            rrd_store::filesystem::AnchoredPath::open_root(project.path()).unwrap(),
            false,
        )
        .unwrap(),
        &layout,
    )
    .expect("install recovery accepts a project that holds its map and its pending file");

    let replay = RrdEngine::apply_installation_at(
        project.path(),
        InstallationTargetKind::ExistingProject,
        &preview,
        &preview.installation.plan_sha256,
        1_800_000_000_001,
        &executable,
    )
    .unwrap();
    assert!(replay.idempotent_replay);
    assert_eq!(
        inspect_at(project.path(), &executable, 1_800_000_000_002)
            .unwrap()
            .status,
        InstallationVerificationStatus::Passed
    );
    open_at(project.path(), &executable, 1_800_000_000_003).unwrap();

    // The map lives below `.rrflow`, so writing it does not change what the
    // project's own source is observed to be.
    let source_after = RrdEngine::plan_installation(
        project.path(),
        InstallationTargetKind::ExistingProject,
        &"2".repeat(64),
        "default",
        None,
        &executable,
    )
    .map(|plan| plan.installation.project_precondition_sha256);
    assert_eq!(
        source_before.is_ok(),
        source_after.is_ok(),
        "planning admits the project the same way before and after the map"
    );
    if let (Ok(before), Ok(after)) = (source_before, source_after) {
        assert_eq!(before, after);
    }
}
