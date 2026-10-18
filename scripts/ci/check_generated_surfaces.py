#!/usr/bin/env python3
"""Check or render projections derived from the canonical RRD OpenAPI export."""

from __future__ import annotations

import argparse
import ast
import hashlib
import json
import re
import subprocess
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[2]
KERNEL_SIGNAL_FIXTURE = (
    ROOT / "crates/kernel/rrd-core/fixtures/telemetry-catalogue-v1.json"
)
CONTRACT_SIGNAL_OUTPUT = (
    ROOT / "crates/transport/rrd-contract/src/generated/signal_catalogue.rs"
)
ENDPOINT_OUTPUTS = {
    "typescript schema": ROOT / "sdks/typescript/src/generated/rrd-openapi.ts",
    "typescript endpoints": ROOT / "sdks/typescript/src/generated/endpoints.ts",
    "python endpoints": ROOT / "sdks/python/src/rrd_client/generated/endpoints.py",
    "go endpoints": ROOT / "sdks/go/endpoints_gen.go",
    "java endpoints": ROOT / "sdks/java/src/main/java/io/rrflow/rrd/OperationId.java",
    "dotnet endpoints": ROOT
    / "sdks/dotnet/src/Rrflow.Rrd.Client/Generated/OperationId.g.cs",
}
SIGNAL_OUTPUTS = {
    "signal reference": ROOT / "docs/reference/protocol/signal-catalogue.md",
    "typescript signal catalogue": ROOT
    / "sdks/typescript/src/generated/signal-catalogue.ts",
    "python signal catalogue": ROOT
    / "sdks/python/src/rrd_client/generated/signal_catalogue.py",
    "go signal catalogue": ROOT / "sdks/go/signal_catalogue_gen.go",
    "java signal catalogue": ROOT
    / "sdks/java/src/main/java/io/rrflow/rrd/SignalCatalogue.java",
    "dotnet signal catalogue": ROOT
    / "sdks/dotnet/src/Rrflow.Rrd.Client/Generated/SignalCatalogue.g.cs",
}
# Authored records that cite the current OpenAPI digest by hand. They are not
# rendered by this tool, so the check requires the citation to be current.
DIGEST_CITATIONS = {
    "public contract reference": ROOT / "docs/reference/protocol/public-contract.md",
    "typescript sdk reference": ROOT / "docs/reference/sdk/typescript.md",
    "python sdk reference": ROOT / "docs/reference/sdk/python.md",
    "go sdk reference": ROOT / "docs/reference/sdk/go.md",
    "java sdk reference": ROOT / "docs/reference/sdk/java.md",
    "dotnet sdk reference": ROOT / "docs/reference/sdk/dotnet.md",
}
SIGNAL_TOP_LEVEL_FIELDS = {
    "contract_version",
    "default_diagnostic_level",
    "default_metric_cardinality_limit",
    "diagnostic_levels",
    "metric_attributes",
    "metric_instruments",
    "runtime_trace_contract_version",
    "trace_attributes",
    "trace_operations",
}
SHA256 = re.compile(r"[0-9a-f]{64}")


class SurfaceError(RuntimeError):
    """A deterministic generated-surface policy failure."""


def require(condition: bool, message: str) -> None:
    if not condition:
        raise SurfaceError(message)


def contract_export() -> tuple[bytes, dict[str, Any]]:
    result = subprocess.run(
        [
            "cargo",
            "run",
            "-q",
            "--locked",
            "-p",
            "rrd-contract",
            "--bin",
            "rrd-contract-export",
        ],
        cwd=ROOT,
        check=False,
        capture_output=True,
    )
    if result.returncode != 0:
        diagnostic = result.stderr.decode("utf-8", errors="replace").strip()
        raise SurfaceError(f"rrd-contract export failed: {diagnostic}")
    try:
        document = json.loads(result.stdout)
    except (UnicodeDecodeError, json.JSONDecodeError) as error:
        raise SurfaceError(f"rrd-contract export is not UTF-8 JSON: {error}") from error
    require(isinstance(document, dict), "rrd-contract export root must be an object")
    return result.stdout, document


