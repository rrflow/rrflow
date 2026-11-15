#!/usr/bin/env python3
"""Refuse to let an agent session end with work outside `main`.

The owner's pre-release rule (AGENTS.md, 2026-11-14) is one branch in one
folder. On 2026-11-09 five worktrees held between 14 and 514 staged files each,
and by 2026-11-14 there were 35 worktrees, 172 local branches and 81 archived
refs: the owner could not tell which checkout held the real work. This check
passes only when `main` is the sole branch, its worktree is the sole worktree,
nothing is uncommitted, stashed or archived, and the remote holds nothing
`main` does not. Run it before stopping.
"""

from __future__ import annotations

import subprocess
import sys
from pathlib import Path

# GitHub creates and removes these merge-queue scratch refs itself.
HOSTED_PREFIXES = ("gh-readonly-queue/",)
# Reference namespaces that hold work outside any branch.
SIDE_NAMESPACES = ("refs/archive/", "refs/stash", "refs/wip/")


def git(repo: Path, *arguments: str, check: bool = True) -> str:
    result = subprocess.run(["git", "-C", str(repo), *arguments], capture_output=True, text=True)
    if check and result.returncode:
        raise RuntimeError(f"git {' '.join(arguments)}: {result.stderr.strip()}")
    return result.stdout.strip()


def problems(repo: Path) -> list[str]:
    found: list[str] = []
    status = [line for line in git(repo, "status", "--porcelain").splitlines() if line]
    if status:
        found.append(f"{len(status)} uncommitted change(s); commit them to main")
    branch = git(repo, "rev-parse", "--abbrev-ref", "HEAD")
    if branch != "main":
        found.append(f"on {branch!r}; all work happens on main")
    others = [name for name in git(repo, "for-each-ref", "--format=%(refname:short)", "refs/heads/").splitlines()
              if name != "main"]
    if others:
        found.append(f"{len(others)} local branch(es) besides main: {', '.join(sorted(others)[:5])}")
    worktrees = [line for line in git(repo, "worktree", "list", "--porcelain").splitlines() if line.startswith("worktree ")]
    if len(worktrees) > 1:
        found.append(f"{len(worktrees)} worktrees; the repository's own checkout is the only one")
    side = [name for name in git(repo, "for-each-ref", "--format=%(refname)").splitlines()
            if name.startswith(SIDE_NAMESPACES)]
    if side:
        found.append(f"{len(side)} ref(s) hold work outside main: {', '.join(sorted(side)[:5])}; absorb each, then delete it")
    upstream = subprocess.run(["git", "-C", str(repo), "rev-parse", "--abbrev-ref", "--symbolic-full-name", "main@{upstream}"],
                              capture_output=True, text=True)
    if upstream.returncode != 0:
        found.append("main has no upstream; push it (git push -u <remote> main)")
        return found
    tracked = upstream.stdout.strip()
    remote = tracked.split("/", 1)[0]
    ahead = int(git(repo, "rev-list", "--count", f"{tracked}..main"))
    if ahead:
        found.append(f"main has {ahead} commit(s) {tracked} lacks; push before stopping")
    extras = []
    for name in git(repo, "for-each-ref", "--format=%(refname:short)", f"refs/remotes/{remote}/").splitlines():
        short = name[len(remote) + 1:]
        if short in ("main", "HEAD") or name == remote or short.startswith(HOSTED_PREFIXES):
            continue
        extras.append(short)
    if extras:
        found.append(f"{remote} holds {len(extras)} branch(es) besides main: {', '.join(sorted(extras)[:5])}")
    return found


def main() -> int:
    repo = Path(__file__).resolve().parents[2]
    found = problems(repo)
    for problem in found:
        print(f"session-end: {problem}")
    if found:
        return 1
    print("session-end: OK: main is the only branch and worktree, clean, nothing unpushed or archived")
    return 0


if __name__ == "__main__":
    sys.exit(main())
