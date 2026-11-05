#!/usr/bin/env python3
"""Require at least 80 % production line coverage per crate.

Usage from the workspace root:
    cargo llvm-cov --workspace --all-features --json --summary-only --output-path <report>
    python3 -I tools/coverage-gate.py <report>
    python3 -I tools/coverage-gate.py --source-only

The gate scans every Rust file under crates/<crate>/src/ before reading coverage.
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
The --source-only option checks source rules without a coverage report.

Exit 0 means that all applicable checks pass. Exit 1 means a source rule or
coverage check failed. Exit 2 means unreadable input, an invalid report or
workspace layout, or an unsupported #[path] attribute anywhere in src/.
"""

import json
import pathlib
import re
import sys
from typing import Any, NamedTuple


TARGET_PERCENT = 80
TOKEN = re.compile(
    r'(?P<space>\s+)|(?P<comment>//[^\n]*|/\*)'
    r'|(?P<raw>(?:br|cr|r)(?P<hashes>\#*)".*?"(?P=hashes))'
    r'|(?P<string>(?:b|c)?"(?:\\.|[^"\\])*")'
    r"|(?P<char>b?'(?:\\(?:u\{[0-9a-fA-F_]+\}|x[0-9a-fA-F]{2}|.)|[^'\\])')"
    r'|(?P<word>(?:r\#)?[A-Za-z_][A-Za-z_0-9]*)|(?P<other>.)',
    re.DOTALL,
)
PAIRS = {"(": ")", "[": "]", "{": "}"}


class Token(NamedTuple):
    value: str
    line: int


class Module(NamedTuple):
    directory: pathlib.Path
    name: str
    line: int
    inline: bool
    test_only: bool


def comment_end(source: str, start: int) -> int:
    """Find the end of a Rust block comment, including nested comments."""
    depth = 1
    for match in re.finditer(r"/\*|\*/", source[start + 2:]):
        depth += 1 if match[0] == "/*" else -1
        if depth == 0:
            return start + 2 + match.end()
    raise ValueError("unterminated block comment")


def rust_tokens(source: str) -> list[Token]:
    """Keep literals opaque and preserve declaration line numbers."""
    result = []
    offset, line = 0, 1
    while offset < len(source):
        match = TOKEN.match(source, offset)
        if match is None:
            raise ValueError(f"cannot read Rust token at line {line}")
        end = comment_end(source, offset) if match[0] == "/*" else match.end()
        if match.lastgroup not in {"space", "comment"}:
            if match[0] == '"':
                raise ValueError(f"unterminated string at line {line}")
            result.append(Token(match[0], line))
        line += source.count("\n", offset, end)
        offset = end
    return result


def group_end(tokens: list[Token], start: int) -> int:
    """Return the index after a balanced group."""
    stack = []
    for index in range(start, len(tokens)):
        value = tokens[index].value
        if value in PAIRS:
            stack.append(PAIRS[value])
        elif value in PAIRS.values():
            if not stack or stack.pop() != value:
                raise ValueError(f"unbalanced delimiter at line {tokens[index].line}")
            if not stack:
                return index + 1
    raise ValueError(f"unterminated group at line {tokens[start].line}")


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


def source_scan(crates: list[str]) -> tuple[set[pathlib.Path], set[str], list[str]]:
    declarations = {}
    for crate in crates:
        for path in sorted((pathlib.Path("crates") / crate / "src").rglob("*.rs")):
            declarations[path] = read_modules(path)
    excluded = test_files(declarations)
    production = set()
    violations = []
    for path, items in declarations.items():
        if path in excluded:
            continue
        production.add(path.parts[1])
        if default_ignored(path):
            violations.append(f"{path.as_posix()}:1: production file matches the default coverage filter")
        for item in items:
            if item.test_only and item.inline:
                violations.append(f"{path.as_posix()}:{item.line}: inline test module {item.name}")
    return excluded, production, violations


def workspace_file(filename: str, crates: set[str]) -> pathlib.Path | None:
    """Map Windows or Linux report paths to workspace paths. The last match wins."""
    parts = pathlib.PureWindowsPath(filename).parts
    found = None
    for index, (directory, crate) in enumerate(zip(parts, parts[1:])):
        if directory == "crates" and crate in crates:
            found = pathlib.Path(*parts[index:])
    return found


def line_counts(report: Any, crates: set[str], excluded: set[pathlib.Path]) -> dict[str, list[int]]:
    """Sum production coverage after checking the raw report mapping."""
    totals: dict[str, list[int]] = {}
    all_lines, mapped_lines = 0, 0
    if not isinstance(report, dict) or not isinstance(report.get("data"), list):
        raise ValueError("expected a data array")
    for export in report["data"]:
        for entry in export["files"]:
            path = workspace_file(entry["filename"], crates)
            lines = entry["summary"]["lines"]
            covered, count = lines["covered"], lines["count"]
            if type(covered) is not int or type(count) is not int or not 0 <= covered <= count:
                raise ValueError("invalid line counts")
            all_lines += count
            if path is None:
                continue
            mapped_lines += count
            if path in excluded:
                continue
            pair = totals.setdefault(path.parts[1], [0, 0])
            pair[0] += covered
            pair[1] += count
    if not report["data"] or (all_lines > 0 and mapped_lines == 0):
        raise ValueError("no file of the report maps to a crate in crates/")
    return totals


def coverage_result(crates: list[str], production: set[str], totals: dict[str, list[int]]) -> int:
    failed = []
    print(f"{'crate':<16} {'covered':>8} {'lines':>8} {'percent':>8}  result")
    for crate in crates:
        covered, count = totals.get(crate, [0, 0])
        percent = f"{covered * 100 / count:.2f}" if count else "-"
        if count == 0:
            passed = crate not in production
            result = "no code" if passed else "NO REPORTED LINES"
        else:
            passed = covered * 100 >= TARGET_PERCENT * count
            result = "ok" if passed else "BELOW TARGET"
        print(f"{crate:<16} {covered:>8} {count:>8} {percent:>8}  {result}")
        if not passed:
            failed.append(crate)
    if failed:
        print(f"\ncoverage-gate: {len(failed)} crate(s) fail coverage: {', '.join(failed)}")
        return 1
    print(f"\ncoverage-gate: every crate meets {TARGET_PERCENT} % line coverage or has no code.")
    return 0


def main(argv: list[str]) -> int:
    if len(argv) != 1:
        print(__doc__, file=sys.stderr)
        return 2
    try:
        crates = workspace_crates(pathlib.Path.cwd())
        if not crates:
            raise ValueError("no crate found in crates/. Run the gate from the workspace root.")
        excluded, production, violations = source_scan(crates)
    except (OSError, ValueError) as error:
        print(f"coverage-gate: cannot scan sources: {error}", file=sys.stderr)
        return 2
    for violation in violations:
        print(violation, file=sys.stderr)
    if argv == ["--source-only"]:
        return int(bool(violations))
    try:
        report = json.loads(pathlib.Path(argv[0]).read_text(encoding="utf-8"))
        totals = line_counts(report, set(crates), excluded)
    except (OSError, ValueError, KeyError, TypeError) as error:
        print(f"coverage-gate: cannot read the report: {error}", file=sys.stderr)
        return 2
    return max(int(bool(violations)), coverage_result(crates, production, totals))


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
