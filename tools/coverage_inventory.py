"""List the production source files of the workspace from Git, not from a directory walk.

The gate loads this module by an explicit file path, because python3 -I
removes the script directory from sys.path.
It also holds the Rust tokenizer and the detector of declaration-only files.
"""

import hashlib
import json
import pathlib
import re
import subprocess
from typing import Callable, NamedTuple

SCHEMA_VERSION = 1
CRATE_FILES = ("crates",)
TOKEN = re.compile(
    r'(?P<space>\s+)|(?P<comment>//[^\n]*|/\*)'
    r'|(?P<raw>(?:br|cr|r)(?P<hashes>\#*)".*?"(?P=hashes))'
    r'|(?P<string>(?:b|c)?"(?:\\.|[^"\\])*")'
    r"|(?P<char>b?'(?:\\(?:u\{[0-9a-fA-F_]+\}|x[0-9a-fA-F]{2}|.)|[^'\\])')"
    r'|(?P<word>(?:r\#)?[A-Za-z_][A-Za-z_0-9]*)|(?P<other>.)',
    re.DOTALL,
)
POLICY_FILE = "tools/coverage-policy.json"
POLICY_KEYS = {"path", "sha256", "configuration", "reason", "evidence"}
SHA256 = re.compile(r"[0-9a-f]{64}")
PAIRS = {"(": ")", "[": "]", "{": "}"}


class Token(NamedTuple):
    value: str
    line: int


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


LITERAL_KINDS = {"raw", "string", "char"}
NUMBER = re.compile(r"-?\d[0-9A-Za-z_]*(\.\d[0-9A-Za-z_]*)?([eE][+-]?\d[0-9A-Za-z_]*)?")
PATH = re.compile(r"(::)?[A-Za-z_][A-Za-z_0-9]*(::[A-Za-z_][A-Za-z_0-9]*)*")
FUNCTION_WORDS = {"fn", "unsafe", "async", "extern"}
TRAIT_ITEMS = {"fn", "type", "const", "unsafe", "async", "extern"}


def split_top(tokens: list[Token], separator: str) -> list[list[Token]]:
    """Split tokens at a separator outside nested groups."""
    parts, start, index = [], 0, 0
    while index < len(tokens):
        if tokens[index].value in PAIRS:
            index = group_end(tokens, index)
            continue
        if tokens[index].value == separator:
            parts.append(tokens[start:index])
            start = index + 1
        index += 1
    return [*parts, tokens[start:]]


def simple_value(tokens: list[Token]) -> bool:
    """Accept a literal, a path, or an array of those values. Accept nothing else."""
    values = [token.value for token in tokens]
    joined = "".join(values)
    if not values:
        return False
    if len(values) == 1:
        match = TOKEN.fullmatch(values[0])
        if match is not None and match.lastgroup in LITERAL_KINDS:
            return True
    if NUMBER.fullmatch(joined) or PATH.fullmatch(joined):
        return True
    if values[0] != "[" or group_end(tokens, 0) != len(tokens):
        return False
    elements = [e for part in split_top(tokens[1:-1], ";") for e in split_top(part, ",")]
    if elements and not elements[-1]:
        elements.pop()
    return all(simple_value(element) for element in elements)


def item_end(tokens: list[Token], start: int, stop_at_brace: bool) -> int | None:
    """Return the index after the item. It ends at a semicolon or, if allowed, a brace group."""
    index = start
    while index < len(tokens):
        value = tokens[index].value
        if value == ";":
            return index + 1
        if value == "{" and stop_at_brace:
            return group_end(tokens, index)
        index = group_end(tokens, index) if value in PAIRS else index + 1
    return None


def static_end(tokens: list[Token], start: int) -> int | None:
    """Accept a const or static item with a simple initializer. Return its end index."""
    end = item_end(tokens, start, False)
    item = tokens[start + 1:end - 1] if end is not None else []
    if end is None or not item or item[0].value in FUNCTION_WORDS:
        return None
    index, depth = 0, 0
    while index < len(item):
        value = item[index].value
        if value in PAIRS:
            index = group_end(item, index)
            continue
        if value == "<":
            depth += 1
        elif value == ">" and not (index and item[index - 1].value == "-"):
            depth -= 1
        elif value == "=" and depth == 0:
            return end if simple_value(item[index + 1:]) else None
        index += 1
    return None


def trait_end(tokens: list[Token], start: int) -> int | None:
    """Accept a trait whose items have no default body and no default constant."""
    index = start
    while index < len(tokens) and tokens[index].value != "{":
        if tokens[index].value == ";":
            return None
        index = group_end(tokens, index) if tokens[index].value in PAIRS else index + 1
    if index >= len(tokens):
        return None
    end = group_end(tokens, index)
    return end if trait_items(tokens[index + 1:end - 1]) else None


