use rrd_contract::CanonicalId;
use rrd_engram::{LocalEngramPermission, LocalOperatorPolicy, LOCAL_OPERATOR_POLICY_FORMAT};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

#[test]
fn authorization_is_key_engram_action_and_time_bound() {
    let temporary = tempfile::tempdir().unwrap();
    let key_path = temporary.path().join("operator.key");
    let key = [7_u8; 32];
    std::fs::write(&key_path, key).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&key_path, std::fs::Permissions::from_mode(0o600)).unwrap();
    }
    let digest = Sha256::digest(key);
    let policy = LocalOperatorPolicy {
        format: LOCAL_OPERATOR_POLICY_FORMAT,
        operator_id: CanonicalId::new("operator-one").unwrap(),
        key_sha256: digest.iter().map(|byte| format!("{byte:02x}")).collect(),
        not_before_unix_ms: 10,
        expires_at_unix_ms: 100,
        engrams: BTreeMap::from([(
            "engram-a".into(),
            BTreeSet::from([LocalEngramPermission::Create]),
        )]),
    };
    let authorized = policy
        .authorize_key_file(
            &key_path,
            CanonicalId::new("engram-a").unwrap(),
            LocalEngramPermission::Create,
            50,
        )
        .unwrap();
    assert_eq!(authorized.operator_id.as_str(), "operator-one");
    assert!(policy
        .authorize_key_file(
            &key_path,
            CanonicalId::new("engram-a").unwrap(),
            LocalEngramPermission::SetDesired,
            50,
        )
        .is_err());
    assert!(policy
        .authorize_key_file(
            &key_path,
            CanonicalId::new("engram-a").unwrap(),
            LocalEngramPermission::ScheduleBackup,
            50,
        )
        .is_err());
    assert!(policy
        .authorize_key_file(
            &key_path,
            CanonicalId::new("engram-b").unwrap(),
            LocalEngramPermission::Create,
            50,
        )
        .is_err());
    assert!(policy
        .authorize_key_file(
            &key_path,
            CanonicalId::new("engram-a").unwrap(),
            LocalEngramPermission::Create,
            100,
        )
        .is_err());
}
