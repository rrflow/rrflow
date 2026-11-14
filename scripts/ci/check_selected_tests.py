"""Fail when a test name CI selects no longer resolves to a test.

Checks, without compiling:
- every selector owned by `selected_tests.py` names an existing test;
- each target with a declared exact test set has exactly that set;
- every `cargo test … -- FILTER` in the workflows matches: `--exact` filters name one
  test, substring filters match at least one (a filter matching nothing passes silently);
- every string literal in a selected Rust target that names a test path of that target,
  such as a test that respawns itself by name, still resolves.
"""
from __future__ import annotations

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import selected_tests as owned  # noqa: E402


def check(root: Path | None = None, counts: dict | None = None) -> list[str]:
    if root is not None:
        owned.ROOT = root
    counts = counts if counts is not None else {}
    counts.update(owned=0, workflow=0, literals=0)
    errors: list[str] = []
    targets: dict[tuple[str, str], owned.Target] = {}

    def target(package: str, name: str) -> owned.Target | None:
        key = (package, name)
        if key not in targets:
            try:
                targets[key] = owned.resolve_target(package, name)
            except (LookupError, FileNotFoundError) as error:
                errors.append(f"{package} {name}: {error}")
                targets[key] = None
        return targets[key]

    def require(selector: owned.Selector, where: str) -> None:
        resolved = target(selector.package, selector.target)
        if resolved is None:
            return
        if not owned.matches(resolved, selector.name, selector.exact):
            leaf = selector.name.rsplit("::", 1)[-1]
            moved = sorted(test for test in resolved.tests if test.rsplit("::", 1)[-1] == leaf)
            hint = f"; it is now {', '.join(moved)}" if moved else ""
            kind = "names no test" if selector.exact else "matches no test"
            errors.append(f"{where}: `{selector.name}` in {selector.package} {selector.target} {kind}{hint}")

    for selector in owned.SELECTORS:
        counts["owned"] += 1
        require(selector, f"selected_tests.py ({selector.owner})")

    for (package, name), expected in owned.EXACT_TEST_SETS.items():
        resolved = target(package, name)
        if resolved is not None and resolved.tests != set(expected):
            extra, missing = sorted(resolved.tests - expected), sorted(expected - resolved.tests)
            errors.append(f"{package} {name}: test set differs from the required set; "
                          f"unexpected {extra}, missing {missing}")

    for workflow in sorted(owned.ROOT.joinpath(".github", "workflows").glob("*.yml")):
        for selector in owned.workflow_selectors(workflow.read_text(encoding="utf-8")):
            counts["workflow"] += 1
            require(selector, workflow.relative_to(owned.ROOT).as_posix())

    for (package, name), resolved in list(targets.items()):
        if resolved is None:
            continue
        roots = {module.split("::", 1)[0] for module in resolved.modules} | {
            test.split("::", 1)[0] for test in resolved.tests}
        for path, literal in resolved.literals:
            first, _, _ = literal.partition("::")
            if literal in resolved.tests:
                counts["literals"] += 1
                continue
            if (first in roots and literal.replace("_", "").replace(":", "").isalnum()
                    and literal not in resolved.modules and literal not in resolved.tests):
                leaf = literal.rsplit("::", 1)[-1]
                moved = sorted(test for test in resolved.tests if test.rsplit("::", 1)[-1] == leaf)
                counts["literals"] += 1
                if moved or any(t.startswith(literal.rsplit("::", 1)[0] + "::") for t in resolved.tests):
                    hint = f"; it is now {', '.join(moved)}" if moved else ""
                    errors.append(f"{path.relative_to(owned.ROOT).as_posix()}: test path literal "
                                  f"`{literal}` names no test in {package} {name}{hint}")
    return errors


def main() -> int:
    counts: dict = {}
    errors = check(counts=counts)
    if errors:
        for error in errors:
            print(f"selected-tests: {error}", file=sys.stderr)
        return 1
    print(f"selected-tests: OK: {counts['owned']} owned selectors, {counts['workflow']} workflow "
          f"filters and {counts['literals']} test-path literals resolve")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
