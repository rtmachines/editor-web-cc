"""Check the job pitr-recovery and its script without Docker (pitr-recovery design.md D8)."""

import hashlib
import importlib.util
import os
import pathlib
import re
import shutil
import subprocess
import tempfile
import time
import unittest
from unittest.mock import patch


CI = pathlib.Path(__file__).resolve().parent
ROOT = CI.parent.parent
SCRIPT = CI / "pitr-recovery.sh"
STEPS_FILE = CI / "switch-steps.sh"
COMMON_FILE = CI / "common.sh"
CHECK = CI / "check.sh"
JOB = "pitr-recovery"
PROJECT = "recipe-ci-1-1"
# The steps of design.md D8 in their order. The diagnostics step sits between refuse and cleanup.
STEPS = ("prepare", "markers", "drill", "refusals", "crash", "recover", "switch", "wal-line", "check", "refuse", "cleanup")
# The subcommands that have a body. Every step of D8 has one.
IMPLEMENTED = STEPS
# The case line of a subcommand. The step switch passes the message of the source line to the shared function.
CASE_LINES = {"switch": "  switch) switch_recovered ;;"}
USAGE = "CI: usage: pitr-recovery.sh " + "|".join(IMPLEMENTED)

DOCKER_CALLS_SPEC = importlib.util.spec_from_file_location("docker_calls", CI / "docker_calls.py")
docker_calls = importlib.util.module_from_spec(DOCKER_CALLS_SPEC)
DOCKER_CALLS_SPEC.loader.exec_module(docker_calls)


def workflow_lines():
    text = (ROOT / ".github" / "workflows" / "ci.yml").read_text(encoding="utf-8")
    return text.replace("\r\n", "\n").splitlines()


def job_body(name):
    """Return the text of one job of ci.yml."""
    lines = workflow_lines()
    header = f"  {name}:"
    if header not in lines:
        return ""
    start = lines.index(header) + 1
    end = start
    while end < len(lines) and (lines[end].startswith("    ") or not lines[end].strip()):
        end += 1
    return "\n".join(lines[start:end])


def step_blocks(body):
    """Return the steps of a job body. The first item holds the text before the first step."""
    return body.split("      - name: ")


def script_text():
    return SCRIPT.read_text(encoding="utf-8")


def shell_function(name):
    """Return the body of one function of pitr-recovery.sh, switch-steps.sh, or common.sh."""
    marker = f"\n{name}() {{\n"
    for text in (script_text(), STEPS_FILE.read_text(encoding="utf-8"), COMMON_FILE.read_text(encoding="utf-8")):
        if marker in text:
            return text.split(marker, 1)[1].split("\n}\n", 1)[0]
    raise KeyError(name)


def write_stub(path, body):
    path.write_text("#!/usr/bin/env bash\n" + body.lstrip("\n"), encoding="utf-8", newline="\n")
    path.chmod(0o755)


def step_result(done, log, state_dir):
    """Collect Docker records, the text log, and state files from one step."""
    argv_log = pathlib.Path(str(log) + ".argv")
    content = {path.name: path.read_text(encoding="utf-8") for path in state_dir.iterdir() if path.is_file()}
    content["argv"] = docker_calls.argument_records(argv_log.read_bytes() if argv_log.exists() else b"")
    return done, log.read_text(encoding="utf-8") if log.exists() else "", content


class WorkflowTests(unittest.TestCase):
    def setUp(self):
        self.body = job_body(JOB)
        if not self.body:
            self.fail(f"ci.yml has no job {JOB}")

    def test_trigger_matches_the_job_compose(self):
        compose = job_body("compose")
        for key in ("if", "runs-on"):
            expected = re.search(rf"^    {key}: .*$", compose, re.M)
            found = re.search(rf"^    {key}: .*$", self.body, re.M)
            self.assertIsNotNone(expected)
            self.assertIsNotNone(found, f"the job has no {key}")
            self.assertEqual(found.group(0), expected.group(0))
        self.assertIn("if: github.event_name != 'pull_request'", self.body)

    def test_job_has_its_own_project_and_image_tag(self):
        self.assertIn("COMPOSE_PROJECT_NAME: recipe-ci-${{ github.run_id }}-${{ github.run_attempt }}", self.body)
        self.assertIn("RECIPE_VERSION: ci-pitr-recovery", self.body)

    def test_job_builds_both_images_with_the_db_context(self):
        self.assertIn("tags: recipe-server:ci-pitr-recovery", self.body)
        self.assertIn("tags: recipe-pgtools:ci-pitr-recovery", self.body)
        self.assertIn("build-contexts: db=deploy/db", self.body)

    def test_job_checks_the_ci_scripts_and_prepares_the_runner(self):
        self.assertIn("run: bash deploy/ci/check.sh", self.body)
        self.assertIn("run: bash deploy/ci/prepare.sh", self.body)

    def test_job_runs_the_steps_of_d8_in_order(self):
        calls = re.findall(r"pitr-recovery\.sh (\S+)", self.body)
        self.assertEqual(calls, list(STEPS))

    def test_diagnostics_step_sits_between_the_last_check_and_cleanup(self):
        order = [self.body.find(f"pitr-recovery.sh {name}\n") for name in STEPS[:-1]]
        order.append(self.body.find("diagnostics.sh"))
        order.append(self.body.find("pitr-recovery.sh cleanup"))
        self.assertNotIn(-1, order)
        self.assertEqual(order, sorted(order))

    def test_cleanup_runs_always_and_diagnostics_only_on_failure(self):
        steps = step_blocks(self.body)
        cleanup = [step for step in steps if "pitr-recovery.sh cleanup" in step]
        self.assertEqual(len(cleanup), 1)
        self.assertIn("if: always()", cleanup[0])
        diagnostics = [step for step in steps if "diagnostics.sh" in step]
        self.assertEqual(len(diagnostics), 1)
        self.assertIn("if: failure()", diagnostics[0])

    def test_every_job_step_has_a_timeout_and_no_condition(self):
        steps = step_blocks(self.body)
        for name in STEPS[:-1]:
            with self.subTest(name=name):
                step = [text for text in steps if f"pitr-recovery.sh {name}\n" in text + "\n"]
                self.assertEqual(len(step), 1)
                self.assertRegex(step[0], r"timeout-minutes: \d+")
                self.assertNotIn("if:", step[0])

    def test_step_limits_stay_below_the_job_limit(self):
        limit = int(re.search(r"^    timeout-minutes: (\d+)$", self.body, re.M).group(1))
        steps = [int(value) for value in re.findall(r"^        timeout-minutes: (\d+)$", self.body, re.M)]
        self.assertTrue(steps)
        self.assertLess(sum(steps), limit)

    def test_the_budget_comment_names_the_sum_of_the_step_limits(self):
        steps = [int(value) for value in re.findall(r"^        timeout-minutes: (\d+)$", self.body, re.M)]
        named = re.search(r"step limits sum to (\d+) minutes", self.body)
        self.assertIsNotNone(named)
        self.assertEqual(int(named.group(1)), sum(steps))


class CheckScriptTests(unittest.TestCase):
    def test_check_sh_runs_this_test(self):
        pattern = "deploy/ci/test_*.py"
        self.assertTrue(pathlib.Path(__file__).resolve().relative_to(ROOT).match(pattern))
        self.assertIn(
            f'\nfor test in {pattern}\ndo\n  python3 -I -B "$test"\ndone\n',
            CHECK.read_text(encoding="utf-8"),
        )


class ScriptTests(unittest.TestCase):
    def setUp(self):
        if not SCRIPT.is_file():
            self.fail("deploy/ci/pitr-recovery.sh is missing")
        self.bash = shutil.which("bash")
        if self.bash is None:
            self.skipTest("bash is not available")

    def test_syntax(self):
        done = subprocess.run([self.bash, "-n", str(SCRIPT)], capture_output=True, check=False)
        self.assertEqual(done.returncode, 0, done.stderr)

    def test_file_uses_lf(self):
        self.assertNotIn(b"\r", SCRIPT.read_bytes())

    def test_early_end_requirement_matches_the_proof_result(self):
        proof_class = "S"
        proof_message = "none"
        lines = script_text().splitlines()

        if proof_class == "R":
            required = "1"
            expected_line = f'EARLY_END_ERROR="{proof_message}"'
        else:
            self.assertEqual(proof_class, "S")
            required = "0"
            expected_line = "# V-02: change B sets 1 with its fix of ADR-0106 (g) step 2."
            self.assertEqual(
                [line for line in lines if line.startswith("EARLY_END_ERROR=")],
                ['EARLY_END_ERROR=""'],
            )

        self.assertEqual(
            [line for line in lines if line.startswith("EARLY_END_REFUSAL_REQUIRED=")],
            [f"EARLY_END_REFUSAL_REQUIRED={required}"],
        )
        self.assertTrue(expected_line in lines, f"Cannot find {expected_line}")

    def test_script_sources_the_shared_files_in_order(self):
        text = script_text()
        self.assertTrue(text.startswith("#!/usr/bin/env bash\n"))
        self.assertIn("set -euo pipefail\n", text)
        common = text.find('source "$(dirname "${BASH_SOURCE[0]}")/common.sh"')
        steps = text.find('source "$(dirname "${BASH_SOURCE[0]}")/switch-steps.sh"')
        self.assertNotEqual(common, -1)
        self.assertNotEqual(steps, -1)
        self.assertLess(common, steps)

    def test_the_shared_steps_have_one_definition_in_the_shared_file(self):
        # design.md D7. The script calls load_active_volume, require_no_labelled_volume, and cleanup from switch-steps.sh.
        shared = STEPS_FILE.read_text(encoding="utf-8")
        for name in ("load_active_volume", "require_no_labelled_volume", "cleanup"):
            marker = f"\n{name}() {{\n"
            with self.subTest(name=name):
                self.assertEqual(shared.count(marker), 1, f"{name} is not defined once in switch-steps.sh")
                self.assertNotIn(marker, script_text(), f"{name} stays in pitr-recovery.sh")
                self.assertNotIn(marker, (CI / "restore-switch.sh").read_text(encoding="utf-8"))

    def test_prepare_runs_the_steps_in_order(self):
        body = shell_function("prepare")
        markers = [
            "start.sh",
            "login.sh",
            "http_probe password",
            "take_base_backup",
        ]
        positions = [body.find(marker) for marker in markers]
        self.assertNotIn(-1, positions, dict(zip(markers, positions)))
        self.assertEqual(positions, sorted(positions), dict(zip(markers, positions)))

    def test_prepare_writes_no_fixture_and_runs_no_pitr_step(self):
        body = shell_function("prepare")
        for forbidden in ("upgrade_smoke.py", "pitr-volume", "pg_switch_wal", "docker kill"):
            self.assertNotIn(forbidden, body)

    def test_prepare_ends_the_gap_segments_after_the_base_backup(self):
        # The gap run of refusals needs complete segments after the base backup (ADR-0106 (p) step 4).
        # Marker 2 must stay in the .partial file until crash (step 3 and step 5), so the switches come before marker 1.
        body = shell_function("prepare")
        self.assertEqual(body.count("switch_wal_segments"), 1, body)
        self.assertLess(body.index("take_base_backup"), body.index("switch_wal_segments"))

    def run_switch_wal_segments(self, sql_status=0):
        """Run the function switch_wal_segments alone with a stub of sql. Return the process and the SQL texts."""
        bash = shutil.which("bash")
        if bash is None:
            self.skipTest("bash is not available")
        with tempfile.TemporaryDirectory() as name:
            log = pathlib.Path(name) / "sql.log"
            program = (
                "set -euo pipefail\n"
                "fail() { printf 'CI: %s\\n' \"$1\" >&2; exit 1; }\n"
                f"sql() {{ printf '%s\\n' \"$*\" >>\"$SQL_LOG\"; return {sql_status}; }}\n"
                "GAP_SEGMENT_COUNT=2\n"
                "switch_wal_segments() {\n" + shell_function("switch_wal_segments") + "\n}\n"
                "switch_wal_segments\n"
            )
            env = dict(os.environ, SQL_LOG=str(log))
            done = subprocess.run([bash, "-c", program], capture_output=True, text=True, check=False, env=env)
            return done, log.read_text(encoding="utf-8") if log.exists() else ""

    def test_switch_wal_segments_writes_before_each_switch(self):
        # A switch with no new WAL does nothing, so a write comes before each switch.
        done, log = self.run_switch_wal_segments()
        self.assertEqual(done.returncode, 0, done.stderr)
        kinds = ["write" if "pg_logical_emit_message" in line else "switch" if "SELECT pg_switch_wal()" in line else line
                 for line in log.splitlines()]
        self.assertEqual(kinds, ["write", "switch", "write", "switch"], log)

    def test_switch_wal_segments_names_the_step_prepare_on_a_failure(self):
        done, _ = self.run_switch_wal_segments(sql_status=1)
        self.assertEqual(done.returncode, 1)
        self.assertEqual(done.stderr.strip(), "CI: pitr-recovery prepare: cannot write WAL before a segment switch.")

    def test_no_segment_switch_runs_after_marker_2(self):
        # ADR-0106 (p) steps 3 and 5. A switch after marker 2 ends its segment, so crash finds no .partial file.
        # It can also put the WAL position of crash on a segment boundary.
        for step in ("markers", "drill", "refusals", "crash"):
            with self.subTest(step=step):
                body = shell_function(step)
                self.assertNotIn("switch_wal_segments", body)
                self.assertNotIn("pg_switch_wal", body)

    def test_cleanup_deletes_volumes_only_through_the_helper(self):
        body = shell_function("cleanup")
        self.assertIn("delete_ci_volumes || status=$?", body)
        self.assertNotRegex(body, r"docker volume (rm|prune)")
        self.assertNotRegex(body, r"down\b.*(-v\b|--volumes)")
        self.assertNotIn("|| exit", body)
        self.assertNotIn("|| return", body)

    def test_volume_creation_carries_the_label(self):
        for line in script_text().splitlines():
            if "docker volume create" in line:
                self.assertIn("recipe.ci", line)

    def test_every_subcommand_of_the_case_has_a_function(self):
        for name in IMPLEMENTED:
            with self.subTest(name=name):
                self.assertIn(CASE_LINES.get(name, f"  {name}) {name.replace('-', '_')} ;;"), script_text())
                self.assertTrue(shell_function(name.replace("-", "_")).strip())

    def test_unknown_subcommand_fails_with_the_usage_line(self):
        done = self.run_script("other")
        self.assertEqual(done.returncode, 1)
        self.assertEqual(done.stderr.strip(), USAGE)

    def test_missing_subcommand_fails_with_the_usage_line(self):
        done = self.run_script()
        self.assertEqual(done.returncode, 1)
        self.assertEqual(done.stderr.strip(), USAGE)

    def test_a_subcommand_without_a_body_does_not_pass(self):
        for name in sorted(set(STEPS) - set(IMPLEMENTED)):
            with self.subTest(name=name):
                done = self.run_script(name)
                self.assertNotEqual(done.returncode, 0)
                self.assertEqual(done.stderr.strip(), USAGE)

    def test_cleanup_continues_after_a_failed_down(self):
        done, log = self.run_cleanup(down_status=1)
        self.assertNotEqual(done.returncode, 0)
        self.assertIn("volume rm recipe-ci-1-1_pgdata", log)
        self.assertEqual(log.count("sudo umount"), 2)

    def test_cleanup_succeeds_when_every_step_succeeds(self):
        done, log = self.run_cleanup(down_status=0)
        self.assertEqual(done.returncode, 0, done.stderr)
        self.assertIn("down --remove-orphans", log)
        self.assertIn("--profile restore", log)
        self.assertIn("--profile pitr", log)
        self.assertIn("volume ls -q --filter label=recipe.ci=recipe-ci-1-1", log)

    def test_cleanup_fails_when_a_labelled_volume_stays(self):
        done, log = self.run_cleanup(down_status=0, volume_stays=True)
        self.assertEqual(done.returncode, 1)
        self.assertIn("CI: cleanup: a volume with the label recipe.ci=recipe-ci-1-1 stays: recipe-ci-1-1_pgdata", done.stderr)
        self.assertEqual(log.count("sudo umount"), 2)

    def test_cleanup_checks_the_labelled_volumes_after_the_deletion(self):
        done, log = self.run_cleanup(down_status=0)
        self.assertEqual(done.returncode, 0, done.stderr)
        listing = "volume ls -q --filter label=recipe.ci=recipe-ci-1-1\n"
        self.assertEqual(log.count(listing), 2)
        self.assertLess(log.index("volume rm "), log.rindex(listing))
        self.assertLess(log.rindex(listing), log.index("sudo umount"))

    def test_cleanup_uses_the_volume_that_switch_made_active(self):
        done, log = self.run_cleanup(down_status=0, active_volume="recipe-ci-1-1_pitr")
        self.assertEqual(done.returncode, 0, done.stderr)
        self.assertIn("RECIPE_PGDATA_VOLUME=recipe-ci-1-1_pitr", log)

    def test_cleanup_rejects_an_invalid_active_volume_name(self):
        done, _ = self.run_cleanup(down_status=0, active_volume="bad name;")
        self.assertNotEqual(done.returncode, 0)
        self.assertIn("CI: the active volume name is invalid", done.stderr)

    def environment(self, directory):
        env = {key: value for key, value in os.environ.items() if not key.startswith(("GITHUB_", "RUNNER_", "COMPOSE_"))}
        env.update({
            "GITHUB_ACTIONS": "true",
            "GITHUB_WORKSPACE": str(ROOT),
            "RUNNER_TEMP": str(directory / "runner"),
            "COMPOSE_PROJECT_NAME": PROJECT,
        })
        return env

    def run_script(self, *arguments):
        with tempfile.TemporaryDirectory() as name:
            return subprocess.run([self.bash, str(SCRIPT), *arguments], capture_output=True, text=True,
                                  check=False, env=self.environment(pathlib.Path(name)))

    def run_cleanup(self, down_status, volume_stays=False, active_volume=None):
        """Run cleanup with stubs for sudo, docker, and mountpoint. Return the process and the call log."""
        with tempfile.TemporaryDirectory() as name:
            directory = pathlib.Path(name)
            stubs = directory / "bin"
            stubs.mkdir()
            log = directory / "calls.log"
            write_stub(stubs / "sudo", SUDO_STUB)
            write_stub(stubs / "docker", DOCKER_STUB)
            write_stub(stubs / "mountpoint", "exit 0")
            env = self.environment(directory)
            env["PATH"] = f"{stubs}{os.pathsep}{env['PATH']}"
            env.update({"STUB_LOG": str(log), "STUB_DOWN_STATUS": str(down_status)})
            if volume_stays:
                env["STUB_VOLUME_STAYS"] = "1"
            if active_volume is not None:
                state = directory / "runner" / PROJECT / "state"
                state.mkdir(parents=True)
                (state / "active.volume").write_text(active_volume + "\n", encoding="utf-8", newline="\n")
            done = subprocess.run([self.bash, str(SCRIPT), "cleanup"], capture_output=True, text=True,
                                  check=False, env=env)
            return done, log.read_text(encoding="utf-8") if log.exists() else ""


# The stub of sudo logs the environment variable RECIPE_PGDATA_VOLUME that --preserve-env passes on.
SUDO_STUB = """
while [[ ${1:-} == --* ]]; do shift; done
if [[ ${1:-} == docker ]]; then echo "RECIPE_PGDATA_VOLUME=${RECIPE_PGDATA_VOLUME:-}" >>"$STUB_LOG"; exec "$@"; fi
echo "sudo $*" >>"$STUB_LOG"
"""

DOCKER_STUB = """
echo "$*" >>"$STUB_LOG"
case "$*" in
  *" down "*) exit "$STUB_DOWN_STATUS" ;;
  "volume ls "*)
    # The volume leaves the list after volume rm, unless STUB_VOLUME_STAYS is set.
    if [[ -n ${STUB_VOLUME_STAYS:-} ]] || ! grep -q '^volume rm ' "$STUB_LOG"; then echo recipe-ci-1-1_pgdata; fi
    ;;
  "volume inspect "*) echo recipe-ci-1-1 ;;
esac
exit 0
"""


# The stub of date logs the read of the epoch seconds. It then calls the real date.
# With STUB_ELAPSED the stub runs a fake clock for the step check. Before app starts, the clock stands 5 s after recovery.start.
# The first read after app starts gives recovery.start plus STUB_ELAPSED. Each later read adds 3 s.
DATE_STUB = """
if [[ $* == "+%s" ]]; then
  echo "clock +%s" >>"$STUB_LOG"
  if [[ -n ${STUB_ELAPSED:-} ]]; then
    start=$(<"$CI_STATE/recovery.start")
    if ! grep -q 'wait-timeout 180 app$' "$STUB_LOG"; then echo $((start + 5)); exit 0; fi
    reads=$(cat "$STUB_COUNTER.after" 2>/dev/null || echo 0)
    echo $((reads + 1)) >"$STUB_COUNTER.after"
    echo $((start + STUB_ELAPSED + reads * 3))
    exit 0
  fi
fi
exec /usr/bin/date "$@"
"""

