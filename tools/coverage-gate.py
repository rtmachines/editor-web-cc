#!/usr/bin/env python3
"""Require at least 80 % production line coverage per crate.

Usage from the workspace root:
    cargo llvm-cov --workspace --all-features --locked --json --output-path <report>
    python3 -I -B tools/coverage-gate.py <report>
    python3 -I -B tools/coverage-gate.py --source-only
    python3 -I -B tools/coverage-gate.py --write-inventory <path>
    python3 -I -B tools/coverage-gate.py --report <report> --inventory <path>

The first form is the legacy form. It builds the inventory itself.
The --write-inventory form writes the inventory as JSON with sorted keys. It creates a missing parent directory.
The last form is the CI form. It reads the inventory file and rebuilds the inventory.
A file that differs from the rebuilt inventory exits 2. An entry with a wrong type also exits 2.
The root is the root field of the file.

The gate lists the tracked Rust files under crates/<crate>/src/ with git ls-files
before it reads coverage. An untracked source file exits 1 without a report entry, and 2 with one.
A directory outside Git exits 2.
The predicates test and feature = "memory" mark test-only modules.
An any predicate needs every argument to be test-only. An all predicate needs
one test-only argument. A not predicate never marks test code.
An external test module excludes its file and all files in its module directory.
A production declaration keeps the shared module and its production descendants.
The gate rejects inline test modules in production files.
It also rejects production files that the default cargo-llvm-cov filter omits:
tests/, examples/, and benches/ directories, tests.rs, *_tests.rs, and *-tests.rs.
The gate removes test files before it adds coverage lines.
A crate with production files and no reported line fails.
Only a crate without production files can show "no code".
A file with only declarations needs no report entry. A positive reported line count makes it required.
The --source-only option checks source rules without a coverage report.
The gate resolves each report path against one root. The root is the directory of
cargo_llvm_cov.manifest_path, or the current directory without that field.
A manifest directory that differs from the inventory root exits 2.
A cargo_llvm_cov value that is not an object exits 2.
A path outside the root is third-party code. A path inside the root and outside
crates/*/src/ is outside the metric. The summary lists both and adds neither.
A path under crates/*/src/ that is not in the inventory, and a path with a ".." segment, exit 2.

The summary shows covered lines, total lines, the percentage, and the uncovered lines
of each crate. A crate that passes but has a missing or zero-line required file shows incomplete in its row.
It names each required file that has uncovered lines. It also lists the
declaration-only, other-configuration, outside-metric, and third-party paths.
It lists the build scripts under build-time code outside the metric, not counted.
The 100 % line goal is not a gate. No output line claims it.

Exit codes:
| Exit | Meaning |
|---|---|
| 0 | Every check passes. Accounting is complete and each crate has at least 80 % lines. |
| 1 | Coverage deficit or incomplete accounting: a crate below 80 %, a required file without lines, a source rule. |
| 2 | Input or policy error: bad report, path, counter, policy, inventory, or argument, no Git work tree, #[path]. |
Exit 2 wins over exit 1. The gate prints no percentage after an input or policy error.
"""

import argparse
import importlib.util
import json
import pathlib
import re
import sys
from typing import Any, NamedTuple


TARGET_PERCENT = 80


def load_inventory() -> Any:
    """Load coverage_inventory.py by file path, because python3 -I removes the script directory."""
    path = pathlib.Path(__file__).resolve().with_name("coverage_inventory.py")
    spec = importlib.util.spec_from_file_location("coverage_inventory", path)
    if spec is None or spec.loader is None:
        raise ValueError(f"cannot load {path.as_posix()}")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


_INVENTORY = load_inventory()
SCHEMA_VERSION = _INVENTORY.SCHEMA_VERSION
Token = _INVENTORY.Token
PAIRS = _INVENTORY.PAIRS
rust_tokens = _INVENTORY.rust_tokens
group_end = _INVENTORY.group_end


class Module(NamedTuple):
    directory: pathlib.Path
    name: str
    line: int
    inline: bool
    test_only: bool


def arguments(tokens: list[Token]) -> list[list[Token]]:
    """Split predicate arguments at commas outside nested groups."""
    result = []
    start, index = 0, 0
    while index < len(tokens):
        value = tokens[index].value
        if value in PAIRS:
            index = group_end(tokens, index)
            continue
        if value == ",":
            result.append(tokens[start:index])
            start = index + 1
        index += 1
    if start < len(tokens):
        result.append(tokens[start:])
    return result


