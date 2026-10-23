import assert from "node:assert/strict";
import test from "node:test";
import { endpoints } from "../src/generated/endpoints.js";
import { RrdApiError, RrdClient, RrdClientError } from "../src/index.js";

const jsonResponse = (value: unknown, status = 200) =>
  new Response(JSON.stringify(value), {
    status,
    headers: { "content-type": "application/json" },
  });

test("generated catalogue tracks all mutation boundaries", () => {
  assert.equal(Object.keys(endpoints).length, 43);
  assert.equal(endpoints["audit-export"].path, "/v1/audit/export");
  assert.equal(endpoints["backup-create"].mutation, true);
  assert.equal(endpoints["transaction-preview"].mutation, true);
});

test("public negotiation retries transport loss and validates the ArkType envelope", async () => {
  let attempts = 0;
  const transport: typeof fetch = async (input, init) => {
    attempts += 1;
    assert.equal(new URL(input.toString()).pathname, "/v1/capabilities");
    assert.equal(init?.method, "GET");
    if (attempts === 1) throw new TypeError("connection reset");
    return jsonResponse({
      protocol: "rrd",
      protocol_version: 1,
      request_id: "server-request",
      operation_id: "server-operation",
      outcome: {
        status: "ok",
        payload: {
          protocol: "rrd",
          protocol_version: 1,
          implementation: "rrd-server",
          implementation_version: "1.0.0",
          deployment: {
            contract_version: 1,
            deployment_form: "single_node_server",
            storage_profile: "rrflow_kv",
            endpoint_presentation: "loopback_http_websocket",
          },
          configuration: {
            format_version: 3,
            revision: 1,
            clock: {
              maximum_rollback_ms: 0,
            },
            storage: {
              mutable_accounted_bytes: 134217728,
              mutable_versions: 1048576,
              projected_output_bytes: 134217728,
              page_cache_bytes: 4194304,
              wal_flush_target_bytes: 67108864,
              memtable_flush_target_versions: 524288,
              compaction_input_bytes: 1073741824,
              compaction_input_logical_bytes: 268435456,
              compaction_input_versions: 4194304,
              compaction_output_segments: 256,
              compaction_elapsed_ms: 30000,
              compaction_l0_segments: 64,
              retained_mutable_bytes: 536870912,
              retained_mutable_versions: 4194304,
              retained_mutable_generations: 64,
              reclamation_inventory_entries: 262144,
              reclamation_metadata_bytes: 67108864,
              reclamation_delete_files: 1024,
              reclamation_delete_bytes: 1073741824,
              reclamation_elapsed_ms: 30000,
              transaction_admission_requests: 64,
              transaction_admission_bytes: 67108864,
              transaction_admission_wait_ms: 30000,
              recovery_metadata_bytes: 67108864,
              recovery_files: 262144,
              recovery_read_bytes: 1099511627776,
              recovery_wal_bytes: 268435456,
              recovery_wal_frames: 1048576,
              recovery_elapsed_ms: 120000,
            },
            reasoning: {
              max_run_elapsed_ms: 900_000,
              max_steps: 256,
              max_step_elapsed_ms: 60_000,
            },
            recall: {
              max_graph_depth: 4,
              max_items: 128,
              max_output_bytes: 524_288,
              max_storage_keys: 100_000,
            },
            query: {
              max_storage_keys: 100_000,
              max_rows: 10_000,
              max_output_bytes: 524_288,
              max_batch_rows: 256,
              max_memory_bytes: 67_108_864,
              max_spill_bytes: 268_435_456,
              max_elapsed_ms: 30_000,
            },
            configuration_sha256:
              "408211d1644753c91e002a8b41af1b5aa14ceff60ce6609addf3d3fc83ded7e0",
          },
          instance: { kind: "instance", id: "sdk-test" },
          capabilities: [],
        },
      },
    });
  };
  const client = new RrdClient({
    baseUrl: "http://127.0.0.1:9477",
    instance: "sdk-test",
    fetch: transport,
  });
  const capabilities = await client.capabilities();
  assert.equal(capabilities.protocol_version, 1);
  assert.equal(capabilities.deployment.deployment_form, "single_node_server");
  assert.equal(capabilities.configuration.reasoning.max_run_elapsed_ms, 900_000);
  assert.equal(attempts, 2);
});

test("the transport is invoked as a free function, as a browser's fetch requires", async () => {
  // Browsers throw "Illegal invocation" when `fetch` runs with any `this`
  // other than the global object; the client must never call it as a method.
  const transport = function (this: unknown, input: RequestInfo | URL) {
    if (this !== undefined && this !== globalThis) {
      throw new TypeError("Failed to execute 'fetch' on 'Window': Illegal invocation");
    }
    assert.equal(new URL(input.toString()).pathname, "/v1/health/live");
    return Promise.resolve(
      jsonResponse({
        protocol: "rrd",
        protocol_version: 1,
        request_id: "server-request",
        operation_id: "server-operation",
        outcome: { status: "ok", payload: { observed_at_unix_ms: 7 } },
      }),
    );
  } as typeof fetch;
  const client = new RrdClient({
    baseUrl: "http://127.0.0.1:9477",
    instance: "sdk-test",
    maxAttempts: 1,
    fetch: transport,
  });
  const liveness = await client.call("health-live", undefined, {});
  assert.equal(liveness.observed_at_unix_ms, 7);
});

