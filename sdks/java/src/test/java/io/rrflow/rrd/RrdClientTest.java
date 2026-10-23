package io.rrflow.rrd;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import com.sun.net.httpserver.HttpExchange;
import com.sun.net.httpserver.HttpServer;
import java.io.IOException;
import java.net.InetSocketAddress;
import java.nio.charset.StandardCharsets;
import java.time.Instant;
import java.util.ArrayList;
import java.util.List;
import java.util.Map;
import java.util.concurrent.atomic.AtomicInteger;
import org.junit.jupiter.api.Test;
import tools.jackson.databind.JsonNode;
import tools.jackson.databind.json.JsonMapper;

final class RrdClientTest {
    private static final JsonMapper JSON = JsonMapper.builder().build();

    @Test
    void publicNegotiationRetriesTransportLossAndValidatesIdentity() throws Exception {
        AtomicInteger attempts = new AtomicInteger();
        HttpServer server = server(exchange -> {
            if (attempts.incrementAndGet() == 1) {
                exchange.close();
                return;
            }
            assertEquals("GET", exchange.getRequestMethod());
            assertEquals("/v1/capabilities", exchange.getRequestURI().getPath());
            write(exchange, 200, ok("server-request", "server-operation", Map.of(
                    "protocol", "rrd",
                    "protocol_version", 1,
                    "implementation", "rrd-server",
                    "implementation_version", "1.0.0",
                    "deployment", Map.of(
                            "contract_version", 1,
                            "deployment_form", "single_node_server",
                            "storage_profile", "rrflow_kv",
                            "endpoint_presentation", "loopback_http_websocket"),
                    "configuration", Map.of(
                            "format_version", 3,
                            "revision", 1,
                            "clock", Map.of("maximum_rollback_ms", 0),
                            "storage", Map.ofEntries(
                                    Map.entry("mutable_accounted_bytes", 134217728),
                                    Map.entry("mutable_versions", 1048576),
                                    Map.entry("projected_output_bytes", 134217728),
                                    Map.entry("page_cache_bytes", 4194304),
                                    Map.entry("wal_flush_target_bytes", 67108864),
                                    Map.entry("memtable_flush_target_versions", 524288),
                                    Map.entry("compaction_input_bytes", 1073741824L),
                                    Map.entry("compaction_input_logical_bytes", 268435456L),
                                    Map.entry("compaction_input_versions", 4194304L),
                                    Map.entry("compaction_output_segments", 256L),
                                    Map.entry("compaction_elapsed_ms", 30000L),
                                    Map.entry("compaction_l0_segments", 64L),
                                    Map.entry("retained_mutable_bytes", 536870912L),
                                    Map.entry("retained_mutable_versions", 4194304L),
                                    Map.entry("retained_mutable_generations", 64L),
                                    Map.entry("reclamation_inventory_entries", 262144L),
                                    Map.entry("reclamation_metadata_bytes", 67108864L),
                                    Map.entry("reclamation_delete_files", 1024L),
                                    Map.entry("reclamation_delete_bytes", 1073741824L),
                                    Map.entry("reclamation_elapsed_ms", 30000L),
                                    Map.entry("transaction_admission_requests", 64L),
                                    Map.entry("transaction_admission_bytes", 67108864L),
                                    Map.entry("transaction_admission_wait_ms", 30000L),
                                    Map.entry("recovery_metadata_bytes", 67108864L),
                                    Map.entry("recovery_files", 262144L),
                                    Map.entry("recovery_read_bytes", 1099511627776L),
                                    Map.entry("recovery_wal_bytes", 268435456L),
                                    Map.entry("recovery_wal_frames", 1048576L),
                                    Map.entry("recovery_elapsed_ms", 120000L)),
                            "reasoning", Map.of(
                                    "max_run_elapsed_ms", 900000,
                                    "max_steps", 256,
                                    "max_step_elapsed_ms", 60000),
                            "recall", Map.of(
                                    "max_graph_depth", 4,
                                    "max_items", 128,
                                    "max_output_bytes", 524288,
                                    "max_storage_keys", 100000),
                            "query", Map.of(
                                    "max_storage_keys", 100000,
                                    "max_rows", 10000,
                                    "max_output_bytes", 524288,
                                    "max_batch_rows", 256,
                                    "max_memory_bytes", 67108864,
                                    "max_spill_bytes", 268435456,
                                    "max_elapsed_ms", 30000),
                            "configuration_sha256",
                            "408211d1644753c91e002a8b41af1b5aa14ceff60ce6609addf3d3fc83ded7e0"),
                    "instance", Map.of("kind", "instance", "id", "sdk-test"),
                    "capabilities", List.of())));
        });
        try {
            RrdClient client = new RrdClient(baseUrl(server), "sdk-test");
            JsonNode capabilities = client.capabilities();
            assertEquals(1, capabilities.path("protocol_version").asInt());
            assertEquals(
                    "single_node_server",
                    capabilities.path("deployment").path("deployment_form").asString());
            assertEquals(2, attempts.get());
        } finally {
            server.stop(0);
        }
    }

