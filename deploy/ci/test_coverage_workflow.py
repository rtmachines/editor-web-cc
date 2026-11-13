"""Check the Coverage workflow contract and reject unsafe mutations."""

import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
TEST_FILES = (
    "tools/test_coverage_gate.py",
    "tools/test_coverage_report.py",
    "tools/test_coverage_inventory.py",
    "tools/test_coverage_summary.py",
    "tools/test_coverage_cli.py",
)
TEST_COMMANDS = "\n".join(f"python3 -I -B {path}" for path in TEST_FILES)
INVENTORY = (
    "python3 -I -B tools/coverage-gate.py --write-inventory "
    "target/coverage-inventory.json"
)
MEASUREMENT = (
    "cargo llvm-cov --workspace --all-features --locked --json "
    "--output-path target/llvm-cov-report.json"
)
GATE = (
    "python3 -I -B tools/coverage-gate.py --report target/llvm-cov-report.json "
    "--inventory target/coverage-inventory.json"
)
UPLOAD_REF = r"actions/upload-artifact@([^\s#]+)"


def coverage_body(workflow: str) -> str:
    """Extract the coverage job at the workflow's fixed indentation."""
    lines = workflow.splitlines(keepends=True)
    headers = [index for index, line in enumerate(lines) if line.rstrip() == "  coverage:"]
    if len(headers) != 1:
        raise ValueError("coverage-job: require exactly one coverage job")
    start = headers[0] + 1
    end = start
    while end < len(lines) and (lines[end].startswith("    ") or not lines[end].strip()):
        end += 1
    return "".join(lines[start:end])


def field(block: str, key: str, indent: int) -> str:
    """Read one scalar or literal block without a YAML dependency."""
    pattern = rf"^ {{{indent}}}{re.escape(key)}: ([^\n]+)$"
    matches = list(re.finditer(pattern, block, re.MULTILINE))
    if len(matches) != 1:
        return ""
    match = matches[0]
    value = match.group(1).split(" #", 1)[0].strip()
    if value != "|":
        return value
    lines = []
    for line in block[match.end():].splitlines()[1:]:
        if line.strip() and not line.startswith(" " * (indent + 2)):
            break
        lines.append(line[indent + 2:])
    return "\n".join(lines)


def command_step(steps: list[str], command: str) -> int:
    """Find a unique step whose run field contains only the required commands."""
    matches = [index for index, step in enumerate(steps)
               if field(step, "run", 8) == command]
    return matches[0] if len(matches) == 1 else -1


def upload_failures(steps: list[str], other_refs: list[str]) -> tuple[int, list[str]]:
    """Check the artifact settings and the shared action revision."""
    uploads = [index for index, step in enumerate(steps)
               if field(step, "uses", 8).startswith("actions/upload-artifact@")]
    if len(uploads) != 1:
        return -1, ["upload-step: require exactly one artifact upload"]
    index = uploads[0]
    upload = steps[index]
    ref = field(upload, "uses", 8).removeprefix("actions/upload-artifact@")
    rules = (
        (bool(re.fullmatch(r"[0-9a-fA-F]{40}", ref)),
         "upload-sha: pin the upload action with a 40-hex SHA"),
        (bool(other_refs) and set(other_refs) == {ref},
         "upload-shared-sha: use the SHA of the other artifact uploads"),
        (field(upload, "if", 8) == "always()",
         "upload-always: run the upload with if: always()"),
        (field(upload, "retention-days", 10) == "14",
         "upload-retention: retain the artifact for 14 days"),
        (field(upload, "if-no-files-found", 10) == "warn",
         "upload-missing-files: warn when artifact files do not exist"),
        ("${{ github.sha }}" in field(upload, "name", 10),
         "upload-name: include the revision in the artifact name"),
        ({"target/llvm-cov-report.json", "target/coverage-inventory.json"}
         <= set(field(upload, "path", 10).splitlines()),
         "upload-paths: upload the full report and the inventory"),
    )
    return index, [message for valid, message in rules if not valid]


