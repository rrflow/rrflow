"""Unit tests for the static test-name resolver and the selected-test check."""
from __future__ import annotations

import sys
import tempfile
import textwrap
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import check_selected_tests as checker  # noqa: E402
import selected_tests as owned  # noqa: E402


def write(root: Path, relative: str, text: str) -> None:
    path = root / relative
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(textwrap.dedent(text), encoding="utf-8")


class Fixture(unittest.TestCase):
    def setUp(self):
        self.original = (owned.ROOT, owned.SELECTORS, owned.EXACT_TEST_SETS)
        self.directory = tempfile.TemporaryDirectory()
        self.root = Path(self.directory.name)
        write(self.root, "crates/adapters/demo/Cargo.toml", '[package]\nname = "demo"\n')
        write(self.root, ".github/workflows/ci.yml", "")
        owned.ROOT = self.root
        owned.SELECTORS = ()
        owned.EXACT_TEST_SETS = {}

    def tearDown(self):
        owned.ROOT, owned.SELECTORS, owned.EXACT_TEST_SETS = self.original
        self.directory.cleanup()

    def suite(self, main: str, **files: str) -> None:
        write(self.root, "crates/adapters/demo/tests/suite/main.rs", main)
        for relative, text in files.items():
            write(self.root, f"crates/adapters/demo/tests/suite/{relative}", text)


class ResolverTests(Fixture):
    def test_module_paths_follow_files_path_attributes_and_inline_modules(self):
        self.suite(
            """
            #[path = "../support/shared.rs"]
            mod shared;
            mod plans;
            #[test]
            fn root_case() {}
            #[cfg(test)]
            mod inline {
                #[test]
                #[ignore = "a } brace in a string does not close the module"]
                fn nested_case() {}
            }
            """,
            **{"plans.rs": "#[test]\npub(super) fn moved_case() {}\n// #[test] fn commented() {}\n"},
        )
        write(self.root, "crates/adapters/demo/tests/support/shared.rs",
              "mod child;\n#[tokio::test(flavor = \"current_thread\")]\nasync fn shared_case() {}\n")
        write(self.root, "crates/adapters/demo/tests/support/child.rs",
              "#[cfg(target_os = \"linux\")]\n#[test]\nfn linux_only() {}\n")
        resolved = owned.resolve_target("demo", "suite")
        self.assertEqual(resolved.tests, {
            "root_case", "inline::nested_case", "plans::moved_case",
            "shared::shared_case", "shared::child::linux_only"})

    def test_helper_functions_without_test_attribute_are_not_tests(self):
        self.suite("#[allow(dead_code)]\nfn helper() {}\n#[test]\nfn real() {}\n")
        self.assertEqual(owned.resolve_target("demo", "suite").tests, {"real"})


class CheckTests(Fixture):
    def test_a_moved_owned_selector_fails_and_names_where_it_went(self):
        self.suite("mod plans;\n", **{"plans.rs": "#[test]\npub(super) fn case() {}\n"})
        owned.SELECTORS = (owned.Selector("demo", "suite", "case", owner="harness"),)
        errors = checker.check()
        self.assertEqual(len(errors), 1)
        self.assertIn("`case`", errors[0])
        self.assertIn("it is now plans::case", errors[0])

    def test_exact_set_reports_unexpected_and_missing_tests(self):
        self.suite("#[test]\nfn kept() {}\n#[test]\nfn added() {}\n")
        owned.EXACT_TEST_SETS = {("demo", "suite"): frozenset({"kept", "dropped"})}
        errors = checker.check()
        self.assertEqual(len(errors), 1)
        self.assertIn("unexpected ['added']", errors[0])
        self.assertIn("missing ['dropped']", errors[0])

    def test_workflow_exact_and_substring_filters_must_match(self):
        self.suite("mod damage;\n#[test]\nfn present() {}\n", **{"damage.rs": "#[test]\nfn torn() {}\n"})
        write(self.root, ".github/workflows/ci.yml", """
            run: cargo test -p demo --test suite --locked -- present --exact --nocapture
            run: cargo test -p demo --test suite --locked -- damage:: --nocapture
            run: cargo test -p demo --test suite --locked -- gone --exact
            run: cargo test -p demo --test suite --locked -- nothing_matches::
            """)
        errors = checker.check()
        self.assertEqual(len(errors), 2, errors)
        self.assertTrue(any("`gone`" in e and "names no test" in e for e in errors))
        self.assertTrue(any("`nothing_matches::`" in e and "matches no test" in e for e in errors))

    def test_a_stale_self_spawn_literal_fails(self):
        self.suite(
            "mod crash;\n",
            **{"crash.rs": """
                #[test]
                fn parent() { spawn("crash::child_moved_away"); }
                mod inner { #[test] fn child_moved_away() {} }
                """},
        )
        owned.SELECTORS = (owned.Selector("demo", "suite", "crash::parent"),)
        errors = checker.check()
        self.assertEqual(len(errors), 1, errors)
        self.assertIn("crash::child_moved_away", errors[0])
        self.assertIn("it is now crash::inner::child_moved_away", errors[0])

    def test_the_real_repository_resolves(self):
        owned.ROOT, owned.SELECTORS, owned.EXACT_TEST_SETS = self.original
        self.assertEqual(checker.check(), [])


if __name__ == "__main__":
    unittest.main()