test("session and query calls construct authenticated bounded v1 envelopes", async () => {
  const requests: { url: URL; init: RequestInit }[] = [];
  const transport: typeof fetch = async (input, init = {}) => {
    const url = new URL(input.toString());
    requests.push({ url, init });
    const envelope = JSON.parse(String(init.body)) as {
      context: { request_id: string; operation_id: string };
    };
    const payload =
      url.pathname === "/v1/sessions"
        ? {
            session_id: "session-1",
            token: "token-1",
            issued_at_unix_ms: 100,
            idle_expires_at_unix_ms: 60_100,
            absolute_expires_at_unix_ms: 300_100,
            limits: {
              idle_timeout_ms: 60_000,
              absolute_timeout_ms: 300_000,
              max_open_transactions: 2,
            },
          }
        : {
            canonical_query: "FROM record:document",
            scope: "instance:sdk-test",
            read_manifest_sha256: "a".repeat(64),
            known_at_cursor: 2,
            schema_revision: 1,
            plan: {
              plan_sha256: "b".repeat(64),
              exact: true,
              deterministic_order: "identity",
              authorization_boundary: "instance:sdk-test",
              candidates: [],
            },
            execution: {
              selected_versions: 2,
              read_evidence: {
                contract_version: 1,
                key_budget: 100,
                point_reads: 1,
                range_scans: 1,
                keys_examined: 2,
                values_decoded: 2,
                decoded_bytes: 128,
                stamp_validation: {
                  method: "rfc9162_direct_versions",
                  change_reads: 0,
                  proof_nodes: 1,
                },
                paths: [
                  {
                    path: "record_versions",
                    point_reads: 0,
                    range_scans: 1,
                    keys_examined: 2,
                    values_decoded: 2,
                    decoded_bytes: 128,
                  },
                  {
                    path: "accumulator_proof",
                    point_reads: 1,
                    range_scans: 0,
                    keys_examined: 0,
                    values_decoded: 0,
                    decoded_bytes: 0,
                  },
                ],
              },
              returned_rows: 0,
              output_bytes: 2,
              truncated: false,
            },
            rows: [],
          };
    return jsonResponse({
      protocol: "rrd",
      protocol_version: 1,
      request_id: envelope.context.request_id,
      operation_id: envelope.context.operation_id,
      outcome: { status: "ok", payload },
    });
  };
  const client = new RrdClient({
    baseUrl: "http://localhost:9477",
    instance: "sdk-test",
    fetch: transport,
  });
  const session = await client.createSession(
    "typescript-sdk",
    "not-persisted",
    {
      limits: {
        idle_timeout_ms: 60_000,
        absolute_timeout_ms: 300_000,
        max_open_transactions: 2,
      },
    },
    {
      requestId: "request-session",
      operationId: "operation-session",
      idempotencyKey: "session-key",
    },
  );
  const query = await client.call(
    "query-execute",
    {
      scope: "instance:sdk-test",
      query: "FROM record:document",
      parameters: {},
      budget: {
        max_storage_keys: 100,
        max_rows: 10,
        max_output_bytes: 4_096,
        max_batch_rows: 10,
      },
    },
    { requestId: "request-query", operationId: "operation-query", session },
  );
  assert.equal(query.known_at_cursor, 2);
  assert.equal(new Headers(requests[0]?.init.headers).get("authorization"), "ApiKey not-persisted");
  assert.equal(new Headers(requests[1]?.init.headers).get("authorization"), "Bearer token-1");
  const queryEnvelope = JSON.parse(String(requests[1]?.init.body));
  assert.deepEqual(queryEnvelope.resource.segments, [{ kind: "instance", id: "sdk-test" }]);
});

test("the client rejects remote cleartext, expired deadlines, and typed API errors", async () => {
  assert.throws(
    () => new RrdClient({ baseUrl: "http://192.0.2.1:9477", instance: "sdk-test" }),
    RrdClientError,
  );
  const client = new RrdClient({
    baseUrl: "http://127.0.0.1:9477",
    instance: "sdk-test",
    maxAttempts: 1,
    fetch: async (_input, init = {}) => {
      const envelope = JSON.parse(String(init.body));
      return jsonResponse(
        {
          protocol: "rrd",
          protocol_version: 1,
          request_id: envelope.context.request_id,
          operation_id: envelope.context.operation_id,
          outcome: {
            status: "error",
            error: {
              code: "permission_denied",
              message: "policy denied",
              retryable: false,
              details: {},
            },
          },
        },
        403,
      );
    },
  });
  const fakeSession = {
    principalId: "typescript-sdk",
    lease: {
      session_id: "session-1",
      token: "token-1",
      issued_at_unix_ms: 1,
      idle_expires_at_unix_ms: 2,
      absolute_expires_at_unix_ms: 3,
      limits: { idle_timeout_ms: 1_000, absolute_timeout_ms: 1_000, max_open_transactions: 1 },
    },
  };
  await assert.rejects(
    client.call(
      "audit-read",
      { after_sequence: 0, limit: 10 },
      { requestId: "request-audit", operationId: "operation-audit", session: fakeSession },
    ),
    (error) => error instanceof RrdApiError && error.code === "permission_denied",
  );
  await assert.rejects(
    client.call(
      "audit-read",
      { after_sequence: 0, limit: 10 },
      {
        requestId: "request-expired",
        operationId: "operation-expired",
        deadlineUnixMs: 1,
        session: fakeSession,
      },
    ),
    /deadline has expired/,
  );
});