def string_value(literal: str) -> str | None:
    """Decode a Rust string literal without changing raw string contents."""
    raw = re.fullmatch(r'r(?P<hashes>#*)"(.*)"(?P=hashes)', literal, re.DOTALL)
    if raw is not None:
        return raw[2]
    if not literal.startswith('"') or not literal.endswith('"'):
        return None
    escapes = {"n": "\n", "r": "\r", "t": "\t", "0": "\0",
               "\\": "\\", '"': '"', "'": "'"}

    def decode(match: re.Match[str]) -> str:
        escape = match[1]
        if escape.startswith("\n"):
            return ""
        if escape.startswith("x"):
            return chr(int(escape[1:], 16))
        if escape.startswith("u{"):
            return chr(int(escape[2:-1].replace("_", ""), 16))
        if escape in escapes:
            return escapes[escape]
        raise ValueError("unsupported escape in cfg string literal")

    return re.sub(r'\\(x[0-9a-fA-F]{2}|u\{[0-9a-fA-F_]+\}|\n[ \t\r\n]*|.)',
                  decode, literal[1:-1], flags=re.DOTALL)


def test_predicate(tokens: list[Token]) -> bool:
    """Apply the test-only predicate rules of design D1."""
    values = [token.value for token in tokens]
    if values == ["test"]:
        return True
    if len(values) == 3 and values[:2] == ["feature", "="]:
        return string_value(values[2]) == "memory"
    if len(values) < 3 or values[1] != "(" or values[-1] != ")":
        return False
    if values[0] not in {"any", "all"}:
        return False
    predicates = [test_predicate(arg) for arg in arguments(tokens[2:-1])]
    return all(predicates) if values[0] == "any" else any(predicates)


def test_attribute(tokens: list[Token]) -> bool:
    """Recognize an outer cfg attribute with a test-only predicate."""
    values = [token.value for token in tokens]
    return (len(values) >= 4 and values[:2] == ["cfg", "("]
            and values[-1] == ")" and test_predicate(tokens[2:-1]))


def path_attribute(tokens: list[Token]) -> bool:
    """Distinguish module paths from parameters of unrelated attributes."""
    if not tokens:
        return False
    if tokens[0].value == "path":
        return True
    if len(tokens) >= 3 and tokens[0].value == "cfg_attr" and tokens[1].value == "(":
        return any(path_attribute(arg) for arg in arguments(tokens[2:-1])[1:])
    return False


def check_path_attributes(tokens: list[Token]) -> None:
    """Reject path attributes, including paths inside cfg_attr."""
    for index, token in enumerate(tokens):
        if token.value != "#":
            continue
        start = index + 1
        if start < len(tokens) and tokens[start].value == "!":
            start += 1
        if start >= len(tokens) or tokens[start].value != "[":
            continue
        end = group_end(tokens, start)
        attribute = tokens[start + 1:end - 1]
        if path_attribute(attribute):
            raise ValueError(f"{token.line}: unsupported #[path] attribute")


def modules(tokens: list[Token], directory: pathlib.Path) -> list[Module]:
    """Read module items and their attributes within one module scope."""
    found = []
    index, test_only = 0, False
    while index < len(tokens):
        value = tokens[index].value
        if value == "#" and index + 1 < len(tokens):
            start = index + 1
            inner = tokens[start].value == "!"
            start += int(inner)
            if start < len(tokens) and tokens[start].value == "[":
                end = group_end(tokens, start)
                test_only |= not inner and test_attribute(tokens[start + 1:end - 1])
                index = end
                continue
        if value == "pub":
            index += 1
            if index < len(tokens) and tokens[index].value == "(":
                index = group_end(tokens, index)
            continue
        if value == "mod" and index + 2 < len(tokens):
            name, delimiter = tokens[index + 1:index + 3]
            if delimiter.value in {";", "{"}:
                inline = delimiter.value == "{"
                found.append(Module(directory, name.value.removeprefix("r#"),
                                    tokens[index].line, inline, test_only))
                end = group_end(tokens, index + 2) if inline else index + 3
                if inline:
                    found.extend(modules(tokens[index + 3:end - 1],
                                         directory / name.value.removeprefix("r#")))
                index, test_only = end, False
                continue
        test_only = False
        index = group_end(tokens, index) if value in PAIRS else index + 1
    return found


