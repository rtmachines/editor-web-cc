"""Check the release toolchain job and run its script with fake tools."""

import os
from pathlib import Path
import re
import shutil
import subprocess
import sys
import tempfile

ROOT = Path(__file__).resolve().parents[2]
COMMAND = "bash deploy/ci/rust-tests.sh release-toolchain"
FIXTURE = """jobs:
  release-toolchain:
    name: Release toolchain check
    runs-on: ubuntu-latest
    timeout-minutes: 30
    steps:
      - name: Check out the repository
        uses: actions/checkout@fixture
        with:
          persist-credentials: false
      - name: Cache the Rust build
        uses: Swatinem/rust-cache@fixture
        with:
          key: release-toolchain-${{ hashFiles('deploy/Dockerfile') }}
          save-if: ${{ github.ref == 'refs/heads/main' }}
      - name: Check the release toolchain
        run: bash deploy/ci/rust-tests.sh release-toolchain
  next:
    name: Other
"""


def check_workflow(workflow: str) -> list[str]:
    """Reject a missing job, skipped checks, and ignored failures."""
    headers = list(re.finditer(r"^  release-toolchain:\s*$", workflow, re.M))
    if len(headers) != 1:
        return ["release-toolchain-job: require exactly one release-toolchain job"]
    body = workflow[headers[0].end():].lstrip("\n")
    body = re.split(r"(?=^  [^\s#])", body, maxsplit=1, flags=re.M)[0]
    failures = []
    restricted = r"""(['"]?)(?:if|needs|continue-on-error)\1[ \t]*:"""
    # Reject unsupported YAML keys instead of decoding them.
    plain_key = r"""(?:[A-Za-z0-9_-]+|"[A-Za-z0-9_-]+"|'[A-Za-z0-9_-]+')[ \t]*:"""
    unsupported_key = '(?!' + plain_key + r')[^\s#]'
    restricted = '(?:' + restricted + '|' + unsupported_key + ')'
    if re.search(r"^    " + restricted, body, re.M):
        failures.append("release-toolchain-job: require an unconditional job that reports failures")
    steps = re.split(r"(?=^      - )", body, flags=re.M)[1:]
    caches = [step for step in steps
              if re.search(r"^        uses: Swatinem/rust-cache@", step, re.M)]
    cache_key = "          key: release-toolchain-${{ hashFiles('deploy/Dockerfile') }}"
    if len(caches) != 1 or not re.search(r"^" + re.escape(cache_key) + r"$", caches[0], re.M):
        failures.append("release-toolchain-cache: require the cache key with the Dockerfile hash")
    run = r"^      (?:  |- )run: " + re.escape(COMMAND) + r"$"
    matches = [step for step in steps if re.search(run, step, re.M)]
    if len(matches) != 1:
        failures.append("release-toolchain-step: require exactly one release toolchain check")
    elif re.search(r"^      (?:  |- )" + restricted, matches[0], re.M):
        failures.append("release-toolchain-step: require an unconditional step that reports failures")
    return failures


def self_test_failures() -> list[str]:
    """Reject each forbidden key and accept the cache save condition."""
    failures = []
    if check_workflow(FIXTURE):
        failures.append("self-test-save-if: accept the cache save condition")
    old_key = FIXTURE.replace(
        "key: release-toolchain-${{ hashFiles('deploy/Dockerfile') }}",
        "key: release-toolchain",
    )
    if not check_workflow(old_key):
        failures.append("self-test-cache-key: reject the cache key without the Dockerfile hash")
    escaped = {
        "if": r'"\u0069f"',
        "needs": r'"ne\u0065ds"',
        "continue-on-error": r'"continue-on-erro\u0072"',
    }
    for anchor, indent, keys in (
        ("    name: Release toolchain check\n", "    ",
         ("if", "needs", "continue-on-error")),
        (f"        run: {COMMAND}\n", "        ", ("if", "needs", "continue-on-error")),
    ):
        for key in keys:
            for spelling in (key, f"'{key}'", f'"{key}"', escaped[key]):
                changed = FIXTURE.replace(anchor, f"{indent}{spelling}: false\n" + anchor, 1)
                if not check_workflow(changed):
                    failures.append(f"self-test-{key}: reject the forbidden key {spelling}")
    for changed in (
        FIXTURE.replace(f"        run: {COMMAND}\n", ""),
        FIXTURE.replace("  release-toolchain:", "  other:"),
        FIXTURE.replace("  next:\n", "  next:\n    if: false\n"),
    ):
        expected_failure = "    if: false\n" not in changed
        if bool(check_workflow(changed)) != expected_failure:
            failures.append("self-test-boundary: check only the release toolchain job")
    return failures


