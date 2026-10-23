#!/usr/bin/env python3
"""Owning table projection characterization, without engine execution."""
import unittest

import render_capacity_profiles as projection


class CapacityProfiles(unittest.TestCase):
    def setUp(self):
        self.source = (projection.ROOT / projection.OWNER).read_text(encoding="utf-8")

    def test_allocation_corpus_and_inheritance(self):
        result = projection.render(self.source)
        desktop, service, fleet, larger = result["profiles"]
        self.assertEqual(desktop["clients"], [1, 4, 8])
        self.assertEqual(service["rates"], [25, 100, 250])
        self.assertEqual(fleet["records_min"], desktop["records_min"] * fleet["estates"])
        self.assertEqual(larger["engine_bytes"], service["engine_bytes"])
        self.assertEqual(larger["logical_source_bytes_min"], 32 * projection.GIB)

    def test_owner_edit_changes_projection_identity_and_value(self):
        before = projection.render(self.source)
        after = projection.render(self.source.replace("4 vCPU, 8 GiB", "6 vCPU, 8 GiB"))
        self.assertNotEqual(before["owner_projection_sha256"], after["owner_projection_sha256"])
        self.assertEqual(after["profiles"][0]["vcpus"], 6)

    def test_nominal_thresholds_are_projected_and_digest_bound(self):
        before = projection.render(self.source)
        after = projection.render(self.source.replace("p95 <= 50 ms", "p95 <= 60 ms"))
        self.assertEqual(before["thresholds"]["point_read"]["p95_ns"], 50_000_000)
        self.assertEqual(before["thresholds"]["maximum_error_basis_points"], 10)
        self.assertEqual(after["thresholds"]["point_read"]["p95_ns"], 60_000_000)
        self.assertNotEqual(before["owner_projection_sha256"], after["owner_projection_sha256"])
        with self.assertRaises(ValueError):
            projection.render(self.source.replace("unexpected terminal errors must be <= 0.1%", "errors are fine"))

    def test_unknown_or_inconsistent_grammar_is_not_silently_dropped(self):
        for source in (
            self.source.replace("| Desktop |", "| Other |"),
            self.source.replace("4 vCPU, 8 GiB", "4 cores, 8 GiB"),
            self.source.replace("under the 8 GiB engine ceiling", "under the 9 GiB engine ceiling"),
        ):
            with self.assertRaises(ValueError):
                projection.render(source)


if __name__ == "__main__":
    unittest.main()
