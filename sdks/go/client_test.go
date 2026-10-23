package rrd

import (
	"bytes"
	"context"
	"encoding/json"
	"errors"
	"io"
	"net/http"
	"strings"
	"testing"
	"time"
)

type roundTripFunc func(*http.Request) (*http.Response, error)

func (function roundTripFunc) RoundTrip(request *http.Request) (*http.Response, error) {
	return function(request)
}

func response(status int, value any) *http.Response {
	encoded, err := json.Marshal(value)
	if err != nil {
		panic(err)
	}
	return &http.Response{
		StatusCode:    status,
		Header:        http.Header{"Content-Type": []string{"application/json"}},
		Body:          io.NopCloser(bytes.NewReader(encoded)),
		ContentLength: int64(len(encoded)),
	}
}

func okEnvelope(requestID, operationID string, payload any) *http.Response {
	return response(http.StatusOK, map[string]any{
		"protocol": "rrd", "protocol_version": 1,
		"request_id": requestID, "operation_id": operationID,
		"outcome": map[string]any{"status": "ok", "payload": payload},
	})
}

func TestCapabilitiesRetryAndIdentity(t *testing.T) {
	t.Parallel()
	attempts := 0
	client, err := NewClient(Config{
		BaseURL: "http://127.0.0.1:9477", Instance: "sdk-test",
		Transport: roundTripFunc(func(request *http.Request) (*http.Response, error) {
			attempts++
			if request.URL.Path != "/v1/capabilities" || request.Method != http.MethodGet {
				t.Fatalf("unexpected request: %s %s", request.Method, request.URL.Path)
			}
			if attempts == 1 {
				return nil, errors.New("connection reset")
			}
			return okEnvelope("server-request", "server-operation", map[string]any{
				"protocol": "rrd", "protocol_version": 1,
				"implementation": "rrd-server", "implementation_version": "1.0.0",
				"deployment": map[string]any{
					"contract_version": 1, "deployment_form": "single_node_server",
					"storage_profile": "rrflow_kv", "endpoint_presentation": "loopback_http_websocket",
				},
				"configuration": map[string]any{
					"format_version": 3, "revision": 1,
					"clock": map[string]any{"maximum_rollback_ms": 0},
					"storage": map[string]any{
						"mutable_accounted_bytes":        134217728,
						"mutable_versions":               1048576,
						"projected_output_bytes":         134217728,
						"page_cache_bytes":               4194304,
						"wal_flush_target_bytes":         67108864,
						"memtable_flush_target_versions": 524288,
						"compaction_input_bytes":         1073741824,
						"compaction_input_logical_bytes": 268435456,
						"compaction_input_versions":      4194304,
						"compaction_output_segments":     256,
						"compaction_elapsed_ms":          30000,
						"compaction_l0_segments":         64,
						"retained_mutable_bytes":         536870912,
						"retained_mutable_versions":      4194304,
						"retained_mutable_generations":   64,
						"reclamation_inventory_entries":  262144,
						"reclamation_metadata_bytes":     67108864,
						"reclamation_delete_files":       1024,
						"reclamation_delete_bytes":       1073741824,
						"reclamation_elapsed_ms":         30000,
						"transaction_admission_requests": 64,
						"transaction_admission_bytes":    67108864,
						"transaction_admission_wait_ms":  30000,
						"recovery_metadata_bytes":        67108864,
						"recovery_files":                 262144,
						"recovery_read_bytes":            1099511627776,
						"recovery_wal_bytes":             268435456,
						"recovery_wal_frames":            1048576,
						"recovery_elapsed_ms":            120000,
					},
					"reasoning": map[string]any{
						"max_run_elapsed_ms": 900000, "max_steps": 256, "max_step_elapsed_ms": 60000,
					},
					"recall": map[string]any{
						"max_graph_depth": 4, "max_items": 128, "max_output_bytes": 524288, "max_storage_keys": 100000,
					},
					"query": map[string]any{
						"max_storage_keys": 100000, "max_rows": 10000, "max_output_bytes": 524288,
						"max_batch_rows": 256, "max_memory_bytes": 67108864,
						"max_spill_bytes": 268435456, "max_elapsed_ms": 30000,
					},
					"configuration_sha256": "408211d1644753c91e002a8b41af1b5aa14ceff60ce6609addf3d3fc83ded7e0",
				},
				"instance":     map[string]any{"kind": "instance", "id": "sdk-test"},
				"capabilities": []any{},
			}), nil
		}),
	})
	if err != nil {
		t.Fatal(err)
	}
	capabilities, err := client.Capabilities(context.Background())
	if err != nil {
		t.Fatal(err)
	}
	if capabilities["protocol_version"] != float64(1) || attempts != 2 {
		t.Fatalf("unexpected capabilities or attempts: %#v %d", capabilities, attempts)
	}
	deployment := capabilities["deployment"].(map[string]any)
	if deployment["deployment_form"] != "single_node_server" {
		t.Fatalf("unexpected deployment profile: %#v", deployment)
	}
}