def canonical_endpoints(document: dict[str, Any]) -> dict[str, dict[str, object]]:
    paths = document.get("paths")
    require(isinstance(paths, dict), "OpenAPI paths must be an object")
    endpoints: dict[str, dict[str, object]] = {}
    for path, path_item in paths.items():
        require(isinstance(path, str), "OpenAPI path name must be a string")
        require(
            isinstance(path_item, dict), f"OpenAPI path item must be an object: {path}"
        )
        for method, operation in path_item.items():
            if method not in {"get", "post", "delete"}:
                continue
            require(
                isinstance(operation, dict),
                f"OpenAPI operation must be an object: {path}",
            )
            operation_id = operation.get("operationId")
            require(
                isinstance(operation_id, str), f"OpenAPI operationId is absent: {path}"
            )
            security = operation.get("security", [])
            scheme = None
            if (
                isinstance(security, list)
                and security
                and isinstance(security[0], dict)
            ):
                scheme = next(iter(security[0]), None)
            authentication = {
                "rrdApiKey": "api_key",
                "rrdBearer": "session_bearer",
            }.get(scheme, "public")
            mutation = operation.get("x-rrd-mutation")
            require(
                isinstance(mutation, bool),
                f"mutation flag is not Boolean: {operation_id}",
            )
            require(
                operation_id not in endpoints, f"duplicate operationId: {operation_id}"
            )
            endpoints[operation_id] = {
                "method": method.upper(),
                "path": path,
                "authentication": authentication,
                "mutation": mutation,
            }
    return dict(sorted(endpoints.items()))


def parse_typescript(source: str) -> dict[str, dict[str, object]]:
    try:
        encoded = source.split("export const endpoints = ", 1)[1].split(
            " as const;", 1
        )[0]
        value = json.loads(encoded)
    except (IndexError, json.JSONDecodeError) as error:
        raise SurfaceError(
            f"cannot parse generated TypeScript endpoints: {error}"
        ) from error
    require(isinstance(value, dict), "generated TypeScript endpoints must be an object")
    return value


def parse_python(source: str) -> dict[str, dict[str, object]]:
    try:
        tree = ast.parse(source)
    except SyntaxError as error:
        raise SurfaceError(
            f"cannot parse generated Python endpoints: {error}"
        ) from error
    for node in tree.body:
        if (
            isinstance(node, ast.AnnAssign)
            and isinstance(node.target, ast.Name)
            and node.target.id == "ENDPOINTS"
            and node.value is not None
        ):
            value = ast.literal_eval(node.value)
            require(
                isinstance(value, dict), "generated Python ENDPOINTS must be a dict"
            )
            return value
    raise SurfaceError("generated Python endpoint map omitted ENDPOINTS")


def parse_go(source: str) -> dict[str, dict[str, object]]:
    names = dict(
        re.findall(
            r'^\s*(Operation\w+)\s+OperationID\s+=\s+"([^"]+)"$', source, re.MULTILINE
        )
    )
    pattern = re.compile(
        r'^\s*(Operation\w+):\s+\{Method: "([^"]+)", Path: "([^"]+)", '
        r'Authentication: "([^"]+)", Mutation: (true|false)\},$',
        re.MULTILINE,
    )
    projection: dict[str, dict[str, object]] = {}
    for symbol, method, path, authentication, mutation in pattern.findall(source):
        require(
            symbol in names, f"generated Go endpoint omits operation constant: {symbol}"
        )
        projection[names[symbol]] = {
            "method": method,
            "path": path,
            "authentication": authentication,
            "mutation": mutation == "true",
        }
    return projection