MARKER_1_HASH = "a" * 64
MARKER_2_HASH = "b" * 64
MARKER_3_HASH = "c" * 64
# The passwords of the markers 1 and 2 in the state of the step check. The stub of openssl prints other values.
MARKER_1_PASSWORD = "1" * 64
MARKER_2_PASSWORD = "2" * 64
TARGET_TIME = "2026-10-30T12:00:05Z"
DRILL_LINE = (
    '{"timestamp":"2026-10-30T12:00:00Z","level":"INFO","job":"pitr-volume","message":"pitr volume completed",'
    '"base_backup":"base_20261030_115900","system_identifier":"7234567890123456789","timeline":2,"chain_seq":41,'
    '"occurred_at":"2026-10-30T12:00:01Z","action":"OwnPasswordChanged","heartbeat_last_at":"2026-10-30T12:00:00Z","drill":true}'
)
PASSWORD_CALL = r"^python3 .*http_probe\.py password$"
TARGET_ERROR_LINE = (
    '{"timestamp":"2026-10-30T12:00:07Z","level":"ERROR","job":"pitr-volume",'
    '"message":"the temporary server did not finish the recovery"}'
)
GAP_ERROR_LINE = (
    '{"timestamp":"2026-10-30T12:00:09Z","level":"ERROR","job":"pitr-volume",'
    '"message":"the recovery ended before the end of the WAL archive"}'
)
# The ERROR line of the guard of walarchive-entry for a segment of another timeline (ADR-0106 (m)).
TIMELINE_ERROR = "segments of another timeline are in the WAL archive"
REFUSE_ERROR_LINE = (
    '{"timestamp":"2026-10-30T12:20:00Z","level":"ERROR","job":"walarchive-entry",'
    '"message":"segments of another timeline are in the WAL archive"}'
)
# The target directory of the move of the step wal-line. @CI_ROOT@ becomes the CI root of the test run.
MOVED_DIR = "@CI_ROOT@/walarchive-disk/walarchive.moved-20261030_120100"
REFUSE_WRONG_LINE = "CI: refuse: the ERROR line of walarchive-entry is wrong for the copy of /wal with a segment of the old timeline."


# The completed line of a recovery to the end. The marker 2 is the newest audit entry. The run is no drill.
RECOVER_LINE = DRILL_LINE.replace('"drill":true', '"drill":false').replace('"chain_seq":41', '"chain_seq":42')
EARLY_END_SEGMENT = "000000010000000000000006"
EARLY_END_STATE = f"0/6000060|{EARLY_END_SEGMENT}|96|16777216|8192\n"
EARLY_END_LINE = RECOVER_LINE.replace('"chain_seq":42', '"chain_seq":41')
EARLY_END_ERROR_LINE = TARGET_ERROR_LINE.replace("the temporary server did not finish the recovery",
                                                "the newest audit entry is invalid")
EARLY_END_HASHES_LINE = (
    '{"timestamp":"2026-10-30T12:00:06Z","level":"INFO","job":"pitr-volume",'
    f'"message":"configuration files of the copy hashed","postgresql_conf":"{"0" * 64}"}}'
)
EXPECTED_LSN = "0/3000060"
OLD_VOLUME = "recipe-ci-1-1_pgdata"
HEARTBEAT_QUERY = r"FROM public\.app_setting WHERE key = 'heartbeat\.last_at'"
NEW_VOLUME = r"recipe-editor_pgdata_[0-9]{8}_[0-9]{6}"
# The volume that recover made. The step switch reads its name from new.volume.
NEW_VOLUME_NAME = "recipe-editor_pgdata_20261030_120100"


# The values of the secret files that the check of the log of db reads. No stub output holds one of them.
SECRET_VALUES = {name: f"secret-value-of-{name}" for name in ("postgres", "migrator", "app", "backup", "replicator")}


def password(number):
    """Return the nth password that the stub of openssl prints."""
    return f"{number:064d}"


def step_environment(directory, stubs, log, step):
    """Return the default environment for a step with stubs."""
    env = os.environ.copy()
    for key in [key for key in env if key.startswith(("GITHUB_", "RUNNER_", "COMPOSE_", "STUB_"))]:
        del env[key]
    env.update({
        "GITHUB_ACTIONS": "true",
        "GITHUB_WORKSPACE": str(ROOT),
        "RUNNER_TEMP": str(directory / "runner"),
        "COMPOSE_PROJECT_NAME": PROJECT,
        "RECIPE_VERSION": "ci-pitr-recovery",
        "CI_WAIT_ATTEMPTS": "3",
        "PATH": f"{stubs}{os.pathsep}{env['PATH']}",
        "STUB_LOG": str(log),
        "STUB_COUNTER": str(directory / "counter"),
        "STUB_PITR_OUTPUT": DRILL_LINE,
        "STUB_TARGET_OUTPUT": TARGET_ERROR_LINE,
        "STUB_GAP_OUTPUT": GAP_ERROR_LINE,
        "STUB_EARLY_END_POSITION": EARLY_END_STATE.rstrip("\n"),
        "STUB_EARLY_END_OUTPUT": EARLY_END_LINE,
        "STUB_RECOVER_OUTPUT": RECOVER_LINE,
        "STUB_REFUSE_OUTPUT": REFUSE_ERROR_LINE,
    })
    if step == "markers":
        env["STUB_SEGMENT"] = EARLY_END_SEGMENT
        env["STUB_LSN"] = "0/60000A0"
    return env


