"""Check migrations with: python -I -B deploy/ci/migration_rewrite.py migrations.

Reject column type changes, volatile defaults on added columns, and DO blocks.
Reject SET NOT NULL without a current valid CHECK on the same table and column.
Only an earlier unconditional CREATE TABLE in the same file exempts table changes.
Check forms inside function and procedure bodies, even without a call.
EXEMPT maps (filename, form) pairs to their authorizing ADRs.
Report dead exemptions. Exit 0 on success, 1 on violations, and 2 on input errors.
"""

import re
import sys
from collections.abc import Iterator
from pathlib import Path


EXEMPT: dict[tuple[str, str], str] = {}
NON_VOLATILE = {"now", "pg_catalog.now"}
WORD = re.compile(r"[A-Za-z_][A-Za-z_0-9$]*")
DOLLAR = re.compile(r"\$(?:[A-Za-z_][A-Za-z_0-9]*)?\$")
Checks = dict[str, tuple[str, bool]]
Table = tuple[str, ...]
Tables = dict[Table, Checks]


def quoted_end(sql: str, start: int) -> int:
    """Find the end of a quoted token, including escaped quote characters."""
    quote = sql[start]
    escape = quote == "'" and start > 0 and sql[start - 1] in "eE"
    index = start + 1
    while index < len(sql):
        if escape and sql[index] == "\\":
            index += 2
        elif sql[index] == quote:
            if sql[index:index + 2] == quote * 2:
                index += 2
            else:
                return index + 1
        else:
            index += 1
    raise ValueError("unterminated quote")


def comment_end(sql: str, start: int) -> int:
    """Find the end of a possibly nested block comment."""
    index, depth = start + 2, 1
    while index < len(sql):
        pair = sql[index:index + 2]
        if pair in {"/*", "*/"}:
            depth += 1 if pair == "/*" else -1
            index += 2
            if depth == 0:
                return index
        else:
            index += 1
    raise ValueError("unterminated block comment")


def token_end(sql: str, start: int) -> tuple[int, bool]:
    """Return a token boundary and whether the token is a comment."""
    if sql.startswith("--", start):
        end = sql.find("\n", start)
        return (len(sql) if end < 0 else end), True
    if sql.startswith("/*", start):
        return comment_end(sql, start), True
    if sql[start] in "'\"":
        return quoted_end(sql, start), False
    dollar = DOLLAR.match(sql, start)
    if dollar:
        end = sql.find(dollar[0], dollar.end())
        if end < 0:
            raise ValueError("unterminated dollar quote")
        return end + len(dollar[0]), False
    word = WORD.match(sql, start)
    return (word.end() if word else start + 1), False


def statements(sql: str) -> Iterator[tuple[int, list[str]]]:
    """Remove comments and split statements outside quoted tokens."""
    index, line, start_line = 0, 1, 1
    tokens: list[str] = []
    brackets: list[str] = []
    while index < len(sql):
        if sql[index].isspace():
            line += sql[index] == "\n"
            index += 1
            continue
        end, comment = token_end(sql, index)
        token = sql[index:end]
        if not comment:
            if not tokens:
                start_line = line
            if token in {"(", "["}:
                brackets.append(")" if token == "(" else "]")
            elif token in {")", "]"}:
                if not brackets or brackets.pop() != token:
                    raise ValueError("unbalanced brackets")
            if token == ";":
                if brackets:
                    raise ValueError("unbalanced brackets")
                if tokens:
                    yield start_line, tokens
                tokens = []
            else:
                tokens.append(token.lower() if WORD.fullmatch(token) else token)
        line += token.count("\n")
        index = end
    if brackets:
        raise ValueError("unbalanced brackets")
    if tokens:
        yield start_line, tokens


def split_actions(tokens: list[str]) -> list[list[str]]:
    """Split commas outside parentheses and array brackets."""
    actions: list[list[str]] = []
    start, depth = 0, 0
    for index, token in enumerate(tokens):
        if token in {"(", "["}:
            depth += 1
        elif token in {")", "]"}:
            depth -= 1
        elif token == "," and depth == 0:
            actions.append(tokens[start:index])
            start = index + 1
    actions.append(tokens[start:])
    return actions


def identifier(token: str) -> str:
    return token[1:-1].replace('""', '"') if token.startswith('"') else token


def table_name(
    tokens: list[str], tables: Tables | None = None, *, temporary: bool = False,
) -> tuple[Table, list[str]]:
    """Keep the schema and resolve unqualified names against temporary tables first."""
    if tokens[:2] == ["if", "exists"]:
        tokens = tokens[2:]
    if tokens[:1] == ["only"]:
        tokens = tokens[1:]
    if not tokens:
        raise ValueError("missing table name")
    name, tokens = (identifier(tokens[0]),), tokens[1:]
    while tokens[:1] == ["."]:
        if len(tokens) < 2:
            raise ValueError("missing qualified name")
        name, tokens = (*name, identifier(tokens[1])), tokens[2:]
    if tokens[:1] == ["*"]:
        tokens = tokens[1:]
    if len(name) == 1:
        temp_name = ("pg_temp", *name)
        if temporary or (tables is not None and temp_name in tables):
            name = temp_name
        else:
            name = ("public", *name)
    return name, tokens