def parse_java(source: str) -> dict[str, dict[str, object]]:
    pattern = re.compile(
        r'^\s*[A-Z0-9_]+\("([^"]+)", "([^"]+)", "([^"]+)", '
        r"Authentication\.([A-Z_]+), (true|false)\)[,;]$",
        re.MULTILINE,
    )
    authentication_names = {
        "PUBLIC": "public",
        "API_KEY": "api_key",
        "SESSION_BEARER": "session_bearer",
    }
    return {
        operation: {
            "method": method,
            "path": path,
            "authentication": authentication_names[authentication],
            "mutation": mutation == "true",
        }
        for operation, method, path, authentication, mutation in pattern.findall(source)
    }


def parse_dotnet(source: str) -> dict[str, dict[str, object]]:
    pattern = re.compile(
        r'^\s*OperationId\.\w+ => new\("([^"]+)", "([^"]+)", "([^"]+)", '
        r"Authentication\.(\w+), (true|false)\),$",
        re.MULTILINE,
    )
    authentication_names = {
        "Public": "public",
        "ApiKey": "api_key",
        "SessionBearer": "session_bearer",
    }
    return {
        operation: {
            "method": method,
            "path": path,
            "authentication": authentication_names[authentication],
            "mutation": mutation == "true",
        }
        for operation, method, path, authentication, mutation in pattern.findall(source)
    }


def validate_endpoint_outputs(
    document: dict[str, Any], digest: str, *, require_current_digest: bool
) -> int:
    sources: dict[str, str] = {}
    for name, path in ENDPOINT_OUTPUTS.items():
        require(path.is_file(), f"{name} is absent: {path.relative_to(ROOT)}")
        sources[name] = path.read_text(encoding="utf-8")
    if require_current_digest:
        for name, source in sources.items():
            require(
                f"OpenAPI SHA-256: {digest}" in source,
                f"{name} is stale: missing OpenAPI digest {digest}",
            )

    expected = canonical_endpoints(document)
    projections = {
        "typescript endpoints": parse_typescript(sources["typescript endpoints"]),
        "python endpoints": parse_python(sources["python endpoints"]),
        "go endpoints": parse_go(sources["go endpoints"]),
        "java endpoints": parse_java(sources["java endpoints"]),
        "dotnet endpoints": parse_dotnet(sources["dotnet endpoints"]),
    }
    for name, projection in projections.items():
        if projection == expected:
            continue
        missing = sorted(set(expected) - set(projection))
        extra = sorted(set(projection) - set(expected))
        mismatched = sorted(
            operation
            for operation in set(expected) & set(projection)
            if expected[operation] != projection[operation]
        )
        raise SurfaceError(
            f"{name} drifted from rrd-contract: "
            f"missing={missing} extra={extra} mismatched={mismatched}"
        )
    return len(expected)


def validate_digest_citations(digest: str) -> None:
    for name, path in DIGEST_CITATIONS.items():
        require(path.is_file(), f"{name} is absent: {path.relative_to(ROOT)}")
        require(
            digest in path.read_text(encoding="utf-8"),
            f"{name} cites a stale OpenAPI digest: {path.relative_to(ROOT)} "
            f"must cite {digest}",
        )


def require_string_list(
    value: object, field: str, expected_length: int | None = None
) -> list[str]:
    require(isinstance(value, list), f"signal {field} must be an array")
    require(
        all(isinstance(item, str) and item for item in value),
        f"signal {field} must contain nonempty strings",
    )
    strings = list(value)
    require(len(strings) == len(set(strings)), f"signal {field} contains duplicates")
    if expected_length is not None:
        require(
            len(strings) == expected_length,
            f"signal {field} must contain {expected_length} entries",
        )
    return strings


