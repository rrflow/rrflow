"""Request coordinates, resource paths, and current operation validation."""

from __future__ import annotations

import re
from collections.abc import Mapping
from dataclasses import dataclass, field
from typing import Any
from urllib.parse import quote

from .error import RrdClientError
from .session import Session

_CORRELATION = re.compile(r"^[A-Za-z0-9._:-]+$")
_CANONICAL = re.compile(r"^[a-z0-9][a-z0-9._-]*$")
_RESOURCE_KINDS = {
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
    "operation",
}


@dataclass(frozen=True, slots=True)
class ResourceSegment:
    kind: str
    id: str


@dataclass(frozen=True, slots=True)
class RequestOptions:
    request_id: str | None = None
    operation_id: str | None = None
    idempotency_key: str | None = None
    deadline_unix_ms: int | None = None
    path_parameters: dict[str, str] = field(default_factory=dict)
    resource: tuple[ResourceSegment, ...] | None = None
    session: Session | None = None
    principal_id: str | None = None
    api_key: str | None = None


def request_context(options: RequestOptions, mutation: bool) -> dict[str, Any]:
    request_id = correlation_id(options.request_id, "request ID")
    operation_id = correlation_id(options.operation_id, "operation ID")
    idempotency_key = (
        correlation_id(options.idempotency_key, "idempotency key")
        if options.idempotency_key
        else None
    )
    if mutation and idempotency_key is None:
        raise RrdClientError("mutating requests require an idempotency key")
    if options.deadline_unix_ms is not None and options.deadline_unix_ms <= 0:
        raise RrdClientError("deadline must be a positive Unix millisecond integer")
    return {
        "request_id": request_id,
        "operation_id": operation_id,
        **({"idempotency_key": idempotency_key} if idempotency_key else {}),
        **({"deadline_unix_ms": options.deadline_unix_ms} if options.deadline_unix_ms else {}),
    }


def default_resource(instance: str, parameters: Mapping[str, str]) -> tuple[ResourceSegment, ...]:
    segments = []
    if engram := parameters.get("engram"):
        segments.append(ResourceSegment("engram", canonical_id(engram, "engram")))
    segments.append(ResourceSegment("instance", instance))
    return tuple(segments)


def resource_segments(segments: tuple[ResourceSegment, ...]) -> list[dict[str, str]]:
    if not 1 <= len(segments) <= 16:
        raise RrdClientError("resource paths must contain 1..=16 segments")
    kinds: set[str] = set()
    result = []
    for segment in segments:
        if segment.kind not in _RESOURCE_KINDS or segment.kind in kinds:
            raise RrdClientError("resource kind is unknown or repeated")
        kinds.add(segment.kind)
        result.append({"kind": segment.kind, "id": canonical_id(segment.id, "resource")})
    return result


def resolve_path(template: str, parameters: Mapping[str, str]) -> str:
    def replace(match: re.Match[str]) -> str:
        name = match.group(1)
        value = parameters.get(name)
        if not value:
            raise RrdClientError(f"missing path parameter {name}")
        return quote(correlation_id(value, f"{name} path parameter"), safe="")

    return re.sub(r"\{([a-z]+)\}", replace, template)


def correlation_id(value: str | None, label: str) -> str:
    if value is None or len(value) > 128 or not _CORRELATION.fullmatch(value):
        raise RrdClientError(f"{label} is not a canonical RRD correlation ID")
    return value


def canonical_id(value: str, label: str) -> str:
    if len(value) > 128 or not _CANONICAL.fullmatch(value):
        raise RrdClientError(f"{label} is not a canonical RRD identifier")
    return value