class StepTests(unittest.TestCase):
    """Run the steps markers and drill with stubs for docker, sudo, python3, openssl, and sleep (design.md D8)."""

    def setUp(self):
        if not SCRIPT.is_file():
            self.fail("deploy/ci/pitr-recovery.sh is missing")
        self.bash = shutil.which("bash")
        if self.bash is None:
            self.skipTest("bash is not available")

    def test_step_stub_records_spaces_empty_arguments_and_newlines(self):
        with tempfile.TemporaryDirectory() as name:
            directory = pathlib.Path(name)
            stub = directory / "docker"
            log = directory / "calls.log"
            write_stub(stub, STEP_DOCKER_STUB)
            env = {**os.environ, "STUB_LOG": str(log)}
            done = subprocess.run([self.bash, str(stub), "a b", "", "x\ny"],
                                  capture_output=True, text=True, check=False, env=env)
            self.assertEqual(done.returncode, 0, done.stderr)
            records = pathlib.Path(str(log) + ".argv")
            self.assertTrue(records.is_file(), "The stub wrote no Docker argument records")
            self.assertEqual(records.read_bytes(), b"3\x00a b\x00\x00x\ny\x00")

    def assert_window_mutant_rejected(self, step, anchor, replacement, state, guard, expected):
        original = script_text()
        self.assertEqual(original.count(anchor), 1)
        with tempfile.TemporaryDirectory() as name:
            directory = pathlib.Path(name)
            for source in (SCRIPT, COMMON_FILE, STEPS_FILE):
                shutil.copyfile(source, directory / source.name)
            mutant = directory / SCRIPT.name
            mutant.write_text(original.replace(anchor, replacement), encoding="utf-8", newline="\n")
            with patch.dict(globals(), SCRIPT=mutant):
                if step == "check":
                    done, log, files = self.run_check(state=state)
                else:
                    done, log, files = self.run_step(step, state=state)
            self.assertEqual(done.returncode, 0, done.stderr + log)
            self.assertIn(" " + " ".join([expected[1], *expected[2]]) + "\n", log)
            self.assertTrue(files["argv"], "The run has no Docker argument records")
            self.assertEqual([docker_calls.docker_call(record) for record in guard(files["argv"])], [expected])

    def test_window_guard_rejects_crash_mutant(self):
        anchor = '  compose rm -s -f app backup || fail "crash: cannot delete app and backup."'
        self.assert_window_mutant_rejected(
            "crash", anchor, anchor + "\n  compose up -d --no-deps app backup",
            self.crash_state(), docker_calls.forbidden_window_calls,
            (True, "up", ["-d", "--no-deps", "app", "backup"]))

    def test_window_guard_rejects_check_prefix_mutant(self):
        anchor = ('  sql -c "TRUNCATE public.session, public.login_attempt" >/dev/null'
                  ' || fail "check: cannot truncate session and login_attempt."')
        self.assert_window_mutant_rejected(
            "check", anchor, "  compose up -d --no-deps app backup || true\n" + anchor,
            self.check_state(), docker_calls.forbidden_check_window_calls,
            (True, "up", ["-d", "--no-deps", "app", "backup"]))

    def walarchive_window_hits(self, step, records):
        start = ("rm", ["-s", "-f", "walarchive"]) if step == "crash" else None
        end = ("up", ["-d", "--no-deps", "walarchive"]) if step == "wal-line" else None
        return docker_calls.forbidden_window_calls(
            records, window=docker_calls.record_window(records, start=start, end=end),
            services=("walarchive",))

    def test_window_guard_rejects_crash_walarchive_mutant(self):
        anchor = '  compose rm -s -f walarchive || fail "crash: cannot delete walarchive."'
        self.assert_window_mutant_rejected(
            "crash", anchor, anchor + "\n  compose up -d --no-deps walarchive || true",
            self.crash_state(), lambda records: self.walarchive_window_hits("crash", records),
            (True, "up", ["-d", "--no-deps", "walarchive"]))

    def run_step(self, step, env_extra=None, state=None):
        """Return the process, the call log, and state content with Docker records."""
        with tempfile.TemporaryDirectory() as name:
            directory = pathlib.Path(name)
            stubs = directory / "bin"
            stubs.mkdir()
            log = directory / "calls.log"
            write_stub(stubs / "sudo", STEP_SUDO_STUB)
            write_stub(stubs / "docker", STEP_DOCKER_STUB)
            write_stub(stubs / "python3", STEP_PYTHON_STUB)
            write_stub(stubs / "openssl", STEP_OPENSSL_STUB)
            write_stub(stubs / "sleep", "exit 0")
            write_stub(stubs / "date", DATE_STUB)
            state_dir = directory / "runner" / PROJECT / "state"
            state_dir.mkdir(parents=True)
            # The check of the log of db reads the values of the secret files.
            secrets_dir = directory / "runner" / PROJECT / "secrets"
            secrets_dir.mkdir()
            for secret_name, secret_value in SECRET_VALUES.items():
                (secrets_dir / f"{secret_name}_password").write_text(secret_value + "\n", encoding="utf-8", newline="\n")
            env = step_environment(directory, stubs, log, step)
            env.update(env_extra or {})
            # Bash can read a Windows path in another form. The placeholder gets the form that bash sees.
            seen = ""
            if any("@CI_ROOT@" in text for text in (state or {}).values()):
                seen = subprocess.run([self.bash, "-c", 'printf %s "$RUNNER_TEMP"'], capture_output=True, text=True,
                                      check=True, env=env).stdout
            for file_name, text in (state or {}).items():
                (state_dir / file_name).write_text(text.replace("@CI_ROOT@", f"{seen}/{PROJECT}"), encoding="utf-8", newline="\n")
            done = subprocess.run([self.bash, str(SCRIPT), step], capture_output=True, text=True, check=False, env=env)
            return step_result(done, log, state_dir)

    def markers_state(self):
        # The step prepare leaves the session cookie and the passwords of the administrator.
        return {"admin_password": "admin-secret\n", "new_password": "new-secret\n", "cookies.json": "{}\n"}

    def drill_state(self):
        return {
            "marker1.seq": "41\n",
            "marker1.hash": MARKER_1_HASH + "\n",
            "target.time": TARGET_TIME + "\n",
        }

    def refusals_state(self):
        # The step markers leaves the two sequences and the WAL position before marker 2.
        return {"marker1.seq": "41\n", "marker2.seq": "42\n", "marker2.hash": MARKER_2_HASH + "\n",
                "early-end.state": EARLY_END_STATE}

    def assert_ordered(self, log, patterns):
        """Each pattern must match after the end of the match before it."""
        offset = 0
        for pattern in patterns:
            match = re.compile(pattern, re.M).search(log, offset)
            self.assertIsNotNone(match, f"no {pattern!r} after offset {offset}:\n{log}")
            offset = match.end()

    # markers

    def test_markers_pass_path_runs_the_steps_in_order(self):
        done, log, _ = self.run_step("markers", {"STUB_SEGMENT": EARLY_END_SEGMENT}, self.markers_state())
        self.assertEqual(done.returncode, 0, done.stderr + log)
        self.assert_ordered(log, [
            HEARTBEAT_QUERY,
            PASSWORD_CALL,
            r"ORDER BY chain_seq DESC LIMIT 1",
            r"date_trunc\('second'",
            r"clock_timestamp\(\) >=",
            r"pg_current_wal_insert_lsn\(\).*pg_walfile_name_offset\(.*\).*wal_segment_size.*wal_block_size",
            PASSWORD_CALL,
            r"ORDER BY chain_seq DESC LIMIT 1",
            r"occurred_at",
            r"SELECT pg_current_wal_lsn\(\)",
            r"flush_lsn >=",
            r"pg_walfile_name",
            rf"^sudo test -f .*{EARLY_END_SEGMENT}\.partial$",
            rf"^sudo test -e .*{EARLY_END_SEGMENT}$",
        ])

    def test_markers_read_the_sizes_in_the_order_of_the_state_file(self):
        assignment = re.search(r'^EARLY_END_POSITION_SQL="([^"]+)"$', script_text(), re.MULTILINE)
        self.assertIsNotNone(assignment, "Cannot find EARLY_END_POSITION_SQL.")
        sql = assignment.group(1).replace("\\\n", "")
        select_list = re.search(r"\)\s*SELECT\s+(.+?)\s+FROM\s+wal_position\b", sql, re.IGNORECASE | re.DOTALL)
        self.assertIsNotNone(select_list, "Cannot find the WAL position select list.")
        self.assertEqual(
            [column.strip() for column in select_list.group(1).split(",")],
            ["lsn", "file_name", "file_offset", "seg.setting", "blk.setting"],
        )
        for alias, setting in (("seg", "wal_segment_size"), ("blk", "wal_block_size")):
            self.assertRegex(
                sql, rf"(?i:\bJOIN\s+pg_settings\s+AS\s+{alias}\s+ON\s+{alias}\.name)\s*=\s*'{setting}'",
            )

    def test_markers_record_the_record_before_marker_2(self):
        done, log, state = self.run_step("markers", {"STUB_SEGMENT": EARLY_END_SEGMENT}, self.markers_state())
        self.assertEqual(done.returncode, 0, done.stderr + log)
        self.assertEqual(state.get("early-end.state"), EARLY_END_STATE)
        done, _, _ = self.run_step("markers", {"STUB_SEGMENT": "000000010000000000000007"}, self.markers_state())
        self.assertEqual(done.returncode, 1)
        self.assertEqual(done.stderr.strip(), "CI: markers: the record before marker 2 is not in the newest segment.")

    def test_markers_wait_for_the_heartbeat_row_before_marker_1(self):
        done, log, _ = self.run_step("markers", state=self.markers_state())
        self.assertEqual(done.returncode, 0, done.stderr + log)
        self.assertEqual(log.count("heartbeat.last_at"), 1)
        self.assertLess(log.index("heartbeat.last_at"), log.index("http_probe.py password"))
        self.assertIn("CI: markers: wait for the row heartbeat.last_at", done.stdout + done.stderr)
        self.assertIn("CI: markers: the row heartbeat.last_at exists", done.stdout + done.stderr)

    def test_markers_poll_again_when_the_heartbeat_row_appears_late(self):
        done, log, _ = self.run_step("markers", {"STUB_HEARTBEAT_MISSES": "1", "CI_HEARTBEAT_WAIT_SECONDS": "3"}, self.markers_state())
        self.assertEqual(done.returncode, 0, done.stderr + log)
        self.assertEqual(log.count("heartbeat.last_at"), 2)
        self.assertLess(log.rindex("heartbeat.last_at"), log.index("http_probe.py password"))

    def test_markers_stop_when_the_heartbeat_row_never_appears(self):
        done, log, state = self.run_step("markers", {"STUB_HEARTBEAT_MISSES": "99", "CI_HEARTBEAT_WAIT_SECONDS": "3"}, self.markers_state())
        self.assertEqual(done.returncode, 1)
        self.assertEqual(done.stderr.strip(), "CI: markers: the job heartbeat wrote no row heartbeat.last_at in 3 s.")
        self.assertEqual(log.count("heartbeat.last_at"), 3)
        self.assertNotIn("http_probe", log)
        self.assertNotIn("marker1.seq", state)

    def test_markers_limit_the_heartbeat_wait_to_the_interval_plus_a_margin(self):
        text = SCRIPT.read_text(encoding="utf-8")
        self.assertIn("HEARTBEAT_INTERVAL_SECONDS=300", text)
        self.assertIn("HEARTBEAT_MARGIN_SECONDS=60", text)
        self.assertIn("reliability.md 7.11", text)

    def test_markers_rotate_the_password_files_for_each_command(self):
        done, log, state = self.run_step("markers", state=self.markers_state())
        self.assertEqual(done.returncode, 0, done.stderr + log)
        self.assertIn(f"passwords current=new-secret next={password(1)}", log)
        self.assertIn(f"passwords current={password(1)} next={password(2)}", log)
        # The step check logs in with the password of the marker 2 and tests the password of the marker 1.
        self.assertEqual(state["new_password"], password(2) + "\n")
        self.assertEqual(state["admin_password"], password(1) + "\n")

    def test_markers_save_both_markers_and_the_target_time(self):
        done, log, state = self.run_step("markers", state=self.markers_state())
        self.assertEqual(done.returncode, 0, done.stderr + log)
        self.assertEqual(state["marker1.seq"], "41\n")
        self.assertEqual(state["marker1.hash"], MARKER_1_HASH + "\n")
        self.assertEqual(state["marker2.seq"], "42\n")
        self.assertEqual(state["marker2.hash"], MARKER_2_HASH + "\n")
        self.assertEqual(state["target.time"], TARGET_TIME + "\n")

    def test_markers_force_no_segment_switch(self):
        done, log, _ = self.run_step("markers", state=self.markers_state())
        self.assertEqual(done.returncode, 0, done.stderr + log)
        self.assertNotIn("pg_switch_wal", log)
        self.assertNotIn("pg_switch_wal", shell_function("markers"))

    def test_markers_require_the_state_of_prepare(self):
        done, log, _ = self.run_step("markers", state={})
        self.assertEqual(done.returncode, 1)
        self.assertEqual(done.stderr.strip(), "CI: markers: run prepare first.")
        self.assertNotIn("http_probe", log)

    def test_markers_stop_when_the_target_time_is_not_between_the_commits(self):
        done, _, _ = self.run_step("markers", {"STUB_BETWEEN": "f"}, self.markers_state())
        self.assertEqual(done.returncode, 1)
        self.assertEqual(done.stderr.strip(), "CI: markers: the target time does not lie between the two commits.")

    def test_markers_stop_when_the_target_time_has_a_wrong_form(self):
        done, log, _ = self.run_step("markers", {"STUB_TARGET": "2026-10-30 12:00:05"}, self.markers_state())
        self.assertEqual(done.returncode, 1)
        self.assertEqual(done.stderr.strip(), "CI: markers: the target time is invalid.")
        self.assertEqual(len(re.findall(PASSWORD_CALL, log, re.M)), 1)

    def test_markers_stop_when_the_newest_entry_is_no_password_change(self):
        done, _, _ = self.run_step("markers", {"STUB_ACTION": "UserLoggedIn"}, self.markers_state())
        self.assertEqual(done.returncode, 1)
        self.assertEqual(done.stderr.strip(), "CI: markers: the newest audit entry of marker 1 is not OwnPasswordChanged.")

    def test_markers_stop_when_the_marker_row_is_malformed(self):
        done, _, _ = self.run_step("markers", {"STUB_ROW": "0|short|OwnPasswordChanged"}, self.markers_state())
        self.assertEqual(done.returncode, 1)
        self.assertEqual(done.stderr.strip(), "CI: markers: the audit marker identity of marker 1 is invalid.")

    def test_markers_stop_when_the_first_command_fails(self):
        done, log, state = self.run_step("markers", {"STUB_HTTP_FAIL_AT": "1"}, self.markers_state())
        self.assertEqual(done.returncode, 1)
        self.assertEqual(done.stderr.strip(), "CI: markers: the command ChangeOwnPassword of marker 1 failed.")
        self.assertEqual(len(re.findall(PASSWORD_CALL, log, re.M)), 1)
        self.assertNotIn("marker1.seq", state)

    def test_markers_stop_when_the_second_command_fails(self):
        done, log, state = self.run_step("markers", {"STUB_HTTP_FAIL_AT": "2"}, self.markers_state())
        self.assertEqual(done.returncode, 1)
        self.assertEqual(done.stderr.strip(), "CI: markers: the command ChangeOwnPassword of marker 2 failed.")
        self.assertNotIn("marker2.seq", state)
        self.assertNotIn("flush_lsn", log)

    def test_markers_stop_when_the_flush_check_never_returns_t(self):
        done, log, _ = self.run_step("markers", {"STUB_FLUSH": "f"}, self.markers_state())
        self.assertEqual(done.returncode, 1)
        self.assertEqual(done.stderr.strip(), "CI: markers: the flush check did not return t.")
        self.assertEqual(log.count("flush_lsn >="), 3)

    def test_markers_stop_when_the_marker_segment_has_no_partial_file(self):
        done, _, _ = self.run_step("markers", {"STUB_PARTIAL_STATUS": "1"}, self.markers_state())
        self.assertEqual(done.returncode, 1)
        self.assertEqual(done.stderr.strip(), "CI: markers: the marker segment has no .partial file.")

    def test_markers_stop_when_the_marker_segment_is_complete(self):
        done, _, _ = self.run_step("markers", {"STUB_COMPLETE_STATUS": "0"}, self.markers_state())
        self.assertEqual(done.returncode, 1)
        self.assertEqual(done.stderr.strip(), "CI: markers: the marker segment is complete, not partial.")

    # drill

    def drill_volume_names(self, log):
        return re.findall(r"^volume (?:create|rm) .*(recipe-editor_pgdata_drill_[0-9_]+)$", log, re.M)

    def test_drill_pass_path_runs_the_steps_in_order(self):
        done, log, _ = self.run_step("drill", state=self.drill_state())
        self.assertEqual(done.returncode, 0, done.stderr + log)
        self.assert_ordered(log, [
            r"^volume create --label recipe\.ci=recipe-ci-1-1 recipe-editor_pgdata_drill_[0-9_]+$",
            r"^compose --project-name recipe-ci-1-1 .* run --rm --no-deps -T -e PITR_MARKER_SEQ -e PITR_MARKER_HASH "
            r"-v recipe-editor_pgdata_drill_[0-9_]+:/var/lib/postgresql/data pitr-volume --target-time "
            + TARGET_TIME + r" --drill$",
            r"^PITR_MARKER_SEQ=41 PITR_MARKER_HASH=" + MARKER_1_HASH + "$",
            r"^run --rm --network none .* -v recipe-editor_pgdata_drill_[0-9_]+:/var/lib/postgresql/data:ro --entrypoint test ",
            r"^volume inspect --format .* recipe-editor_pgdata_drill_[0-9_]+$",
            r"^volume rm recipe-editor_pgdata_drill_[0-9_]+$",
        ])

    def test_drill_deletes_only_the_volume_that_it_created(self):
        done, log, state = self.run_step("drill", state=self.drill_state())
        self.assertEqual(done.returncode, 0, done.stderr + log)
        names = self.drill_volume_names(log)
        self.assertEqual(len(names), 2, log)
        self.assertEqual(names[0], names[1])
        self.assertEqual(len(re.findall(r"^volume rm ", log, re.M)), 1)
        self.assertNotIn("drill.volume", state)

    def test_drill_keeps_the_completed_line_for_the_switch_check(self):
        done, log, state = self.run_step("drill", state=self.drill_state())
        self.assertEqual(done.returncode, 0, done.stderr + log)
        self.assertEqual(state["pitr-volume.log"].strip().splitlines()[-1], DRILL_LINE)

    def test_drill_expects_a_refusal_of_both_switch_checks(self):
        done, log, _ = self.run_step("drill", state=self.drill_state())
        self.assertEqual(done.returncode, 0, done.stderr + log)
        self.assertIn("expect_switch_refusals", shell_function("drill"))
        self.assertIn("refuse_drill_volume", shell_function("expect_switch_refusals"))
        self.assertIn("read_pitr_line", shell_function("expect_switch_refusals"))
        self.assertIn("-e /var/lib/postgresql/data/pitr-drill", log)

    def test_drill_requires_the_state_of_markers(self):
        for missing in ("marker1.seq", "marker1.hash", "target.time"):
            with self.subTest(missing=missing):
                state = self.drill_state()
                del state[missing]
                done, log, _ = self.run_step("drill", state=state)
                self.assertEqual(done.returncode, 1)
                self.assertEqual(done.stderr.strip(), "CI: drill: run markers first.")
                self.assertEqual(log, "")

    def test_drill_refuses_a_state_value_outside_its_grammar(self):
        cases = {
            "marker1.seq": ("0\n", "CI: drill: the sequence number of marker 1 is invalid."),
            "marker1.hash": ("ABC\n", "CI: drill: the hash of marker 1 is invalid."),
            "target.time": ("2026-10-30 12:00:05\n", "CI: drill: the target time is invalid."),
        }
        for name, (value, message) in cases.items():
            with self.subTest(name=name):
                state = self.drill_state()
                state[name] = value
                done, log, _ = self.run_step("drill", state=state)
                self.assertEqual(done.returncode, 1)
                self.assertEqual(done.stderr.strip(), message)
                self.assertEqual(log, "")

    def assert_failed_drill_deleted_its_volume(self, done, log, message):
        self.assertEqual(done.returncode, 1, done.stderr + log)
        self.assertEqual(done.stderr.strip().splitlines()[0], message)
        names = self.drill_volume_names(log)
        self.assertEqual(len(names), 2, log)
        self.assertEqual(names[0], names[1])
        self.assertLess(log.index("volume create"), log.index("volume rm"))

    def test_drill_with_drill_false_fails_and_deletes_its_volume(self):
        line = DRILL_LINE.replace('"drill":true', '"drill":false')
        done, log, state = self.run_step("drill", {"STUB_PITR_OUTPUT": line}, self.drill_state())
        self.assert_failed_drill_deleted_its_volume(done, log, "CI: drill: the completed line of pitr-volume has drill false.")
        self.assertNotIn("--entrypoint test", log)
        self.assertNotIn("drill.volume", state)

    def test_drill_with_another_newest_entry_fails_and_deletes_its_volume(self):
        line = DRILL_LINE.replace('"chain_seq":41', '"chain_seq":42')
        done, log, _ = self.run_step("drill", {"STUB_PITR_OUTPUT": line}, self.drill_state())
        self.assert_failed_drill_deleted_its_volume(done, log, "CI: drill: the newest audit entry of the drill volume is not marker 1.")

    def test_drill_with_another_action_fails_and_deletes_its_volume(self):
        line = DRILL_LINE.replace('"action":"OwnPasswordChanged"', '"action":"UserLoggedIn"')
        done, log, _ = self.run_step("drill", {"STUB_PITR_OUTPUT": line}, self.drill_state())
        self.assert_failed_drill_deleted_its_volume(done, log, "CI: drill: the newest audit entry of the drill volume is not marker 1.")

    def test_drill_with_a_wrong_last_line_fails_and_deletes_its_volume(self):
        done, log, _ = self.run_step("drill", {"STUB_PITR_OUTPUT": '{"level":"INFO","message":"other"}'}, self.drill_state())
        self.assert_failed_drill_deleted_its_volume(done, log, "CI: drill: the last line of pitr-volume is wrong.")

    def test_drill_fails_when_pitr_volume_fails_and_deletes_its_volume(self):
        env = {"STUB_PITR_OUTPUT": '{"level":"ERROR","job":"pitr-volume","message":"x"}', "STUB_PITR_STATUS": "1"}
        done, log, _ = self.run_step("drill", env, self.drill_state())
        self.assert_failed_drill_deleted_its_volume(done, log, "CI: drill: pitr-volume failed.")
        self.assertNotIn("--entrypoint test", log)

    def test_drill_fails_when_the_switch_check_does_not_refuse(self):
        done, log, _ = self.run_step("drill", {"STUB_DRILL_TEST_STATUS": "1"}, self.drill_state())
        self.assert_failed_drill_deleted_its_volume(done, log, "CI: drill: the switch check accepted the drill volume.")

    def test_drill_fails_when_the_switch_check_cannot_run(self):
        done, log, _ = self.run_step("drill", {"STUB_DRILL_TEST_STATUS": "125"}, self.drill_state())
        self.assert_failed_drill_deleted_its_volume(done, log, "CI: drill: the switch check refused for another reason.")

    def test_drill_fails_when_the_deletion_fails_and_names_the_volume(self):
        done, _, _ = self.run_step("drill", {"STUB_VOLUME_RM_STATUS": "1"}, self.drill_state())
        self.assertEqual(done.returncode, 1)
        self.assertRegex(done.stderr, r"CI: drill: cannot delete the drill volume recipe-editor_pgdata_drill_[0-9_]+\.")

    def test_drill_does_not_delete_a_volume_without_the_label_of_the_job(self):
        done, log, _ = self.run_step("drill", {"STUB_LABEL": "other-project"}, self.drill_state())
        self.assertEqual(done.returncode, 1)
        self.assertRegex(done.stderr, r"CI: drill: the drill volume recipe-editor_pgdata_drill_[0-9_]+ has no label of this job\.")
        self.assertNotIn("volume rm", log)

    def test_drill_text_has_the_arguments_and_the_guards(self):
        body = shell_function("drill") + shell_function("delete_drill_volume") + shell_function("has_job_label")
        self.assertIn("--target-time", body)
        self.assertIn("--drill", body)
        self.assertIn("recipe.ci", body)
        self.assertNotIn("|| exit", body)
        self.assertNotRegex(body, r"volume prune|down\b.*(-v\b|--volumes)")
        self.assertNotIn("delete_ci_volumes", body)
        self.assertNotIn("RECIPE_PGDATA_VOLUME", shell_function("delete_drill_volume"))

    def test_drill_deletes_through_the_shared_function_of_common(self):
        drill = shell_function("delete_drill_volume")
        self.assertIn("delete_labelled_volume", drill)
        self.assertNotIn("docker volume rm", script_text())
        common = COMMON_FILE.read_text(encoding="utf-8")
        shared = shell_function("delete_labelled_volume")
        self.assertEqual(common.count("docker volume rm"), 1)
        self.assertLess(shared.index("has_job_label"), shared.index("docker volume rm"))
        self.assertEqual(common.count("\nhas_job_label() {"), 1)
        self.assertNotIn("\nhas_job_label() {", script_text())
        self.assertNotIn("fail ", shared)


    # refusals

    def run_refusals(self, env_extra=None, state=None):
        # The default listing of /wal holds the start segment 2, three complete segments, and the .partial file of segment 6.
        env = {"STUB_SEGMENTS": REFUSALS_SEGMENTS}
        env.update(env_extra or {})
        return self.run_step("refusals", env, self.refusals_state() if state is None else state)

    def run_refusals_with_policy(self, required, error, env=None):
        """Run a temporary script with the given proof policy."""
        text, count = re.subn(r"(?m)^EARLY_END_REFUSAL_REQUIRED=[01]$", f"EARLY_END_REFUSAL_REQUIRED={required}", script_text())
        self.assertEqual(count, 1)
        text, count = re.subn(r"(?m)^EARLY_END_ERROR=.*$", f'EARLY_END_ERROR="{error}"', text)
        self.assertEqual(count, 1)
        with tempfile.TemporaryDirectory() as name:
            directory = pathlib.Path(name)
            for source in (COMMON_FILE, STEPS_FILE):
                shutil.copyfile(source, directory / source.name)
            candidate = directory / SCRIPT.name
            candidate.write_text(text, encoding="utf-8")
            with patch.dict(globals(), SCRIPT=candidate):
                return self.run_refusals(env)

    def test_refusals_pass_path_runs_the_steps_in_order(self):
        done, log, _ = self.run_refusals()
        self.assertEqual(done.returncode, 0, done.stderr + log)
        volume = r"recipe-editor_pgdata_drill_[0-9_]+"
        real = r".*/walarchive-disk/walarchive"
        self.assert_ordered(log, [
            r"^compose --project-name recipe-ci-1-1 .* ps -aq db$",
            r"^volume ls -q --filter label=recipe\.ci=recipe-ci-1-1$",
            r"interval '1 hour'",
            rf"^volume create --label recipe\.ci=recipe-ci-1-1 {volume}$",
            rf"^compose --project-name recipe-ci-1-1 .* run --rm --no-deps -T -v {volume}:/var/lib/postgresql/data "
            rf"pitr-volume --target-time {TARGET_TIME}$",
            rf"^volume inspect --format .* {volume}$",
            rf"^volume rm {volume}$",
            rf"^sudo find {real}/base ",
            rf"^sudo sed {real}/base/base_20261030_115900/backup_label$",
            rf"^sudo find {real} -maxdepth 1 -type f ",
            r"^sudo test -e .*/state/wal-copy$",
            r"^sudo install -d .*/state/wal-copy$",
            r"^sudo cp -a -- .*/walarchive-disk/walarchive/\. .*/state/wal-copy/$",
            r"^sudo find .*/state/wal-copy/base ",
            r"^sudo sed .*/state/wal-copy/base/base_20261030_115900/backup_label$",
            r"^sudo find .*/state/wal-copy -maxdepth 1 -type f ",
            r"^sudo rm -- .*/state/wal-copy/000000010000000000000003$",
            rf"^volume create --label recipe\.ci=recipe-ci-1-1 {volume}$",
            rf"^compose --project-name recipe-ci-1-1 .* run --rm --no-deps -T "
            rf"-v {volume}:/var/lib/postgresql/data -v .*/state/wal-copy:/wal:ro pitr-volume$",
            rf"^volume inspect --format .* {volume}$",
            rf"^volume rm {volume}$",
            r"^sudo rm -rf -- .*/state/wal-copy$",
            rf"^sudo cp -a -- {real}/\. .*/state/wal-copy/$",
            rf"^sudo test -f .*/state/wal-copy/{EARLY_END_SEGMENT}\.partial$",
            rf"^sudo test -e .*/state/wal-copy/{EARLY_END_SEGMENT}$",
            r"^sudo od ",
            r"^sudo dd ",
            r"^sudo od ",
            rf"^volume create --label recipe\.ci=recipe-ci-1-1 {volume}$",
            rf"^compose --project-name recipe-ci-1-1 .* run --rm --no-deps -T "
            rf"-v {volume}:/var/lib/postgresql/data -v .*/state/wal-copy:/wal:ro pitr-volume$",
            rf"^volume inspect --format .* {volume}$",
            rf"^volume rm {volume}$",
            r"^sudo rm -rf -- .*/state/wal-copy$",
            r"^compose --project-name recipe-ci-1-1 .* ps -aq db$",
            r"^volume ls -q --filter label=recipe\.ci=recipe-ci-1-1$",
        ])

    def test_refusals_run_the_early_end_run_on_a_damaged_copy(self):
        done, log, _ = self.run_refusals()
        self.assertEqual(done.returncode, 0, done.stderr + log)
        partial = rf"\S*/state/wal-copy/{EARLY_END_SEGMENT}\.partial"
        self.assert_ordered(log, [
            r"^sudo rm -rf -- .*/state/wal-copy$",
            r"^sudo cp -a -- .*/walarchive-disk/walarchive/\. .*/state/wal-copy/$",
            rf"^sudo od .* -j 116 .*{partial}$",
            rf"^sudo dd (?=.*\bof={partial} )(?=.*\bseek=116\b).*\bconv=notrunc$",
            rf"^sudo od .* -j 116 .*{partial}$",
            r"^volume create ",
            r"^compose .* -v \S*/state/wal-copy:/wal:ro pitr-volume$",
            r"^volume rm ",
            r"^sudo rm -rf -- .*/state/wal-copy$",
        ])
        self.assertEqual(len(re.findall(r"^sudo dd ", log, re.M)), 1)
        self.assertNotIn("PITR_EXPECTED_LSN", log)
        self.assertNotIn("PITR_MARKER_", log)
        self.assertIn("refusals: early end: byte_new=90", done.stdout.splitlines())

    def test_refusals_stream_the_early_end_output(self):
        done, log, _ = self.run_refusals()
        self.assertEqual(done.returncode, 0, done.stderr + log)
        self.assertEqual(done.stdout.splitlines().count(EARLY_END_LINE), 1)
        body = shell_function("run_early_end")
        self.assertIn('| tee "$log_file"', body)
        self.assertNotIn('cat "$log_file"', body)

    def test_refusals_invert_the_byte_by_its_value(self):
        for offset, old, new in ((96, 165, 90), (8176, 18, 237)):
            with self.subTest(offset=offset):
                state = self.refusals_state()
                state["early-end.state"] = f"0/{0x6000000 + offset:X}|{EARLY_END_SEGMENT}|{offset}|16777216|8192\n"
                done, log, _ = self.run_refusals(state=state)
                self.assertEqual(done.returncode, 0, done.stderr + log)
                lines = done.stdout.splitlines()
                self.assertIn(f"refusals: early end: byte_old={old}", lines)
                self.assertIn(f"refusals: early end: byte_new={new}", lines)

    def test_refusals_choose_the_byte_by_the_offset_in_the_page(self):
        for offset, seek in ((859104, 859124), (8168, 8188), (8169, 8169), (8176, 8176)):
            with self.subTest(offset=offset):
                state = self.refusals_state()
                state["early-end.state"] = f"0/{0x6000000 + offset:X}|{EARLY_END_SEGMENT}|{offset}|16777216|8192\n"
                done, log, _ = self.run_refusals(state=state)
                self.assertEqual(done.returncode, 0, done.stderr + log)
                self.assertRegex(log, rf"(?m)^sudo dd .*\bseek={seek}\b")

    def test_refusals_fail_when_the_byte_does_not_change(self):
        done, _, _ = self.run_refusals({"STUB_EARLY_END_SKIP_DD": "1"})
        self.assertEqual(done.returncode, 1, done.stderr)
        self.assertIn("CI: refusals: the byte did not change to its inverse.", done.stderr.splitlines())
        self.assertIn("refusals: early end: class=I", done.stdout.splitlines())

    def test_refusals_log_each_proof_value(self):
        done, log, _ = self.run_refusals()
        self.assertEqual(done.returncode, 0, done.stderr + log)
        expected = {"lsn": "0/6000060", "segment": EARLY_END_SEGMENT, "offset": "96", "wal_segment_size": "16777216",
                    "wal_block_size": "8192", "byte_offset": "116", "byte_old": "165", "byte_new": "90",
                    "marker1_seq": "41", "marker2_seq": "42", "exit_code": "0", "error_line": "none",
                    "last_line": EARLY_END_LINE, "class": "S"}
        prefix = "refusals: early end: "
        self.assertCountEqual([line for line in done.stdout.splitlines() if line.startswith(prefix)],
                              [f"{prefix}{name}={value}" for name, value in expected.items()])

    def test_refusals_log_the_proof_after_a_failed_command(self):
        done, _, _ = self.run_refusals({"STUB_EARLY_END_NO_BASE": "1"})
        self.assertEqual(done.returncode, 1, done.stderr)
        self.assertRegex(done.stderr, r"(?m)^CI: refusals: .*/wal-copy holds no base backup\.$")
        lines = done.stdout.splitlines()
        self.assertIn("refusals: early end: lsn=0/6000060", lines, done.stderr)
        self.assertIn("refusals: early end: class=I", lines, done.stderr)
        self.assertNotIn("unbound variable", done.stderr)

    def test_refusals_class_a_precondition_refusal_as_inconclusive(self):
        line = (
            '{"timestamp":"2026-10-30T12:00:07Z","level":"ERROR","job":"pitr-volume",'
            '"message":"the target directory is not empty"}'
        )
        done, _, _ = self.run_refusals({"STUB_EARLY_END_STATUS": "1", "STUB_EARLY_END_OUTPUT": line})
        self.assertEqual(done.returncode, 1, done.stdout + done.stderr)
        self.assertIn("refusals: early end: class=I", done.stdout.splitlines())

    def test_refusals_accept_a_refusal_of_the_early_end_run(self):
        for line in (EARLY_END_ERROR_LINE, EARLY_END_ERROR_LINE[:-1] + ',"exit_code":7}'):
            with self.subTest(line=line):
                env = {"STUB_EARLY_END_STATUS": "7", "STUB_EARLY_END_OUTPUT": f"{EARLY_END_HASHES_LINE}\n{line}"}
                done, log, _ = self.run_refusals(env)
                self.assertEqual(done.returncode, 0, done.stderr + log)
                for proof in ("class=R", "exit_code=7", f"error_line={line}", f"last_line={line}"):
                    self.assertIn(f"refusals: early end: {proof}", done.stdout.splitlines())
        done, log, _ = self.run_refusals_with_policy(1, "the newest audit entry is invalid", env)
        self.assertEqual(done.returncode, 0, done.stderr + log)
        done, log, _ = self.run_refusals_with_policy(1, "another error", env)
        self.assertEqual(done.returncode, 1, done.stdout + log)
        self.assertIn("refusals: early end: class=R", done.stdout.splitlines())

    def test_refusals_accept_a_completion_at_marker_1_only_when_not_required(self):
        for required in (0, 1):
            with self.subTest(required=required):
                done, log, _ = self.run_refusals_with_policy(required, "")
                self.assertEqual(done.returncode, required, done.stderr + log)
                self.assertIn("refusals: early end: class=S", done.stdout.splitlines())

    def test_refusals_class_a_line_of_another_job_as_inconclusive(self):
        cases = (
            (7, EARLY_END_HASHES_LINE + "\n", EARLY_END_ERROR_LINE),
            (0, "", EARLY_END_LINE),
        )
        for status, prefix, line in cases:
            with self.subTest(status=status):
                line = line.replace('"job":"pitr-volume"', '"job":"walarchive-entry"')
                env = {"STUB_EARLY_END_STATUS": str(status), "STUB_EARLY_END_OUTPUT": prefix + line}
                done, _, _ = self.run_refusals(env)
                self.assertEqual(done.returncode, 1, done.stdout + done.stderr)
                self.assertIn("refusals: early end: class=I", done.stdout.splitlines())

    def test_refusals_fail_on_an_inconclusive_early_end_run(self):
        for status, output in ((0, RECOVER_LINE), (0, EARLY_END_LINE.replace("OwnPasswordChanged", "Login")),
                               (1, ""), (1, f"{EARLY_END_HASHES_LINE}\n{EARLY_END_ERROR_LINE}\n{EARLY_END_ERROR_LINE}"),
                               (1, f"{EARLY_END_HASHES_LINE}\n{EARLY_END_ERROR_LINE}\n{EARLY_END_LINE}")):
            with self.subTest(status=status, output=output):
                done, log, state = self.run_refusals({"STUB_EARLY_END_STATUS": str(status), "STUB_EARLY_END_OUTPUT": output})
                self.assertEqual(done.returncode, 1, done.stdout + log)
                self.assertIn("refusals: early end: class=I", done.stdout.splitlines())
                self.assertNotIn("drill.volume", state)
                self.assertNotIn("refusals.wal-copy", state)
        env = {
            "STUB_EARLY_END_STATUS": "0",
            "STUB_EARLY_END_OUTPUT": f"{EARLY_END_ERROR_LINE}\n{EARLY_END_ERROR_LINE}\n{EARLY_END_LINE}",
        }
        for required in (0, 1):
            with self.subTest(required=required):
                done, _, _ = self.run_refusals_with_policy(required, "", env)
                self.assertEqual(done.returncode, 1, done.stdout + done.stderr)
                self.assertIn("refusals: early end: class=I", done.stdout.splitlines())
        for env in ({"STUB_EARLY_END_SEGMENT_MISSING": "1"},
                    {"STUB_SEGMENTS": REFUSALS_SEGMENTS + "000000010000000000000007.partial\n"}):
            with self.subTest(env=env):
                done, log, state = self.run_refusals(env)
                self.assertEqual(done.returncode, 1, done.stdout + log)
                self.assertIn("refusals: early end: class=I", done.stdout.splitlines())
                self.assertIn("refusals: early end: exit_code=none", done.stdout.splitlines())
                self.assertNotIn("sudo dd ", log)
                self.assertNotIn("refusals.wal-copy", state)

    def test_refusals_refuse_a_complete_early_end_segment(self):
        done, _, _ = self.run_refusals({"STUB_EARLY_END_SEGMENT_COMPLETE": "1"})
        self.assertEqual(done.returncode, 1, done.stderr)
        self.assertIn("CI: refusals: the segment for the early end is complete.", done.stderr.splitlines())
        self.assertIn("refusals: early end: class=I", done.stdout.splitlines())

    def test_refusals_write_no_wal_and_switch_no_segment(self):
        # Marker 2 must stay in the .partial file until crash. The step prepare ended the gap segments.
        done, log, _ = self.run_refusals()
        self.assertEqual(done.returncode, 0, done.stderr + log)
        self.assertNotIn("pg_switch_wal", log)
        self.assertNotIn("pg_logical_emit_message", log)

    def test_refusals_delete_only_a_middle_complete_segment(self):
        done, log, _ = self.run_refusals()
        self.assertEqual(done.returncode, 0, done.stderr + log)
        removed = re.findall(r"^sudo rm -- (\S+)$", log, re.M)
        self.assertEqual(len(removed), 1, removed)
        self.assertTrue(removed[0].endswith("/state/wal-copy/000000010000000000000003"), removed)
        for kept in ("000000010000000000000002", "000000010000000000000005", "000000010000000000000006.partial"):
            self.assertFalse(removed[0].endswith(kept), removed)
        self.assertNotIn(".partial", removed[0])

    def test_refusals_keep_the_newest_complete_segment_of_the_timeline(self):
        # A complete segment of another timeline is not the newest one of the timeline of the base backup.
        copy = ("000000010000000000000002\n000000010000000000000003\n000000010000000000000004\n"
                "000000020000000000000009\n000000010000000000000005.partial\n")
        done, log, _ = self.run_refusals({"STUB_COPY_SEGMENTS": copy})
        self.assertEqual(done.returncode, 0, done.stderr + log)
        removed = re.findall(r"^sudo rm -- (\S+)$", log, re.M)
        self.assertEqual(len(removed), 1, removed)
        self.assertTrue(removed[0].endswith("/state/wal-copy/000000010000000000000003"), removed)

    def test_refusals_fail_when_the_archive_never_holds_two_complete_segments(self):
        # One complete segment after the start segment and the .partial file are not enough.
        segments = "000000010000000000000002\n000000010000000000000003\n000000010000000000000004.partial\n"
        done, log, _ = self.run_refusals({"STUB_SEGMENTS": segments})
        self.assert_failed_refusals_cleaned_up(
            done, log, "CI: refusals: the archive holds fewer than two complete segments after the start segment.")
        self.assertEqual(len(re.findall(r"^sudo find \S*/walarchive-disk/walarchive -maxdepth 1 -type f ", log, re.M)), 3)
        self.assertNotIn("sudo cp ", log)
        self.assertNotIn("sudo rm -- ", log)

    def test_refusals_fail_when_a_partial_file_is_the_only_segment_after_the_start(self):
        segments = "000000010000000000000002\n000000010000000000000003.partial\n"
        done, log, _ = self.run_refusals({"STUB_SEGMENTS": segments})
        self.assert_failed_refusals_cleaned_up(
            done, log, "CI: refusals: the archive holds fewer than two complete segments after the start segment.")

    def test_refusals_fail_when_the_copy_has_no_middle_segment(self):
        copy = "000000010000000000000002\n000000010000000000000003\n000000010000000000000004.partial\n"
        done, log, _ = self.run_refusals({"STUB_COPY_SEGMENTS": copy})
        self.assert_failed_refusals_cleaned_up(
            done, log, "CI: refusals: the copy of /wal has no complete segment between the start segment and the newest one.")
        self.assertNotIn("sudo rm -- ", log)
        self.assertRegex(log, r"(?m)^sudo rm -rf -- .*/state/wal-copy$")

    def test_refusals_delete_each_scratch_volume_and_the_copy(self):
        done, log, state = self.run_refusals()
        self.assertEqual(done.returncode, 0, done.stderr + log)
        self.assertEqual(len(re.findall(r"^volume create ", log, re.M)), 3)
        self.assertEqual(len(re.findall(r"^volume rm ", log, re.M)), 3)
        self.assertEqual(len(re.findall(r"^sudo rm -rf -- ", log, re.M)), 2)
        self.assertNotIn("drill.volume", state)
        self.assertNotIn("refusals.wal-copy", state)

    def test_refusals_never_write_to_the_real_wal_archive(self):
        done, log, _ = self.run_refusals()
        self.assertEqual(done.returncode, 0, done.stderr + log)
        real = "/walarchive-disk/walarchive"
        for line in log.splitlines():
            if re.match(r"^sudo (rm|install|mv|chmod|chown|touch|tee|truncate|ln|link)\b", line):
                self.assertNotIn(real, line)
            if line.startswith("sudo dd "):
                self.assertNotIn(real, line)
            if line.startswith("sudo cp "):
                self.assertTrue(line.endswith("/state/wal-copy/"), line)
        self.assertEqual(len(re.findall(r"^sudo rm -- ", log, re.M)), 1)
        self.assertIn(":/wal:ro pitr-volume", log)

    def test_refusals_never_link_the_wal_copy_to_the_real_archive(self):
        done, log, _ = self.run_refusals()
        self.assertEqual(done.returncode, 0, done.stderr + log)
        for line in log.splitlines():
            self.assertNotRegex(line, r"^sudo ln(?:\s|$)")
            if line.startswith("sudo cp "):
                options = line.partition(" -- ")[0]
                self.assertNotRegex(options, r"\s(?:--link|-[a-zA-Z]*l[a-zA-Z]*)(?:\s|$)", line)

    def test_refusals_run_the_gap_run_without_the_expected_position(self):
        # The check of the newest segment alone must refuse the copy, so no run gets PITR_EXPECTED_LSN.
        done, log, _ = self.run_refusals()
        self.assertEqual(done.returncode, 0, done.stderr + log)
        self.assertNotIn("PITR_EXPECTED_LSN", log)
        self.assertNotIn("pg_current_wal_lsn", log)
        self.assertNotIn("PITR_MARKER", log)
        self.assertNotIn("--drill", log)
        self.assertNotIn("PITR_EXPECTED_LSN", shell_function("refusals"))

    def test_refusals_require_the_state_of_markers(self):
        done, log, _ = self.run_refusals(state={})
        self.assertEqual(done.returncode, 1)
        self.assertEqual(done.stderr.strip(), "CI: refusals: run markers first.")
        self.assertEqual(log, "")

    def assert_failed_refusals_cleaned_up(self, done, log, message, volumes=1):
        self.assertEqual(done.returncode, 1, done.stderr + log)
        self.assertEqual(done.stderr.strip().splitlines()[0], message)
        self.assertEqual(len(re.findall(r"^volume create ", log, re.M)), volumes, log)
        self.assertEqual(len(re.findall(r"^volume rm ", log, re.M)), volumes, log)

    def test_refusals_fail_when_the_target_time_run_succeeds(self):
        done, log, _ = self.run_refusals({"STUB_TARGET_STATUS": "0"})
        self.assert_failed_refusals_cleaned_up(
            done, log, "CI: refusals: pitr-volume accepted the target time after the newest commit.")
        self.assertNotIn("wal-copy", log)

    def test_refusals_fail_when_the_gap_run_succeeds(self):
        done, log, _ = self.run_refusals({"STUB_GAP_STATUS": "0"})
        self.assert_failed_refusals_cleaned_up(
            done, log, "CI: refusals: pitr-volume accepted the copy of /wal with a missing segment.", volumes=2)
        self.assertRegex(log, r"(?m)^sudo rm -rf -- .*/state/wal-copy$")

    def test_refusals_fail_on_a_wrong_error_text_of_the_target_time_run(self):
        line = TARGET_ERROR_LINE.replace("did not finish the recovery", "failed")
        done, log, _ = self.run_refusals({"STUB_TARGET_OUTPUT": line})
        self.assert_failed_refusals_cleaned_up(
            done, log, "CI: refusals: the ERROR line of pitr-volume is wrong for the target time after the newest commit.")

    def test_refusals_fail_on_a_wrong_error_text_of_the_gap_run(self):
        line = GAP_ERROR_LINE.replace("ended before the end of the WAL archive", "ended")
        done, log, _ = self.run_refusals({"STUB_GAP_OUTPUT": line})
        self.assert_failed_refusals_cleaned_up(
            done, log, "CI: refusals: the ERROR line of pitr-volume is wrong for the copy of /wal with a missing segment.", volumes=2)
        self.assertRegex(log, r"(?m)^sudo rm -rf -- .*/state/wal-copy$")

    def test_refusals_fail_when_the_error_line_is_not_the_only_one(self):
        done, log, _ = self.run_refusals({"STUB_TARGET_OUTPUT": TARGET_ERROR_LINE + "\n" + TARGET_ERROR_LINE})
        self.assert_failed_refusals_cleaned_up(
            done, log, "CI: refusals: the ERROR line of pitr-volume is wrong for the target time after the newest commit.")

    def test_refusals_fail_when_pitr_volume_writes_no_error_line(self):
        done, log, _ = self.run_refusals({"STUB_GAP_OUTPUT": ""})
        self.assert_failed_refusals_cleaned_up(
            done, log, "CI: refusals: the ERROR line of pitr-volume is wrong for the copy of /wal with a missing segment.", volumes=2)

    def test_refusals_fail_when_the_copy_directory_exists(self):
        done, log, _ = self.run_refusals({"STUB_COMPLETE_STATUS": "0"})
        self.assertEqual(done.returncode, 1)
        self.assertEqual(done.stderr.strip(), "CI: refusals: the directory of the WAL copy exists.")
        self.assertNotIn("sudo cp", log)
        self.assertNotIn("sudo rm", log)

    def test_refusals_detect_a_change_of_the_production_volume(self):
        done, log, _ = self.run_refusals({"STUB_PROD_CHANGE": "volume"})
        self.assertEqual(done.returncode, 1, done.stderr + log)
        self.assertEqual(done.stderr.strip(), "CI: refusals: a production fact changed during the step.")

    def test_refusals_detect_a_restart_of_the_database_container(self):
        done, log, _ = self.run_refusals({"STUB_PROD_CHANGE": "container"})
        self.assertEqual(done.returncode, 1, done.stderr + log)
        self.assertEqual(done.stderr.strip(), "CI: refusals: a production fact changed during the step.")

    def test_refusals_detect_a_scratch_volume_that_stays(self):
        done, log, _ = self.run_refusals({"STUB_VOLUME_STAYS": "1"})
        self.assertEqual(done.returncode, 1, done.stderr + log)
        self.assertEqual(done.stderr.strip(), "CI: refusals: a production fact changed during the step.")

    def test_refusals_fail_when_the_deletion_of_a_volume_fails(self):
        done, _, _ = self.run_refusals({"STUB_VOLUME_RM_STATUS": "1"})
        self.assertEqual(done.returncode, 1)
        self.assertRegex(done.stderr, r"CI: drill: cannot delete the drill volume recipe-editor_pgdata_drill_[0-9_]+\.")

    def test_refusals_text_has_no_new_volume_deletion_and_no_exit_idiom(self):
        body = shell_function("refusals") + shell_function("expect_pitr_error") + shell_function("refusals_exit")
        self.assertNotIn("docker volume rm", body)
        self.assertNotIn("|| exit", body)
        self.assertIn("delete_drill_volume", body)
        self.assertNotRegex(body, r"volume prune|down\b.*(-v\b|--volumes)")
        self.assertNotIn("pg_switch_wal", body)
        self.assertNotIn("switch_wal_segments", body)
        self.assertIn("wait_gap_segments", body)
        self.assertEqual(script_text().count("docker volume rm"), 0)


    # crash

    def crash_state(self):
        # The step markers leaves the identity of marker 2.
        return self.refusals_state()

    def test_crash_pass_path_runs_the_steps_in_order(self):
        done, log, files = self.run_step("crash", state=self.crash_state())
        self.assertEqual(done.returncode, 0, done.stderr + log)
        self.assertEqual(docker_calls.forbidden_window_calls(files["argv"]), [])
        self.assertEqual(self.walarchive_window_hits("crash", files["argv"]), [])
        self.assert_ordered(log, [
            r"^compose --project-name recipe-ci-1-1 .* ps -aq db$",
            r"^inspect --format \{\{range \.Mounts\}\}",
            r"^volume inspect --format .* " + OLD_VOLUME + "$",
            r"^compose --project-name recipe-ci-1-1 .* rm -s -f app backup$",
            r"SELECT pg_current_wal_lsn\(\)",
            r"flush_lsn >=",
            r"pg_walfile_name",
            r"^sudo test -f .*000000010000000000000003\.partial$",
            r"^sudo test -e .*000000010000000000000003$",
            r"^compose --project-name recipe-ci-1-1 .* kill -s SIGKILL db$",
            r"^inspect --format \{\{\.State\.Running\}\} \{\{\.State\.ExitCode\}\} ",
            r"^compose --project-name recipe-ci-1-1 .* rm -s -f walarchive$",
            r"^run --rm --network none .* -v " + OLD_VOLUME + r":/var/lib/postgresql/data --entrypoint rm ",
        ])

    def test_crash_saves_the_position_and_the_start_time(self):
        before = int(time.time())
        done, log, state = self.run_step("crash", state=self.crash_state())
        after = int(time.time())
        self.assertEqual(done.returncode, 0, done.stderr + log)
        self.assertEqual(state["expected.lsn"], EXPECTED_LSN + "\n")
        self.assertRegex(state["recovery.start"], r"^[0-9]{10,}\n$")
        self.assertTrue(before <= int(state["recovery.start"]) <= after, state["recovery.start"])
        self.assertIn("CI: crash: the recovery time starts at " + state["recovery.start"].strip(), done.stdout + done.stderr)

    def test_crash_deletes_only_global_pg_control_of_the_old_volume(self):
        done, log, _ = self.run_step("crash", state=self.crash_state())
        self.assertEqual(done.returncode, 0, done.stderr + log)
        deletions = re.findall(r"^run .*--entrypoint rm .*$", log, re.M)
        self.assertEqual(len(deletions), 1, log)
        self.assertEqual(deletions[0], "run --rm --network none --security-opt no-new-privileges:true "
                         "-v " + OLD_VOLUME + ":/var/lib/postgresql/data --entrypoint rm "
                         "recipe-pgtools:ci-pitr-recovery /var/lib/postgresql/data/global/pg_control")
        self.assertNotIn("volume rm", log)

    def test_crash_stops_the_writers_before_it_reads_the_position(self):
        done, log, _ = self.run_step("crash", state=self.crash_state())
        self.assertEqual(done.returncode, 0, done.stderr + log)
        self.assertLess(log.index("rm -s -f app backup"), log.index("pg_current_wal_lsn"))
        self.assertLess(log.index("kill -s SIGKILL db"), log.index("rm -s -f walarchive"))
        self.assertNotIn("pg_switch_wal", log)

    def test_crash_has_no_stop_of_walarchive(self):
        self.assertNotIn("stop walarchive", shell_function("crash"))

    def test_crash_reads_the_clock_once_before_it_stops_the_writers(self):
        done, log, _ = self.run_step("crash", state=self.crash_state())
        self.assertEqual(done.returncode, 0, done.stderr + log)
        self.assertEqual(log.count("clock +%s"), 1, log)
        self.assertLess(log.index("clock +%s"), log.index("rm -s -f app backup"))

    def test_crash_requires_the_state_of_markers(self):
        for missing in ("marker2.seq", "marker2.hash"):
            with self.subTest(missing=missing):
                state = self.crash_state()
                del state[missing]
                done, log, _ = self.run_step("crash", state=state)
                self.assertEqual(done.returncode, 1)
                self.assertEqual(done.stderr.strip(), "CI: crash: run markers first.")
                self.assertEqual(log, "")

    def assert_stopped_before(self, done, log, message, marker):
        """The step failed with this first line and never reached the command with this text."""
        self.assertEqual(done.returncode, 1, done.stderr + log)
        self.assertEqual(done.stderr.strip().splitlines()[0], message)
        self.assertNotIn(marker, log)

    def test_crash_fails_when_the_marker_segment_is_complete(self):
        done, log, state = self.run_step("crash", {"STUB_COMPLETE_STATUS": "0"}, self.crash_state())
        self.assert_stopped_before(done, log, "CI: crash: the marker segment is complete, not partial.", "kill ")
        self.assertNotIn("recovery.start", state)

    def test_crash_fails_when_the_segment_has_no_partial_file(self):
        done, log, _ = self.run_step("crash", {"STUB_PARTIAL_STATUS": "1"}, self.crash_state())
        self.assert_stopped_before(done, log, "CI: crash: the marker segment has no .partial file.", "kill ")

    def test_crash_fails_when_the_flush_check_never_returns_t(self):
        done, log, _ = self.run_step("crash", {"STUB_FLUSH": "f"}, self.crash_state())
        self.assert_stopped_before(done, log, "CI: crash: the flush check did not return t.", "kill ")
        self.assertEqual(log.count("flush_lsn >="), 3)

    def test_crash_fails_when_the_wal_position_is_malformed(self):
        done, log, state = self.run_step("crash", {"STUB_LSN": "bad"}, self.crash_state())
        self.assert_stopped_before(done, log, "CI: crash: the WAL position is invalid.", "kill ")
        self.assertNotIn("flush_lsn", log)
        self.assertNotIn("expected.lsn", state)

    def test_crash_fails_when_the_wal_segment_name_is_malformed(self):
        done, log, _ = self.run_step("crash", {"STUB_SEGMENT": "bad"}, self.crash_state())
        self.assert_stopped_before(done, log, "CI: crash: the WAL segment name is invalid.", "kill ")

    def test_crash_fails_when_db_does_not_use_the_old_volume(self):
        done, log, _ = self.run_step("crash", {"STUB_DB_VOLUME": "other_volume"}, self.crash_state())
        self.assert_stopped_before(done, log, "CI: crash: db does not use the volume " + OLD_VOLUME + ".", "rm -s -f app backup")

    def test_crash_fails_when_the_old_volume_has_no_label_of_the_job(self):
        done, log, _ = self.run_step("crash", {"STUB_LABEL": "other-project"}, self.crash_state())
        self.assert_stopped_before(
            done, log, "CI: crash: the old volume " + OLD_VOLUME + " has no label of this job.", "rm -s -f app backup")

    def test_crash_fails_when_the_services_do_not_stop(self):
        done, log, _ = self.run_step("crash", {"STUB_REMOVE_APP_STATUS": "1"}, self.crash_state())
        self.assert_stopped_before(done, log, "CI: crash: cannot delete app and backup.", "pg_current_wal_lsn")

    def test_crash_fails_when_db_is_already_dead_at_kill_time(self):
        done, log, state = self.run_step("crash", {"STUB_KILL_STATUS": "1"}, self.crash_state())
        self.assert_stopped_before(done, log, "CI: crash: cannot kill db with SIGKILL.", "--entrypoint rm")
        self.assertNotIn("rm -s -f walarchive", log)
        self.assertNotIn("recovery.start", state)

    def test_crash_fails_when_db_does_not_end_with_sigkill(self):
        for name, value in (("running", "true 0"), ("other exit code", "false 0")):
            with self.subTest(name=name):
                done, log, _ = self.run_step("crash", {"STUB_DB_STATE": value}, self.crash_state())
                self.assert_stopped_before(done, log, "CI: crash: db did not end with SIGKILL.", "--entrypoint rm")
                self.assertEqual(len(re.findall(r"^inspect --format \{\{\.State\.Running\}\}", log, re.M)), 3)

    def test_crash_fails_when_walarchive_cannot_be_deleted(self):
        done, log, _ = self.run_step("crash", {"STUB_REMOVE_WALARCHIVE_STATUS": "1"}, self.crash_state())
        self.assert_stopped_before(done, log, "CI: crash: cannot delete walarchive.", "--entrypoint rm")

    def test_crash_fails_when_the_deletion_of_pg_control_fails(self):
        done, log, state = self.run_step("crash", {"STUB_RM_STATUS": "1"}, self.crash_state())
        self.assertEqual(done.returncode, 1, done.stderr + log)
        self.assertEqual(done.stderr.strip(), "CI: crash: cannot delete global/pg_control of the old volume.")
        self.assertIn("rm -s -f walarchive", log)
        self.assertNotIn("recovery.start", state)

    def test_crash_text_has_one_deletion_and_no_exit_idiom(self):
        body = shell_function("crash")
        self.assertNotIn("|| exit", body)
        self.assertNotIn("docker volume rm", body)
        self.assertNotRegex(body, r"volume prune|down\b.*(-v\b|--volumes)")
        self.assertEqual(body.count("--entrypoint rm"), 1)
        self.assertEqual(script_text().count("docker volume rm"), 0)

    # recover

    def recover_state(self):
        # The steps markers and crash leave this state.
        state = self.refusals_state()
        state.update({"expected.lsn": EXPECTED_LSN + "\n", "recovery.start": "1790000000\n"})
        return state

    def test_recover_pass_path_runs_the_steps_in_order(self):
        done, log, files = self.run_step("recover", state=self.recover_state())
        self.assertEqual(done.returncode, 0, done.stderr + log)
        self.assertEqual(docker_calls.forbidden_window_calls(files["argv"]), [])
        self.assertEqual(self.walarchive_window_hits("recover", files["argv"]), [])
        self.assert_ordered(log, [
            rf"^volume create --label recipe\.ci=recipe-ci-1-1 {NEW_VOLUME}$",
            rf"^compose --project-name recipe-ci-1-1 .* run --rm --no-deps -T -e PITR_MARKER_SEQ -e PITR_MARKER_HASH "
            rf"-e PITR_EXPECTED_LSN={EXPECTED_LSN} -v {NEW_VOLUME}:/var/lib/postgresql/data pitr-volume$",
            r"^PITR_MARKER_SEQ=42 PITR_MARKER_HASH=" + MARKER_2_HASH + "$",
        ])

    def test_recover_saves_the_new_volume_and_the_completed_line(self):
        done, log, state = self.run_step("recover", state=self.recover_state())
        self.assertEqual(done.returncode, 0, done.stderr + log)
        name = state["new.volume"].strip()
        self.assertRegex(name, rf"^{NEW_VOLUME}$")
        self.assertIn(f"volume create --label recipe.ci=recipe-ci-1-1 {name}\n", log)
        self.assertEqual(state["pitr-volume.log"].strip().splitlines()[-1], RECOVER_LINE)
        self.assertNotIn("drill.volume", state)

    def test_recover_runs_to_the_end_without_target_time_or_drill(self):
        done, log, _ = self.run_step("recover", state=self.recover_state())
        self.assertEqual(done.returncode, 0, done.stderr + log)
        self.assertNotIn("--target-time", log)
        self.assertNotIn("--drill", log)
        self.assertNotIn("volume rm", log)
        self.assertNotIn("PITR_MARKER_SEQ=41", log)

    def test_recover_requires_the_state_of_the_earlier_steps(self):
        cases = {
            "marker2.seq": "CI: recover: run markers first.",
            "marker2.hash": "CI: recover: run markers first.",
            "expected.lsn": "CI: recover: run crash first.",
            "recovery.start": "CI: recover: run crash first.",
        }
        for missing, message in cases.items():
            with self.subTest(missing=missing):
                state = self.recover_state()
                del state[missing]
                done, log, _ = self.run_step("recover", state=state)
                self.assertEqual(done.returncode, 1)
                self.assertEqual(done.stderr.strip(), message)
                self.assertEqual(log, "")

    def test_recover_refuses_a_state_value_outside_its_grammar(self):
        cases = {
            "marker2.seq": ("0\n", "CI: recover: the sequence number of marker 2 is invalid."),
            "marker2.hash": ("ABC\n", "CI: recover: the hash of marker 2 is invalid."),
            "expected.lsn": ("bad\n", "CI: recover: the expected WAL position is invalid."),
            "recovery.start": ("soon\n", "CI: recover: the start time of the recovery is invalid."),
        }
        for name, (value, message) in cases.items():
            with self.subTest(name=name):
                state = self.recover_state()
                state[name] = value
                done, log, _ = self.run_step("recover", state=state)
                self.assertEqual(done.returncode, 1)
                self.assertEqual(done.stderr.strip(), message)
                self.assertEqual(log, "")

    def assert_recover_failed(self, done, log, state, message):
        """The step failed with this first line. It wrote no volume name and deleted no volume."""
        self.assertEqual(done.returncode, 1, done.stderr + log)
        self.assertEqual(done.stderr.strip().splitlines()[0], message)
        self.assertNotIn("new.volume", state)
        self.assertNotIn("volume rm", log)

    def test_recover_fails_when_pitr_volume_fails(self):
        env = {"STUB_RECOVER_OUTPUT": '{"level":"ERROR","job":"pitr-volume","message":"x"}', "STUB_RECOVER_STATUS": "1"}
        done, log, state = self.run_step("recover", env, self.recover_state())
        self.assert_recover_failed(done, log, state, "CI: recover: pitr-volume failed.")

    def test_recover_fails_when_the_line_comes_from_a_drill(self):
        line = DRILL_LINE.replace('"chain_seq":41', '"chain_seq":42')
        done, log, state = self.run_step("recover", {"STUB_RECOVER_OUTPUT": line}, self.recover_state())
        self.assert_recover_failed(done, log, state, "CI: recover: the completed line of pitr-volume has drill true.")

    def test_recover_fails_when_the_newest_entry_is_not_marker_2(self):
        cases = {
            "sequence": RECOVER_LINE.replace('"chain_seq":42', '"chain_seq":41'),
            "action": RECOVER_LINE.replace('"action":"OwnPasswordChanged"', '"action":"UserLoggedIn"'),
        }
        for name, line in cases.items():
            with self.subTest(name=name):
                done, log, state = self.run_step("recover", {"STUB_RECOVER_OUTPUT": line}, self.recover_state())
                self.assert_recover_failed(
                    done, log, state, "CI: recover: the newest audit entry of the new volume is not marker 2.")

    def test_recover_fails_when_the_last_line_is_wrong(self):
        done, log, state = self.run_step("recover", {"STUB_RECOVER_OUTPUT": '{"level":"INFO","message":"other"}'},
                                         self.recover_state())
        self.assert_recover_failed(done, log, state, "CI: recover: the last line of pitr-volume is wrong.")

    def test_recover_fails_when_the_volume_cannot_be_created(self):
        done, log, state = self.run_step("recover", {"STUB_VOLUME_CREATE_STATUS": "1"}, self.recover_state())
        self.assertEqual(done.returncode, 1, done.stderr + log)
        self.assertRegex(done.stderr.strip(), rf"^CI: recover: cannot create the volume {NEW_VOLUME}\.$")
        self.assertNotIn("run --rm --no-deps", log)
        self.assertNotIn("new.volume", state)

    def test_recover_text_has_no_exit_idiom_and_no_new_deletion(self):
        body = shell_function("recover") + shell_function("create_labelled_volume")
        self.assertIn("recipe.ci", body)
        self.assertNotIn("|| exit", body)
        self.assertNotIn("docker volume rm", body)
        self.assertNotIn("--drill", body)
        self.assertNotIn("--target-time", body)
        self.assertEqual(script_text().count("docker volume rm"), 0)

    # switch

    def switch_state(self):
        # The step recover leaves the new volume and the completed line of pitr-volume.
        return {"new.volume": NEW_VOLUME_NAME + "\n", "pitr-volume.log": RECOVER_LINE + "\n"}

    def run_switch(self, env_extra=None, state=None):
        # STUB_DRILL_TEST_STATUS=1 means that the new volume holds no file pitr-drill.
        env = {"STUB_DRILL_TEST_STATUS": "1"}
        env.update(env_extra or {})
        return self.run_step("switch", env, self.switch_state() if state is None else state)

    def test_switch_pass_path_runs_the_steps_in_order(self):
        done, log, files = self.run_switch()
        self.assertEqual(done.returncode, 0, done.stderr + log)
        self.assertEqual(docker_calls.forbidden_window_calls(files["argv"]), [])
        self.assertEqual(self.walarchive_window_hits("switch", files["argv"]), [])
        self.assert_ordered(log, [
            r"^compose --project-name recipe-ci-1-1 .* stop proxy app backup walarchive$",
            r"^compose --project-name recipe-ci-1-1 .* ps -aq db$",
            r"^kill 0123abcd$",
            r"^inspect --format \{\{\.State\.Running\}\} 0123abcd$",
            r"^compose --project-name recipe-ci-1-1 .* rm -f proxy app backup walarchive db$",
            r"^run --rm --network none .* -v " + OLD_VOLUME + r":/var/lib/postgresql/data:ro --entrypoint pg_controldata ",
            r"^compose --project-name recipe-ci-1-1 .* config --format json$",
            r"upgrade_checks\.py volume-name ",
            rf"^run --rm --network none .* -v {NEW_VOLUME_NAME}:/var/lib/postgresql/data:ro --entrypoint pg_controldata ",
            rf"^run --rm --network none .* -v {NEW_VOLUME_NAME}:/var/lib/postgresql/data:ro --entrypoint test ",
            r"^compose --project-name recipe-ci-1-1 .* up -d --no-deps --wait --wait-timeout 180 db$",
            r"^inspect --format \{\{range \.Mounts\}\}",
        ])

    def test_switch_goes_on_when_the_kill_of_the_dead_db_fails(self):
        # The step crash killed db. The kill of the switch fails, and db does not run.
        done, log, _ = self.run_switch()
        self.assertEqual(done.returncode, 0, done.stderr + log)
        self.assertEqual(len(re.findall(r"(?m)^kill 0123abcd$", log)), 1, log)
        self.assertNotIn(" kill ", log)
        self.assertEqual(len(re.findall(r"(?m)^compose .* up -d ", log)), 1, log)
        self.assertNotIn("volume rm", log)

    def test_switch_writes_unreadable_for_the_old_volume_and_the_line_for_the_new_one(self):
        done, log, state = self.run_switch()
        self.assertEqual(done.returncode, 0, done.stderr + log)
        self.assertEqual(
            state["change.log"],
            f"volume: {OLD_VOLUME}\ntag: ci-pitr-recovery\nDatabase system identifier: unreadable\n"
            "Latest checkpoint's TimeLineID: unreadable\n")
        self.assertEqual(state["old.volume"], OLD_VOLUME + "\n")
        self.assertEqual(state["active.volume"], NEW_VOLUME_NAME + "\n")

    def test_switch_keeps_the_state_that_wal_line_and_check_read(self):
        done, log, state = self.run_switch()
        self.assertEqual(done.returncode, 0, done.stderr + log)
        self.assertEqual(state["pitr-volume.log"], RECOVER_LINE + "\n")
        self.assertEqual(state["new.volume"], NEW_VOLUME_NAME + "\n")
        self.assertNotIn("new.system-identifier", state)

    def test_switch_compares_the_identifier_and_the_timeline_of_the_line(self):
        cases = {
            "identifier": ({"STUB_NEW_IDENTIFIER": "1111111111111111111"},
                           "CI: switch: the system identifier of the new volume differs from the line of pitr-volume."),
            "timeline": ({"STUB_NEW_TIMELINE": "3"},
                         "CI: switch: the timeline of the new volume differs from the line of pitr-volume."),
            "state": ({"STUB_NEW_STATE": "in production"},
                      "CI: switch: the new volume is not in the state 'shut down'."),
        }
        for name, (env, message) in cases.items():
            with self.subTest(name=name):
                done, log, state = self.run_switch(env)
                self.assertEqual(done.returncode, 1, done.stderr + log)
                self.assertEqual(done.stderr.strip(), message)
                self.assertNotRegex(log, r"(?m)up -d .* db$")
                self.assertNotIn("active.volume", state)

    def test_switch_stops_when_compose_does_not_show_the_new_volume(self):
        done, log, state = self.run_switch({"STUB_COMPOSE_VOLUME": "other_volume"})
        self.assertEqual(done.returncode, 1, done.stderr + log)
        self.assertEqual(done.stderr.strip(), "CI: switch: Compose does not show the new volume for pgdata.")
        self.assertNotRegex(log, r"(?m)up -d .* db$")
        self.assertNotIn("active.volume", state)

    def test_switch_refuses_a_volume_with_the_file_pitr_drill(self):
        done, log, state = self.run_switch({"STUB_DRILL_TEST_STATUS": "0"})
        self.assertEqual(done.returncode, 1, done.stderr + log)
        self.assertEqual(done.stderr.strip(),
                         "CI: switch: the new volume holds the file pitr-drill. A drill volume never becomes production.")
        self.assertNotRegex(log, r"(?m)up -d .* db$")
        self.assertNotIn("active.volume", state)

    def test_switch_refuses_a_line_with_drill_true_before_it_stops_a_container(self):
        state = self.switch_state()
        state["pitr-volume.log"] = DRILL_LINE + "\n"
        done, log, content = self.run_switch(state=state)
        self.assertEqual(done.returncode, 1, done.stderr + log)
        self.assertEqual(done.stderr.strip(),
                         "CI: switch: the line of pitr-volume comes from a drill. A drill volume never becomes production.")
        self.assertEqual(log, "")
        self.assertNotIn("change.log", content)

    def test_switch_refuses_a_last_line_that_is_not_the_completed_line(self):
        state = self.switch_state()
        state["pitr-volume.log"] = '{"level":"INFO","message":"other"}\n'
        done, log, content = self.run_switch(state=state)
        self.assertEqual(done.returncode, 1, done.stderr + log)
        self.assertEqual(done.stderr.strip(), "CI: switch: the last line of pitr-volume is wrong.")
        self.assertEqual(log, "")
        self.assertNotIn("change.log", content)

    def test_switch_requires_the_new_volume_and_the_line_of_pitr_volume(self):
        for missing in ("new.volume", "pitr-volume.log"):
            with self.subTest(missing=missing):
                state = self.switch_state()
                del state[missing]
                done, log, _ = self.run_switch(state=state)
                self.assertEqual(done.returncode, 1)
                self.assertEqual(done.stderr.strip(), "CI: switch: run pitr-volume first.")
                self.assertEqual(log, "")

    def test_switch_stops_when_the_kill_fails_and_db_still_runs(self):
        done, log, state = self.run_switch({"STUB_DB_RUNNING": "true"})
        self.assertEqual(done.returncode, 1, done.stderr + log)
        self.assertEqual(done.stderr.strip(), "CI: switch: cannot kill db.")
        self.assertNotIn(" rm -f ", log)
        self.assertNotIn("change.log", state)
        self.assertNotIn("active.volume", state)

    def test_switch_calls_the_shared_switch_with_the_pitr_line(self):
        self.assertIn('  switch) switch_recovered ;;', script_text())
        self.assertIn('switch "pitr volume completed"', shell_function("switch_recovered"))
        self.assertNotIn("stop_services", script_text())
        self.assertIn('docker kill "$(container db)"', shell_function("stop_services"))

    # switch, role passwords (ADR-0106 (p) step 6). A recovered cluster keeps the passwords of the recovery point.

    def test_switch_sets_the_role_passwords_after_db_starts_and_proves_each_login(self):
        done, log, state = self.run_switch()
        self.assertEqual(done.returncode, 0, done.stderr + log)
        self.assert_ordered(log, [
            r"^compose --project-name recipe-ci-1-1 .* up -d --no-deps --wait --wait-timeout 180 db$",
            r"^inspect --format \{\{range \.Mounts\}\}",
            r"^compose --project-name recipe-ci-1-1 .* exec -T -u postgres db psql -v ON_ERROR_STOP=1 --no-password --no-psqlrc --dbname postgres$",
            r"^compose --project-name recipe-ci-1-1 .* --entrypoint bash backup-premigrate -c ",
            r"^done probe recipe_migrator recipe_app recipe_backup recipe_replicator$",
        ])
        self.assertEqual(len(re.findall(r"backup-premigrate -c ", log)), 1, log)
        self.assertEqual(state["active.volume"], NEW_VOLUME_NAME + "\n")

    def test_switch_logs_the_password_steps_after_the_shared_steps(self):
        done, log, _ = self.run_switch()
        self.assertEqual(done.returncode, 0, done.stderr + log)
        self.assert_ordered(done.stdout, [
            r"^CI: switch: step 5, start db on the new volume$",
            r"^CI: switch: done$",
            r"^CI: switch: set the passwords from the secret files$",
            r"^CI: switch: password login of each login role works$",
        ])

    def test_switch_stops_with_one_named_line_when_the_passwords_fail(self):
        done, log, state = self.run_switch({"STUB_PASSWORDS_STATUS": "1"})
        self.assertEqual(done.returncode, 1, done.stderr + log)
        self.assertEqual(done.stderr.strip(), "CI: switch: cannot set the role passwords.")
        self.assertNotIn("backup-premigrate", log)

    def test_switch_stops_with_one_named_line_when_a_role_cannot_log_in(self):
        for result in ("password", "reject", "other"):
            with self.subTest(result=result):
                done, log, _ = self.run_switch({"STUB_PROBE_BAD_ROLE": "recipe_app", "STUB_PROBE_BAD_RESULT": result})
                self.assertEqual(done.returncode, 1, done.stderr + log)
                self.assertEqual(done.stderr.strip(), f"CI: switch: the login of recipe_app gave '{result}'. Expected 'ok'.")

    def test_switch_stops_when_the_probe_container_fails(self):
        done, log, _ = self.run_switch({"STUB_PROBE_STATUS": "1"})
        self.assertEqual(done.returncode, 1, done.stderr + log)
        self.assertEqual(done.stderr.strip(), "CI: switch: the probe container failed.")

    def test_switch_stops_when_the_probe_reports_too_few_roles(self):
        done, log, _ = self.run_switch({"STUB_PROBE_SKIP_ROLE": "recipe_replicator"})
        self.assertEqual(done.returncode, 1, done.stderr + log)
        self.assertEqual(done.stderr.strip(), "CI: switch: the probe reported 3 of 4 roles.")

    def test_switch_does_not_run_the_checks_that_need_a_fresh_cluster(self):
        # A restored cluster has no passwords, so restore-switch checks that a login fails first. A recovered cluster has them.
        outer = shell_function("switch_recovered")
        body = shell_function("prove_role_logins")
        self.assertLess(outer.index('switch "pitr volume completed"'), outer.index("prove_role_logins"))
        for forbidden in ("expect_probe password", "expect_probe reject", "check_db_log"):
            self.assertNotIn(forbidden, outer + body)
        self.assertLess(body.index("set_role_passwords"), body.index("expect_probe ok"))
        self.assertNotRegex(outer + body, r"\|\| exit 1")

    def test_the_probe_has_one_definition_in_the_shared_file(self):
        shared = STEPS_FILE.read_text(encoding="utf-8")
        for name in ("probe_roles", "expect_probe", "check_db_log_passwords"):
            self.assertEqual(shared.count(f"\n{name}() {{\n"), 1, name)
            self.assertNotIn(f"\n{name}() {{\n", script_text())
            self.assertNotIn(f"\n{name}() {{\n", (CI / "restore-switch.sh").read_text(encoding="utf-8"))

    # wal-line

    def wal_line_state(self):
        # The step switch leaves this state.
        done, log, state = self.run_switch()
        self.assertEqual(done.returncode, 0, done.stderr + log)
        return {name: text for name, text in state.items() if name != "argv"}

    def run_wal_line(self, env_extra=None, state=None):
        return self.run_step("wal-line", env_extra, self.wal_line_state() if state is None else state)

    def test_wal_line_pass_path_runs_the_steps_in_order(self):
        done, log, files = self.run_wal_line()
        self.assertEqual(done.returncode, 0, done.stderr + log)
        self.assertEqual(docker_calls.forbidden_window_calls(files["argv"]), [])
        self.assertEqual(self.walarchive_window_hits("wal-line", files["argv"]), [])
        self.assert_ordered(log, [
            r"^compose --project-name recipe-ci-1-1 .* ps -aq walarchive$",
            r"^sudo mkdir -m 0700 -- \S+\.moved-[0-9]{8}_[0-9]{6}$",
            r"^sudo tee -- \S+\.moved-[0-9]{8}_[0-9]{6}/old-system-identifier$",
            r"^sudo cat \S+/\.system-identifier$",
            r"^sudo mv -t \S+\.moved-[0-9]{8}_[0-9]{6} -- ",
            r"^sudo mkdir -m 0700 -- \S+\.moved-[0-9]{8}_[0-9]{6}/base$",
            r"^sudo mv -t \S+\.moved-[0-9]{8}_[0-9]{6}/base -- ",
            r"^compose --project-name recipe-ci-1-1 .* up -d --no-deps walarchive$",
            r"SELECT state, flush_lsn FROM pg_stat_replication",
            r"exec -T walarchive basebackup-once$",
            r"SELECT pg_switch_wal\(\)",
            r"flush_lsn >=",
            r"exec -T walarchive pg_verifybackup ",
            r"^sudo cat \S+/\.system-identifier$",
        ])

    def test_wal_line_moves_the_history_files_with_the_segments(self):
        done, log, _ = self.run_wal_line()
        self.assertEqual(done.returncode, 0, done.stderr + log)
        moves = re.findall(r"(?m)^sudo mv -t \S+\.moved-[0-9]{8}_[0-9]{6} -- (.*)$", log)
        self.assertEqual(len(moves), 1, log)
        names = [path.rsplit("/", 1)[1] for path in moves[0].split()]
        self.assertEqual(names, [
            "000000010000000000000002", "000000010000000000000003.partial", "00000002.history", ".system-identifier"])
        self.assertEqual(len(re.findall(r"(?m)^sudo find .*\[0-9A-F\]\{8\}\\\.history -printf", log)), 2, log)

    def test_wal_line_writes_only_the_identifier_of_the_line_into_the_old_identifier_file(self):
        # ADR-0106 (k), (l). The change log entry is unreadable. The timeline of the line belongs to the new cluster.
        done, log, state = self.run_wal_line()
        self.assertEqual(done.returncode, 0, done.stderr + log)
        self.assertEqual(state["stub.old-system-identifier"], "Database system identifier: 7234567890123456789\n")
        self.assertNotIn("TimeLineID", state["stub.old-system-identifier"])

    def test_wal_line_saves_the_base_backups_and_keeps_the_line_of_pitr_volume(self):
        done, log, state = self.run_wal_line()
        self.assertEqual(done.returncode, 0, done.stderr + log)
        self.assertEqual(state["wal-new.base"], "base_20261030_120200\n")
        self.assertEqual(state["wal-old.bases"], "base_20261030_115900\n")
        self.assertRegex(state["wal-target.dir"], r"\.moved-[0-9]{8}_[0-9]{6}\n$")
        self.assertIn("wal-target.manifest", state)
        self.assertEqual(state["pitr-volume.log"], RECOVER_LINE + "\n")

    def test_wal_line_stops_when_the_archive_file_names_another_cluster(self):
        done, log, _ = self.run_wal_line({"STUB_ARCHIVE_ID": "1111111111111111111"})
        self.assertEqual(done.returncode, 1, done.stderr + log)
        self.assertEqual(
            done.stderr.strip(),
            "CI: wal-line: the file .system-identifier does not hold the identifier of the old cluster.")
        self.assertNotIn("sudo mv ", log)

    def test_wal_line_stops_when_the_new_line_file_is_wrong(self):
        # The stub answers with another identifier after walarchive starts.
        done, log, _ = self.run_wal_line({"STUB_NEW_ARCHIVE_ID": "1111111111111111111"})
        self.assertEqual(done.returncode, 1, done.stderr + log)
        self.assertEqual(done.stderr.strip(),
                         "CI: wal-line: the file .system-identifier of the new line is missing or wrong.")

    def test_wal_line_requires_the_state_of_switch(self):
        for missing in ("change.log", "old.volume", "active.volume", "pitr-volume.log"):
            with self.subTest(missing=missing):
                state = self.wal_line_state()
                del state[missing]
                done, log, _ = self.run_wal_line(state=state)
                self.assertEqual(done.returncode, 1)
                self.assertEqual(done.stderr.strip(), "CI: wal-line: run switch first.")
                self.assertEqual(log, "")

    def test_wal_line_refuses_a_line_with_drill_true_before_it_changes_the_archive(self):
        state = self.wal_line_state()
        state["pitr-volume.log"] = DRILL_LINE + "\n"
        done, log, _ = self.run_wal_line(state=state)
        self.assertEqual(done.returncode, 1, done.stderr + log)
        self.assertEqual(done.stderr.strip(),
                         "CI: wal-line: the line of pitr-volume comes from a drill. A drill volume never becomes production.")
        self.assertEqual(log, "")

    def test_wal_line_stops_when_the_walarchive_container_exists(self):
        done, log, _ = self.run_wal_line({"STUB_WALARCHIVE_PS": "abc123"})
        self.assertEqual(done.returncode, 1, done.stderr + log)
        self.assertEqual(done.stderr.strip(), "CI: wal-line: the walarchive container exists. The switch must delete it.")
        self.assertNotIn("sudo mkdir", log)

    def test_wal_line_stops_when_the_stream_check_never_passes(self):
        done, log, _ = self.run_wal_line({"STUB_STREAM_ROW": ""})
        self.assertEqual(done.returncode, 1, done.stderr + log)
        self.assertEqual(done.stderr.strip(), "CI: wal-line: the stream check did not find one streaming row.")
        self.assertNotIn("basebackup-once", log)

    def test_wal_line_stops_when_the_flush_check_never_returns_t(self):
        done, log, _ = self.run_wal_line({"STUB_FLUSH": "f"})
        self.assertEqual(done.returncode, 1, done.stderr + log)
        self.assertEqual(done.stderr.strip(), "CI: wal-line: the flush check did not return t.")
        self.assertNotIn("pg_verifybackup", log)

    def test_wal_line_calls_the_shared_steps(self):
        self.assertIn("  wal-line) wal_line ;;", script_text())
        body = shell_function("wal_line")
        self.assertIn("read_pitr_line wal-line", body)
        self.assertIn('start_wal_line "$LINE_IDENTIFIER"', body)

    def test_the_line_of_pitr_volume_stays_until_the_step_check(self):
        # The step check reads the line again. No step before it may delete the file.
        for text in (script_text(), STEPS_FILE.read_text(encoding="utf-8")):
            for line in text.splitlines():
                if "pitr-volume.log" in line:
                    self.assertNotRegex(line, r"\brm\b|\bmv\b|truncate|^\s*:\s*>")
        body = shell_function("switch") + shell_function("wal_line")
        self.assertNotRegex(body, r">\s*\"?\$CI_STATE/pitr-volume\.log")

    def test_switch_and_wal_line_texts_have_no_exit_idiom_and_no_new_deletion(self):
        text = shell_function("wal_line") + shell_function("stop_services")
        self.assertNotIn("|| exit", text)
        self.assertNotIn("docker volume rm", text)
        self.assertEqual(script_text().count("docker volume rm"), 0)

    # check

    def check_state(self):
        # The steps markers, crash, recover, switch, and wal-line leave this state.
        # admin_password holds the password of marker 1 and new_password holds the password of marker 2.
        return {
            "admin_password": MARKER_1_PASSWORD + "\n",
            "new_password": MARKER_2_PASSWORD + "\n",
            "cookies.json": "{}\n",
            "marker2.seq": "42\n",
            "marker2.hash": MARKER_2_HASH + "\n",
            "recovery.start": "1790000000\n",
            "wal-new.base": "base_20261030_120200\n",
            "active.volume": NEW_VOLUME_NAME + "\n",
        }

    def run_check(self, env_extra=None, state=None):
        # The default clock gives a recovery time of 1200 s. The row of the newest audit entry is marker 3.
        env = {
            "STUB_ELAPSED": "1200",
            "STUB_ACCEPTED_PASSWORD": MARKER_2_PASSWORD,
            "STUB_ROW": f"43|{MARKER_3_HASH}|OwnPasswordChanged",
        }
        env.update(env_extra or {})
        return self.run_step("check", env, self.check_state() if state is None else state)

    def test_check_pass_path_runs_the_steps_in_order(self):
        done, log, files = self.run_check()
        self.assertEqual(done.returncode, 0, done.stderr + log)
        self.assertEqual(docker_calls.forbidden_check_window_calls(files["argv"]), [])
        self.assert_ordered(log, [
            r"TRUNCATE public\.session, public\.login_attempt",
            r"^compose --project-name recipe-ci-1-1 .* up -d --no-deps --wait --wait-timeout 180 app$",
            r"^clock \+%s$",
            r"^compose --project-name recipe-ci-1-1 .* up -d --no-deps --wait --wait-timeout 180 proxy backup$",
            r"exec -T proxy wget .*readyz",
            "^relogin password=" + MARKER_1_PASSWORD + "$",
            "^relogin password=" + MARKER_2_PASSWORD + "$",
            r"^compose --project-name recipe-ci-1-1 .* exec -T backup backup-now$",
            r"^compose --project-name recipe-ci-1-1 .* exec -T backup restore-test$",
            r"^compose --project-name recipe-ci-1-1 .* exec -T walarchive basebackup-once$",
            PASSWORD_CALL,
            r"ORDER BY chain_seq DESC LIMIT 1",
            r"SELECT pg_current_wal_lsn\(\)",
            r"flush_lsn >=",
            r"^sudo test -f .*000000010000000000000003\.partial$",
            r"^compose --project-name recipe-ci-1-1 .* run --rm --no-deps -T -e PITR_CI=1 -e PITR_MARKER_SEQ -e PITR_MARKER_HASH pitr-check$",
            r" logs --no-color --no-log-prefix db$",
            r"^sudo grep -c -F -f \S+/secrets/postgres_password .*/secrets/replicator_password \S+/db-after-check\.log$",
        ])

    def test_check_puts_the_password_of_marker_2_back_after_the_login_with_marker_1(self):
        done, log, state = self.run_check()
        self.assertEqual(done.returncode, 0, done.stderr + log)
        # The new marker moves the passwords by one place: marker 2 is the old one and the new password is the last.
        self.assertEqual(state["admin_password"], MARKER_2_PASSWORD + "\n")
        self.assertNotEqual(state["new_password"], MARKER_1_PASSWORD + "\n")
        self.assertEqual(log.count("relogin password="), 2, log)

    def test_check_passes_the_identity_of_the_new_marker_to_pitr_check(self):
        done, log, state = self.run_check()
        self.assertEqual(done.returncode, 0, done.stderr + log)
        self.assertIn(f"PITR_MARKER_SEQ=43 PITR_MARKER_HASH={MARKER_3_HASH}\n", log)
        self.assertEqual(state["marker3.seq"], "43\n")
        self.assertEqual(state["marker3.hash"], MARKER_3_HASH + "\n")

    def test_check_stops_when_the_password_of_marker_1_logs_in(self):
        done, log, _ = self.run_check({"STUB_ACCEPTED_PASSWORD": ""})
        self.assertEqual(done.returncode, 1, done.stderr + log)
        self.assertEqual(done.stderr.strip(), "CI: check: the login with the password of marker 1 passed.")
        self.assertNotIn("backup-now", log)

    def test_check_stops_when_the_password_of_marker_2_does_not_log_in(self):
        done, log, _ = self.run_check({"STUB_ACCEPTED_PASSWORD": "9" * 64})
        self.assertEqual(done.returncode, 1, done.stderr + log)
        self.assertEqual(done.stderr.strip(), "CI: check: the login with the password of marker 2 failed.")
        self.assertNotIn("backup-now", log)

    def test_check_stops_when_the_log_of_db_holds_a_password(self):
        # The stub of docker prints the value of the secret app_password in the log of db.
        done, log, _ = self.run_check({"STUB_DB_LOG": f"LOG:  statement with {SECRET_VALUES['app']}"})
        self.assertEqual(done.returncode, 1, done.stderr + log)
        self.assertEqual(done.stderr.strip(), "CI: check: the log of db holds 1 lines with a password.")
        self.assertNotIn("check: done", done.stdout)

    def test_check_stops_when_the_log_of_db_is_empty(self):
        done, log, _ = self.run_check({"STUB_DB_LOG": ""})
        self.assertEqual(done.returncode, 1, done.stderr + log)
        self.assertEqual(done.stderr.strip(), "CI: check: the log of db is empty.")

    def test_check_stops_when_readyz_does_not_answer_ok(self):
        done, log, _ = self.run_check({"STUB_READYZ": "fail"})
        self.assertEqual(done.returncode, 1, done.stderr + log)
        self.assertEqual(done.stderr.strip(), "CI: check: /readyz did not answer ok.")
        self.assertNotIn("relogin", log)

    def test_check_names_each_failed_command(self):
        cases = {
            "backup-now": ({"STUB_BACKUP_NOW_STATUS": "1"}, "CI: check: backup-now failed.", "exec -T backup restore-test"),
            "restore-test": ({"STUB_RESTORE_TEST_STATUS": "1"}, "CI: check: restore-test failed.", "exec -T walarchive basebackup-once"),
            "restore-test without the now set": (
                {"STUB_RESTORE_TEST_OUTPUT": '{"level":"INFO","message":"restore test passed","set":"recipe_20261030_1205"}'},
                "CI: check: restore-test did not prove the new now set.", "exec -T walarchive basebackup-once"),
            "basebackup-once": ({"STUB_BASEBACKUP_STATUS": "1"}, "CI: check: basebackup-once failed.", "http_probe.py password"),
            "new marker": ({"STUB_HTTP_FAIL_AT": "1"}, "CI: check: the command ChangeOwnPassword of marker 3 failed.", "PITR_CI=1"),
            "pitr-check": ({"STUB_PITR_CHECK_STATUS": "1"}, "CI: check: pitr-check failed.", "check: done"),
        }
        for name, (env, message, later) in cases.items():
            with self.subTest(name=name):
                done, log, _ = self.run_check(env)
                self.assertEqual(done.returncode, 1, done.stderr + log)
                self.assertEqual(done.stderr.strip(), message)
                self.assertNotIn(later, log + done.stdout)

    def test_check_logs_the_time_of_each_step_and_the_recovery_time(self):
        done, log, _ = self.run_check()
        self.assertEqual(done.returncode, 0, done.stderr + log)
        self.assertIn("CI: check: the recovery time is 1200 s. The limit is 3600 s.\n", done.stdout)
        for number in (1, 2, 3, 4, 5):
            with self.subTest(step=number):
                self.assertRegex(done.stdout, rf"(?m)^CI: check: step {number} took [0-9]+ s$")

    def test_check_accepts_a_recovery_time_of_exactly_the_limit(self):
        done, log, _ = self.run_check({"STUB_ELAPSED": "3600"})
        self.assertEqual(done.returncode, 0, done.stderr + log)
        self.assertIn("CI: check: the recovery time is 3600 s. The limit is 3600 s.\n", done.stdout)

    def test_check_fails_when_the_recovery_time_is_above_the_limit(self):
        done, log, _ = self.run_check({"STUB_ELAPSED": "3601"})
        self.assertEqual(done.returncode, 1, done.stderr + log)
        self.assertEqual(done.stderr.strip(), "CI: check: the recovery time of 3601 s is above the limit of 3600 s.")
        self.assertNotIn("relogin", log)

    def test_check_measures_the_recovery_time_from_the_start_of_crash(self):
        done, log, _ = self.run_check({"STUB_ELAPSED": "42"}, self.check_state() | {"recovery.start": "1780000000\n"})
        self.assertEqual(done.returncode, 0, done.stderr + log)
        self.assertIn("CI: check: the recovery time is 42 s. The limit is 3600 s.\n", done.stdout)

    def test_check_requires_the_state_of_the_earlier_steps(self):
        cases = {
            "wal-new.base": "CI: check: run wal-line first.",
            "active.volume": "CI: check: run wal-line first.",
            "recovery.start": "CI: check: run crash first.",
            "admin_password": "CI: check: run markers first.",
            "new_password": "CI: check: run markers first.",
            "cookies.json": "CI: check: run markers first.",
        }
        for missing, message in cases.items():
            with self.subTest(missing=missing):
                state = self.check_state()
                del state[missing]
                done, log, _ = self.run_check(state=state)
                self.assertEqual(done.returncode, 1)
                self.assertEqual(done.stderr.strip(), message)
                self.assertEqual(log, "")

    def test_check_rejects_an_invalid_start_time(self):
        done, log, _ = self.run_check(state=self.check_state() | {"recovery.start": "soon\n"})
        self.assertEqual(done.returncode, 1)
        self.assertEqual(done.stderr.strip(), "CI: check: the start time of the recovery is invalid.")
        self.assertEqual(log, "")

    def test_check_follows_the_order_of_the_task(self):
        body = shell_function("check")
        markers = ["run wal-line first", "TRUNCATE public.session, public.login_attempt", "wait-timeout 180 app",
                   "check_recovery_time", "proxy backup", "check_readyz", "refuse_marker_1_password",
                   "http_probe relogin", "backup-now", "restore-test", "take_base_backup", "commit_marker 3",
                   "wait_marker_flush", "check_pitr", "check_db_log_passwords"]
        found = [body.find(marker) for marker in markers]
        self.assertNotIn(-1, found, dict(zip(markers, found)))
        self.assertEqual(found, sorted(found), dict(zip(markers, found)))

    def test_the_recovery_limit_is_a_named_constant_that_cites_reliability_md(self):
        text = script_text()
        self.assertRegex(text, r"(?m)^# .*reliability\.md 7\.7.*\nRECOVERY_TIME_LIMIT_SECONDS=3600$")
        self.assertNotRegex(shell_function("check_recovery_time"), r"\b3600\b")

    def test_check_reuses_the_shared_checks_of_the_restore_path(self):
        shared = STEPS_FILE.read_text(encoding="utf-8")
        restore = (CI / "restore-switch.sh").read_text(encoding="utf-8")
        for name in ("readyz_ok", "check_readyz", "check_pitr"):
            marker = f"\n{name}() {{\n"
            with self.subTest(name=name):
                self.assertEqual(shared.count(marker), 1)
                self.assertNotIn(marker, script_text())
                self.assertNotIn(marker, restore)

    def test_check_has_no_exit_idiom_and_no_new_deletion(self):
        text = shell_function("check") + shell_function("check_recovery_time") + shell_function("refuse_marker_1_password")
        self.assertNotIn("|| exit", text)
        self.assertNotIn("docker volume rm", text)
        self.assertEqual(script_text().count("docker volume rm"), 0)
        self.assertNotRegex(text, r"\brm\b(?! -f -- \"\$CI_STATE/cookies\.json\")")

    # refuse

    def refuse_state(self):
        # The step wal-line leaves the target directory of the move and its manifest. The step check leaves the active volume.
        return self.check_state() | {"wal-target.dir": MOVED_DIR + "\n", "wal-target.manifest": ""}

    def run_refuse(self, env_extra=None, state=None):
        return self.run_step("refuse", env_extra, self.refuse_state() if state is None else state)

    def assert_failed_refuse_cleaned_up(self, done, log, message):
        self.assertEqual(done.returncode, 1, done.stderr + log)
        self.assertEqual(done.stderr.strip().splitlines()[0], message)
        self.assertRegex(log, r"(?m)^sudo rm -rf -- .*/state/wal-copy$")
        self.assertNotRegex(log, r"(?m)^volume (create|rm) ")

    def test_refuse_pass_path_runs_the_steps_in_order(self):
        done, log, _ = self.run_refuse()
        self.assertEqual(done.returncode, 0, done.stderr + log)
        self.assert_ordered(log, [
            r"^compose --project-name recipe-ci-1-1 .* ps -aq db$",
            r"^volume ls -q --filter label=recipe\.ci=recipe-ci-1-1$",
            r"^sudo test -e .*/state/wal-copy$",
            r"^sudo install -d .*/state/wal-copy$",
            r"^sudo cp -a -- .*/walarchive-disk/walarchive/\. .*/state/wal-copy/$",
            r"^sudo find \S+\.moved-20261030_120100 -maxdepth 1 -type f ",
            r"^sudo test -e .*/state/wal-copy/000000010000000000000002$",
            r"^sudo install -o 999 -g 999 -m 0600 -- \S+\.moved-20261030_120100/000000010000000000000002 "
            r".*/state/wal-copy/000000010000000000000002$",
            r"^compose --project-name recipe-ci-1-1 .* run --rm --no-deps -T -v .*/state/wal-copy:/wal walarchive$",
            r"^sudo find \S+\.moved-20261030_120100 -mindepth 1$",
            r"^sudo rm -rf -- .*/state/wal-copy$",
            r"^compose --project-name recipe-ci-1-1 .* ps -aq db$",
            r"^volume ls -q --filter label=recipe\.ci=recipe-ci-1-1$",
        ])
        self.assertIn("CI: refuse: done", done.stdout)

    def test_refuse_puts_the_old_segment_only_into_the_copy(self):
        done, log, state = self.run_refuse()
        self.assertEqual(done.returncode, 0, done.stderr + log)
        self.assertEqual(len(re.findall(r"^sudo install -o 999 ", log, re.M)), 1)
        self.assertEqual(len(re.findall(r"^sudo cp ", log, re.M)), 1)
        self.assertNotIn("refusals.wal-copy", state)

    def test_refuse_writes_only_into_the_copy_and_creates_no_volume(self):
        done, log, _ = self.run_refuse()
        self.assertEqual(done.returncode, 0, done.stderr + log)
        writes = [line for line in log.splitlines() if re.match(r"^sudo (rm|install|cp|mv|chmod|chown|touch|tee|truncate|ln|dd)\b", line)]
        self.assertGreaterEqual(len(writes), 4)
        for line in writes:
            self.assertRegex(line.split()[-1], r"/state/wal-copy(/.*)?$", line)
        self.assertNotRegex(log, r"(?m)^volume (create|rm) ")

    def test_refuse_runs_walarchive_entry_on_the_copy_only(self):
        done, log, _ = self.run_refuse()
        self.assertEqual(done.returncode, 0, done.stderr + log)
        runs = [line for line in log.splitlines() if " run " in line]
        self.assertEqual(len(runs), 1, runs)
        self.assertRegex(runs[0], r" run --rm --no-deps -T -v \S+/state/wal-copy:/wal walarchive$")
        self.assertNotIn("PITR_", log)
        self.assertNotIn("pg_switch_wal", log)
        self.assertNotIn("up -d", log)

    def test_refuse_reads_the_facts_of_production_before_and_after(self):
        done, log, _ = self.run_refuse()
        self.assertEqual(done.returncode, 0, done.stderr + log)
        self.assertEqual(len(re.findall(r"^inspect --format container ", log, re.M)), 2)
        self.assertEqual(len(re.findall(r"^volume ls ", log, re.M)), 2)

    def test_refuse_requires_the_state_of_wal_line(self):
        for missing in ("wal-target.dir", "wal-target.manifest", "active.volume"):
            with self.subTest(missing=missing):
                state = self.refuse_state()
                del state[missing]
                done, log, _ = self.run_refuse(state=state)
                self.assertEqual(done.returncode, 1)
                self.assertEqual(done.stderr.strip(), "CI: refuse: run wal-line first.")
                self.assertEqual(log, "")

    def test_refuse_rejects_a_target_directory_with_another_name(self):
        for value in ("/tmp/other\n", MOVED_DIR.replace("walarchive.moved-", "other.moved-") + "\n", MOVED_DIR + "/x\n"):
            with self.subTest(value=value):
                done, log, _ = self.run_refuse(state=self.refuse_state() | {"wal-target.dir": value})
                self.assertEqual(done.returncode, 1)
                self.assertEqual(done.stderr.strip(), "CI: refuse: the target directory of the move is invalid.")
                self.assertEqual(log, "")

    def test_refuse_fails_when_the_guard_accepts_the_copy(self):
        done, log, _ = self.run_refuse({"STUB_REFUSE_STATUS": "0"})
        self.assert_failed_refuse_cleaned_up(
            done, log, "CI: refuse: walarchive-entry accepted the copy of /wal with a segment of the old timeline.")

    def test_refuse_fails_on_another_error_text(self):
        line = REFUSE_ERROR_LINE.replace(TIMELINE_ERROR, "the file .system-identifier is malformed")
        done, log, _ = self.run_refuse({"STUB_REFUSE_OUTPUT": line})
        self.assert_failed_refuse_cleaned_up(done, log, REFUSE_WRONG_LINE)

    def test_refuse_fails_when_the_error_line_is_not_the_only_one(self):
        done, log, _ = self.run_refuse({"STUB_REFUSE_OUTPUT": REFUSE_ERROR_LINE + "\n" + REFUSE_ERROR_LINE})
        self.assert_failed_refuse_cleaned_up(done, log, REFUSE_WRONG_LINE)

    def test_refuse_fails_when_the_error_line_comes_from_another_job(self):
        line = REFUSE_ERROR_LINE.replace("walarchive-entry", "pitr-volume")
        done, log, _ = self.run_refuse({"STUB_REFUSE_OUTPUT": line})
        self.assert_failed_refuse_cleaned_up(done, log, REFUSE_WRONG_LINE)

    def test_refuse_fails_when_the_guard_writes_no_error_line(self):
        done, log, _ = self.run_refuse({"STUB_REFUSE_OUTPUT": ""})
        self.assert_failed_refuse_cleaned_up(done, log, REFUSE_WRONG_LINE)

    def test_refuse_fails_when_the_target_directory_holds_no_segment(self):
        done, log, _ = self.run_refuse({"STUB_OLD_SEGMENTS": ""})
        self.assert_failed_refuse_cleaned_up(
            done, log, "CI: refuse: the target directory of the move holds no segment of the old timeline.")
        self.assertNotIn(" run ", log)

    def test_refuse_fails_when_the_copy_directory_exists(self):
        done, log, _ = self.run_refuse({"STUB_COMPLETE_STATUS": "0"})
        self.assertEqual(done.returncode, 1)
        self.assertEqual(done.stderr.strip(), "CI: refuse: the directory of the WAL copy exists.")
        self.assertNotIn("sudo cp", log)
        self.assertNotIn("sudo rm", log)

    def test_refuse_detects_a_change_of_the_production_volume(self):
        done, log, _ = self.run_refuse({"STUB_PROD_CHANGE": "volume"})
        self.assertEqual(done.returncode, 1, done.stderr + log)
        self.assertEqual(done.stderr.strip(), "CI: refuse: a production fact changed during the step.")

    def test_refuse_detects_a_restart_of_the_database_container(self):
        done, log, _ = self.run_refuse({"STUB_PROD_CHANGE": "container"})
        self.assertEqual(done.returncode, 1, done.stderr + log)
        self.assertEqual(done.stderr.strip(), "CI: refuse: a production fact changed during the step.")

    def test_refuse_detects_a_change_of_the_target_directory(self):
        done, log, _ = self.run_refuse({"STUB_TARGET_MANIFEST": "/x/000000010000000000000002"})
        self.assertEqual(done.returncode, 1, done.stderr + log)
        self.assertEqual(done.stderr.strip(), "CI: refuse: the target directory of the move changed during the step.")
        self.assertRegex(log, r"(?m)^sudo rm -rf -- .*/state/wal-copy$")

    def test_refuse_expects_the_message_of_adr_0106_m(self):
        self.assertRegex(script_text(), r'(?m)^TIMELINE_ERROR="segments of another timeline are in the WAL archive"$')

    def test_refuse_shares_the_copy_and_the_check_with_refusals(self):
        # One definition each. The two steps differ in the parameters ERROR_STEP and ERROR_JOB only.
        text = script_text()
        for name in ("expect_pitr_error", "delete_wal_copy", "refusals_exit", "copy_wal_archive", "production_facts"):
            with self.subTest(name=name):
                self.assertEqual(text.count(f"\n{name}() {{\n"), 1)
        body = shell_function("refuse")
        for used in ("copy_wal_archive", "expect_pitr_error", "production_facts", "delete_wal_copy"):
            self.assertIn(used, body)

    def test_refuse_deletes_the_copy_only_through_its_state_file(self):
        body = shell_function("refuse") + shell_function("old_timeline_segment")
        self.assertNotRegex(body, r"\brm\b")
        self.assertNotIn("|| exit", body)
        self.assertIn("trap refusals_exit EXIT", body)
        self.assertEqual(script_text().count("rm -rf"), 1)
        self.assertEqual(script_text().count("docker volume rm"), 0)