def validate_signal_projection(
    catalogue: object, fingerprint: object, source: str
) -> tuple[dict[str, Any], str]:
    require(isinstance(catalogue, dict), f"{source} signal catalogue must be an object")
    require(
        set(catalogue) == SIGNAL_TOP_LEVEL_FIELDS,
        f"{source} signal catalogue top-level fields drifted: "
        f"missing={sorted(SIGNAL_TOP_LEVEL_FIELDS - set(catalogue))} "
        f"extra={sorted(set(catalogue) - SIGNAL_TOP_LEVEL_FIELDS)}",
    )
    require(
        isinstance(fingerprint, str) and SHA256.fullmatch(fingerprint) is not None,
        f"{source} signal catalogue fingerprint must be lowercase SHA-256",
    )
    require(
        type(catalogue["contract_version"]) is int
        and catalogue["contract_version"] == 1,
        "signal contract_version must be 1",
    )
    require(
        type(catalogue["runtime_trace_contract_version"]) is int
        and catalogue["runtime_trace_contract_version"] == 1,
        "signal runtime_trace_contract_version must be 1",
    )
    require(
        type(catalogue["default_metric_cardinality_limit"]) is int
        and catalogue["default_metric_cardinality_limit"] > 0,
        "signal default_metric_cardinality_limit must be positive",
    )
    levels = require_string_list(catalogue["diagnostic_levels"], "diagnostic_levels", 4)
    require(
        catalogue["default_diagnostic_level"] in levels,
        "signal default_diagnostic_level must belong to diagnostic_levels",
    )
    metric_attributes = require_string_list(
        catalogue["metric_attributes"], "metric_attributes", 9
    )

    trace_operations = catalogue["trace_operations"]
    require(
        isinstance(trace_operations, list) and trace_operations,
        "signal trace_operations must be nonempty",
    )
    operation_names: list[str] = []
    for index, descriptor in enumerate(trace_operations):
        require(
            isinstance(descriptor, dict), f"trace operation {index} must be an object"
        )
        require(
            set(descriptor) == {"boundary", "name"},
            f"trace operation {index} fields drifted",
        )
        require(
            isinstance(descriptor["name"], str) and descriptor["name"],
            f"trace operation {index} name is invalid",
        )
        require(
            isinstance(descriptor["boundary"], str) and descriptor["boundary"],
            f"trace operation {index} boundary is invalid",
        )
        operation_names.append(descriptor["name"])
    require(
        len(operation_names) == len(set(operation_names)),
        "signal trace_operations contains duplicate names",
    )

    trace_attributes = catalogue["trace_attributes"]
    require(
        isinstance(trace_attributes, list) and trace_attributes,
        "signal trace_attributes must be nonempty",
    )
    trace_attribute_names: list[str] = []
    for index, descriptor in enumerate(trace_attributes):
        require(
            isinstance(descriptor, dict), f"trace attribute {index} must be an object"
        )
        require(set(descriptor) == {"name"}, f"trace attribute {index} fields drifted")
        require(
            isinstance(descriptor["name"], str) and descriptor["name"],
            f"trace attribute {index} name is invalid",
        )
        trace_attribute_names.append(descriptor["name"])
    require(
        len(trace_attribute_names) == len(set(trace_attribute_names)),
        "signal trace_attributes contains duplicate names",
    )

    metrics = catalogue["metric_instruments"]
    require(
        isinstance(metrics, list) and len(metrics) == 22,
        "signal metric_instruments must contain 22 entries",
    )
    instruments: list[str] = []
    for index, descriptor in enumerate(metrics):
        require(
            isinstance(descriptor, dict), f"metric instrument {index} must be an object"
        )
        require(
            set(descriptor)
            == {"attributes", "description", "instrument", "kind", "unit"},
            f"metric instrument {index} fields drifted",
        )
        for field in ("description", "instrument", "kind", "unit"):
            require(
                isinstance(descriptor[field], str) and descriptor[field],
                f"metric instrument {index} {field} is invalid",
            )
        attributes = require_string_list(
            descriptor["attributes"], f"metric_instruments[{index}].attributes"
        )
        require(
            set(attributes) <= set(metric_attributes),
            f"metric instrument {index} uses an undeclared attribute",
        )
        instruments.append(descriptor["instrument"])
    require(
        len(instruments) == len(set(instruments)),
        "signal metric_instruments contains duplicate names",
    )
    return catalogue, fingerprint


