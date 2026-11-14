use super::*;

#[test]
fn control_reader_bounds_and_recovers_at_the_next_frame() {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async {
            let mut bytes = vec![b'x'; RRFLOW_NODE_MAX_CONTROL_LINE_BYTES + 1];
            bytes.extend_from_slice(
                b"\n{\"version\":1,\"request_id\":\"next\",\"command\":{\"command\":\"status\"}}\n",
            );
            let mut reader = BufReader::new(bytes.as_slice());
            assert_eq!(read_control_line(&mut reader).await.unwrap(), Some(Err(())));
            let next = read_control_line(&mut reader)
                .await
                .unwrap()
                .unwrap()
                .unwrap();
            assert_eq!(
                serde_json::from_slice::<RrdNodeRequest>(&next)
                    .unwrap()
                    .command,
                RrdNodeCommand::Status,
            );
            assert_eq!(read_control_line(&mut reader).await.unwrap(), None);
        });
}

#[test]
fn control_contract_denies_unknown_fields() {
    let error = serde_json::from_str::<RrdNodeRequest>(
        r#"{"version":1,"request_id":"test","command":{"command":"status"},"unversioned_surprise":true}"#,
    )
    .unwrap_err();
    assert!(error.to_string().contains("unknown field"));
}

#[test]
fn control_contract_denies_unknown_versions_and_request_identities() {
    for request in [
        RrdNodeRequest {
            version: RRFLOW_NODE_CONTROL_VERSION + 1,
            request_id: "future".into(),
            command: RrdNodeCommand::Status,
        },
        RrdNodeRequest {
            version: RRFLOW_NODE_CONTROL_VERSION,
            request_id: String::new(),
            command: RrdNodeCommand::Status,
        },
    ] {
        assert!(request.validate().is_err());
    }
}
