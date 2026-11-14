#!/usr/bin/env python3
"""Native adversarial checks for a qualification-owned Windows image pin."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import time
import unittest

from selected_tests import NATIVE_SAMPLE_HELPER
from windows_harness_image import pin_harness_image, verify_receipt_digest

HARNESS = None


class WindowsHarnessImageTests(unittest.TestCase):
    def setUp(self):
        self.base = Path(tempfile.gettempdir()).resolve(strict=True)
        self.root = Path(tempfile.mkdtemp(prefix="rrflow-image-pin-test-", dir=self.base)).resolve(strict=True)
        self.assertEqual(self.root.parent, self.base)
        self.work = self.root / "work"
        self.work.mkdir()
        self.exe = self.work / "harness.exe"
        shutil.copyfile(HARNESS, self.exe)
        self.digest = hashlib.sha256(self.exe.read_bytes()).hexdigest()

    def tearDown(self):
        self.assertEqual(self.root.resolve(strict=True), self.root)
        self.assertEqual(self.root.parent, self.base)
        self.assertTrue(self.root.name.startswith("rrflow-image-pin-test-"))
        for path in (self.root / "leaf-link", self.root / "worker.pid", self.root / "worker.release",
                     self.work / "harness.exe", self.work / "renamed.exe", self.work / "alias.exe",
                     self.work / "different.exe"):
            if path.exists() or path.is_symlink():
                path.unlink()
        directory_link = self.root / "dir-link"
        if directory_link.is_symlink():
            directory_link.rmdir()
        self.work.rmdir()
        self.root.rmdir()

    def test_live_image_and_ancestors_are_pinned(self):
        marker = self.root / "worker.pid"
        release = self.root / "worker.release"
        request = {"pid": os.getpid(), "harness_pid": os.getpid(), "allocation": None,
                   "hold_marker": str(marker)}
        env = dict(os.environ, RRFLOW_NATIVE_SAMPLE_WORKER_V1=json.dumps(request))
        process = subprocess.Popen([str(self.exe), "--exact", NATIVE_SAMPLE_HELPER,
                                    "--ignored", "--nocapture", "--quiet"],
                                   env=env, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
        try:
            deadline = time.monotonic() + 20
            while not marker.exists():
                self.assertIsNone(process.poll(), "owned helper exited before hold")
                self.assertLess(time.monotonic(), deadline, "owned helper did not hold")
                time.sleep(0.02)
            self.assertEqual(int(marker.read_text()), process.pid)
            renamed = self.work / "renamed.exe"
            try:
                os.replace(self.exe, renamed)
            except OSError as error:
                # A filesystem may already refuse renaming an executing image.
                print(f"unguarded_running_rename=denied winerror={error.winerror}")
            else:
                os.replace(renamed, self.exe)
                print("unguarded_running_rename=allowed")
            with pin_harness_image(self.exe) as digest:
                self.assertEqual(digest, self.digest)
                self.assertIsNone(process.poll())
                with self.assertRaises(OSError):
                    os.replace(self.exe, renamed)
                with self.assertRaises(OSError):
                    os.replace(self.work, self.root / "renamed-parent")
                with self.assertRaises(OSError):
                    fd = os.open(self.exe, os.O_WRONLY)
                    os.close(fd)
                verify_receipt_digest({"harness_sha256": digest}, digest)
                with self.assertRaisesRegex(RuntimeError, "digest differs"):
                    verify_receipt_digest({"harness_sha256": "wrong"}, digest)
        finally:
            release.write_text("release")
            try:
                output, error = process.communicate(timeout=15)
            except subprocess.TimeoutExpired:
                process.terminate()
                process.communicate(timeout=10)
                raise
        self.assertEqual(process.returncode, 0, error.decode(errors="replace")[-2048:])
        self.assertIn(b"rrflow_native_sample_v1=", output)

    def test_existing_hardlink_is_denied(self):
        os.link(self.exe, self.work / "alias.exe")
        with self.assertRaisesRegex(RuntimeError, "exactly one hard link"):
            with pin_harness_image(self.exe):
                pass

    def test_added_hardlink_cannot_replace_pinned_image_with_other_bytes(self):
        alias = self.work / "alias.exe"
        different = self.work / "different.exe"
        different.write_bytes(b"different executable")
        with pin_harness_image(self.exe) as digest:
            os.link(self.exe, alias)
            # NTFS can allow replacing one name with another name for the same
            # file. That operation must retain the same image bytes.
            try:
                os.replace(alias, self.exe)
            except OSError:
                pass  # Denying even this same-file rename is also safe.
            self.assertEqual(hashlib.sha256(self.exe.read_bytes()).hexdigest(), digest)
            with self.assertRaises(OSError):
                os.replace(different, self.exe)
            with self.assertRaises(OSError):
                os.unlink(self.exe)

    def test_preexisting_writer_is_denied(self):
        descriptor = os.open(self.exe, os.O_WRONLY)
        try:
            with self.assertRaises(OSError):
                with pin_harness_image(self.exe):
                    pass
        finally:
            os.close(descriptor)

    def test_alternate_stream_is_denied(self):
        with self.assertRaisesRegex(RuntimeError, "invalid harness image path"):
            with pin_harness_image(Path(str(self.exe) + ":other")):
                pass

    def test_reparse_leaf_and_ancestor_are_denied(self):
        leaf = self.root / "leaf-link"
        parent = self.root / "dir-link"
        try:
            os.symlink(self.exe, leaf)
            os.symlink(self.work, parent, target_is_directory=True)
        except OSError as error:
            if error.winerror == 1314:
                self.skipTest("host does not permit creating a test symlink")
            raise
        for path in (leaf, parent / "harness.exe"):
            with self.subTest(path=path):
                with self.assertRaises((OSError, RuntimeError)):
                    with pin_harness_image(path):
                        pass


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    selection = parser.add_mutually_exclusive_group(required=True)
    selection.add_argument("--harness", type=Path)
    selection.add_argument("--profile", choices=("debug", "release"))
    parser.add_argument("--offline", action="store_true")
    args = parser.parse_args()
    if sys.platform != "win32":
        raise SystemExit("Windows is required")
    if args.profile:
        from test_native_sample_worker import artifact
        HARNESS = artifact(args.profile, args.offline)
    else:
        HARNESS = args.harness
    if not HARNESS.is_absolute() or not HARNESS.is_file():
        raise SystemExit("an absolute existing harness is required")
    unittest.main(argv=[sys.argv[0]])