def kernel_signal_projection() -> tuple[dict[str, Any], str]:
    require(KERNEL_SIGNAL_FIXTURE.is_file(), "kernel signal fixture is absent")
    try:
        fixture = json.loads(KERNEL_SIGNAL_FIXTURE.read_bytes())
    except (UnicodeDecodeError, json.JSONDecodeError) as error:
        raise SurfaceError(
            f"kernel signal fixture is not UTF-8 JSON: {error}"
        ) from error
    require(isinstance(fixture, dict), "kernel signal fixture root must be an object")
    require(
        set(fixture) == {"catalogue", "sha256"},
        "kernel signal fixture fields must be exactly catalogue and sha256",
    )
    return validate_signal_projection(
        fixture.get("catalogue"), fixture.get("sha256"), "kernel fixture"
    )


def signal_projection(document: dict[str, Any]) -> tuple[dict[str, Any], str]:
    return validate_signal_projection(
        document.get("x-rrd-signal-catalogue"),
        document.get("x-rrd-signal-catalogue-sha256"),
        "OpenAPI",
    )


def chunks(value: str, width: int = 64) -> list[str]:
    return [value[index : index + width] for index in range(0, len(value), width)]


def quoted(value: str) -> str:
    return json.dumps(value, ensure_ascii=True)


def python_quoted(value: str) -> str:
    double_quoted = quoted(value)
    single_quoted = (
        "'"
        + (
            value.replace("\\", "\\\\")
            .replace("'", "\\'")
            .replace("\n", "\\n")
            .replace("\r", "\\r")
            .replace("\t", "\\t")
        )
        + "'"
    )
    return single_quoted if len(single_quoted) < len(double_quoted) else double_quoted


def render_rust(catalogue_json: str, fingerprint: str) -> str:
    lines = [
        "// Generated by scripts/ci/check_generated_surfaces.py. Do not edit.",
        "// Source: crates/kernel/rrd-core/fixtures/telemetry-catalogue-v1.json",
        "",
        "pub(super) const SIGNAL_CATALOGUE_SHA256: &str =",
        f'    "{fingerprint}";',
        "pub(super) const SIGNAL_CATALOGUE_JSON: &str = concat!(",
    ]
    lines.extend(f"    {quoted(part)}," for part in chunks(catalogue_json, 60))
    lines.extend([");", ""])
    return "\n".join(lines)


def render_typescript(
    catalogue_json: str, fingerprint: str, openapi_sha256: str
) -> str:
    lines = [
        "// Generated by scripts/ci/check_generated_surfaces.py. Do not edit.",
        f"// OpenAPI SHA-256: {openapi_sha256}",
        "",
        f'export const SIGNAL_CATALOGUE_SHA256 = "{fingerprint}" as const;',
        f'export const OPENAPI_DOCUMENT_SHA256 = "{openapi_sha256}" as const;',
        "export const SIGNAL_CATALOGUE_JSON =",
    ]
    parts = chunks(catalogue_json)
    lines.extend(
        f"  {quoted(part)}{' +' if index < len(parts) - 1 else ';'}"
        for index, part in enumerate(parts)
    )
    lines.extend(
        [
            "",
            "const deepFreeze = (value: unknown): unknown => {",
            '  if (value !== null && typeof value === "object") {',
            "    for (const child of Object.values(value as Record<string, unknown>)) {",
            "      deepFreeze(child);",
            "    }",
            "    Object.freeze(value);",
            "  }",
            "  return value;",
            "};",
            "",
            "export const SIGNAL_CATALOGUE = deepFreeze(",
            "  JSON.parse(SIGNAL_CATALOGUE_JSON),",
            ") as Readonly<Record<string, unknown>>;",
            "",
        ]
    )
    return "\n".join(lines)


def render_python(catalogue_json: str, fingerprint: str, openapi_sha256: str) -> str:
    lines = [
        '"""Generated signal catalogue identity. Do not edit."""',
        "",
        f'SIGNAL_CATALOGUE_SHA256: str = "{fingerprint}"',
        f'OPENAPI_DOCUMENT_SHA256: str = "{openapi_sha256}"',
        "SIGNAL_CATALOGUE_JSON: str = (",
    ]
    lines.extend(f"    {python_quoted(part)}" for part in chunks(catalogue_json, 60))
    lines.extend([")", ""])
    return "\n".join(lines)