def trait_items(tokens: list[Token]) -> bool:
    """Check that each trait item is a declaration that ends at a semicolon."""
    index = 0
    while index < len(tokens):
        if tokens[index].value == "#":
            start = index + 1
            if start >= len(tokens) or tokens[start].value != "[":
                return False
            index = group_end(tokens, start)
            continue
        end = item_end(tokens, index, True)
        item = tokens[index:end] if end is not None else []
        if not item or item[0].value not in TRAIT_ITEMS or item[-1].value != ";":
            return False
        if item[0].value == "const" and any(token.value == "=" for token in item):
            return False
        index = end if end is not None else len(tokens)
    return True


def has_call(tokens: list[Token]) -> bool:
    """Return True when an expression holds a call, a macro call, or a block."""
    values = [token.value for token in tokens]
    return any(
        value in {"(", "{"} or (value == "!" and index > 0 and PATH.fullmatch(values[index - 1]))
        for index, value in enumerate(values)
    )


def expressions_are_plain(tokens: list[Token], nested: bool = False) -> bool:
    """Check array lengths and enum discriminants of an item for calls."""
    index = 0
    while index < len(tokens):
        value = tokens[index].value
        if value in PAIRS:
            end = group_end(tokens, index)
            inner = tokens[index + 1:end - 1]
            lengths = split_top(inner, ";")[1:] if value == "[" else []
            if any(has_call(length) for length in lengths):
                return False
            if not expressions_are_plain(inner, True):
                return False
            index = end
            continue
        if value == "=" and nested:
            end = next((i for i in range(index, len(tokens)) if tokens[i].value == ","), len(tokens))
            if has_call(tokens[index + 1:end]):
                return False
        index += 1
    return True


def type_item_end(tokens: list[Token], start: int, stop_at_brace: bool) -> int | None:
    """Return the end of a type or struct item. Return None when it holds a call."""
    end = item_end(tokens, start, stop_at_brace)
    if end is None or not expressions_are_plain(tokens[start:end]):
        return None
    return end


def declaration_end(tokens: list[Token], index: int) -> int | None:
    """Return the index after the declaration at index, or None for any other item."""
    value = tokens[index].value
    following = tokens[index + 1].value if index + 1 < len(tokens) else ""
    if value == "#":
        start = index + 1 + int(following == "!")
        if start < len(tokens) and tokens[start].value == "[":
            return group_end(tokens, start)
        return None
    if value == "pub":
        return group_end(tokens, index + 1) if following == "(" else index + 1
    if value == "mod" and index + 2 < len(tokens):
        delimiter = tokens[index + 2].value
        if delimiter == ";":
            return index + 3
        if delimiter == "{":
            end = group_end(tokens, index + 2)
            return end if declarations_only(tokens[index + 3:end - 1]) else None
        return None
    if value == "extern":
        return item_end(tokens, index + 1, False) if following == "crate" else None
    if value == "use":
        return item_end(tokens, index + 1, False)
    if value == "type":
        return type_item_end(tokens, index + 1, False)
    if value in {"struct", "enum", "union"}:
        return type_item_end(tokens, index + 1, True)
    if value == "trait" or (value == "unsafe" and following == "trait"):
        return trait_end(tokens, index + 1)
    if value in {"const", "static"}:
        return static_end(tokens, index)
    return None


def declarations_only(tokens: list[Token]) -> bool:
    """Check that every top-level item is in the declaration list of design D1."""
    index = 0
    while index < len(tokens):
        end = declaration_end(tokens, index)
        if end is None:
            return False
        index = end
    return True


def is_declaration_only(source: str) -> bool:
    """Return True when the source has only declarations. Executable code never passes."""
    return declarations_only(rust_tokens(source))


def git_paths(root: pathlib.Path, *options: str) -> list[str]:
    """Run git ls-files under crates/ and return the NUL-separated paths."""
    command = ["git", "-C", str(root), "ls-files", "-z", *options, "--", *CRATE_FILES]
    try:
        result = subprocess.run(command, capture_output=True, check=False)
    except OSError as error:
        raise ValueError(f"cannot run git: {error}") from error
    if result.returncode != 0:
        message = result.stderr.decode("utf-8", errors="replace").strip()
        raise ValueError(f"git ls-files failed in {root.as_posix()}: {message}")
    return [item.decode("utf-8") for item in result.stdout.split(b"\0") if item]