def add_checks(tokens: list[str], checks: Checks) -> None:
    """Record named, exact IS NOT NULL checks without guessing constraint names."""
    for index, token in enumerate(tokens):
        if token != "check" or tokens[index + 1:index + 2] != ["("]:
            continue
        end, depth = index + 2, 1
        while end < len(tokens) and depth:
            depth += (tokens[end] == "(") - (tokens[end] == ")")
            end += 1
        expression = tokens[index + 2:end - 1]
        while expression[:1] == ["("] and expression[-1:] == [")"]:
            expression = expression[1:-1]
        if len(expression) != 4 or expression[1:] != ["is", "not", "null"]:
            continue
        column = identifier(expression[0])
        if index < 2 or tokens[index - 2] != "constraint":
            # PostgreSQL chooses unnamed constraint names, including suffixes and truncation.
            continue
        name = identifier(tokens[index - 1])
        valid = not any(tokens[i:i + 2] == ["not", "valid"] for i in range(end, len(tokens)))
        checks[name] = column, valid


def remove_state(action: list[str], checks: Checks) -> None:
    kind, tail = action[1], action[2:]
    if kind not in {"constraint", "column"}:
        kind, tail = "column", action[1:]
    if tail[:2] == ["if", "exists"]:
        tail = tail[2:]
    if not tail:
        raise ValueError("missing DROP name")
    name = identifier(tail[0])
    if kind == "constraint":
        checks.pop(name, None)
    else:
        for constraint, (column, _) in list(checks.items()):
            if column == name:
                del checks[constraint]


def rename_state(action: list[str], checks: Checks) -> None:
    tail = action[1:]
    kind = tail[0]
    if kind in {"column", "constraint"}:
        tail = tail[1:]
    if len(tail) != 3 or tail[1] != "to":
        return
    old, new = identifier(tail[0]), identifier(tail[2])
    if kind == "constraint":
        if old in checks:
            checks[new] = checks.pop(old)
    else:
        for name, (column, valid) in list(checks.items()):
            if column == old:
                checks[name] = new, valid


def update_checks(action: list[str], checks: Checks) -> None:
    if action[:1] == ["add"]:
        add_checks(action, checks)
    elif action[:2] == ["validate", "constraint"] and len(action) > 2:
        name = identifier(action[2])
        if name in checks:
            checks[name] = checks[name][0], True
    elif action[:1] == ["drop"] and len(action) > 1:
        remove_state(action, checks)
    elif action[:1] == ["rename"] and len(action) > 1:
        rename_state(action, checks)


def default_expression(tokens: list[str]) -> list[str]:
    """Exclude column constraints that follow the default expression."""
    depth, cases = 0, 0
    for index, token in enumerate(tokens):
        cases += (token == "case") - (token == "end")
        previous = tokens[max(0, index - 2):index]
        null_test = previous[-1:] == ["is"] or previous == ["is", "not"]
        if depth == 0 and cases == 0 and index > 0:
            if token in {
                "constraint", "check", "unique", "primary", "references", "collate",
                "generated",
            }:
                return tokens[:index]
            if not null_test and (
                token == "null" or tokens[index:index + 2] == ["not", "null"]
            ):
                return tokens[:index]
        depth += (token in {"(", "["}) - (token in {")", "]"})
    return tokens


def volatile_default(action: list[str]) -> bool:
    if not action or action[0] != "add":
        return False
    if action[1:2] in (["constraint"], ["check"], ["foreign"], ["unique"], ["primary"]):
        return False
    column = action[1:]
    if column[:1] == ["column"]:
        column = column[1:]
    if column[:3] == ["if", "not", "exists"]:
        column = column[3:]
    if len(column) > 1 and column[1] in {
        "serial", "smallserial", "bigserial", "serial2", "serial4", "serial8",
    }:
        return True
    depth = 0
    for index, token in enumerate(column):
        if depth == 0 and token == "generated":
            generation = column[index + 1:]
            if generation[:3] == ["always", "as", "identity"] or generation[:4] == [
                "by", "default", "as", "identity",
            ]:
                return True
        depth += (token in {"(", "["}) - (token in {")", "]"})
    if "default" not in action:
        return False
    tokens = default_expression(action[action.index("default") + 1:])
    for index, token in enumerate(tokens[:-1]):
        if tokens[index + 1] != "(" or not (WORD.fullmatch(token) or token.startswith('"')):
            continue
        name, start = identifier(token), index
        if token == "varying" and index > 0 and tokens[index - 1] in {"bit", "char", "character"}:
            start -= 1
        if index >= 2 and tokens[index - 1] == ".":
            name, start = f"{identifier(tokens[index - 2])}.{name}", index - 2
        if token == "cast" or (start >= 1 and tokens[start - 1] == "as"):
            continue
        if start >= 2 and tokens[start - 2:start] == [":", ":"]:
            continue
        if name not in NON_VOLATILE:
            return True
    return False


