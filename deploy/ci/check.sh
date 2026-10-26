#!/usr/bin/env bash
# Check syntax and reject shell tracing before the stack starts.
set -euo pipefail
umask 077

for script in deploy/ci/*.sh
do
  bash -n "$script"
done
python3 -I -B deploy/ci/test_http_probe.py
python3 -I -B deploy/ci/test_upgrade_checks.py
python3 -I -B - <<'PY'
from pathlib import Path
import re


def shell_tracing(data):
    # Match command boundaries and shebangs. Skip quoted text and comments.
    boundary = rb'(?:^|[;&|()]|\b(?:run|shell):)[ \t]*'
    shebang = rb'^\#![ \t]*(?:/[^\s]+/env[ \t]+(?:-S[ \t]+)?)?'
    command = rb'(?:set|(?:/[\w.-]+)*/(?:ba)?sh|(?:ba)?sh)'
    options = rb'(?:[ \t]+(?:--[a-z-]+|-[a-wyzA-Z]+))*'
    tracing = rb'[ \t]+(?:-[a-zA-Z]*x[a-zA-Z]*|-o[ \t]+xtrace)(?=$|[\s;&|()])'
    pattern = rb'(?P<tracing>(?:' + shebang + b'|' + boundary + b')'
    pattern += command + options + tracing + b')'
    pattern += rb'''|'[^']*'|"(?:\\.|[^"\\])*"|\#[^\n]*'''
    data = data.replace(b'\\\n', b' ')
    return any(match.group('tracing') for match in re.finditer(pattern, data, re.M))


def test_shell_tracing():
    unsafe = [
        b'set -x', b'  set -eux', b'set -o xtrace',
        b'bash -x script.sh', b'sh -x script.sh',
        b'#!/bin/bash -x', b'#!/bin/sh -x', b'#!/usr/bin/env bash -x',
        b'run: set -x', b'shell: bash -x {0}', b'  set -e -o xtrace',
    ]
    unsafe.extend(prefix + b'set -x' for prefix in (
        b'true; ', b'true && ', b'true || ', b'true | ',
    ))
    clean = [
        b'set -euo pipefail', b'set +x', b'set +o xtrace',
        b'#!/usr/bin/env bash', b'#!/bin/sh', b'bash script.sh',
        b'# set -x', b'  # bash -x script.sh', b'# set -o xtrace',
        b'true # comment; set -x', b'set -euo pipefail # set -x',
        b'echo "Do not use set -x"', b"echo 'comment; set -x'",
    ]
    for fixture in unsafe:
        if not shell_tracing(fixture):
            raise SystemExit(f'tracing fixture passed: {fixture!r}')
    for fixture in clean:
        if shell_tracing(fixture):
            raise SystemExit(f'clean fixture failed: {fixture!r}')
    print('CI tracing fixtures passed.')


test_shell_tracing()
paths = list(Path('.github/workflows').rglob('*'))
paths.extend(Path('deploy').rglob('*'))
for path in paths:
    if not path.is_file():
        continue
    data = path.read_bytes()
    if path.suffix not in ('.sh', '.bash', '.py', '.yml', '.yaml') and not data.startswith(b'#!'):
        continue
    if path.suffix == '.sh' and b'\r' in data:
        raise SystemExit(f'CRLF in {path}')
    # Comments can describe the ban. Only a shell command enables tracing.
    if shell_tracing(data):
        raise SystemExit(f'shell tracing in {path}')
print('CI syntax and tracing checks passed.')
PY
