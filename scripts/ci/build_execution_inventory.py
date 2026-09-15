#!/usr/bin/env python3
"""Build or verify the exhaustive RRFlow 1.0 file execution inventory."""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUTPUT = ROOT / "docs" / "roadmap" / "rrflow-1.0-file-plan.jsonl"
ROADMAP = ROOT / "docs" / "roadmap" / "rrflow-1.0.md"
ACTIVE_CHANGE_PLAN = "docs/roadmap/rrflow-1.0-active-change.json"

PACKAGE_GATES: dict[str, tuple[str, ...]] = {
    "rrd-core": ("A-07", "C-01", "C-02", "C-03", "H-05", "I-01"),
    "rrd-lsm": ("A-07", "C-02", "C-06", "C-07", "D-09", "J-02", "J-04"),
    "rrd-store": (
        "A-07",
        "C-01",
        "C-02",
        "C-03",
        "C-04",
        "C-05",
        "E-01",
        "E-02",
        "E-03",
        "E-04",
        "F-01",
        "F-02",
    ),
    "rrd-query": (
        "A-07",
        "B-05",
        "E-01",
        "E-02",
        "E-03",
        "E-05",
        "F-01",
        "F-02",
        "F-03",
        "F-04",
        "F-05",
        "H-02",
        "H-03",
    ),
    "rrd-vector": (
        "A-07",
        "D-05",
        "E-04",
        "E-05",
        "F-03",
        "H-01",
        "J-02",
        "J-04",
    ),
    "rrd-inference": (
        "A-07",
        "B-03",
        "D-05",
        "E-04",
        "G-01",
        "G-03",
        "G-06",
        "H-01",
        "J-04",
    ),
    "rrd-security": ("A-07", "D-01", "H-04", "H-05", "J-02"),
    "rrd-estate": ("A-07", "D-01", "D-02", "D-10", "H-05", "J-03"),
    "rrd-engine": (
        "A-07",
        "C-03",
        "D-01",
        "D-02",
        "D-05",
        "E-04",
        "F-04",
        "F-05",
        "G-01",
        "G-02",
        "G-04",
        "G-05",
        "H-01",
        "H-02",
        "H-03",
        "H-05",
        "I-02",
        "I-03",
        "I-05",
        "I-07",
    ),
    "rrd-contract": (
        "A-07",
        "B-03",
        "B-04",
        "B-05",
        "D-01",
        "D-02",
        "D-06",
        "G-01",
        "H-04",
        "I-01",
        "I-03",
        "I-05",
    ),
    "rrd-client": ("A-07", "B-04", "H-04", "H-07", "J-03"),
    "rrd-server": ("A-07", "B-04", "B-05", "H-03", "H-04", "H-05", "J-03"),
    "rrflow-cli": ("A-07", "C-05", "D-01", "I-06", "J-01", "J-03"),
    "rrflow-mcp": ("A-07", "H-04", "H-05", "I-04", "J-03"),
    "rrflow-edge": ("A-07", "D-07", "H-04", "H-07", "J-03", "J-05"),
    "rrflow-local-process": (
        "A-07",
        "C-03",
        "D-01",
        "D-02",
        "H-04",
        "H-05",
        "I-03",
        "J-01",
        "J-02",
        "J-03",
        "J-05",
    ),
    "rrd-cluster": ("A-07", "D-10", "H-05", "J-02", "J-04"),
    "rrd-kubernetes": (
        "A-07",
        "C-02",
        "C-03",
        "D-01",
        "D-02",
        "H-04",
        "H-05",
        "H-07",
        "J-01",
        "J-02",
        "J-03",
        "J-05",
    ),
    "rrflow-kubernetes": (
        "A-07",
        "C-02",
        "C-03",
        "D-01",
        "D-02",
        "E-01",
        "E-02",
        "E-03",
        "E-04",
        "E-05",
        "F-01",
        "F-02",
        "F-03",
        "F-04",
        "F-05",
        "G-04",
        "G-05",
        "H-01",
        "H-02",
        "H-04",
        "H-05",
        "H-07",
        "J-01",
        "J-02",
        "J-03",
        "J-05",
    ),
    "rrd-maintenance": ("A-07", "D-02", "D-10", "I-07", "J-02"),
    "rrd-operator-knowledge": ("A-07", "D-06", "H-01", "J-02"),
    "rrflow-eval": ("A-07", "G-06", "J-02", "J-04"),
}

