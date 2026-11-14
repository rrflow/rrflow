use super::*;

#[test]
fn arguments_require_one_installed_project_and_distribution_executable() {
    assert!(parse_args(std::iter::empty()).is_err());
    assert!(parse_args(["--db".into(), "state".into()].into_iter()).is_err());
    assert!(parse_args(["initialize".into()].into_iter()).is_err());
    assert!(parse_args(["--root".into(), "project".into()].into_iter()).is_err());
    assert!(parse_args(["--project".into(), "project".into()].into_iter()).is_err());
    let args = parse_args(
        [
            "--project".into(),
            "project".into(),
            "--distribution-executable".into(),
            "rrflow".into(),
        ]
        .into_iter(),
    )
    .unwrap();
    assert_eq!(args.bind, "127.0.0.1:9477".parse().unwrap());
    assert_eq!(args.project, PathBuf::from("project"));
    assert_eq!(args.distribution_executable, PathBuf::from("rrflow"));
}

#[test]
fn shutdown_control_files_are_a_paired_absolute_contract() {
    let root = std::env::current_dir().unwrap();
    let request = root.join("shutdown.request");
    let complete = root.join("shutdown.complete");
    let incomplete = vec![
        "--project".into(),
        "project".into(),
        "--distribution-executable".into(),
        "rrflow".into(),
        "--shutdown-request-file".into(),
        request.to_string_lossy().into_owned(),
    ];
    assert!(parse_args(incomplete.into_iter()).is_err());

    let arguments = vec![
        "--project".into(),
        "project".into(),
        "--distribution-executable".into(),
        "rrflow".into(),
        "--shutdown-request-file".into(),
        request.to_string_lossy().into_owned(),
        "--shutdown-complete-file".into(),
        complete.to_string_lossy().into_owned(),
    ];
    let args = parse_args(arguments.into_iter()).unwrap();
    assert_eq!(args.shutdown_complete_file, Some(complete));
}

#[test]
fn readiness_file_is_absolute_and_distinct_from_shutdown_controls() {
    let root = std::env::current_dir().unwrap();
    let ready = root.join("rrd.ready");
    let arguments = vec![
        "--project".into(),
        "project".into(),
        "--distribution-executable".into(),
        "rrflow".into(),
        "--ready-file".into(),
        ready.to_string_lossy().into_owned(),
    ];
    let args = parse_args(arguments.into_iter()).unwrap();
    assert_eq!(args.ready_file, Some(ready.clone()));
    assert!(parse_args(
        [
            "--project".into(),
            "project".into(),
            "--distribution-executable".into(),
            "rrflow".into(),
            "--ready-file".into(),
            "relative.ready".into(),
        ]
        .into_iter()
    )
    .is_err());

    let shutdown = vec![
        "--project".into(),
        "project".into(),
        "--distribution-executable".into(),
        "rrflow".into(),
        "--ready-file".into(),
        ready.to_string_lossy().into_owned(),
        "--shutdown-request-file".into(),
        ready.to_string_lossy().into_owned(),
        "--shutdown-complete-file".into(),
        root.join("shutdown.complete")
            .to_string_lossy()
            .into_owned(),
    ];
    assert!(parse_args(shutdown.into_iter()).is_err());
}

#[test]
fn mutual_tls_files_are_an_all_or_nothing_contract() {
    let incomplete = [
        "--project".into(),
        "project".into(),
        "--distribution-executable".into(),
        "rrflow".into(),
        "--tls-cert".into(),
        "server.pem".into(),
    ];
    assert!(parse_args(incomplete.into_iter()).is_err());
}
