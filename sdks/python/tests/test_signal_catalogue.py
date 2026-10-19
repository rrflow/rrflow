from __future__ import annotations

import json

from rrd_client import OPENAPI_DOCUMENT_SHA256, SIGNAL_CATALOGUE_JSON, SIGNAL_CATALOGUE_SHA256

EXPECTED_OPENAPI_SHA256 = "7dc64c487dd6930cf99751cbca84e8fc1debc7ad43c9c4b22e847b5138077ede"
EXPECTED_SIGNAL_SHA256 = "239df2ced5974d4351ca01565369646964cb65c63d0fbae01dddccb4c3ac5a07"


def test_generated_signal_catalogue_is_public_exact_and_inert() -> None:
    catalogue = json.loads(SIGNAL_CATALOGUE_JSON)

    assert SIGNAL_CATALOGUE_SHA256 == EXPECTED_SIGNAL_SHA256
    assert OPENAPI_DOCUMENT_SHA256 == EXPECTED_OPENAPI_SHA256
    assert catalogue["contract_version"] == 1
    assert len(catalogue["diagnostic_levels"]) == 4
    assert len(catalogue["metric_attributes"]) == 9
    assert len(catalogue["metric_instruments"]) == 22
    forbidden = {
        "activation",
        "active_level",
        "current_level",
        "diagnostic_snapshot",
        "enabled",
        "events",
        "exporter",
        "lifecycle",
        "samples",
        "subscriber",
        "timestamp",
    }
    assert forbidden.isdisjoint(catalogue)
