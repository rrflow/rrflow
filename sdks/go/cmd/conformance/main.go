package main

import (
	"context"
	"crypto/sha256"
	"encoding/hex"
	"encoding/json"
	"errors"
	"fmt"
	"maps"
	"os"
	"time"

	rrd "github.com/rrflow/rrflow/sdks/go"
)

type manifest struct {
	FormatVersion          int               `json:"format_version"`
	CorpusSHA256           string            `json:"corpus_sha256"`
	CorpusPath             string            `json:"corpus_path"`
	BaseURL                string            `json:"base_url"`
	RetryBaseURLs          map[string]string `json:"retry_base_urls"`
	IncompatibleVersionURL string            `json:"incompatible_version_url"`
}

func main() {
	manifestPath := os.Getenv("RRD_SDK_CONFORMANCE_MANIFEST")
	require(manifestPath != "", "RRD_SDK_CONFORMANCE_MANIFEST is required")
	var harness manifest
	decodeFile(manifestPath, &harness)
	corpusBytes, err := os.ReadFile(harness.CorpusPath)
	check(err)
	digest := sha256.Sum256(corpusBytes)
	require(hex.EncodeToString(digest[:]) == harness.CorpusSHA256, "corpus digest differs")
	var corpus map[string]any
	check(json.Unmarshal(corpusBytes, &corpus))
	identity := object(corpus["identity"])
	expected := object(corpus["expected"])
	sessionFixture := object(corpus["session"])
	transaction := object(corpus["transaction"])
	vector := object(corpus["vector"])
	changefeed := object(corpus["changefeed"])
	backupFixture := object(corpus["backup"])
	instance := text(identity["instance"])

	retryClient := client(harness.RetryBaseURLs["go"], instance, integer(expected["retry_attempts"]))
	capabilities, err := retryClient.Capabilities(context.Background())
	check(err)
	require(integer(capabilities["protocol_version"]) == integer(corpus["protocol_version"]), "retry negotiation differs")
	versionClient := client(harness.IncompatibleVersionURL, instance, 2)
	_, err = versionClient.Capabilities(context.Background())
	require(err != nil, "incompatible version was accepted")

	api := client(harness.BaseURL, instance, 2)
	capabilities, err = api.Capabilities(context.Background())
	check(err)
	require(text(capabilities["protocol"]) == text(corpus["protocol"]), "protocol differs")
	catalogue, err := api.EndpointCatalogue(context.Background())
	check(err)
	require(len(array(catalogue["endpoints"])) == integer(expected["endpoint_count"]), "endpoint count differs")
	_, err = api.CreateSession(context.Background(), text(identity["principal"]), "wrong-sdk-conformance-key", object(sessionFixture["create"]), mutation("wrong-key", nil, nil, time.Time{}))
	var apiError *rrd.APIError
	require(errors.As(err, &apiError) && apiError.Code == text(expected["typed_error"]), "typed auth error differs")
	session, err := api.CreateSession(context.Background(), text(identity["principal"]), text(identity["api_key"]), object(sessionFixture["create"]), mutation("session-create", nil, nil, time.Time{}))
	check(err)
	renewed, err := api.Call(context.Background(), rrd.OperationSessionRenew, object(sessionFixture["renew"]), mutation("session-renew", session, map[string]string{"session": session.Lease.SessionID}, time.Time{}))
	check(err)
	renewedBytes, err := json.Marshal(renewed)
	check(err)
	check(json.Unmarshal(renewedBytes, &session.Lease))
	coldRequest := object(object(corpus["cold_history"])["read"])
	cold, err := api.Call(context.Background(), rrd.OperationColdHistoryRead, coldRequest, read("cold-history", session, nil, time.Time{}))
	check(err)
	coldGeneration := object(cold["generation"])
	coldChanges := array(cold["changes"])
	require(text(cold["scope"]) == text(coldRequest["scope"]), "cold scope differs")
	require(integer(coldGeneration["generation"]) == integer(coldRequest["generation"]), "cold generation differs")
	require(integer(coldGeneration["through_cursor"]) > integer(coldGeneration["requested_after_cursor"]), "cold cursor did not advance")
	require(len(coldChanges) > 0 && integer(coldGeneration["rows"]) == len(coldChanges), "cold changes differ")
	for _, change := range coldChanges {
		require(text(object(change)["scope"]) == text(cold["scope"]), "cold change scope differs")
	}
	seeded := false
	for _, change := range coldChanges {
		if text(object(change)["actor"]) == text(object(corpus["cold_history"])["expected_actor"]) {
			seeded = true
		}
	}
	require(seeded, "cold seeded history is absent")
	_, err = api.Call(context.Background(), rrd.OperationVectorCollectionEnsure, object(vector["ensure"]), mutation("vector-ensure", session, nil, time.Time{}))
	check(err)
	previewLease, err := api.Call(context.Background(), rrd.OperationTransactionBegin, object(transaction["preview_begin"]), mutation("preview-begin", session, nil, time.Time{}))
	check(err)
	previewID := text(previewLease["transaction_id"])
	preview, err := api.Call(context.Background(), rrd.OperationTransactionPreview, object(transaction["preview"]), mutation("transaction-preview", session, map[string]string{"transaction": previewID}, time.Time{}))
	check(err)
	require(text(preview["transaction_id"]) == previewID, "preview transaction differs")
	aborted, err := api.Call(context.Background(), rrd.OperationTransactionAbort, object(transaction["abort"]), mutation("transaction-abort", session, map[string]string{"transaction": previewID}, time.Time{}))
	check(err)
	require(text(aborted["state"]) == "aborted", "transaction did not abort")
	commitLease, err := api.Call(context.Background(), rrd.OperationTransactionBegin, object(transaction["commit_begin"]), mutation("commit-begin", session, nil, time.Time{}))
	check(err)
	commitID := text(commitLease["transaction_id"])
	committed, err := api.Call(context.Background(), rrd.OperationTransactionCommit, object(transaction["commit"]), mutation("transaction-commit", session, map[string]string{"transaction": commitID}, time.Now().Add(time.Duration(integer(transaction["commit_deadline_timeout_ms"]))*time.Millisecond)))
	check(err)
	require(text(committed["operation_sha256"]) == text(object(transaction["commit"])["operation_sha256"]), "commit digest differs")
	query, err := api.Call(context.Background(), rrd.OperationQueryExecute, object(corpus["query"]), read("query", session, nil, time.Time{}))
	check(err)
	require(containsIdentity(array(query["rows"]), text(expected["query_identity"])), "query identity absent")
	vectors, err := api.Call(context.Background(), rrd.OperationVectorSearch, object(vector["search"]), read("vector-search", session, nil, time.Time{}))
	check(err)
	require(containsVector(array(vectors["hits"]), text(expected["vector_reference"])), "vector reference absent")
	// The runners share one engram, so activate the generation this build returned.
	quantization := object(vector["quantization"])
	built, err := api.Call(context.Background(), rrd.OperationVectorQuantizationBuild, object(quantization["build"]), mutation("quantization-build", session, nil, time.Time{}))
	check(err)
	artifact := object(built["artifact"])
	activate := clone(object(quantization["activate"]))
	require(text(artifact["artifact_id"]) == text(activate["artifact_id"]), "built quantization artifact differs")
	activate["generation"] = artifact["generation"]
	_, err = api.Call(context.Background(), rrd.OperationVectorQuantizationActivate, activate, mutation("quantization-activate", session, nil, time.Time{}))
	check(err)
	quantized, err := api.Call(context.Background(), rrd.OperationVectorSearch, object(quantization["search"]), read("quantized-search", session, nil, time.Time{}))
	check(err)
	require(text(quantized["access_path"]) == text(expected["quantized_access_path"]), "quantized search did not use TurboQuant")
	require(containsVector(array(quantized["hits"]), text(expected["vector_reference"])), "quantized vector reference absent")
	// The graph index and its payload index. The changefeed read below proves the
	// scope stays readable with an index in it.
	index := object(vector["index"])
	_, err = api.Call(context.Background(), rrd.OperationVectorPayloadIndexEnsure, object(index["payload_index"]), mutation("payload-index-ensure", session, nil, time.Time{}))
	check(err)
	builtIndex, err := api.Call(context.Background(), rrd.OperationVectorIndexEnsure, object(index["ensure"]), mutation("index-ensure", session, nil, time.Time{}))
	check(err)
	require(text(object(builtIndex["index"])["kind"]) == text(expected["indexed_access_path"]), "index ensure did not build HNSW")
	indexed, err := api.Call(context.Background(), rrd.OperationVectorSearch, object(index["search"]), read("indexed-search", session, nil, time.Time{}))
	check(err)
	require(text(indexed["access_path"]) == text(expected["indexed_access_path"]), "filtered search did not use the graph index")
	require(containsVector(array(indexed["hits"]), text(expected["indexed_reference"])), "indexed vector reference absent")
	_, err = api.Call(context.Background(), rrd.OperationVectorPayloadIndexEnsure, object(index["spare_payload_index"]), mutation("payload-index-spare", session, nil, time.Time{}))
	check(err)
	_, err = api.Call(context.Background(), rrd.OperationVectorPayloadIndexDelete, object(index["delete_payload_index"]), mutation("payload-index-delete", session, nil, time.Time{}))
	check(err)
	changes, err := api.Call(context.Background(), rrd.OperationChangefeedRead, object(changefeed["read"]), read("changefeed-read", session, nil, time.Time{}))
	check(err)
	followRead := clone(object(changefeed["read"]))
	followRead["after_cursor"] = changes["head_cursor"]
	followed, err := api.Call(context.Background(), rrd.OperationChangefeedFollow, map[string]any{"read": followRead, "wait_timeout_ms": changefeed["follow_wait_timeout_ms"]}, read("changefeed-follow", session, nil, time.Time{}))
	check(err)
	require(followed["timed_out"] == true, "changefeed follow did not time out")
	cancelContext, cancel := context.WithTimeout(context.Background(), time.Duration(integer(changefeed["cancel_after_ms"]))*time.Millisecond)
	_, err = api.Call(cancelContext, rrd.OperationChangefeedFollow, map[string]any{"read": followRead, "wait_timeout_ms": changefeed["cancellation_wait_timeout_ms"]}, read("changefeed-cancel", session, nil, time.Time{}))
	cancel()
	require(err != nil, "caller cancellation was not observed")
	backup, err := api.Call(context.Background(), rrd.OperationBackupCreate, object(backupFixture["create"]), mutation("backup-create", session, nil, time.Time{}))
	check(err)
	backupSnapshot := object(backup["backup"])
	backups, err := api.Call(context.Background(), rrd.OperationBackupList, object(backupFixture["list"]), read("backup-list", session, nil, time.Time{}))
	check(err)
	require(containsBackup(array(backups["backups"]), text(backupSnapshot["backup_sha256"])), "backup absent")
	engram, err := api.Call(context.Background(), rrd.OperationEngramRead, object(corpus["engram"]), read("engram-read", session, map[string]string{"engram": text(identity["engram"])}, time.Time{}))
	check(err)
	require(integer(engram["revision"]) == integer(expected["engram_revision"]), "engram revision differs")
	closed, err := api.Call(context.Background(), rrd.OperationSessionClose, object(sessionFixture["close"]), mutation("session-close", session, map[string]string{"session": session.Lease.SessionID}, time.Time{}))
	check(err)
	require(text(closed["state"]) == "closed", "session did not close")
	fmt.Printf("SDK conformance OK: language=go corpus_sha256=%s\n", harness.CorpusSHA256)
}