def workspace_crates(root: pathlib.Path) -> list[str]:
    """Return the names of directories in crates/ that hold a Cargo.toml."""
    return sorted(p.parent.name for p in (root / "crates").glob("*/Cargo.toml"))


def read_modules(path: pathlib.Path) -> list[Module]:
    directory = path.parent if path.name in {"lib.rs", "main.rs", "mod.rs"} else path.with_suffix("")
    try:
        tokens = rust_tokens(path.read_text(encoding="utf-8"))
        check_path_attributes(tokens)
        return modules(tokens, directory)
    except (OSError, ValueError) as error:
        raise ValueError(f"{path.as_posix()}: {error}") from error


def test_files(declarations: dict[pathlib.Path, list[Module]]) -> set[pathlib.Path]:
    """Collect external test modules and every source file in their directories."""
    excluded = set()
    for items in declarations.values():
        for item in items:
            if item.inline or not item.test_only:
                continue
            directory = item.directory / item.name
            module_file = directory.with_name(directory.name + ".rs")
            excluded.update(path for path in declarations
                            if path == module_file or directory in path.parents)
    pending = list(declarations.keys() - excluded)
    while pending:
        path = pending.pop()
        for item in declarations[path]:
            if item.inline or item.test_only:
                continue
            directory = item.directory / item.name
            candidates = (directory.with_name(directory.name + ".rs"), directory / "mod.rs")
            for candidate in candidates:
                if candidate in excluded:
                    excluded.remove(candidate)
                    pending.append(candidate)
    return excluded


def default_ignored(path: pathlib.Path) -> bool:
    """Copy the cargo-llvm-cov 0.6.24 default test filename filter."""
    return re.search(
        r"(^|/)(tests|examples|benches)/|/(tests\.rs|[0-9a-zA-Z_-]+[_-]tests\.rs)$",
        path.as_posix(),
    ) is not None


def build_inventory(root: pathlib.Path) -> tuple[dict, dict[pathlib.Path, list[Module]]]:
    """Build the inventory of the tracked sources. Return the declarations of each file with it."""
    declarations: dict[pathlib.Path, list[Module]] = {}

    def classify(paths: list[pathlib.Path]) -> set[pathlib.Path]:
        for path in paths:
            declarations[path] = read_modules(path)
        return test_files(declarations)

    module = load_inventory()
    return module.source_inventory(root, module.load_policy(root), classify), declarations


def paths_of(inventory: dict, classification: str) -> set[pathlib.Path]:
    """Collect the paths of one classification from the inventory."""
    return {pathlib.Path(e["path"]) for e in inventory["files"] if e["classification"] == classification}


def source_violations(inventory: dict, declarations: dict[pathlib.Path, list[Module]]) -> list[str]:
    """List the source rule violations of ADR-0127 (e) and (f) and the untracked files."""
    excluded = paths_of(inventory, "test_only")
    violations = [f"{path}:1: untracked source file" for path in inventory["untracked"]]
    for path, items in declarations.items():
        if path in excluded:
            continue
        if default_ignored(path):
            violations.append(f"{path.as_posix()}:1: production file matches the default coverage filter")
        for item in items:
            if item.test_only and item.inline:
                violations.append(f"{path.as_posix()}:{item.line}: inline test module {item.name}")
    return violations


def source_scan(root: pathlib.Path) -> tuple[dict, list[str]]:
    """Build the inventory from Git and list the source violations. Untracked files are violations."""
    inventory, declarations = build_inventory(root)
    return inventory, source_violations(inventory, declarations)


class ReportPath(NamedTuple):
    """Place of one report entry: source, outside_metric, or third_party."""

    kind: str
    path: pathlib.Path | None


class Outside(NamedTuple):
    """Report entries that the metric does not count, as (filename, line count) pairs."""

    third_party: list[tuple[str, int]]
    outside_metric: list[tuple[str, int]]


