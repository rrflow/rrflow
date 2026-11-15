package io.rrflow.rrd;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.nio.file.Files;
import java.nio.file.Path;
import java.security.MessageDigest;
import java.time.Duration;
import java.time.Instant;
import java.util.HashSet;
import java.util.HexFormat;
import java.util.Map;
import java.util.Set;
import org.junit.jupiter.api.Assumptions;
import org.junit.jupiter.api.Test;
import tools.jackson.databind.JsonNode;
import tools.jackson.databind.json.JsonMapper;
import tools.jackson.databind.node.ObjectNode;

final class SdkConformanceTest {
    private static final JsonMapper JSON = JsonMapper.builder().build();
    private static final Set<String> REQUIRED_DOMAINS = Set.of(
            "auth",
            "backup",
            "cancellation",
            "cold_history",
            "crud",
            "engram",
            "live_feeds",
            "query",
            "retries",
            "sessions",
            "transactions",
            "typed_errors",
            "vectors",
            "versions");

    @Test
    void sdkConformanceQualifiesTheSharedCorpus() throws Exception {
        String manifestPath = System.getenv("RRD_SDK_CONFORMANCE_MANIFEST");
        Assumptions.assumeTrue(manifestPath != null, "SDK conformance harness is not active");
        JsonNode manifest = JSON.readTree(Files.readAllBytes(Path.of(manifestPath)));
        byte[] corpusBytes = Files.readAllBytes(Path.of(manifest.path("corpus_path").asString()));
        JsonNode corpus = JSON.readTree(corpusBytes);
        assertEquals(corpus.path("format_version").asInt(), manifest.path("format_version").asInt());
        assertEquals(
                manifest.path("corpus_sha256").asString(),
                HexFormat.of().formatHex(MessageDigest.getInstance("SHA-256").digest(corpusBytes)));
        Set<String> domains = new HashSet<>();
        corpus.path("required_domains").forEach(domain -> domains.add(domain.asString()));
        assertEquals(REQUIRED_DOMAINS, domains);

        JsonNode identity = corpus.path("identity");
        JsonNode expected = corpus.path("expected");
        JsonNode sessionFixture = corpus.path("session");
        JsonNode transaction = corpus.path("transaction");
        JsonNode vector = corpus.path("vector");
        JsonNode changefeed = corpus.path("changefeed");
        JsonNode backupFixture = corpus.path("backup");
        String instance = identity.path("instance").asString();

        RrdClient retryClient = client(
                manifest.path("retry_base_urls").path("java").asString(),
                instance,
                expected.path("retry_attempts").asInt());
        assertEquals(
                corpus.path("protocol_version").asInt(),
                retryClient.capabilities().path("protocol_version").asInt());

        RrdClient incompatibleClient =
                client(manifest.path("incompatible_version_url").asString(), instance, 2);
        assertThrows(RrdClientException.class, incompatibleClient::capabilities);

        RrdClient api = client(manifest.path("base_url").asString(), instance, 2);
        assertEquals(corpus.path("protocol").asString(), api.capabilities().path("protocol").asString());
        assertEquals(
                expected.path("endpoint_count").asInt(),
                api.endpointCatalogue().path("endpoints").size());
        RrdApiException authError = assertThrows(
                RrdApiException.class,
                () -> api.createSession(
                        identity.path("principal").asString(),
                        "wrong-sdk-conformance-key",
                        sessionFixture.path("create"),
                        options("wrong-key", null, true, Map.of(), null)));
        assertEquals(expected.path("typed_error").asString(), authError.code());

        Session session = api.createSession(
                identity.path("principal").asString(),
                identity.path("api_key").asString(),
                sessionFixture.path("create"),
                options("session-create", null, true, Map.of(), null));
        JsonNode renewed = api.call(
                OperationId.SESSION_RENEW,
                sessionFixture.path("renew"),
                options(
                        "session-renew",
                        session,
                        true,
                        Map.of("session", session.lease().sessionId()),
                        null));
        session = new Session(
                session.principalId(),
                new Session.SessionLease(
                        renewed.path("session_id").asString(), renewed.path("token").asString()));

        JsonNode coldRequest = corpus.path("cold_history").path("read");
        JsonNode cold = api.call(
                OperationId.COLD_HISTORY_READ,
                coldRequest,
                options("cold-history", session, false, Map.of(), null));
        JsonNode coldGeneration = cold.path("generation");
        assertEquals(coldRequest.path("scope").asString(), cold.path("scope").asString());
        assertEquals(
                coldRequest.path("generation").asLong(),
                coldGeneration.path("generation").asLong());
        assertTrue(coldGeneration.path("through_cursor").asLong()
                > coldGeneration.path("requested_after_cursor").asLong());
        assertTrue(cold.path("changes").size() > 0);
        assertEquals(cold.path("changes").size(), coldGeneration.path("rows").asInt());
        String expectedColdActor = corpus.path("cold_history").path("expected_actor").asString();
        boolean seededCold = false;
        for (JsonNode change : cold.path("changes")) {
            assertEquals(cold.path("scope").asString(), change.path("scope").asString());
            if (expectedColdActor.equals(change.path("actor").asString())) {
                seededCold = true;
            }
        }
        assertTrue(seededCold);

        api.call(
                OperationId.VECTOR_COLLECTION_ENSURE,
                vector.path("ensure"),
                options("vector-ensure", session, true, Map.of(), null));
        JsonNode previewLease = api.call(
                OperationId.TRANSACTION_BEGIN,
                transaction.path("preview_begin"),
                options("preview-begin", session, true, Map.of(), null));
        String previewId = previewLease.path("transaction_id").asString();
        JsonNode preview = api.call(
                OperationId.TRANSACTION_PREVIEW,
                transaction.path("preview"),
                options(
                        "transaction-preview",
                        session,
                        true,
                        Map.of("transaction", previewId),
                        null));
        assertEquals(previewId, preview.path("transaction_id").asString());
        JsonNode aborted = api.call(
                OperationId.TRANSACTION_ABORT,
                transaction.path("abort"),
                options(
                        "transaction-abort",
                        session,
                        true,
                        Map.of("transaction", previewId),
                        null));
        assertEquals("aborted", aborted.path("state").asString());

        JsonNode commitLease = api.call(
                OperationId.TRANSACTION_BEGIN,
                transaction.path("commit_begin"),
                options("commit-begin", session, true, Map.of(), null));
        JsonNode committed = api.call(
                OperationId.TRANSACTION_COMMIT,
                transaction.path("commit"),
                options(
                        "transaction-commit",
                        session,
                        true,
                        Map.of("transaction", commitLease.path("transaction_id").asString()),
                        Instant.now().plusMillis(
                                transaction.path("commit_deadline_timeout_ms").asLong())));
        assertEquals(
                transaction.path("commit").path("operation_sha256").asString(),
                committed.path("operation_sha256").asString());

        JsonNode query = api.call(
                OperationId.QUERY_EXECUTE,
                corpus.path("query"),
                options("query", session, false, Map.of(), null));
        assertTrue(containsIdentity(
                query.path("rows"), expected.path("query_identity").asString()));
        JsonNode vectors = api.call(
                OperationId.VECTOR_SEARCH,
                vector.path("search"),
                options("vector-search", session, false, Map.of(), null));
        assertTrue(containsVector(
                vectors.path("hits"), expected.path("vector_reference").asString()));
        // The runners share one engram, so activate the generation this build returned.
        JsonNode quantization = vector.path("quantization");
        JsonNode built = api.call(
                OperationId.VECTOR_QUANTIZATION_BUILD,
                quantization.path("build"),
                options("quantization-build", session, true, Map.of(), null));
        JsonNode artifact = built.path("artifact");
        assertEquals(
                quantization.path("activate").path("artifact_id").asString(),
                artifact.path("artifact_id").asString());
        ObjectNode activate = (ObjectNode) quantization.path("activate").deepCopy();
        activate.set("generation", artifact.path("generation"));
        api.call(
                OperationId.VECTOR_QUANTIZATION_ACTIVATE,
                activate,
                options("quantization-activate", session, true, Map.of(), null));
        JsonNode quantized = api.call(
                OperationId.VECTOR_SEARCH,
                quantization.path("search"),
                options("quantized-search", session, false, Map.of(), null));
        assertEquals(
                expected.path("quantized_access_path").asString(),
                quantized.path("access_path").asString());
        assertTrue(containsVector(
                quantized.path("hits"), expected.path("vector_reference").asString()));

        // The graph index and its payload index. The changefeed read below proves
        // the scope stays readable with an index in it.
        JsonNode index = vector.path("index");
        api.call(
                OperationId.VECTOR_PAYLOAD_INDEX_ENSURE,
                index.path("payload_index"),
                options("payload-index-ensure", session, true, Map.of(), null));
        JsonNode builtIndex = api.call(
                OperationId.VECTOR_INDEX_ENSURE,
                index.path("ensure"),
                options("index-ensure", session, true, Map.of(), null));
        assertEquals(
                expected.path("indexed_access_path").asString(),
                builtIndex.path("index").path("kind").asString());
        JsonNode indexed = api.call(
                OperationId.VECTOR_SEARCH,
                index.path("search"),
                options("indexed-search", session, false, Map.of(), null));
        assertEquals(
                expected.path("indexed_access_path").asString(),
                indexed.path("access_path").asString());
        assertTrue(containsVector(
                indexed.path("hits"), expected.path("indexed_reference").asString()));
        api.call(
                OperationId.VECTOR_PAYLOAD_INDEX_ENSURE,
                index.path("spare_payload_index"),
                options("payload-index-spare", session, true, Map.of(), null));
        api.call(
                OperationId.VECTOR_PAYLOAD_INDEX_DELETE,
                index.path("delete_payload_index"),
                options("payload-index-delete", session, true, Map.of(), null));

        JsonNode changes = api.call(
                OperationId.CHANGEFEED_READ,
                changefeed.path("read"),
                options("changefeed-read", session, false, Map.of(), null));
        ObjectNode followRead = (ObjectNode) changefeed.path("read").deepCopy();
        followRead.put("after_cursor", changes.path("head_cursor").asLong());
        ObjectNode followPayload = JSON.createObjectNode();
        followPayload.set("read", followRead);
        followPayload.put(
                "wait_timeout_ms", changefeed.path("follow_wait_timeout_ms").asLong());
        JsonNode followed = api.call(
                OperationId.CHANGEFEED_FOLLOW,
                followPayload,
                options("changefeed-follow", session, false, Map.of(), null));
        assertTrue(followed.path("timed_out").asBoolean());

        RrdClient cancellationClient = client(manifest.path("base_url").asString(), instance, 1);
        ObjectNode cancellationPayload = JSON.createObjectNode();
        cancellationPayload.set("read", followRead);
        cancellationPayload.put(
                "wait_timeout_ms", changefeed.path("cancellation_wait_timeout_ms").asLong());
        Session activeSession = session;
        assertThrows(
                RrdClientException.class,
                () -> cancellationClient.call(
                        OperationId.CHANGEFEED_FOLLOW,
                        cancellationPayload,
                        options(
                                "changefeed-cancel",
                                activeSession,
                                false,
                                Map.of(),
                                Instant.now().plusMillis(
                                        changefeed.path("cancel_after_ms").asLong()))));

        JsonNode backup = api.call(
                OperationId.BACKUP_CREATE,
                backupFixture.path("create"),
                options("backup-create", session, true, Map.of(), null));
        JsonNode backupSnapshot = backup.path("backup");
        String labelPrefix = backupFixture.path("create").path("label").asString() + "--";
        String label = backupSnapshot.path("label").asString();
        assertTrue(label.startsWith(labelPrefix));
        assertTrue(label.substring(labelPrefix.length()).matches("^[0-9a-f]{16}$"));
        JsonNode backups = api.call(
                OperationId.BACKUP_LIST,
                backupFixture.path("list"),
                options("backup-list", session, false, Map.of(), null));
        assertTrue(containsBackup(
                backups.path("backups"), backupSnapshot.path("backup_sha256").asString()));
        JsonNode engram = api.call(
                OperationId.ENGRAM_READ,
                corpus.path("engram"),
                options(
                        "engram-read",
                        session,
                        false,
                        Map.of("engram", identity.path("engram").asString()),
                        null));
        assertEquals(expected.path("engram_revision").asInt(), engram.path("revision").asInt());
        JsonNode closed = api.call(
                OperationId.SESSION_CLOSE,
                sessionFixture.path("close"),
                options(
                        "session-close",
                        session,
                        true,
                        Map.of("session", session.lease().sessionId()),
                        null));
        assertEquals("closed", closed.path("state").asString());
        System.err.printf(
                "SDK conformance OK: language=java corpus_sha256=%s%n",
                manifest.path("corpus_sha256").asString());
    }

