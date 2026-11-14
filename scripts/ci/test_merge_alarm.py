#!/usr/bin/env python3
"""Regressions for the merge alarm, against a recorded fake of the `gh` command."""

import json
import unittest

import merge_alarm


class FakeGitHub:
    """Answers `gh` the way GitHub would and records every write."""

    def __init__(self, pull=None, issues=()):
        self.pull = {
            "state": "OPEN",
            "isDraft": False,
            "headRefOid": "abc",
            "author": {"login": "owner", "is_bot": False},
            "url": "https://example.test/pull/7",
            **(pull or {}),
        }
        self.issues = list(issues)
        self.writes = []

    def __call__(self, *args):
        if args[:2] == ("issue", "list"):
            return json.dumps(self.issues)
        if args[:2] == ("pr", "view"):
            return json.dumps(self.pull)
        if args[:2] == ("pr", "list"):
            return json.dumps([{"number": 7}])
        self.writes.append(args)
        return ""

    def wrote(self, *prefix):
        return [write for write in self.writes if write[: len(prefix)] == prefix]


def dequeued(reason="CI_FAILURE"):
    return {"action": "dequeued", "reason": reason, "pull_request": {"number": 7}}


def run_event(**overrides):
    workflow_run = {
        "event": "merge_group",
        "conclusion": "failure",
        "name": "ci",
        "html_url": "https://example.test/run/1",
        "head_branch": "gh-readonly-queue/main/pr-7-abc123",
        "head_sha": "abc",
        "pull_requests": [],
        **overrides,
    }
    return {"workflow_run": workflow_run, "repository": {"default_branch": "main"}}


def dispatch(name, event, github):
    naps = []
    merge_alarm.dispatch(name, event, github, naps.append)
    return naps


class MergeAlarmTests(unittest.TestCase):
    def test_a_failed_queue_run_opens_one_issue_and_comments(self):
        github = FakeGitHub()
        dispatch("workflow_run", run_event(), github)
        created = github.wrote("issue", "create")
        self.assertEqual(len(created), 1)
        self.assertIn("Merge blocked: PR #7", created[0])
        body = created[0][created[0].index("--body") + 1]
        self.assertIn("@owner", body)
        self.assertIn("https://example.test/run/1", body)
        self.assertIn("gh pr merge 7 --auto --merge", body)
        self.assertEqual(len(github.wrote("pr", "comment")), 1)

    def test_a_second_failure_adds_to_the_open_issue(self):
        github = FakeGitHub(issues=[{"number": 3, "title": "Merge blocked: PR #7"}])
        dispatch("workflow_run", run_event(), github)
        self.assertEqual(github.wrote("issue", "create"), [])
        self.assertEqual(len(github.wrote("issue", "comment", "3")), 1)
        self.assertEqual(github.wrote("pr", "comment"), [])

    def test_runs_that_need_no_alarm_stay_quiet(self):
        cases = [
            ("cancelled", {}, run_event(conclusion="cancelled")),
            ("success", {}, run_event(conclusion="success")),
            ("skipped", {}, run_event(conclusion="skipped")),
            ("superseded", {"headRefOid": "newer"}, run_event(event="pull_request", pull_requests=[{"number": 7}])),
            ("draft", {"isDraft": True}, run_event()),
            ("unattributable branch", {}, run_event(head_branch="something-else")),
            ("push to another branch", {}, run_event(event="push", head_branch="agent/x")),
        ]
        for name, pull, event in cases:
            with self.subTest(name):
                github = FakeGitHub(pull=pull)
                dispatch("workflow_run", event, github)
                self.assertEqual(github.writes, [])

    def test_a_pull_request_failure_without_a_link_is_found_by_branch(self):
        github = FakeGitHub()
        dispatch("workflow_run", run_event(event="pull_request", head_branch="agent/x"), github)
        self.assertEqual(len(github.wrote("issue", "create")), 1)

    def test_a_merge_leaving_the_queue_is_not_an_alarm(self):
        github = FakeGitHub(pull={"state": "MERGED"})
        dispatch("pull_request", dequeued("MERGE"), github)
        self.assertEqual(github.writes, [])

    def test_a_removal_that_settles_open_sounds_after_waiting(self):
        github = FakeGitHub()
        naps = dispatch("pull_request", dequeued(), github)
        self.assertEqual(len(naps), merge_alarm.STATE_CHECKS - 1)
        created = github.wrote("issue", "create")
        self.assertEqual(len(created), 1)
        self.assertIn("CI_FAILURE", created[0][created[0].index("--body") + 1])

    def test_closing_the_pull_request_closes_its_issue(self):
        for merged, word in ((True, "merged"), (False, "closed without merging")):
            with self.subTest(merged=merged):
                github = FakeGitHub(issues=[{"number": 3, "title": "Merge blocked: PR #7"}])
                event = {"action": "closed", "pull_request": {"number": 7, "merged": merged}}
                dispatch("pull_request", event, github)
                closed = github.wrote("issue", "close", "3")
                self.assertEqual(len(closed), 1)
                self.assertIn(word, closed[0][-1])

    def test_red_main_opens_and_green_main_closes(self):
        red = FakeGitHub()
        dispatch("workflow_run", run_event(event="push", head_branch="main"), red)
        self.assertIn("Main is red: ci", red.wrote("issue", "create")[0])
        green = FakeGitHub(issues=[{"number": 9, "title": "Main is red: ci"}])
        dispatch("workflow_run", run_event(event="push", head_branch="main", conclusion="success"), green)
        self.assertEqual(len(green.wrote("issue", "close", "9")), 1)

    def test_a_failed_nightly_or_manual_qualification_of_main_sounds_the_main_alarm(self):
        # Candidates no longer run the full matrix, so its failures must surface here.
        for trigger in ("schedule", "workflow_dispatch"):
            with self.subTest(trigger=trigger):
                red = FakeGitHub()
                dispatch("workflow_run", run_event(event=trigger, head_branch="main"), red)
                self.assertEqual(len(red.wrote("issue", "create")), 1)
        other = FakeGitHub()
        dispatch("workflow_run", run_event(event="workflow_dispatch", head_branch="agent/x"), other)
        self.assertEqual(other.wrote("issue", "create"), [])

    def test_a_bot_author_is_not_mentioned(self):
        github = FakeGitHub(pull={"author": {"login": "app/x", "is_bot": True}})
        dispatch("workflow_run", run_event(), github)
        created = github.wrote("issue", "create")[0]
        self.assertNotIn("@app/x", created[created.index("--body") + 1])


if __name__ == "__main__":
    unittest.main()