# The stub of sudo passes docker through and logs every other command. The tests of the flush check set the status of test.
REFUSALS_SEGMENTS = "".join(f"{name}\n" for name in (
    "000000010000000000000002",
    "000000010000000000000003",
    "000000010000000000000004",
    "000000010000000000000005",
    "000000010000000000000006.partial",
))


class RunbookRecoveryDeleteTests(unittest.TestCase):
    DELETE_COMMAND = "docker compose rm -s -f app backup"

    def setUp(self):
        spec = importlib.util.spec_from_file_location("test_upgrade_runbook", CI / "test_upgrade_runbook.py")
        self.runbook = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(self.runbook)

    def test_step_1_deletes_app_and_backup_in_one_bash_line(self):
        step = self.runbook.runbook_step("Recover from the WAL archive", "1. ")
        blocks = self.runbook.runbook_bash_blocks(step)
        self.assertEqual(blocks, [self.DELETE_COMMAND],
                         f"recovery step 1 must have one Bash block with only {self.DELETE_COMMAND!r}")

    def test_crash_function_uses_the_runbook_delete_command(self):
        command = re.escape(self.DELETE_COMMAND.removeprefix("docker "))
        self.assertRegex(shell_function("crash"), rf"(?m)^\s*{command}(?:\s*\|\|[^\n]+)?$",
                         "crash() must execute compose rm -s -f app backup")

    def test_step_4_deletes_walarchive_in_one_bash_line(self):
        step = self.runbook.runbook_step("Recover from the WAL archive", "4. ")
        self.assertEqual(self.runbook.stripped_bash_blocks(step), [["docker compose rm -s -f walarchive"]])

    def test_crash_function_uses_the_runbook_walarchive_delete_command(self):
        step = self.runbook.runbook_step("Recover from the WAL archive", "4. ")
        blocks = self.runbook.stripped_bash_blocks(step)
        self.assertEqual(len(blocks), 1)
        self.assertEqual(len(blocks[0]), 1)
        command = re.escape(blocks[0][0].removeprefix("docker "))
        self.assertRegex(shell_function("crash"), rf"(?m)^\s*{command}(?:\s*\|\|[^\n]+)?$",
                         "crash() must execute the recovery step 4 command")

    def test_step_4_offers_walarchive_log_copy(self):
        step = self.runbook.runbook_step("Recover from the WAL archive", "4. ")
        self.assertIn("docker compose logs --no-color walarchive > <file>",
                      self.runbook.runbook_inline_code_spans(step))

    def test_step_4_log_copy_precedes_walarchive_delete(self):
        step = self.runbook.runbook_step("Recover from the WAL archive", "4. ")
        log = "docker compose logs --no-color walarchive > <file>"
        blocks = self.runbook.runbook_bash_blocks(step)
        self.assertIn(log, self.runbook.runbook_inline_code_spans(step))
        self.assertEqual(self.runbook.stripped_bash_blocks(step), [["docker compose rm -s -f walarchive"]])
        self.assertLess(step.index(log), step.index(blocks[0]),
                        "recovery step 4 must copy walarchive logs before the delete block")

    def test_step_7_recreates_walarchive_after_a_failed_attempt(self):
        step = self.runbook.runbook_step("Recover from the WAL archive", "7. Compare")
        self.assertIn("docker compose up -d --no-deps walarchive", self.runbook.runbook_inline_code_spans(step))
        self.assertNotIn("Start `walarchive`, if you stopped it in step 4", step)

    def test_step_7_resets_the_tag_before_recreating_walarchive(self):
        step = self.runbook.runbook_step("Recover from the WAL archive", "7. Compare")
        text = " ".join(step.replace("`", "").split())
        self.assertIn("If <recovery tag> differs from <tag>,", text)
        markers = ("change into the directory deploy/ of <tag>",
                   "Set RECIPE_VERSION in .env to <tag>",
                   "docker compose up -d --no-deps walarchive")
        positions = [text.find(marker) for marker in markers]
        self.assertNotIn(-1, positions, "failed recovery must reset the directory and tag before creating walarchive")
        self.assertEqual(positions, sorted(positions), "failed recovery must reset the directory and tag before creating walarchive")

    def test_step_8_ends_the_attempt_for_a_drill_volume(self):
        step = self.runbook.runbook_step("Recover from the WAL archive", "8. Run the switch")
        self.assertIn("This volume is a drill volume. Do not switch. End the attempt as in step 7.", step)
        self.assertNotIn("Stop. This volume is a drill volume.", step)

    def test_section_has_no_start_of_a_deleted_service(self):
        section = self.runbook.runbook_heading_section("Recover from the WAL archive", 3)
        self.assertEqual(self.runbook.runbook_deleted_service_starts(section, ("app", "backup", "walarchive")), [],
                         "recovery section must use up -d --no-deps for app, backup, and walarchive after the delete")

    def test_recovery_window_keeps_app_and_backup_without_a_container(self):
        section = self.runbook.runbook_heading_section("Recover from the WAL archive", 3)
        steps = {}
        for match in re.finditer(r"(?ms)^(\d+)\. .*?(?=^\d+\. |\Z)", section):
            number = match.group(1)
            self.assertNotIn(number, steps, f"duplicate recovery step {number}")
            steps[number] = match.group(0)
        guard = self.runbook.runbook_deleted_service_starts
        subcommands = ("up", "create", "run", "start", "restart")
        for number in range(1, 12):
            self.assertIn(str(number), steps, f"recovery step {number} must exist")
        for number in range(1, 11):
            self.assertEqual(guard(steps[str(number)], subcommands=subcommands), [],
                             f"recovery step {number} must keep app and backup without a container")
        step = steps["11"]
        matches = list(re.finditer(r"(?ms)^    2\. .*?(?=^    \d+\. |\Z)", step))
        self.assertEqual(len(matches), 1, "recovery step 11 must have one sub-step 2")
        substep = matches[0].group(0)
        reference = 'Run step 9 of "Restore into production".'
        self.assertEqual(substep.count(reference), 1,
                         "recovery sub-step 11.2 must have one reference to restore step 9")
        end = matches[0].start() + substep.index(reference)
        self.assertEqual(guard(step[:end], subcommands=subcommands), [],
                         "recovery step 11 must keep app and backup without a container before the reference")

    def test_recovery_window_guard_rejects_duplicate_step_numbers(self):
        section = self.runbook.runbook_heading_section("Recover from the WAL archive", 3)
        anchor = "5. Create the new volume."
        self.assertEqual(section.count(anchor), 1)
        mutant = section.replace(
            anchor, "5. Run `docker compose up -d --no-deps app`.\n" + anchor, 1)
        with patch.object(self.runbook, "runbook_heading_section", return_value=mutant):
            with self.assertRaisesRegex(AssertionError, r"duplicate recovery step 5\b"):
                self.test_recovery_window_keeps_app_and_backup_without_a_container()

    def test_recovery_window_guard_rejects_runbook_mutants(self):
        section = self.runbook.runbook_heading_section("Recover from the WAL archive", 3)
        cases = (
            ("t02_runbook_recovery_step5", "5. Create the new volume.",
             " Run `docker compose up -d --no-deps app`."),
            ("t02_runbook_recovery_step11_2", "    2. ",
             "Run `docker compose up -d --no-deps app`. "),
        )
        for name, anchor, insertion in cases:
            with self.subTest(mutant=name):
                self.assertEqual(section.count(anchor), 1, anchor)
                mutant = section.replace(anchor, anchor + insertion, 1)
                with patch.object(self.runbook, "runbook_heading_section", return_value=mutant):
                    with self.assertRaisesRegex(AssertionError, re.escape("docker compose up -d --no-deps app")):
                        self.test_recovery_window_keeps_app_and_backup_without_a_container()

    def test_section_guard_rejects_service_lists_and_double_backticks(self):
        for section in ("Run `docker compose start proxy backup`.",
                        "Run `docker compose start walarchive`.",
                        "Run ``docker compose start app``.",
                        "Run `docker compose -f compose.yaml start proxy backup`.",
                        "```text\ndocker compose restart app\n```",
                        "```bash\ndocker compose start proxy app\n```"):
            with self.subTest(section=section), patch.object(
                    self.runbook, "runbook_heading_section", return_value=section), self.assertRaises(AssertionError):
                self.test_section_has_no_start_of_a_deleted_service()

    def test_section_guard_allows_bare_start_and_other_services(self):
        section = "Never run `docker compose start` here. `docker compose start proxy` names no deleted service."
        with patch.object(self.runbook, "runbook_heading_section", return_value=section):
            self.test_section_has_no_start_of_a_deleted_service()


