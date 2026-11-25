"""Check the musl allocator and its default options against ADR-0224."""

import json
import os
import re
import subprocess
import sys
from pathlib import Path

MUSL = "x86_64-unknown-linux-musl"
GNU = "x86_64-unknown-linux-gnu"
MAIN_RS = Path("crates/recipe-server/src/main.rs")
UNWIND_RS = Path("crates/recipe-server/tests/mimalloc_unwind.rs")
MISSING_PACKAGE = "error: package ID specification `mimalloc` did not match any packages"
EMPTY_TREE = "warning: nothing to print."
TREE_ROOT = re.compile(r"^mimalloc v\S+")
SERVER_EDGE = re.compile(r"^(?:├──|└──) recipe-server v\S+", re.MULTILINE)

RUST_COMMENT_OR_LITERAL = re.compile(
    r'//[^\n]*|/\*|[bc]?r(?P<hashes>\#*)".*?"(?P=hashes)'
    r'|[bc]?"(?:\\.|[^"\\])*"|b?\'(?:\\.|[^\'\\\n])\'',
    re.DOTALL,
)
ATTRIBUTE_OR_DELIMITER = re.compile(r"(?P<attribute>#\s*(?P<inner>!\s*)?\[)|[()\[\]{}]")
CLOSING_DELIMITERS = {"(": ")", "[": "]", "{": "}"}
GLOBAL_ALLOCATOR = re.compile(r"#\s*\[\s*(?:r#)?global_allocator\s*\]")
CFG_ATTR = re.compile(r"\s*(?:r#)?cfg_attr\s*\(")
GLOBAL_ALLOCATOR_NAME = re.compile(r"\bglobal_allocator\b")
CONDITIONAL_ATTRIBUTE = re.compile(r"#\s*!?\s*\[\s*(?:r#)?cfg(?:_attr)?\b")
LINT_ATTRIBUTE = re.compile(r"#\s*\[\s*(?:r#)?(allow|expect)\s*\(")
ALLOCATOR_SHADOWING = re.compile(r"\bmod\s+mimalloc\b|\bas\s+(?:mimalloc|MiMalloc)\b")
USE_ITEM = re.compile(r"(?<![\w#])use\b([^;()\[\]\"<>]+;)")
USE_BRANCH = re.compile(r"([^,{};]*)([{},;])")
MUSL_ATTRIBUTE = re.compile(r'#\s*!?\s*\[\s*cfg\s*\(\s*target_env\s*=\s*"musl"\s*\)\s*\]')
STATIC = re.compile(
    r"\s*(?:pub(?:\s*\([^)]*\))?\s+)?static\s+\w+\s*:\s*([^=;]+?)\s*=\s*([^;]+?)\s*;"
)
ALLOCATOR = "mimalloc::MiMalloc"
UNWIND_TEST_ARGUMENT = re.compile(
    rb"""(?<!\S)--test(?:[ \t]+|=)(?:mimalloc_unwind|"mimalloc_unwind"|'mimalloc_unwind')"""
    rb"(?=$|[\s;&|()<>])"
)
ENV_VARIABLE = re.compile(rb"(?:\b|(?<=-e))(?i:mimalloc_)\w*")
SCAN_EXCLUSIONS = {Path("ci/allocator_rule.py"), Path("ci/test_allocator_rule.py")}


def check_tree(root, target):
    """Return a diagnostic when Cargo reports the wrong dependency for the target."""
    result = subprocess.run(
        ["cargo", "tree", "--locked", "-p", "recipe-server", "-e", "normal",
         "-i", "mimalloc", "--charset", "utf8", "--target", target],
        cwd=root, capture_output=True, text=True, encoding="utf-8", check=False,
        env={**os.environ, "CARGO_TERM_COLOR": "never"},
    )
    stdout = result.stdout.strip()
    stderr = result.stderr.splitlines()
    errors = [line for line in stderr if line.startswith("error:")]
    if not stdout and (
        (result.returncode == 0 and EMPTY_TREE in stderr and not errors)
        or (result.returncode == 101 and errors == [MISSING_PACKAGE])
    ):
        if target == MUSL:
            return f"{target}: The musl build is missing the mimalloc allocator."
        return None
    if result.returncode != 0 or errors:
        detail = result.stderr.strip() or f"Cargo exited with status {result.returncode}."
        return f"{target}: Cannot read the mimalloc dependency tree. {detail}"
    if TREE_ROOT.match(stdout):
        if target == GNU:
            return f"{target}: The gnu build depends on mimalloc."
        if SERVER_EDGE.search(stdout):
            return None
    if target == MUSL:
        return f"{target}: The musl build requires mimalloc with recipe-server as its dependent."
    return f"{target}: Cargo returned an unexpected mimalloc tree result."