def check_job(body: str, other_refs: list[str], existing_tests: set[str]) -> list[str]:
    """Return one diagnostic for each failed workflow rule."""
    steps = re.split(r"(?=^      - )", body, flags=re.MULTILINE)[1:]
    test_step = command_step(steps, TEST_COMMANDS)
    inventory = command_step(steps, INVENTORY)
    measurement = command_step(steps, MEASUREMENT)
    gate = command_step(steps, GATE)
    upload, failures = upload_failures(steps, other_refs)
    conditions = re.findall(r"^ *(?:- )?if *:", body, re.MULTILINE)
    rules = (
        (field(body, "name", 4) == "Coverage", "job-name: keep the name Coverage"),
        (field(body, "timeout-minutes", 4) == "45", "job-timeout: keep the 45-minute timeout"),
        (any(field(step, "tool", 10) == "cargo-llvm-cov@0.6.24" for step in steps),
         "coverage-version: keep cargo-llvm-cov@0.6.24"),
        ("--summary-only" not in body, "no-summary-only: measure the full report"),
        (test_step >= 0, "coverage-tests: run exactly the five python3 -I -B commands in one step"),
        (set(TEST_FILES) <= existing_tests, "coverage-test-files: keep all five test files"),
        (inventory >= 0, "inventory-command: write target/coverage-inventory.json"),
        (measurement >= 0, "measurement-command: write the full target/llvm-cov-report.json"),
        (gate >= 0, "gate-command: pass the report and inventory to the gate"),
        (0 <= test_step < measurement,
         "test-order: run all coverage tests before the measurement"),
        (0 <= inventory < measurement, "inventory-order: write the inventory before the measurement"),
        (0 <= measurement < gate, "gate-order: measure before the gate"),
        (0 <= gate < upload, "upload-order: apply the gate before the upload"),
        (upload >= 0 and conditions == ["        if:"]
         and field(steps[upload], "if", 8) == "always()",
         "always-scope: allow if only on the upload step with always()"),
        (not re.search(r"^ *(?:- )?shell *:", body, re.MULTILINE),
         "no-shell: use the default shell throughout the job"),
        ("continue-on-error" not in body, "no-continue-on-error: propagate every step failure"),
        ("--ignore-run-fail" not in body, "no-ignore-run-fail: propagate measurement failures"),
    )
    return [message for valid, message in rules if not valid] + failures


# This fixture has five processing steps and one tool installation step.
FIXTURE = """    name: Coverage
    timeout-minutes: 45
    steps:
      - name: Install cargo-llvm-cov
        uses: taiki-e/install-action@1168f6810a3cb30a3514bf5756a3b7f0422c6df8
        with:
          tool: cargo-llvm-cov@0.6.24
      - name: Test the coverage tools
        run: |
          python3 -I -B tools/test_coverage_gate.py
          python3 -I -B tools/test_coverage_report.py
          python3 -I -B tools/test_coverage_inventory.py
          python3 -I -B tools/test_coverage_summary.py
          python3 -I -B tools/test_coverage_cli.py
      - name: Write the coverage inventory
        run: python3 -I -B tools/coverage-gate.py --write-inventory target/coverage-inventory.json
      - name: Measure the line coverage
        run: cargo llvm-cov --workspace --all-features --locked --json --output-path target/llvm-cov-report.json
      - name: Apply the coverage gate
        run: python3 -I -B tools/coverage-gate.py --report target/llvm-cov-report.json --inventory target/coverage-inventory.json
      - name: Upload the coverage report
        if: always()
        uses: actions/upload-artifact@cf430e030ddbb5b0abf93d22962f4752f3646cd9 # v7.0.2
        with:
          name: coverage-report-${{ github.sha }}
          path: |
            target/llvm-cov-report.json
            target/coverage-inventory.json
          retention-days: 14
          if-no-files-found: warn
"""
FIXTURE_SHA = "cf430e030ddbb5b0abf93d22962f4752f3646cd9"


def mutations() -> list[tuple[str, str]]:
    """Name the rule that each independent text mutation must fail."""
    replacements = (
        ("job-name", "name: Coverage", "name: Other"),
        ("job-timeout", "timeout-minutes: 45", "timeout-minutes: 44"),
        ("coverage-version", "cargo-llvm-cov@0.6.24", "cargo-llvm-cov@0.6.23"),
        ("no-summary-only", "--json --output-path", "--json --summary-only --output-path"),
        ("inventory-command", "--write-inventory", "--source-only"),
        ("measurement-command", "--workspace --all-features", "--workspace"),
        ("gate-command", "--inventory target/coverage-inventory.json", ""),
        ("upload-sha", FIXTURE_SHA, "v7.0.2"),
        ("upload-shared-sha", FIXTURE_SHA, "a" * 40),
        ("upload-always", "        if: always()\n", ""),
        ("upload-retention", "retention-days: 14", "retention-days: 1"),
        ("upload-missing-files", "if-no-files-found: warn", "if-no-files-found: ignore"),
        ("upload-name", "${{ github.sha }}", "latest"),
        ("upload-paths", "            target/llvm-cov-report.json\n", ""),
        ("upload-paths", "            target/coverage-inventory.json\n", ""),
        ("always-scope", "        run: cargo", "        if: always()\n        run: cargo"),
        ("no-continue-on-error", "    steps:", "    continue-on-error: true\n    steps:"),
        ("no-continue-on-error", "        run: cargo", "        continue-on-error: true\n        run: cargo"),
        ("no-ignore-run-fail", "--json --output-path", "--json --ignore-run-fail --output-path"),
    )
    cases = [(rule, FIXTURE.replace(old, new, 1)) for rule, old, new in replacements]
    for path in TEST_FILES:
        cases.append(("coverage-tests", FIXTURE.replace(f"python3 -I -B {path}", f"python3 {path}")))
        cases.append(("coverage-tests", FIXTURE.replace(f"          python3 -I -B {path}\n", "")))
    return cases