class ComposeRecoveryTests(unittest.TestCase):
    SET = "recipe_20261030_1205_now"

    def run_recovery(self, env_extra=None):
        with tempfile.TemporaryDirectory() as name:
            directory = pathlib.Path(name)
            scripts = directory / "workspace" / "deploy" / "ci"
            scripts.mkdir(parents=True)
            for file in ("recovery.sh", "common.sh"):
                shutil.copyfile(CI / file, scripts / file)
            write_stub(scripts / "memory.sh", '''
memory_exec() { local service=$1; shift; compose exec -T "$service" "$@"; }
memory_pitr() { echo memory_pitr >>"$STUB_LOG"; }
''')
            write_stub(scripts / "modes.sh", 'echo modes >>"$STUB_LOG"')
            stubs = directory / "bin"
            stubs.mkdir()
            for file, body in (("sudo", STEP_SUDO_STUB), ("docker", STEP_DOCKER_STUB),
                               ("python3", STEP_PYTHON_STUB)):
                write_stub(stubs / file, body)
            root = directory / "runner" / PROJECT
            state = root / "state"
            state.mkdir(parents=True)
            backups = root / "backups"
            backups.mkdir()
            originals = self.write_set(backups)
            env = os.environ.copy()
            for key in [key for key in env if key.startswith(("GITHUB_", "RUNNER_", "COMPOSE_", "STUB_"))]:
                del env[key]
            env.update({"GITHUB_ACTIONS": "true", "GITHUB_WORKSPACE": str(directory / "workspace"),
                        "RUNNER_TEMP": str(directory / "runner"), "COMPOSE_PROJECT_NAME": PROJECT,
                        "PATH": f"{stubs}{os.pathsep}{env['PATH']}", "STUB_LOG": str(directory / "calls.log"),
                        "STUB_COMPOSE_RECOVERY": "1"})
            env.update(env_extra or {})
            done = subprocess.run([shutil.which("bash"), str(scripts / "recovery.sh")], env=env,
                                  capture_output=True, text=True, check=False, timeout=15)
            remaining = {path.name: path.read_bytes() for path in backups.iterdir() if path.is_file()}
            copied = {path.name: path.read_bytes() for path in (state / "copied").glob("*")}
            return done, pathlib.Path(env["STUB_LOG"]).read_text(), remaining, originals, copied

    def write_set(self, backups):
        files = {f"{self.SET}.dump": b"PGDMP fixture dump\n", f"{self.SET}.counts": b"public.recipe|1\n",
                 "globals_20261030_1205_now.sql": b"-- fixture globals\n"}
        manifest = "".join(f"{hashlib.sha256(files[file]).hexdigest()}  {file}\n"
                           for file in (f"{self.SET}.dump", "globals_20261030_1205_now.sql", f"{self.SET}.counts"))
        files[f"{self.SET}.dump.sha256"] = manifest.encode()
        # This opaque tag fixture contains no key. The pgtools suites check the cryptography.
        files[f"{self.SET}.dump.sha256.hmac"] = b"unchanged fixture tag\n"
        for file, content in files.items():
            (backups / file).write_bytes(content)
            (backups / file).chmod(0o600)
        return files

    def assert_copy_deleted(self, remaining, originals):
        self.assertEqual({name: data for name, data in remaining.items() if name != "restore-test.result"}, originals)

    def test_pass_logs_the_tag_file_and_real_restore_of_the_now_set(self):
        done, log, remaining, originals, _ = self.run_recovery()
        self.assertEqual(done.returncode, 0, done.stderr + log)
        self.assertIn(f"{self.SET}.dump.sha256.hmac", done.stdout)
        self.assertIn(f"real pg_restore passed for {self.SET}", done.stdout)
        self.assertIn(f'"message":"restore test passed","set":"{self.SET}"', done.stdout)
        self.assert_copy_deleted(remaining, originals)

    def test_refusal_logs_the_d4_message_and_failed_result(self):
        done, log, remaining, originals, _ = self.run_recovery()
        self.assertEqual(done.returncode, 0, done.stderr + log)
        self.assertEqual(log.count("exec -T backup restore-test\n"), 2, log)
        self.assertIn('"level":"ERROR","job":"restore-test","message":"the tag of the set does not match the manifest"',
                      done.stdout)
        self.assertNotIn('"fault"', done.stdout)
        self.assertIn('"result":"failed"', done.stdout)
        self.assertIn(b'"result":"failed"', remaining["restore-test.result"])
        self.assert_copy_deleted(remaining, originals)
        self.assertLess(log.index("exec -T backup restore-test\n", log.index("exec -T backup restore-test\n") + 1),
                        log.index("exec -T walarchive basebackup-once\n"))

    def test_copy_changes_one_dump_byte_and_rebuilds_the_manifest_with_the_old_tag(self):
        done, log, _, originals, copied = self.run_recovery()
        self.assertEqual(done.returncode, 0, done.stderr + log)
        manifests = sorted(name for name in copied if name.endswith(".dump.sha256"))
        self.assertEqual(len(manifests), 2, log)
        manifest = manifests[-1]
        self.assertGreater(manifest, f"{self.SET}.dump.sha256")
        changed = copied[manifest.removesuffix(".sha256")]
        original = originals[f"{self.SET}.dump"]
        self.assertEqual(len(changed), len(original))
        self.assertEqual(sum(left != right for left, right in zip(changed, original)), 1)
        self.assertEqual(copied[manifest + ".hmac"], originals[f"{self.SET}.dump.sha256.hmac"])
        entries = [line.split("  ") for line in copied[manifest].decode().splitlines()]
        base = manifest.removesuffix(".dump.sha256")
        self.assertEqual([file for _, file in entries],
                         [f"{base}.dump", f"globals_{base.removeprefix('recipe_')}.sql", f"{base}.counts"])
        for digest, file in entries:
            self.assertEqual(hashlib.sha256(copied[file]).hexdigest(), digest)
        for file in [file for _, file in entries] + [manifest, manifest + ".hmac"]:
            self.assertRegex(log, rf"sudo install -o 999 -g 999 -m 0600 .* /\S+/{re.escape(file)}\n")

    def test_pass_requires_a_successful_restore_of_the_now_set(self):
        cases = ({"STUB_RESTORE_TEST_OUTPUT": ""}, {"STUB_RESTORE_TEST_STATUS": "1"},
                 {"STUB_RESTORE_TEST_OUTPUT": '{"message":"restore test passed","set":"recipe_20261030_1205"}'})
        for env in cases:
            with self.subTest(env=env):
                done, log, remaining, originals, copied = self.run_recovery(env)
                self.assertNotEqual(done.returncode, 0, done.stdout + log)
                self.assertNotIn("real pg_restore passed", done.stdout)
                self.assertNotIn("sudo install ", log)
                self.assertFalse(copied)
                self.assert_copy_deleted(remaining, originals)

    def test_missing_tag_stops_before_the_copy(self):
        done, log, remaining, originals, copied = self.run_recovery({"STUB_NO_TAG": "1"})
        self.assertNotEqual(done.returncode, 0, done.stdout + log)
        self.assertIn("tag file", done.stderr)
        self.assertNotIn("sudo install ", log)
        self.assertFalse(copied)
        self.assert_copy_deleted(remaining, originals)

    def test_refusal_requires_the_d4_error_for_the_copied_set(self):
        cases = ({"STUB_TAG_ERROR": "the newest set has no tag file"},
                 {"STUB_TAG_ERROR": "the manifest of the newest set does not verify"},
                 {"STUB_TAG_ERROR_LEVEL": "INFO"}, {"STUB_TAG_ERROR_SET": self.SET},
                 {"STUB_TAG_ERROR_EXTRA": ',"fault":"tag-mismatch"'}, {"STUB_TAG_ERROR_COUNT": "2"},
                 {"STUB_TAG_ERROR_COUNT": "0"}, {"STUB_REFUSAL_STATUS": "0"})
        for env in cases:
            with self.subTest(env=env):
                done, log, remaining, originals, _ = self.run_recovery(env)
                self.assertNotEqual(done.returncode, 0, done.stdout + log)
                self.assertNotIn("exec -T walarchive basebackup-once", log)
                self.assert_copy_deleted(remaining, originals)

    def test_refusal_requires_the_failed_result(self):
        for result in ("passed", "unknown", "missing"):
            with self.subTest(result=result):
                done, log, remaining, originals, _ = self.run_recovery({"STUB_TAG_RESULT": result})
                self.assertNotEqual(done.returncode, 0, done.stdout + log)
                self.assertNotIn("exec -T walarchive basebackup-once", log)
                self.assert_copy_deleted(remaining, originals)

    def test_failed_copy_deletes_every_partial_file(self):
        done, log, remaining, originals, _ = self.run_recovery({"STUB_INSTALL_FAIL_AT": "3"})
        self.assertNotEqual(done.returncode, 0, done.stdout + log)
        self.assertIn("sudo rm -f -- ", log)
        self.assert_copy_deleted(remaining, originals)

    def test_cleanup_refuses_a_copy_left_behind(self):
        for status in ("0", "1"):
            with self.subTest(status=status):
                done, log, remaining, originals, _ = self.run_recovery({"STUB_COPY_STAYS": status})
                self.assertNotEqual(done.returncode, 0, done.stdout + log)
                self.assertIn("cannot delete the set copy", done.stderr)
                self.assertNotIn("exec -T walarchive basebackup-once", log)
                self.assertGreater(len(remaining), len(originals))