    private static RrdClient client(String baseUrl, String instance, int attempts) {
        return new RrdClient(
                baseUrl, instance, Duration.ofSeconds(10), attempts, 4 * 1024 * 1024, null);
    }

    private static RequestOptions options(
            String step,
            Session session,
            boolean mutation,
            Map<String, String> paths,
            Instant deadline) {
        return new RequestOptions(
                "java-" + step + "-request",
                "java-" + step + "-operation",
                mutation ? "java-" + step + "-key" : null,
                deadline,
                paths,
                null,
                session,
                null);
    }

    private static boolean containsIdentity(JsonNode rows, String expected) {
        for (JsonNode row : rows) {
            if (expected.equals(row.path("identity").asString())) {
                return true;
            }
        }
        return false;
    }

    private static boolean containsVector(JsonNode hits, String expected) {
        for (JsonNode hit : hits) {
            JsonNode reference = hit.path("reference");
            if (expected.equals(
                    reference.path("kind").asString() + ":" + reference.path("id").asString())) {
                return true;
            }
        }
        return false;
    }

    private static boolean containsBackup(JsonNode backups, String expected) {
        for (JsonNode backup : backups) {
            if (expected.equals(backup.path("backup_sha256").asString())) {
                return true;
            }
        }
        return false;
    }
}
