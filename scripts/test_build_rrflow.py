#!/usr/bin/env python3
"""Failure oracles for source capture and actual-compilation provenance."""
import copy
from pathlib import Path
import subprocess
import tempfile
import tomllib
import unittest

import build_rrflow as build


class BuildIdentityTests(unittest.TestCase):
    def test_diagnostic_cannot_change_semantic_profile(self):
        manifest = tomllib.loads((build.ROOT / "Cargo.toml").read_text(encoding="utf-8"))
        build.check_profiles(manifest)
        for setting in ("panic", "opt-level", "overflow-checks", "debug-assertions", "lto"):
            changed = copy.deepcopy(manifest)
            changed["profile"]["diagnostic"][setting] = "unapproved"
            with self.assertRaises(ValueError):
                build.check_profiles(changed)

    def test_source_capture_rejects_dirty_and_detects_content_drift(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            def git(*args):
                subprocess.run(["git", *args], cwd=root, check=True, capture_output=True)
            git("init")
            git("config", "user.email", "build-test@example.invalid")
            git("config", "user.name", "Build identity test")
            (root / "Cargo.lock").write_text("locked", encoding="utf-8")
            (root / "source.rs").write_text("before", encoding="utf-8")
            git("add", ".")
            git("commit", "-m", "fixture")
            before = build.source_snapshot(root, False)
            self.assertFalse(before["dirty"])
            (root / "source.rs").write_text("after", encoding="utf-8")
            with self.assertRaisesRegex(ValueError, "dirty"):
                build.source_snapshot(root, False)
            after = build.source_snapshot(root, True)
            self.assertTrue(after["dirty"])
            self.assertNotEqual(before["source_content_sha256"], after["source_content_sha256"])
            with self.assertRaisesRegex(ValueError, "source changed"):
                build.require_same(before, after, "source")
            (root / "untracked.rs").write_text("new", encoding="utf-8")
            self.assertNotEqual(after, build.source_snapshot(root, True))

    def test_actual_features_and_semantic_settings_are_bound(self):
        package = {"name": "rrflow-cli", "version": "1.0.0"}
        message = {
            "reason": "compiler-artifact", "package_id": "local",
            "target": {"name": "rrflow", "kind": ["bin"]},
            "profile": {"opt_level": "3", "debug_assertions": False,
                        "overflow_checks": False, "test": False},
            "features": ["full"],
            "executable": str(Path(tempfile.gettempdir()) / "rrflow-build-fixture"),
        }
        units, _ = build.compilation([message], {"local": package})
        for field, value in (("features", ["other"]), ("profile", {
                **message["profile"], "overflow_checks": True})):
            changed = copy.deepcopy(message)
            changed[field] = value
            other, _ = build.compilation([changed], {"local": package})
            with self.assertRaises(ValueError):
                build.require_same(units, other, "closure")
        with self.assertRaises(ValueError):
            build.compilation([], {"local": package})

    def test_environment_is_digest_only_and_sensitive_to_overrides(self):
        first = build.build_environment({"RUSTFLAGS": "secret-test-value"})
        self.assertEqual(len(first), 64)
        self.assertNotIn("secret", first)
        self.assertNotEqual(first, build.build_environment({"RUSTFLAGS": "other"}))
        self.assertEqual(first, build.build_environment({
            "RUSTFLAGS": "secret-test-value", "UNRELATED_SECRET": "not-recorded"}))


if __name__ == "__main__":
    unittest.main()