STEP_SUDO_STUB = """
while [[ ${1:-} == --* ]]; do shift; done
if [[ ${1:-} == docker ]]; then exec "$@"; fi
echo "sudo $*" >>"$STUB_LOG"
if [[ ${STUB_COMPOSE_RECOVERY:-} == 1 ]]; then
  case "$*" in
    "install "*)
      count=$(grep -c '^sudo install ' "$STUB_LOG")
      [[ $count != "${STUB_INSTALL_FAIL_AT:-0}" ]] || { echo 'install: cannot copy file' >&2; exit 1; }
      args=()
      while (( $# )); do
        case $1 in -o|-g) shift 2 ;; *) args+=("$1"); shift ;; esac
      done
      exec "${args[@]}" ;;
    "rm "*) if [[ -n ${STUB_COPY_STAYS:-} ]]; then exit "$STUB_COPY_STAYS"; fi; exec "$@" ;;
    "test -f "*.hmac) if [[ ${STUB_NO_TAG:-0} != 0 ]]; then exit "$STUB_NO_TAG"; fi; exec "$@" ;;
    "test "*"/backups/"*|"cat "*|"bash "*|"dd "*) exec "$@" ;;
  esac
fi
case "$*" in
  "cp -a -- "*"/state/wal-copy/")
    if [[ $(grep -c '^sudo cp -a -- ' "$STUB_LOG") == 2 && ${STUB_EARLY_END_SEGMENT_MISSING:-0} == 0 ]]; then
      dir=${@: -1}
      mkdir -p -- "$dir"
      partial="${dir}000000010000000000000006.partial"
      if [[ ${STUB_EARLY_END_SEGMENT_COMPLETE:-0} == 1 ]]; then
        : >"${partial%.partial}"
      fi
      for offset in 96 8168 8169 8176 859104; do
        printf '\\022' | dd of="$partial" bs=1 seek="$offset" conv=notrunc status=none || exit 1
        printf '\\245' | dd of="$partial" bs=1 seek="$((offset + 20))" conv=notrunc status=none || exit 1
      done
    fi ;;
  "test -f "*"/state/wal-copy/"*.partial|"test -e "*"/state/wal-copy/"[0-9]*) exec "$@" ;;
  "od "*"/state/wal-copy/"*.partial) exec "$@" ;;
  "dd of="*"/state/wal-copy/"*.partial*)
    if [[ ${STUB_EARLY_END_SKIP_DD:-0} == 1 ]]; then exit 0; fi
    exec "$@" ;;
  "rm -rf -- "*"/state/wal-copy") exec "$@" ;;
  "test -f "*.dump.sha256) exit 1 ;;
  "test -f "*) exit "${STUB_PARTIAL_STATUS:-0}" ;;
  "test -e "*) exit "${STUB_COMPLETE_STATUS:-1}" ;;
  "find "*".moved-"*" -mindepth 1")
    # The listing of the target directory of the move. The step refuse compares it with the manifest of wal-line.
    if [[ -n ${STUB_TARGET_MANIFEST:-} ]]; then printf '%s\\n' "$STUB_TARGET_MANIFEST"; fi ;;
  "find "*".moved-"*"-maxdepth 1 -type f"*)
    # The segments of the old timeline in the target directory of the move.
    printf '%s' "${STUB_OLD_SEGMENTS-000000010000000000000002
}" ;;
  "find "*"/base -mindepth"*)
    if [[ ${STUB_EARLY_END_NO_BASE:-0} == 1 && $2 == */state/wal-copy/base ]] &&
      [[ $(grep -c '^sudo cp -a -- ' "$STUB_LOG") == 2 ]]; then exit 0; fi
    # The move of the base backups empties the list.
    if ! grep -q '^sudo mv -t [^ ]*/base -- ' "$STUB_LOG"; then printf '%s\\n' "${STUB_BASES:-base_20261030_115900}"; fi ;;
  "find "*".history"*)
    # The move of the segment files takes the history files too.
    if ! grep -q '^sudo mv -t [^ ]*moved-[0-9_]* -- ' "$STUB_LOG"; then printf '%s\\n' "${STUB_HISTORY-00000002.history}"; fi ;;
  "find "*"-maxdepth 1 -type f"*)
    # STUB_COPY_SEGMENTS gives the listing for the gap run. The next copy has the listing of the archive.
    if [[ $* == *"/wal-copy -maxdepth"* && -n ${STUB_COPY_SEGMENTS:-} ]] && \
      [[ $(grep -c '^sudo cp -a -- ' "$STUB_LOG") == 1 ]]; then printf '%s' "$STUB_COPY_SEGMENTS"
    elif ! grep -q '^sudo mv -t [^ ]*moved-[0-9_]* -- ' "$STUB_LOG"; then printf '%s' "${STUB_SEGMENTS-000000010000000000000002
000000010000000000000003.partial
}"; fi ;;
  "tee "*) cat >"$CI_STATE/stub.old-system-identifier" ;;
  "grep "*) shift; exec grep "$@" ;;
  "cat "*)
    if [[ -n ${STUB_NEW_ARCHIVE_ID:-} ]] && grep -q 'up -d --no-deps walarchive' "$STUB_LOG"; then echo "$STUB_NEW_ARCHIVE_ID"
    else echo "${STUB_ARCHIVE_ID:-7234567890123456789}"; fi ;;
  "sed "*"backup_label") printf '%s\\n' "${STUB_START_SEGMENT:-000000010000000000000002}" ;;
esac
exit 0
"""

