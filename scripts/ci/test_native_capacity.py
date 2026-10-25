"""Capacity orchestration/cleanup checks; no native disk is created or filled."""
from contextlib import contextmanager, redirect_stdout
import io
import json
import plistlib
import subprocess
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

import run_native_capacity as runner
import run_filesystem_containment as containment


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


class OwnedImageDetachment(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix="rrflow-detach-")
        self.addCleanup(self.temp.cleanup)
        self.image = Path(self.temp.name).resolve() / "capacity.dmg"
        self.image.write_bytes(b"owned fixture")
        self.clock = 0.0
        self.calls = []
        self.sleeps = []

    def info(self, devices=("/dev/disk7",), duplicate=False):
        rows = [{"image-path": str(self.image), "system-entities": [
            {"dev-entry": device} for device in devices]}] if devices else []
        if duplicate:
            rows *= 2
        return subprocess.CompletedProcess([], 0, plistlib.dumps({"images": rows}).decode(), "")

    def detach(self, code=0):
        return subprocess.CompletedProcess([], code, "", "native busy" if code == 16 else "native failure")

    def invoke(self, responses, advance=0.1):
        if callable(responses):
            respond = responses
        else:
            iterator = iter(responses)
            def respond(*_):
                return next(iterator)
        def run(*args, **kwargs):
            self.calls.append(args)
            self.assertTrue(kwargs["process_group"])
            self.assertFalse(kwargs["check"])
            self.assertGreater(kwargs["timeout"], 0)
            self.assertLessEqual(kwargs["timeout"], 30)
            self.clock += advance
            return respond(*args)
        def sleep(seconds):
            self.sleeps.append(seconds)
            self.clock += seconds
        with patch.object(containment, "run", side_effect=run), \
             patch.object(containment.time, "monotonic", side_effect=lambda: self.clock), \
             patch.object(containment.time, "sleep", side_effect=sleep):
            containment.detach_owned_image(self.image)

    def test_success_requires_verified_absence(self):
        self.invoke([self.info(), self.detach(), self.info(())])
        self.assertEqual([x[1] for x in self.calls], ["info", "detach", "info"])
        self.assertEqual(self.image.read_bytes(), b"owned fixture")

    def test_busy_rechecks_ownership_before_retry(self):
        self.invoke([self.info(), self.detach(16), self.info(), self.detach(), self.info(())])
        self.assertEqual([x[1] for x in self.calls], ["info", "detach", "info", "detach", "info"])
        self.assertEqual([x[-1] for x in self.calls if x[1] == "detach"], ["/dev/disk7"] * 2)

    def test_busy_response_can_be_resolved_by_confirmed_absence(self):
        self.invoke([self.info(), self.detach(16), self.info(())])
        self.assertEqual(len(self.calls), 3)

    def test_busy_attachment_can_settle_within_existing_deadline(self):
        def response(*args):
            if args[1] == "info":
                return self.info(()) if self.clock >= 2.5 else self.info()
            self.assertEqual(args[1:], ("detach", "-force", "/dev/disk7"))
            return self.detach(16)
        self.invoke(response)
        self.assertEqual(self.sleeps, [1, 2])
        self.assertEqual(sum(x[1] == "detach" for x in self.calls), 2)
        self.assertLess(self.clock, 30)
        self.assertTrue(self.image.exists(), "settlement never authorizes image deletion here")

    def test_nonbusy_failure_keeps_native_diagnostics_and_never_retries(self):
        with self.assertRaisesRegex(RuntimeError, r"hdiutil detach \(5\): native failure"):
            self.invoke([self.info(), self.detach(5)])
        self.assertEqual(len(self.calls), 2)
        self.assertTrue(self.image.exists())

    def test_changed_or_ambiguous_device_is_never_detached(self):
        with self.assertRaisesRegex(RuntimeError, "identity changed"):
            self.invoke([self.info(), self.detach(16), self.info(("/dev/disk9",))])
        self.assertEqual(len(self.calls), 3)
        self.calls.clear()
        with self.assertRaisesRegex(RuntimeError, "ambiguous"):
            self.invoke([self.info(duplicate=True)])
        self.assertEqual(len(self.calls), 1)

    def test_busy_retries_are_finite_and_keep_fixture(self):
        with self.assertRaisesRegex(RuntimeError, "bounded cleanup.*native busy"):
            self.invoke([item for _ in range(5) for item in (self.info(), self.detach(16))] + [self.info()])
        self.assertEqual(sum(x[1] == "detach" for x in self.calls), 5)
        self.assertEqual(self.sleeps, [1, 2, 4, 8])
        self.assertLess(self.clock, 30)
        self.assertTrue(self.image.exists())

    def test_deadline_is_shared_across_all_commands(self):
        with self.assertRaisesRegex(RuntimeError, "cleanup deadline"):
            self.invoke([self.info(), self.detach(16), self.info()], advance=15)
        self.assertEqual(len(self.calls), 2)

    def test_success_with_attachment_still_present_is_denied(self):
        with self.assertRaisesRegex(RuntimeError, "remains attached"):
            self.invoke([self.info(), self.detach(), self.info()])
        self.assertEqual(len(self.calls), 3)
        self.assertTrue(self.image.exists())

    def test_missing_device_and_failed_inspection_never_detach(self):
        with self.assertRaisesRegex(RuntimeError, "whole-device identity"):
            self.invoke([self.info(("/dev/disk7s1",))])
        self.calls.clear()
        with self.assertRaisesRegex(RuntimeError, "inspection failed"):
            self.invoke([self.detach(5)])
        self.assertEqual(len(self.calls), 1)


if __name__ == "__main__":
    unittest.main()
