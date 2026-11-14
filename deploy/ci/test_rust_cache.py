"""Check that Rust cache steps save only on main."""

import re
from contextlib import redirect_stdout
from io import StringIO
from pathlib import Path
from tempfile import TemporaryDirectory
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[2]
SAVE_IF = "save-if: ${{ github.ref == 'refs/heads/main' }}"


def cache_failures(workflow: str) -> list[str]:
    """Return each cache step that lacks the required input."""
    lines = workflow.splitlines()
    failures = []
    for index, line in enumerate(lines):
        header = re.fullmatch(r"( +)- .*", line)
        if header is None:
            continue
        indent = len(header.group(1)) + 2
        end = index + 1
        while end < len(lines):
            following = lines[end]
            if (following.strip() and not following.lstrip().startswith("#")
                    and len(following) - len(following.lstrip()) < indent):
                break
            end += 1
        step = [" " * indent + line.lstrip()[2:], *lines[index + 1:end]]
        action = r"(?i:Swatinem)/rust-cache@"
        # Reject flow-style cache steps instead of decoding them.
        if step[0].lstrip().startswith("{") and re.search(action, "\n".join(step)):
            failures.append(f"line {index + 1}: use a block-style rust-cache step")
            continue
        uses = (rf"""^ {{{indent}}}uses:[ \t]+(['"]?){action}"""
                r"""[^\s#'"]+\1(?:\s+#.*)?$""")
        if not any(re.fullmatch(uses, field) for field in step):
            continue
        in_with = False
        has_save_if = False
        for field in step:
            if (field.strip() and not field.lstrip().startswith("#")
                    and len(field) - len(field.lstrip()) <= indent):
                in_with = field == " " * indent + "with:"
            if in_with and field == " " * (indent + 2) + SAVE_IF:
                has_save_if = True
        if not has_save_if:
            failures.append(f"line {index + 1}: rust-cache needs {SAVE_IF} in with:")
    return failures


def self_test_failures() -> list[str]:
    """Accept a valid step and reject missing or misplaced inputs."""
    step = "      - uses: Swatinem/rust-cache@fixture\n"
    inputs = "        with:\n          " + SAVE_IF + "\n"
    valid = step + inputs
    failures = []
    uses_forms = (
        "uses: 'Swatinem/rust-cache@fixture'",
        'uses: "Swatinem/rust-cache@fixture"',
        "uses:   Swatinem/rust-cache@fixture",
        "uses:   'Swatinem/rust-cache@fixture'",
        'uses:   "Swatinem/rust-cache@fixture"',
        "uses: swatinem/rust-cache@fixture",
    )
    for uses in uses_forms:
        fixture = "      - " + uses + "\n"
        if not cache_failures(fixture):
            failures.append(f"self-test: reject cache step without save-if: {uses}")
        if cache_failures(fixture + inputs):
            failures.append(f"self-test: accept cache step with save-if: {uses}")
    for fixture in (valid, step + "        with:\n          key: fixture\n"
                    + "          " + SAVE_IF + "\n"):
        if cache_failures(fixture):
            failures.append("self-test: accept a cache step that saves only on main")
    invalid = (
        step,
        valid.replace("refs/heads/main", "refs/heads/other"),
        valid.replace("with:", "env:"),
        valid.replace("          save-if:", "        save-if:"),
        valid.replace("          save-if:", "          # save-if:"),
        step + "      - name: Other step\n" + inputs,
        "      - {uses: Swatinem/rust-cache@fixture}\n",
    )
    for index, fixture in enumerate(invalid):
        if not cache_failures(fixture):
            failures.append(f"self-test: reject invalid cache fixture {index + 1}")
    if len(cache_failures(step + valid)) != 1:
        failures.append("self-test: check each cache step separately")
    failures.extend(workflow_self_test_failures(step, inputs))
    return failures


def workflow_self_test_failures(step: str, inputs: str) -> list[str]:
    """Check both workflow extensions through the main entry point."""
    failures = []
    with TemporaryDirectory() as directory:
        root = Path(directory)
        workflows = root / ".github/workflows"
        workflows.mkdir(parents=True)
        for suffix in (".yml", ".yaml"):
            path = workflows / ("fixture" + suffix)
            for content, expected in ((step, 1), (step + inputs, 0)):
                path.write_text(content, encoding="utf-8")
                with patch.dict(globals(), ROOT=root, self_test_failures=lambda: []):
                    with redirect_stdout(StringIO()):
                        result = main()
                if result != expected:
                    failures.append(f"self-test: check {suffix} workflow with exit {expected}")
            path.unlink()
    return failures


def main() -> int:
    """Check each workflow and print each failed rule."""
    failures = self_test_failures()
    paths = (ROOT / ".github/workflows").glob("*")
    for path in sorted(path for path in paths if path.suffix in (".yml", ".yaml")):
        for failure in cache_failures(path.read_text(encoding="utf-8")):
            failures.append(f"{path.relative_to(ROOT)}:{failure}")
    for failure in failures:
        print(failure)
    if failures:
        return 1
    print("CI Rust cache check and self-tests passed.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