    @Test
    void sessionAndQueryConstructAuthenticatedV1Envelopes() throws Exception {
        List<String> authorization = new ArrayList<>();
        List<JsonNode> envelopes = new ArrayList<>();
        HttpServer server = server(exchange -> {
            JsonNode request = JSON.readTree(exchange.getRequestBody().readAllBytes());
            authorization.add(exchange.getRequestHeaders().getFirst("Authorization"));
            envelopes.add(request);
            JsonNode context = request.path("context");
            Object payload = exchange.getRequestURI().getPath().equals("/v1/sessions")
                    ? Map.of(
                            "session_id", "session-1",
                            "token", "token-1",
                            "issued_at_unix_ms", 100,
                            "idle_expires_at_unix_ms", 60100,
                            "absolute_expires_at_unix_ms", 300100,
                            "limits", Map.of(
                                    "idle_timeout_ms", 60000,
                                    "absolute_timeout_ms", 300000,
                                    "max_open_transactions", 2))
                    : Map.of(
                            "canonical_query", "FROM record:document",
                            "scope", "instance:sdk-test",
                            "read_manifest_sha256", "a".repeat(64),
                            "known_at_cursor", 2,
                            "schema_revision", 1,
                            "plan", Map.of(),
                            "execution", Map.of(),
                            "rows", List.of());
            write(exchange, 200, ok(
                    context.path("request_id").asString(),
                    context.path("operation_id").asString(),
                    payload));
        });
        try {
            RrdClient client = new RrdClient(baseUrl(server), "sdk-test");
            Session session = client.createSession(
                    "java-sdk",
                    "not-persisted",
                    Map.of("limits", Map.of(
                            "idle_timeout_ms", 60000,
                            "absolute_timeout_ms", 300000,
                            "max_open_transactions", 2)),
                    options("request-session", "operation-session", "session-key", null));
            JsonNode query = client.call(
                    OperationId.QUERY_EXECUTE,
                    Map.of(
                            "scope", "instance:sdk-test",
                            "query", "FROM record:document",
                            "parameters", Map.of(),
                            "budget", Map.of(
                                    "max_storage_keys", 100,
                                    "max_rows", 10,
                                    "max_output_bytes", 4096,
                                    "max_batch_rows", 10)),
                    new RequestOptions(
                            "request-query", "operation-query", null, null,
                            Map.of(), null, session, null));
            assertEquals(2, query.path("known_at_cursor").asInt());
            assertEquals(List.of("ApiKey not-persisted", "Bearer token-1"), authorization);
            assertEquals(
                    "sdk-test",
                    envelopes.get(1).path("resource").path("segments").get(0).path("id").asString());
        } finally {
            server.stop(0);
        }
    }

    @Test
    void remoteCleartextDeadlinesTypedErrorsAndGenerationAreEnforced() throws Exception {
        assertThrows(
                RrdClientException.class,
                () -> new RrdClient("http://192.0.2.1:9477", "sdk-test"));
        HttpServer server = server(exchange -> {
            JsonNode request = JSON.readTree(exchange.getRequestBody().readAllBytes());
            JsonNode context = request.path("context");
            write(exchange, 403, Map.of(
                    "protocol", "rrd",
                    "protocol_version", 1,
                    "request_id", context.path("request_id").asString(),
                    "operation_id", context.path("operation_id").asString(),
                    "outcome", Map.of(
                            "status", "error",
                            "error", Map.of(
                                    "code", "permission_denied",
                                    "message", "policy denied",
                                    "retryable", false))));
        });
        try {
            RrdClient client = new RrdClient(baseUrl(server), "sdk-test");
            Session session = new Session(
                    "java-sdk", new Session.SessionLease("session-1", "token-1"));
            RrdApiException error = assertThrows(
                    RrdApiException.class,
                    () -> client.call(
                            OperationId.AUDIT_READ,
                            Map.of("after_sequence", 0, "limit", 10),
                            new RequestOptions(
                                    "request-audit", "operation-audit", null, null,
                                    Map.of(), null, session, null)));
            assertEquals("permission_denied", error.code());
            assertTrue(error.details().isEmpty());
            RrdClientException expired = assertThrows(
                    RrdClientException.class,
                    () -> client.call(
                            OperationId.AUDIT_READ,
                            Map.of("after_sequence", 0, "limit", 10),
                            new RequestOptions(
                                    "request-expired", "operation-expired", null,
                                    Instant.ofEpochMilli(1), Map.of(), null, session, null)));
            assertTrue(expired.getMessage().contains("deadline has expired"));
            assertEquals(48, OperationId.values().length);
            assertTrue(OperationId.BACKUP_CREATE.mutation());
            assertTrue(OperationId.TRANSACTION_PREVIEW.mutation());
            assertEquals(
                    OperationId.Authentication.PUBLIC,
                    OperationId.CAPABILITIES_READ.authentication());
        } finally {
            server.stop(0);
        }
    }

    private static RequestOptions options(
            String requestId, String operationId, String idempotencyKey, Session session) {
        return new RequestOptions(
                requestId, operationId, idempotencyKey, null, Map.of(), null, session, null);
    }

    private static HttpServer server(ThrowingHandler handler) throws IOException {
        HttpServer server = HttpServer.create(new InetSocketAddress("127.0.0.1", 0), 0);
        server.createContext("/", exchange -> {
            try {
                handler.handle(exchange);
            } catch (Exception error) {
                exchange.close();
                throw new IOException(error);
            }
        });
        server.start();
        return server;
    }

    private static String baseUrl(HttpServer server) {
        return "http://127.0.0.1:" + server.getAddress().getPort();
    }

    private static Map<String, Object> ok(String requestId, String operationId, Object payload) {
        return Map.of(
                "protocol", "rrd",
                "protocol_version", 1,
                "request_id", requestId,
                "operation_id", operationId,
                "outcome", Map.of("status", "ok", "payload", payload));
    }

    private static void write(HttpExchange exchange, int status, Object body) throws IOException {
        byte[] encoded = JSON.writeValueAsBytes(body);
        exchange.getResponseHeaders().set("Content-Type", "application/json");
        exchange.sendResponseHeaders(status, encoded.length);
        exchange.getResponseBody().write(encoded);
        exchange.close();
    }

    @FunctionalInterface
    private interface ThrowingHandler {
        void handle(HttpExchange exchange) throws Exception;
    }
}