def source_crate(path: str) -> str | None:
    """Return the crate of a crates/<crate>/src/**/*.rs path, or None for other paths."""
    parts = pathlib.PurePosixPath(path).parts
    if len(parts) < 4 or parts[0] != "crates" or parts[2] != "src":
        return None
    return parts[1] if parts[-1].endswith(".rs") else None


def is_build_script(path: str) -> bool:
    """Tell whether a path is exactly crates/<crate>/build.rs."""
    parts = pathlib.PurePosixPath(path).parts
    return len(parts) == 3 and parts[0] == "crates" and parts[2] == "build.rs"


def classification(root: pathlib.Path, path: str, test_only: set[pathlib.Path]) -> str:
    """Name the class of one tracked file: test_only, declaration_only, or required."""
    if pathlib.Path(path) in test_only:
        return "test_only"
    try:
        declarations = is_declaration_only((root / path).read_text(encoding="utf-8"))
    except (OSError, ValueError) as error:
        raise ValueError(f"{path}: {error}") from error
    return "declaration_only" if declarations else "required"


def load_policy(root: pathlib.Path) -> list[dict]:
    """Read the policy entries. An absent file is an empty list. Any other fault raises ValueError."""
    path = root / POLICY_FILE
    if not path.exists():
        return []
    try:
        document = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, ValueError) as error:
        raise ValueError(f"{POLICY_FILE}: {error}") from error
    if not isinstance(document, dict) or set(document) != {"schema_version", "entries"}:
        raise ValueError(f"{POLICY_FILE}: expected the keys schema_version and entries")
    if document["schema_version"] != SCHEMA_VERSION or not isinstance(document["entries"], list):
        raise ValueError(f"{POLICY_FILE}: bad schema_version or entries")
    return document["entries"]


def check_entry(entry: object, seen: set[str]) -> str:
    """Check the shape of one policy entry and return its path."""
    if not isinstance(entry, dict) or set(entry) != POLICY_KEYS:
        raise ValueError(f"{POLICY_FILE}: an entry needs exactly the keys {sorted(POLICY_KEYS)}")
    for key, value in entry.items():
        if not isinstance(value, str) or not value.strip():
            raise ValueError(f"{POLICY_FILE}: empty or non-text field {key}")
    path = entry["path"]
    if any(character in path for character in "*?["):
        raise ValueError(f"{POLICY_FILE}: wildcard in path {path}")
    if path in seen:
        raise ValueError(f"{POLICY_FILE}: duplicate path {path}")
    if not SHA256.fullmatch(entry["sha256"]):
        raise ValueError(f"{POLICY_FILE}: {path}: sha256 is not 64 lowercase hex characters")
    seen.add(path)
    return path


def apply_policy(files: list[dict], policy: list[dict]) -> None:
    """Mark each policy file other_configuration. Any policy fault raises ValueError."""
    seen: set[str] = set()
    by_path = {item["path"]: item for item in files}
    for entry in policy:
        path = check_entry(entry, seen)
        item = by_path.get(path)
        if item is None:
            raise ValueError(f"{POLICY_FILE}: unused entry, {path} is not in the inventory")
        if item["classification"] in {"test_only", "declaration_only"}:
            raise ValueError(f"{POLICY_FILE}: {path} is {item['classification']} and needs no entry")
        if item["sha256"] != entry["sha256"]:
            raise ValueError(f"{POLICY_FILE}: stale hash for {path}")
        item["classification"] = "other_configuration"
        item["configuration"] = entry["configuration"]


def source_inventory(
    root: pathlib.Path,
    policy: list[dict],
    classify: Callable[[list[pathlib.Path]], set[pathlib.Path]] | None = None,
) -> dict:
    """Build the inventory of tracked source files and list untracked ones.

    The optional classify callback receives the tracked paths and returns the
    test-only subset. A file that holds only declarations is declaration_only.
    A file with a valid policy entry is other_configuration. Every other file is required.
    """
    tracked = sorted(p for p in git_paths(root) if source_crate(p))
    untracked = sorted(
        p for p in git_paths(root, "--others", "--exclude-standard") if source_crate(p)
    )
    build_scripts = sorted(p for p in git_paths(root) if is_build_script(p))
    test_only = classify([pathlib.Path(p) for p in tracked]) if classify else set()
    files = []
    for path in tracked:
        try:
            digest = hashlib.sha256((root / path).read_bytes()).hexdigest()
        except OSError as error:
            raise ValueError(f"{path}: {error}") from error
        files.append({
            "path": path,
            "sha256": digest,
            "crate": source_crate(path),
            "classification": classification(root, path, test_only),
        })
    apply_policy(files, policy)
    return {
        "schema_version": SCHEMA_VERSION,
        "root": root.as_posix(),
        "files": files,
        "untracked": untracked,
        "build_scripts": build_scripts,
    }
