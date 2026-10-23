#!/usr/bin/env python3
"""Pure outer-receipt tests: no process allocation, engine or soak is launched."""
import json
from pathlib import Path
import tempfile
import unittest
from run_sustained_capacity import NotRun, retain_launch


class LaunchTests(unittest.TestCase):
    def test_failure_interruption_and_unavailable_preserve_unsealed_evidence(self):
        for error, status in [(RuntimeError("cleanup failure"), "failed"),
                              (KeyboardInterrupt(), "interrupted"), (NotRun("no delegated controller"), "not_run")]:
            with tempfile.TemporaryDirectory() as temporary:
                root = Path(temporary) / "run"
                with self.assertRaises(type(error)):
                    with retain_launch(root, {"frozen": True}, "a" * 64):
                        raise error
                receipt = json.loads((root / "launch.json").read_text())
                self.assertEqual(receipt["status"], status)
                self.assertFalse(receipt["cleanup_confirmed"])
                self.assertIsNone(receipt["exit_code"])
                self.assertFalse((root / "result.json").exists())
                self.assertEqual(json.loads((root / "campaign.json").read_text()), {"frozen": True})

    def test_inner_success_does_not_override_outer_failure(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary) / "run"
            with self.assertRaises(RuntimeError):
                with retain_launch(root, {"frozen": True}, "a" * 64) as state:
                    root.mkdir()
                    (root / "result.json").write_text('{"status":"completed"}')
                    state["exit_code"] = 0
                    raise RuntimeError("native cleanup failed")
            receipt = json.loads((root / "launch.json").read_text())
            self.assertEqual(receipt["status"], "failed")
            self.assertEqual(receipt["reason"], "native-cleanup-unconfirmed")
            self.assertEqual(json.loads((root / "result.json").read_text())["status"], "completed")

    def test_final_receipt_is_exclusive_and_preserves_existing_campaign(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary) / "run"
            root.mkdir()
            (root / "campaign.json").write_text('{"retained":true}')
            with retain_launch(root, {"frozen": True}, "a" * 64) as state:
                state.update(status="completed", exit_code=0, cleanup_confirmed=True, reason=None)
            before = (root / "launch.json").read_bytes()
            self.assertEqual(json.loads(before)["status"], "completed")
            with self.assertRaises(FileExistsError):
                with retain_launch(root, {}, "b" * 64):
                    pass
            self.assertEqual((root / "launch.json").read_bytes(), before)
            self.assertEqual((root / "campaign.json").read_text(), '{"retained":true}')


if __name__ == "__main__":
    unittest.main()
