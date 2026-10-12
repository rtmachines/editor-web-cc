#!/usr/bin/env python3
"""Coverage gate: at least 80 % line coverage per crate (reliability.md section 6).

Usage:
    cargo llvm-cov --workspace --all-features --json --summary-only --output-path <report>
    python3 -I tools/coverage-gate.py <report>

The script reads the JSON summary of cargo llvm-cov. It adds the line counts
of each file to the crate in `crates/<name>/` that holds the file. A crate
below the target fails the gate. A crate with no executable line passes and
shows "no code". The lines of an inline `#[cfg(test)]` module count as lines
of the crate.

Exit 0 when every crate meets the target. Exit 1 when a crate is below the
target. Exit 2 when the report or the workspace layout is not readable, or
when the report has lines but no file maps to a crate.
"""
import json
import pathlib
import sys

# reliability.md section 6: 80 % line coverage per crate.
TARGET_PERCENT = 80

def workspace_crates(root):
    """Return the names of the directories in crates/ that hold a Cargo.toml."""
    return sorted(p.parent.name for p in (root / "crates").glob("*/Cargo.toml"))


def crate_of_file(filename, crates):
    """Return the crate of a source file, or None.

    The path is `.../crates/<crate>/...`. PureWindowsPath accepts both path
    separators, so the same code reads a Windows report and a Linux report.
    The last match wins, so a "crates" directory above the workspace root
    does not change the result.
    """
    parts = pathlib.PureWindowsPath(filename).parts
    found = None
    for directory, crate in zip(parts, parts[1:]):
        if directory == "crates" and crate in crates:
            found = crate
    return found


def report_lines(report):
    """Return the number of executable lines of every file in the report."""
    return sum(entry["summary"]["lines"]["count"] for export in report["data"] for entry in export["files"])


def line_counts(report, crates):
    """Return {crate: [covered, count]} from an llvm-cov JSON summary."""
    totals = {}
    for export in report["data"]:
        for entry in export["files"]:
            crate = crate_of_file(entry["filename"], crates)
            if crate is None:
                continue
            lines = entry["summary"]["lines"]
            pair = totals.setdefault(crate, [0, 0])
            pair[0] += lines["covered"]
            pair[1] += lines["count"]
    return totals


def main(argv):
    if len(argv) != 1:
        print(__doc__, file=sys.stderr)
        return 2
    crates = workspace_crates(pathlib.Path.cwd())
    if not crates:
        print("coverage-gate: no crate found in crates/. Run the gate from the workspace root.", file=sys.stderr)
        return 2

    try:
        report = json.loads(pathlib.Path(argv[0]).read_text(encoding="utf-8"))
        totals = line_counts(report, set(crates))
        all_lines = report_lines(report)
    except (OSError, ValueError, KeyError, TypeError) as error:
        print(f"coverage-gate: cannot read the report: {error}", file=sys.stderr)
        return 2

    # A gate that maps no file to a crate checks nothing. Stop with an error.
    mapped_lines = sum(count for _, count in totals.values())
    if not report["data"] or (all_lines > 0 and mapped_lines == 0):
        print("coverage-gate: no file of the report maps to a crate in crates/.", file=sys.stderr)
        return 2

    failed = []
    print(f"{'crate':<16} {'covered':>8} {'lines':>8} {'percent':>8}  result")
    for crate in crates:
        covered, count = totals.get(crate, [0, 0])
        if count == 0:
            print(f"{crate:<16} {covered:>8} {count:>8} {'-':>8}  no code")
            continue
        # Integer comparison: covered / count >= TARGET_PERCENT / 100.
        passed = covered * 100 >= TARGET_PERCENT * count
        percent = f"{covered * 100 / count:.2f}"
        print(f"{crate:<16} {covered:>8} {count:>8} {percent:>8}  {'ok' if passed else 'BELOW TARGET'}")
        if not passed:
            failed.append(crate)

    if failed:
        print(f"\ncoverage-gate: {len(failed)} crate(s) below {TARGET_PERCENT} %: {', '.join(failed)}")
        return 1
    print(f"\ncoverage-gate: every crate meets {TARGET_PERCENT} % line coverage or has no code.")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