FILE_OVERRIDES: dict[str, tuple[str, ...]] = {
    "Cargo.toml": ("H-05", "J-01", "J-02", "J-03", "J-04", "J-05"),
    "crates/kernel/rrd-core/src/claim.rs": (
        "C-03",
        "G-02",
        "G-04",
        "H-04",
        "H-05",
        "J-01",
    ),
    "crates/kernel/rrd-core/src/runtime.rs": (
        "C-02",
        "C-03",
        "H-05",
        "I-01",
        "J-01",
    ),
    "crates/kernel/rrd-core/src/reasoning_tree.rs": (
        "B-03",
        "G-01",
        "G-02",
        "G-04",
        "H-01",
        "H-05",
    ),
    "crates/kernel/rrd-core/src/trace.rs": ("C-04", "H-05"),
    "crates/kernel/rrd-core/tests/golden.rs": ("C-01", "J-01"),
    "crates/persistence/rrd-lsm/src/database.rs": (
        "C-02",
        "C-04",
        "C-06",
        "C-07",
        "D-01",
        "F-05",
        "J-04",
    ),
    "crates/persistence/rrd-lsm/src/manifest.rs": ("C-05", "C-06", "C-07", "D-01"),
    "crates/persistence/rrd-lsm/src/memtable.rs": (
        "C-02",
        "C-04",
        "C-06",
        "C-07",
        "F-01",
    ),
    "crates/persistence/rrd-lsm/src/segment/mod.rs": (
        "C-04",
        "C-05",
        "C-06",
        "C-07",
        "F-01",
        "F-02",
        "F-05",
        "J-04",
    ),
    "crates/persistence/rrd-lsm/src/segment/format.rs": (
        "C-05",
        "C-06",
        "C-07",
        "F-01",
        "F-02",
        "J-04",
    ),
    "crates/persistence/rrd-lsm/src/wal.rs": ("C-02", "C-05", "C-07", "J-04"),
    "crates/persistence/rrd-lsm/src/write_diagnostics.rs": ("C-07", "J-04"),
    "crates/persistence/rrd-store/src/engine.rs": ("C-02", "C-03", "C-04"),
    "crates/persistence/rrd-store/src/keyspaces.rs": (
        "C-01",
        "C-03",
        "C-04",
        "C-05",
        "E-01",
        "E-02",
        "E-03",
        "E-04",
    ),
    "crates/persistence/rrd-store/src/rrflow_kv.rs": (
        "C-01",
        "C-02",
        "C-03",
        "C-04",
        "C-05",
        "C-07",
        "D-01",
        "J-04",
    ),
    "crates/persistence/rrd-store/src/outcome.rs": ("C-02", "C-03"),
    "crates/persistence/rrd-store/src/control.rs": ("C-03", "H-05"),
    "crates/persistence/rrd-store/examples/engine_benchmark.rs": (
        "C-02",
        "C-04",
        "C-06",
        "C-07",
        "J-04",
    ),
    "crates/persistence/rrd-store/examples/ai_hotset_benchmark.rs": (
        "C-04",
        "C-06",
        "F-05",
        "J-04",
    ),
    "crates/persistence/rrd-store/tests/rrflow_kv_model_soak.rs": (
        "C-02",
        "C-03",
        "C-05",
    ),
    "crates/persistence/rrd-store/tests/rrflow_kv_open.rs": ("C-05",),
    "crates/persistence/rrd-store/tests/rrflow_kv_operator.rs": (
        "C-03",
        "C-04",
        "C-07",
        "J-04",
    ),
    "crates/compute/rrd-query/src/arrow.rs": ("F-01", "F-02"),
    "crates/compute/rrd-query/src/execute.rs": (
        "E-01",
        "E-03",
        "F-01",
        "F-02",
        "F-03",
        "F-04",
    ),
    "crates/compute/rrd-query/src/live.rs": ("H-03",),
    "crates/compute/rrd-query/src/pipeline.rs": ("B-05", "F-01", "F-04"),
    "crates/compute/rrd-query/src/plan.rs": ("B-05", "E-05", "F-02", "F-03"),
    "crates/compute/rrd-query/src/bm25.rs": ("E-03", "F-03", "H-01"),
    "crates/compute/rrd-query/src/index.rs": ("E-02", "E-03", "E-05"),
    "crates/compute/rrd-query/src/syntax.rs": ("B-05", "F-03"),
    "crates/compute/rrd-vector/src/exact.rs": ("E-04", "J-04"),
    "crates/compute/rrd-vector/src/hnsw.rs": ("E-04", "E-05", "J-04"),
    "crates/compute/rrd-vector/src/plan.rs": ("E-04", "E-05"),
    "crates/compute/rrd-vector/src/runtime.rs": ("E-04", "E-05", "F-03"),
    "crates/compute/rrd-inference/src/lib.rs": (
        "B-03",
        "D-05",
        "E-04",
        "G-01",
        "H-01",
    ),
    "crates/compute/rrd-inference/src/fastembed_local.rs": (
        "D-05",
        "E-04",
        "H-01",
        "J-04",
    ),
    "crates/compute/rrd-vector/src/contract.rs": ("D-05", "E-04", "J-04"),
    "crates/compute/rrd-vector/src/catalog.rs": ("E-04", "E-05", "H-01"),
    "crates/compute/rrd-vector/src/compact.rs": ("E-04", "F-03", "J-04"),
    "crates/compute/rrd-vector/src/accelerator.rs": ("E-04", "J-04"),
    "crates/compute/rrd-vector/src/turboquant.rs": ("E-04", "J-01", "J-04"),
    "crates/compute/rrd-vector/src/turbo_segment.rs": ("E-04", "J-01", "J-04"),
    "crates/transport/rrd-contract/src/function.rs": (
        "A-07",
        "C-03",
        "D-01",
        "H-04",
        "I-01",
        "I-02",
        "I-06",
        "J-01",
        "J-02",
    ),
    "crates/transport/rrd-contract/src/read.rs": ("C-04", "F-04", "H-04", "H-05"),
    "crates/authority/rrd-engine/src/runtime/read_evidence.rs": (
        "C-04",
        "F-04",
        "H-05",
    ),
    "crates/authority/rrd-engine/src/engine/context.rs": (
        "G-02",
        "G-05",
        "H-01",
        "H-02",
    ),
    "crates/authority/rrd-engine/src/engine/memory_estate.rs": (
        "C-03",
        "D-01",
        "E-01",
        "E-02",
        "G-02",
        "H-01",
        "H-04",
        "H-05",
        "J-01",
    ),
    "crates/authority/rrd-engine/src/engine/tests/memory_estate.rs": (
        "C-03",
        "D-01",
        "G-02",
        "G-04",
        "H-01",
        "H-04",
        "H-05",
        "J-01",
    ),
    "crates/authority/rrd-engine/src/engine/retrieval.rs": ("F-03", "H-01", "H-02"),
    "crates/authority/rrd-engine/src/engine/retrieval_query.rs": (
        "E-05",
        "F-03",
        "H-01",
        "H-02",
    ),
    "crates/authority/rrd-engine/src/engine/query.rs": ("B-05", "F-04", "H-04"),
    "crates/authority/rrd-engine/src/engine/query_transaction.rs": (
        "C-02",
        "C-03",
        "G-04",
    ),
    "crates/authority/rrd-engine/src/engine/transaction.rs": (
        "C-02",
        "C-03",
        "G-04",
        "H-05",
        "J-02",
    ),
    "crates/authority/rrd-engine/src/engine/session.rs": ("H-04", "H-05", "J-02"),
    "crates/authority/rrd-engine/src/engine/security.rs": ("H-04", "H-05", "J-02"),
    "crates/authority/rrd-engine/src/engine/security_bootstrap.rs": (
        "C-02",
        "C-03",
        "D-01",
        "D-02",
        "H-05",
        "J-01",
        "J-02",
        "J-03",
        "J-05",
    ),
    "crates/authority/rrd-engine/src/engine/invocation.rs": (
        "H-04",
        "H-05",
        "J-02",
    ),
    "crates/authority/rrd-engine/src/operator.rs": (
        "C-03",
        "G-02",
        "G-04",
        "H-04",
        "H-05",
        "J-01",
    ),
    "crates/authority/rrd-engine/src/engine/inference.rs": (
        "D-05",
        "E-04",
        "H-01",
    ),
    "crates/authority/rrd-engine/src/engine/vector.rs": (
        "D-05",
        "E-04",
        "H-01",
    ),
    "crates/authority/rrd-engine/src/edge.rs": ("H-04", "H-07", "J-03", "J-05"),
    "crates/authority/rrd-engine/src/engine/subscription.rs": ("B-04", "H-03", "H-04"),
    "crates/transport/rrd-server/src/http/websocket.rs": ("B-04", "H-03", "H-04"),
    "crates/authority/rrd-security/src/lib.rs": (
        "C-02",
        "C-03",
        "D-01",
        "H-04",
        "H-05",
        "J-01",
        "J-02",
    ),
    "crates/authority/rrd-security/tests/security_authority.rs": (
        "C-02",
        "C-03",
        "D-01",
        "H-04",
        "H-05",
        "J-02",
    ),
    "crates/adapters/rrflow-cli/src/bin/rrd-security-bootstrap.rs": (
        "D-01",
        "J-01",
        "J-03",
    ),
    "crates/adapters/rrflow-cli/tests/security_bootstrap.rs": (
        "D-01",
        "D-02",
        "J-01",
        "J-02",
        "J-03",
    ),
    "crates/adapters/rrflow-cli/src/dev/supervisor.rs": (
        "C-02",
        "C-03",
        "D-01",
        "D-02",
        "H-04",
        "H-05",
        "J-01",
        "J-02",
        "J-03",
        "J-05",
    ),
    "crates/adapters/rrflow-cli/src/command.rs": (
        "C-03",
        "D-01",
        "G-02",
        "H-01",
        "H-04",
        "H-05",
        "J-01",
    ),
    "crates/adapters/rrflow-cli/tests/operator_surface.rs": (
        "C-03",
        "D-01",
        "G-02",
        "G-04",
        "H-01",
        "H-04",
        "H-05",
        "J-01",
    ),
    "crates/transport/rrd-contract/src/attunement.rs": ("D-01", "D-02", "D-05"),
    "crates/transport/rrd-contract/src/memory_estate.rs": (
        "C-03",
        "D-01",
        "G-02",
        "H-01",
        "H-04",
        "J-01",
    ),
    "crates/transport/rrd-contract/src/router.rs": ("B-03", "G-01", "G-02", "G-03"),
    "crates/transport/rrd-contract/src/reasoning_tree.rs": (
        "B-03",
        "G-01",
        "G-02",
        "G-04",
        "H-01",
        "H-05",
    ),
    "crates/operations/rrd-cluster/src/node_runtime.rs": (
        "D-01",
        "G-02",
        "H-04",
        "H-05",
        "J-01",
        "J-02",
    ),
    "deploy/kubernetes/example-rrdinstance.json": (
        "A-07",
        "D-01",
        "J-03",
        "J-05",
    ),
    "deploy/kubernetes/operator-rbac.json": (
        "A-07",
        "D-01",
        "H-07",
        "J-02",
        "J-03",
        "J-05",
    ),
    "deploy/kubernetes/rrdinstances.rrflow.io-crd.json": (
        "A-07",
        "B-04",
        "D-01",
        "J-03",
        "J-05",
    ),
    "fixtures/rrd-sdk-conformance-v1.json": ("C-04", "H-04", "J-02"),
    "sdks/dotnet/Rrflow.Rrd.slnx": ("J-02", "J-03", "J-05"),
    "sdks/dotnet/scripts/generate.py": ("H-04", "J-02", "J-05"),
    "sdks/dotnet/src/Rrflow.Rrd.Client/Errors.cs": ("H-04", "J-02"),
    "sdks/dotnet/src/Rrflow.Rrd.Client/RrdClient.cs": (
        "B-04",
        "H-04",
        "H-05",
        "H-07",
        "J-02",
        "J-03",
        "J-05",
    ),
    "sdks/dotnet/src/Rrflow.Rrd.Client/Rrflow.Rrd.Client.csproj": (
        "H-04",
        "J-02",
        "J-03",
        "J-05",
    ),
    "sdks/dotnet/src/Rrflow.Rrd.Client/packages.lock.json": ("J-03", "J-05"),
    "sdks/dotnet/tests/Rrflow.Rrd.Client.Tests/RrdClientTests.cs": (
        "B-04",
        "H-04",
        "H-07",
        "J-02",
    ),
    "sdks/dotnet/tests/Rrflow.Rrd.Client.Tests/SdkConformanceTests.cs": (
        "H-04",
        "J-02",
        "J-05",
    ),
    "sdks/dotnet/tests/Rrflow.Rrd.Client.Tests/Rrflow.Rrd.Client.Tests.csproj": (
        "H-04",
        "J-02",
        "J-05",
    ),
    "sdks/dotnet/tests/Rrflow.Rrd.Client.Tests/packages.lock.json": (
        "J-03",
        "J-05",
    ),
    "sdks/java/src/main/java/io/rrflow/rrd/RrdClient.java": (
        "H-04",
        "J-02",
        "J-05",
    ),
    "sdks/java/src/test/java/io/rrflow/rrd/RrdClientTest.java": (
        "H-04",
        "J-02",
        "J-05",
    ),
}

REMOVE_OR_REWRITE = {
    "crates/kernel/rrd-core/src/claim.rs": "derive any producer attribution from the authenticated principal and same-stamp provider representation; an arbitrary actor string cannot authorize, resolve a seat, or persist routed work",
    "crates/transport/rrd-contract/src/memory_estate.rs": "rename the ambiguous MemoryEstate contract directly to canonical seat and provider-representation vocabulary while preserving validation, subject-digest redaction, and temporal relation semantics; leave no alias",
    "crates/authority/rrd-engine/src/engine/memory_estate.rs": "move seat planning and commit into one authenticated engine operation, consume the installed specialization, and replace broad snapshot reconstruction with canonical native graph/index access; leave no old module or type alias",
    "crates/authority/rrd-engine/src/operator.rs": "bind claim attribution to canonical authenticated identity and representation semantics rather than accepting Producer.actor as sufficient identity",
    "crates/adapters/rrflow-cli/src/command.rs": "remove hardcoded Clyffy identity defaults and caller-selected actor attribution; consume the installed specialization and public authenticated engine operations",
    "crates/authority/rrd-engine/src/engine/security_bootstrap.rs": "absorb strict validation, bounded secret input, atomic initial policy/audit, exact replay, and drift denial into the D-01 install authority; remove the static rrflowKV/path/time opener and manifest dialect",
    "crates/adapters/rrflow-cli/src/bin/rrd-security-bootstrap.rs": "delete after the sole rrflow install preview/apply path performs engine-owned cold-start security; leave no forwarding binary",
    "crates/adapters/rrflow-cli/tests/security_bootstrap.rs": "move useful validation, redaction, replay, drift, and reopen cases into install conformance plus engine crash/secret matrices; reject the old command and manifest shape",
    "crates/adapters/rrflow-cli/src/dev/supervisor.rs": "preserve bounded process safety in rrflow-local-process and move security initialization to D-01; remove private supervisor/bootstrap state and direct companion-binary orchestration",
    "crates/authority/rrd-security/src/lib.rs": "split pure identity, typed verifier, policy, and audit semantics from the direct StorageEngine repository; only RrdEngine may persist or mutate security state",
    "crates/operations/rrd-kubernetes/Cargo.toml": "replace with the planned outward rrflow-kubernetes package manifest during A-07; leave no forwarding package",
    "crates/operations/rrd-kubernetes/src/lib.rs": "split useful closed API and deterministic rendering behavior into planned rrflow-kubernetes seams over engine-issued plans; remove the direct desired-state and bootstrap authority",
    "crates/operations/rrd-kubernetes/src/controller.rs": "replace with engine-fenced reconciliation, standard observations, authenticated readiness, explicit field conflicts, and receipt submission in the outward adapter",
    "crates/operations/rrd-kubernetes/src/main.rs": "move controller process composition to rrflow-kubernetes after the public plan/client boundary exists",
    "crates/operations/rrd-kubernetes/src/bin/rrd-kubernetes-crd.rs": "replace with the rrflow-kubernetes CRD generator for the sole current projection schema",
    "crates/operations/rrd-kubernetes/tests/contract.rs": "preserve useful validation/rendering cases in the new contract corpus and replace successful old-authority assertions",
    "crates/operations/rrd-kubernetes/tests/crd.rs": "preserve deterministic schema assertions and replace string-only qualification with API-server and security evidence",
    "deploy/kubernetes/example-rrdinstance.json": "regenerate from the accepted install/effect projection and a verified release candidate; remove the placeholder bootstrap shape",
    "deploy/kubernetes/operator-rbac.json": "regenerate as a namespace-scoped least-privilege default with explicit client selection and negative Secret evidence",
    "deploy/kubernetes/rrdinstances.rrflow.io-crd.json": "regenerate from the sole rrflow-kubernetes projection schema with standard conditions and no earlier-shape reader",
    "crates/compute/rrd-query/src/execute.rs": "decompose eager loading into native access operators and a streaming DataFusion execution boundary",
    "crates/compute/rrd-query/src/arrow.rs": "replace Vec<QueryRow>-to-Arrow snapshot materialization with stamped page/batch adapters",
    "crates/compute/rrd-query/src/live.rs": "replace two-snapshot diffing with commit-impact evaluation",
}

