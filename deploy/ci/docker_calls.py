"""Parse shell commands for the Docker guards."""

import shlex


def argument_records(data):
    """Return argument lists from count-prefixed NUL records."""
    if not data:
        return []
    if not data.endswith(b"\x00"):
        raise ValueError("Truncated argument record")
    fields = data.split(b"\x00")[:-1]
    records = []
    position = 0
    while position < len(fields):
        count = fields[position]
        if not count.isdigit():
            raise ValueError("Invalid argument count")
        position += 1
        end = position + int(count)
        if end > len(fields):
            raise ValueError("Truncated argument record")
        records.append([word.decode("utf-8", errors="surrogateescape")
                        for word in fields[position:end]])
        position = end
    return records


DOCKER_VALUE_OPTIONS = {
    "--context", "-c", "--host", "-H", "--config", "--log-level", "-l",
    "--tlscacert", "--tlscert", "--tlskey",
}
COMPOSE_VALUE_OPTIONS = {
    "--project-name", "-p", "-f", "--file", "--profile", "--ansi",
    "--env-file", "--project-directory", "--progress", "--parallel",
}
COMPOSE_SUBCOMMAND_VALUE_OPTIONS = {
    "up": {"--attach", "--exit-code-from", "--no-attach", "--pull", "--scale",
           "-t", "--timeout", "--wait-timeout"},
    "create": {"--pull", "--scale"},
    "run": {"--cap-add", "--cap-drop", "--entrypoint", "-e", "--env",
            "--env-from-file", "-l", "--label", "--name", "-p", "--publish",
            "--pull", "-u", "--user", "-v", "--volume", "-w", "--workdir"},
}


def _option_word_count(option, value_options):
    """Return the word count of an option and its value."""
    if option.startswith("--"):
        return 2 if option in value_options else 1
    for index, letter in enumerate(option[1:], start=1):
        if f"-{letter}" in value_options:
            return 2 if index == len(option) - 1 else 1
    return 1


def forbidden_compose_call(subcommand, arguments, *, services=("app", "backup"),
                           subcommands=("up", "create", "run", "start", "restart")):
    """Return whether a Compose call can start a deleted service."""
    if subcommand not in subcommands:
        return False
    value_options = COMPOSE_SUBCOMMAND_VALUE_OPTIONS.get(subcommand, ())
    operands = []
    no_deps = False
    position = 0
    while position < len(arguments):
        word = arguments[position]
        if word.startswith("-"):
            if word == "--no-deps":
                no_deps = True
            position += _option_word_count(word, value_options)
        else:
            operands.append(word)
            position += 1
    if subcommand in ("up", "create", "run"):
        if not no_deps or not operands:
            return True
    return bool(set(services).intersection(operands))


def record_window(records, *, start=None, end=None):
    """Return a slice that excludes the given unique Compose boundary records."""
    calls = [docker_call(record) for record in records]
    bounds = []
    for boundary, offset in ((start, 1), (end, 0)):
        if boundary is None:
            bounds.append(None)
            continue
        subcommand, arguments = boundary
        matches = [index for index, call in enumerate(calls)
                   if call == (True, subcommand, arguments)]
        if len(matches) != 1:
            command = " ".join([subcommand, *arguments])
            raise AssertionError(f"{command}: expected one record, found {len(matches)}")
        bounds.append(matches[0] + offset)
    return slice(*bounds)


def forbidden_window_calls(records, *, window=slice(None), services=("app", "backup")):
    """Return refused records from a window of a nonempty Docker run."""
    if not records:
        raise AssertionError("The run has no Docker argument records")
    hits = []
    for record in records[window]:
        call = docker_call(record)
        if call is None:
            continue
        is_compose, subcommand, arguments = call
        if is_compose and forbidden_compose_call(subcommand, arguments, services=services):
            hits.append(record)
    return hits


def forbidden_check_window_calls(records):
    """Return refused records before the two allowed starts of check."""
    if not records:
        raise AssertionError("The run has no Docker argument records")
    calls = [docker_call(record) for record in records]
    truncate = [index for index, call in enumerate(calls)
                if call is not None and call[:2] == (True, "exec")
                and call[2][-2:] == ["-c", "TRUNCATE public.session, public.login_attempt"]]
    if not len(truncate) == 1:
        raise AssertionError("check needs one TRUNCATE record")
    app = truncate[0] + 1
    options = ["-d", "--no-deps", "--wait", "--wait-timeout", "180"]
    if not (app < len(calls) and calls[app] == (True, "up", options + ["app"])):
        raise AssertionError("The record after TRUNCATE must start app with the required options")
    backup = [index for index in range(app + 1, len(calls))
              if calls[index] == (True, "up", options + ["proxy", "backup"])]
    if not backup:
        raise AssertionError("check needs the end record that starts proxy and backup")
    return (forbidden_window_calls(records, window=slice(0, app))
            + forbidden_window_calls(records, window=slice(app + 1, backup[0]),
                                     services=("backup",)))


def _skip_global_options(arguments, position, value_options):
    """Return the position after global options and their values."""
    while position < len(arguments) and arguments[position].startswith("-"):
        option = arguments[position]
        position += _option_word_count(option, value_options)
    return position


def docker_call(arguments, *, compose=False):
    """Return (is_compose, subcommand, operands), or None without a subcommand.

    Read arguments after docker, or after docker-compose when compose is True.
    """
    position = 0
    if not compose:
        position = _skip_global_options(arguments, position, DOCKER_VALUE_OPTIONS)
        if position < len(arguments) and arguments[position] == "compose":
            compose = True
            position += 1
    if compose:
        position = _skip_global_options(arguments, position, COMPOSE_VALUE_OPTIONS)
    if position >= len(arguments):
        return None
    return compose, arguments[position], arguments[position + 1:]


def _command_substitutions(text):
    """Yield the inner text of paired command substitutions."""
    position = 0
    while position < len(text):
        if text.startswith("$(", position):
            start = position + 2
            end = start
            depth = 1
            while end < len(text) and depth:
                if text[end] == "(":
                    depth += 1
                elif text[end] == ")":
                    depth -= 1
                end += 1
            if not depth:
                yield text[start:end - 1]
                position = end
                continue
        elif text[position] == "`":
            end = text.find("`", position + 1)
            if end != -1:
                yield text[position + 1:end]
                position = end + 1
                continue
        position += 1


def shell_commands(command, *, _depth=0):
    """Return simple commands without redirections and their targets."""
    lexer = shlex.shlex(command, posix=True, punctuation_chars=True)
    lexer.whitespace_split = True
    lexer.commenters = ""
    try:
        tokens = list(lexer)
    except ValueError:
        tokens = command.split()
    commands = []
    words = []
    skip_target = False
    for token in tokens:
        if skip_target:
            skip_target = False
            continue
        if set(token) <= set(";&|()") and token:
            if words:
                commands.append(words)
                words = []
        elif set(token) <= set("<>&") and ("<" in token or ">" in token):
            if words and words[-1].isdigit():
                words.pop()
            skip_target = True
        else:
            words.append(token)
    if words:
        commands.append(words)
    if _depth < 8:
        # A backtick pair can span several lexer words.
        for inner in _command_substitutions(" ".join(tokens)):
            commands.extend(shell_commands(inner, _depth=_depth + 1))
    return commands
