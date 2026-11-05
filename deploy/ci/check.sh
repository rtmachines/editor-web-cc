#!/usr/bin/env bash
# Check syntax and reject shell tracing before the stack starts.
set -euo pipefail
umask 077

for script in deploy/ci/*.sh
do
  bash -n "$script"
done
for test in deploy/ci/test_*.py
do
  python3 -I -B "$test"
done
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


def test_migration_freeze_job():
    # The job needs the full history to compare the migrations with the base.
    lines = Path('.github/workflows/ci.yml').read_text(encoding='utf-8').splitlines()
    header = '  migration-freeze:'
    if header not in lines:
        raise SystemExit('ci.yml has no job migration-freeze')
    start = lines.index(header) + 1
    end = start
    # A job body is indented by four spaces or more. Blank lines belong to it too.
    while end < len(lines) and (lines[end].startswith('    ') or not lines[end].strip()):
        end += 1
    body = '\n'.join(lines[start:end])
    if 'fetch-depth: 0' not in body:
        raise SystemExit('migration-freeze does not fetch the full history')
    if 'run: python3 -I -B deploy/ci/migration_freeze.py' not in body:
        raise SystemExit('migration-freeze does not run migration_freeze.py')
    if re.search(r'^    if:', body, re.M):
        raise SystemExit('migration-freeze has an if: condition')
    print('CI migration-freeze job check passed.')


test_migration_freeze_job()


def test_ci_scripts_job():
    lines = Path('.github/workflows/ci.yml').read_text(encoding='utf-8').splitlines()
    if '  pull_request:' not in lines:
        raise SystemExit('ci.yml has no pull_request trigger')
    header = '  ci-scripts:'
    if header not in lines:
        raise SystemExit('ci.yml has no job ci-scripts')
    start = lines.index(header) + 1
    end = start
    while end < len(lines) and (lines[end].startswith('    ') or not lines[end].strip()):
        end += 1
    body = '\n'.join(lines[start:end])
    if not re.search(r'^    runs-on: ubuntu-24\.04$', body, re.M):
        raise SystemExit('ci-scripts does not use ubuntu-24.04')
    if not re.search(r'^    timeout-minutes: [1-9][0-9]*$', body, re.M):
        raise SystemExit('ci-scripts has no positive job timeout')
    if re.search(r'^    (?:if|needs):', body, re.M):
        raise SystemExit('ci-scripts must run on each pull request')
    if not re.search(r'^        uses: actions/checkout@[0-9a-f]{40}(?: #.*)?$', body, re.M):
        raise SystemExit('ci-scripts does not check out the repository with a pinned action')
    if not re.search(r'^        run: bash deploy/ci/check\.sh$', body, re.M):
        raise SystemExit('ci-scripts does not run check.sh')
    if re.search(r'^        (?:if|continue-on-error):', body, re.M):
        raise SystemExit('ci-scripts must run its checks and report failures')
    if re.search(r'^    (?:container|services):|\bdocker\b', body, re.M | re.I):
        raise SystemExit('ci-scripts must run without Docker')
    print('CI ci-scripts job check passed.')


test_ci_scripts_job()
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
