#!/usr/bin/env python3
"""Make a pull request that cannot merge impossible to miss; docs/operations/ci.md owns this."""

from __future__ import annotations

import json
import os
import re
import subprocess
import sys
import time
from pathlib import Path
from typing import Any, Callable

LABEL = "merge-blocked"
FAILED_CONCLUSIONS = {"failure", "timed_out", "startup_failure"}
QUEUE_BRANCH = re.compile(r"^gh-readonly-queue/.+/pr-(\d+)-[0-9a-f]+$")

# A merge also removes the pull request from the queue, and its state can lag
# the event, so a removal is only an alarm once the state has had time to settle.
STATE_CHECKS = 4
STATE_CHECK_SECONDS = 15

Gh = Callable[..., str]


def gh(*args: str) -> str:
    done = subprocess.run(["gh", *args], capture_output=True, text=True)
    if done.returncode:
        raise SystemExit(f"gh {args[0]} {args[1]} failed: {done.stderr.strip()}")
    return done.stdout


def pull_title(number: int) -> str:
    return f"Merge blocked: PR #{number}"


def main_title(workflow: str) -> str:
    return f"Main is red: {workflow}"


def open_issue(run: Gh, title: str) -> int | None:
    listed = json.loads(
        run("issue", "list", "--label", LABEL, "--state", "open", "--limit", "200", "--json", "number,title")
    )
    return next((issue["number"] for issue in listed if issue["title"] == title), None)


def sound(run: Gh, title: str, what: str, follow_up: str, pull: int | None = None, mention: str = "") -> None:
    run("label", "create", LABEL, "--color", "B60205", "--description", "Could not merge, or main is red", "--force")
    existing = open_issue(run, title)
    if existing is not None:
        run("issue", "comment", str(existing), "--body", f"Still blocked. {what}")
        return
    body = f"{mention}{what}\n\n**Follow-up, for a person or an AI session:**\n\n{follow_up}\n"
    run("issue", "create", "--title", title, "--label", LABEL, "--body", body)
    if pull is not None:
        run("pr", "comment", str(pull), "--body", f":rotating_light: **{title}.** {mention}{what}")


def silence(run: Gh, title: str, why: str) -> None:
    existing = open_issue(run, title)
    if existing is not None:
        run("issue", "close", str(existing), "--comment", why)


def view_pull(run: Gh, number: int) -> dict[str, Any]:
    return json.loads(
        run("pr", "view", str(number), "--json", "state,isDraft,headRefOid,author,url")
    )


def mention_of(pull: dict[str, Any]) -> str:
    author = pull.get("author") or {}
    return f"@{author['login']} " if author.get("login") and not author.get("is_bot") else ""


def follow_up_for(number: int, url: str) -> str:
    return (
        f"1. Read why it stopped: {url}\n"
        "2. Fix the cause on the pull request's branch. Do not remove or weaken a failing check.\n"
        f"3. When CI passes, queue it again: `gh pr merge {number} --auto --merge`.\n\n"
        "This issue closes by itself when the pull request merges or closes."
    )


def on_pull_request(event: dict[str, Any], run: Gh, sleep: Callable[[float], None]) -> None:
    number = int(event["pull_request"]["number"])
    if event["action"] == "closed":
        outcome = "merged" if event["pull_request"].get("merged") else "closed without merging"
        silence(run, pull_title(number), f"Resolved: pull request #{number} {outcome}.")
        return
    if event["action"] != "dequeued":
        return
    for attempt in range(STATE_CHECKS):
        pull = view_pull(run, number)
        if pull["state"] != "OPEN":
            return
        if attempt < STATE_CHECKS - 1:
            sleep(STATE_CHECK_SECONDS)
    reason = str(event.get("reason") or "no reason given")
    sound(
        run,
        pull_title(number),
        f"The merge queue removed this pull request: {reason}.",
        follow_up_for(number, pull["url"]),
        number,
        mention_of(pull),
    )


def candidate_pull(run_data: dict[str, Any], run: Gh) -> int | None:
    trigger = run_data["event"]
    if trigger == "merge_group":
        match = QUEUE_BRANCH.match(run_data["head_branch"] or "")
        return int(match.group(1)) if match else None
    if trigger == "pull_request":
        linked = run_data.get("pull_requests") or []
        if linked:
            return int(linked[0]["number"])
        found = json.loads(
            run("pr", "list", "--head", run_data["head_branch"], "--state", "open", "--json", "number")
        )
        return int(found[0]["number"]) if found else None
    return None


def on_workflow_run(event: dict[str, Any], run: Gh) -> None:
    run_data = event["workflow_run"]
    conclusion, name, url = run_data["conclusion"], run_data["name"], run_data["html_url"]
    # A push to main, or the nightly or a manual qualification run of main.
    if (run_data["event"] in ("push", "schedule", "workflow_dispatch")
            and run_data["head_branch"] == event["repository"]["default_branch"]):
        title = main_title(name)
        if conclusion == "success":
            silence(run, title, f"Resolved: {url} passed on main.")
        elif conclusion in FAILED_CONCLUSIONS:
            sound(run, title, f"{name} ended `{conclusion}` on main: {url}", f"Read the run and repair main: {url}")
        return
    if conclusion not in FAILED_CONCLUSIONS:
        return
    number = candidate_pull(run_data, run)
    if number is None:
        return
    pull = view_pull(run, number)
    if pull["state"] != "OPEN" or pull["isDraft"]:
        return
    if run_data["event"] == "pull_request" and run_data["head_sha"] != pull["headRefOid"]:
        return
    where = "in the merge queue" if run_data["event"] == "merge_group" else "on this pull request"
    sound(
        run,
        pull_title(number),
        f"{name} ended `{conclusion}` {where}: {url}",
        follow_up_for(number, url),
        number,
        mention_of(pull),
    )


def dispatch(name: str, event: dict[str, Any], run: Gh, sleep: Callable[[float], None]) -> None:
    if name == "pull_request":
        on_pull_request(event, run, sleep)
    elif name == "workflow_run":
        on_workflow_run(event, run)


def main() -> int:
    event = json.loads(Path(os.environ["GITHUB_EVENT_PATH"]).read_text(encoding="utf-8"))
    dispatch(os.environ["GITHUB_EVENT_NAME"], event, gh, time.sleep)
    return 0


if __name__ == "__main__":
    sys.exit(main())
