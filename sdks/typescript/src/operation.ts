/** Generated operation typing plus current request-coordinate validation. */

import { RrdClientError } from "./error.js";
import type { OperationId } from "./generated/endpoints.js";
import type { operations, paths } from "./generated/rrd-openapi.js";
import type { Session } from "./session.js";

export type { OperationId, operations, paths };

export type RequestPayload<K extends OperationId> = operations[K] extends {
  requestBody: { content: { "application/json": infer Envelope } };
}
  ? Envelope extends { payload: infer Payload }
    ? Payload
    : never
  : undefined;

export type SuccessPayload<K extends OperationId> = operations[K] extends {
  responses: { 200: { content: { "application/json": infer Envelope } } };
}
  ? Envelope extends { outcome: infer Outcome }
    ? Outcome extends { status: "ok"; payload: infer Payload }
      ? Payload
      : never
    : never
  : never;

export interface ResourceSegment {
  kind:
    | "organization"
    | "engram"
    | "project"
    | "instance"
    | "node"
    | "shard"
    | "collection"
    | "table"
    | "record"
    | "transaction"
    | "snapshot"
    | "backup"
    | "operation";
  id: string;
}

export interface RequestOptions {
  requestId?: string;
  operationId?: string;
  idempotencyKey?: string;
  deadlineUnixMs?: number;
  pathParameters?: Readonly<Record<string, string>>;
  resource?: readonly ResourceSegment[];
  session?: Session;
  apiKey?: { principalId: string; credential: string };
  signal?: AbortSignal;
}

export interface RequestContext {
  request_id: string;
  operation_id: string;
  idempotency_key?: string;
  deadline_unix_ms?: number;
}

export function context(options: RequestOptions, mutation: boolean): RequestContext {
  const requestId = correlationId(options.requestId, "request ID");
  const operationId = correlationId(options.operationId, "operation ID");
  const idempotencyKey = options.idempotencyKey
    ? correlationId(options.idempotencyKey, "idempotency key")
    : undefined;
  if (mutation && !idempotencyKey)
    throw new RrdClientError("mutating requests require an idempotency key");
  if (
    options.deadlineUnixMs !== undefined &&
    (!Number.isSafeInteger(options.deadlineUnixMs) || options.deadlineUnixMs <= 0)
  ) {
    throw new RrdClientError("deadline must be a positive safe Unix millisecond integer");
  }
  return {
    request_id: requestId,
    operation_id: operationId,
    ...(idempotencyKey ? { idempotency_key: idempotencyKey } : {}),
    ...(options.deadlineUnixMs ? { deadline_unix_ms: options.deadlineUnixMs } : {}),
  };
}

export function defaultResource(
  instance: string,
  parameters: Readonly<Record<string, string>> | undefined,
): ResourceSegment[] {
  const segments: ResourceSegment[] = [];
  if (parameters?.engram)
    segments.push({ kind: "engram", id: canonicalId(parameters.engram, "engram") });
  segments.push({ kind: "instance", id: instance });
  return segments;
}

export function validateResource(resource: readonly ResourceSegment[]): ResourceSegment[] {
  if (resource.length === 0 || resource.length > 16) {
    throw new RrdClientError("resource paths must contain 1..=16 segments");
  }
  const kinds = new Set<string>();
  return resource.map((segment) => {
    if (kinds.has(segment.kind)) throw new RrdClientError("resource paths must not repeat a kind");
    kinds.add(segment.kind);
    return { kind: segment.kind, id: canonicalId(segment.id, `${segment.kind} resource`) };
  });
}

export function resolvePath(
  template: string,
  parameters: Readonly<Record<string, string>> | undefined,
): string {
  return template.replace(/\{([a-z]+)\}/g, (_, name: string) => {
    const value = parameters?.[name];
    if (!value) throw new RrdClientError(`missing path parameter ${name}`);
    return encodeURIComponent(correlationId(value, `${name} path parameter`));
  });
}

export function correlationId(value: string | undefined, label: string): string {
  if (!value || value.length > 128 || !/^[A-Za-z0-9._:-]+$/.test(value)) {
    throw new RrdClientError(`${label} is not a canonical RRD correlation ID`);
  }
  return value;
}

export function canonicalId(value: string, label: string): string {
  if (value.length > 128 || !/^[a-z0-9][a-z0-9._-]*$/.test(value)) {
    throw new RrdClientError(`${label} is not a canonical RRD identifier`);
  }
  return value;
}