def render_go(catalogue_json: str, fingerprint: str, openapi_sha256: str) -> str:
    parts = chunks(catalogue_json, 60)
    lines = [
        "// Code generated by scripts/ci/check_generated_surfaces.py. DO NOT EDIT.",
        f"// OpenAPI SHA-256: {openapi_sha256}",
        "",
        "package rrd",
        "",
        "const (",
        f'\tSignalCatalogueSHA256 = "{fingerprint}"',
        f'\tOpenAPIDocumentSHA256 = "{openapi_sha256}"',
        '\tSignalCatalogueJSON   = "" +',
    ]
    lines.extend(
        f"\t\t{quoted(part)}{' +' if index < len(parts) - 1 else ''}"
        for index, part in enumerate(parts)
    )
    lines.extend([")", ""])
    return "\n".join(lines)


def render_java(catalogue_json: str, fingerprint: str, openapi_sha256: str) -> str:
    parts = chunks(catalogue_json, 60)
    lines = [
        "// Generated by scripts/ci/check_generated_surfaces.py. Do not edit.",
        f"// OpenAPI SHA-256: {openapi_sha256}",
        "package io.rrflow.rrd;",
        "",
        "/** Immutable identity projection of the canonical RRFlow signal catalogue. */",
        "public final class SignalCatalogue {",
        f'    public static final String SIGNAL_CATALOGUE_SHA256 = "{fingerprint}";',
        f'    public static final String OPENAPI_DOCUMENT_SHA256 = "{openapi_sha256}";',
        "    public static final String JSON =",
    ]
    lines.extend(
        f"            {quoted(part)}{' +' if index < len(parts) - 1 else ';'}"
        for index, part in enumerate(parts)
    )
    lines.extend(
        [
            "",
            "    private SignalCatalogue() {}",
            "}",
            "",
        ]
    )
    return "\n".join(lines)


def render_dotnet(catalogue_json: str, fingerprint: str, openapi_sha256: str) -> str:
    parts = chunks(catalogue_json, 60)
    lines = [
        "// <auto-generated />",
        "// Generated by scripts/ci/check_generated_surfaces.py. Do not edit.",
        f"// OpenAPI SHA-256: {openapi_sha256}",
        "namespace Rrflow.Rrd;",
        "",
        (
            "/// <summary>Immutable identity projection of the canonical RRFlow "
            "signal catalogue.</summary>"
        ),
        "public static class SignalCatalogue",
        "{",
        f'    public const string SignalCatalogueSha256 = "{fingerprint}";',
        f'    public const string OpenApiDocumentSha256 = "{openapi_sha256}";',
        "    public const string Json =",
    ]
    lines.extend(
        f"        {quoted(part)}{' +' if index < len(parts) - 1 else ';'}"
        for index, part in enumerate(parts)
    )
    lines.extend(["}", ""])
    return "\n".join(lines)


def markdown_code(value: object) -> str:
    return f"`{str(value).replace('`', '&#96;')}`"


def markdown_text(value: object) -> str:
    return str(value).replace("|", "\\|").replace("\n", " ")


