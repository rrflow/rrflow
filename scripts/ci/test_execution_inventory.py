#!/usr/bin/env python3
"""Prove parent inventory never depends on an optional submodule checkout."""

from __future__ import annotations

import subprocess
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

import build_execution_inventory as inventory


class GitlinkInventoryTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        self.git("init", "--quiet")
        self.output = self.root / "inventory.jsonl"
        self.output.write_text("", encoding="utf-8")
        for name, value in (("ROOT", self.root), ("OUTPUT", self.output), ("PLANNED_PATHS", {})):
            patcher = patch.object(inventory, name, value)
            patcher.start()
            self.addCleanup(patcher.stop)

    def git(self, *args: str, input: bytes | None = None) -> bytes:
        return subprocess.run(
            ["git", *args], cwd=self.root, input=input, check=True, capture_output=True
        ).stdout

    def pin(self, revision: str) -> None:
        self.git("update-index", "--add", "--cacheinfo", f"160000,{revision},connectome")

    def test_uninitialized_populated_and_dirty_child_have_identical_parent_records(self) -> None:
        self.pin("1" * 40)
        absent = inventory.records()
        child = self.root / "connectome"
        child.mkdir()
        self.assertEqual(absent, inventory.records())
        (child / "README.md").write_text("child source", encoding="utf-8")
        (child / "secret.txt").write_text("must not enter parent inventory", encoding="utf-8")
        self.assertEqual(absent, inventory.records())
        (child / "README.md").write_text("uncommitted child edit", encoding="utf-8")
        self.assertEqual(absent, inventory.records())
        link = next(record for record in absent if record["path"] == "connectome")
        self.assertEqual("git-submodule", link["kind"])
        self.assertEqual("1" * 40, link["git_revision"])
        self.assertIsNone(link["lines"])

    def test_pin_change_changes_revision_and_digest(self) -> None:
        self.pin("1" * 40)
        before = inventory.current_record("connectome", inventory.gitlinks())
        self.pin("2" * 40)
        after = inventory.current_record("connectome", inventory.gitlinks())
        self.assertNotEqual(before["git_revision"], after["git_revision"])
        self.assertNotEqual(before["sha256"], after["sha256"])
        self.assertEqual(before["bytes"], after["bytes"])

    def test_regular_file_still_tracks_bytes(self) -> None:
        path = self.root / "README.md"
        path.write_bytes(b"first\n")
        before = inventory.current_record("README.md", inventory.gitlinks())
        path.write_bytes(b"second\n")
        after = inventory.current_record("README.md", inventory.gitlinks())
        self.assertEqual("documentation", before["kind"])
        self.assertEqual(1, before["lines"])
        self.assertNotEqual(before["sha256"], after["sha256"])

    def test_unmerged_gitlink_is_rejected(self) -> None:
        conflict = f"160000 {'1' * 40} 1\tconnectome\n160000 {'2' * 40} 2\tconnectome\n"
        self.git("update-index", "--index-info", input=conflict.encode())
        with self.assertRaisesRegex(ValueError, "unmerged index entry: connectome"):
            inventory.gitlinks()


if __name__ == "__main__":
    unittest.main()