class Listed(NamedTuple):
    """Paths that the summary lists and the metric does not count."""

    declaration_only: set[pathlib.Path]
    other_configuration: dict[pathlib.Path, str]
    outside: Outside
    build_scripts: tuple[str, ...] = ()


def pure_path(text: str) -> pathlib.PurePath:
    """Parse text as a Windows path when it has a drive letter or a backslash. Else parse it as POSIX."""
    windows = re.match(r"[A-Za-z]:", text) is not None or "\\" in text
    return (pathlib.PureWindowsPath if windows else pathlib.PurePosixPath)(text)


def resolve_root(report: Any, inventory_root: pathlib.PurePath) -> pathlib.PurePath:
    """Take the manifest directory of the report as root. Without it, take the inventory root.

    A manifest directory that differs from the inventory root is an input error.
    """
    if not isinstance(report, dict) or "cargo_llvm_cov" not in report:
        return inventory_root
    meta = report["cargo_llvm_cov"]
    if not isinstance(meta, dict):
        raise ValueError("cargo_llvm_cov must be an object")
    if "manifest_path" not in meta:
        return inventory_root
    manifest = meta["manifest_path"]
    if not isinstance(manifest, str) or not manifest:
        raise ValueError("manifest_path must be a non-empty string")
    root = pure_path(manifest).parent
    if root != inventory_root:
        raise ValueError(f"the manifest directory {root} differs from the root {inventory_root}")
    return root


def report_path(filename: str, root: pathlib.PurePath, known: set[pathlib.Path],
                untracked: frozenset[str] = frozenset()) -> ReportPath:
    """Place a report path against the root. The root decides how the path is parsed.

    A path outside the root is third-party code. A path under crates/*/src/ must be in the inventory.
    An untracked path in the report gets its own message.
    """
    path = type(root)(filename)
    if ".." in path.parts:
        raise ValueError(f"{filename}: path with a parent segment")
    if path.is_absolute():
        if not path.is_relative_to(root):
            return ReportPath("third_party", None)
        path = path.relative_to(root)
    elif path.anchor:
        raise ValueError(f"{filename}: path with a drive or root but no absolute form")
    parts = path.parts
    if not parts:
        raise ValueError(f"{filename}: path without a file name")
    if len(parts) < 4 or parts[0] != "crates" or parts[2] != "src":
        return ReportPath("outside_metric", None)
    workspace_path = pathlib.Path(*parts)
    if workspace_path not in known:
        if workspace_path.as_posix() in untracked:
            raise ValueError(f"{workspace_path.as_posix()}: the report holds an untracked source file")
        raise ValueError(f"{filename}: path under crates/*/src/ is not in the source inventory")
    return ReportPath("source", workspace_path)


def member(container: Any, key: str, kind: type, where: str) -> Any:
    """Return container[key] when it exists and has the type kind. Else raise an input error."""
    if not isinstance(container, dict):
        raise ValueError(f"{where}: expected an object")
    if key not in container:
        raise ValueError(f"{where}: missing key {key}")
    value = container[key]
    if not isinstance(value, kind):
        raise ValueError(f"{where}: {key} has the wrong type")
    return value


def line_counter(lines: Any, key: str, where: str) -> int:
    """Return one line counter. It must be an int and not a Boolean. A missing counter is an error."""
    value = member(lines, key, object, where)
    if type(value) is not int or value < 0:
        raise ValueError(f"{where}: {key} is not a non-negative integer")
    return value


def report_entries(report: Any) -> list[tuple[str, int, int]]:
    """Check the shape of the whole report. Return (filename, covered, count) for each entry.

    The report has exactly one export object. Each counter pair has 0 <= covered <= count.
    """
    exports = member(report, "data", list, "report")
    if len(exports) != 1:
        raise ValueError(f"data must hold exactly one export object, found {len(exports)}")
    files = member(exports[0], "files", list, "data[0]")
    entries = []
    for index, entry in enumerate(files):
        filename = member(entry, "filename", str, f"files[{index}]")
        lines = member(member(entry, "summary", dict, filename), "lines", dict, f"{filename} summary")
        covered = line_counter(lines, "covered", filename)
        count = line_counter(lines, "count", filename)
        if covered > count:
            raise ValueError(f"{filename}: covered {covered} is above count {count}")
        entries.append((filename, covered, count))
    return entries