def order_mutations() -> list[tuple[str, str]]:
    """Move each ordered step across its required boundary."""
    parts = re.split(r"(?=^      - )", FIXTURE, flags=re.MULTILINE)
    cases = []
    for rule, left, right in (
        ("test-order", 2, 4),
        ("inventory-order", 3, 4),
        ("gate-order", 4, 5),
        ("upload-order", 5, 6),
    ):
        changed = parts.copy()
        changed[left], changed[right] = changed[right], changed[left]
        cases.append((rule, "".join(changed)))
    return cases


def execution_mutations() -> list[tuple[str, str]]:
    """Reject conditions, shell overrides, and commands that mask failures."""
    cases = []
    for condition in ("false", "${{ false }}"):
        cases.append(("always-scope", FIXTURE.replace(
            f"        run: {GATE}", f"        if: {condition}\n        run: {GATE}", 1)))
    cases.append(("always-scope", FIXTURE.replace(
        f"        run: {MEASUREMENT}",
        f"        if: github.event_name != 'pull_request'\n        run: {MEASUREMENT}", 1)))
    cases.append(("always-scope", "    if: false\n" + FIXTURE))
    cases.append(("upload-always", FIXTURE.replace("if: always()", "if: false", 1)))
    for rule, command in (("inventory-command", INVENTORY),
                          ("measurement-command", MEASUREMENT), ("gate-command", GATE)):
        cases.append((rule, FIXTURE.replace(
            f"        run: {command}",
            f"        run: |\n          set +e\n          {command}\n          true", 1)))
        cases.append((rule, FIXTURE.replace(
            f"        run: {command}",
            f"        run: |\n          # Extra line\n          {command}", 1)))
    cases.append(("no-shell", FIXTURE.replace(
        "        run: |", "        shell: bash {0}\n        run: |", 1)))
    cases.append(("no-shell", "    defaults:\n      run:\n        shell: bash {0}\n" + FIXTURE))
    for extra in ("set +e", "true", "# Extra line"):
        cases.append(("coverage-tests", FIXTURE.replace(
            "        run: |", f"        run: |\n          {extra}", 1)))
    return cases


def self_test_failures() -> list[str]:
    """Check a valid fixture, job boundaries, and rejected mutations."""
    refs = [FIXTURE_SHA, FIXTURE_SHA]
    files = set(TEST_FILES)
    failures = []
    if check_job(FIXTURE, refs, files):
        failures.append("self-test-valid: accept the valid fixture")
    workflow = "jobs:\n  coverage:\n" + FIXTURE + "  next:\n    name: Other\n"
    if coverage_body(workflow) != FIXTURE:
        failures.append("self-test-boundary: exclude the next job")
    try:
        coverage_body("jobs:\n  next:\n    name: Coverage\n")
    except ValueError:
        pass
    else:
        failures.append("self-test-missing-job: reject a missing coverage job")
    for rule, changed in mutations() + order_mutations() + execution_mutations():
        if not any(message.startswith(rule + ":") for message in check_job(changed, refs, files)):
            failures.append(f"self-test-{rule}: reject the mutated fixture")
    for path in TEST_FILES:
        errors = check_job(FIXTURE, refs, files - {path})
        if not any(message.startswith("coverage-test-files:") for message in errors):
            failures.append(f"self-test-coverage-test-files: reject the missing file {path}")
    return failures


def main() -> int:
    """Print each failed rule on one line and return the contract result."""
    failures = self_test_failures()
    workflow = (ROOT / ".github/workflows/ci.yml").read_text(encoding="utf-8")
    try:
        body = coverage_body(workflow)
    except ValueError as error:
        failures.append(str(error))
    else:
        outside = workflow.replace("  coverage:\n" + body, "", 1)
        refs = re.findall(r"^        uses: " + UPLOAD_REF, outside, re.MULTILINE)
        files = {path for path in TEST_FILES if (ROOT / path).is_file()}
        failures.extend(check_job(body, refs, files))
    for failure in failures:
        print(failure)
    if failures:
        return 1
    print("CI coverage workflow check and mutation tests passed.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