# The stub of docker answers the queries of db by their text. The marker row depends on the number of password commands so far.
STEP_DOCKER_STUB = """
printf '%s\\0' "$#" "$@" >>"$STUB_LOG.argv"
RESTORE_TEST_LINE='{"timestamp":"2026-10-30T12:05:00Z","level":"INFO","job":"restore-test","message":"restore test passed","set":"recipe_20261030_1205_now"}'
echo "$*" >>"$STUB_LOG"
case "$*" in
  *"--entrypoint bash backup-premigrate -c "*)
    # The probe of the login roles. The roles follow the word probe. The text of the script holds the word psql, so this case comes first.
    if [[ ${STUB_PROBE_STATUS:-0} != 0 ]]; then exit "$STUB_PROBE_STATUS"; fi
    seen=0
    for arg in "$@"; do
      if [[ $seen == 1 && $arg != "${STUB_PROBE_SKIP_ROLE:-}" ]]; then
        if [[ $arg == "${STUB_PROBE_BAD_ROLE:-}" ]]; then echo "$arg|${STUB_PROBE_BAD_RESULT:-password}"; else echo "$arg|ok"; fi
      fi
      if [[ $arg == probe ]]; then seen=1; fi
    done ;;
  *"psql "*)
    case "$*" in
      *"--no-psqlrc --dbname postgres"*) cat >/dev/null; exit "${STUB_PASSWORDS_STATUS:-0}" ;;
      *"SELECT state, flush_lsn"*) echo "${STUB_STREAM_ROW-streaming|0/4000000}" ;;
      *"pg_switch_wal()"*) echo 0/4000000 ;;
      *"ORDER BY chain_seq DESC LIMIT 1"*)
        if [[ -n ${STUB_ROW:-} ]]; then echo "$STUB_ROW"; exit 0; fi
        count=$(grep -c 'http_probe.py password$' "$STUB_LOG" || true)
        if [[ $count -ge 2 ]]; then echo "42|bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb|${STUB_ACTION:-OwnPasswordChanged}"
        else echo "41|aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa|${STUB_ACTION:-OwnPasswordChanged}"; fi ;;
      *"date_trunc('second'"*) echo "${STUB_TARGET:-2026-10-30T12:00:05Z}" ;;
      *"heartbeat.last_at"*)
        # The row exists once the number of queries is above STUB_HEARTBEAT_MISSES.
        count=$(grep -c 'heartbeat.last_at' "$STUB_LOG" || true)
        if [[ $count -gt ${STUB_HEARTBEAT_MISSES:-0} ]]; then echo 1; else echo 0; fi ;;
      *"clock_timestamp() >="*) echo t ;;
      *"occurred_at"*) echo "${STUB_BETWEEN:-t}" ;;
      *"SELECT pg_current_wal_lsn()"*) echo "${STUB_LSN:-0/3000060}" ;;
      *"flush_lsn >="*) echo "${STUB_FLUSH:-t}" ;;
      *"pg_walfile_name_offset"*) echo "$STUB_EARLY_END_POSITION" ;;
      *"pg_walfile_name"*) echo "${STUB_SEGMENT:-000000010000000000000003}" ;;
    esac ;;
  *"--entrypoint pg_controldata"*)
    # The old volume lost global/pg_control. The new volume is a shut down cluster of the line of pitr-volume.
    if [[ $* == *"-v $(<"$CI_STATE/new.volume"):"* ]]; then
      echo "Database cluster state:               ${STUB_NEW_STATE:-shut down}"
      echo "Database system identifier:           ${STUB_NEW_IDENTIFIER:-7234567890123456789}"
      echo "Latest checkpoint's TimeLineID:       ${STUB_NEW_TIMELINE:-2}"
    else exit "${STUB_OLD_CONTROL_STATUS:-1}"; fi ;;
  *"exec -T walarchive basebackup-once")
    printf '%s\\n' '{"timestamp":"2026-10-30T12:02:00Z","level":"INFO","job":"basebackup-once","message":"base backup completed","base":"base_20261030_120200"}'
    exit "${STUB_BASEBACKUP_STATUS:-0}" ;;
  *"exec -T proxy wget"*) printf '%s\n' "${STUB_READYZ-ok}" ;;
  *" logs --no-color --no-log-prefix db")
    # The log of db. STUB_DB_LOG replaces the clean line. An empty STUB_DB_LOG gives an empty log.
    if [[ -n ${STUB_DB_LOG-LOG:  database system is ready to accept connections} ]]; then
      printf '%s\n' "${STUB_DB_LOG-LOG:  database system is ready to accept connections}"
    fi ;;
  *"exec -T backup backup-now") exit "${STUB_BACKUP_NOW_STATUS:-0}" ;;
  *"exec -T backup restore-test")
    if [[ ${STUB_COMPOSE_RECOVERY:-} == 1 ]] && [[ $(grep -c 'exec -T backup restore-test$' "$STUB_LOG") -eq 2 ]]; then
      mkdir "$CI_STATE/copied"
      cp -- "$RECIPE_BACKUP_DIR/"* "$CI_STATE/copied/"
      manifests=("$RECIPE_BACKUP_DIR/"*.dump.sha256)
      newest=${manifests[${#manifests[@]}-1]##*/}
      set=${newest%.dump.sha256}
      for ((i=0; i<${STUB_TAG_ERROR_COUNT:-1}; i++)); do
        printf '{"level":"%s","job":"restore-test","message":"%s","set":"%s"%s}\\n' \\
          "${STUB_TAG_ERROR_LEVEL:-ERROR}" "${STUB_TAG_ERROR:-the tag of the set does not match the manifest}" \\
          "${STUB_TAG_ERROR_SET:-$set}" "${STUB_TAG_ERROR_EXTRA:-}"
      done
      if [[ ${STUB_TAG_RESULT:-failed} != missing ]]; then
        printf '{"result":"%s","finished_at":"2026-10-30T12:06:00Z"}\\n' "${STUB_TAG_RESULT:-failed}" >"$RECIPE_BACKUP_DIR/restore-test.result"
      fi
      exit "${STUB_REFUSAL_STATUS:-1}"
    fi
    printf '%s\n' "${STUB_RESTORE_TEST_OUTPUT-$RESTORE_TEST_LINE}"
    exit "${STUB_RESTORE_TEST_STATUS:-0}" ;;
  *" -e PITR_CI=1 "*" pitr-check")
    echo "PITR_MARKER_SEQ=${PITR_MARKER_SEQ:-} PITR_MARKER_HASH=${PITR_MARKER_HASH:-}" >>"$STUB_LOG"
    exit "${STUB_PITR_CHECK_STATUS:-0}" ;;
  *" ps -aq walarchive") if [[ -n ${STUB_WALARCHIVE_PS:-} ]]; then echo "$STUB_WALARCHIVE_PS"; fi ;;
  "kill "*) exit "${STUB_DOCKER_KILL_STATUS:-1}" ;;
  *"--entrypoint rm"*) exit "${STUB_RM_STATUS:-0}" ;;
  *"--entrypoint test"*) exit "${STUB_DRILL_TEST_STATUS:-0}" ;;
  *":/wal walarchive")
    # The guard run of the step refuse. walarchive-entry gets the copy of /wal.
    printf '%s\\n' "$STUB_REFUSE_OUTPUT"
    exit "${STUB_REFUSE_STATUS:-1}" ;;
  *":/wal:ro pitr-volume"*)
    if [[ $(grep -c ':/wal:ro pitr-volume' "$STUB_LOG") == 2 ]]; then
      if [[ -n $STUB_EARLY_END_OUTPUT ]]; then printf '%s\\n' "$STUB_EARLY_END_OUTPUT"; fi
      exit "${STUB_EARLY_END_STATUS:-0}"
    fi
    printf '%s\\n' "$STUB_GAP_OUTPUT"
    exit "${STUB_GAP_STATUS:-1}" ;;
  *"-e PITR_EXPECTED_LSN="*"pitr-volume")
    echo "PITR_MARKER_SEQ=${PITR_MARKER_SEQ:-} PITR_MARKER_HASH=${PITR_MARKER_HASH:-}" >>"$STUB_LOG"
    printf '%s\\n' "$STUB_RECOVER_OUTPUT"
    exit "${STUB_RECOVER_STATUS:-0}" ;;
  *" rm -s -f app backup") exit "${STUB_REMOVE_APP_STATUS:-0}" ;;
  *" rm -s -f walarchive") exit "${STUB_REMOVE_WALARCHIVE_STATUS:-0}" ;;
  *" kill -s SIGKILL db") exit "${STUB_KILL_STATUS:-0}" ;;
  "inspect --format {{.State.Running}} {{.State.ExitCode}} "*) echo "${STUB_DB_STATE:-false 137}" ;;
  "inspect --format {{.State.Running}} "*) echo "${STUB_DB_RUNNING:-false}" ;;
  *"pitr-volume --target-time"*"--drill"*)
    echo "PITR_MARKER_SEQ=${PITR_MARKER_SEQ:-} PITR_MARKER_HASH=${PITR_MARKER_HASH:-}" >>"$STUB_LOG"
    printf '%s\\n' "$STUB_PITR_OUTPUT"
    exit "${STUB_PITR_STATUS:-0}" ;;
  *"pitr-volume --target-time"*)
    printf '%s\\n' "$STUB_TARGET_OUTPUT"
    exit "${STUB_TARGET_STATUS:-1}" ;;
  *" ps -aq db") echo 0123abcd ;;
  "inspect --format {{range .Mounts}}"*)
    # After db starts on the new volume, the mount is the new volume.
    if grep -q 'up -d --no-deps --wait --wait-timeout 180 db$' "$STUB_LOG"; then cat "$CI_STATE/new.volume"; exit 0; fi
    # The second read of the data volume of db differs when STUB_PROD_CHANGE is volume.
    count=$(grep -c '^inspect --format {{range .Mounts}}' "$STUB_LOG" || true)
    if [[ ${STUB_PROD_CHANGE:-} == volume && $count -ge 2 ]]; then echo other_volume; else echo "${STUB_DB_VOLUME:-recipe-ci-1-1_pgdata}"; fi ;;
  "inspect --format container "*)
    count=$(grep -c '^inspect --format container ' "$STUB_LOG" || true)
    if [[ ${STUB_PROD_CHANGE:-} == container && $count -ge 2 ]]; then echo "container 0123abcd 2026-10-30T12:09:00Z"; else echo "container 0123abcd 2026-10-30T12:00:00Z"; fi ;;
  "volume ls "*)
    # A scratch volume leaves the list after volume rm, unless STUB_VOLUME_STAYS is set.
    echo recipe-ci-1-1_pgdata
    for name in $(grep '^volume create ' "$STUB_LOG" | awk '{print $NF}'); do
      if [[ -n ${STUB_VOLUME_STAYS:-} ]] || ! grep -q "^volume rm $name\\$" "$STUB_LOG"; then echo "$name"; fi
    done ;;
  "volume create "*) exit "${STUB_VOLUME_CREATE_STATUS:-0}" ;;
  "volume inspect "*) echo "${STUB_LABEL:-recipe-ci-1-1}" ;;
  "volume rm "*) exit "${STUB_VOLUME_RM_STATUS:-0}" ;;
esac
exit 0
"""