def normal_path(filename: str, root: pathlib.PurePath) -> pathlib.PurePath:
    """Return the path of an entry relative to the root. A path outside the root stays absolute."""
    path = type(root)(filename)
    return path.relative_to(root) if path.is_absolute() and path.is_relative_to(root) else path


def reject_duplicates(entries: list[tuple[str, int, int]], root: pathlib.PurePath) -> None:
    """Raise an input error when two entries have one normalised path."""
    seen: dict[pathlib.PurePath, str] = {}
    for filename, _covered, _count in entries:
        key = normal_path(filename, root)
        if key in seen:
            raise ValueError(f"duplicate report entries for one path: {seen[key]} and {filename}")
        seen[key] = filename


def line_counts(
    report: Any, root: pathlib.PurePath, known: set[pathlib.Path], excluded: set[pathlib.Path],
    untracked: frozenset[str] = frozenset(),
) -> tuple[dict[str, list[int]], dict[pathlib.Path, tuple[int, int]], Outside]:
    """Check the whole report first. Then sum production coverage per crate and per file."""
    entries = report_entries(report)
    placed = [(filename, report_path(filename, root, known, untracked), covered, count)
              for filename, covered, count in entries]
    reject_duplicates(entries, root)
    totals: dict[str, list[int]] = {}
    reported: dict[pathlib.Path, tuple[int, int]] = {}
    outside = Outside([], [])
    all_lines = sum(count for _filename, _place, _covered, count in placed)
    mapped_lines = 0
    for filename, place, covered, count in placed:
        if place.path is None:
            target = outside.third_party if place.kind == "third_party" else outside.outside_metric
            target.append((filename, count))
            continue
        path = place.path
        mapped_lines += count
        if path in excluded:
            continue
        reported[path] = (covered, count)
        pair = totals.setdefault(path.parts[1], [0, 0])
        pair[0] += covered
        pair[1] += count
    if all_lines > 0 and mapped_lines == 0:
        raise ValueError("no file of the report maps to a crate in crates/")
    return totals, reported, outside


def print_list(title: str, lines: list[str]) -> None:
    """Print a titled list and one blank line. Print nothing for an empty list."""
    if lines:
        print(f"coverage-gate: {title}:")
        print("\n".join(lines))
        print()


def print_listed(listed: Listed) -> None:
    """List the declaration-only, other-configuration, outside-metric, third-party, and build-time paths."""
    print_list("declaration-only files, not counted", sorted(p.as_posix() for p in listed.declaration_only))
    print_list("other-configuration files, not counted", sorted(f"{p.as_posix()} ({name})" for p, name in listed.other_configuration.items()))
    outside = listed.outside
    print_list("outside the metric, inside the root, not counted",
               [f"{filename} {count} lines" for filename, count in outside.outside_metric])
    print_list("third-party code outside the root, not counted",
               [f"{filename} {count} lines" for filename, count in outside.third_party])
    print_list("build-time code outside the metric, not counted", list(listed.build_scripts))


def unreported_files(
    production: set[pathlib.Path], reported: dict[pathlib.Path, tuple[int, int]]
) -> list[str]:
    """List each production file that has no report entry or a line count of 0."""
    return sorted(path.as_posix() for path in production if reported.get(path, (0, 0))[1] == 0)


def print_uncovered_files(production: set[pathlib.Path], reported: dict[pathlib.Path, tuple[int, int]]) -> None:
    """List each required file with uncovered lines. Put the largest gap first."""
    gaps = []
    for path in production:
        covered, count = reported.get(path, (0, 0))
        if count > covered:
            gaps.append((count - covered, path.as_posix()))
    gaps.sort(key=lambda gap: (-gap[0], gap[1]))
    print_list("required files with uncovered lines",
               [f"{path} {uncovered} uncovered lines" for uncovered, path in gaps])


