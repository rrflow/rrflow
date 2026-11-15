package io.rrflow.rrd;

import java.net.http.HttpRequest;
import java.util.ArrayList;
import java.util.HashMap;
import java.util.HashSet;
import java.util.List;
import java.util.Map;
import java.util.Set;
import java.util.regex.Pattern;

/** Current common operation binding; H-04 owns generated concrete bindings. */
final class OperationBinding {
    private static final Pattern CORRELATION = Pattern.compile("^[A-Za-z0-9._:-]+$");
    private static final Pattern CANONICAL = Pattern.compile("^[a-z0-9][a-z0-9._-]*$");
    private static final Set<String> RESOURCE_KINDS = Set.of(
            "organization",
            "engram",
            "project",
            "instance",
            "node",
            "shard",
            "collection",
            "table",
            "record",
            "transaction",
            "snapshot",
            "backup",
            "operation");

    private final String instance;

    OperationBinding(String instance) {
        this.instance = instance;
    }

    Map<String, Object> requestContext(OperationId operation, RequestOptions options) {
        if (operation.method().equals("GET")) {
            return null;
        }
        Map<String, Object> context = new HashMap<>();
        context.put("request_id", correlation(options.requestId(), "request ID"));
        context.put("operation_id", correlation(options.operationId(), "operation ID"));
        if (options.idempotencyKey() != null) {
            context.put(
                    "idempotency_key",
                    correlation(options.idempotencyKey(), "idempotency key"));
        } else if (operation.mutation()) {
            throw new RrdClientException("mutating requests require an idempotency key");
        }
        if (options.deadline() != null) {
            long milliseconds = options.deadline().toEpochMilli();
            if (milliseconds <= 0) {
                throw new RrdClientException(
                        "deadline must be a positive Unix millisecond instant");
            }
            context.put("deadline_unix_ms", milliseconds);
        }
        return context;
    }

    void authenticate(
            HttpRequest.Builder builder, OperationId operation, RequestOptions options) {
        switch (operation.authentication()) {
            case PUBLIC -> { }
            case API_KEY -> {
                RequestOptions.ApiKey apiKey = options.apiKey();
                if (apiKey == null
                        || apiKey.credential() == null
                        || apiKey.credential().isEmpty()) {
                    throw new RrdClientException(
                            operation.wireName() + " requires API-key authentication");
                }
                builder.header(
                        "X-RRD-Principal", canonical(apiKey.principalId(), "principal"));
                builder.header("Authorization", "ApiKey " + apiKey.credential());
            }
            case SESSION_BEARER -> {
                Session session = options.session();
                if (session == null
                        || session.lease().sessionId().isEmpty()
                        || session.lease().token().isEmpty()) {
                    throw new RrdClientException(
                            operation.wireName() + " requires a valid session");
                }
                builder.header("X-RRD-Session", session.lease().sessionId());
                builder.header("Authorization", "Bearer " + session.lease().token());
            }
        }
    }

    List<ResourceSegment> resource(RequestOptions options) {
        List<ResourceSegment> resource = options.resource();
        if (resource == null) {
            resource = defaultResource(options.pathParameters());
        }
        return validateResource(resource);
    }

    private List<ResourceSegment> defaultResource(Map<String, String> parameters) {
        List<ResourceSegment> result = new ArrayList<>();
        String engram = parameters.get("engram");
        if (engram != null) {
            result.add(new ResourceSegment("engram", canonical(engram, "engram")));
        }
        result.add(new ResourceSegment("instance", instance));
        return result;
    }

    private static List<ResourceSegment> validateResource(List<ResourceSegment> segments) {
        if (segments.isEmpty() || segments.size() > 16) {
            throw new RrdClientException("resource paths must contain 1..=16 segments");
        }
        Set<String> kinds = new HashSet<>();
        List<ResourceSegment> result = new ArrayList<>();
        for (ResourceSegment segment : segments) {
            if (!RESOURCE_KINDS.contains(segment.kind()) || !kinds.add(segment.kind())) {
                throw new RrdClientException("resource kind is unknown or repeated");
            }
            result.add(new ResourceSegment(segment.kind(), canonical(segment.id(), "resource")));
        }
        return List.copyOf(result);
    }

    static String correlation(String value, String label) {
        if (value == null || value.length() > 128 || !CORRELATION.matcher(value).matches()) {
            throw new RrdClientException(label + " is not a canonical RRD correlation ID");
        }
        return value;
    }

    static String canonical(String value, String label) {
        if (value == null || value.length() > 128 || !CANONICAL.matcher(value).matches()) {
            throw new RrdClientException(label + " is not a canonical RRD identifier");
        }
        return value;
    }
}