def render_signal_reference(
    catalogue: dict[str, Any], fingerprint: str, openapi_sha256: str
) -> str:
    lines = [
        "# RRFlow signal catalogue",
        "",
        (
            "**Status:** active generated signal-contract reference; H-05 runtime "
            "instrumentation remains incomplete"
        ),
        "**Coordinate:** `rrflow://rrflow-instance/data/reference/protocol/signal-catalogue`",
        (
            "**Owner:** generated discovery projection of the kernel signal catalogue; "
            "linked from `docs/reference/protocol/README.md`"
        ),
        "",
        "Generated by `scripts/ci/check_generated_surfaces.py` from the kernel's",
        "test-verified fixture and canonical `rrd-contract-export` OpenAPI document.",
        "Do not edit this projection. The",
        "[kernel catalogue](../../../crates/kernel/rrd-core/src/telemetry.rs) owns",
        "signal semantics; the [public contract](public-contract.md) owns this",
        "discovery projection. Neither this record nor an SDK activates telemetry or",
        "becomes lifecycle, planning, status, or evidence authority.",
        "",
        "## Identity",
        "",
        "| Field | Value |",
        "|---|---|",
        f"| Signal contract version | {markdown_code(catalogue['contract_version'])} |",
        (
            "| Runtime trace contract version | "
            f"{markdown_code(catalogue['runtime_trace_contract_version'])} |"
        ),
        f"| Signal catalogue SHA-256 | {markdown_code(fingerprint)} |",
        f"| OpenAPI document SHA-256 | {markdown_code(openapi_sha256)} |",
        f"| Default diagnostic level | {markdown_code(catalogue['default_diagnostic_level'])} |",
        (
            "| Default metric cardinality limit | "
            f"{markdown_code(catalogue['default_metric_cardinality_limit'])} "
            "points per collection cycle |"
        ),
        "",
        "## Diagnostic levels",
        "",
        "| Order | Level | Default |",
        "|---:|---|---|",
    ]
    for index, level in enumerate(catalogue["diagnostic_levels"], start=1):
        default = "yes" if level == catalogue["default_diagnostic_level"] else "no"
        lines.append(f"| {index} | {markdown_code(level)} | {default} |")
    lines.extend(
        [
            "",
            "## Permitted metric dimensions",
            "",
            "These are descriptor names only. Runtime values remain bounded by each",
            "instrumenting site and cannot introduce arbitrary labels.",
            "",
            "| Order | Dimension |",
            "|---:|---|",
        ]
    )
    for index, attribute in enumerate(catalogue["metric_attributes"], start=1):
        lines.append(f"| {index} | {markdown_code(attribute)} |")
    lines.extend(
        [
            "",
            "## Trace operations",
            "",
            "| Order | Operation | Boundary |",
            "|---:|---|---|",
        ]
    )
    for index, operation in enumerate(catalogue["trace_operations"], start=1):
        lines.append(
            f"| {index} | {markdown_code(operation['name'])} | "
            f"{markdown_code(operation['boundary'])} |"
        )
    lines.extend(["", "## Trace attributes", "", "| Order | Attribute |", "|---:|---|"])
    for index, attribute in enumerate(catalogue["trace_attributes"], start=1):
        lines.append(f"| {index} | {markdown_code(attribute['name'])} |")
    lines.extend(
        [
            "",
            "## Metric instruments",
            "",
            "| Order | Instrument | Kind | Unit | Permitted dimensions | Description |",
            "|---:|---|---|---|---|---|",
        ]
    )
    for index, metric in enumerate(catalogue["metric_instruments"], start=1):
        attributes = ", ".join(markdown_code(value) for value in metric["attributes"])
        lines.append(
            f"| {index} | {markdown_code(metric['instrument'])} | "
            f"{markdown_code(metric['kind'])} | {markdown_code(metric['unit'])} | "
            f"{attributes} | {markdown_text(metric['description'])} |"
        )
    lines.extend(
        [
            "",
            "## Evidence boundary",
            "",
            "Generation proves that the private contract bridge, OpenAPI, this reference,",
            "and the five checked-in SDK identities agree with one kernel catalogue.",
            "That does not prove runtime collection, propagation, export,",
            "diagnostic-level parity, overhead",
            "budgets, build identity, engine behavior, benchmark promotion, or release",
            "readiness. Those remain owned by the H-05 and J acceptance records.",
            "",
        ]
    )
    return "\n".join(lines)