def check_features(root, run, *flags):
    """Require the allocator features measured in ADR-0224."""
    label = f"{MUSL} ({run})"
    try:
        result = subprocess.run(
            ["cargo", "metadata", "--locked", "--format-version", "1",
             "--filter-platform", MUSL, *flags],
            cwd=root, capture_output=True, text=True, encoding="utf-8", check=False,
            env={**os.environ, "CARGO_TERM_COLOR": "never"},
        )
        if result.returncode != 0:
            detail = result.stderr.strip() or f"Cargo exited with status {result.returncode}."
            raise ValueError(detail)
        metadata = json.loads(result.stdout)
    except (OSError, ValueError) as error:
        message = f"{label}: Cannot read Cargo metadata for the allocator. {error}"
        raise ValueError(message) from error

    resolve = metadata.get("resolve")
    if not resolve:
        return f"{label}: Cargo metadata has no resolve graph for the allocator."
    nodes = {node["id"]: node["features"] for node in resolve.get("nodes", [])}
    # Cargo lists the empty default feature of mimalloc as an active feature.
    for name, expected in (("mimalloc", ["default"]), ("libmimalloc-sys", [])):
        packages = [package for package in metadata["packages"]
                    if package["name"] == name and package["id"] in nodes]
        if not packages:
            return f"{label}: {name}: Require a resolve node. Found no resolved features."
        for package in packages:
            features = nodes[package["id"]]
            if features != expected:
                return (
                    f"{label}: {name}: Expected resolved features {expected!r}. Found {features!r}."
                )
            if name == "mimalloc":
                default = package["features"].get("default")
                if default != []:
                    return f"{label}: {name}: Declare no features in default. Found {default!r}."
    return None


def block_comment_end(source, start):
    """Find the end of a Rust block comment, including nested comments."""
    depth = 1
    for delimiter in re.compile(r"/\*|\*/").finditer(source, start):
        depth += 1 if delimiter[0] == "/*" else -1
        if depth == 0:
            return delimiter.end()
    return len(source)


def rust_code(source):
    """Replace comments and literals that cannot form the required declaration."""
    pieces = []
    cursor = 0
    while match := RUST_COMMENT_OR_LITERAL.search(source, cursor):
        pieces.append(source[cursor:match.start()])
        cursor = match.end()
        if match[0] == "/*":
            cursor = block_comment_end(source, cursor)
        if match[0] == '"musl"':
            pieces.append(match[0])
        elif match[0].startswith(("//", "/*")):
            pieces.append(" ")
        else:
            pieces.append('""')
    pieces.append(source[cursor:])
    return "".join(pieces)


def rust_attributes(source):
    """Yield each attribute with its closing offset and delimiter depth."""
    delimiters = []
    attribute = None
    attribute_depth = 0
    for token in ATTRIBUTE_OR_DELIMITER.finditer(source):
        value = token[0]
        if token["attribute"]:
            if attribute is None:
                attribute = token
                attribute_depth = len(delimiters)
            delimiters.append("]")
        elif value in CLOSING_DELIMITERS:
            delimiters.append(CLOSING_DELIMITERS[value])
        elif delimiters and value == delimiters[-1]:
            delimiters.pop()
            if attribute is not None and len(delimiters) == attribute_depth:
                yield attribute, token.end(), attribute_depth
                attribute = None


def outer_attribute_stacks(source, attributes):
    """Group outer attributes when only whitespace separates them."""
    stack = []
    for attribute, end, depth in attributes:
        if attribute["inner"]:
            continue
        start = attribute.start()
        if stack and source[stack[-1][1]:start].strip():
            yield stack
            stack = []
        stack.append((start, end, depth))
    if stack:
        yield stack


def check_main_cfg(source, attributes, stack, allocator_start):
    """Require the musl attribute above the allocator in main.rs."""
    if any(depth == 0 and attribute["inner"]
           and CONDITIONAL_ATTRIBUTE.match(source, attribute.start(), end)
           for attribute, end, depth in attributes):
        return f"{MAIN_RS}: Keep #[global_allocator] free of crate-level cfg attributes."
    conditions = [(start, end) for start, end, _ in stack
                  if CONDITIONAL_ATTRIBUTE.match(source, start, end)]
    if len(conditions) != 1 or not any(
        start < allocator_start and MUSL_ATTRIBUTE.fullmatch(source, start, end)
        for start, end in conditions
    ):
        return f'{MAIN_RS}: Put only #[cfg(target_env = "musl")] above #[global_allocator].'
    return None


def check_unwind_cfg(source, attributes):
    """Require the musl crate attribute as the only conditional attribute."""
    conditions = [(attribute, end, depth) for attribute, end, depth in attributes
                  if CONDITIONAL_ATTRIBUTE.match(source, attribute.start(), end)]
    if len(conditions) == 1:
        attribute, end, depth = conditions[0]
        if depth == 0 and attribute["inner"] and MUSL_ATTRIBUTE.fullmatch(
            source, attribute.start(), end
        ):
            return None
    return f'{UNWIND_RS}: Use #![cfg(target_env = "musl")] as the only cfg attribute.'


