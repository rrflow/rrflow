use super::*;
use rrd_contract::{InspectStorageCursor, StorageCursorAction};

#[test]
fn physical_cursor_is_session_scoped_retry_safe_and_released_on_close() {
    let (_root, engine) = isolated_engine();
    let session = engine
        .create_session(
            &session_request(9000, 8),
            &id("cursor-session"),
            1000,
            "create",
            "create",
        )
        .unwrap();
    let other = engine
        .create_session(
            &session_request(9000, 8),
            &id("other-session"),
            1000,
            "other",
            "other",
        )
        .unwrap();
    let request = |action| InspectStorageCursor {
        scope: "instance:test-instance".into(),
        cursor_id: id("retained"),
        action,
    };
    let call = |action| {
        engine.inspect_storage_cursor(
            &session.session_id,
            &session.token,
            &request(action),
            1100,
            "cursor",
            "cursor",
        )
    };
    let open = StorageCursorAction::Open {
        batch_rows: 1,
        batch_bytes: 4096,
        lifetime_ms: 8000,
    };
    let initial = call(open.clone()).unwrap();
    assert!(initial.rows > 0 && initial.allocated_bytes > 0);
    assert_eq!(call(open).unwrap(), initial);
    assert!(engine
        .inspect_storage_cursor(
            &other.session_id,
            &other.token,
            &request(StorageCursorAction::Observe {}),
            1100,
            "other",
            "other"
        )
        .is_err());
    assert!(call(StorageCursorAction::Advance {
        expected_ordinal: 100
    })
    .is_err());
    assert_eq!(call(StorageCursorAction::Observe {}).unwrap(), initial);
    let retained = call(StorageCursorAction::CancelSource {}).unwrap();
    assert!(!retained.source_open);
    assert_eq!(retained.page_sha256, initial.page_sha256);
    assert!(retained.allocated_bytes > 0);
    engine
        .close_session(
            &session.session_id,
            &session.token,
            &CloseSession {},
            &id("close-cursor"),
            1200,
            "close",
            "close",
        )
        .unwrap();
    let counters = engine.storage.physical_store_evidence().unwrap();
    assert_eq!(counters.active_read_views, Some(0));
    assert_eq!(counters.retained_projected_bytes, Some(0));
    assert!(call(StorageCursorAction::Observe {}).is_err());
}

#[test]
fn physical_cursor_expiry_sweep_releases_both_owners_without_wall_clock_sleep() {
    let (_root, engine) = isolated_engine();
    let session = engine
        .create_session(
            &session_request(9000, 8),
            &id("expiry-session"),
            1000,
            "create",
            "create",
        )
        .unwrap();
    let request = InspectStorageCursor {
        scope: "instance:test-instance".into(),
        cursor_id: id("expired"),
        action: StorageCursorAction::Open {
            batch_rows: 1,
            batch_bytes: 4096,
            lifetime_ms: 8000,
        },
    };
    engine
        .inspect_storage_cursor(
            &session.session_id,
            &session.token,
            &request,
            1100,
            "open",
            "open",
        )
        .unwrap();
    engine
        .storage_cursors
        .lock()
        .unwrap()
        .values_mut()
        .for_each(|entry| entry.deadline = std::time::Instant::now());
    engine.reap_storage_cursors().unwrap();
    let counters = engine.storage.physical_store_evidence().unwrap();
    assert_eq!(counters.active_read_views, Some(0));
    assert_eq!(counters.retained_projected_bytes, Some(0));
    assert!(engine.storage_cursors.lock().unwrap().is_empty());
}
