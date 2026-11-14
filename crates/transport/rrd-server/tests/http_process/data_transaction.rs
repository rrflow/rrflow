//! Atomic data transactions over HTTP and the authenticated estate read.

use super::*;

#[test]
pub(super) fn data_transaction_atomically_commits_every_public_model_and_replays_after_restart() {
    let (_temporary, root, server) = start_root();
    let create = envelope(
        json!({
            "limits": {
                "idle_timeout_ms": 60_000,
                "absolute_timeout_ms": 300_000,
                "max_open_transactions": 2
            }
        }),
        Some("data-session"),
        None,
    );
    let (status, created) = post(&server, "/v1/sessions", &create, None);
    assert_eq!(status, 200, "{created}");
    let session_id = payload(&created)["session_id"].as_str().unwrap().to_owned();
    let token = payload(&created)["token"].as_str().unwrap().to_owned();
    let ensure_collection = envelope(
        json!({
            "scope": "instance:socket-test",
            "collection_id": "documents",
            "vectors": [{
                "name": "title",
                "field": "title_embedding",
                "kind": "dense",
                "dimensions": 2,
                "metric": "cosine",
                "memory_tier": "cached"
            }]
        }),
        Some("ensure-documents-collection"),
        None,
    );
    let (status, denied) = post(
        &server,
        "/v1/vector/collections/ensure",
        &ensure_collection,
        None,
    );
    assert_eq!(status, 401, "{denied}");
    let (status, ensured) = post(
        &server,
        "/v1/vector/collections/ensure",
        &ensure_collection,
        Some((&session_id, &token)),
    );
    assert_eq!(status, 200, "{ensured}");
    assert_eq!(
        payload(&ensured)["collection"]["collection_id"],
        "documents"
    );
    assert_eq!(payload(&ensured)["collection"]["generation"], 1);
    assert_eq!(payload(&ensured)["idempotent_replay"], false);
    // Catalogue configuration is part of the transaction read stamp. Install
    // it before opening the transaction so the commit is bound to the exact
    // vector contract it validates against.
    let begin = envelope(
        json!({"scope": "data", "timeout_ms": 60_000}),
        Some("begin-data"),
        None,
    );
    let (status, began) = post(
        &server,
        "/v1/transactions",
        &begin,
        Some((&session_id, &token)),
    );
    assert_eq!(status, 200, "{began}");
    let transaction_id = payload(&began)["transaction_id"]
        .as_str()
        .unwrap()
        .to_owned();
    let zero_digest = "0".repeat(64);
    let mutations = json!([
        {
            "mutation": "put_schema",
            "registry": {
                "revision": 1,
                "migration": "bootstrap all public data models",
                "tables": {
                    "document": {"model": "relational", "mode": "strict"},
                    "series": {"model": "relational", "mode": "strict"},
                    "linked": {"model": "graph_relation", "mode": "strict"},
                    "observed": {"model": "event", "mode": "strict"},
                    "embedding": {"model": "vector", "mode": "schemaless"},
                    "sample": {"model": "time_series", "mode": "schemaless"},
                    "location": {"model": "geo", "mode": "schemaless"},
                    "object": {"model": "object", "mode": "schemaless"}
                },
                "records": {
                    "document": {
                        "properties": {"title": {"value_type": "string", "required": true}},
                        "allow_additional_properties": false,
                        "unique_properties": []
                    },
                    "series": {
                        "properties": {},
                        "allow_additional_properties": true,
                        "unique_properties": []
                    }
                },
                "relations": {
                    "linked": {
                        "from": ["document"],
                        "to": ["document"],
                        "properties": {},
                        "allow_additional_properties": true,
                        "unique_pair": true
                    }
                },
                "events": {
                    "observed": {
                        "subject_required": true,
                        "subject_types": ["document"],
                        "properties": {},
                        "allow_additional_properties": true
                    }
                }
            }
        },
        {
            "mutation": "assert_claim",
            "subject": "alpha",
            "predicate": "status",
            "object": "indexed",
            "valid_from": 100,
            "tx_time": 100,
            "producer": "socket-test",
            "confidence": 1.0
        },
        {
            "mutation": "put_record",
            "reference": {"kind": "document", "id": "alpha"},
            "valid_from": 100,
            "properties": {"title": {"type": "string", "value": "Alpha"}}
        },
        {
            "mutation": "put_record",
            "reference": {"kind": "document", "id": "beta"},
            "valid_from": 100,
            "properties": {"title": {"type": "string", "value": "Beta"}}
        },
        {
            "mutation": "put_record",
            "reference": {"kind": "series", "id": "latency"},
            "valid_from": 100,
            "properties": {}
        },
        {
            "mutation": "put_relation",
            "reference": {"kind": "linked", "id": "alpha-beta"},
            "from": {"kind": "document", "id": "alpha"},
            "to": {"kind": "document", "id": "beta"},
            "valid_from": 100,
            "properties": {"reason": {"type": "string", "value": "test"}}
        },
        {
            "mutation": "append_event",
            "kind": "observed",
            "subject": {"kind": "document", "id": "alpha"},
            "properties": {"source": {"type": "string", "value": "socket"}}
        },
        {
            "mutation": "put_vector",
            "reference": {"kind": "embedding", "id": "alpha-title"},
            "subject": {"kind": "document", "id": "alpha"},
            "collection_id": "documents",
            "vector_name": "title",
            "field": "title_embedding",
            "valid_from": 100,
            "value": {"kind": "dense", "values": [0.6, 0.8]},
            "properties": {"tenant": {"type": "string", "value": "alpha"}}
        },
        {
            "mutation": "append_series_sample",
            "reference": {"kind": "sample", "id": "latency-100"},
            "series": {"kind": "series", "id": "latency"},
            "observed_at": 100,
            "value": {"type": "unsigned", "value": 17},
            "properties": {}
        },
        {
            "mutation": "put_geo",
            "reference": {"kind": "location", "id": "alpha-office"},
            "subject": {"kind": "document", "id": "alpha"},
            "field": "office",
            "valid_from": 100,
            "value": {"kind": "point", "point": {"longitude": -73.0, "latitude": 40.0}},
            "properties": {}
        },
        {
            "mutation": "publish_object_reference",
            "reference": {"kind": "object", "id": "alpha-body"},
            "subject": {"kind": "document", "id": "alpha"},
            "sha256": zero_digest,
            "length": 0,
            "media_type": "text/plain",
            "receipt": {
                "backend": "fixture-verified",
                "key": format!("objects/sha256/00/{}", "0".repeat(64))
            },
            "properties": {}
        }
    ]);
    let typed: Vec<TransactionMutation> = serde_json::from_value(mutations.clone()).unwrap();
    let preview = envelope(
        json!({
            "mutations": mutations.clone(),
            "valid_at": u64::MAX,
            "max_storage_keys": 100
        }),
        Some("prepare-data"),
        None,
    );
    let preview_path = format!("/v1/transactions/{transaction_id}/preview");
    let (status, previewed) = post(
        &server,
        &preview_path,
        &preview,
        Some((&session_id, &token)),
    );
    assert_eq!(status, 200, "{previewed}");
    assert_eq!(payload(&previewed)["read_cursor"], 0);
    assert_eq!(payload(&previewed)["idempotent_replay"], false);
    assert_eq!(payload(&previewed)["prospective"]["known_at_cursor"], 11);
    assert_eq!(payload(&previewed)["prospective"]["schema_revision"], 1);
    assert_eq!(
        payload(&previewed)["prospective"]["entries"]
            .as_array()
            .unwrap()
            .len(),
        9
    );
    let prospective = payload(&previewed)["prospective"].clone();
    let (status, ready) = http(server.address, "GET", "/v1/health/ready", &[], &[]);
    assert_eq!(status, 200, "{ready}");
    assert_eq!(payload(&ready)["runtime_cursor"], 0);

    let mut substituted_preview = preview.clone();
    substituted_preview["payload"]["mutations"][2]["properties"]["title"]["value"] =
        json!("Substituted");
    let (status, conflict) = post(
        &server,
        &preview_path,
        &substituted_preview,
        Some((&session_id, &token)),
    );
    assert_eq!(status, 409, "{conflict}");
    server.stop();

    let server = start(&root);
    let (status, replayed_preview) = post(
        &server,
        &preview_path,
        &preview,
        Some((&session_id, &token)),
    );
    assert_eq!(status, 200, "{replayed_preview}");
    assert_eq!(payload(&replayed_preview)["idempotent_replay"], true);
    assert_eq!(payload(&replayed_preview)["prospective"], prospective);
    let (status, ready) = http(server.address, "GET", "/v1/health/ready", &[], &[]);
    assert_eq!(status, 200, "{ready}");
    assert_eq!(payload(&ready)["runtime_cursor"], 0);

    let operation_sha256 = transaction_operation_sha256(&typed);
    let commit = envelope(
        json!({
            "operation_sha256": operation_sha256,
            "mutations": mutations
        }),
        Some("commit-data"),
        Some(u64::MAX),
    );
    let path = format!("/v1/transactions/{transaction_id}/commit");
    let (status, committed) = post(&server, &path, &commit, Some((&session_id, &token)));
    assert_eq!(status, 200, "{committed}");
    let receipt = payload(&committed);
    assert_eq!(receipt["mutation_count"], 11);
    assert_eq!(receipt["first_runtime_cursor"], 1);
    assert_eq!(receipt["last_runtime_cursor"], 11);
    assert_eq!(receipt["claim_mutation_count"], 1);
    assert_eq!(receipt["first_claim_sequence"], 1);
    assert_eq!(receipt["last_claim_sequence"], 1);
    assert_eq!(receipt["idempotent_replay"], false);
    assert_eq!(receipt["runtime_commit_sha256"].as_str().unwrap().len(), 64);
    let (status, replayed) = post(
        &server,
        "/v1/vector/collections/ensure",
        &ensure_collection,
        Some((&session_id, &token)),
    );
    assert_eq!(status, 200, "{replayed}");
    assert_eq!(payload(&replayed)["idempotent_replay"], true);
    let collision = envelope(
        json!({
            "scope": "instance:socket-test",
            "collection_id": "documents",
            "vectors": [{
                "name": "title",
                "field": "title_embedding",
                "kind": "dense",
                "dimensions": 3,
                "metric": "cosine",
                "memory_tier": "cached"
            }]
        }),
        Some("ensure-documents-collection"),
        None,
    );
    let (status, collision) = post(
        &server,
        "/v1/vector/collections/ensure",
        &collision,
        Some((&session_id, &token)),
    );
    assert_eq!(status, 409, "{collision}");
    let list_collections = envelope(json!({"scope": "instance:socket-test"}), None, None);
    let (status, listed) = post(
        &server,
        "/v1/vector/collections/list",
        &list_collections,
        Some((&session_id, &token)),
    );
    assert_eq!(status, 200, "{listed}");
    assert_eq!(payload(&listed)["revision"], 1);
    assert_eq!(
        payload(&listed)["collections"][0]["collection_id"],
        "documents"
    );
    let vector_search = envelope(
        json!({
            "scope": "instance:socket-test",
            "valid_at": 100,
            "collection_id": "documents",
            "vector_name": "title",
            "query": {"kind": "dense", "values": [0.6, 0.8]},
            "filter": {
                "kind": "condition",
                "condition": {
                    "property": "tenant",
                    "operator": {"operator": "equals", "value": {"type": "string", "value": "alpha"}}
                }
            },
            "top_k": 1,
            "max_storage_keys": 100
        }),
        None,
        None,
    );
    let (status, denied) = post(&server, "/v1/vector/search", &vector_search, None);
    assert_eq!(status, 401, "{denied}");
    let (status, searched) = post(
        &server,
        "/v1/vector/search",
        &vector_search,
        Some((&session_id, &token)),
    );
    assert_eq!(status, 200, "{searched}");
    let search = payload(&searched);
    assert_eq!(search["collection_id"], "documents");
    assert_eq!(search["vector_name"], "title");
    assert_eq!(search["known_at_cursor"], 11);
    assert_eq!(search["access_path"], "exact_scan");
    assert_eq!(search["exact"], true);
    assert_eq!(search["hits"][0]["reference"]["id"], "alpha-title");
    assert_eq!(search["hits"][0]["subject"]["id"], "alpha");
    assert_eq!(search["hits"][0]["source_cursor"], 8);
    assert!((search["hits"][0]["score"].as_f64().unwrap() - 1.0).abs() < 1e-9);
    assert!(search["selected_versions"].as_u64().unwrap() > 0);
    assert_direct_read_evidence(&search["read_evidence"], ReadAccessPath::VectorVersions);
    let filtered_out = envelope(
        json!({
            "scope": "instance:socket-test",
            "valid_at": 100,
            "collection_id": "documents",
            "vector_name": "title",
            "query": {"kind": "dense", "values": [0.6, 0.8]},
            "filter": {
                "kind": "condition",
                "condition": {
                    "property": "tenant",
                    "operator": {"operator": "equals", "value": {"type": "string", "value": "other"}}
                }
            },
            "top_k": 1,
            "max_storage_keys": 100
        }),
        None,
        None,
    );
    let (status, filtered) = post(
        &server,
        "/v1/vector/search",
        &filtered_out,
        Some((&session_id, &token)),
    );
    assert_eq!(status, 200, "{filtered}");
    assert!(payload(&filtered)["hits"].as_array().unwrap().is_empty());
    let scroll_points = envelope(
        json!({
            "scope": "instance:socket-test",
            "collection_id": "documents",
            "vector_name": "title",
            "valid_at": 100,
            "limit": 1,
            "max_storage_keys": 100,
            "filter": {
                "kind": "condition",
                "condition": {
                    "property": "tenant",
                    "operator": {"operator": "equals", "value": {"type": "string", "value": "alpha"}}
                }
            }
        }),
        None,
        None,
    );
    let (status, denied) = post(&server, "/v1/vector/points/scroll", &scroll_points, None);
    assert_eq!(status, 401, "{denied}");
    let (status, scrolled) = post(
        &server,
        "/v1/vector/points/scroll",
        &scroll_points,
        Some((&session_id, &token)),
    );
    assert_eq!(status, 200, "{scrolled}");
    let point_page = payload(&scrolled);
    assert_eq!(point_page["known_at_cursor"], 11);
    assert_eq!(point_page["points"][0]["reference"]["id"], "alpha-title");
    assert_eq!(
        point_page["points"][0]["payload"]["tenant"]["value"],
        "alpha"
    );
    assert_eq!(point_page["truncated"], false);
    assert!(point_page.get("next_after").is_none());
    assert!(point_page["selected_versions"].as_u64().unwrap() > 0);
    assert_direct_read_evidence(&point_page["read_evidence"], ReadAccessPath::VectorVersions);
    let retrieve_points = envelope(
        json!({
            "scope": "instance:socket-test",
            "collection_id": "documents",
            "vector_name": "title",
            "valid_at": 100,
            "references": [
                {"kind": "embedding", "id": "alpha-title"},
                {"kind": "embedding", "id": "missing-title"}
            ],
            "max_storage_keys": 100
        }),
        None,
        None,
    );
    let (status, denied) = post(
        &server,
        "/v1/vector/points/retrieve",
        &retrieve_points,
        None,
    );
    assert_eq!(status, 401, "{denied}");
    let (status, retrieved) = post(
        &server,
        "/v1/vector/points/retrieve",
        &retrieve_points,
        Some((&session_id, &token)),
    );
    assert_eq!(status, 200, "{retrieved}");
    let point_batch = payload(&retrieved);
    assert_eq!(point_batch["points"][0]["reference"]["id"], "alpha-title");
    assert_eq!(point_batch["missing"][0]["id"], "missing-title");
    assert!(point_batch["selected_versions"].as_u64().unwrap() > 0);
    assert_direct_read_evidence(
        &point_batch["read_evidence"],
        ReadAccessPath::VectorVersions,
    );
    let wrong_dimensions = envelope(
        json!({
            "scope": "instance:socket-test",
            "valid_at": 100,
            "collection_id": "documents",
            "vector_name": "title",
            "query": {"kind": "dense", "values": [0.6, 0.8, 0.0]},
            "top_k": 1,
            "max_storage_keys": 100
        }),
        None,
        None,
    );
    let (status, mismatch) = post(
        &server,
        "/v1/vector/search",
        &wrong_dimensions,
        Some((&session_id, &token)),
    );
    assert_eq!(status, 400, "{mismatch}");
    for (query, identity, expected) in [
        (
            "FROM series:series AT VALID 100 KNOWN HEAD WHERE series_id = \"latency\" PROJECT observed_at, value",
            "series:sample:latency-100",
            ("value", "unsigned", json!(17)),
        ),
        (
            "FROM series:series AT VALID 100 KNOWN HEAD WHERE observed_at >= 100 AND series_id != \"other\" PROJECT observed_at",
            "series:sample:latency-100",
            ("observed_at", "unsigned", json!(100)),
        ),
        (
            "FROM geo:location AT VALID 100 KNOWN HEAD WHERE subject_id = \"alpha\" PROJECT geometry_kind, longitude, latitude",
            "geo:location:alpha-office",
            ("longitude", "decimal", json!("-73")),
        ),
        (
            "FROM traverse:linked START document:alpha DIRECTION OUTGOING DEPTH 3 AT VALID 100 KNOWN HEAD PROJECT node_id, depth, path",
            "traversal:linked:document:alpha:1:alpha-beta:document:beta",
            ("node_id", "string", json!("beta")),
        ),
    ] {
        let request = envelope(
            json!({
                "scope": "instance:socket-test",
                "query": query,
                "parameters": {},
                "budget": {
                    "max_storage_keys": 100,
                    "max_rows": 10,
                    "max_output_bytes": 4096,
                    "max_batch_rows": 10
                }
            }),
            None,
            None,
        );
        let (status, queried) = post(
            &server,
            "/v1/query",
            &request,
            Some((&session_id, &token)),
        );
        assert_eq!(status, 200, "{queried}");
        let result = payload(&queried);
        assert_eq!(result["known_at_cursor"], 11);
        assert_eq!(result["plan"]["exact"], true);
        assert_eq!(result["rows"][0]["identity"], identity);
        assert_eq!(result["rows"][0]["values"][expected.0]["type"], expected.1);
        assert_eq!(
            result["rows"][0]["values"][expected.0]["value"],
            expected.2
        );
    }
    let select_relationship = envelope(
        json!({
            "scope": "instance:socket-test",
            "query": "SELECT left.id, right.to_id FROM record:document JOIN relation:linked ON id = from_id AT VALID 100 KNOWN HEAD WHERE left.id = \"alpha\"",
            "parameters": {},
            "budget": {
                "max_storage_keys": 100,
                "max_rows": 10,
                "max_output_bytes": 4096,
                "max_batch_rows": 10
            }
        }),
        None,
        None,
    );
    let (status, denied) = post(&server, "/v1/query", &select_relationship, None);
    assert_eq!(status, 401, "{denied}");
    let (status, selected) = post(
        &server,
        "/v1/query",
        &select_relationship,
        Some((&session_id, &token)),
    );
    assert_eq!(status, 200, "{selected}");
    let selected = payload(&selected);
    assert_eq!(selected["known_at_cursor"], 11);
    assert_eq!(selected["rows"].as_array().unwrap().len(), 1);
    assert_eq!(selected["rows"][0]["values"]["left.id"]["value"], "alpha");
    assert_eq!(
        selected["rows"][0]["values"]["right.to_id"]["value"],
        "beta"
    );
    let selected_rows = selected["rows"].clone();
    let live_query = envelope(
        json!({
            "scope": "instance:socket-test",
            "query": "FROM series:series AT VALID 100 KNOWN HEAD WHERE series_id = \"latency\" PROJECT observed_at, value",
            "parameters": {},
            "after_cursor": 0,
            "budget": {
                "max_storage_keys": 100,
                "max_rows": 10,
                "max_output_bytes": 4096,
                "max_batch_rows": 10
            },
            "max_delta_rows": 10
        }),
        None,
        None,
    );
    let (status, denied) = post(&server, "/v1/query/live/poll", &live_query, None);
    assert_eq!(status, 401, "{denied}");
    let (status, live) = post(
        &server,
        "/v1/query/live/poll",
        &live_query,
        Some((&session_id, &token)),
    );
    assert_eq!(status, 200, "{live}");
    let live = payload(&live);
    assert_eq!(live["from_cursor"], 0);
    assert_eq!(live["through_cursor"], 11);
    assert_eq!(live["added"][0]["identity"], "series:sample:latency-100");
    assert!(live["updated"].as_array().unwrap().is_empty());
    assert!(live["removed"].as_array().unwrap().is_empty());
    let ensure_index = envelope(
        json!({
            "scope": "instance:socket-test",
            "index_id": "document-title",
            "definition_query": "FROM record:document AT VALID 100 KNOWN HEAD PROJECT title",
            "unique": false,
            "budget": {
                "max_storage_keys": 100,
                "max_rows": 10,
                "max_output_bytes": 4096,
                "max_batch_rows": 10
            }
        }),
        Some("ensure-document-title"),
        None,
    );
    let (status, denied) = post(&server, "/v1/query/indexes/ensure", &ensure_index, None);
    assert_eq!(status, 401, "{denied}");
    let (status, ensured) = post(
        &server,
        "/v1/query/indexes/ensure",
        &ensure_index,
        Some((&session_id, &token)),
    );
    assert_eq!(status, 200, "{ensured}");
    assert_eq!(payload(&ensured)["idempotent_replay"], false);
    assert_eq!(payload(&ensured)["index"]["state"], "ready");
    assert_eq!(payload(&ensured)["index"]["source_cursor"], 4);
    assert_eq!(payload(&ensured)["index"]["artifact_rows"], 2);
    let (status, replayed) = post(
        &server,
        "/v1/query/indexes/ensure",
        &ensure_index,
        Some((&session_id, &token)),
    );
    assert_eq!(status, 200, "{replayed}");
    assert_eq!(payload(&replayed)["idempotent_replay"], true);
    let independently_ensured = envelope(
        ensure_index["payload"].clone(),
        Some("ensure-document-title-second"),
        None,
    );
    let (status, independently_ensured) = post(
        &server,
        "/v1/query/indexes/ensure",
        &independently_ensured,
        Some((&session_id, &token)),
    );
    assert_eq!(status, 200, "{independently_ensured}");
    assert_eq!(payload(&independently_ensured)["idempotent_replay"], false);
    assert_eq!(payload(&independently_ensured)["index"]["generation"], 1);
    assert_eq!(payload(&independently_ensured)["index"]["source_cursor"], 4);
    let mut collision = ensure_index.clone();
    collision["payload"]["definition_query"] =
        json!("FROM record:document AT VALID 101 KNOWN HEAD PROJECT title");
    let (status, collision) = post(
        &server,
        "/v1/query/indexes/ensure",
        &collision,
        Some((&session_id, &token)),
    );
    assert_eq!(status, 409, "{collision}");
    let list_indexes = envelope(json!({"scope": "instance:socket-test"}), None, None);
    let (status, listed) = post(
        &server,
        "/v1/query/indexes/list",
        &list_indexes,
        Some((&session_id, &token)),
    );
    assert_eq!(status, 200, "{listed}");
    assert_eq!(payload(&listed)["indexes"].as_array().unwrap().len(), 1);
    assert_eq!(payload(&listed)["indexes"][0]["index_id"], "document-title");
    let indexed_query = envelope(
        json!({
            "scope": "instance:socket-test",
            "query": "FROM record:document AT VALID 100 KNOWN HEAD WHERE title = \"Alpha\" PROJECT title EXPLAIN CONTRACT",
            "parameters": {},
            "budget": {
                "max_storage_keys": 100,
                "max_rows": 10,
                "max_output_bytes": 4096,
                "max_batch_rows": 10
            }
        }),
        None,
        None,
    );
    let (status, indexed) = post(
        &server,
        "/v1/query",
        &indexed_query,
        Some((&session_id, &token)),
    );
    assert_eq!(status, 200, "{indexed}");
    assert!(payload(&indexed)["plan"]["candidates"]
        .as_array()
        .unwrap()
        .iter()
        // The index's commit-maintained entries serve an equality on its
        // field at the head; proving an artifact fresh would read the family.
        .any(
            |candidate| candidate["name"] == "commit_index:document-title"
                && candidate["selected"] == true
        ));
    assert_eq!(
        payload(&indexed)["rows"][0]["identity"],
        "record:document:alpha"
    );
    let first_feed = envelope(
        json!({
            "scope": "instance:socket-test",
            "after_cursor": 0,
            "limit": 3
        }),
        None,
        None,
    );
    let (status, denied) = post(&server, "/v1/changes/read", &first_feed, None);
    assert_eq!(status, 401, "{denied}");
    let (status, first_page) = post(
        &server,
        "/v1/changes/read",
        &first_feed,
        Some((&session_id, &token)),
    );
    assert_eq!(status, 200, "{first_page}");
    let first_page = payload(&first_page);
    assert_eq!(first_page["requested_after_cursor"], 0);
    assert_eq!(first_page["through_cursor"], 3);
    assert_eq!(first_page["head_cursor"], 11);
    assert_eq!(first_page["has_more"], true);
    assert_eq!(first_page["changes"].as_array().unwrap().len(), 3);
    assert_eq!(first_page["changes"][0]["mutation"]["family"], "data");
    assert_eq!(
        first_page["changes"][0]["mutation"]["mutation"]["mutation"],
        "put_schema"
    );
    assert_eq!(first_page["changes"][1]["mutation"]["family"], "claim");
    assert_eq!(
        first_page["changes"][1]["mutation"]["claim"]["producer"],
        "socket-test"
    );
    assert_eq!(
        first_page["changes"][1]["mutation"]["claim"]["tier"],
        "local"
    );
    let first_page_tail = first_page["changes"][2]["change_sha256"]
        .as_str()
        .unwrap()
        .to_owned();
    let second_feed = envelope(
        json!({
            "scope": "instance:socket-test",
            "after_cursor": 3,
            "limit": 8
        }),
        None,
        None,
    );
    let (status, second_page) = post(
        &server,
        "/v1/changes/read",
        &second_feed,
        Some((&session_id, &token)),
    );
    assert_eq!(status, 200, "{second_page}");
    let second_page = payload(&second_page);
    assert_eq!(second_page["through_cursor"], 11);
    assert_eq!(second_page["has_more"], false);
    assert_eq!(second_page["changes"].as_array().unwrap().len(), 8);
    assert_eq!(
        second_page["changes"][0]["previous_change_sha256"],
        first_page_tail
    );
    assert_eq!(
        second_page["changes"][7]["mutation"]["mutation"]["mutation"],
        "publish_object_reference"
    );
    server.stop();

    let server = start(&root);
    let (status, selected_after_restart) = post(
        &server,
        "/v1/query",
        &select_relationship,
        Some((&session_id, &token)),
    );
    assert_eq!(status, 200, "{selected_after_restart}");
    assert_eq!(payload(&selected_after_restart)["known_at_cursor"], 11);
    assert_eq!(payload(&selected_after_restart)["rows"], selected_rows);
    let (status, replayed) = post(&server, &path, &commit, Some((&session_id, &token)));
    assert_eq!(status, 200, "{replayed}");
    assert_eq!(payload(&replayed)["idempotent_replay"], true);
    assert_eq!(
        payload(&replayed)["runtime_commit_sha256"],
        receipt["runtime_commit_sha256"]
    );
    let resumed_feed = envelope(
        json!({
            "scope": "instance:socket-test",
            "after_cursor": 10,
            "limit": 1
        }),
        None,
        None,
    );
    let (status, resumed) = post(
        &server,
        "/v1/changes/read",
        &resumed_feed,
        Some((&session_id, &token)),
    );
    assert_eq!(status, 200, "{resumed}");
    assert_eq!(payload(&resumed)["through_cursor"], 11);
    assert_eq!(payload(&resumed)["changes"][0]["cursor"], 11);
    assert_eq!(
        payload(&resumed)["changes"][0]["mutation"]["mutation"]["mutation"],
        "publish_object_reference"
    );
    server.stop();

    let engine = RrflowKvStore::open(&root).unwrap();
    let page = engine
        .runtime()
        .changes_since(0, 32, Some(&ScopeId::new("instance:socket-test").unwrap()))
        .unwrap();
    assert_eq!(page.changes.len(), 11);
    assert_eq!(engine.runtime().cursor().unwrap(), 11);
    assert_eq!(engine.claims().sequence().unwrap(), 1);
}

