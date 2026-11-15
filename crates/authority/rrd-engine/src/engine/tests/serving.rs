use super::*;
use crate::engine::clock::FixedClock;
use rrd_contract::InstallationTargetKind;
use serde_json::json;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};

const INSTALLED_AT: u64 = 1_800_000_000_000;

fn installed_project() -> (tempfile::TempDir, PathBuf) {
    let project = tempfile::tempdir().unwrap();
    std::fs::write(project.path().join("README.md"), "project").unwrap();
    let executable = Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml");
    let preview = RrdEngine::plan_installation(
        project.path(),
        InstallationTargetKind::ExistingProject,
        &"1".repeat(64),
        "default",
        None,
        &executable,
    )
    .unwrap();
    RrdEngine::apply_installation_at(
        project.path(),
        InstallationTargetKind::ExistingProject,
        &preview,
        &preview.installation.plan_sha256,
        INSTALLED_AT,
        &executable,
    )
    .unwrap();
    (project, executable)
}

fn open(project: &Path, executable: &Path) -> crate::engine::Result<RrdEngine> {
    RrdEngine::open_installed_with_clock(project, executable, &FixedClock::at(INSTALLED_AT + 1_000))
}

fn record_path(project: &Path) -> PathBuf {
    project.join(".rrflow/serving.json")
}

fn recorded_url(project: &Path) -> String {
    RrdEngine::read_serving_record(project)
        .unwrap()
        .expect("a served endpoint record")
        .url
}

#[test]
fn a_served_engram_records_its_endpoint_and_withdraws_it() {
    let (project, executable) = installed_project();
    assert_eq!(
        RrdEngine::read_serving_record(project.path()).unwrap(),
        None
    );
    let engine = open(project.path(), &executable).unwrap();
    let address: SocketAddr = "127.0.0.1:41234".parse().unwrap();

    let registration = engine
        .serving_publisher()
        .unwrap()
        .publish(address)
        .unwrap();

    let record = RrdEngine::read_serving_record(project.path())
        .unwrap()
        .expect("a served endpoint record");
    assert_eq!(&record, registration.record());
    assert_eq!(record.url, "http://127.0.0.1:41234");
    assert_eq!(record.address().unwrap(), address);
    assert_eq!(&record.instance_id, engine.instance_id());
    assert_eq!(record.process_id, std::process::id());
    let written: serde_json::Value =
        serde_json::from_slice(&std::fs::read(record_path(project.path())).unwrap()).unwrap();
    let mut fields: Vec<_> = written.as_object().unwrap().keys().cloned().collect();
    fields.sort();
    assert_eq!(
        fields,
        [
            "format_version",
            "instance_id",
            "process_id",
            "product_version",
            "url"
        ],
        "the record holds no credential or other field"
    );
    let locator = std::fs::read_to_string(project.path().join(".rrflow/config.toml")).unwrap();
    assert!(!locator.contains("41234"), "the locator holds no endpoint");

    registration.withdraw().unwrap();
    assert!(!record_path(project.path()).exists());
    assert_eq!(
        RrdEngine::read_serving_record(project.path()).unwrap(),
        None
    );
}

#[test]
fn a_stale_record_is_replaced_and_withdrawal_leaves_a_newer_one() {
    let (project, executable) = installed_project();
    let engine = open(project.path(), &executable).unwrap();
    let publisher = engine.serving_publisher().unwrap();

    // A killed process never withdraws. The next publication, made while the
    // writer lock is held, replaces its record.
    let stale = publisher
        .publish("127.0.0.1:41000".parse().unwrap())
        .unwrap();
    let current = publisher
        .publish("127.0.0.1:41001".parse().unwrap())
        .unwrap();
    assert_eq!(recorded_url(project.path()), "http://127.0.0.1:41001");

    // Withdrawal removes only the record this registration wrote.
    stale.withdraw().unwrap();
    assert_eq!(recorded_url(project.path()), "http://127.0.0.1:41001");
    current.withdraw().unwrap();
    assert!(!record_path(project.path()).exists());
}

#[test]
fn publication_refuses_an_object_it_cannot_remove() {
    let (project, executable) = installed_project();
    let engine = open(project.path(), &executable).unwrap();
    std::fs::create_dir(record_path(project.path())).unwrap();

    assert!(engine
        .serving_publisher()
        .unwrap()
        .publish("127.0.0.1:41002".parse().unwrap())
        .is_err());
    assert!(
        record_path(project.path()).is_dir(),
        "an object the engine did not write is not replaced"
    );
}

#[test]
fn a_record_naming_another_address_instance_or_shape_is_refused() {
    let (project, executable) = installed_project();
    let instance = open(project.path(), &executable)
        .unwrap()
        .instance_id()
        .to_string();
    let path = record_path(project.path());
    let write = |value: &serde_json::Value| {
        std::fs::write(&path, serde_json::to_vec(value).unwrap()).unwrap();
    };
    let valid = json!({
        "format_version": 1,
        "instance_id": instance,
        "url": "http://127.0.0.1:9477",
        "process_id": 7,
        "product_version": "1.0.0",
    });
    write(&valid);
    assert_eq!(recorded_url(project.path()), "http://127.0.0.1:9477");

    for (field, value) in [
        ("url", json!("http://192.0.2.1:9477")),
        ("url", json!("https://127.0.0.1:9477")),
        ("url", json!("http://localhost:9477")),
        ("url", json!("http://127.0.0.1:9477/")),
        ("instance_id", json!("another-instance")),
        ("format_version", json!(2)),
        ("product_version", json!("")),
        ("credential", json!("a secret")),
    ] {
        let mut record = valid.clone();
        record[field] = value;
        write(&record);
        let error = RrdEngine::read_serving_record(project.path()).unwrap_err();
        assert!(
            matches!(error, ServiceError::Contract(_)),
            "{field}: {error:?}"
        );
    }

    std::fs::remove_file(&path).unwrap();
    std::fs::create_dir(&path).unwrap();
    assert!(RrdEngine::read_serving_record(project.path()).is_err());
}

#[test]
fn opening_an_engram_another_engine_holds_is_refused_as_in_use() {
    let (project, executable) = installed_project();
    let engine = open(project.path(), &executable).unwrap();

    let Err(error) = open(project.path(), &executable) else {
        panic!("a second engine opened an engram another engine holds");
    };
    assert!(matches!(error, ServiceError::EngramInUse(_)), "{error:?}");
    assert_eq!(
        error.kind(),
        crate::engine::ServiceErrorKind::FailedPrecondition
    );
    assert!(error
        .to_string()
        .contains("already open in another process"));

    drop(engine);
    open(project.path(), &executable).unwrap();
}