STEP_PYTHON_STUB = """
echo "python3 $*" >>"$STUB_LOG"
if [[ $* == *" volume-name "* ]]; then echo "${STUB_COMPOSE_VOLUME:-$(<"$CI_STATE/new.volume")}"; fi
if [[ ${*: -1} == password ]]; then
  echo "passwords current=$(<"$CI_STATE/admin_password") next=$(<"$CI_STATE/new_password")" >>"$STUB_LOG"
  count=$(grep -c 'http_probe.py password$' "$STUB_LOG" || true)
  if [[ $count == "${STUB_HTTP_FAIL_AT:-0}" ]]; then exit 1; fi
fi
if [[ ${*: -1} == relogin ]]; then
  echo "relogin password=$(<"$CI_STATE/new_password")" >>"$STUB_LOG"
  if [[ -n ${STUB_ACCEPTED_PASSWORD:-} && $(<"$CI_STATE/new_password") != "$STUB_ACCEPTED_PASSWORD" ]]; then exit 1; fi
fi
exit 0
"""

STEP_OPENSSL_STUB = """
count=$(cat "$STUB_COUNTER" 2>/dev/null || echo 0)
count=$((count + 1))
echo "$count" >"$STUB_COUNTER"
printf '%064d\\n' "$count"
"""


if __name__ == "__main__":
    unittest.main()
