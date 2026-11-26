"""Test the structure of the load job and the validation of its reports."""

import json
import os
import re
import subprocess
import sys
from pathlib import Path
from tempfile import TemporaryDirectory
from textwrap import dedent
from types import ModuleType


def load_workflow_checks():
    # Run the structure check definitions from the local check.sh.
    path = Path(__file__).with_name('check.sh')
    pattern = r"^# Start of the load job checks\.\n(.*?)^# End of the load job checks\.$"
    match = re.search(pattern, path.read_text(encoding='utf-8'), re.MULTILINE | re.DOTALL)
    if not match:
        raise SystemExit('check.sh has no load job checks')
    checks = ModuleType('load_workflow_checks')
    vars(checks).update(Path=Path, re=re)
    exec(compile(match.group(1), str(path), 'exec'), vars(checks))  # noqa: S102
    return checks


def reject_load_fixture(checks, body, check, message):
    try:
        check(body, checks.load_job_steps(body))
    except SystemExit as error:
        if message not in str(error):
            raise SystemExit(f'load fixture failed for the wrong reason. {error}') from error
    else:
        raise SystemExit(f'load accepted a fixture without {message}')


def test_load_value_policy(checks, body, steps):
    blocks = re.split(r'(?=^      - name: )', body, flags=re.MULTILINE)
    without_value_leg = ''.join(
        block for block in blocks
        if not re.search(r'^        id: (value_seed|value|value_check)$', block, re.MULTILINE)
    )
    reject_load_fixture(checks, without_value_leg, checks.check_load_policy, 'the value leg')
    value_check = steps['value_check']
    without_allowance = re.sub(
        r'^        continue-on-error: .*\n', '', value_check, flags=re.MULTILINE,
    )
    reject_load_fixture(
        checks, body.replace(value_check, without_allowance, 1),
        checks.check_load_policy, 'value_check',
    )
    without_status = value_check.replace('!cancelled() && ', '', 1)
    reject_load_fixture(
        checks, body.replace(value_check, without_status, 1),
        lambda _, changed: checks.check_load_commands(changed), 'value_check',
    )
    value_seed = steps['value_seed']
    without_condition = re.sub(r'^        if: .*\n', '', value_seed, flags=re.MULTILINE)
    reject_load_fixture(
        checks, body.replace(value_seed, without_condition, 1),
        lambda _, changed: checks.check_load_commands(changed), 'value_seed must run',
    )
    seed_id = '        id: value_seed\n'
    with_comment = body.replace(
        seed_id, seed_id + '        # continue-on-error: comparison bounds only\n', 1,
    )
    checks.check_load_policy(with_comment, checks.load_job_steps(with_comment))
    with_allowance = body.replace(seed_id, seed_id + '        continue-on-error: true\n', 1)
    reject_load_fixture(
        checks, with_allowance, checks.check_load_policy,
        'load has the wrong continue-on-error for value_seed',
    )
    print('CI load value leg structure fixtures passed.')


def load_values_fixture():
    reading = dict.fromkeys(('peak', 'current', 'anon', 'file', 'limit', 'oom',
                             'oom_kill', 'max', 'RestartCount'), 0)
    reading.update(container_id='a' * 64, OOMKilled=False, peak=100, limit=536870912)
    phases = {f'value-{phase}': reading.copy() for phase in (
        'seed', 'precheck', 'baseline-1', 'round-1', 'baseline-2', 'round-2',
    )}
    phases['value-round-1'].update(peak=536870912, oom=1, max=1, RestartCount=1)
    phases['value-baseline-2']['container_id'] = 'b' * 64
    phases['value-round-2'].update(peak=402653184, container_id='b' * 64)
    rounds = []
    for number, body_bytes in ((1, 16777216), (2, 13206677)):
        phase = phases[f'value-round-{number}']
        reference = phases[f'value-baseline-{number}']['anon']
        rounds.append({
            'round': number, 'container_id': phase['container_id'], 'peak': phase['peak'],
            'body_bytes': body_bytes, 'run_statistic': phase['peak'] / 4,
            'mem_available_min_kib': 1024, 'reference_anon': reference,
            'barrier_anon': reference + 4 * body_bytes, 'overlap_margin': 0,
        })
    return {
        'seed': {'seed_seconds': 1},
        'timings': {'seed_seconds': 1, 'timings_seconds': dict.fromkeys(
            ('ApproveVersion', 'CreateVersion', 'GetVersion'), 0.1)},
        'flood': {'logins_per_second': 1, 'p99_seconds': 0.1},
        'memory': {'timings': reading, 'flood': reading, 'value': phases,
                   'peak': 536870912, 'adr_0064_o_memory_condition': 'met'},
        'value': {'passed': False, 'failures': ['round 2 reached the ceiling'], 'rounds': rounds,
                  'job_elapsed_s': 20, 'leg_elapsed_s': 10},
    }


def values_program(step):
    pattern = r"^          python3 -I -B - <<'PY'\n(.*?)^          PY$"
    match = re.search(pattern, step, re.MULTILINE | re.DOTALL)
    if not match:
        raise SystemExit('load values has no isolated Python check')
    return dedent(match.group(1))


def run_values_fixture(program, reports):
    with TemporaryDirectory() as directory:
        for name, report in reports.items():
            path = Path(directory) / f'load-{name}.json'
            path.write_text(json.dumps(report), encoding='utf-8')
        return subprocess.run(
            [sys.executable, '-I', '-B', '-'], input=program,
            env=dict(os.environ, CI_STATE=directory), capture_output=True, text=True,
            check=False,
        )


def test_missing_run_statistic(program):
    reports = load_values_fixture()
    del reports['value']['rounds'][1]['run_statistic']
    require_values_rejection(program, reports, 'load-value.json round 2 run_statistic')


def test_missing_overlap_margin(program):
    reports = load_values_fixture()
    del reports['value']['rounds'][1]['overlap_margin']
    require_values_rejection(program, reports, 'load-value.json round 2 overlap_margin')


def test_null_barrier_values(program):
    reports = load_values_fixture()
    reports['value']['rounds'][0].update(barrier_anon=None, overlap_margin=None)
    result = run_values_fixture(program, reports)
    if result.returncode:
        raise SystemExit(f'Load values rejected null barrier values. {result.stderr.strip()}')


def require_values_rejection(program, reports, expected):
    result = run_values_fixture(program, reports)
    if result.returncode != 1 or expected not in result.stderr:
        output = result.stderr.strip() or result.stdout.strip()
        raise SystemExit(f'Expected exit 1 and {expected}. Got exit {result.returncode}. {output}')


def test_load_values(step):
    program = values_program(step)
    result = run_values_fixture(program, load_values_fixture())
    if result.returncode:
        raise SystemExit(f'Load values rejected complete reports. {result.stderr.strip()}')
    reports = load_values_fixture()
    reports['memory']['value']['value-round-1']['peak'] = None
    require_values_rejection(program, reports, 'value-round-1 peak')
    reports = load_values_fixture()
    del reports['memory']['value']['value-round-2']
    require_values_rejection(program, reports, 'value-round-2 reading')
    test_missing_run_statistic(program)
    test_missing_overlap_margin(program)
    test_null_barrier_values(program)
    print('CI load values fixtures passed.')


def main():
    checks = load_workflow_checks()
    body = checks.load_job_body()
    steps = checks.load_job_steps(body)
    checks.check_load_policy(body, steps)
    test_load_value_policy(checks, body, steps)
    test_load_values(steps['values'])


if __name__ == '__main__':
    main()
