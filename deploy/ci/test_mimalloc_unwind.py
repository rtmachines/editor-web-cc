#!/usr/bin/env python3
"""Check both musl unwind test runs with a Cargo stub."""

import os
from pathlib import Path
import subprocess
import tempfile


RUST_TESTS = Path(__file__).with_name("rust-tests.sh").resolve()
COMMAND = "test --target x86_64-unknown-linux-musl -p recipe-server --test mimalloc_unwind --locked"
CARGO_STUB = """#!/bin/bash
set -euo pipefail
{
  printf '%q ' "$@"
  printf '\\n'
} >> "$COMMAND_LOG"
profile=dev
count=$DEV_COUNT
for argument in "$@"
do
  if [[ $argument == --release ]]
  then
    profile=release
    count=$RELEASE_COUNT
  fi
done
printf 'test result: ok. %s passed; 0 failed; 0 ignored; 0 measured; 0 filtered out\\n' "$count"
if [[ $profile == "$FAIL_PROFILE" ]]
then
  exit 101
fi
"""


def run_fixture(
    dev_count: int, release_count: int, fail_profile: str,
) -> tuple[subprocess.CompletedProcess[str], list[str]]:
    """Run the mode with a Cargo stub at the front of PATH."""
    with tempfile.TemporaryDirectory() as directory:
        tree = Path(directory)
        cargo = tree / "cargo"
        cargo.write_text(CARGO_STUB, encoding="utf-8")
        cargo.chmod(0o700)
        log = tree / "commands.log"
        environment = dict(
            os.environ, PATH=f"{tree}:{os.defpath}", COMMAND_LOG=str(log),
            DEV_COUNT=str(dev_count), RELEASE_COUNT=str(release_count), FAIL_PROFILE=fail_profile,
        )
        result = subprocess.run(
            ["bash", str(RUST_TESTS), "mimalloc-unwind"], cwd=tree, env=environment,
            capture_output=True, text=True, check=False,
        )
        commands = log.read_text(encoding="utf-8").splitlines() if log.exists() else []
        return result, [command.removesuffix(" ") for command in commands]


def main() -> None:
    """Check the count, exit status, and arguments of each profile."""
    cases = (
        (1, 1, ""),
        (0, 0, ""),
        (1, 0, ""),
        (0, 1, ""),
        (2, 1, ""),
        (1, 1, "dev"),
        (1, 1, "release"),
    )
    for dev_count, release_count, fail_profile in cases:
        result, commands = run_fixture(dev_count, release_count, fail_profile)
        case = (
            f"Counts {dev_count} dev and {release_count} release. Cargo failure {fail_profile!r}."
        )
        should_pass = dev_count == release_count == 1 and not fail_profile
        if (result.returncode == 0) != should_pass:
            raise SystemExit(
                f"{case} Unexpected exit {result.returncode}.\n{result.stderr.strip()}"
            )
        if commands != [COMMAND, f"{COMMAND} --release"]:
            raise SystemExit(f"{case} Unexpected Cargo commands {commands!r}.")
        for profile, count in (("dev", dev_count), ("release", release_count)):
            message = f"Expected 1 passing mimalloc unwind test in the {profile} profile."
            if count != 1 and message not in result.stderr:
                raise SystemExit(f"{case} Missing diagnostic. {message}")
    print("Mimalloc unwind Cargo stub checks passed.")


if __name__ == "__main__":
    main()