C04_DIRECT_READ_PATHS = {
    "crates/adapters/rrflow-cli/src/command.rs",
    "crates/adapters/rrflow-mcp/src/authority.rs",
    "crates/adapters/rrflow-mcp/src/main.rs",
    "crates/adapters/rrflow-mcp/tests/stdio.rs",
    "crates/authority/rrd-engine/src/engine/context.rs",
    "crates/authority/rrd-engine/src/engine/data.rs",
    "crates/authority/rrd-engine/src/engine/inference.rs",
    "crates/authority/rrd-engine/src/engine/memory_estate.rs",
    "crates/authority/rrd-engine/src/engine/query.rs",
    "crates/authority/rrd-engine/src/engine/retrieval.rs",
    "crates/authority/rrd-engine/src/engine/retrieval_query.rs",
    "crates/authority/rrd-engine/src/engine/tests/context.rs",
    "crates/authority/rrd-engine/src/engine/tests/data_crud.rs",
    "crates/authority/rrd-engine/src/engine/tests/function.rs",
    "crates/authority/rrd-engine/src/engine/tests/memory_estate.rs",
    "crates/authority/rrd-engine/src/engine/tests/native_inference.rs",
    "crates/authority/rrd-engine/src/engine/tests/query_transaction.rs",
    "crates/authority/rrd-engine/src/engine/tests/recovery.rs",
    "crates/authority/rrd-engine/src/engine/tests/security.rs",
    "crates/authority/rrd-engine/src/engine/tests/vector_index.rs",
    "crates/authority/rrd-engine/src/engine/transaction.rs",
    "crates/authority/rrd-engine/src/engine/vector/collection.rs",
    "crates/authority/rrd-engine/src/engine/vector/index.rs",
    "crates/authority/rrd-engine/src/engine/vector/points.rs",
    "crates/authority/rrd-engine/src/engine/vector/quantization.rs",
    "crates/authority/rrd-engine/src/engine/vector/search.rs",
    "crates/authority/rrd-engine/src/runtime/data_plane.rs",
    "crates/authority/rrd-engine/src/runtime/operator_knowledge.rs",
    "crates/authority/rrd-engine/src/runtime/query.rs",
    "crates/authority/rrd-engine/tests/direct_read_source_closure.rs",
    "crates/authority/rrd-engine/tests/engine_authority.rs",
    "crates/authority/rrd-engine/tests/runtime_query_trace.rs",
    "crates/kernel/rrd-core/src/trace.rs",
    "crates/transport/rrd-client/tests/real_server.rs",
    "crates/transport/rrd-contract/fixtures/router-contract-v1.json",
    "crates/transport/rrd-contract/fixtures/websocket-protocol-v1.json",
    "crates/transport/rrd-contract/src/inference.rs",
    "crates/transport/rrd-contract/src/lib.rs",
    "crates/transport/rrd-contract/src/memory_context.rs",
    "crates/transport/rrd-contract/src/memory_estate.rs",
    "crates/transport/rrd-contract/src/router.rs",
    "crates/transport/rrd-contract/tests/public_contract.rs",
    "crates/transport/rrd-contract/tests/router_contract.rs",
    "crates/transport/rrd-server/tests/http_process.rs",
    "fixtures/rrd-sdk-conformance-v1.json",
    "sdks/dotnet/src/Rrflow.Rrd.Client/Generated/OperationId.g.cs",
    "sdks/dotnet/tests/Rrflow.Rrd.Client.Tests/RrdClientTests.cs",
    "sdks/go/client_test.go",
    "sdks/go/endpoints_gen.go",
    "sdks/java/src/main/java/io/rrflow/rrd/OperationId.java",
    "sdks/java/src/test/java/io/rrflow/rrd/RrdClientTest.java",
    "sdks/python/src/rrd_client/generated/endpoints.py",
    "sdks/python/tests/test_client.py",
    "sdks/typescript/src/generated/endpoints.ts",
    "sdks/typescript/src/generated/rrd-openapi.ts",
    "sdks/typescript/tests/client.test.ts",
}

H05_OBSERVABILITY_PATHS = {
    "crates/adapters/rrflow-cli/Cargo.toml",
    "crates/adapters/rrflow-cli/src/command.rs",
    "crates/adapters/rrflow-cli/src/main.rs",
    "crates/compute/rrd-query/src/fusion.rs",
    "crates/evaluation/rrflow-eval/Cargo.toml",
    "crates/evaluation/rrflow-eval/src/main.rs",
    "crates/transport/rrd-client/Cargo.toml",
    "crates/transport/rrd-client/src/client.rs",
    "crates/transport/rrd-client/src/lib.rs",
    "crates/transport/rrd-client/src/operation.rs",
    "crates/transport/rrd-client/src/transport.rs",
    "crates/transport/rrd-client/tests/real_server.rs",
    "crates/transport/rrd-contract/Cargo.toml",
    "crates/transport/rrd-contract/src/diagnostic.rs",
    "crates/transport/rrd-contract/src/generated/signal_catalogue.rs",
    "crates/transport/rrd-contract/src/lib.rs",
    "crates/transport/rrd-contract/tests/public_contract.rs",
    "crates/transport/rrd-contract/tests/signal_catalogue_projection.rs",
    "docs/reference/protocol/public-contract.md",
    "docs/reference/protocol/signal-catalogue.md",
    "docs/roadmap/rrflow-1.0-execution/generated-surfaces.md",
    "scripts/ci/build_execution_inventory.py",
    "scripts/ci/check_generated_surfaces.py",
    "sdks/dotnet/src/Rrflow.Rrd.Client/Generated/OperationId.g.cs",
    "sdks/dotnet/src/Rrflow.Rrd.Client/Generated/SignalCatalogue.g.cs",
    "sdks/dotnet/tests/Rrflow.Rrd.Client.Tests/SignalCatalogueTests.cs",
    "sdks/go/endpoints_gen.go",
    "sdks/go/signal_catalogue_gen.go",
    "sdks/go/signal_catalogue_test.go",
    "sdks/java/src/main/java/io/rrflow/rrd/OperationId.java",
    "sdks/java/src/main/java/io/rrflow/rrd/SignalCatalogue.java",
    "sdks/java/src/test/java/io/rrflow/rrd/SignalCatalogueTest.java",
    "sdks/python/src/rrd_client/__init__.py",
    "sdks/python/src/rrd_client/generated/__init__.py",
    "sdks/python/src/rrd_client/generated/endpoints.py",
    "sdks/python/src/rrd_client/generated/signal_catalogue.py",
    "sdks/python/tests/test_signal_catalogue.py",
    "sdks/typescript/src/generated/endpoints.ts",
    "sdks/typescript/src/generated/rrd-openapi.ts",
    "sdks/typescript/src/generated/signal-catalogue.ts",
    "sdks/typescript/src/index.ts",
    "sdks/typescript/tests/signal-catalogue.test.ts",
}

GENERATED_PROJECTIONS = {
    "crates/transport/rrd-contract/src/generated/signal_catalogue.rs": "scripts/ci/check_generated_surfaces.py",
    "docs/reference/protocol/signal-catalogue.md": "scripts/ci/check_generated_surfaces.py",
    "sdks/dotnet/src/Rrflow.Rrd.Client/Generated/OperationId.g.cs": "sdks/dotnet/scripts/generate.py",
    "sdks/dotnet/src/Rrflow.Rrd.Client/Generated/SignalCatalogue.g.cs": "scripts/ci/check_generated_surfaces.py",
    "sdks/go/endpoints_gen.go": "sdks/go/cmd/generate/main.go",
    "sdks/go/signal_catalogue_gen.go": "scripts/ci/check_generated_surfaces.py",
    "sdks/java/src/main/java/io/rrflow/rrd/OperationId.java": "sdks/java/scripts/generate.py",
    "sdks/java/src/main/java/io/rrflow/rrd/SignalCatalogue.java": "scripts/ci/check_generated_surfaces.py",
    "sdks/python/src/rrd_client/generated/endpoints.py": "sdks/python/scripts/generate.py",
    "sdks/python/src/rrd_client/generated/signal_catalogue.py": "scripts/ci/check_generated_surfaces.py",
    "sdks/typescript/src/generated/endpoints.ts": "sdks/typescript/scripts/generate.ts",
    "sdks/typescript/src/generated/rrd-openapi.ts": "sdks/typescript/scripts/generate.ts",
    "sdks/typescript/src/generated/signal-catalogue.ts": "scripts/ci/check_generated_surfaces.py",
}

