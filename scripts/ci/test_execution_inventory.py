#!/usr/bin/env python3
"""Prove optional Connectome pins never become parent source inputs."""

from __future__ import annotations

import copy
import subprocess
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

import build_execution_inventory as inventory


class OptionalClientInventoryTests(unittest.TestCase):
    PIN = "abc549cf7bb61f2ed4a20353ccb2e3621da515c9"

    def setUp(self) -> None:
        self.temporary = tempfile.TemporaryDirectory(prefix="rrflow-client-inventory-")
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name).resolve()
        self.assertEqual(self.root.parent, Path(tempfile.gettempdir()).resolve())
        self.git("init", "--quiet")
        (self.root / "README.md").write_text("# RRFlow fixture\n", encoding="utf-8")
        gates = ("A-06", "A-07", "D-01", "H-06", "J-01", "J-05", "KB-08")
        (self.root / "roadmap.md").write_text(
            "\n".join(f"| [x] | {gate} | fixture |" for gate in gates) + "\n",
            encoding="utf-8",
        )
        for name, value in (
            ("ROOT", self.root),
            ("OUTPUT", self.root / "file-plan.jsonl"),
            ("ROADMAP", self.root / "roadmap.md"),
            ("PLANNED_PATHS", {}),
        ):
            override = patch.object(inventory, name, value)
            override.start()
            self.addCleanup(override.stop)

    def git(self, *arguments: str) -> subprocess.CompletedProcess[bytes]:
        return subprocess.run(
            ["git", *arguments],
            cwd=self.root,
            check=True,
            capture_output=True,
        )

    def add_client(self) -> None:
        modules = self.root / ".gitmodules"
        modules.write_text(
            '[submodule "connectome"]\n'
            f"\tpath = {inventory.CONNECTOME_PATH}\n"
            f"\turl = {inventory.CONNECTOME_REPOSITORY}\n"
            f"\tbranch = {inventory.CONNECTOME_BRANCH}\n",
            encoding="utf-8",
        )
        self.git("add", ".gitmodules")
        self.git("update-index", "--add", "--cacheinfo", f"160000,{self.PIN},connectome")

    def test_no_submodule_remains_a_supported_engine_checkout(self) -> None:
        self.assertEqual(inventory.gitlinks(), {})
        inventory.validate(inventory.records())

    def test_absent_and_present_client_produce_identical_pin_evidence(self) -> None:
        self.add_client()
        absent = inventory.render()
        client = self.root / "connectome"
        client.mkdir()
        (client / "package.json").write_text("not an engine input", encoding="utf-8")
        (client / "private-state.bin").write_bytes(b"must never be read")
        read_bytes = Path.read_bytes

        def reject_client_reads(path: Path) -> bytes:
            self.assertFalse(path.resolve().is_relative_to(client))
            return read_bytes(path)

        with patch.object(Path, "read_bytes", reject_client_reads):
            present = inventory.render()
        self.assertEqual(absent, present)
        records = inventory.records()
        client_records = [record for record in records if record["path"] == "connectome"]
        self.assertEqual(client_records, [inventory.gitlink_record("connectome", self.PIN)])
        self.assertFalse(any(str(record["path"]).startswith("connectome/") for record in records))

    def test_pin_tampering_cannot_claim_the_current_index_revision(self) -> None:
        self.add_client()
        records = copy.deepcopy(inventory.records())
        for record in records:
            if record["path"] == "connectome":
                record["git_revision"] = "1" * 40
        with self.assertRaisesRegex(ValueError, "does not match the gitlink"):
            inventory.validate(records)

    def test_other_gitlinks_and_unmerged_pins_are_rejected(self) -> None:
        self.add_client()
        self.git("update-index", "--add", "--cacheinfo", f"160000,{self.PIN},foreign")
        with self.assertRaisesRegex(ValueError, "only the optional connectome gitlink"):
            inventory.gitlinks()
        self.git("update-index", "--force-remove", "foreign")
        self.git("update-index", "--force-remove", "connectome")
        subprocess.run(
            ["git", "update-index", "--index-info"],
            cwd=self.root,
            input=f"160000 {self.PIN} 1\tconnectome\n".encode(),
            check=True,
            capture_output=True,
        )
        with self.assertRaisesRegex(ValueError, "only the optional connectome gitlink"):
            inventory.gitlinks()

    def test_exact_repository_branch_path_and_plain_update_policy_are_required(self) -> None:
        self.add_client()
        modules = self.root / ".gitmodules"
        canonical = modules.read_text(encoding="utf-8")
        for replacement in (
            canonical.replace("path = connectome", "path = apps/connectome"),
            canonical.replace(inventory.CONNECTOME_REPOSITORY, "https://example.com/client.git"),
            canonical.replace(inventory.CONNECTOME_BRANCH, "main"),
            canonical.replace(f"\tbranch = {inventory.CONNECTOME_BRANCH}\n", ""),
            canonical + "\tupdate = !custom-command\n",
            canonical + "\tpath = connectome\n",
            canonical + '[submodule "foreign"]\n\tpath = foreign\n',
        ):
            with self.subTest(declaration=replacement):
                modules.write_text(replacement, encoding="utf-8")
                with self.assertRaisesRegex(ValueError, "only the canonical connectome repository"):
                    inventory.gitlinks()
        modules.write_text(canonical, encoding="utf-8")

    def test_missing_declaration_and_orphan_declaration_fail_closed(self) -> None:
        self.add_client()
        (self.root / ".gitmodules").unlink()
        with self.assertRaisesRegex(ValueError, "requires canonical .gitmodules"):
            inventory.gitlinks()
        self.add_client()
        self.git("update-index", "--force-remove", "connectome")
        with self.assertRaisesRegex(ValueError, "only the canonical connectome repository"):
            inventory.gitlinks()


if __name__ == "__main__":
    unittest.main()