def run_fixture(dockerfile: str, cargo_status: int = 0) -> tuple[int, list[str]]:
    """Run the copied script with logged tools at the front of PATH."""
    with tempfile.TemporaryDirectory() as directory:
        tree = Path(directory)
        scripts = tree / "deploy/ci"
        scripts.mkdir(parents=True)
        shutil.copyfile(ROOT / "deploy/ci/rust-tests.sh", scripts / "rust-tests.sh")
        (tree / "deploy/Dockerfile").write_text(dockerfile, encoding="utf-8")
        binaries = tree / "bin"
        binaries.mkdir()
        log = tree / "commands.log"
        fake = """#!/bin/bash
set -euo pipefail
printf '%s|%s|%s\\n' "${0##*/}" "${RUSTUP_TOOLCHAIN-}" "$*" >> "$COMMAND_LOG"
if [[ ${0##*/} == cargo ]]; then
  exit "$CARGO_STATUS"
fi
"""
        for name in ("rustup", "cargo"):
            path = binaries / name
            path.write_text(fake, encoding="utf-8")
            path.chmod(0o700)
        environment = dict(os.environ, PATH=f"{binaries}:{os.defpath}",
                           COMMAND_LOG=str(log), CARGO_STATUS=str(cargo_status))
        environment.pop("RUSTUP_TOOLCHAIN", None)
        result = subprocess.run(
            ["bash", str(scripts / "rust-tests.sh"), "release-toolchain"],
            cwd=tree, env=environment, capture_output=True, text=True, check=False,
        )
        entries = log.read_text(encoding="utf-8").splitlines() if log.exists() else []
        return result.returncode, entries


def run_test_failures() -> list[str]:
    """Catch fixed versions, wrong arguments, invalid input, and lost failures."""
    failures = []
    for version in ("1.99.1", "1.98.0"):
        status, entries = run_fixture(f"ENV RUSTUP_TOOLCHAIN={version} \\\n    OTHER=value\n")
        expected = [
            f"rustup||toolchain install {version} --profile minimal",
            f"cargo|{version}|check --release --locked --package recipe-server",
        ]
        if status != 0 or entries != expected:
            failures.append(f"release-toolchain-run: {version} returned {status}, commands {entries!r}")
    for name, dockerfile in (
        ("missing", "ENV OTHER=value\n"),
        ("duplicate", "ENV RUSTUP_TOOLCHAIN=1.99.1\nENV RUSTUP_TOOLCHAIN=1.98.0\n"),
        ("invalid", "ENV RUSTUP_TOOLCHAIN=1.99 \\\n"),
    ):
        status, entries = run_fixture(dockerfile)
        if status != 1 or entries:
            failures.append(f"release-toolchain-{name}: require exit 1 before any tool runs")
    status, entries = run_fixture("ENV RUSTUP_TOOLCHAIN=1.99.1\n", cargo_status=101)
    if status == 0 or len(entries) != 2 or not entries[-1].startswith("cargo|"):
        failures.append("release-toolchain-failure: run cargo and propagate its failure")
    return failures


def main() -> int:
    """Print failed checks without running a real Rust tool."""
    if sys.argv[1:]:
        print("usage: test_release_toolchain.py")
        return 1
    failures = []
    failures.extend(self_test_failures())
    workflow = (ROOT / ".github/workflows/ci.yml").read_text(encoding="utf-8")
    failures.extend(check_workflow(workflow))
    failures.extend(run_test_failures())
    for failure in failures:
        print(failure)
    if failures:
        return 1
    print("Release toolchain workflow, self-tests, and fake-PATH checks passed.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