func client(baseURL, instance string, attempts int) *rrd.Client {
	result, err := rrd.NewClient(rrd.Config{BaseURL: baseURL, Instance: instance, RequestTimeout: 10 * time.Second, MaxAttempts: attempts})
	check(err)
	return result
}

func mutation(step string, session *rrd.Session, paths map[string]string, deadline time.Time) rrd.RequestOptions {
	return rrd.RequestOptions{RequestID: "go-" + step + "-request", OperationID: "go-" + step + "-operation", IdempotencyKey: "go-" + step + "-key", Session: session, PathParameters: paths, Deadline: deadline}
}

func read(step string, session *rrd.Session, paths map[string]string, deadline time.Time) rrd.RequestOptions {
	return rrd.RequestOptions{RequestID: "go-" + step + "-request", OperationID: "go-" + step + "-operation", Session: session, PathParameters: paths, Deadline: deadline}
}

func decodeFile(path string, value any) {
	encoded, err := os.ReadFile(path)
	check(err)
	check(json.Unmarshal(encoded, value))
}
func object(value any) map[string]any {
	result, ok := value.(map[string]any)
	require(ok, "value is not an object")
	return result
}
func array(value any) []any {
	result, ok := value.([]any)
	require(ok, "value is not an array")
	return result
}
func text(value any) string {
	result, ok := value.(string)
	require(ok, "value is not text")
	return result
}
func integer(value any) int {
	result, ok := value.(float64)
	require(ok, "value is not numeric")
	return int(result)
}
func clone(value map[string]any) map[string]any {
	result := make(map[string]any, len(value))
	maps.Copy(result, value)
	return result
}
func containsIdentity(rows []any, expected string) bool {
	for _, row := range rows {
		if text(object(row)["identity"]) == expected {
			return true
		}
	}
	return false
}
func containsVector(hits []any, expected string) bool {
	for _, hit := range hits {
		reference := object(object(hit)["reference"])
		if text(reference["kind"])+":"+text(reference["id"]) == expected {
			return true
		}
	}
	return false
}
func containsBackup(backups []any, expected string) bool {
	for _, backup := range backups {
		if text(object(backup)["backup_sha256"]) == expected {
			return true
		}
	}
	return false
}
func check(err error) {
	if err != nil {
		panic(err)
	}
}
func require(condition bool, message string) {
	if !condition {
		panic(message)
	}
}