PLANNED_PATHS: dict[str, tuple[str, ...]] = {
    "scripts/ci/check_change_plan.py": ("J-01", "J-02"),
    "scripts/ci/test_change_plan.py": ("J-01", "J-02"),
    "crates/persistence/rrd-lsm/src/lib.rs": ("C-02", "C-07", "D-01"),
    "crates/persistence/rrd-lsm/tests/inspection.rs": (
        "C-07",
        "D-01",
        "D-11",
        "J-02",
    ),
    "crates/persistence/rrd-store/src/lib.rs": ("C-03", "D-01"),
    "crates/persistence/rrd-store/src/repository/control.rs": (
        "C-03",
        "D-01",
        "H-05",
    ),
    "crates/persistence/rrd-store/src/repository/runtime.rs": (
        "C-03",
        "D-01",
        "H-05",
    ),
    "crates/persistence/rrd-store/tests/installed_lifecycle.rs": (
        "C-03",
        "D-01",
        "J-02",
    ),
    "crates/authority/rrd-engine/assets/install/default-profile-v1.json": (
        "D-01",
        "J-03",
        "J-05",
    ),
    "crates/authority/rrd-engine/src/engine/installation.rs": (
        "D-01",
        "H-05",
        "J-01",
        "J-02",
        "J-03",
        "J-05",
    ),
    "crates/authority/rrd-engine/src/engine/tests/installation.rs": (
        "D-01",
        "J-02",
        "J-03",
    ),
    "crates/adapters/rrflow-cli/src/installed.rs": (
        "D-01",
        "H-04",
        "H-05",
        "J-01",
        "J-03",
    ),
    "docs/research/rrflow-installation-repair-and-distribution-research.md": (
        "D-01",
        "D-11",
        "J-01",
        "J-03",
        "J-05",
    ),
    "docs/evidence/change-journals/gate-d/d-01-installed-ui-walking-product.md": (
        "D-01",
        "J-01",
        "J-02",
    ),
    "docs/evidence/change-journals/gate-d/README.md": (
        "D-01",
        "KB-08",
        "J-01",
    ),
    "crates/kernel/rrd-core/src/telemetry.rs": ("H-05",),
    "crates/kernel/rrd-core/tests/telemetry_contract.rs": ("H-05", "J-02"),
    "crates/transport/rrd-contract/src/observability.rs": ("H-05",),
    "crates/transport/rrd-contract/fixtures/observability-v1.json": ("H-05",),
    "crates/transport/rrd-contract/tests/observability_contract.rs": (
        "H-05",
        "J-02",
    ),
    "crates/authority/rrd-engine/src/engine/observability/mod.rs": ("H-05",),
    "crates/authority/rrd-engine/src/engine/observability/trace.rs": ("H-05",),
    "crates/authority/rrd-engine/src/engine/observability/metrics.rs": ("H-05",),
    "crates/authority/rrd-engine/src/engine/observability/capture.rs": ("H-05",),
    "crates/authority/rrd-engine/tests/observability_conformance.rs": (
        "H-05",
        "J-02",
    ),
    "crates/transport/rrd-server/src/observability.rs": ("H-05",),
    "crates/transport/rrd-server/src/http/trace_context.rs": ("H-05",),
    "crates/transport/rrd-server/tests/observability_process.rs": (
        "H-05",
        "J-02",
    ),
    "crates/transport/rrd-client/src/trace_context.rs": ("H-05",),
    "crates/adapters/rrflow-cli/src/diagnostics.rs": ("D-01", "H-05", "J-03"),
    "crates/adapters/rrflow-cli/tests/diagnostic_identity.rs": (
        "D-01",
        "H-05",
        "J-02",
        "J-03",
    ),
    "crates/evaluation/rrflow-eval/src/latency.rs": ("H-05", "J-04"),
    "crates/evaluation/rrflow-eval/tests/latency_evidence.rs": (
        "H-05",
        "J-02",
        "J-04",
    ),
    "crates/transport/rrd-contract/src/read.rs": ("C-04", "F-04", "H-04", "H-05"),
    "crates/transport/rrd-contract/fixtures/read-evidence-v1.json": (
        "C-04",
        "H-04",
    ),
    "crates/transport/rrd-contract/tests/read_evidence_contract.rs": (
        "C-04",
        "H-04",
    ),
    "crates/authority/rrd-engine/src/runtime/read_evidence.rs": (
        "C-04",
        "F-04",
        "H-05",
    ),
    "crates/transport/rrd-contract/fixtures/knowledge-package-v1.json": ("KB-02",),
    "crates/transport/rrd-contract/fixtures/function-contract-v1.json": (
        "A-07",
        "I-01",
        "I-02",
    ),
    "crates/transport/rrd-contract/fixtures/model-manifest-v1.json": ("B-03",),
    "crates/transport/rrd-contract/fixtures/project-command-capability-v1.json": (
        "D-06",
    ),
    "crates/transport/rrd-contract/fixtures/activity-v1.json": ("I-03",),
    "crates/transport/rrd-contract/fixtures/websocket-protocol-v1.json": ("B-04",),
    "crates/transport/rrd-contract/tests/knowledge_contract.rs": ("KB-02",),
    "crates/transport/rrd-contract/tests/function_contract.rs": (
        "A-07",
        "I-01",
        "I-02",
    ),
    "crates/transport/rrd-contract/tests/deployment_contract.rs": (
        "A-07",
        "B-04",
        "H-04",
    ),
    "crates/transport/rrd-contract/tests/model_manifest_contract.rs": ("B-03",),
    "crates/transport/rrd-contract/tests/capability_contract.rs": ("D-06",),
    "crates/transport/rrd-contract/tests/activity_contract.rs": ("I-03",),
    "crates/transport/rrd-contract/tests/websocket_contract.rs": ("B-04",),
    "crates/transport/rrd-client/src/client.rs": ("A-07", "D-01", "H-04"),
    "crates/transport/rrd-client/src/endpoint.rs": ("A-07", "H-04", "H-07"),
    "crates/transport/rrd-client/src/error.rs": ("A-07", "H-04", "J-02"),
    "crates/transport/rrd-client/src/operation.rs": ("A-07", "H-04"),
    "crates/transport/rrd-client/src/retry.rs": ("A-07", "H-04", "J-02"),
    "crates/transport/rrd-client/src/session.rs": ("A-07", "H-04", "J-02"),
    "crates/transport/rrd-client/src/subscription.rs": (
        "A-07",
        "B-04",
        "H-04",
    ),
    "crates/transport/rrd-client/src/transport.rs": (
        "A-07",
        "B-04",
        "H-07",
        "J-03",
    ),
    "crates/transport/rrd-client/tests/operation_coverage.rs": ("H-04",),
    "crates/transport/rrd-client/tests/protocol_validation.rs": (
        "H-04",
        "J-02",
    ),
    "crates/transport/rrd-client/tests/transport_faults.rs": (
        "B-04",
        "H-04",
        "H-07",
        "J-02",
    ),
    "sdks/typescript/src/client.ts": ("A-07", "H-04"),
    "sdks/typescript/src/endpoint.ts": ("A-07", "H-04", "H-07"),
    "sdks/typescript/src/error.ts": ("A-07", "H-04", "J-02"),
    "sdks/typescript/src/operation.ts": ("A-07", "H-04"),
    "sdks/typescript/src/retry.ts": ("A-07", "H-04", "J-02"),
    "sdks/typescript/src/session.ts": ("A-07", "H-04", "J-02"),
    "sdks/typescript/src/subscription.ts": ("B-04", "H-04"),
    "sdks/typescript/src/transport.ts": (
        "A-07",
        "B-04",
        "H-07",
        "J-03",
    ),
    "sdks/typescript/src/generated/validators.ts": ("H-04", "J-02"),
    "sdks/typescript/tests/operation-coverage.test.ts": ("H-04",),
    "sdks/typescript/tests/protocol-validation.test.ts": ("H-04", "J-02"),
    "sdks/typescript/tests/transport-faults.test.ts": (
        "B-04",
        "H-04",
        "H-07",
        "J-02",
    ),
    "sdks/typescript/tests/package-consumer.test.ts": ("J-03", "J-05"),
    "sdks/typescript/tests/browser-conformance.test.ts": (
        "B-04",
        "H-04",
        "H-07",
        "J-03",
        "J-05",
    ),
    "sdks/typescript/tests/sdk-conformance.ts": ("A-07", "H-04", "J-02"),
    "sdks/python/src/rrd_client/async_client.py": ("H-04",),
    "sdks/python/src/rrd_client/endpoint.py": ("A-07", "H-04", "H-07"),
    "sdks/python/src/rrd_client/error.py": ("A-07", "H-04", "J-02"),
    "sdks/python/src/rrd_client/operation.py": ("A-07", "H-04"),
    "sdks/python/src/rrd_client/retry.py": ("A-07", "H-04", "J-02"),
    "sdks/python/src/rrd_client/session.py": ("A-07", "H-04", "J-02"),
    "sdks/python/src/rrd_client/subscription.py": ("B-04", "H-04"),
    "sdks/python/src/rrd_client/transport.py": (
        "A-07",
        "B-04",
        "H-07",
        "J-03",
    ),
    "sdks/python/src/rrd_client/py.typed": ("A-07", "J-03"),
    "sdks/python/src/rrd_client/generated/models.py": ("H-04", "J-02"),
    "sdks/python/tests/test_operation_coverage.py": ("H-04",),
    "sdks/python/tests/test_protocol_validation.py": ("H-04", "J-02"),
    "sdks/python/tests/test_transport_faults.py": (
        "B-04",
        "H-04",
        "H-07",
        "J-02",
    ),
    "sdks/python/tests/test_async_client.py": ("B-04", "H-04"),
    "sdks/python/tests/test_package_consumer.py": ("J-03", "J-05"),
    "sdks/go/doc.go": ("A-07", "J-03"),
    "sdks/go/config.go": ("A-07", "H-04", "H-07"),
    "sdks/go/endpoint.go": ("A-07", "H-04", "H-07"),
    "sdks/go/errors.go": ("A-07", "H-04", "J-02"),
    "sdks/go/operation.go": ("A-07", "H-04"),
    "sdks/go/retry.go": ("A-07", "H-04", "J-02"),
    "sdks/go/session.go": ("A-07", "H-04", "J-02"),
    "sdks/go/subscription.go": ("B-04", "H-04"),
    "sdks/go/transport.go": (
        "A-07",
        "B-04",
        "H-07",
        "J-03",
    ),
    "sdks/go/models_gen.go": ("H-04", "J-02"),
    "sdks/go/operation_coverage_test.go": ("H-04",),
    "sdks/go/protocol_validation_test.go": ("H-04", "J-02"),
    "sdks/go/transport_faults_test.go": (
        "B-04",
        "H-04",
        "H-07",
        "J-02",
    ),
    "sdks/go/subscription_test.go": ("B-04", "H-04", "J-02"),
    "sdks/go/package_consumer_test.go": ("J-03", "J-05"),
    "sdks/java/src/main/java/io/rrflow/rrd/package-info.java": (
        "A-07",
        "J-03",
    ),
    "sdks/java/src/main/java/io/rrflow/rrd/ClientConfig.java": (
        "A-07",
        "H-04",
        "H-07",
    ),
    "sdks/java/src/main/java/io/rrflow/rrd/EndpointResolver.java": (
        "A-07",
        "H-04",
        "H-07",
    ),
    "sdks/java/src/main/java/io/rrflow/rrd/HttpTransport.java": (
        "A-07",
        "B-04",
        "H-04",
        "H-07",
        "J-03",
    ),
    "sdks/java/src/main/java/io/rrflow/rrd/OperationBinding.java": (
        "A-07",
        "H-04",
    ),
    "sdks/java/src/main/java/io/rrflow/rrd/OperationExecutor.java": (
        "A-07",
        "H-04",
    ),
    "sdks/java/src/main/java/io/rrflow/rrd/ProtocolCodec.java": (
        "A-07",
        "H-04",
        "J-02",
    ),
    "sdks/java/src/main/java/io/rrflow/rrd/RetryPolicy.java": (
        "A-07",
        "H-04",
        "J-02",
    ),
    "sdks/java/src/main/java/io/rrflow/rrd/RrdCall.java": ("B-04", "H-04"),
    "sdks/java/src/main/java/io/rrflow/rrd/Subscription.java": (
        "B-04",
        "H-04",
    ),
    "sdks/java/src/main/java/io/rrflow/rrd/WebSocketTransport.java": (
        "B-04",
        "H-04",
        "H-07",
        "J-03",
    ),
    "sdks/java/src/main/java/io/rrflow/rrd/generated/OperationModels.java": (
        "H-04",
        "J-02",
    ),
    "sdks/java/src/test/java/io/rrflow/rrd/OperationCoverageTest.java": ("H-04",),
    "sdks/java/src/test/java/io/rrflow/rrd/ProtocolValidationTest.java": (
        "H-04",
        "J-02",
    ),
    "sdks/java/src/test/java/io/rrflow/rrd/TransportFaultsTest.java": (
        "B-04",
        "H-04",
        "H-07",
        "J-02",
    ),
    "sdks/java/src/test/java/io/rrflow/rrd/SubscriptionTest.java": (
        "B-04",
        "H-04",
        "J-02",
    ),
    "sdks/java/src/test/java/io/rrflow/rrd/ConcurrencyTest.java": (
        "B-04",
        "H-04",
        "J-02",
    ),
    "sdks/java/src/test/java/io/rrflow/rrd/PackageConsumerTest.java": (
        "J-03",
        "J-05",
    ),
    "sdks/dotnet/global.json": ("A-07", "J-03", "J-05"),
    "sdks/dotnet/Directory.Build.props": (
        "A-07",
        "J-01",
        "J-02",
        "J-05",
    ),
    "sdks/dotnet/Directory.Packages.props": ("A-07", "J-02", "J-05"),
    "sdks/dotnet/README.md": ("A-07", "J-03", "J-05"),
    "sdks/dotnet/scripts/build_package.py": ("J-03", "J-05"),
    "sdks/dotnet/src/Rrflow.Rrd.Client/ClientOptions.cs": (
        "A-07",
        "H-04",
        "H-07",
    ),
    "sdks/dotnet/src/Rrflow.Rrd.Client/EndpointResolver.cs": (
        "A-07",
        "H-04",
        "H-07",
    ),
    "sdks/dotnet/src/Rrflow.Rrd.Client/HttpTransport.cs": (
        "A-07",
        "B-04",
        "H-04",
        "H-07",
        "J-03",
    ),
    "sdks/dotnet/src/Rrflow.Rrd.Client/OperationBinding.cs": (
        "A-07",
        "H-04",
    ),
    "sdks/dotnet/src/Rrflow.Rrd.Client/OperationExecutor.cs": (
        "A-07",
        "H-04",
    ),
    "sdks/dotnet/src/Rrflow.Rrd.Client/ProtocolCodec.cs": (
        "A-07",
        "H-04",
        "J-02",
    ),
    "sdks/dotnet/src/Rrflow.Rrd.Client/RequestOptions.cs": (
        "A-07",
        "H-04",
    ),
    "sdks/dotnet/src/Rrflow.Rrd.Client/ResourcePath.cs": (
        "A-07",
        "H-04",
    ),
    "sdks/dotnet/src/Rrflow.Rrd.Client/RetryPolicy.cs": (
        "A-07",
        "H-04",
        "J-02",
    ),
    "sdks/dotnet/src/Rrflow.Rrd.Client/RrdCall.cs": ("B-04", "H-04"),
    "sdks/dotnet/src/Rrflow.Rrd.Client/Session.cs": (
        "A-07",
        "H-04",
        "J-02",
    ),
    "sdks/dotnet/src/Rrflow.Rrd.Client/Subscription.cs": ("B-04", "H-04"),
    "sdks/dotnet/src/Rrflow.Rrd.Client/WebSocketTransport.cs": (
        "B-04",
        "H-04",
        "H-07",
        "J-03",
    ),
    "sdks/dotnet/src/Rrflow.Rrd.Client/Generated/OperationId.g.cs": (
        "A-07",
        "H-04",
        "J-02",
    ),
    "sdks/dotnet/src/Rrflow.Rrd.Client/Generated/OperationModels.g.cs": (
        "H-04",
        "J-02",
    ),
    "sdks/dotnet/src/Rrflow.Rrd.Client/Generated/RrdJsonContext.g.cs": (
        "H-04",
        "J-02",
        "J-03",
    ),
    "sdks/dotnet/tests/Rrflow.Rrd.Client.Tests/ConcurrencyTests.cs": (
        "B-04",
        "H-04",
        "J-02",
    ),
    "sdks/dotnet/tests/Rrflow.Rrd.Client.Tests/OperationCoverageTests.cs": ("H-04",),
    "sdks/dotnet/tests/Rrflow.Rrd.Client.Tests/PackageConsumerTests.cs": (
        "J-03",
        "J-05",
    ),
    "sdks/dotnet/tests/Rrflow.Rrd.Client.Tests/ProtocolValidationTests.cs": (
        "H-04",
        "J-02",
    ),
    "sdks/dotnet/tests/Rrflow.Rrd.Client.Tests/SubscriptionTests.cs": (
        "B-04",
        "H-04",
        "J-02",
    ),
    "sdks/dotnet/tests/Rrflow.Rrd.Client.Tests/TransportFaultsTests.cs": (
        "B-04",
        "H-04",
        "H-07",
        "J-02",
    ),
    "crates/compute/rrd-attunement/Cargo.toml": ("A-07", "D-03"),
    "crates/compute/rrd-attunement/src/lib.rs": ("A-07", "D-03"),
    "crates/compute/rrd-attunement/src/inventory.rs": ("D-03",),
    "crates/compute/rrd-attunement/src/capability.rs": ("D-06",),
    "crates/compute/rrd-attunement/src/parse.rs": ("D-04",),
    "crates/compute/rrd-attunement/src/normalize.rs": ("D-05",),
    "crates/compute/rrd-attunement/src/entity_link.rs": ("D-05",),
    "crates/compute/rrd-attunement/src/ground.rs": ("D-05",),
    "crates/compute/rrd-attunement/src/verify.rs": ("D-05",),
    "crates/compute/rrd-attunement/tests/inventory.rs": ("D-03",),
    "crates/compute/rrd-attunement/tests/capability_discovery.rs": ("D-06",),
    "crates/compute/rrd-attunement/tests/fixtures/project-command-capabilities.toml": (
        "D-06",
    ),
    "crates/compute/rrd-attunement/tests/incremental_parse.rs": ("D-04",),
    "crates/compute/rrd-attunement/tests/phases.rs": ("D-05",),
    "crates/persistence/rrd-lsm/src/transaction.rs": ("C-02",),
    "crates/persistence/rrd-lsm/tests/transaction_conformance.rs": ("C-02",),
    "crates/persistence/rrd-lsm/tests/hybrid_segment.rs": ("C-06", "C-07"),
    "crates/persistence/rrd-lsm/tests/mapped_page_lifetime.rs": ("C-07", "F-01"),
    "crates/persistence/rrd-lsm/tests/write_diagnostics.rs": ("C-07", "J-04"),
    "crates/persistence/rrd-lsm/src/segment/mod.rs": ("C-06",),
    "crates/persistence/rrd-lsm/src/segment/format.rs": ("C-06", "C-07"),
    "crates/persistence/rrd-lsm/src/segment/spine.rs": ("C-06", "C-07"),
    "crates/persistence/rrd-lsm/src/segment/column.rs": ("C-06", "C-07"),
    "crates/persistence/rrd-lsm/src/segment/reader.rs": ("C-06", "C-07"),
    "crates/persistence/rrd-lsm/src/segment/writer.rs": ("C-06", "C-07"),
    "crates/persistence/rrd-lsm/src/segment/cache.rs": ("C-06", "C-07", "F-05"),
    "crates/persistence/rrd-store/src/key_codec.rs": ("C-01",),
    "crates/persistence/rrd-store/src/transaction.rs": ("C-02", "C-03"),
    "crates/persistence/rrd-store/tests/key_codec.rs": ("C-01",),
    "crates/persistence/rrd-store/tests/storage_profile_conformance.rs": ("C-02",),
    "crates/persistence/rrd-store/tests/semantic_commit_atomicity.rs": ("C-03",),
    "crates/persistence/rrd-store/tests/native_index_commit.rs": (
        "C-03",
        "E-02",
        "E-03",
        "E-04",
    ),
    "crates/persistence/rrd-store/tests/direct_read_paths.rs": ("C-04",),
    "crates/persistence/rrd-store/src/access/mod.rs": ("C-03", "C-04"),
    "crates/persistence/rrd-store/src/access/runtime_state.rs": ("C-03", "C-04"),
    "crates/persistence/rrd-store/src/access/semantic_commit.rs": ("C-03",),
    "crates/persistence/rrd-store/src/access/record.rs": ("C-04",),
    "crates/persistence/rrd-store/src/access/relation.rs": ("C-04", "E-01"),
    "crates/persistence/rrd-store/src/access/index.rs": ("C-03", "E-02", "E-03"),
    "crates/persistence/rrd-store/src/access/vector.rs": ("C-03", "E-04"),
    "crates/transport/rrd-contract/fixtures/graphql-equivalence-v1.json": ("B-05",),
    "crates/compute/rrd-query/src/graphql.rs": ("B-05",),
    "crates/compute/rrd-query/tests/graphql_equivalence.rs": ("B-05",),
    "crates/compute/rrd-query/tests/provider_streaming.rs": ("F-01", "F-02"),
    "crates/compute/rrd-query/tests/native_operators.rs": ("F-03",),
    "crates/compute/rrd-query/tests/resource_budgets.rs": ("F-04",),
    "crates/compute/rrd-query/src/budget.rs": ("F-04",),
    "crates/compute/rrd-query/src/provider/mod.rs": ("F-01",),
    "crates/compute/rrd-query/src/provider/scan.rs": ("F-01", "F-02"),
    "crates/compute/rrd-query/src/provider/stream.rs": ("F-01", "F-04"),
    "crates/compute/rrd-query/src/provider/metrics.rs": ("F-02", "F-04"),
    "crates/compute/rrd-query/src/physical/mod.rs": ("F-03",),
    "crates/compute/rrd-query/src/physical/graph.rs": ("E-01", "F-03"),
    "crates/compute/rrd-query/src/physical/bm25.rs": ("E-03", "F-03"),
    "crates/compute/rrd-query/src/physical/vector.rs": ("E-04", "F-03"),
    "crates/compute/rrd-query/src/physical/rrf.rs": ("F-03", "H-02"),
    "crates/compute/rrd-inference/src/router.rs": ("G-01",),
    "crates/transport/rrd-contract/src/knowledge.rs": ("KB-02",),
    "crates/transport/rrd-contract/src/deployment.rs": (
        "A-07",
        "B-04",
        "H-04",
    ),
    "crates/transport/rrd-contract/src/lifecycle.rs": ("D-01", "D-11"),
    "crates/transport/rrd-contract/src/model_manifest.rs": ("B-03",),
    "crates/transport/rrd-contract/src/websocket.rs": ("B-04",),
    "crates/transport/rrd-contract/src/engine_event.rs": ("I-01",),
    "crates/transport/rrd-contract/src/capability.rs": ("D-06", "I-06"),
    "crates/transport/rrd-contract/src/activity.rs": ("I-03",),
    "crates/transport/rrd-contract/src/skill.rs": ("I-05",),
    "crates/authority/rrd-engine/src/engine/attunement.rs": (
        "D-02",
        "D-05",
        "KB-06",
    ),
    "crates/authority/rrd-engine/src/engine/function/mod.rs": (
        "A-07",
        "C-03",
        "H-04",
        "I-01",
        "I-02",
        "I-06",
    ),
    "crates/authority/rrd-engine/src/engine/function/catalogue.rs": (
        "A-07",
        "C-01",
        "C-03",
        "C-04",
        "D-01",
        "I-06",
    ),
    "crates/authority/rrd-engine/src/engine/function/execution.rs": (
        "A-07",
        "C-03",
        "H-04",
        "H-05",
        "J-02",
    ),
    "crates/authority/rrd-engine/src/engine/function/javascript.rs": (
        "A-07",
        "J-02",
        "J-05",
    ),
    "crates/authority/rrd-engine/src/engine/function/webassembly.rs": (
        "A-07",
        "J-02",
        "J-05",
    ),
    "crates/authority/rrd-engine/src/engine/function/transaction_binding.rs": (
        "A-07",
        "C-03",
        "I-01",
        "I-02",
    ),
    "crates/authority/rrd-engine/src/engine/integrity.rs": ("D-01", "D-11"),
    "crates/authority/rrd-engine/src/engine/repair.rs": ("D-11",),
    "crates/authority/rrd-engine/src/engine/capabilities.rs": ("D-06", "I-06"),
    "crates/authority/rrd-engine/src/engine/activities.rs": ("I-03",),
    "crates/authority/rrd-engine/src/engine/routing.rs": ("G-02", "G-04", "G-05"),
    "crates/authority/rrd-engine/src/engine/events.rs": ("I-01", "I-02"),
    "crates/authority/rrd-engine/src/engine/routines.rs": ("I-03", "I-07"),
    "crates/authority/rrd-engine/src/engine/skills.rs": ("I-05", "I-07"),
    "crates/authority/rrd-engine/tests/attunement_resume.rs": (
        "D-02",
        "D-05",
        "KB-06",
        "KB-07",
    ),
    "crates/authority/rrd-engine/tests/function_conformance.rs": (
        "A-07",
        "C-03",
        "H-04",
        "H-05",
        "I-01",
        "I-02",
        "I-06",
        "J-02",
        "J-03",
        "J-05",
    ),
    "crates/authority/rrd-engine/tests/install_security.rs": (
        "D-01",
        "D-02",
        "J-02",
        "J-03",
    ),
    "crates/authority/rrd-engine/tests/routing_conformance.rs": (
        "G-02",
        "G-04",
        "G-05",
    ),
    "crates/authority/rrd-engine/tests/context_flow.rs": ("H-01", "H-02"),
    "crates/authority/rrd-engine/tests/commit_impact_live.rs": ("H-03",),
    "crates/authority/rrd-engine/tests/automation_conformance.rs": (
        "I-01",
        "I-02",
        "I-03",
        "I-05",
        "I-07",
    ),
    "crates/authority/rrd-engine/tests/capability_activity_conformance.rs": (
        "D-06",
        "I-03",
        "I-06",
        "J-02",
    ),
    "crates/adapters/rrflow-lfg/Cargo.toml": ("A-07", "G-01", "G-03"),
    "crates/adapters/rrflow-lfg/src/lib.rs": ("G-01", "G-03"),
    "crates/adapters/rrflow-lfg/tests/router_conformance.rs": (
        "G-01",
        "G-03",
        "G-06",
    ),
    "crates/adapters/rrflow-local-process/Cargo.toml": ("A-07", "D-01"),
    "crates/adapters/rrflow-local-process/src/lib.rs": ("A-07", "D-01"),
    "crates/adapters/rrflow-local-process/src/artifact.rs": ("D-01", "J-02"),
    "crates/adapters/rrflow-local-process/src/diagnostics.rs": ("D-01", "J-02"),
    "crates/adapters/rrflow-local-process/src/identity.rs": ("D-01", "J-02"),
    "crates/adapters/rrflow-local-process/src/invocation.rs": (
        "D-01",
        "J-02",
    ),
    "crates/adapters/rrflow-local-process/src/activity.rs": (
        "I-03",
        "J-02",
    ),
    "crates/adapters/rrflow-local-process/src/platform/mod.rs": ("D-01", "J-02"),
    "crates/adapters/rrflow-local-process/src/platform/unix.rs": ("D-01", "J-02"),
    "crates/adapters/rrflow-local-process/src/platform/windows.rs": (
        "D-01",
        "J-02",
    ),
    "crates/adapters/rrflow-local-process/src/readiness.rs": (
        "D-01",
        "H-04",
        "J-02",
    ),
    "crates/adapters/rrflow-local-process/src/shutdown.rs": (
        "D-01",
        "H-04",
        "J-02",
    ),
    "crates/adapters/rrflow-local-process/tests/artifact_identity.rs": (
        "D-01",
        "J-02",
    ),
    "crates/adapters/rrflow-local-process/tests/conformance.rs": (
        "D-01",
        "D-02",
        "H-04",
        "J-02",
        "J-03",
        "J-05",
    ),
    "crates/adapters/rrflow-local-process/tests/activity_conformance.rs": (
        "I-03",
        "J-02",
    ),
    "crates/adapters/rrflow-local-process/tests/effect_gap.rs": ("D-02", "J-02"),
    "crates/adapters/rrflow-local-process/tests/resource_limits.rs": (
        "D-01",
        "J-02",
    ),
    "crates/adapters/rrflow-kubernetes/Cargo.toml": ("A-07", "D-01"),
    "crates/adapters/rrflow-kubernetes/src/lib.rs": ("A-07", "D-01"),
    "crates/adapters/rrflow-kubernetes/src/main.rs": ("A-07", "D-01"),
    "crates/adapters/rrflow-kubernetes/src/api.rs": ("A-07", "B-04", "D-01"),
    "crates/adapters/rrflow-kubernetes/src/plan.rs": (
        "A-07",
        "C-02",
        "C-03",
        "D-01",
        "D-02",
    ),
    "crates/adapters/rrflow-kubernetes/src/render.rs": ("A-07", "D-01", "J-03"),
    "crates/adapters/rrflow-kubernetes/src/reconcile.rs": (
        "A-07",
        "D-02",
        "H-04",
        "H-05",
        "J-02",
    ),
    "crates/adapters/rrflow-kubernetes/src/readiness.rs": (
        "A-07",
        "D-01",
        "H-04",
        "J-02",
    ),
    "crates/adapters/rrflow-kubernetes/src/status.rs": (
        "A-07",
        "B-04",
        "H-04",
        "H-05",
    ),
    "crates/adapters/rrflow-kubernetes/src/finalize.rs": (
        "A-07",
        "D-02",
        "J-02",
    ),
    "crates/adapters/rrflow-kubernetes/src/bin/rrflow-kubernetes-crd.rs": (
        "A-07",
        "D-01",
        "J-03",
    ),
    "crates/adapters/rrflow-kubernetes/tests/contract.rs": (
        "A-07",
        "B-04",
        "D-01",
    ),
    "crates/adapters/rrflow-kubernetes/tests/api_server.rs": (
        "A-07",
        "D-01",
        "J-02",
        "J-03",
    ),
    "crates/adapters/rrflow-kubernetes/tests/install.rs": (
        "A-07",
        "D-01",
        "D-02",
        "J-03",
        "J-05",
    ),
    "crates/adapters/rrflow-kubernetes/tests/reconcile.rs": (
        "A-07",
        "D-02",
        "H-04",
        "H-05",
        "J-02",
    ),
    "crates/adapters/rrflow-kubernetes/tests/effect_gap.rs": (
        "A-07",
        "C-03",
        "D-02",
        "J-02",
    ),
    "crates/adapters/rrflow-kubernetes/tests/engine_conformance.rs": (
        "A-07",
        "C-02",
        "C-03",
        "E-01",
        "E-02",
        "E-03",
        "E-04",
        "E-05",
        "F-01",
        "F-02",
        "F-03",
        "F-04",
        "F-05",
        "G-04",
        "G-05",
        "H-01",
        "H-02",
        "H-04",
        "H-05",
        "J-02",
        "J-03",
        "J-05",
    ),
    "crates/adapters/rrflow-kubernetes/tests/deletion_recovery.rs": (
        "A-07",
        "D-02",
        "J-02",
        "J-03",
    ),
    "crates/adapters/rrflow-kubernetes/tests/resource_limits.rs": (
        "A-07",
        "F-04",
        "J-02",
        "J-04",
        "J-05",
    ),
    "crates/adapters/rrflow-kubernetes/tests/security.rs": (
        "A-07",
        "H-04",
        "H-07",
        "J-02",
        "J-05",
    ),
    "crates/adapters/rrflow-host-events/Cargo.toml": ("A-07", "I-04"),
    "crates/adapters/rrflow-host-events/src/lib.rs": ("I-04",),
    "crates/adapters/rrflow-host-events/src/reference.rs": ("I-04",),
    "crates/adapters/rrflow-host-events/src/claude.rs": ("I-04",),
    "crates/adapters/rrflow-host-events/src/codex.rs": ("I-04",),
    "crates/adapters/rrflow-host-events/src/gemini.rs": ("I-04",),
    "crates/adapters/rrflow-host-events/fixtures/host-event-v1.json": ("I-04",),
    "crates/adapters/rrflow-host-events/tests/conformance.rs": ("I-04",),
    "crates/adapters/rrflow-mesh/Cargo.toml": ("A-07", "H-07"),
    "crates/adapters/rrflow-mesh/src/lib.rs": ("H-07",),
    "crates/adapters/rrflow-mesh/src/resolver.rs": ("H-07",),
    "crates/adapters/rrflow-mesh/tests/resolver_conformance.rs": ("H-07",),
    "crates/adapters/rrflow-devforge/Cargo.toml": ("A-07", "D-07", "D-08", "D-10"),
    "crates/adapters/rrflow-devforge/src/lib.rs": ("D-07", "D-08", "D-10"),
    "crates/adapters/rrflow-devforge/src/placement.rs": ("D-07",),
    "crates/adapters/rrflow-devforge/src/mounts.rs": ("D-08",),
    "crates/adapters/rrflow-devforge/src/accounting.rs": ("D-09",),
    "crates/adapters/rrflow-devforge/src/hibernation.rs": ("D-10",),
    "crates/adapters/rrflow-devforge/tests/placement.rs": ("D-07",),
    "crates/adapters/rrflow-devforge/tests/mounts.rs": ("D-08",),
    "crates/adapters/rrflow-devforge/tests/accounting.rs": ("D-09",),
    "crates/adapters/rrflow-devforge/tests/hibernation.rs": ("D-10",),
    "crates/persistence/rrd-lsm/src/inspection.rs": ("D-01", "C-07", "D-11"),
    "crates/persistence/rrd-store/src/integrity.rs": ("D-01", "D-11"),
    "crates/adapters/rrflow-cli/src/repair.rs": ("D-11",),
    "crates/adapters/rrflow-cli/src/automation_install.rs": ("I-06",),
    "crates/adapters/rrflow-cli/tests/installed_lifecycle.rs": (
        "D-01",
        "D-11",
        "J-03",
        "J-05",
    ),
    "crates/adapters/rrflow-cli/tests/repair_conformance.rs": ("D-11", "J-02"),
    "crates/adapters/rrflow-cli/tests/automation_install_conformance.rs": ("I-06",),
    "crates/adapters/rrflow-cli/templates/project-v1/manifest.toml": ("D-01",),
    "crates/adapters/rrflow-cli/templates/project-v1/AGENTS.md": ("D-01",),
    "crates/adapters/rrflow-cli/templates/project-v1/CLAUDE.md": ("D-01",),
    "crates/adapters/rrflow-cli/templates/project-v1/GEMINI.md": ("D-01",),
    "crates/adapters/rrflow-cli/templates/project-v1/.rrflow/config.toml": ("D-01",),
    "crates/adapters/rrflow-cli/tests/fixtures/install-empty/.keep": ("D-01",),
    "crates/adapters/rrflow-cli/tests/fixtures/install-existing/README.md": ("D-01",),
    "crates/adapters/rrflow-cli/tests/fixtures/install-existing/AGENTS.md": ("D-01",),
    "crates/compute/rrd-attunement/src/source.rs": ("D-06",),
    "crates/compute/rrd-attunement/tests/source_discovery.rs": ("D-06",),
    "crates/compute/rrd-attunement/tests/fixtures/external-sources.toml": ("D-06",),
    "crates/operations/rrd-operator-knowledge/src/source.rs": ("D-06",),
    "crates/transport/rrd-server/src/http/handlers/graphql.rs": ("H-04",),
    "crates/transport/rrd-server/tests/cross_surface_conformance.rs": ("H-04",),
    "crates/evaluation/rrflow-eval/src/bin/release-evidence.rs": ("J-02", "J-04"),
    "docs/evidence/integration/connectome-conformance.json": ("H-06", "J-03"),
    "eval/connectome_conformance.py": ("H-06", "J-03"),
    "scripts/knowledge/export.py": ("KB-03",),
    "scripts/knowledge/test_export.py": ("KB-03", "KB-04"),
    "scripts/release/qualify.py": ("J-02", "J-03"),
    "scripts/release/compare_deployment.py": ("J-04", "J-05"),
    "scripts/release/assemble.py": ("J-03", "J-05"),
    "scripts/release/verify.py": ("J-05",),
    "scripts/release/test_distribution.py": ("J-03", "J-05"),
    "fixtures/release/deployment-baselines-v1.toml": ("J-04",),
    "fixtures/release/distribution-manifest-v1.json": ("J-05",),
    "fixtures/rrd-function-conformance-v1.json": (
        "A-07",
        "C-03",
        "H-04",
        "J-02",
        "J-03",
        "J-05",
    ),
    "docs/architecture/instance-topology.md": ("KB-05",),
    "docs/evidence/README.md": ("KB-05", "KB-08"),
    "docs/evidence/comparisons/README.md": ("KB-05", "KB-08"),
    "docs/evidence/comparisons/rrflow-surrealdb-claim-differential.md": ("KB-05",),
    "docs/evidence/test-plans/README.md": ("KB-05", "KB-08"),
    ".github/workflows/rrd-lsm-benchmark.yml": ("C-07", "J-02", "J-04"),
    "docs/evidence/change-journals/gate-c/c-07-native-batch-write-tail-attribution.md": (
        "C-07",
        "J-04",
    ),
    "docs/evidence/test-plans/persistence-scenario-matrix.md": (
        "KB-05",
        "C-07",
        "J-04",
    ),
    "docs/reference/storage/rrflowkv-benchmark-harness.md": ("C-07", "J-04"),
    "docs/guides/README.md": ("D-01", "KB-08"),
    "docs/guides/installation/README.md": ("D-01", "KB-08"),
    "docs/guides/installation/security-bootstrap.md": ("D-01",),
    "docs/operations/README.md": ("KB-05", "KB-08"),
    "docs/reference/automation/README.md": ("KB-05", "KB-08"),
    "docs/reference/automation/functions.md": ("KB-05",),
    "docs/reference/automation/project-command-capabilities.md": ("KB-05",),
    "docs/reference/client/README.md": ("KB-05", "KB-08"),
    "docs/reference/client/connectome.md": (
        "KB-05",
        "H-04",
        "H-06",
        "H-07",
        "J-03",
        "J-05",
    ),
    "docs/reference/client/connectome-implementation-guide.md": (
        "D-01",
        "H-04",
        "H-06",
        "J-03",
        "J-05",
    ),
    "docs/reference/context/README.md": ("KB-05", "KB-08"),
    "docs/reference/context/context-maintenance.md": ("KB-05",),
    "docs/reference/context/retrieval.md": ("KB-05",),
    "docs/reference/data/README.md": ("KB-05", "KB-08"),
    "docs/reference/data/multi-model-object-contract.md": ("KB-05",),
    "docs/reference/data/schema-catalogue.md": ("KB-05",),
    "docs/reference/data/time-travel-and-rollback.md": ("KB-05",),
    "docs/reference/deployment/README.md": ("KB-05", "KB-08"),
    "docs/reference/deployment/edge.md": ("KB-05",),
    "docs/reference/deployment/kubernetes-operator.md": ("KB-05",),
    "docs/reference/deployment/local-process-driver.md": ("KB-05",),
    "docs/reference/deployment/modes.md": ("KB-05",),
    "docs/reference/distributed/README.md": ("KB-05", "KB-08"),
    "docs/reference/distributed/cluster-contract.md": ("KB-05",),
    "docs/reference/inference/README.md": ("KB-05", "KB-08"),
    "docs/reference/inference/embedding-and-model-bound-search.md": ("KB-05",),
    "docs/reference/operations/README.md": ("KB-05", "KB-08"),
    "docs/reference/operations/estate-control.md": ("KB-05",),
    "docs/reference/protocol/README.md": ("KB-05", "KB-08"),
    "docs/reference/protocol/public-contract.md": ("KB-05",),
    "docs/reference/protocol/server.md": ("KB-05",),
    "docs/reference/protocol/subscriptions.md": ("KB-05",),
    "docs/reference/query/README.md": ("KB-05", "KB-08"),
    "docs/reference/query/index-catalogue.md": ("KB-05",),
    "docs/reference/query/live-query.md": ("KB-05",),
    "docs/reference/query/multi-model.md": ("KB-05",),
    "docs/reference/query/transactions-and-joins.md": ("KB-05",),
    "docs/reference/release/README.md": ("KB-05", "KB-08"),
    "docs/reference/release/version-policy.md": ("KB-05",),
    "docs/reference/sdk/README.md": ("KB-05", "KB-08"),
    "docs/reference/sdk/dotnet.md": ("KB-05",),
    "docs/reference/sdk/go.md": ("KB-05",),
    "docs/reference/sdk/java.md": ("KB-05",),
    "docs/reference/sdk/python.md": ("KB-05",),
    "docs/reference/sdk/rust.md": ("KB-05",),
    "docs/reference/sdk/typescript.md": ("KB-05",),
    "docs/reference/security/README.md": ("KB-05", "KB-08"),
    "docs/reference/security/authority.md": ("KB-05",),
    "docs/reference/security/local-estate-authorization.md": ("KB-05",),
    "docs/reference/storage/logical-archive.md": ("KB-05",),
    "docs/reference/storage/tiered-persistence.md": ("KB-05",),
    "docs/reference/vector/README.md": ("KB-05", "KB-08"),
    "docs/reference/vector/collections.md": ("KB-05",),
    "docs/reference/vector/compact-dense-artifact.md": ("KB-05",),
    "docs/reference/vector/hnsw-projection.md": ("KB-05",),
    "docs/reference/vector/memory-tiers.md": ("KB-05",),
    "docs/reference/vector/quantization-lifecycle.md": ("KB-05",),
    "docs/reference/vector/search.md": ("KB-05",),
    "docs/research/qdrant-capability-inventory.md": ("KB-05",),
    "docs/research/surrealdb-capability-inventory.md": ("KB-05",),
}


