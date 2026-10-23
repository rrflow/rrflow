"""Capacity orchestration/cleanup checks; no native disk is created or filled."""
from contextlib import contextmanager, redirect_stdout
import io
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

import run_native_capacity as runner


class NativeCapacityCases(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix="rrflow-capacity-")
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name).resolve(strict=True)
        self.events = []
        self.generation = 0
        self.config = {"size_mib": 256, "modes": ["fresh", "existing"],
                       "points": list(runner.POINTS), "executable": "unused", "harness_sha256": "test"}

    @contextmanager
    def volume(self, fixture, size):
        self.assertEqual(fixture, self.root)
        self.assertEqual(size, 256)
        self.assertFalse((fixture / "capacity.img").exists(), "prior image was reused")
        self.generation += 1
        self.events.append(("attach", self.generation))
        (fixture / "capacity.img").write_bytes(b"owned image placeholder")
        (fixture / "volume").mkdir()
        descriptor = fixture / "volume.json"
        descriptor.write_text("{}", encoding="utf-8")
        try:
            yield descriptor, {"volume_uuid": str(self.generation)}
        finally:
            self.events.append(("detach", self.generation))

    def execute(self, executable, descriptor, point, mode):
        self.assertEqual(self.events[-1], ("attach", self.generation))
        self.assertTrue(descriptor.is_file())
        self.events.append(("execute", self.generation))
        return {"build": "test", "expected_sha256": f"{mode}:{point}"}

    def test_each_case_gets_a_new_image_and_identity_after_prior_cleanup(self):
        output = io.StringIO()
        with patch.object(runner, "volume", self.volume), patch.object(runner, "execute", self.execute), redirect_stdout(output):
            runner.cases(self.config, self.root)
        receipt = json.loads(output.getvalue().removeprefix("native_capacity_suite="))
        self.assertEqual(receipt["version"], 2)
        self.assertEqual(len(receipt["cases"]), 8)
        self.assertEqual([case["filesystem"]["volume_uuid"] for case in receipt["cases"]], list(map(str, range(1, 9))))
        self.assertEqual(self.events, [(event, index) for index in range(1, 9) for event in ("attach", "execute", "detach")])
        self.assertEqual(list(self.root.iterdir()), [])

    def test_failure_stops_later_cases_and_retains_failed_image(self):
        def fail(executable, descriptor, point, mode):
            if self.generation == 2:
                raise RuntimeError("selected native error missing")
            return self.execute(executable, descriptor, point, mode)
        with patch.object(runner, "volume", self.volume), patch.object(runner, "execute", fail), redirect_stdout(io.StringIO()) as output:
            with self.assertRaisesRegex(RuntimeError, "selected native error missing"):
                runner.cases(self.config, self.root)
        self.assertEqual(self.generation, 2)
        self.assertEqual(self.events[-1], ("detach", 2))
        self.assertTrue((self.root / "capacity.img").is_file())
        self.assertNotIn("native_capacity_suite=", output.getvalue())

    def test_cleanup_refuses_mounted_nonempty_or_invalid_artifacts_before_deletion(self):
        image = self.root / "capacity.img"
        image.write_bytes(b"preserve")
        mount = self.root / "volume"
        mount.mkdir()
        with patch.object(runner.os.path, "ismount", return_value=True):
            with self.assertRaisesRegex(Exception, "attached mount"):
                runner.cleanup_detached_volume(self.root)
        self.assertEqual(image.read_bytes(), b"preserve")
        marker = mount / "not-detached"
        marker.write_bytes(b"preserve")
        with self.assertRaisesRegex(Exception, "not empty"):
            runner.cleanup_detached_volume(self.root)
        self.assertEqual(image.read_bytes(), b"preserve")
        marker.unlink()
        (self.root / "volume.json").mkdir()
        with self.assertRaisesRegex(Exception, "escaped fixture"):
            runner.cleanup_detached_volume(self.root)
        self.assertEqual(image.read_bytes(), b"preserve")

    def test_detachment_failure_retains_image_and_prevents_next_case(self):
        @contextmanager
        def failed_detach(fixture, size):
            with self.volume(fixture, size) as owned:
                yield owned
            raise RuntimeError("native detachment failed")
        with patch.object(runner, "volume", failed_detach), patch.object(runner, "execute", self.execute), redirect_stdout(io.StringIO()) as output:
            with self.assertRaisesRegex(RuntimeError, "native detachment failed"):
                runner.cases(self.config, self.root)
        self.assertEqual(self.generation, 1)
        self.assertTrue((self.root / "capacity.img").is_file())
        self.assertNotIn("native_capacity_suite=", output.getvalue())


if __name__ == "__main__":
    unittest.main()
