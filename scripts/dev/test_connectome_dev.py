"""Unit tests for the development-loop launcher's pure logic."""
from __future__ import annotations

import os
import stat
import sys
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parent))
import connectome_dev as dev  # noqa: E402


def fake_engine(directory: Path) -> Path:
    directory.mkdir(parents=True, exist_ok=True)
    engine = directory / dev.ENGINE_NAME
    engine.write_bytes(b"not a real engine")
    engine.chmod(engine.stat().st_mode | stat.S_IXUSR)
    return engine


class PathTests(unittest.TestCase):
    def test_engine_directory_goes_first_exactly_once(self):
        with tempfile.TemporaryDirectory() as temporary:
            engine = fake_engine(Path(temporary) / "debug")
            other = str(Path(temporary) / "other")
            current = os.pathsep.join([other, str(engine.parent), "", other])
            entries = dev.path_with_engine_first(engine, current).split(os.pathsep)
            self.assertEqual(entries, [str(engine.parent), other, other])

    def test_the_built_engine_is_the_one_path_resolves(self):
        with tempfile.TemporaryDirectory() as temporary:
            built = fake_engine(Path(temporary) / "built")
            stale = fake_engine(Path(temporary) / "stale")
            good = {"PATH": dev.path_with_engine_first(built, str(stale.parent))}
            self.assertEqual(dev.engine_on_path(good), built.resolve())

    def test_report_refuses_when_path_resolves_another_engine(self):
        with tempfile.TemporaryDirectory() as temporary:
            built = fake_engine(Path(temporary) / "built")
            stale = fake_engine(Path(temporary) / "stale")
            shadowed = {"PATH": os.pathsep.join([str(stale.parent), str(built.parent)])}
            with self.assertRaisesRegex(dev.DevLoopError, "not the engine this checkout built"):
                dev.report(built, shadowed)

    def test_report_refuses_when_no_engine_is_on_path(self):
        with tempfile.TemporaryDirectory() as temporary:
            built = fake_engine(Path(temporary) / "built")
            with self.assertRaisesRegex(dev.DevLoopError, "resolves rrflow to None"):
                dev.report(built, {"PATH": ""})


class StagingTests(unittest.TestCase):
    def test_staging_gives_a_single_link_copy_of_a_hard_linked_build(self):
        with tempfile.TemporaryDirectory() as temporary:
            built = fake_engine(Path(temporary) / "debug")
            os.link(built, built.parent / "deps-copy")  # what cargo leaves behind
            self.assertEqual(built.stat().st_nlink, 2)
            staged = dev.stage_engine(built, Path(temporary) / "dev-loop")
            self.assertEqual(staged.stat().st_nlink, 1)
            self.assertEqual(dev.sha256(staged), dev.sha256(built))
            self.assertEqual(sorted(path.name for path in staged.parent.iterdir()), [dev.ENGINE_NAME])

    def test_restaging_replaces_the_previous_engine(self):
        with tempfile.TemporaryDirectory() as temporary:
            built = fake_engine(Path(temporary) / "debug")
            staged = dev.stage_engine(built, Path(temporary) / "dev-loop")
            built.write_bytes(b"a newer build")
            self.assertEqual(dev.sha256(dev.stage_engine(built, staged.parent)), dev.sha256(built))


class ReadinessTests(unittest.TestCase):
    def test_a_missing_submodule_or_dependencies_is_named(self):
        with tempfile.TemporaryDirectory() as temporary:
            connectome = Path(temporary)
            with self.assertRaisesRegex(dev.DevLoopError, "submodule is not checked out"):
                dev.connectome_ready(connectome)
            (connectome / "package.json").write_text("{}", encoding="utf-8")
            with self.assertRaisesRegex(dev.DevLoopError, "dependencies are not installed"):
                dev.connectome_ready(connectome)
            (connectome / "node_modules").mkdir()
            with patch.object(dev.shutil, "which", return_value=None):
                with self.assertRaisesRegex(dev.DevLoopError, "pnpm is not on PATH"):
                    dev.connectome_ready(connectome)

    def test_sha256_is_the_file_digest(self):
        with tempfile.TemporaryDirectory() as temporary:
            path = Path(temporary) / "x"
            path.write_bytes(b"abc")
            self.assertEqual(dev.sha256(path),
                             "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad")


if __name__ == "__main__":
    unittest.main()