def git_files() -> list[str]:
    result = subprocess.run(
        [
            "git",
            "ls-files",
            "-z",
            "--cached",
            "--others",
            "--exclude-standard",
        ],
        cwd=ROOT,
        check=True,
        capture_output=True,
    )
    paths = [item.decode("utf-8") for item in result.stdout.split(b"\0") if item]
    output_relative = OUTPUT.relative_to(ROOT).as_posix()
    if output_relative not in paths:
        paths.append(output_relative)
    return sorted(paths)


def file_kind(path: str) -> str:
    suffix = Path(path).suffix.lower()
    if path.endswith("Cargo.toml") or path == "Cargo.toml":
        return "cargo-manifest"
    if path == "Cargo.lock":
        return "dependency-lock"
    if suffix == ".rs":
        if "/tests/" in path or path.endswith("/tests.rs"):
            return "rust-test"
        if "/examples/" in path:
            return "rust-example"
        if "/benches/" in path:
            return "rust-benchmark"
        return "rust-source"
    if suffix == ".md":
        return "documentation"
    if suffix in {".json", ".hex", ".xml"}:
        return "fixture-or-generated-contract"
    if suffix in {".py", ".sh", ".ts", ".go", ".java", ".cs"}:
        return "tool-sdk-or-test-source"
    if suffix in {".toml", ".yaml", ".yml"}:
        return "configuration-or-deployment"
    return "repository-asset"


