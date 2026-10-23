#!/usr/bin/env python3
"""Mechanical projection of J-04's owning workload ladder; no qualification."""
from __future__ import annotations
from decimal import Decimal
import argparse
import hashlib
import json
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OWNER = "docs/roadmap/rrflow-1.0/gate-j.md"
OUTPUT = ROOT / "eval/capacity-profiles-v1.json"
GIB = 1024**3


def match(pattern, value):
    found = re.fullmatch(pattern, value)
    if found is None:
        raise ValueError("J-04 profile grammar changed; review owning table and projection")
    return found.groups()


def number(value):
    return int(value.replace(",", ""))


def render(text):
    section = text.split("### Initial workload ladder\n", 1)[1].split("\nThese are explicit laboratory", 1)[0]
    payload = re.search(r"(\d+) KiB deterministic record payloads", section)
    if payload is None:
        raise ValueError("missing owning record payload")
    payload_bytes = int(payload[1]) * 1024
    rows = [line for line in section.splitlines() if line.startswith("| ")][1:]
    if len(rows) != 4:
        raise ValueError("capacity projection requires all four owning profile rows")
    profiles = []
    for index, row in enumerate(rows):
        name, allocation, load = [cell.strip() for cell in row.strip("|").split("|")]
        profile = {"id": name.lower().replace(" ", "-"), "name": name}
        if index < 2:
            cpu, host, engine, records, edges, vectors, dimensions = match(
                r"(\d+) vCPU, (\d+) GiB host RAM; engine RSS ceiling (\d+) GiB; (?:one estate with )?([\d,]+) records, ([\d,]+) edges, ([\d,]+) dense (\d+)-dimensional vectors", allocation)
            clients, rates = match(r"([\d/]+) (?:concurrent )?clients; offered ([\d/]+) requests/s; .+", load)
            profile.update(vcpus=int(cpu), host_bytes=int(host)*GIB, engine_bytes=int(engine)*GIB,
                           estates=1, records_min=number(records), edges_min=number(edges),
                           vectors_min=number(vectors), vector_dimensions=int(dimensions),
                           logical_source_bytes_min=number(records)*payload_bytes,
                           clients=[int(v) for v in clients.split("/")], rates=[int(v) for v in rates.split("/")],
                           requires_estate_interference=False)
        elif index == 2:
            cpu, host, engine, estate_word = match(
                r"(\d+) vCPU, (\d+) GiB host RAM; (\d+) GiB aggregate engine RSS ceiling; (\w+) desktop-sized estates, aggregate corpus \4 times the desktop row", allocation)
            words = {word: i for i, word in enumerate("zero one two three four five six seven eight nine ten".split())}
            estates = words[estate_word]
            clients, = match(r"(\d+) clients per estate; measure quiet, equal-load and one abusive estate at 2x its admitted rate; .+", load)
            desktop = profiles[0]
            profile.update(vcpus=int(cpu), host_bytes=int(host)*GIB, engine_bytes=int(engine)*GIB,
                           estates=estates, records_min=desktop["records_min"]*estates,
                           edges_min=desktop["edges_min"]*estates, vectors_min=desktop["vectors_min"]*estates,
                           vector_dimensions=desktop["vector_dimensions"],
                           logical_source_bytes_min=desktop["logical_source_bytes_min"]*estates,
                           clients=[int(clients)*estates], rates=[], requires_estate_interference=True)
        else:
            source, vectors, dimensions = match(
                r"Same single-estate allocation; at least (\d+) GiB measured logical source data and ([\d,]+) dense (\d+)-dimensional vectors; source bytes, on-disk bytes and index bytes reported independently", allocation)
            service = profiles[1]
            ceiling, = match(r"Repeat selective and broad queries, ingestion, compaction, backup and cancellation under the (\d+) GiB engine ceiling; .+", load)
            if int(ceiling)*GIB != service["engine_bytes"]:
                raise ValueError("data-larger-than-memory ceiling conflicts with its allocation")
            profile.update(vcpus=service["vcpus"], host_bytes=service["host_bytes"], engine_bytes=service["engine_bytes"],
                           estates=1, records_min=0, edges_min=0, vectors_min=number(vectors),
                           vector_dimensions=int(dimensions), logical_source_bytes_min=int(source)*GIB,
                           clients=[], rates=[], requires_estate_interference=False)
        profiles.append(profile)
    if [p["id"] for p in profiles] != ["desktop", "single-service-estate", "fleet-isolation", "data-larger-than-memory"]:
        raise ValueError("owning profile coordinates changed")
    threshold_section = text.split("### Proposed pass/fail thresholds\n", 1)[1].split("### Workload and evidence structure", 1)[0]
    normalized = " ".join(threshold_section.split())
    values = re.search(r"At desktop (\d+) requests/s and single-estate (\d+) requests/s, propose point-read p95 <= (\d+) ms, durable-commit p95 <= (\d+) ms, and bounded recall/context p95 <= (\d+) ms, all client end-to-end on loopback, with corresponding p99 ceilings of (\d+)/(\d+)/(\d+) ms", normalized)
    error = re.search(r"unexpected terminal errors must be <= ([0-9.]+)%", normalized)
    if values is None or error is None:
        raise ValueError("J-04 nominal threshold grammar changed")
    desktop_rate, service_rate, point95, write95, range95, point99, write99, range99 = map(int, values.groups())
    error_bps = Decimal(error[1]) * 100
    if error_bps != int(error_bps) or not 0 <= error_bps <= 10000:
        raise ValueError("J-04 error threshold is not exact basis points")
    thresholds = {"nominal_rates": [{"profile": "desktop", "requests_per_second": desktop_rate},
                                     {"profile": "single-service-estate", "requests_per_second": service_rate}],
                  "point_read": {"p95_ns": point95*1000000, "p99_ns": point99*1000000},
                  "mixed_write": {"p95_ns": write95*1000000, "p99_ns": write99*1000000},
                  "range_read": {"p95_ns": range95*1000000, "p99_ns": range99*1000000},
                  "maximum_error_basis_points": int(error_bps)}
    return {"format_version": 1, "owner": OWNER,
            "owner_projection_sha256": hashlib.sha256((section+"\n"+threshold_section).encode()).hexdigest(),
            "record_payload_bytes": payload_bytes, "profiles": profiles, "thresholds": thresholds}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    encoded = (json.dumps(render((ROOT / OWNER).read_text(encoding="utf-8")), indent=2) + "\n").encode()
    if args.check:
        if OUTPUT.read_bytes() != encoded:
            raise SystemExit("capacity profile projection drift; regenerate from J-04")
    else:
        OUTPUT.write_bytes(encoded)
    print("capacity profiles: four J-04 projections; no capacity claim or workload executed")


if __name__ == "__main__":
    main()