func TestSessionAndQueryAuthenticationAndEnvelopes(t *testing.T) {
	t.Parallel()
	type captured struct {
		Authorization string
		Envelope      map[string]any
	}
	requests := make([]captured, 0, 2)
	client, err := NewClient(Config{
		BaseURL: "http://localhost:9477", Instance: "sdk-test",
		Transport: roundTripFunc(func(request *http.Request) (*http.Response, error) {
			var body map[string]any
			if decodeErr := json.NewDecoder(request.Body).Decode(&body); decodeErr != nil {
				t.Fatal(decodeErr)
			}
			requests = append(requests, captured{request.Header.Get("Authorization"), body})
			contextBody := body["context"].(map[string]any)
			if request.URL.Path == "/v1/sessions" {
				return okEnvelope(contextBody["request_id"].(string), contextBody["operation_id"].(string), map[string]any{
					"session_id": "session-1", "token": "token-1", "issued_at_unix_ms": 100,
					"idle_expires_at_unix_ms": 60100, "absolute_expires_at_unix_ms": 300100,
					"limits": map[string]any{"idle_timeout_ms": 60000, "absolute_timeout_ms": 300000, "max_open_transactions": 2},
				}), nil
			}
			return okEnvelope(contextBody["request_id"].(string), contextBody["operation_id"].(string), map[string]any{
				"canonical_query": "FROM record:document", "scope": "instance:sdk-test",
				"read_manifest_sha256": strings.Repeat("a", 64), "known_at_cursor": 2,
				"schema_revision": 1, "plan": map[string]any{}, "execution": map[string]any{}, "rows": []any{},
			}), nil
		}),
	})
	if err != nil {
		t.Fatal(err)
	}
	session, err := client.CreateSession(context.Background(), "go-sdk", "not-persisted", map[string]any{
		"limits": map[string]any{"idle_timeout_ms": 60000, "absolute_timeout_ms": 300000, "max_open_transactions": 2},
	}, RequestOptions{RequestID: "request-session", OperationID: "operation-session", IdempotencyKey: "session-key"})
	if err != nil {
		t.Fatal(err)
	}
	result, err := client.Call(context.Background(), OperationQueryExecute, map[string]any{
		"scope": "instance:sdk-test", "query": "FROM record:document", "parameters": map[string]any{},
		"budget": map[string]any{"max_storage_keys": 100, "max_rows": 10, "max_output_bytes": 4096, "max_batch_rows": 10},
	}, RequestOptions{RequestID: "request-query", OperationID: "operation-query", Session: session})
	if err != nil {
		t.Fatal(err)
	}
	if result["known_at_cursor"] != float64(2) || requests[0].Authorization != "ApiKey not-persisted" ||
		requests[1].Authorization != "Bearer token-1" {
		t.Fatalf("unexpected query/auth result: %#v %#v", result, requests)
	}
	resource := requests[1].Envelope["resource"].(map[string]any)["segments"].([]any)
	if resource[0].(map[string]any)["id"] != "sdk-test" {
		t.Fatalf("unexpected resource: %#v", resource)
	}
}

func TestSecurityDeadlinesErrorsAndGeneratedCoverage(t *testing.T) {
	t.Parallel()
	if _, err := NewClient(Config{BaseURL: "http://192.0.2.1:9477", Instance: "sdk-test"}); err == nil {
		t.Fatal("remote cleartext URL was accepted")
	}
	client, err := NewClient(Config{
		BaseURL: "http://127.0.0.1:9477", Instance: "sdk-test", MaxAttempts: 1,
		Transport: roundTripFunc(func(request *http.Request) (*http.Response, error) {
			var body map[string]any
			if decodeErr := json.NewDecoder(request.Body).Decode(&body); decodeErr != nil {
				t.Fatal(decodeErr)
			}
			requestContext := body["context"].(map[string]any)
			return response(http.StatusForbidden, map[string]any{
				"protocol": "rrd", "protocol_version": 1,
				"request_id": requestContext["request_id"], "operation_id": requestContext["operation_id"],
				"outcome": map[string]any{"status": "error", "error": map[string]any{
					"code": "permission_denied", "message": "policy denied", "retryable": false,
				}},
			}), nil
		}),
	})
	if err != nil {
		t.Fatal(err)
	}
	session := &Session{PrincipalID: "go-sdk", Lease: SessionLease{SessionID: "session-1", Token: "token-1"}}
	_, err = client.Call(context.Background(), OperationAuditRead, map[string]any{"after_sequence": 0, "limit": 10}, RequestOptions{
		RequestID: "request-audit", OperationID: "operation-audit", Session: session,
	})
	var apiError *APIError
	if !errors.As(err, &apiError) || apiError.Code != "permission_denied" || len(apiError.Details) != 0 {
		t.Fatalf("unexpected API error: %v", err)
	}
	_, err = client.Call(context.Background(), OperationAuditRead, map[string]any{"after_sequence": 0, "limit": 10}, RequestOptions{
		RequestID: "request-expired", OperationID: "operation-expired", Session: session,
		Deadline: time.UnixMilli(1),
	})
	if err == nil || !strings.Contains(err.Error(), "deadline has expired") {
		t.Fatalf("unexpected deadline result: %v", err)
	}
	if len(endpoints) != 48 || !endpoints[OperationBackupCreate].Mutation ||
		!endpoints[OperationTransactionPreview].Mutation ||
		endpoints[OperationCapabilitiesRead].Authentication != "public" {
		t.Fatalf("generated endpoint catalogue is incomplete: %#v", endpoints)
	}
}