def package_name(path: str) -> str | None:
    parts = Path(path).parts
    return parts[2] if len(parts) > 2 and parts[0] == "crates" else None


def planned_gates(path: str) -> tuple[str, ...]:
    if path == ACTIVE_CHANGE_PLAN:
        gates = ("J-01", "J-02")
    elif path in FILE_OVERRIDES:
        gates = ("A-07", *FILE_OVERRIDES[path])
    elif path in PLANNED_PATHS:
        gates = PLANNED_PATHS[path]
    elif (package := package_name(path)) is not None:
        gates = PACKAGE_GATES.get(package, ("A-07", "J-01"))
    elif path in {"README.md", "docs/README.md"} or (
        path.startswith("docs/") and path.endswith("/README.md")
    ):
        gates = ("A-06", "KB-08", "J-01")
    elif path.startswith("docs/") and len(Path(path).parts) == 2:
        gates = ("KB-05", "J-01")
    elif path.startswith("docs/") or path in {
        "AGENTS.md",
        "CLAUDE.md",
        "GEMINI.md",
        "SPEC.md",
    }:
        gates = ("A-06", "J-01")
    elif path.startswith("sdks/"):
        gates = ("H-04", "J-02", "J-05")
    elif path.startswith("eval/"):
        gates = ("G-06", "J-02", "J-04")
    elif path.startswith("deploy/"):
        gates = ("D-07", "D-10", "J-03", "J-05")
    elif path.startswith("fixtures/"):
        gates = ("B-04", "H-04", "J-02")
    elif path == "scripts/ci/check_documentation.py" or path == (
        ".github/workflows/ci-reusable.yml"
    ):
        gates = ("KB-04", "A-07", "J-01", "J-02", "J-05")
    elif path.startswith(("scripts/", ".github/")):
        gates = ("A-06", "A-07", "J-01", "J-02", "J-05")
    else:
        gates = ("A-07", "J-01", "J-05")
    additions = (
        *(("C-04",) if path in C04_DIRECT_READ_PATHS else ()),
        *(("H-05",) if path in H05_OBSERVABILITY_PATHS else ()),
    )
    return tuple(dict.fromkeys((*gates, *additions)))


