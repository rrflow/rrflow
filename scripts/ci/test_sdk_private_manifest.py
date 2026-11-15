"""Native filesystem checks for the SDK harness rendezvous boundary."""

import contextlib
import io
import os
import sys
from pathlib import Path
import tempfile
import unittest

from sdk_private_manifest import create_manifest_staging, private_workspace, verify_manifest
from run_sdk_conformance import run, wait_for_manifest


class PrivateManifestTests(unittest.TestCase):
    def test_created_child_is_private(self):
        with private_workspace() as root:
            path = create_manifest_staging(root) if os.name == "nt" else root / "manifest.json"
            with path.open("w" if os.name == "nt" else "x", encoding="utf-8") as file:
                file.write("{}")
            if os.name == "posix":
                path.chmod(0o600)
            verify_manifest(path)
            self.assertEqual(wait_for_manifest(path, None), {})
        self.assertFalse(root.exists())

    @unittest.skipUnless(os.name == "posix", "POSIX permission test")
    def test_posix_rejects_group_or_world_access(self):
        with private_workspace() as root:
            path = root / "manifest.json"
            path.write_text("{}", encoding="utf-8")
            for mode in (0o640, 0o604, 0o666):
                path.chmod(mode)
                with self.subTest(mode=mode), self.assertRaises(RuntimeError):
                    verify_manifest(path)

    @unittest.skipUnless(os.name == "nt", "Windows DACL test")
    def test_windows_rejects_everyone_access(self):
        from sdk_private_manifest import _create_windows_directory, _user_sid

        with tempfile.TemporaryDirectory() as parent:
            root = Path(parent) / "permissive"
            user = _user_sid()
            _create_windows_directory(root, f"O:{user}D:P(A;OICI;FA;;;{user})(A;OICI;FA;;;WD)")
            path = root / "manifest.json"
            path.write_text("{}", encoding="utf-8")
            with self.assertRaises(RuntimeError):
                verify_manifest(path)

    @unittest.skipUnless(os.name == "nt", "Windows staging test")
    def test_windows_staging_is_created_once_and_verified(self):
        with private_workspace() as root:
            path = create_manifest_staging(root)
            verify_manifest(path)
            self.assertEqual(path.read_bytes(), b"")
            with self.assertRaises(OSError):
                create_manifest_staging(root)
            self.assertEqual(path.read_bytes(), b"")

    def test_conformance_marker_is_required_even_for_successful_process(self):
        with contextlib.redirect_stdout(io.StringIO()):
            with self.assertRaisesRegex(RuntimeError, "required corpus success marker"):
                run([sys.executable, "-c", "print('no tests ran')"], required_output="expected-marker")

    def test_conformance_marker_can_cross_stream_chunks(self):
        with contextlib.redirect_stdout(io.StringIO()):
            run(
                [sys.executable, "-c", "print('x' * 4093 + 'expected-marker')"],
                required_output="expected-marker",
            )

    @unittest.skipUnless(os.name == "nt", "Windows command resolution test")
    def test_windows_resolves_cmd_tool_before_execution(self):
        with private_workspace() as root:
            executable = root / "rrflow-sdk-probe.cmd"
            executable.write_text("@exit /b 0\n", encoding="ascii")
            env = os.environ.copy()
            env["PATH"] = str(root) + os.pathsep + env.get("PATH", "")
            run(["rrflow-sdk-probe"], env)


if __name__ == "__main__":
    unittest.main()