def crate_table(
    crates: list[str], production: set[pathlib.Path], totals: dict[str, list[int]], missing: list[str]
) -> list[str]:
    """Print one row for each crate. Return the crates that fail.

    A crate that meets the target but misses a required file or holds a zero-line one is incomplete, never ok.
    """
    failed = []
    incomplete = {pathlib.PurePosixPath(path).parts[1] for path in missing}
    crates_with_code = {path.parts[1] for path in production}
    print(f"{'crate':<16} {'covered':>8} {'lines':>8} {'percent':>8} {'uncovered':>10}  result")
    for crate in crates:
        covered, count = totals.get(crate, [0, 0])
        percent = f"{covered * 100 / count:.2f}" if count else "-"
        if count == 0:
            passed = crate not in crates_with_code
            result = "no code" if passed else "NO REPORTED LINES"
        else:
            passed = covered * 100 >= TARGET_PERCENT * count
            result = "ok" if passed else "BELOW TARGET"
        if passed and crate in incomplete:
            passed, result = False, "incomplete"
        print(f"{crate:<16} {covered:>8} {count:>8} {percent:>8} {count - covered:>10}  {result}")
        if not passed:
            failed.append(crate)
    return failed


def coverage_result(
    crates: list[str],
    production: set[pathlib.Path],
    totals: dict[str, list[int]],
    reported: dict[pathlib.Path, tuple[int, int]],
    listed: Listed,
) -> int:
    """Print the summary and return 0 or 1. Exit 2 never reaches this function."""
    missing = unreported_files(production, reported)
    failed = crate_table(crates, production, totals, missing)
    print()
    print_uncovered_files(production, reported)
    print_list("required files without reported lines", missing)
    print_listed(listed)
    if failed:
        print(f"coverage-gate: {len(failed)} crate(s) fail coverage: {', '.join(failed)}")
        return 1
    if missing:
        return 1
    print(f"coverage-gate: every crate meets {TARGET_PERCENT} % line coverage or has no code.")
    return 0


def validate_report(report: Any, inventory: dict) -> dict:
    """Check the report against the inventory and collect the line results.

    An input or policy error raises ValueError. A coverage deficit is the business of coverage_result.
    """
    root = resolve_root(report, pure_path(inventory["root"]))
    excluded = paths_of(inventory, "test_only")
    declaration_only = paths_of(inventory, "declaration_only")
    other_configuration = paths_of(inventory, "other_configuration")
    production = paths_of(inventory, "required")
    known = excluded | production | declaration_only | other_configuration
    totals, reported, outside = line_counts(report, root, known, excluded, frozenset(inventory["untracked"]))
    held = sorted(path.as_posix() for path in other_configuration if path in reported)
    if held:
        raise ValueError(f"unused policy entry, the report holds {', '.join(held)}")
    production |= {path for path in declaration_only if reported.get(path, (0, 0))[1] > 0}
    configurations = {pathlib.Path(e["path"]): str(e.get("configuration", ""))
                      for e in inventory["files"] if e["classification"] == "other_configuration"}
    listed = Listed(declaration_only - production, configurations, outside,
                    tuple(inventory["build_scripts"]))
    return {"schema_version": SCHEMA_VERSION, "totals": totals, "reported": reported,
            "production": production, "listed": listed}


def load_inventory_file(path: str) -> dict:
    """Read an inventory file. A bad file, version, or shape raises ValueError."""
    try:
        stored = json.loads(pathlib.Path(path).read_text(encoding="utf-8"))
    except (OSError, ValueError) as error:
        raise ValueError(f"cannot read the inventory file {path}: {error}") from error
    if not isinstance(stored, dict):
        raise ValueError(f"the inventory file {path} must hold an object")
    version = stored.get("schema_version")
    if isinstance(version, bool) or version != SCHEMA_VERSION:
        raise ValueError(f"the inventory file {path} has schema_version {version!r}, expected {SCHEMA_VERSION}")
    for key, kind in (("root", str), ("files", list), ("untracked", list), ("build_scripts", list)):
        if not isinstance(stored.get(key), kind):
            raise ValueError(f"the inventory file {path} needs the key {key} of type {kind.__name__}")
    check_inventory_entries(path, stored)
    return stored


def check_inventory_entries(path: str, stored: dict) -> None:
    """Require the field types that the writer produces. A bad entry raises ValueError."""
    for index, item in enumerate(stored["files"]):
        fields = ("path", "crate", "classification", "sha256")
        if not isinstance(item, dict) or not all(isinstance(item.get(name), str) for name in fields):
            raise ValueError(f"the inventory file {path} has a bad files entry at index {index}")
        if not isinstance(item.get("configuration", ""), str):
            raise ValueError(f"the inventory file {path} has a bad configuration at files index {index}")
    for key in ("untracked", "build_scripts"):
        for index, item in enumerate(stored[key]):
            if not isinstance(item, str):
                raise ValueError(f"the inventory file {path} has a bad {key} entry at index {index}")


