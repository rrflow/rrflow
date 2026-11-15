#!/usr/bin/env python3
"""Regressions for the one-branch session-end check."""

import subprocess
import tempfile
import unittest
from pathlib import Path

from check_session_end import problems


def git(repo: Path, *arguments: str) -> str:
    result = subprocess.run(
        ["git", "-C", str(repo), "-c", "user.name=t", "-c", "user.email=t@example.invalid", *arguments],
        capture_output=True, text=True, check=True,
    )
    return result.stdout.strip()


class SessionEndTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        base = Path(self.temp.name)
        self.remote = base / "remote.git"
        self.repo = base / "work"
        subprocess.run(["git", "init", "-q", "--bare", "-b", "main", str(self.remote)], check=True)
        subprocess.run(["git", "init", "-q", "-b", "main", str(self.repo)], check=True)
        (self.repo / "a.txt").write_text("a\n")
        git(self.repo, "add", "-A")
        git(self.repo, "commit", "-q", "-m", "first")
        git(self.repo, "remote", "add", "development", str(self.remote))
        git(self.repo, "push", "-q", "-u", "development", "main")

    def tearDown(self):
        self.temp.cleanup()

    def test_clean_main_passes(self):
        self.assertEqual(problems(self.repo), [])

    def test_uncommitted_change_fails(self):
        (self.repo / "a.txt").write_text("changed\n")
        self.assertTrue(any("uncommitted" in p for p in problems(self.repo)))

    def test_unpushed_commit_fails(self):
        (self.repo / "b.txt").write_text("b\n")
        git(self.repo, "add", "-A")
        git(self.repo, "commit", "-q", "-m", "second")
        self.assertTrue(any("lacks" in p for p in problems(self.repo)))

    def test_a_second_local_branch_fails(self):
        git(self.repo, "branch", "feature")
        self.assertTrue(any("local branch" in p for p in problems(self.repo)))

    def test_a_wip_branch_is_no_longer_an_exit(self):
        git(self.repo, "switch", "-q", "-c", "agent-wip-1")
        found = problems(self.repo)
        self.assertTrue(any("all work happens on main" in p for p in found))

    def test_a_second_worktree_fails(self):
        git(self.repo, "worktree", "add", "-q", "--detach", str(self.repo.parent / "second"))
        self.assertTrue(any("worktrees" in p for p in problems(self.repo)))

    def test_an_archive_ref_fails(self):
        git(self.repo, "update-ref", "refs/archive/x/y", "HEAD")
        self.assertTrue(any("outside main" in p for p in problems(self.repo)))

    def test_a_stash_fails(self):
        (self.repo / "a.txt").write_text("changed\n")
        git(self.repo, "stash")
        self.assertTrue(any("outside main" in p for p in problems(self.repo)))

    def test_a_remote_branch_besides_main_fails(self):
        git(self.repo, "push", "-q", "development", "main:refs/heads/side")
        git(self.repo, "fetch", "-q", "development")
        self.assertTrue(any("holds 1 branch" in p for p in problems(self.repo)))

    def test_hosted_merge_queue_refs_are_ignored(self):
        git(self.repo, "push", "-q", "development", "main:refs/heads/gh-readonly-queue/main/pr-1-abc")
        git(self.repo, "fetch", "-q", "development")
        self.assertEqual(problems(self.repo), [])


if __name__ == "__main__":
    unittest.main()
