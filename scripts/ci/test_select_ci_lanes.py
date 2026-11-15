#!/usr/bin/env python3
"""Regressions for the proportional candidate CI lane selection."""

import tempfile
import tomllib
import unittest
from pathlib import Path

import re

from select_ci_lanes import (LANES, PINNED_BY, ROOT, TEST_BINARIES, select, with_dependents,
                             workspace_graph)
from workspace_packages import member_directories

METADATA = {
    "workspace_members": ["core", "lsm", "engine", "cli", "eval", "lab"],
    "packages": [
        {"id": "core", "name": "rrd-core", "dependencies": []},
        {"id": "lsm", "name": "rrd-lsm", "dependencies": [{"name": "rrd-core"}]},
        {"id": "engine", "name": "rrd-engine", "dependencies": [{"name": "rrd-core"}, {"name": "rrd-lsm"}]},
        {"id": "cli", "name": "rrflow-cli", "dependencies": [{"name": "rrd-engine"}, {"name": "serde"}]},
        {"id": "eval", "name": "rrflow-eval", "dependencies": [{"name": "rrd-core"}]},
        {"id": "lab", "name": "rrflow-lab-core", "dependencies": []},
    ],
}


class LaneSelectionTests(unittest.TestCase):
    def setUp(self):
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        self.root = Path(temporary.name)
        members = [("kernel", "rrd-core"), ("persistence", "rrd-lsm"), ("authority", "rrd-engine"),
                   ("adapters", "rrflow-cli"), ("evaluation", "rrflow-eval"), ("transport", "rrd-contract"),
                   ("evaluation/lab", "rrflow-lab-core")]
        for group, crate in members:
            manifest = self.root / "crates" / group / crate / "Cargo.toml"
            manifest.parent.mkdir(parents=True)
            manifest.write_text(f'[package]\nname = "{crate}"\nversion = "1.0.0"\n', encoding="utf-8")
        # A fuzz package nested inside a member is not itself a member.
        fuzz = self.root / "crates" / "evaluation" / "rrflow-eval" / "fuzz" / "Cargo.toml"
        fuzz.parent.mkdir(parents=True)
        fuzz.write_text('[package]\nname = "rrflow-eval-fuzz"\nversion = "0.0.0"\n', encoding="utf-8")
        listed = ", ".join(f'"crates/{group}/{crate}"' for group, crate in members)
        (self.root / "Cargo.toml").write_text(f"[workspace]\nmembers = [{listed}]\n", encoding="utf-8")

    def lanes(self, *paths, **options):
        result = select(list(paths), metadata=METADATA, root=self.root, **options)
        return {lane for lane in LANES if result[lane]}, result["packages"]

    def dependents(self, *paths):
        return select(list(paths), metadata=METADATA, root=self.root)["dependents"]

    def test_documentation_only_runs_the_documentation_lane(self):
        selected, packages = self.lanes("docs/roadmap/rrflow-1.0.md", "README.md", ".github/ISSUE_TEMPLATE/bug.yml")
        self.assertEqual(selected, {"docs"})
        self.assertEqual(packages, "")

    def test_script_edit_runs_the_script_lane(self):
        selected, _ = self.lanes("scripts/inference/replicate_weight_edit.py", "eval/README.md")
        self.assertEqual(selected, {"scripts", "docs"})

    def test_crate_edit_tests_the_crate_and_compiles_its_dependents(self):
        selected, packages = self.lanes("crates/evaluation/rrflow-eval/src/lab/mod.rs")
        self.assertEqual(selected, {"engine"})
        self.assertEqual(packages, "-p rrflow-eval")
        self.assertEqual(self.dependents("crates/evaluation/rrflow-eval/src/lab/mod.rs"), "")
        selected, packages = self.lanes("crates/authority/rrd-engine/src/engine/vector/search.rs")
        self.assertEqual(selected, {"engine"})
        self.assertEqual(packages, "-p rrd-engine")
        self.assertEqual(self.dependents("crates/authority/rrd-engine/src/engine/vector/search.rs"), "-p rrflow-cli")

    def test_workspace_inputs_compile_the_whole_workspace_without_the_full_matrix(self):
        for path in ("Cargo.toml", "Cargo.lock", "rust-toolchain.toml", ".cargo/config.toml"):
            with self.subTest(path=path):
                result = select([path], metadata=METADATA, root=self.root)
                self.assertEqual({lane for lane in LANES if result[lane]}, {"engine"})
                self.assertEqual(result["packages"], "")
                self.assertEqual(result["dependents"], "--workspace")

    def test_sdk_edit_runs_conformance_without_an_empty_engine_package_lane(self):
        selected, packages = self.lanes("sdks/python/src/rrd_client/operation.py")
        self.assertEqual(selected, {"sdk"})
        self.assertEqual(packages, "")

    def test_transport_crate_edit_runs_conformance_too(self):
        selected, packages = self.lanes("crates/transport/rrd-contract/src/lib.rs")
        self.assertEqual(selected, {"sdk", "engine"})
        self.assertEqual(packages, "-p rrd-contract -p rrflow-qualification-storage")

    def test_a_crate_that_pins_the_changed_crates_identity_is_tested_not_only_compiled(self):
        # The storage qualification fixture records the public catalogue's
        # digest. Compiling that crate cannot see the fixture go stale.
        result = select(["crates/transport/rrd-contract/src/catalogue.rs"], metadata=METADATA, root=self.root)
        self.assertIn("-p rrflow-qualification-storage", result["packages"])
        self.assertNotIn("rrflow-qualification-storage", result["dependents"])
        # A crate that pins nothing selects nothing extra.
        self.assertEqual(self.lanes("crates/persistence/rrd-lsm/src/lib.rs")[1], "-p rrd-lsm")

    def test_a_crate_whose_tests_run_a_workspace_binary_has_it_built_first(self):
        result = select(["crates/transport/rrd-server/src/http/maintenance.rs"],
                        metadata=METADATA, root=ROOT)
        self.assertEqual(result["binaries"], "-p rrflow-cli --bin rrd-estate-controller")
        self.assertEqual(select(["crates/persistence/rrd-lsm/src/lib.rs"], metadata=METADATA,
                                root=self.root)["binaries"], "")

    def test_every_test_that_runs_a_workspace_binary_is_declared(self):
        # `workspace_binary("name")` is how a test finds another package's built
        # binary. Each crate that does so must name it in TEST_BINARIES.
        used: dict[str, set[str]] = {}
        for directory, package in member_directories(ROOT).items():
            for source in (ROOT / directory).rglob("*.rs"):
                for binary in re.findall(r'workspace_binary\("([^"]+)"\)', source.read_text(encoding="utf-8")):
                    used.setdefault(package, set()).add(binary)
        self.assertTrue(used, "no test uses workspace_binary any more; remove TEST_BINARIES")
        for package, binaries in used.items():
            with self.subTest(package=package):
                self.assertIn(package, TEST_BINARIES)
                self.assertEqual(binaries, {TEST_BINARIES[package][1]})
        members = set(member_directories(ROOT).values())
        for package, (owner, _) in TEST_BINARIES.items():
            self.assertIn(package, used, f"{package} no longer runs a workspace binary")
            self.assertIn(owner, members)

    def test_every_pinning_crate_is_a_workspace_member_that_depends_on_its_owner(self):
        members = {name: directory for directory, name in member_directories(ROOT).items()}
        self.assertTrue(PINNED_BY)
        for owner, pinners in PINNED_BY.items():
            self.assertIn(owner, members)
            for pinner in pinners:
                with self.subTest(owner=owner, pinner=pinner):
                    self.assertIn(pinner, members)
                    with (ROOT / members[pinner] / "Cargo.toml").open("rb") as source:
                        manifest = tomllib.load(source)
                    self.assertIn(owner, manifest.get("dependencies", {}),
                                  f"{pinner} no longer depends on {owner}; remove it from PINNED_BY")

    def test_no_changed_path_selects_the_full_matrix(self):
        # The full matrix is release qualification: nightly and on demand only.
        for path in (
            "crates/persistence/rrd-lsm/src/database.rs",
            "crates/kernel/rrd-core/src/lib.rs",
            "crates/adapters/rrflow-cli/src/main.rs",
            "crates/authority/rrd-engine/src/engine/transaction.rs",
            "crates/authority/rrd-engine/src/engine/installation/admission.rs",
            "crates/authority/rrd-engine/src/engine/tests/recovery.rs",
            ".github/workflows/ci-reusable.yml",
            "scripts/ci/run_native_capacity.py",
            "rust-toolchain.toml",
            "Cargo.lock",
            "connectome",
            "deploy/compose.yml",
            ".gitignore",
            "tools/new-thing.toml",
        ):
            with self.subTest(path=path):
                self.assertNotIn("full", self.lanes(path, "docs/README.md")[0], path)

    def test_durable_engine_and_storage_edits_test_their_crate(self):
        selected, packages = self.lanes("crates/persistence/rrd-lsm/src/database.rs")
        self.assertEqual(selected, {"engine"})
        self.assertEqual(packages, "-p rrd-lsm")
        self.assertEqual(self.dependents("crates/persistence/rrd-lsm/src/database.rs"), "-p rrd-engine -p rrflow-cli")
        selected, packages = self.lanes("crates/authority/rrd-engine/src/engine/transaction.rs",
                                        "crates/authority/rrd-engine/src/engine/tests/recovery.rs")
        self.assertEqual(selected, {"engine"})
        self.assertEqual(packages, "-p rrd-engine")
        selected, packages = self.lanes("crates/adapters/rrflow-cli/src/main.rs")
        self.assertEqual(selected, {"engine"})
        self.assertEqual(packages, "-p rrflow-cli")

    def test_workflow_ci_script_and_policy_files_run_the_script_lane(self):
        for path in (".github/workflows/ci-reusable.yml", "scripts/ci/run_native_capacity.py", ".gitignore",
                     ".gitattributes", ".gitmodules", "connectome", "deploy/compose.yml", "VERSION",
                     ".devcontainer/devcontainer.json"):
            with self.subTest(path=path):
                self.assertEqual(self.lanes(path)[0], {"scripts"}, path)

    def test_unknown_path_runs_every_light_lane_and_compiles_the_workspace(self):
        result = select(["tools/new-thing.toml"], metadata=METADATA, root=self.root)
        self.assertEqual({lane for lane in LANES if result[lane]}, {"docs", "scripts", "engine", "sdk"})
        self.assertEqual(result["dependents"], "--workspace")
        self.assertEqual(result["unknown"], ["tools/new-thing.toml"])

    def test_a_nested_member_is_its_own_package_not_an_unknown_path(self):
        selected, packages = self.lanes("crates/evaluation/lab/rrflow-lab-core/src/lib.rs")
        self.assertEqual(selected, {"engine"})
        self.assertEqual(packages, "-p rrflow-lab-core")

    def test_a_nested_non_member_belongs_to_the_member_around_it(self):
        selected, packages = self.lanes("crates/evaluation/rrflow-eval/fuzz/fuzz_targets/manifest.rs")
        self.assertEqual(selected, {"engine"})
        self.assertEqual(packages, "-p rrflow-eval")
        self.assertNotIn("fuzz", packages)

    def test_unknown_crate_directory_compiles_the_workspace(self):
        result = select(["crates/compute/not-a-crate/src/lib.rs"], metadata=METADATA, root=self.root)
        self.assertEqual({lane for lane in LANES if result[lane]}, {"engine"})
        self.assertEqual(result["dependents"], "--workspace")
        self.assertEqual(result["unknown"], ["crates/compute/not-a-crate/src/lib.rs"])

    def test_only_a_forced_selection_is_the_full_matrix(self):
        self.assertEqual(self.lanes()[0], set())
        self.assertEqual(self.lanes("README.md", force_full=True)[0], {"full"})
        self.assertEqual(self.lanes("crates/persistence/rrd-lsm/src/lib.rs", force_full=True), ({"full"}, ""))

    def test_dependents_close_over_the_workspace_only(self):
        graph = workspace_graph(METADATA)
        self.assertEqual(with_dependents({"rrd-core"}, graph),
                         {"rrd-core", "rrd-lsm", "rrd-engine", "rrflow-cli", "rrflow-eval"})
        self.assertEqual(with_dependents({"rrflow-cli"}, graph), {"rrflow-cli"})
        self.assertNotIn("serde", graph)

    def test_windows_separators_are_normalized(self):
        selected, packages = self.lanes("crates\\evaluation\\rrflow-eval\\src\\lib.rs")
        self.assertEqual(selected, {"engine"})
        self.assertEqual(packages, "-p rrflow-eval")


if __name__ == "__main__":
    unittest.main()