#[test]
pub(super) fn authenticated_estate_read_returns_the_public_snapshot_only() {
    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path().join("instance");
    {
        let engine = RrflowKvStore::open(&root).unwrap();
        let repository =
            rrd_estate::EstateRepository::new(&engine, CanonicalId::new("estate-a").unwrap());
        repository
            .create(&estate_context(10, "create-estate"))
            .unwrap();
        repository
            .set_desired(&rrd_estate::SetDesired {
                context: estate_context(20, "deploy-project-a"),
                instance_id: CanonicalId::new("project-a").unwrap(),
                idempotency_key: "deploy-project-a".into(),
                target: rrd_estate::DesiredTarget {
                    phase: rrd_estate::DesiredPhase::Running,
                    deployment_ref: CanonicalId::new("local-rrd").unwrap(),
                    version: "1.0.0".into(),
                    configuration_sha256: "a".repeat(64),
                },
            })
            .unwrap();
    }
    let server = start(&root);
    let create = envelope(
        json!({
            "limits": {
                "idle_timeout_ms": 60_000,
                "absolute_timeout_ms": 300_000,
                "max_open_transactions": 2,
            }
        }),
        Some("estate-session"),
        None,
    );
    let (status, created) = post(&server, "/v1/sessions", &create, None);
    assert_eq!(status, 200);
    let session_id = payload(&created)["session_id"].as_str().unwrap();
    let token = payload(&created)["token"].as_str().unwrap();

    let mut read = envelope(json!({}), None, None);
    read["resource"]["segments"] = json!([
        {"kind": "estate", "id": "estate-a"},
        {"kind": "instance", "id": "socket-test"}
    ]);
    let (status, denied) = post(&server, "/v1/estates/estate-a/read", &read, None);
    assert_eq!(status, 401);
    assert_eq!(denied["outcome"]["error"]["code"], "unauthenticated");

    let (status, response) = post(
        &server,
        "/v1/estates/estate-a/read",
        &read,
        Some((session_id, token)),
    );
    assert_eq!(status, 200, "{response}");
    assert_eq!(payload(&response)["id"], "estate-a");
    assert_eq!(payload(&response)["revision"], 2);
    assert_eq!(payload(&response)["instances"][0]["id"], "project-a");
    assert_eq!(payload(&response)["operations"][0]["state"], "pending");
    assert!(payload(&response).get("idempotency").is_none());
    assert_eq!(payload(&response)["idempotency_binding_count"], 1);

    let (status, mismatch) = post(
        &server,
        "/v1/estates/other-estate/read",
        &read,
        Some((session_id, token)),
    );
    assert_eq!(status, 412);
    assert_eq!(mismatch["outcome"]["error"]["code"], "failed_precondition");
}