def allocator_import_rule(tree: str) -> str | None:
    """Return the rule that an import tree breaks."""
    parents = [""]
    for branch, delimiter in USE_BRANCH.findall(tree):
        path, *aliases = re.split(r"\s+as\s+", branch.strip(), maxsplit=1)
        name = path.removesuffix("::").rsplit("::", 1)[-1].strip()
        if delimiter == "{":
            parents.append(name or parents[-1])
            continue
        if name == "*":
            return "Replace glob imports with explicit imports."
        if aliases:
            name = aliases[0]
        elif name == "self":
            name = parents[-1]
        if name in ("mimalloc", "r#mimalloc"):
            return "Remove imports that bind mimalloc."
        if delimiter == "}" and len(parents) > 1:
            parents.pop()
    return None


def check_static(root, path):
    """Return a diagnostic when a Rust file fails the allocator rule."""
    source = rust_code((root / path).read_text(encoding="utf-8"))
    attributes = list(GLOBAL_ALLOCATOR.finditer(source))
    parsed_attributes = list(rust_attributes(source))
    if len(attributes) != 1 or any(
        CFG_ATTR.match(source, attribute.end(), end)
        and GLOBAL_ALLOCATOR_NAME.search(source, attribute.end(), end)
        for attribute, end, _ in parsed_attributes
    ):
        return f"{path}: Declare exactly one #[global_allocator] attribute."

    allocator_start = attributes[0].start()
    stack = next(
        (group for group in outer_attribute_stacks(source, parsed_attributes)
         if any(start == allocator_start for start, _, _ in group)),
        None,
    )
    if stack is None:
        return f"{path}: Attach #[global_allocator] to a static."
    if stack[0][2] != 0:
        return f"{path}: Put #[global_allocator] on a static at crate scope."
    if path == UNWIND_RS:
        diagnostic = check_unwind_cfg(source, parsed_attributes)
    else:
        diagnostic = check_main_cfg(source, parsed_attributes, stack, allocator_start)
    if diagnostic:
        return diagnostic

    declaration = STATIC.match(source, stack[-1][1])
    if declaration is None:
        return f"{path}: Attach #[global_allocator] to a static."
    allocator_type, allocator_value = (
        re.sub(r"\s*::\s*", "::", part) for part in declaration.groups()
    )
    if allocator_type != ALLOCATOR:
        return f"{path}: Use allocator type {ALLOCATOR}. Found {allocator_type}."
    if allocator_value != ALLOCATOR:
        return f"{path}: Use allocator value {ALLOCATOR}. Found {allocator_value}."
    if shadowing := ALLOCATOR_SHADOWING.search(source):
        return f"{path}: Remove allocator shadowing. Found {shadowing[0]!r}."
    for start, end, _ in stack:
        if lint := LINT_ATTRIBUTE.match(source, start, end):
            return f"{path}: Remove #[{lint[1]}] from the allocator static."
    for item in USE_ITEM.finditer(source):
        if rule := allocator_import_rule(item[1]):
            return f"{path}: {rule}"
    return None


def deploy_variables(root):
    """Report each allocator variable on a line outside a comment."""
    deploy = root / "deploy"
    for path in sorted(deploy.rglob("*")):
        if not path.is_file() or path.relative_to(deploy) in SCAN_EXCLUSIONS:
            continue
        comment = b"--" if path.suffix == ".sql" else b"#"
        with path.open("rb") as file:
            for number, line in enumerate(file, 1):
                if line.lstrip().startswith(comment):
                    continue
                line = UNWIND_TEST_ARGUMENT.sub(b" ", line)
                for variable in sorted(set(ENV_VARIABLE.findall(line))):
                    name = variable.decode("ascii")
                    yield f"{path.relative_to(root)}:{number}: Remove {name}."


def main():
    """Exit 0 on success, 1 on a rule violation, or 2 on an input error."""
    root = Path(__file__).resolve().parents[2]
    try:
        diagnostics = [check_tree(root, target) for target in (MUSL, GNU)]
        diagnostics.append(check_features(root, "default"))
        diagnostics.append(check_features(root, "all features", "--all-features"))
        diagnostics.extend(check_static(root, path) for path in (MAIN_RS, UNWIND_RS))
        diagnostics.extend(deploy_variables(root))
    except (OSError, ValueError) as error:
        print(f"Cannot check the allocator rule. {error}", file=sys.stderr)
        return 2
    violations = [diagnostic for diagnostic in diagnostics if diagnostic]
    if violations:
        for diagnostic in violations:
            print(diagnostic)
        return 1
    print("Allocator rule passed.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