def forbidden_forms(action: list[str], checks: Checks) -> list[str]:
    if volatile_default(action):
        return ["volatile default"]
    if action[:1] != ["alter"]:
        return []
    tail = action[1:]
    if tail[:1] == ["column"]:
        tail = tail[1:]
    if not tail:
        return []
    column, operation = identifier(tail[0]), tail[1:]
    if operation[:1] == ["type"] or operation[:3] == ["set", "data", "type"]:
        return ["column type change"]
    if operation[:3] == ["set", "not", "null"]:
        if not any(check_column == column and valid for check_column, valid in checks.values()):
            return ["set not null without valid check"]
    return []


def alter_table(tokens: list[str], tables: Tables, created: set[Table]) -> list[str]:
    table, tail = table_name(tokens, tables)
    actions = split_actions(tail)
    checks = tables.setdefault(table, {})
    for action in actions:
        update_checks(action, checks)
    forms = [] if table in created else [
        form for action in actions for form in forbidden_forms(action, checks)
    ]
    for action in actions:
        if action[:2] == ["rename", "to"] and len(action) == 3:
            new = (*table[:-1], identifier(action[2]))
            tables[new] = tables.pop(table)
            if table in created:
                created.remove(table)
                created.add(new)
            table = new
    return forms


def create_table(
    tokens: list[str], tables: Tables, created: set[Table], *, temporary: bool = False,
) -> None:
    conditional = tokens[:3] == ["if", "not", "exists"]
    table, tail = table_name(tokens[3:] if conditional else tokens, temporary=temporary)
    if conditional and table in tables:
        return
    checks: Checks = {}
    tables[table] = checks
    if not conditional:
        created.add(table)
    if tail[:1] == ["("]:
        for definition in split_actions(tail[1:-1]):
            add_checks(definition, checks)


def quoted_body(token: str) -> str | None:
    """Read SQL from a dollar quote or a single quote."""
    dollar = DOLLAR.match(token)
    if dollar:
        return token[len(dollar[0]):-len(dollar[0])]
    if token.startswith("'"):
        return token[1:-1].replace("''", "'")
    return None


def body_forms(sql: str, tables: Tables, created: set[Table]) -> list[str]:
    """Check body forms without treating conditional DDL as migration state."""
    forms: list[str] = []
    for _, tokens in statements(sql):
        for index, token in enumerate(tokens):
            if tokens[index:index + 2] == ["alter", "table"]:
                local_tables = {table: checks.copy() for table, checks in tables.items()}
                forms.extend(alter_table(tokens[index + 2:], local_tables, created.copy()))
                break
            if token == "do" and (
                tokens[index + 1:index + 2] not in (["nothing"], ["update"])
            ):
                forms.append("DO block")
                break
            if token == "execute" and index + 1 < len(tokens):
                body = quoted_body(tokens[index + 1])
                if body is not None:
                    forms.extend(body_forms(body, tables, created))
    return forms


def check_statement(tokens: list[str], tables: Tables, created: set[Table]) -> list[str]:
    if tokens[:1] == ["do"]:
        return ["DO block"]
    if tokens[:2] == ["alter", "table"]:
        return alter_table(tokens[2:], tables, created)
    create = tokens[1:]
    if create[:2] == ["or", "replace"]:
        create = create[2:]
    if tokens[:1] == ["create"] and create[:1] in (["function"], ["procedure"]):
        forms = []
        for index, token in enumerate(create[:-1]):
            if token == "as":
                body = quoted_body(create[index + 1])
                if body is not None:
                    forms.extend(body_forms(body, tables, created))
        return forms
    temporary = create[:1] in (["temporary"], ["temp"])
    if create[:1] in (["temporary"], ["temp"], ["unlogged"]):
        create = create[1:]
    if tokens[:1] == ["create"] and create[:1] == ["table"]:
        create_table(create[1:], tables, created, temporary=temporary)
    elif tokens[:2] == ["drop", "table"]:
        for target in split_actions(tokens[2:]):
            table, _ = table_name(target, tables)
            tables.pop(table, None)
            created.discard(table)
    return []


def check_files(directory: Path) -> int:
    tables: Tables = {}
    matched: set[tuple[str, str]] = set()
    failures = 0
    paths = sorted(path for path in directory.iterdir() if path.suffix == ".sql")
    for path in paths:
        created: set[Table] = set()
        try:
            sql = path.read_text(encoding="utf-8")
            for line, tokens in statements(sql):
                for form in check_statement(tokens, tables, created):
                    key = path.name, form
                    matched.add(key)
                    if key not in EXEMPT:
                        print(f"{path}:{line}: {form}")
                        failures += 1
        except (OSError, UnicodeError, ValueError) as error:
            raise ValueError(f"{path}: {error}") from error
    for name, form in sorted(EXEMPT.keys() - matched):
        print(f"{directory / name}:0: dead exemption")
        failures += 1
    return int(failures > 0)


def main() -> int:
    if len(sys.argv) != 2:
        print(__doc__, file=sys.stderr)
        return 2
    try:
        return check_files(Path(sys.argv[1]))
    except (OSError, ValueError) as error:
        print(f"migration rewrite check: {error}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    sys.exit(main())
