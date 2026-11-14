use super::*;

#[test]
fn only_an_advanced_cursor_gets_the_stale_transaction_action() {
    let stale = classify_rejected_commit(rrd_store::Error::RuntimeConflict {
        expected: 7,
        actual: 8,
    });
    assert!(matches!(
        stale,
        ServiceError::TransactionSnapshotStale {
            expected: 7,
            actual: 8
        }
    ));
    assert_eq!(stale.kind(), ServiceErrorKind::Conflict);
    assert!(!stale.retryable());
    for actual in [6, 7] {
        let other = classify_rejected_commit(rrd_store::Error::RuntimeConflict {
            expected: 7,
            actual,
        });
        assert!(matches!(other, ServiceError::StorageConflict(_)));
        assert!(other.retryable());
    }
    let unrelated =
        classify_rejected_commit(rrd_store::Error::IdempotencyConflict("fixture".into()));
    assert!(matches!(unrelated, ServiceError::StorageConflict(_)));
}