def action(path: str) -> str:
    if path in REMOVE_OR_REWRITE:
        return REMOVE_OR_REWRITE[path]
    if path in {"CLAUDE.md", "GEMINI.md"}:
        return "retain as a forwarding-only host instruction adapter; never add lifecycle state or copied rules"
    if path == "AGENTS.md":
        return "retain as the only provider-neutral repository instruction body"
    if path == "README.md":
        return "retain as the product portal; link owners and never absorb detailed roadmap or architecture bodies"
    if path == ACTIVE_CHANGE_PLAN:
        return "replace only in a separate planning commit; never edit alongside implementation"
    if path == "Cargo.lock":
        return "regenerate only after an accepted manifest change; never hand-edit"
    if path in GENERATED_PROJECTIONS:
        return (
            f"regenerate deterministically with {GENERATED_PROJECTIONS[path]}; "
            "treat as a replaceable projection and never hand-edit semantic state"
        )
    if path.startswith("docs/") and len(Path(path).parts) == 2:
        return "review the complete record in KB-05, merge accepted current material into its one owner, then retain that owner or remove the redundant source"
    if path.startswith("docs/history/"):
        return "merge any current requirement into its active owner; retain only evidence-required provenance and remove redundant narrative"
    if path.startswith("docs/"):
        return "retain under its declared documentation owner; update only when the owning gate supplies evidence"
    if (
        "/tests/" in path
        or "/src/test/" in path
        or path.endswith("/tests.rs")
        or path.startswith("eval/")
    ):
        return "retain as inventory; rewrite or extend only to supply the acceptance evidence assigned to this file"
    if path.startswith("sdks/"):
        return "retain as an outward projection; regenerate and test only after the shared contract changes"
    return "review the complete file at its first assigned gate; retain unchanged unless the execution-map work package names a required modification"