def inventory_differences(stored: dict, rebuilt: dict) -> list[str]:
    """Name each difference between the stored and the rebuilt inventory."""
    found = ["root differs"] if stored["root"] != rebuilt["root"] else []
    changed = set(map(str, stored["untracked"])) ^ set(rebuilt["untracked"])
    found += [f"{path} is untracked in only one of them" for path in sorted(changed)]
    changed = set(map(str, stored["build_scripts"])) ^ set(rebuilt["build_scripts"])
    found += [f"{path} is a build script in only one of them" for path in sorted(changed)]
    old = {item.get("path"): item for item in stored["files"] if isinstance(item, dict)}
    new = {item["path"]: item for item in rebuilt["files"]}
    found += [f"{path} is not in the file" for path in sorted(set(new) - set(old))]
    found += [f"{path} is not in the tree" for path in sorted(set(old) - set(new), key=str)]
    found += [f"{path} has another hash, crate, or classification" for path in sorted(set(old) & set(new))
              if old[path] != new[path]]
    if not found and stored != rebuilt:
        found.append("the file order or its fields differ")
    return found


def check_inventory_file(path: str, rebuilt: dict) -> None:
    """Require that the file equals the rebuilt inventory. A difference raises ValueError."""
    differences = inventory_differences(load_inventory_file(path), rebuilt)
    if differences:
        raise ValueError(f"the inventory file {path} is stale: {', '.join(differences)}")


def parse_arguments(argv: list[str]) -> argparse.Namespace:
    """Parse the four forms. Bad arguments print the usage on stderr and exit with 2."""
    parser = argparse.ArgumentParser(
        prog="coverage-gate.py", allow_abbrev=False,
        usage="coverage-gate.py <report> | --source-only | --write-inventory <path>"
              " | --report <report> --inventory <path>")
    parser.add_argument("legacy_report", nargs="?", metavar="report")
    parser.add_argument("--source-only", action="store_true")
    parser.add_argument("--write-inventory", metavar="path")
    parser.add_argument("--report", metavar="report")
    parser.add_argument("--inventory", metavar="path")
    args = parser.parse_args(argv)
    forms = [args.legacy_report is not None, args.source_only, args.write_inventory is not None,
             args.report is not None or args.inventory is not None]
    if forms.count(True) != 1:
        parser.error("use exactly one form")
    if (args.report is None) != (args.inventory is None):
        parser.error("--report and --inventory go together")
    return args


def write_inventory(path: str, inventory: dict) -> None:
    """Write the inventory as JSON with sorted keys and a final newline."""
    target = pathlib.Path(path)
    target.parent.mkdir(parents=True, exist_ok=True)
    target.write_text(json.dumps(inventory, indent=2, sort_keys=True) + "\n", encoding="utf-8")


def main(argv: list[str]) -> int:
    args = parse_arguments(argv)
    root = pathlib.Path.cwd()
    try:
        crates = workspace_crates(root)
        if not crates:
            raise ValueError("no crate found in crates/. Run the gate from the workspace root.")
        inventory, declarations = build_inventory(root)
        if args.write_inventory is not None:
            write_inventory(args.write_inventory, inventory)
            return 0
        if args.inventory is not None:
            check_inventory_file(args.inventory, inventory)
    except (OSError, ValueError) as error:
        print(f"coverage-gate: cannot scan sources: {error}", file=sys.stderr)
        return 2
    violations = source_violations(inventory, declarations)
    for violation in violations:
        print(violation, file=sys.stderr)
    if args.source_only:
        return int(bool(violations))
    try:
        report = json.loads(pathlib.Path(args.report or args.legacy_report).read_text(encoding="utf-8"))
        result = validate_report(report, inventory)
    except (OSError, ValueError, KeyError, TypeError) as error:
        print(f"coverage-gate: cannot read the report: {error}", file=sys.stderr)
        return 2
    return max(int(bool(violations)), coverage_result(
        crates, result["production"], result["totals"], result["reported"], result["listed"]))


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