def rendered_signal_outputs(
    catalogue: dict[str, Any], fingerprint: str, openapi_sha256: str
) -> dict[str, str]:
    catalogue_json = json.dumps(
        catalogue, ensure_ascii=True, separators=(",", ":"), sort_keys=True
    )
    return {
        "signal reference": render_signal_reference(
            catalogue, fingerprint, openapi_sha256
        ),
        "typescript signal catalogue": render_typescript(
            catalogue_json, fingerprint, openapi_sha256
        ),
        "python signal catalogue": render_python(
            catalogue_json, fingerprint, openapi_sha256
        ),
        "go signal catalogue": render_go(catalogue_json, fingerprint, openapi_sha256),
        "java signal catalogue": render_java(
            catalogue_json, fingerprint, openapi_sha256
        ),
        "dotnet signal catalogue": render_dotnet(
            catalogue_json, fingerprint, openapi_sha256
        ),
    }


def apply_contract_signal_output(expected: str, *, write: bool) -> None:
    expected_bytes = expected.encode("utf-8")
    if write:
        CONTRACT_SIGNAL_OUTPUT.parent.mkdir(parents=True, exist_ok=True)
        CONTRACT_SIGNAL_OUTPUT.write_bytes(expected_bytes)
        return
    require(
        CONTRACT_SIGNAL_OUTPUT.is_file(),
        f"contract signal projection is absent: {CONTRACT_SIGNAL_OUTPUT.relative_to(ROOT)}",
    )
    require(
        CONTRACT_SIGNAL_OUTPUT.read_bytes() == expected_bytes,
        f"contract signal projection is stale: {CONTRACT_SIGNAL_OUTPUT.relative_to(ROOT)}",
    )


def apply_signal_outputs(outputs: dict[str, str], *, write: bool) -> None:
    require(
        set(outputs) == set(SIGNAL_OUTPUTS),
        "internal signal renderer output set drifted",
    )
    for name, expected in outputs.items():
        path = SIGNAL_OUTPUTS[name]
        expected_bytes = expected.encode("utf-8")
        if write:
            require(
                path.parent.is_dir(), f"signal output parent is absent: {path.parent}"
            )
            path.write_bytes(expected_bytes)
            continue
        require(path.is_file(), f"{name} is absent: {path.relative_to(ROOT)}")
        actual = path.read_bytes()
        require(actual == expected_bytes, f"{name} is stale: {path.relative_to(ROOT)}")


def arguments() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    mode = parser.add_mutually_exclusive_group()
    mode.add_argument(
        "--check", action="store_true", help="check without writing (default)"
    )
    mode.add_argument(
        "--write", action="store_true", help="rewrite declared signal projections"
    )
    return parser.parse_args()


def main() -> None:
    args = arguments()
    kernel_catalogue, kernel_fingerprint = kernel_signal_projection()
    kernel_catalogue_json = json.dumps(
        kernel_catalogue, ensure_ascii=True, separators=(",", ":"), sort_keys=True
    )
    apply_contract_signal_output(
        render_rust(kernel_catalogue_json, kernel_fingerprint), write=args.write
    )
    raw, document = contract_export()
    digest = hashlib.sha256(raw).hexdigest()
    operation_count = validate_endpoint_outputs(
        document, digest, require_current_digest=not args.write
    )
    if not args.write:
        validate_digest_citations(digest)
    catalogue, fingerprint = signal_projection(document)
    require(
        catalogue == kernel_catalogue,
        "OpenAPI signal catalogue drifted from the kernel fixture projection",
    )
    require(
        fingerprint == kernel_fingerprint,
        "OpenAPI signal fingerprint drifted from the kernel fixture projection",
    )
    outputs = rendered_signal_outputs(catalogue, fingerprint, digest)
    apply_signal_outputs(outputs, write=args.write)
    mode = "rendered" if args.write else "verified"
    print(
        f"generated surface parity: {operation_count} HTTP operations; "
        f"signal catalogue {fingerprint}; OpenAPI {digest}; "
        f"{mode} {len(outputs) + 1} signal projections"
    )


if __name__ == "__main__":
    try:
        main()
    except SurfaceError as error:
        raise SystemExit(f"generated surfaces: {error}") from error