def line_count(data: bytes) -> int | None:
    if b"\0" in data:
        return None
    try:
        data.decode("utf-8")
    except UnicodeDecodeError:
        return None
    if not data:
        return 0
    return data.count(b"\n") + (0 if data.endswith(b"\n") else 1)


def current_record(path: str) -> dict[str, object]:
    relative_output = OUTPUT.relative_to(ROOT).as_posix()
    if path == relative_output:
        return {
            "path": path,
            "state": "generated",
            "kind": "execution-inventory",
            "coverage": "self-record; content digest and line count intentionally omitted",
            "planned_gates": ["A-06"],
            "action": "regenerate deterministically after repository path or content changes",
        }
    data = (ROOT / path).read_bytes()
    lines = line_count(data)
    return {
        "path": path,
        "state": "current",
        "kind": file_kind(path),
        "bytes": len(data),
        "lines": lines,
        "sha256": hashlib.sha256(data).hexdigest(),
        "coverage": f"lines 1-{lines}" if lines is not None else f"bytes 0-{len(data)}",
        "planned_gates": list(dict.fromkeys(planned_gates(path))),
        "action": action(path),
    }


def records() -> list[dict[str, object]]:
    current_paths = git_files()
    result = [current_record(path) for path in current_paths]
    current_set = set(current_paths)
    for path, gates in PLANNED_PATHS.items():
        if path in current_set:
            continue
        result.append(
            {
                "path": path,
                "state": "planned",
                "kind": file_kind(path),
                "coverage": "file does not exist; creation is gated",
                "planned_gates": list(gates),
                "action": "create only in the first assigned gate and only after its prerequisites pass",
            }
        )
    return sorted(
        result, key=lambda record: (str(record["path"]), str(record["state"]))
    )


def validate(records_to_check: list[dict[str, object]]) -> None:
    roadmap = ROADMAP.read_text(encoding="utf-8")
    roadmap_ids = set(
        re.findall(r"(?m)^\| \[[ x]\] \| ((?:[A-J]-\d{2})|(?:KB-\d{2})) \|", roadmap)
    )
    incomplete_ids = set(
        re.findall(r"(?m)^\| \[ \] \| ((?:[A-J]-\d{2})|(?:KB-\d{2})) \|", roadmap)
    )
    paths = [str(record["path"]) for record in records_to_check]
    if len(paths) != len(set(paths)):
        duplicates = sorted({path for path in paths if paths.count(path) > 1})
        raise ValueError(f"duplicate execution-plan paths: {duplicates}")

    output_relative = OUTPUT.relative_to(ROOT).as_posix()
    expected_current = set(git_files())
    actual_current = {
        str(record["path"])
        for record in records_to_check
        if record["state"] in {"current", "generated"}
    }
    if actual_current != expected_current:
        raise ValueError(
            "current file coverage differs from git/non-ignored inventory: "
            f"missing={sorted(expected_current - actual_current)} "
            f"unexpected={sorted(actual_current - expected_current)}"
        )

    assigned_ids = {
        gate
        for record in records_to_check
        for gate in record.get("planned_gates", [])
        if isinstance(gate, str)
    }
    uncovered = sorted(incomplete_ids - assigned_ids)
    if uncovered:
        raise ValueError(f"unchecked roadmap gates lack a file assignment: {uncovered}")

    for record in records_to_check:
        path = str(record["path"])
        normalized = Path(path).as_posix()
        if (
            normalized != path
            or path.startswith("/")
            or ".." in Path(path).parts
            or path in {"", "."}
        ):
            raise ValueError(f"execution-plan path is not normalized: {path!r}")
        gates = record.get("planned_gates")
        if not isinstance(gates, list) or not gates:
            raise ValueError(f"execution-plan path has no assigned gate: {path}")
        unknown = sorted(set(gates) - roadmap_ids)
        if unknown:
            raise ValueError(f"execution-plan path {path} has unknown gates: {unknown}")
        state = record["state"]
        if state == "current":
            if not {"bytes", "sha256", "coverage"}.issubset(record):
                raise ValueError(
                    f"current path lacks complete baseline evidence: {path}"
                )
        elif state == "planned":
            if (ROOT / path).exists():
                raise ValueError(
                    f"planned path already exists and must be reclassified: {path}"
                )
        elif state == "generated":
            if path != output_relative:
                raise ValueError(f"only the file plan may use generated state: {path}")
        else:
            raise ValueError(f"execution-plan path has invalid state {state!r}: {path}")


def render() -> bytes:
    planned_records = records()
    validate(planned_records)
    return b"".join(
        json.dumps(record, sort_keys=True, separators=(",", ":")).encode("utf-8")
        + b"\n"
        for record in planned_records
    )


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    expected = render()
    if args.check:
        observed = OUTPUT.read_bytes() if OUTPUT.is_file() else b""
        if observed != expected:
            print(
                "execution-inventory: drift; run scripts/ci/build_execution_inventory.py",
                file=sys.stderr,
            )
            return 1
        print(
            f"execution-inventory: OK: {len(expected.splitlines())} current, generated, and planned path records"
        )
        return 0
    OUTPUT.write_bytes(expected)
    print(
        f"execution-inventory: wrote {len(expected.splitlines())} current, generated, and planned path records"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
