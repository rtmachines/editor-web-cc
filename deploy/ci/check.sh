#!/usr/bin/env bash
# Check syntax and reject shell tracing before the stack starts.
set -euo pipefail
umask 077

for script in deploy/ci/*.sh
do
  bash -n "$script"
done
python3 -I -B -W error - <<'PY'
from pathlib import Path

for path in sorted(Path('deploy/ci').glob('*.py')):
    compile(path.read_bytes(), str(path), 'exec')
PY
for test in deploy/ci/test_*.py
do
  python3 -I -B "$test"
done
python3 -I -B - <<'PY'
from pathlib import Path
import re


def cc_debian13_matches(source, candidate):
    # ADR-0128 (h) permits only these substitutions.
    substitutions = (
        ('    RECIPE_TARGET=x86_64-unknown-linux-musl \\\n',
         '    RECIPE_TARGET=x86_64-unknown-linux-gnu \\\n'),
        ('    CC_x86_64_unknown_linux_musl=musl-gcc\n',
         '    CC_x86_64_unknown_linux_gnu=gcc\n'),
        ('    && apt-get install --no-install-recommends --yes musl-tools \\\n',
         '    && apt-get install --no-install-recommends --yes gcc \\\n'),
        ('FROM gcr.io/distroless/static-debian13:nonroot@sha256:'
         'd62067879de890a1def9da0e7766b7d20810a5e5555cef8506b752437d7a3e88 AS runtime\n',
         'FROM gcr.io/distroless/cc-debian13:nonroot@sha256:'
         'b7e81cb1af38d9345680e68bcf564d388f96304e25222030fc8c051faf8565b3 AS runtime\n'),
        ('    /build/target/x86_64-unknown-linux-musl/release/recipe-server /usr/local/bin/recipe-server\n',
         '    /build/target/x86_64-unknown-linux-gnu/release/recipe-server /usr/local/bin/recipe-server\n'),
    )
    for old, new in substitutions:
        if source.count(old) != 1:
            raise SystemExit('deploy/Dockerfile does not match the fixed cc-debian13 substitutions')
        source = source.replace(old, new, 1)
    return source == candidate


def test_cc_debian13_dockerfile():
    source = Path('deploy/Dockerfile').read_bytes().decode('utf-8')
    path = Path('deploy/ci/Dockerfile.cc-debian13')
    if not path.is_file():
        raise SystemExit('deploy/ci/Dockerfile.cc-debian13 is missing')
    candidate = path.read_bytes().decode('utf-8')
    if not cc_debian13_matches(source, candidate):
        raise SystemExit('Dockerfile.cc-debian13 differs outside the fixed substitutions')
    original = 'RUN cargo chef prepare --recipe-path recipe.json\n'
    changed = 'RUN cargo chef prepare --recipe-path other.json\n'
    if candidate.count(original) != 1:
        raise SystemExit('Dockerfile.cc-debian13 has no unique RUN line for the fixture')
    fixture = candidate.replace(original, changed, 1)
    if cc_debian13_matches(source, fixture):
        raise SystemExit('Dockerfile.cc-debian13 check accepted a changed RUN line')
    print('CI cc-debian13 Dockerfile check and changed RUN fixture passed.')


test_cc_debian13_dockerfile()


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


def load_job_body():
    lines = Path('.github/workflows/ci.yml').read_text(encoding='utf-8').splitlines()
    header = '  load:'
    if header not in lines:
        raise SystemExit('ci.yml has no job load')
    start = lines.index(header) + 1
    end = start
    while end < len(lines) and (lines[end].startswith('    ') or not lines[end].strip()):
        end += 1
    return '\n'.join(lines[start:end])


def require_load(text, pattern, message):
    if not re.search(pattern, text, re.M):
        raise SystemExit('load: ' + message)


def load_job_steps(body):
    steps = {}
    for block in re.split(r'^      - name: ', body, flags=re.M)[1:]:
        match = re.search(r'^        id: ([a-z_][a-z_0-9]*)$', block, re.M)
        if match:
            identifier = match.group(1)
            if identifier in steps:
                raise SystemExit('load has a duplicate step id')
            steps[identifier] = block
    return steps


def check_load_policy(body, steps):
    required = (
        (r"^    if: github.event_name != 'pull_request'$", 'wrong event condition'),
        (r'^    runs-on: ubuntu-24\.04$', 'wrong runner'),
        (r'^    timeout-minutes: 60$', 'wrong job timeout'),
        (r'^      fail-fast: false$', 'matrix must keep both legs'),
        (r'^        image: \[musl, cc-debian13\]$', 'wrong image matrix'),
    )
    for pattern, message in required:
        require_load(body, pattern, message)
    if re.search(r'^    continue-on-error:', body, re.M):
        raise SystemExit('load must not ignore job failures')
    order = ('runner', 'prepare', 'build_musl', 'build_cc_debian13', 'build_pgtools',
             'start', 'login', 'password', 'approver', 'seed', 'timings',
             'memory_timings', 'flood', 'memory_check', 'values', 'upload',
             'diagnostics', 'cleanup')
    if tuple(steps) != order:
        raise SystemExit('load steps do not follow ADR-0128 (j)')
    bounds = {'timings', 'flood', 'memory_check'}
    conditional = "        continue-on-error: ${{ matrix.image == 'cc-debian13' }}"
    if len(re.findall(r'^ *continue-on-error:', body, re.M)) != len(bounds):
        raise SystemExit('load ignores failures outside its bound steps')
    for identifier, block in steps.items():
        lines = [line for line in block.splitlines() if 'continue-on-error:' in line]
        if lines != ([conditional] if identifier in bounds else []):
            raise SystemExit('load ignores a failure outside the cc-debian13 bounds')


def check_load_builds(steps):
    for leg, dockerfile, scope in (
        ('musl', 'deploy/Dockerfile', 'recipe-server'),
        ('cc_debian13', 'deploy/ci/Dockerfile.cc-debian13', 'recipe-server-cc-debian13'),
    ):
        block = steps['build_' + leg]
        image = leg.replace('_', '-')
        require_load(block, rf"^        if: matrix.image == '{image}'$", 'wrong build condition')
        require_load(block, rf'^          file: {re.escape(dockerfile)}$', 'wrong Dockerfile')
        require_load(block, rf'^          cache-from: type=gha,scope={scope}$', 'wrong cache scope')
        for setting in ('load: true', 'push: false', 'tags: recipe-server:ci-load-${{ matrix.image }}'):
            require_load(block, '^          ' + re.escape(setting) + '$', 'wrong server image setting')
        if leg == 'musl' and 'cache-to:' in block:
            raise SystemExit('load musl must not write the build cache')
        if leg == 'cc_debian13':
            require_load(block, r'^          cache-to: type=gha,mode=max,scope=recipe-server-cc-debian13$',
                         'cc-debian13 must use its own writable cache')
    require_load(steps['build_pgtools'], r'^          tags: recipe-pgtools:ci-load-\$\{\{ matrix.image \}\}$',
                 'pgtools tag does not match the stack')


def check_load_commands(steps):
    commands = {
        'runner': 'python3 -I -B deploy/ci/load_flood.py runner',
        'prepare': 'bash deploy/ci/prepare.sh', 'start': 'bash deploy/ci/start.sh',
        'login': 'bash deploy/ci/login.sh', 'password': 'http_probe password',
        'approver': 'admin.create_user("load-approver")',
        'seed': 'python3 -I -B deploy/ci/load_seed.py',
        'timings': 'load_timing.py', 'memory_timings': 'bash deploy/ci/load.sh memory-timings',
        'flood': 'bash deploy/ci/load.sh flood', 'memory_check': 'bash deploy/ci/load.sh memory-check',
        'diagnostics': 'bash deploy/ci/diagnostics.sh', 'cleanup': 'bash deploy/ci/cleanup.sh',
    }
    for identifier, command in commands.items():
        if command not in steps[identifier]:
            raise SystemExit('load is missing the command of step ' + identifier)
    require_load(steps['seed'], r'^        timeout-minutes: 30$', 'wrong seed timeout')
    for identifier in ('memory_timings', 'flood', 'memory_check', 'values'):
        require_load(steps[identifier],
                     r"^        if: \$\{\{ !cancelled\(\) && steps.seed.outcome == 'success' \}\}$",
                     'missing result collection after a bound failure')
    require_load(steps['diagnostics'], r'^        if: failure\(\)$', 'missing failure diagnostics')
    require_load(steps['cleanup'], r'^        if: always\(\)$', 'missing unconditional cleanup')
    require_load(steps['flood'], r'^        timeout-minutes: [1-9][0-9]*$', 'flood needs a step timeout')


def check_load_artifacts(steps):
    upload = steps['upload']
    require_load(upload, r'^        uses: actions/upload-artifact@[0-9a-f]{40}(?: #.*)?$',
                 'upload action must use a pinned revision')
    require_load(upload, r'^          name: load-\$\{\{ matrix.image \}\}$', 'artifact must name its leg')
    require_load(upload, r'^          if-no-files-found: error$', 'missing values must fail the upload')
    require_load(upload, r"^        if: \$\{\{ !cancelled\(\) && steps.values.outcome == 'success' \}\}$",
                 'upload must require the values check')
    paths = re.findall(r'^            (.+)$', upload, re.M)
    prefix = '${{ runner.temp }}/${{ env.COMPOSE_PROJECT_NAME }}/state/'
    expected = [prefix + f'load-{name}.json' for name in ('seed', 'timings', 'flood', 'memory')]
    if paths != expected:
        raise SystemExit('load must upload only its four JSON value reports')
    for field in ('seed_seconds', 'timings_seconds', 'logins_per_second', 'p99_seconds', 'peak'):
        if field not in steps['values']:
            raise SystemExit('load does not require all measured values')


def test_load_job():
    body = load_job_body()
    steps = load_job_steps(body)
    check_load_policy(body, steps)
    check_load_builds(steps)
    check_load_commands(steps)
    check_load_artifacts(steps)
    print('CI load job check passed.')


test_load_job()


def compose_smoke_steps():
    workflow = Path('.github/workflows/ci.yml').read_text(encoding='utf-8')
    match = re.search(r'^  compose:\n((?:    .*\n|\n)*)', workflow, re.M)
    if not match:
        raise SystemExit('ci.yml has no job compose')
    body = match.group(1)
    steps = re.split(r'^      - name: ', body, flags=re.M)[1:]
    return body, steps


def smoke_command_step(steps, command):
    matches = [index for index, block in enumerate(steps)
               if re.search(r'^        run: ' + re.escape(command) + r'$', block, re.M)]
    if len(matches) != 1:
        raise SystemExit('compose needs one step that runs ' + command)
    return matches[0]


def require_smoke(block, pattern, message):
    if not re.search(pattern, block, re.M):
        raise SystemExit('compose smoke: ' + message)


def check_smoke_upload(steps, smoke_index):
    uploads = [index for index, block in enumerate(steps)
               if re.search(r'^        uses: actions/upload-artifact@', block, re.M)]
    if len(uploads) != 1 or uploads[0] <= smoke_index:
        raise SystemExit('compose smoke needs one upload after its scan')
    upload = steps[uploads[0]]
    require_smoke(upload, r'^        uses: actions/upload-artifact@[0-9a-f]{40}(?: #.*)?$',
                  'pin the upload action')
    condition = "${{ !cancelled() && (steps.smoke.outcome == 'success' || steps.smoke.outcome == 'failure') }}"
    require_smoke(upload, '^        if: ' + re.escape(condition) + '$',
                  'upload scanned artifacts after a passed or failed suite')
    require_smoke(upload, r'^          retention-days: 1$', 'keep artifacts for one day')
    script = Path('deploy/ci/smoke.sh').read_text(encoding='utf-8')
    directory = re.search(r'^upload=\$CI_STATE/([a-z-]+)$', script, re.M)
    if not directory:
        raise SystemExit('smoke.sh has no upload directory')
    expected = '${{ runner.temp }}/${{ env.COMPOSE_PROJECT_NAME }}/state/' + directory.group(1)
    paths = re.findall(r'^          path: (.+)$', upload, re.M)
    if paths != [expected] or re.search(r'^            \S', upload, re.M):
        raise SystemExit('compose smoke must upload only the scanned upload directory')


def check_smoke_setup(steps, smoke_index):
    patterns = (
        (r'^        uses: actions/setup-node@[0-9a-f]{40}(?: #.*)?$',
         (r'^          node-version-file: e2e/\.nvmrc$', r'^          cache: npm$',
          r'^          cache-dependency-path: e2e/package-lock\.json$')),
        (r'^        run: npm ci$', (r'^        working-directory: e2e$',)),
        (r'^        uses: actions/cache@[0-9a-f]{40}(?: #.*)?$',
         (r'^          path: ~/\.cache/ms-playwright$',
          r"^          key: .*\$\{\{ runner.os \}\}.*\$\{\{ hashFiles\('e2e/package-lock.json'\) \}\}.*$")),
        (r'^        run: npx playwright install --with-deps chromium$',
         (r'^        working-directory: e2e$',)),
    )
    previous = -1
    for pattern, settings in patterns:
        matches = [index for index, block in enumerate(steps) if re.search(pattern, block, re.M)]
        if len(matches) != 1 or not previous < matches[0] < smoke_index:
            raise SystemExit('compose smoke setup is missing or out of order')
        previous = matches[0]
        block = steps[previous]
        if re.search(r'^        (?:if|continue-on-error):', block, re.M):
            raise SystemExit('compose smoke setup must run and report failures')
        for setting in settings:
            require_smoke(block, setting, 'wrong Node, npm, or Chromium cache setting')


def check_compose_smoke(body, steps):
    smoke_index = smoke_command_step(steps, 'bash deploy/ci/smoke.sh')
    before = smoke_command_step(steps, 'bash deploy/ci/pgtools.sh')
    after = smoke_command_step(steps, 'bash deploy/ci/stop.sh')
    if not before < smoke_index < after:
        raise SystemExit('compose smoke must run after pgtools.sh and before stop.sh')
    require_smoke(body, r'^    timeout-minutes: 45$', 'keep the job timeout at 45 minutes')
    smoke = steps[smoke_index]
    require_smoke(smoke, r'^        id: smoke$', 'give the smoke step its id')
    require_smoke(smoke, r'^        timeout-minutes: 10$', 'set the smoke timeout to 10 minutes')
    if 'GITHUB_ENV' in smoke:
        raise SystemExit('compose smoke must not write to GITHUB_ENV')
    if re.search(r'^        (?:if|continue-on-error|env):', smoke, re.M):
        raise SystemExit('compose smoke must run without workflow password variables and report failures')
    check_smoke_setup(steps, smoke_index)
    check_smoke_upload(steps, smoke_index)


def test_compose_smoke_step():
    body, steps = compose_smoke_steps()
    check_compose_smoke(body, steps)
    print('CI compose smoke step check passed.')


test_compose_smoke_step()
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


def check_secure_refusal(lines):
    command = r'^      (?:  |- )run: bash deploy/ci/rust-tests\.sh secure$'
    if len(re.findall(command, '\n'.join(lines), re.M)) != 1:
        raise SystemExit('secure refusal needs exactly one step that runs rust-tests.sh secure')
    header = '  test:'
    if header not in lines:
        raise SystemExit('ci.yml has no job test')
    start = lines.index(header) + 1
    end = start
    while end < len(lines) and (lines[end].startswith('    ') or not lines[end].strip()):
        end += 1
    body = '\n'.join(lines[start:end])
    restricted_key = r"""(['"]?)(?:if|needs|continue-on-error)\1[ \t]*:"""
    # Reject unsupported YAML keys instead of decoding them.
    plain_key = r"""(?:[A-Za-z0-9_-]+|"[A-Za-z0-9_-]+"|'[A-Za-z0-9_-]+')[ \t]*:"""
    unsupported_key = '(?!' + plain_key + r')[^\s#]'
    restricted_key = '(?:' + restricted_key + '|' + unsupported_key + ')'
    if re.search(r'^    ' + restricted_key, body, re.M):
        raise SystemExit('test must run on each pull request and report failures')
    steps = re.split(r'(?=^      - )', body, flags=re.M)[1:]
    matches = [step for step in steps if re.search(command, step, re.M)]
    if len(matches) != 1:
        raise SystemExit('secure refusal step must run in the job test')
    if re.search(r'^      (?:  |- )' + restricted_key, matches[0], re.M):
        raise SystemExit('secure refusal step must run and report failures')


def test_secure_refusal_step():
    lines = Path('.github/workflows/ci.yml').read_text(encoding='utf-8').splitlines()
    check_secure_refusal(lines)
    print('CI secure refusal step check passed.')


def test_secure_refusal_mutations():
    workflow = Path('.github/workflows/ci.yml').read_text(encoding='utf-8')
    anchors = (
        ('  test:\n', '    ', 'test must run on each pull request and report failures'),
        ('        run: bash deploy/ci/rust-tests.sh secure\n', '        ',
         'secure refusal step must run and report failures'),
    )
    for anchor, indent, message in anchors:
        if workflow.count(anchor) != 1:
            raise SystemExit('secure refusal mutation needs a unique anchor')
        cases = [('if', '${{ false }}'), ('needs', 'compose'),
                 ('continue-on-error', 'true'), (r'\u0069f', 'false'),
                 (r'continue-on-erro\u0072', 'true')]
        if indent == '    ':
            cases.append((r'ne\u0065ds', 'compose'))
        fields = []
        for key, value in cases:
            quotes = ('"',) if '\\' in key else ('', "'", '"')
            for quote in quotes:
                for space in ('', ' '):
                    field = f'{indent}{quote}{key}{quote}{space}: {value}\n'
                    fields.append(field)
        fields.extend((f'{indent}? if\n{indent}: false\n',
                       f'{indent}&disabled if: false\n'))
        if indent == '    ':
            fields.append(f'{indent}<<: *x\n')
        else:
            fields.extend((f'{indent}? continue-on-error\n{indent}: true\n',
                           f'{indent}!!str if: false\n'))
        for field in fields:
            fixture = workflow.replace(anchor, anchor + field, 1)
            try:
                check_secure_refusal(fixture.splitlines())
            except SystemExit as error:
                if str(error) != message:
                    raise SystemExit(f'wrong secure refusal error: {error}') from error
            else:
                raise SystemExit(f'secure refusal check accepted {field.strip()!r} at {indent!r}')
    print('CI secure refusal mutation checks passed.')


test_secure_refusal_step()
test_secure_refusal_mutations()
PY
