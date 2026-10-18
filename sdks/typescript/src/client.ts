/** Client construction, discovery calls, and generic typed HTTP dispatch. */

import { type ClientConfig, loopbackUrl } from "./endpoint.js";
import { RrdApiError, RrdClientError } from "./error.js";
import { endpoints, type OperationId } from "./generated/endpoints.js";
import {
  canonicalId,
  context,
  defaultResource,
  type RequestOptions,
  type RequestPayload,
  resolvePath,
  type SuccessPayload,
  validateResource,
} from "./operation.js";
import { attemptLimit, boundedInteger, remainingTimeout } from "./retry.js";
import type { Session } from "./session.js";
import { decodeResponse } from "./transport.js";

export class RrdClient {
  readonly instance: string;
  readonly baseUrl: URL;
  readonly requestTimeoutMs: number;
  readonly maxAttempts: number;
  readonly maxResponseBytes: number;
  readonly transport: typeof globalThis.fetch;

  constructor(config: ClientConfig) {
    this.instance = canonicalId(config.instance, "instance");
    this.baseUrl = loopbackUrl(config.baseUrl);
    this.requestTimeoutMs = boundedInteger(config.requestTimeoutMs ?? 5_000, 1, 300_000, "timeout");
    this.maxAttempts = boundedInteger(config.maxAttempts ?? 2, 1, 8, "max attempts");
    this.maxResponseBytes = boundedInteger(
      config.maxResponseBytes ?? 4 * 1024 * 1024,
      1,
      16 * 1024 * 1024,
      "response limit",
    );
    const transport = config.fetch ?? globalThis.fetch;
    if (!transport) throw new RrdClientError("a Fetch API implementation is required");
    // A browser's fetch must be invoked as a free function: called as a method
    // of the client (`this.transport(...)`) it throws "Illegal invocation".
    this.transport = (input, init) => transport(input, init);
  }

  async capabilities(): Promise<SuccessPayload<"capabilities-read">> {
    const capabilities = await this.call("capabilities-read", undefined, {});
    if (
      capabilities.protocol !== "rrd" ||
      capabilities.protocol_version !== 1 ||
      capabilities.instance.id !== this.instance
    ) {
      throw new RrdClientError("RRD capability protocol or instance identity differs");
    }
    return capabilities;
  }

  endpointCatalogue(): Promise<SuccessPayload<"endpoint-catalogue">> {
    return this.call("endpoint-catalogue", undefined, {});
  }

  openApi(): Promise<SuccessPayload<"openapi-read">> {
    return this.call("openapi-read", undefined, {});
  }

  async createSession(
    principalId: string,
    credential: string,
    payload: RequestPayload<"session-create">,
    options: Omit<RequestOptions, "apiKey" | "session">,
  ): Promise<Session> {
    const canonicalPrincipal = canonicalId(principalId, "principal");
    if (!credential) throw new RrdClientError("API-key credential must not be empty");
    const lease = await this.call("session-create", payload, {
      ...options,
      apiKey: { principalId: canonicalPrincipal, credential },
    });
    return { principalId: canonicalPrincipal, lease };
  }

  async call<K extends OperationId>(
    operation: K,
    payload: RequestPayload<K>,
    options: RequestOptions,
  ): Promise<SuccessPayload<K>> {
    const descriptor = endpoints[operation];
    const requestContext =
      descriptor.method === "GET" ? undefined : context(options, descriptor.mutation);
    const headers = new Headers({ Accept: "application/json" });
    if (descriptor.authentication === "api_key") {
      if (!options.apiKey) throw new RrdClientError(`${operation} requires API-key authentication`);
      headers.set("X-RRD-Principal", canonicalId(options.apiKey.principalId, "principal"));
      headers.set("Authorization", `ApiKey ${options.apiKey.credential}`);
    } else if (descriptor.authentication === "session_bearer") {
      if (!options.session) throw new RrdClientError(`${operation} requires a session`);
      headers.set("X-RRD-Session", options.session.lease.session_id);
      headers.set("Authorization", `Bearer ${options.session.lease.token}`);
    }
    const url = new URL(resolvePath(descriptor.path, options.pathParameters), this.baseUrl);
    let body: string | undefined;
    if (descriptor.method !== "GET") {
      headers.set("Content-Type", "application/json");
      body = JSON.stringify({
        protocol: "rrd",
        protocol_version: 1,
        context: requestContext,
        resource: {
          segments: options.resource
            ? validateResource(options.resource)
            : defaultResource(this.instance, options.pathParameters),
        },
        payload,
      });
    }

    const attempts = attemptLimit(
      descriptor.method,
      descriptor.mutation,
      options.idempotencyKey,
      this.maxAttempts,
    );
    let lastError: unknown;
    for (let attempt = 0; attempt < attempts; attempt += 1) {
      const remaining = remainingTimeout(this.requestTimeoutMs, options.deadlineUnixMs);
      const timeoutSignal = AbortSignal.timeout(remaining);
      const signal = options.signal
        ? AbortSignal.any([options.signal, timeoutSignal])
        : timeoutSignal;
      try {
        const response = await this.transport(url, {
          method: descriptor.method,
          headers,
          ...(body === undefined ? {} : { body }),
          signal,
        });
        return await decodeResponse<K>(response, this.maxResponseBytes, requestContext);
      } catch (error) {
        if (error instanceof RrdApiError || error instanceof RrdClientError) throw error;
        lastError = error;
        if (attempt + 1 === attempts || options.signal?.aborted) break;
      }
    }
    throw new RrdClientError(
      lastError instanceof Error
        ? `RRD transport failed: ${lastError.message}`
        : "RRD transport failed",
    );
  }
}
