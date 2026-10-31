"""Check the job restore-switch and its script without Docker."""

import importlib.util
import json
import os
import pathlib
import re
import shutil
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import Mock


CI = pathlib.Path(__file__).resolve().parent
ROOT = CI.parent.parent
SCRIPT = CI / "restore-switch.sh"
SWITCH_STEPS = CI / "switch-steps.sh"
PROJECT = "recipe-ci-1-1"
FIXTURE_HASH = "ab" * 32


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


def script_text():
    return SCRIPT.read_text(encoding="utf-8")


def switch_steps_text():
    return SWITCH_STEPS.read_text(encoding="utf-8") if SWITCH_STEPS.is_file() else ""


def shell_function(name):
    """Return the body of one function of restore-switch.sh or of switch-steps.sh."""
    for text in (script_text(), switch_steps_text()):
        if f"\n{name}() {{\n" in text:
            return text.split(f"\n{name}() {{\n", 1)[1].split("\n}\n", 1)[0]
    raise KeyError(name)


class WorkflowTests(unittest.TestCase):
    def setUp(self):
        self.body = job_body("restore-switch")
        if not self.body:
            self.fail("ci.yml has no job restore-switch")

    def test_trigger_matches_the_job_compose(self):
        self.assertIn("if: github.event_name != 'pull_request'", self.body)
        self.assertIn("runs-on: ubuntu-24.04", self.body)

    def test_job_has_its_own_project_and_image_tag(self):
        self.assertIn("COMPOSE_PROJECT_NAME: recipe-ci-${{ github.run_id }}-${{ github.run_attempt }}", self.body)
        self.assertIn("RECIPE_VERSION: ci-restore-switch", self.body)

    def test_job_builds_both_images_with_the_db_context(self):
        self.assertIn("tags: recipe-server:ci-restore-switch", self.body)
        self.assertIn("tags: recipe-pgtools:ci-restore-switch", self.body)
        self.assertIn("build-contexts: db=deploy/db", self.body)

    def test_job_runs_only_the_subcommands_that_exist(self):
        calls = re.findall(r"restore-switch\.sh (\S+)", self.body)
        self.assertEqual(calls, ["prepare", "restore", "switch", "access", "wal-line", "check", "refuse", "switch-back", "keep", "cleanup"])

    def test_cleanup_runs_always_and_diagnostics_only_on_failure(self):
        steps = self.body.split("      - name: ")
        cleanup = [step for step in steps if "restore-switch.sh cleanup" in step]
        self.assertEqual(len(cleanup), 1)
        self.assertIn("if: always()", cleanup[0])
        diagnostics = [step for step in steps if "diagnostics.sh" in step]
        self.assertEqual(len(diagnostics), 1)
        self.assertIn("if: failure()", diagnostics[0])

    def test_every_step_has_a_timeout_below_the_job_limit(self):
        limit = int(re.search(r"^    timeout-minutes: (\d+)$", self.body, re.M).group(1))
        steps = [int(value) for value in re.findall(r"^        timeout-minutes: (\d+)$", self.body, re.M)]
        self.assertTrue(steps)
        self.assertLess(sum(steps), limit)

    def test_the_budget_comment_names_the_sum_of_the_step_limits(self):
        # The comment counts the step limits, the diagnostics, and the cleanup. The sum stays below the job limit.
        steps = [int(value) for value in re.findall(r"^        timeout-minutes: (\d+)$", self.body, re.M)]
        named = re.search(r"step limits sum to (\d+) minutes", self.body)
        self.assertIsNotNone(named)
        self.assertEqual(int(named.group(1)), sum(steps))

    def test_job_checks_the_ci_scripts_and_prepares_the_runner(self):
        self.assertIn("run: bash deploy/ci/check.sh", self.body)
        self.assertIn("run: bash deploy/ci/prepare.sh", self.body)


class ScriptTests(unittest.TestCase):
    def setUp(self):
        if not SCRIPT.is_file():
            self.fail("deploy/ci/restore-switch.sh is missing")
        self.bash = shutil.which("bash")
        if self.bash is None:
            self.skipTest("bash is not available")

    def test_syntax(self):
        done = subprocess.run([self.bash, "-n", str(SCRIPT)], capture_output=True, check=False)
        self.assertEqual(done.returncode, 0, done.stderr)

    def test_file_uses_lf(self):
        self.assertNotIn(b"\r", SCRIPT.read_bytes())

    def test_prepare_runs_the_steps_in_order(self):
        body = shell_function("prepare")
        markers = [
            "start.sh",
            "login.sh",
            "http_probe password",
            "upgrade_smoke.py",
            "http_probe export",
            "row-counts",
            "backup-now",
            "restore-test",
        ]
        positions = [body.find(marker) for marker in markers]
        self.assertNotIn(-1, positions, dict(zip(markers, positions)))
        self.assertEqual(positions, sorted(positions), dict(zip(markers, positions)))

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

    def test_unknown_subcommand_fails_with_the_usage_line(self):
        done = self.run_script("other")
        self.assertEqual(done.returncode, 1)
        self.assertIn("CI: usage: restore-switch.sh prepare|restore|switch|access|wal-line|check|refuse|switch-back|keep|cleanup", done.stderr)

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

    def environment(self, directory):
        env = {key: value for key, value in os.environ.items() if not key.startswith(("GITHUB_", "RUNNER_", "COMPOSE_"))}
        env.update({
            "GITHUB_ACTIONS": "true",
            "GITHUB_WORKSPACE": str(ROOT),
            "RUNNER_TEMP": str(directory / "runner"),
            "COMPOSE_PROJECT_NAME": PROJECT,
        })
        return env

    def run_script(self, *arguments, env=None):
        with tempfile.TemporaryDirectory() as name:
            done = subprocess.run([self.bash, str(SCRIPT), *arguments], capture_output=True, text=True,
                                  check=False, env=env or self.environment(pathlib.Path(name)))
        return done

    def run_cleanup(self, down_status, volume_stays=False):
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
            done = subprocess.run([self.bash, str(SCRIPT), "cleanup"], capture_output=True, text=True,
                                  check=False, env=env)
            return done, log.read_text(encoding="utf-8") if log.exists() else ""


SUDO_STUB = """
while [[ ${1:-} == --* ]]; do shift; done
if [[ ${1:-} == docker ]]; then exec "$@"; fi
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


def write_stub(path, body):
    path.write_text("#!/usr/bin/env bash\n" + body.lstrip("\n"), encoding="utf-8", newline="\n")
    path.chmod(0o755)


class RestoreSwitchWorkflowOrderTests(unittest.TestCase):
    def test_restore_and_switch_run_after_prepare_and_before_diagnostics(self):
        body = job_body("restore-switch")
        names = ("prepare", "restore", "switch", "access", "wal-line", "check", "refuse", "switch-back", "keep", "cleanup")
        order = [body.find(f"restore-switch.sh {name}\n") for name in names]
        order.insert(9, body.find("diagnostics.sh"))
        self.assertNotIn(-1, order)
        self.assertEqual(order, sorted(order))

    def test_restore_and_switch_steps_have_timeouts(self):
        steps = job_body("restore-switch").split("      - name: ")
        for name in ("restore", "switch", "access", "wal-line", "check", "refuse", "switch-back", "keep"):
            with self.subTest(name=name):
                step = [text for text in steps if f"restore-switch.sh {name}\n" in text + "\n"]
                self.assertEqual(len(step), 1)
                self.assertRegex(step[0], r"timeout-minutes: \d+")
                self.assertNotIn("if:", step[0])


class RestoreSwitchTextTests(unittest.TestCase):
    """Check the order, the commands, and the forbidden commands in the text of the script."""

    def positions(self, body, markers):
        found = [body.find(marker) for marker in markers]
        self.assertNotIn(-1, found, dict(zip(markers, found)))
        self.assertEqual(found, sorted(found), dict(zip(markers, found)))

    def test_restore_follows_the_order_of_adr_0103_b(self):
        self.positions(shell_function("restore"), [
            "compose stop app backup",
            "docker volume create --label",
            "restore-volume",
            "new.system-identifier",
        ])

    def test_restore_mounts_the_new_volume_into_the_service(self):
        body = shell_function("restore")
        self.assertIn(':/var/lib/postgresql/data"', body)
        self.assertIn("compose run --rm --no-deps -T", body)
        self.assertNotIn("down", body)

    def test_switch_follows_the_steps_of_adr_0103_f(self):
        self.positions(shell_function("switch"), [
            "stop_services kill",
            "check_unclean_state",
            "stop_services stop",
            "change.log",
            "require_compose_volume",
            "check_new_volume",
            "start_db",
            "active.volume",
        ])
        self.assertIn("config --format json", shell_function("require_compose_volume"))
        self.assertIn("up -d --no-deps --wait", shell_function("start_db"))

    def test_switch_never_uses_start_or_down(self):
        text = "".join(shell_function(name) for name in (
            "switch", "stop_services", "restart_old_db", "require_compose_volume", "start_db"))
        self.assertNotRegex(text, r"compose (\S+ )*(start|down)\b")
        self.assertNotRegex(text, r"docker volume (rm|prune)")

    def test_stop_services_kills_db_and_removes_the_containers(self):
        body = shell_function("stop_services")
        self.assertIn('docker kill "$(container db)"', body)
        self.assertIn("rm -f proxy app backup walarchive db", body)

    def test_stop_services_accepts_a_failed_kill_only_when_db_does_not_run(self):
        # The recovery switch follows the step crash, so db is dead and docker kill fails. The restore path kills a running db as before.
        body = shell_function("stop_services")
        kill_lines = [line.strip() for line in body.splitlines() if line.strip()]
        start = kill_lines.index('docker kill "$(container db)" >/dev/null \\')
        self.assertEqual(kill_lines[start + 1:start + 3], [
            "|| [[ $(docker inspect --format '{{.State.Running}}' \"$(container db)\") == false ]] \\",
            '|| fail "switch: cannot kill db."',
        ])

    def test_wait_backup_minute_has_one_definition_in_common_sh(self):
        # A second copy of the helper can lose a suffix. The copy in restore-switch.sh lost _now.
        found = {path.name: path.read_text(encoding="utf-8").count("\nwait_backup_minute() {\n")
                 for path in sorted(CI.glob("*.sh"))}
        self.assertEqual({name: count for name, count in found.items() if count}, {"common.sh": 1})

    def test_controldata_runs_without_network_and_with_a_read_only_mount(self):
        body = shell_function("controldata")
        self.assertIn("--network none", body)
        self.assertIn(':/var/lib/postgresql/data:ro"', body)
        self.assertIn("pg_controldata", body)


class AccessTextTests(unittest.TestCase):
    """Check the order and the forbidden commands in the text of access."""

    def test_access_follows_the_order_of_adr_0103_b_step_6(self):
        body = shell_function("access")
        markers = ["active.volume", "expect_probe reject postgres", "expect_probe password", "check_db_log",
                   "set_role_passwords", "expect_probe ok"]
        found = [body.find(marker) for marker in markers]
        self.assertNotIn(-1, found, dict(zip(markers, found)))
        self.assertEqual(found, sorted(found), dict(zip(markers, found)))

    def test_access_starts_no_service_and_deletes_nothing(self):
        text = shell_function("access") + shell_function("probe_roles") + shell_function("set_role_passwords")
        self.assertNotRegex(text, r"compose (\S+ )*(start|down|rm|stop|up)\b")
        self.assertNotRegex(text, r"docker volume (rm|prune|create)")

    def test_passwords_never_reach_a_command_line(self):
        text = script_text() + switch_steps_text()
        self.assertNotRegex(text, r"PGPASSWORD|--password|password=|\s-W\b")
        for name in ("postgres", "migrator", "app", "backup", "replicator"):
            self.assertIn(f"\\set {name}_password `cat /run/secrets/{name}_password`", text)
        for statement in ("SET log_statement = 'none';", "SET log_min_error_statement = 'panic';",
                          "SET log_min_duration_statement = -1;", "ALTER ROLE recipe_app PASSWORD :'app_password';"):
            self.assertIn(statement, text)
        self.assertIn("compose exec -T -u postgres db psql -v ON_ERROR_STOP=1 --no-password --no-psqlrc",
                      shell_function("set_role_passwords"))

    def test_probe_runs_in_a_one_shot_container_on_the_backend_network(self):
        body = shell_function("probe_roles")
        self.assertIn("args=(run --rm --no-deps -T)", body)
        self.assertIn("compose \"${args[@]}\"", body)
        self.assertIn("backup-premigrate", body)
        self.assertIn("--entrypoint bash", body)
        self.assertIn(":ro", body)


STUB_SET = "recipe_20260101_0000_now"
OLD_VOLUME = f"{PROJECT}_pgdata"
JOB_TAG = "ci-restore-switch"
# The lines that switch writes for the old volume: the name, the tag, the system identifier, and the timeline (ADR-0106 (l)).
TIMELINE_LABEL = "Latest checkpoint's TimeLineID"
OLD_ENTRY = f"volume: {OLD_VOLUME}\ntag: {JOB_TAG}\nDatabase system identifier: 1111\n{TIMELINE_LABEL}: 1\n"

DOCKER_SWITCH_STUB = r"""
echo "$*" >>"$STUB_LOG"
echo "VOL=${RECIPE_PGDATA_VOLUME:-}" >>"$STUB_LOG.env"
args=" $* "
case "$args" in
  *" --entrypoint test "*)
    exit "${STUB_DRILL_STATUS:-1}"
    ;;
  *" compose "*" run "*" restore-volume "*)
    echo '{"level":"INFO","message":"restore volume start"}'
    if [[ -n ${STUB_RESTORE_LAST+x} ]]; then
      echo "$STUB_RESTORE_LAST"
    else
      printf '{"level":"INFO","message":"restore volume completed","set":"%s","system_identifier":"%s"}\n' \
        "$STUB_SET" "${STUB_NEW_ID:-2222}"
    fi
    ;;
  *" compose "*" ps -aq db "*) echo abc123def ;;
  " kill "*)
    rm -f "$STUB_DIR/db.running"
    echo "${STUB_KILL_STATE:-in production}" >"$STUB_DIR/old.state"
    ;;
  *" compose "*" up "*)
    touch "$STUB_DIR/db.running"
    echo "$RECIPE_PGDATA_VOLUME" >"$STUB_DIR/db.volume"
    if [[ $RECIPE_PGDATA_VOLUME == "$STUB_OLD" ]]; then echo "in production" >"$STUB_DIR/old.state"; fi
    ;;
  *" compose "*" stop "*)
    if [[ $args == *" db "* && -e $STUB_DIR/db.running ]]; then
      rm -f "$STUB_DIR/db.running"
      if [[ $(cat "$STUB_DIR/db.volume") == "$STUB_OLD" ]]; then echo "shut down" >"$STUB_DIR/old.state"; fi
    fi
    ;;
  *" compose "*" config --format json "*)
    printf '{"volumes":{"pgdata":{"name":"%s"}}}\n' "${STUB_CONFIG_VOLUME:-$RECIPE_PGDATA_VOLUME}"
    ;;
  " run "*)
    for arg in "$@"; do
      if [[ $arg == *:/var/lib/postgresql/data:ro ]]; then volume=${arg%%:*}; fi
    done
    if [[ $volume == "$STUB_OLD" && -n ${STUB_OLD_FAIL:-} ]]; then exit 1; fi
    if [[ $volume == "$STUB_OLD" ]]; then
      state=$(cat "$STUB_DIR/old.state"); id=1111; timeline=${STUB_OLD_TL-1}
    else
      state=${STUB_NEW_STATE:-shut down}; id=${STUB_NEW_ID:-2222}; timeline=${STUB_NEW_TL-1}
    fi
    echo "pg_control version number:            1700"
    echo "Database system identifier:           $id"
    echo "Database cluster state:               $state"
    if [[ -n $timeline ]]; then echo "Latest checkpoint's TimeLineID:       $timeline"; fi
    ;;
  " inspect "*) echo "${STUB_INSPECT_VOLUME:-$(cat "$STUB_DIR/db.volume")}" ;;
esac
exit 0
"""


class StubbedSubcommandTests(unittest.TestCase):
    def setUp(self):
        self.bash = shutil.which("bash")
        if self.bash is None:
            self.skipTest("bash is not available")

    def run_subcommand(self, name, state=None, stub_env=None, docker_stub=DOCKER_SWITCH_STUB, tree=None,
                       sudo_stub=SUDO_STUB, stub_python=False, read=(), stub_files=None, python_stub=None,
                       extra_stubs=None, driver=None):
        """Run one subcommand with stubs. Return the process, the call log, and the files of CI_STATE."""
        with tempfile.TemporaryDirectory() as temp:
            directory = pathlib.Path(temp)
            stubs = directory / "bin"
            stubs.mkdir()
            stub_dir = directory / "stub"
            stub_dir.mkdir()
            (stub_dir / "old.state").write_text("shut down\n", encoding="utf-8")
            (stub_dir / "db.volume").write_text(OLD_VOLUME + "\n", encoding="utf-8")
            (stub_dir / "db.running").write_text("", encoding="utf-8")
            for key, value in (stub_files or {}).items():
                (stub_dir / key).write_text(value, encoding="utf-8", newline="\n")
            ci_state = directory / "runner" / PROJECT / "state"
            ci_state.mkdir(parents=True)
            run_dir = ci_state.parent
            for key, value in {"backup.set": STUB_SET + "\n", **(state or {})}.items():
                text = value.replace("@RUN@", run_dir.as_posix())
                (ci_state / key).write_text(text, encoding="utf-8", newline="\n")
            for key, value in (tree or {}).items():
                path = run_dir / key
                if value is None:
                    path.mkdir(parents=True, exist_ok=True)
                else:
                    path.parent.mkdir(parents=True, exist_ok=True)
                    path.write_text(value.replace("@RUN@", run_dir.as_posix()), encoding="utf-8", newline="\n")
            log = directory / "calls.log"
            write_stub(stubs / "sudo", sudo_stub)
            write_stub(stubs / "docker", docker_stub)
            write_stub(stubs / "mountpoint", 'exit "${STUB_MOUNTPOINT_STATUS:-1}"')
            write_stub(stubs / "stat", STAT_STUB)
            write_stub(stubs / "sleep", "exit 0")
            if stub_python:
                write_stub(stubs / "python3", python_stub or PYTHON_STUB)
            for key, value in (extra_stubs or {}).items():
                write_stub(stubs / key, value)
            env = {key: value for key, value in os.environ.items() if not key.startswith(("GITHUB_", "RUNNER_", "COMPOSE_"))}
            env.update({
                "GITHUB_ACTIONS": "true",
                "GITHUB_WORKSPACE": str(ROOT),
                "RUNNER_TEMP": str(directory / "runner"),
                "COMPOSE_PROJECT_NAME": PROJECT,
                "RECIPE_VERSION": JOB_TAG,
                "CI_WAIT_ATTEMPTS": "3",
                "PATH": f"{stubs}{os.pathsep}{env['PATH']}",
                "STUB_LOG": str(log),
                "STUB_DIR": str(stub_dir),
                "STUB_SET": STUB_SET,
                "STUB_OLD": OLD_VOLUME,
            })
            env.update({key: value.replace("@RUN@", run_dir.as_posix()) for key, value in (stub_env or {}).items()})
            command = [self.bash, str(SCRIPT), name]
            if driver is not None:
                # The driver sources the shared steps and runs one shell command, for example a call of switch with an argument.
                shared = CI.as_posix()
                command = [self.bash, "-c", f'source "{shared}/common.sh"; source "{shared}/switch-steps.sh"; {driver}']
            done = subprocess.run(command, capture_output=True, text=True, check=False, env=env)
            files = {path.name: path.read_text(encoding="utf-8") for path in ci_state.iterdir()}
            text = log.read_text(encoding="utf-8") if log.exists() else ""
            env_log = pathlib.Path(str(log) + ".env")
            files["env.log"] = env_log.read_text(encoding="utf-8") if env_log.exists() else ""
            listing = [item.relative_to(run_dir).as_posix() for item in run_dir.rglob("*")
                       if ci_state not in item.parents and item != ci_state]
            files["tree.list"] = "\n".join(sorted(listing))
            for pattern in read:
                found = sorted(run_dir.glob(pattern))
                files["read:" + pattern] = found[0].read_text(encoding="utf-8") if found else None
            return done, text, files

    def assertInOrder(self, log, patterns):
        position = 0
        for pattern in patterns:
            match = re.compile(pattern).search(log, position)
            self.assertIsNotNone(match, f"{pattern!r} missing after offset {position} in:\n{log}")
            position = match.end()


class RestoreBehaviorTests(StubbedSubcommandTests):
    def test_restore_runs_the_steps_in_order(self):
        done, log, _ = self.run_subcommand("restore")
        self.assertEqual(done.returncode, 0, done.stderr)
        self.assertInOrder(log, [
            r"compose --project-name recipe-ci-1-1 .* stop app backup\n",
            rf"volume create --label recipe\.ci={PROJECT} recipe-editor_pgdata_\d{{8}}_\d{{6}}\n",
            r"compose .* run --rm --no-deps -T -v recipe-editor_pgdata_\d{8}_\d{6}:/var/lib/postgresql/data restore-volume 20260101_0000_now\n",
        ])
        self.assertNotRegex(log, r"\b(down|volume rm|kill|start)\b")

    def test_restore_saves_the_volume_and_the_identifier(self):
        done, _, files = self.run_subcommand("restore")
        self.assertEqual(done.returncode, 0, done.stderr)
        self.assertRegex(files["new.volume"], r"^recipe-editor_pgdata_\d{8}_\d{6}\n$")
        self.assertEqual(files["new.system-identifier"], "2222\n")
        self.assertIn('"restore volume completed"', files["restore-volume.log"])
        self.assertIn("CI: restore: the volume ", done.stdout)

    def test_restore_rejects_each_bad_last_line(self):
        wrong_set = '{"message":"restore volume completed","set":"recipe_20250101_0000_now","system_identifier":"2222"}'
        wrong_message = f'{{"message":"restore test passed","set":"{STUB_SET}","system_identifier":"2222"}}'
        bad_id = f'{{"message":"restore volume completed","set":"{STUB_SET}","system_identifier":"12x"}}'
        for name, line in (("set", wrong_set), ("message", wrong_message), ("identifier", bad_id), ("empty", "")):
            with self.subTest(name=name):
                done, log, files = self.run_subcommand("restore", stub_env={"STUB_RESTORE_LAST": line})
                self.assertEqual(done.returncode, 1)
                self.assertIn("CI: restore: the last line of restore-volume is wrong.", done.stderr)
                self.assertNotIn("new.system-identifier", files)
                self.assertNotIn("volume rm", log)


class SwitchBehaviorTests(StubbedSubcommandTests):
    NEW = "recipe-editor_pgdata_20260101_000000"
    STATE = {"new.volume": NEW + "\n", "new.system-identifier": "2222\n"}

    def test_switch_runs_the_steps_of_adr_0103_f_in_order(self):
        done, log, _ = self.run_subcommand("switch", self.STATE)
        self.assertEqual(done.returncode, 0, done.stderr)
        run = r"run --rm --network none .*-v {}:/var/lib/postgresql/data:ro .*pg_controldata.*\n"
        self.assertInOrder(log, [
            r" stop proxy app backup walarchive\n",
            r"kill abc123def\n",
            r" rm -f proxy app backup walarchive db\n",
            run.format(OLD_VOLUME),
            r" up -d --no-deps --wait --wait-timeout 180 db\n",
            r" stop proxy app backup walarchive db\n",
            r" rm -f proxy app backup walarchive db\n",
            run.format(OLD_VOLUME),
            r" config --format json\n",
            run.format(self.NEW),
            r" up -d --no-deps --wait --wait-timeout 180 db\n",
            r"inspect .*abc123def\n",
        ])
        self.assertNotRegex(log, r"compose .* (start|down)\b")
        self.assertNotIn("volume rm", log)
        self.assertEqual(log.count("kill "), 1)

    def test_switch_proves_the_unclean_branch_and_writes_the_change_log(self):
        done, _, files = self.run_subcommand("switch", self.STATE)
        self.assertEqual(done.returncode, 0, done.stderr)
        self.assertEqual(files["switch.unclean-state"], "in production\n")
        self.assertEqual(files["old.volume"], OLD_VOLUME + "\n")
        self.assertEqual(files["change.log"], OLD_ENTRY)
        self.assertEqual(files["active.volume"], self.NEW + "\n")
        self.assertIn("CI: switch: the first stop left the state 'in production'. db starts again.", done.stdout)

    def test_switch_writes_the_timeline_of_the_old_volume_into_the_change_log(self):
        # ADR-0106 (l). The line has the label of pg_controldata and follows the identifier line.
        done, _, files = self.run_subcommand("switch", self.STATE, {"STUB_OLD_TL": "3"})
        self.assertEqual(done.returncode, 0, done.stderr)
        self.assertEqual(files["change.log"], OLD_ENTRY.replace(f"{TIMELINE_LABEL}: 1", f"{TIMELINE_LABEL}: 3"))
        self.assertTrue(files["change.log"].endswith("Database system identifier: 1111\nLatest checkpoint's TimeLineID: 3\n"))

    def test_switch_stops_when_pg_controldata_shows_no_timeline_of_the_old_volume(self):
        done, _, files = self.run_subcommand("switch", self.STATE, {"STUB_OLD_TL": ""})
        self.assertEqual(done.returncode, 1)
        self.assertIn("CI: switch: the timeline of the old volume is invalid.", done.stderr)
        self.assertNotIn("change.log", files)
        self.assertNotIn("active.volume", files)

    def test_switch_fails_when_the_kill_leaves_a_clean_state(self):
        done, log, files = self.run_subcommand("switch", self.STATE, {"STUB_KILL_STATE": "shut down"})
        self.assertEqual(done.returncode, 1)
        self.assertIn("CI: switch: the first stop left the state 'shut down'. The restart branch did not run.", done.stderr)
        self.assertNotIn("active.volume", files)
        self.assertNotRegex(log, r" up ")

    def test_switch_stops_when_the_new_identifier_differs(self):
        done, log, files = self.run_subcommand("switch", self.STATE, {"STUB_NEW_ID": "9999"})
        self.assertEqual(done.returncode, 1)
        self.assertIn("CI: switch: the system identifier of the new volume differs from the line of restore-volume.", done.stderr)
        self.assertEqual(log.count(" up -d"), 1)
        self.assertNotIn("volume rm", log)
        self.assertNotIn("active.volume", files)

    def test_switch_stops_when_the_new_volume_is_not_shut_down(self):
        done, log, _ = self.run_subcommand("switch", self.STATE, {"STUB_NEW_STATE": "in production"})
        self.assertEqual(done.returncode, 1)
        self.assertIn("CI: switch: the new volume is not in the state 'shut down'.", done.stderr)
        self.assertEqual(log.count(" up -d"), 1)

    def test_switch_stops_when_compose_shows_another_volume(self):
        done, log, _ = self.run_subcommand("switch", self.STATE, {"STUB_CONFIG_VOLUME": "other"})
        self.assertEqual(done.returncode, 1)
        self.assertIn("CI: switch: Compose does not show the new volume for pgdata.", done.stderr)
        self.assertEqual(log.count(" up -d"), 1)

    def test_switch_stops_when_db_mounts_another_volume(self):
        done, _, _ = self.run_subcommand("switch", self.STATE, {"STUB_INSPECT_VOLUME": "other"})
        self.assertEqual(done.returncode, 1)
        self.assertIn("CI: switch: db does not use the new volume.", done.stderr)

    def test_switch_needs_the_files_of_restore(self):
        done, log, _ = self.run_subcommand("switch", {})
        self.assertEqual(done.returncode, 1)
        self.assertIn("CI: switch: run restore first.", done.stderr)
        self.assertEqual(log, "")

    def test_later_subcommands_use_the_active_volume(self):
        _, _, files = self.run_subcommand("cleanup", {"active.volume": self.NEW + "\n"})
        self.assertIn(f"VOL={self.NEW}", files["env.log"])
        self.assertNotIn(f"VOL={OLD_VOLUME}", files["env.log"])


RESTORE_SOURCE = "restore volume completed"
PITR_SOURCE = "pitr volume completed"
SOURCE_ERROR = "CI: switch: the source line must be 'restore volume completed' or 'pitr volume completed'."


def pitr_line(identifier="2222", timeline="2", drill="false", message=PITR_SOURCE):
    """Return a JSON line as pitr-volume writes it (ADR-0106 (d), (f) step 12)."""
    return (f'{{"level":"INFO","message":"{message}","base":"base_20260101_000000",'
            f'"system_identifier":"{identifier}","timeline":{timeline},"chain_seq":7,"drill":{drill}}}')


def pitr_state(line=None):
    """Return the state files of a recovery. The log holds a start line and the last line."""
    log = '{"level":"INFO","message":"pitr volume start"}\n' + (pitr_line() if line is None else line) + "\n"
    return {"new.volume": SwitchBehaviorTests.NEW + "\n", "pitr-volume.log": log}


class SwitchSourceLineTests(StubbedSubcommandTests):
    """Task 6.4 of pitr-recovery (design.md D7, ADR-0106 (j)). The switch takes the source line as its parameter."""

    NEW = SwitchBehaviorTests.NEW
    RESTORE_STATE = SwitchBehaviorTests.STATE
    PITR_ENV = {"STUB_NEW_TL": "2"}

    def run_switch(self, source, state, env=None):
        return self.run_subcommand("driver", state, env, driver=f'switch "{source}"')

    def run_pitr(self, state=None, env=None):
        return self.run_switch(PITR_SOURCE, pitr_state() if state is None else state, {**self.PITR_ENV, **(env or {})})

    def test_the_script_passes_the_restore_source_to_switch(self):
        self.assertRegex(script_text(), r'(?m)^  switch\) switch "restore volume completed" ;;$')
        self.assertNotRegex(script_text(), r"(?m)^\s*switch\) switch ;;")

    def test_an_unknown_source_stops_the_run_before_any_command(self):
        for source in ("", "restore volume start", "Pitr volume completed", "pitr volume completed ", "restore"):
            with self.subTest(source=source):
                done, log, files = self.run_switch(source, self.RESTORE_STATE)
                self.assertEqual(done.returncode, 1)
                self.assertIn(SOURCE_ERROR, done.stderr)
                self.assertEqual(log, "")
                self.assertNotIn("change.log", files)

    def test_a_missing_source_stops_the_run(self):
        done, log, _ = self.run_subcommand("driver", self.RESTORE_STATE, driver="switch")
        self.assertEqual(done.returncode, 1)
        self.assertIn(SOURCE_ERROR, done.stderr)
        self.assertEqual(log, "")

    def test_the_restore_source_keeps_the_behavior(self):
        done, log, files = self.run_switch(RESTORE_SOURCE, self.RESTORE_STATE)
        self.assertEqual(done.returncode, 0, done.stderr)
        self.assertEqual(files["change.log"], OLD_ENTRY)
        self.assertEqual(files["switch.unclean-state"], "in production\n")
        self.assertEqual(files["active.volume"], self.NEW + "\n")
        self.assertEqual(log.count("kill "), 1)
        self.assertEqual(log.count(" up -d"), 2)
        self.assertNotIn("--entrypoint test", log)

    def test_the_restore_source_still_stops_on_a_missing_timeline_of_the_old_volume(self):
        done, _, files = self.run_switch(RESTORE_SOURCE, self.RESTORE_STATE, {"STUB_OLD_TL": ""})
        self.assertEqual(done.returncode, 1)
        self.assertIn("CI: switch: the timeline of the old volume is invalid.", done.stderr)
        self.assertNotIn("change.log", files)

    def test_the_restore_source_compares_the_identifier_only(self):
        # The new volume has the timeline 5. The restore line has no timeline, so the run passes.
        done, _, _ = self.run_switch(RESTORE_SOURCE, self.RESTORE_STATE, {"STUB_NEW_TL": "5"})
        self.assertEqual(done.returncode, 0, done.stderr)

    def test_the_restore_source_ignores_a_file_pitr_drill(self):
        done, log, _ = self.run_switch(RESTORE_SOURCE, self.RESTORE_STATE, {"STUB_DRILL_STATUS": "0"})
        self.assertEqual(done.returncode, 0, done.stderr)
        self.assertNotIn("--entrypoint test", log)

    def test_the_pitr_source_runs_the_steps_without_a_start_of_db_on_the_old_volume(self):
        done, log, files = self.run_pitr()
        self.assertEqual(done.returncode, 0, done.stderr)
        run = r"run --rm --network none .*-v {}:/var/lib/postgresql/data:ro .*pg_controldata.*\n"
        self.assertInOrder(log, [
            r" stop proxy app backup walarchive\n",
            r"kill abc123def\n",
            r" rm -f proxy app backup walarchive db\n",
            run.format(OLD_VOLUME),
            r" config --format json\n",
            run.format(self.NEW),
            r"--entrypoint test .*/var/lib/postgresql/data/pitr-drill\n",
            r" up -d --no-deps --wait --wait-timeout 180 db\n",
            r"inspect .*abc123def\n",
        ])
        self.assertEqual(log.count(" up -d"), 1)
        self.assertNotRegex(log, r" stop proxy app backup walarchive db\n")
        self.assertNotIn("switch.unclean-state", files)
        self.assertEqual(files["active.volume"], self.NEW + "\n")
        self.assertEqual(files["old.volume"], OLD_VOLUME + "\n")

    def test_the_pitr_source_needs_no_unclean_state_of_the_old_volume(self):
        done, _, files = self.run_pitr(env={"STUB_KILL_STATE": "shut down"})
        self.assertEqual(done.returncode, 0, done.stderr)
        self.assertNotIn("switch.unclean-state", files)

    def test_the_pitr_source_writes_the_identifier_and_the_timeline_of_the_old_volume(self):
        done, _, files = self.run_pitr(env={"STUB_OLD_TL": "3"})
        self.assertEqual(done.returncode, 0, done.stderr)
        self.assertEqual(files["change.log"], OLD_ENTRY.replace(f"{TIMELINE_LABEL}: 1", f"{TIMELINE_LABEL}: 3"))

    def test_the_pitr_source_writes_unreadable_when_pg_controldata_fails(self):
        done, log, files = self.run_pitr(env={"STUB_OLD_FAIL": "1"})
        self.assertEqual(done.returncode, 0, done.stderr)
        self.assertEqual(files["change.log"], (f"volume: {OLD_VOLUME}\ntag: {JOB_TAG}\n"
                                               f"Database system identifier: unreadable\n{TIMELINE_LABEL}: unreadable\n"))
        self.assertEqual(files["active.volume"], self.NEW + "\n")
        self.assertEqual(log.count(" up -d"), 1)

    def test_the_pitr_source_writes_unreadable_for_a_missing_timeline_only(self):
        done, _, files = self.run_pitr(env={"STUB_OLD_TL": ""})
        self.assertEqual(done.returncode, 0, done.stderr)
        self.assertEqual(files["change.log"], (f"volume: {OLD_VOLUME}\ntag: {JOB_TAG}\n"
                                               f"Database system identifier: 1111\n{TIMELINE_LABEL}: unreadable\n"))

    def test_the_pitr_source_accepts_a_timeline_in_quotes(self):
        line = pitr_line().replace('"timeline":2', '"timeline":"2"')
        done, _, _ = self.run_pitr(pitr_state(line))
        self.assertEqual(done.returncode, 0, done.stderr)

    def test_the_pitr_source_stops_when_the_timeline_of_the_new_volume_differs(self):
        done, log, files = self.run_pitr(env={"STUB_NEW_TL": "3"})
        self.assertEqual(done.returncode, 1)
        self.assertIn("CI: switch: the timeline of the new volume differs from the line of pitr-volume.", done.stderr)
        self.assertEqual(log.count(" up -d"), 0)
        self.assertNotIn("volume rm", log)
        self.assertNotIn("active.volume", files)

    def test_the_pitr_source_stops_when_the_new_volume_has_no_timeline(self):
        done, log, _ = self.run_pitr(env={"STUB_NEW_TL": ""})
        self.assertEqual(done.returncode, 1)
        self.assertIn("CI: switch: the timeline of the new volume differs from the line of pitr-volume.", done.stderr)
        self.assertEqual(log.count(" up -d"), 0)

    def test_the_pitr_source_stops_when_the_identifier_of_the_new_volume_differs(self):
        done, log, files = self.run_pitr(env={"STUB_NEW_ID": "9999"})
        self.assertEqual(done.returncode, 1)
        self.assertIn("CI: switch: the system identifier of the new volume differs from the line of pitr-volume.", done.stderr)
        self.assertEqual(log.count(" up -d"), 0)
        self.assertNotIn("active.volume", files)

    def test_the_pitr_source_stops_when_the_new_volume_is_not_shut_down(self):
        done, log, _ = self.run_pitr(env={"STUB_NEW_STATE": "in production"})
        self.assertEqual(done.returncode, 1)
        self.assertIn("CI: switch: the new volume is not in the state 'shut down'.", done.stderr)
        self.assertEqual(log.count(" up -d"), 0)

    def test_the_pitr_source_refuses_a_volume_with_the_file_pitr_drill(self):
        done, log, files = self.run_pitr(env={"STUB_DRILL_STATUS": "0"})
        self.assertEqual(done.returncode, 1)
        self.assertIn("CI: switch: the new volume holds the file pitr-drill. A drill volume never becomes production.", done.stderr)
        self.assertEqual(log.count(" up -d"), 0)
        self.assertNotIn("volume rm", log)
        self.assertNotIn("active.volume", files)

    def test_the_pitr_source_stops_when_the_test_for_the_file_pitr_drill_fails(self):
        done, log, files = self.run_pitr(env={"STUB_DRILL_STATUS": "125"})
        self.assertEqual(done.returncode, 1)
        self.assertIn("CI: switch: cannot test the new volume for the file pitr-drill.", done.stderr)
        self.assertEqual(log.count(" up -d"), 0)
        self.assertNotIn("active.volume", files)

    def test_the_drill_test_runs_in_a_container_without_network_and_with_a_read_only_mount(self):
        body = shell_function("refuse_drill_volume")
        self.assertIn(':/var/lib/postgresql/data:ro"', body)
        self.assertIn("--network none", body)
        self.assertIn("--entrypoint test", body)

    def test_the_pitr_source_refuses_a_line_with_drill_true(self):
        done, log, files = self.run_pitr(pitr_state(pitr_line(drill="true")))
        self.assertEqual(done.returncode, 1)
        self.assertIn("CI: switch: the line of pitr-volume comes from a drill. A drill volume never becomes production.", done.stderr)
        self.assertEqual(log.count(" up -d"), 0)
        self.assertNotIn("active.volume", files)

    def test_the_pitr_source_rejects_each_bad_last_line(self):
        cases = {
            "restore line": pitr_line(message=RESTORE_SOURCE),
            "other message": pitr_line(message="pitr volume start"),
            "bad identifier": pitr_line(identifier="12x"),
            "bad timeline": pitr_line(timeline='"x"'),
            "no timeline": pitr_line().replace('"timeline":2,', ""),
            "no drill": pitr_line().replace(',"drill":false', ""),
            "bad drill": pitr_line(drill='"no"'),
            "not json": "pitr volume completed",
            "empty": "",
        }
        for name, line in cases.items():
            with self.subTest(name=name):
                done, log, files = self.run_pitr(pitr_state(line))
                self.assertEqual(done.returncode, 1)
                self.assertIn("CI: switch: the last line of pitr-volume is wrong.", done.stderr)
                self.assertEqual(log, "")
                self.assertNotIn("change.log", files)

    def test_the_pitr_source_needs_the_files_of_pitr_volume(self):
        for state in ({}, {"new.volume": self.NEW + "\n"}, {"pitr-volume.log": pitr_line() + "\n"}):
            with self.subTest(files=sorted(state)):
                done, log, _ = self.run_switch(PITR_SOURCE, state, self.PITR_ENV)
                self.assertEqual(done.returncode, 1)
                self.assertIn("CI: switch: run pitr-volume first.", done.stderr)
                self.assertEqual(log, "")

    def test_the_pitr_source_stops_when_compose_shows_another_volume(self):
        done, log, _ = self.run_pitr(env={"STUB_CONFIG_VOLUME": "other"})
        self.assertEqual(done.returncode, 1)
        self.assertIn("CI: switch: Compose does not show the new volume for pgdata.", done.stderr)
        self.assertEqual(log.count(" up -d"), 0)

    def test_the_pitr_source_stops_when_db_mounts_another_volume(self):
        done, _, _ = self.run_pitr(env={"STUB_INSPECT_VOLUME": "other"})
        self.assertEqual(done.returncode, 1)
        self.assertIn("CI: switch: db does not use the new volume.", done.stderr)

    def test_the_text_of_switch_names_both_sources_and_checks_the_old_state_once(self):
        body = shell_function("switch")
        self.assertIn('"$SOURCE_PITR"', body)
        self.assertIn('"$SOURCE_RESTORE"', body)
        self.assertEqual(body.count("check_unclean_state"), 1)
        self.assertEqual(body.count("restart_old_db"), 1)


DOCKER_ACCESS_STUB = r"""
echo "$*" >>"$STUB_LOG"
args=" $* "
case "$args" in
  *" compose "*" run "*" --entrypoint bash "*)
    echo "VOL=${RECIPE_PGDATA_VOLUME:-}" >>"$STUB_LOG.env"
    roles=("$@")
    while [[ ${roles[0]} != probe ]]; do roles=("${roles[@]:1}"); done
    roles=("${roles[@]:1}")
    for role in "${roles[@]}"; do
      if [[ $role == postgres ]]; then result=${STUB_PG_RESULT:-reject}
      elif [[ -e $STUB_DIR/passwords.set ]]; then result=${STUB_AFTER_RESULT:-ok}
      elif [[ ${STUB_BEFORE_ROLE%%=*} == "$role" ]]; then result=${STUB_BEFORE_ROLE#*=}
      else result=${STUB_BEFORE_RESULT:-password}; fi
      echo "$role|$result"
    done
    echo "noise line"
    ;;
  *" compose "*" logs "*)
    printf '%s\n' "${STUB_DB_LOG:-LOG:  database system is ready to accept connections}"
    ;;
  *" compose "*" exec "*" psql "*)
    cat >"$RUNNER_TEMP/$COMPOSE_PROJECT_NAME/state/test.psql.stdin"
    touch "$STUB_DIR/passwords.set"
    ;;
esac
exit 0
"""


class AccessBehaviorTests(StubbedSubcommandTests):
    NEW = "recipe-editor_pgdata_20260101_000000"
    STATE = {"active.volume": NEW + "\n"}
    FOUR = "recipe_migrator recipe_app recipe_backup recipe_replicator"
    PROBE = r" run --rm --no-deps -T [\s\S]*?--entrypoint bash backup-premigrate -c [\s\S]*? probe "

    def run_access(self, stub_env=None, state=None):
        return self.run_subcommand("access", self.STATE if state is None else state, stub_env, DOCKER_ACCESS_STUB)

    def test_access_checks_before_the_passwords_and_logs_in_after(self):
        done, log, files = self.run_access()
        self.assertEqual(done.returncode, 0, done.stderr)
        self.assertInOrder(log, [
            self.PROBE + r"postgres\n",
            self.PROBE + self.FOUR + r"\n",
            r" logs --no-color --no-log-prefix db\n",
            r" exec -T -u postgres db psql -v ON_ERROR_STOP=1 --no-password --no-psqlrc --dbname postgres\n",
            self.PROBE + self.FOUR + r"\n",
        ])
        self.assertEqual(log.count("--entrypoint bash"), 3)
        for line in ("CI: access: step 1, postgres over TCP",
                     "CI: access: step 2, password login of each login role before the passwords",
                     "CI: access: step 3, the log of db holds no line of 01-roles.sh",
                     "CI: access: step 4, set the passwords from the secret files",
                     "CI: access: step 5, password login of each login role after the passwords",
                     "CI: access: done"):
            self.assertIn(line, done.stdout)
        self.assertIn(f"VOL={self.NEW}", files["env.log"])
        self.assertNotIn(f"VOL={OLD_VOLUME}", files["env.log"])

    def test_access_mounts_the_secret_files_read_only(self):
        done, log, _ = self.run_access()
        self.assertEqual(done.returncode, 0, done.stderr)
        for name in ("postgres", "migrator", "app", "backup", "replicator"):
            self.assertRegex(log, rf"-v \S+/{name}_password:/ci-secrets/{name}_password:ro")

    def test_access_sends_the_password_statements_through_standard_input_only(self):
        done, log, files = self.run_access()
        self.assertEqual(done.returncode, 0, done.stderr)
        stdin = files["test.psql.stdin"]
        for role, name in (("postgres", "postgres"), ("recipe_migrator", "migrator"), ("recipe_app", "app"),
                           ("recipe_backup", "backup"), ("recipe_replicator", "replicator")):
            self.assertIn(f"\\set {name}_password `cat /run/secrets/{name}_password`", stdin)
            self.assertIn(f"ALTER ROLE {role} PASSWORD :'{name}_password';", stdin)
        self.assertNotIn("ALTER ROLE", log)
        self.assertNotIn("PASSWORD", log)

    def test_access_needs_the_active_volume(self):
        done, log, _ = self.run_access(state={})
        self.assertEqual(done.returncode, 1)
        self.assertIn("CI: access: run switch first.", done.stderr)
        self.assertEqual(log, "")

    def test_access_fails_when_postgres_does_not_get_the_reject_rule(self):
        for result in ("ok", "password", "other"):
            with self.subTest(result=result):
                done, log, _ = self.run_access({"STUB_PG_RESULT": result})
                self.assertEqual(done.returncode, 1)
                self.assertIn(f"CI: access: the login of postgres gave '{result}'. Expected 'reject'.", done.stderr)
                self.assertEqual(log.count("--entrypoint bash"), 1)

    def test_access_fails_when_a_login_works_before_the_passwords(self):
        done, log, _ = self.run_access({"STUB_BEFORE_ROLE": "recipe_app=ok"})
        self.assertEqual(done.returncode, 1)
        self.assertIn("CI: access: the login of recipe_app gave 'ok'. Expected 'password'.", done.stderr)
        self.assertNotIn(" logs ", log)
        self.assertNotIn(" exec ", log)

    def test_access_fails_when_a_login_gets_another_error_before_the_passwords(self):
        done, log, _ = self.run_access({"STUB_BEFORE_RESULT": "reject"})
        self.assertEqual(done.returncode, 1)
        self.assertIn("Expected 'password'.", done.stderr)
        self.assertNotIn(" exec ", log)

    def test_access_fails_when_the_log_holds_a_line_of_the_init_script(self):
        lines = (
            "running /docker-entrypoint-initdb.d/01-roles.sh",
            "CREATE ROLE",
            "CREATE DATABASE",
            "PostgreSQL init process complete; ready for start up.",
        )
        for line in lines:
            with self.subTest(line=line):
                text = f"{line}\nLOG:  database system is ready to accept connections"
                done, log, _ = self.run_access({"STUB_DB_LOG": text})
                self.assertEqual(done.returncode, 1)
                self.assertIn("CI: access: the log of db holds 1 lines of 01-roles.sh.", done.stderr)
                self.assertNotIn(" exec ", log)

    def test_access_fails_when_the_log_shows_no_start(self):
        done, log, _ = self.run_access({"STUB_DB_LOG": "nothing"})
        self.assertEqual(done.returncode, 1)
        self.assertIn("CI: access: the log of db has no start line.", done.stderr)
        self.assertNotIn(" exec ", log)

    def test_access_fails_when_a_login_fails_after_the_passwords(self):
        done, log, _ = self.run_access({"STUB_AFTER_RESULT": "password"})
        self.assertEqual(done.returncode, 1)
        self.assertIn("CI: access: the login of recipe_migrator gave 'password'. Expected 'ok'.", done.stderr)
        self.assertEqual(log.count("--entrypoint bash"), 3)

    def test_access_fails_when_the_probe_reports_too_few_roles(self):
        stub = DOCKER_ACCESS_STUB.replace('echo "$role|$result"', '[[ $role == recipe_backup ]] || echo "$role|$result"')
        done, _, _ = self.run_subcommand("access", self.STATE, None, stub)
        self.assertEqual(done.returncode, 1)
        self.assertIn("CI: access: the probe reported 3 of 4 roles.", done.stderr)


SUDO_EXEC_STUB = """
while [[ ${1:-} == --* ]]; do shift; done
if [[ ${1:-} != docker ]]; then echo "sudo $*" >>"$STUB_LOG"; fi
exec "$@"
"""

STAT_STUB = r"""
# STUB_STAT_TARGET_DEVICE gives a wrong device number to the target directory.
if [[ -n ${STUB_STAT_TARGET_DEVICE:-} && $* == *.moved-* ]]; then echo "$STUB_STAT_TARGET_DEVICE"; exit 0; fi
exec /usr/bin/stat "$@"
"""

PYTHON_STUB = r"""
echo "python3 $*" >>"$STUB_LOG"
case " $* " in
  *" export "*) printf '%s\n' "${STUB_EXPORT_HASH:?}" >"$CI_STATE/${*: -1}" ;;
esac
[[ " $* " == *" ${STUB_PROBE_FAIL:-none} "* ]] && exit 1
exit 0
"""

DOCKER_WAL_STUB = r"""
echo "$*" >>"$STUB_LOG"
args=" $* "
case "$args" in
  *" compose "*" ps -aq walarchive "*)
    if [[ -n ${STUB_WALARCHIVE_ID:-} ]]; then echo "$STUB_WALARCHIVE_ID"; fi
    ;;
  *" compose "*" up "*" walarchive "*)
    if [[ -z ${STUB_NO_IDENTITY:-} ]]; then echo "${STUB_NEW_ID:-2222}" >"$STUB_ARCHIVE/.system-identifier"; fi
    ;;
  *" exec -T walarchive basebackup-once "*)
    name=${STUB_BASE_NAME:-base_20260201_000000}
    mkdir -p "$STUB_ARCHIVE/base/$name"
    printf '{"level":"INFO","job":"basebackup","message":"base backup completed","base":"%s"}\n' "$name"
    ;;
  *" exec -T walarchive pg_verifybackup "*)
    if [[ ${STUB_VERIFY_FAIL:-} == 1 ]]; then exit 1; fi
    ;;
  *" exec -T proxy wget "*) echo "${STUB_READYZ:-ok}" ;;
  *" exec -T backup backup-now "*) echo '{"message":"backup completed"}' ;;
  *" compose "*" run "*" pitr-check "*)
    if [[ ${STUB_PITR_FAIL:-} == 1 ]]; then exit 1; fi
    echo '{"message":"pitr check passed"}'
    ;;
  *" compose "*" logs "*) printf '%s\n' "${STUB_DB_LOG:-LOG:  database system is ready to accept connections}" ;;
  " volume inspect "*)
    if [[ ${STUB_VOLUME_MISSING:-} == 1 ]]; then exit 1; fi
    ;;
  *" compose "*" exec "*" psql "*" -c "*)
    case "$args" in
      *"SELECT state, flush_lsn"*) echo "${STUB_STREAM_ROW-streaming|0/5000060}" ;;
      *"SELECT pg_switch_wal()"*) echo 0/6000000 ;;
      *"SELECT EXISTS"*) echo "${STUB_FLUSH:-t}" ;;
      *"SELECT pg_current_wal_lsn()"*) echo 0/7000060 ;;
      *"SELECT pg_walfile_name("*) echo 000000010000000000000007 ;;
      *"SELECT chain_seq, encode(entry_hash"*) echo "${STUB_MARKER-7|$(printf 'cd%.0s' {1..32})}" ;;
      *"SELECT count(*) FROM public."*)
        table=${args##*FROM public.}; table=${table%% *}
        tr ';' '\n' <<<"${STUB_COUNTS:-}" | sed -n "s/^$table|//p"
        ;;
    esac
    ;;
  *" compose "*" exec "*" psql "*)
    cat >/dev/null
    echo "${STUB_CHAIN-0|t|t}"
    ;;
esac
exit 0
"""

ARCHIVE_ID = "2222"
MOVED = "walarchive-disk/walarchive.moved-20260101_000000"
SEGMENT_ONE = "000000010000000000000001"
SEGMENT_TWO = "000000010000000000000002.partial"
OLD_BASE = "base_20260101_000000"
NEW_BASE = "base_20260201_000000"
CHECK_BASE = "base_20260301_000000"
SECRET_VALUE = "secret-value-of-the-app-role"
EXPORT_HASH = "ab" * 32
COUNTS = "app_user|2;machine|1;unit|3;recipe|1;recipe_version|1;parameter|5;parameter_limit|5;audit_log|10"
COUNTS_FILE = COUNTS.replace(";", "\n") + "\n"

WAL_TREE = {
    f"walarchive-disk/walarchive/{SEGMENT_ONE}": "segment one",
    f"walarchive-disk/walarchive/{SEGMENT_TWO}": "segment two",
    "walarchive-disk/walarchive/.system-identifier": "1111\n",
    "walarchive-disk/walarchive/.walarchive-marker": "",
    "walarchive-disk/walarchive/.lock": "",
    f"walarchive-disk/walarchive/base/{OLD_BASE}/backup_manifest": "{}",
    "walarchive-disk/walarchive/base/.tmp-base_20260102_000000/part": "part",
    "walarchive-disk/walarchive/.pitr-scratch": None,
}
WAL_STATE = {
    "change.log": OLD_ENTRY,
    "old.volume": OLD_VOLUME + "\n",
    "new.volume": "recipe-editor_pgdata_20260101_000000\n",
    "new.system-identifier": ARCHIVE_ID + "\n",
    "active.volume": "recipe-editor_pgdata_20260101_000000\n",
}
CHECK_TREE = {
    "walarchive-disk/walarchive/.system-identifier": ARCHIVE_ID + "\n",
    f"walarchive-disk/walarchive/base/{NEW_BASE}/backup_manifest": "{}",
    "walarchive-disk/walarchive/000000010000000000000007.partial": "partial",
    f"{MOVED}/old-system-identifier": f"Database system identifier: 1111\n{TIMELINE_LABEL}: 1\n",
    f"{MOVED}/{SEGMENT_ONE}": "segment one",
    f"{MOVED}/base/{OLD_BASE}/backup_manifest": "{}",
    "secrets/app_password": SECRET_VALUE + "\n",
    **{f"secrets/{name}_password": f"value-of-{name}\n" for name in ("postgres", "migrator", "backup", "replicator")},
}
CHECK_MANIFEST = "".join(
    f"@RUN@/{MOVED}/{name}\n"
    for name in ("old-system-identifier", SEGMENT_ONE, "base", f"base/{OLD_BASE}", f"base/{OLD_BASE}/backup_manifest")
)
CHECK_STATE = {
    **WAL_STATE,
    "wal-new.base": NEW_BASE + "\n",
    "wal-old.bases": OLD_BASE + "\n",
    "wal-target.dir": f"@RUN@/{MOVED}\n",
    "wal-target.manifest": CHECK_MANIFEST,
    "row-counts": COUNTS_FILE,
    "export.sha256": EXPORT_HASH + "\n",
}
WAL_ENV = {"STUB_ARCHIVE": "@RUN@/walarchive-disk/walarchive", "STUB_NEW_ID": ARCHIVE_ID, "STUB_EXPORT_HASH": EXPORT_HASH, "STUB_COUNTS": COUNTS}


class SwitchStepsFileTests(unittest.TestCase):
    """Task 6.1 of pitr-recovery (design.md D7). The shared file holds the role passwords and the new WAL line."""

    MOVED = ("set_role_passwords", "start_wal_line")
    SHARED_BY_BOTH = ("load_active_volume", "require_no_labelled_volume", "cleanup")

    def test_shared_file_exists_with_valid_syntax_and_lf(self):
        self.assertTrue(SWITCH_STEPS.is_file(), "deploy/ci/switch-steps.sh is missing")
        self.assertNotIn(b"\r", SWITCH_STEPS.read_bytes())
        bash = shutil.which("bash")
        if bash is None:
            self.skipTest("bash is not available")
        done = subprocess.run([bash, "-n", str(SWITCH_STEPS)], capture_output=True, check=False)
        self.assertEqual(done.returncode, 0, done.stderr)

    def test_restore_switch_sources_the_shared_file_from_its_own_directory(self):
        self.assertRegex(script_text(), r'(?m)^source "\$\(dirname "\$\{BASH_SOURCE\[0\]\}"\)/switch-steps\.sh"$')

    def test_the_shared_file_defines_the_two_functions(self):
        text = switch_steps_text()
        for name in self.MOVED:
            self.assertIn(f"\n{name}() {{\n", text, f"{name} is not in switch-steps.sh")

    def test_restore_switch_no_longer_defines_the_moved_functions(self):
        for name in self.MOVED:
            self.assertNotIn(f"\n{name}() {{\n", script_text(), f"{name} stays in restore-switch.sh")

    def test_the_functions_of_both_job_scripts_have_one_definition_in_the_shared_file(self):
        # Task 6.2 of pitr-recovery (design.md D7). restore-switch.sh and pitr-recovery.sh must not copy these functions.
        pitr = (CI / "pitr-recovery.sh").read_text(encoding="utf-8")
        for name in self.SHARED_BY_BOTH:
            marker = f"\n{name}() {{\n"
            with self.subTest(name=name):
                self.assertEqual(switch_steps_text().count(marker), 1, f"{name} is not defined once in switch-steps.sh")
                self.assertNotIn(marker, script_text(), f"{name} stays in restore-switch.sh")
                self.assertNotIn(marker, pitr, f"{name} stays in pitr-recovery.sh")

    def test_the_shared_file_defines_switch_and_restore_switch_does_not(self):
        # Task 6.2 of pitr-recovery (design.md D7). Both job scripts get the switch from the shared file.
        self.assertIn("\nswitch() {\n", switch_steps_text(), "switch is not in switch-steps.sh")
        self.assertNotIn("\nswitch() {\n", script_text(), "switch stays in restore-switch.sh")

    def test_the_shared_file_calls_only_functions_that_it_or_common_sh_defines(self):
        # A function that only restore-switch.sh defines would break every other job script that sources the file.
        pattern = r"(?m)^([a-z_][a-z0-9_]*)\(\) \{$"
        own = set(re.findall(pattern, switch_steps_text()))
        common = set(re.findall(pattern, (CI / "common.sh").read_text(encoding="utf-8")))
        foreign = set(re.findall(pattern, script_text())) - own - common
        code = "\n".join(line for line in switch_steps_text().splitlines() if not line.lstrip().startswith("#"))
        position = r"(?:^|[;&|(!{]|\$\(|\b(?:if|then|do|else|elif|while|until|retry)\b)\s*"
        called = {name for name in foreign if re.search(position + name + r"\b", code, re.MULTILINE)}
        self.assertEqual(called, set(), "switch-steps.sh calls functions of restore-switch.sh")

    def test_the_globals_of_the_moved_functions_have_defaults_in_the_shared_file(self):
        text = switch_steps_text()
        for name in ("WAL_SUFFIX", "WAL_LABEL", "WAIT_ATTEMPTS", "IDENTIFIER_PATTERN", "SEGMENT_REGEX"):
            self.assertRegex(text, rf"(?m)^{name}=", f"{name} has no default in switch-steps.sh")
            self.assertNotRegex(script_text(), rf"(?m)^{name}=", f"{name} stays in restore-switch.sh")


class WalLineTextTests(unittest.TestCase):
    """Check the order and the forbidden commands in the text of wal-line."""

    def test_wal_line_follows_the_steps_of_adr_0103_s(self):
        self.assertIn("run switch first", shell_function("wal_line"))
        self.assertIn("start_wal_line", shell_function("wal_line"))
        body = shell_function("start_wal_line")
        markers = ["ps -aq walarchive", "make_target", "write_old_identifier", "move_segments",
                   "move_bases", "up -d --no-deps walarchive", "new_line_checks"]
        found = [body.find(marker) for marker in markers]
        self.assertNotIn(-1, found, dict(zip(markers, found)))
        self.assertEqual(found, sorted(found), dict(zip(markers, found)))
        body = shell_function("new_line_checks")
        markers = ["stream_ready", "take_base_backup", "pg_switch_wal", "flush_reached", "wal_check", "wal-new$WAL_SUFFIX.base"]
        found = [body.find(marker) for marker in markers]
        self.assertNotIn(-1, found, dict(zip(markers, found)))
        self.assertEqual(found, sorted(found), dict(zip(markers, found)))

    def test_wal_line_deletes_nothing(self):
        text = "".join(shell_function(name) for name in (
            "wal_line", "start_wal_line", "make_target", "move_segments", "move_bases", "take_base_backup",
            "new_line_checks"))
        self.assertNotRegex(text, r"\brm\b|-delete|--force|mv -f|unlink")
        self.assertNotRegex(text, r"docker volume|compose (\S+ )*(down|rm|kill|stop)\b")

    def test_check_follows_the_order_of_the_task(self):
        body = shell_function("check")
        markers = ["run wal-line first", "TRUNCATE public.session, public.login_attempt", "up -d --no-deps --wait",
                   "verify_export", "compare_counts", "check_chain",
                   "check_flush", "wal_check", "check_pitr", "check_db_log_passwords", "take_base_backup",
                   "exec -T backup backup-now", "check_base_directory", "check_kept_files"]
        found = [body.find(marker) for marker in markers]
        self.assertNotIn(-1, found, dict(zip(markers, found)))
        self.assertEqual(found, sorted(found), dict(zip(markers, found)))

    def test_check_deletes_no_volume_and_no_archive_file(self):
        text = "".join(shell_function(name) for name in (
            "check", "compare_counts", "check_chain", "check_base_directory", "check_kept_files", "check_db_log_passwords"))
        self.assertNotRegex(text, r"sudo rm|rm -r|-delete|--force|mv ")
        self.assertNotRegex(text, r"docker volume (rm|prune|create)|compose (\S+ )*(down|rm|kill|stop)\b")

    def test_check_reads_the_chain_query_from_the_restore_test_library(self):
        self.assertIn("restore-steps.sh", shell_function("check_chain"))

    def test_check_never_puts_a_password_into_a_command(self):
        body = shell_function("check_db_log_passwords")
        self.assertIn("grep -c -F", body)
        self.assertNotRegex(body, r"\$\(sudo cat")


class WalLineBehaviorTests(StubbedSubcommandTests):
    def run_wal_line(self, stub_env=None, state=None, tree=None, read=()):
        env = {**WAL_ENV, **(stub_env or {})}
        return self.run_subcommand("wal-line", WAL_STATE if state is None else state, env, DOCKER_WAL_STUB,
                                   tree=WAL_TREE if tree is None else tree, sudo_stub=SUDO_EXEC_STUB, read=read)

    def moved_entries(self, listing):
        match = re.search(r"^(walarchive-disk/walarchive\.moved-\d{8}_\d{6})$", listing, re.M)
        self.assertIsNotNone(match, listing)
        prefix = match.group(1) + "/"
        return match.group(1), {line[len(prefix):] for line in listing.splitlines() if line.startswith(prefix)}

    def test_wal_line_moves_the_old_files_and_starts_the_new_line(self):
        done, log, files = self.run_wal_line(read=("walarchive-disk/walarchive/.system-identifier", "walarchive-disk/walarchive.moved-*/.system-identifier"))
        self.assertEqual(done.returncode, 0, done.stderr)
        target, moved = self.moved_entries(files["tree.list"])
        for name in (SEGMENT_ONE, SEGMENT_TWO, ".system-identifier", "old-system-identifier", f"base/{OLD_BASE}",
                     f"base/{OLD_BASE}/backup_manifest"):
            self.assertIn(name, moved)
        listing = files["tree.list"].splitlines()
        for name in (".walarchive-marker", ".lock", ".pitr-scratch", ".system-identifier", f"base/{NEW_BASE}",
                     "base/.tmp-base_20260102_000000/part"):
            self.assertIn(f"walarchive-disk/walarchive/{name}", listing)
        for name in (SEGMENT_ONE, SEGMENT_TWO, f"base/{OLD_BASE}"):
            self.assertNotIn(f"walarchive-disk/walarchive/{name}", listing)
        self.assertEqual(files["read:walarchive-disk/walarchive/.system-identifier"], ARCHIVE_ID + "\n")
        self.assertEqual(files["read:walarchive-disk/walarchive.moved-*/.system-identifier"], "1111\n")

    HISTORY_TREE = {
        **WAL_TREE,
        "walarchive-disk/walarchive/00000002.history": "1\t0/3000000\tno recovery target specified\n",
        "walarchive-disk/walarchive/0000000A.history": "history ten",
        "walarchive-disk/walarchive/foo.history.tmp": "stray one",
        "walarchive-disk/walarchive/00000002.history.bak": "stray two",
        "walarchive-disk/walarchive/0000000b.history": "lower case",
        "walarchive-disk/walarchive/0002.history": "short",
        "walarchive-disk/walarchive/.00000003.history": "hidden",
        "walarchive-disk/walarchive/000000010000000000000003.history": "long",
    }
    HISTORY_MOVED = ("00000002.history", "0000000A.history")
    HISTORY_STAYS = ("foo.history.tmp", "00000002.history.bak", "0000000b.history", "0002.history",
                     ".00000003.history", "000000010000000000000003.history")

    def test_wal_line_moves_each_history_file_and_no_other_new_name(self):
        # ADR-0106 (k) and design.md D6. The grammar is ^[0-9A-F]{8}\.history$ at the top of /wal.
        done, _, files = self.run_wal_line(tree=self.HISTORY_TREE)
        self.assertEqual(done.returncode, 0, done.stderr)
        _, moved = self.moved_entries(files["tree.list"])
        listing = files["tree.list"].splitlines()
        for name in self.HISTORY_MOVED:
            self.assertIn(name, moved)
            self.assertNotIn(f"walarchive-disk/walarchive/{name}", listing)
        for name in self.HISTORY_STAYS:
            self.assertNotIn(name, moved)
            self.assertIn(f"walarchive-disk/walarchive/{name}", listing)
        manifest = files["wal-target.manifest"].splitlines()
        self.assertTrue(any(line.endswith("/00000002.history") for line in manifest))
        self.assertFalse(any(line.endswith("/00000002.history.bak") for line in manifest))

    def test_wal_line_moves_the_history_file_in_the_same_move_as_the_segments(self):
        done, log, _ = self.run_wal_line(tree=self.HISTORY_TREE)
        self.assertEqual(done.returncode, 0, done.stderr)
        moves = re.findall(r"^sudo mv -t \S+walarchive\.moved-\d{8}_\d{6} -- .+$", log, re.M)
        self.assertEqual(len(moves), 1, log)
        self.assertIn("/00000002.history", moves[0])
        self.assertIn(SEGMENT_ONE, moves[0])
        self.assertNotIn("foo.history.tmp", moves[0])

    def test_wal_line_stops_when_a_history_file_stays_in_the_archive_directory(self):
        # A move that leaves a file of the old line must stop the run before walarchive starts.
        body = shell_function("move_segments")
        self.assertIn("archive_history", body)
        self.assertIn("a history file stays in the archive directory", body)

    def test_the_history_expression_is_pinned(self):
        # design.md D6. The runbook, restore-switch.sh, and this file use the same expression.
        self.assertIn(r"HISTORY_REGEX='.*/[0-9A-F]{8}\.history'", switch_steps_text())
        body = shell_function("archive_history")
        self.assertIn("-regextype posix-extended -regex \"$HISTORY_REGEX\"", body)
        self.assertIn("-maxdepth 1 -type f", body)

    def test_wal_line_writes_the_state_files(self):
        done, _, files = self.run_wal_line()
        self.assertEqual(done.returncode, 0, done.stderr)
        target, _ = self.moved_entries(files["tree.list"])
        self.assertTrue(files["wal-target.dir"].strip().endswith(target))
        self.assertEqual(files["wal-old.bases"], OLD_BASE + "\n")
        self.assertEqual(files["wal-new.base"], NEW_BASE + "\n")
        manifest = files["wal-target.manifest"].splitlines()
        self.assertTrue(any(line.endswith(f"/{SEGMENT_ONE}") for line in manifest))
        self.assertTrue(any(line.endswith("/old-system-identifier") for line in manifest))
        self.assertTrue(any(line.endswith(f"/base/{OLD_BASE}/backup_manifest") for line in manifest))

    def test_wal_line_accepts_an_archive_without_the_base_directory(self):
        # basebackup-once creates /wal/base at its first run. Before that run, the old line has no base backup.
        tree = {key: value for key, value in WAL_TREE.items() if "/base/" not in key}
        done, _, files = self.run_wal_line(tree=tree)
        self.assertEqual(done.returncode, 0, done.stderr)
        self.assertEqual(files["wal-old.bases"], "")
        self.assertEqual(files["wal-new.base"], NEW_BASE + "\n")
        _, moved = self.moved_entries(files["tree.list"])
        self.assertIn("base", moved)
        self.assertFalse(any(name.startswith("base/") for name in moved))

    OLD_IDENTIFIER_READ = "read:walarchive-disk/walarchive.moved-*/old-system-identifier"

    def test_wal_line_copies_the_identifier_and_timeline_lines_of_the_change_log(self):
        # ADR-0106 (k). The file holds the identifier line, then the timeline line, in the form of the change log.
        done, _, files = self.run_wal_line(read=("walarchive-disk/walarchive.moved-*/old-system-identifier",))
        self.assertEqual(done.returncode, 0, done.stderr)
        self.assertEqual(files[self.OLD_IDENTIFIER_READ],
                         f"Database system identifier: 1111\n{TIMELINE_LABEL}: 1\n")

    def test_wal_line_copies_the_identifier_line_only_for_a_legacy_entry(self):
        # ADR-0106 (l). An entry from before the ADR holds no timeline line.
        done, _, files = self.run_wal_line(state={**WAL_STATE, "change.log": LEGACY_CHANGE_LOG},
                                           read=("walarchive-disk/walarchive.moved-*/old-system-identifier",))
        self.assertEqual(done.returncode, 0, done.stderr)
        self.assertEqual(files[self.OLD_IDENTIFIER_READ], "Database system identifier: 1111\n")

    def test_wal_line_copies_the_timeline_of_the_last_entry(self):
        # A switch back appends an entry. The cluster that the switch leaves is the last entry.
        log_text = LEGACY_CHANGE_LOG + f"volume: {BACK_NEW}\ntag: {JOB_TAG}\nDatabase system identifier: 1111\n{TIMELINE_LABEL}: 7\n"
        done, _, files = self.run_wal_line(state={**WAL_STATE, "change.log": log_text},
                                           read=("walarchive-disk/walarchive.moved-*/old-system-identifier",))
        self.assertEqual(done.returncode, 0, done.stderr)
        self.assertEqual(files[self.OLD_IDENTIFIER_READ],
                         f"Database system identifier: 1111\n{TIMELINE_LABEL}: 7\n")

    def test_wal_line_stops_when_the_timeline_of_the_change_log_is_malformed(self):
        for value in ("", "abc", "12345678901", "-1", "1 2"):
            with self.subTest(value=value):
                log_text = OLD_ENTRY.replace(f"{TIMELINE_LABEL}: 1", f"{TIMELINE_LABEL}: {value}")
                done, _, files = self.run_wal_line(state={**WAL_STATE, "change.log": log_text})
                self.assertEqual(done.returncode, 1)
                self.assertIn("CI: wal-line: the timeline in the change log is not valid.", done.stderr)
                self.assertNotIn("old-system-identifier", files["tree.list"])

    def test_wal_line_keeps_the_old_behavior_for_an_unreadable_timeline(self):
        # ADR-0106 (j). The switch writes the word unreadable. The file then holds the identifier line only.
        log_text = OLD_ENTRY.replace(f"{TIMELINE_LABEL}: 1", f"{TIMELINE_LABEL}: unreadable")
        done, _, files = self.run_wal_line(state={**WAL_STATE, "change.log": log_text},
                                           read=("walarchive-disk/walarchive.moved-*/old-system-identifier",))
        self.assertEqual(done.returncode, 0, done.stderr)
        self.assertEqual(files[self.OLD_IDENTIFIER_READ], "Database system identifier: 1111\n")

    def test_wal_line_writes_the_readable_timeline_of_the_change_log_and_not_the_timeline_of_pitr_volume(self):
        # ADR-0106 (k). A readable entry gives both lines from the change log. The line of pitr-volume has timeline 2.
        state = {**WAL_STATE, **pitr_state()}
        done, _, files = self.run_wal_line(state=state,
                                           read=("walarchive-disk/walarchive.moved-*/old-system-identifier",))
        self.assertEqual(done.returncode, 0, done.stderr)
        self.assertEqual(files[self.OLD_IDENTIFIER_READ],
                         f"Database system identifier: 1111\n{TIMELINE_LABEL}: 1\n")

    UNREADABLE_ENTRY = (f"volume: {OLD_VOLUME}\ntag: {JOB_TAG}\n"
                        f"Database system identifier: unreadable\n{TIMELINE_LABEL}: unreadable\n")

    def test_wal_line_uses_the_new_volume_when_the_last_entry_is_unreadable(self):
        # ADR-0106 (j), (k), (l), and (p). A recovery keeps the identifier. The line of pitr-volume gives the identifier only.
        # Its timeline belongs to the new cluster, so the file holds no timeline line.
        state = {**WAL_STATE, **pitr_state(), "change.log": self.UNREADABLE_ENTRY}
        tree = {**WAL_TREE, "walarchive-disk/walarchive/.system-identifier": "2222\n"}
        done, _, files = self.run_wal_line(state=state, tree=tree,
                                           read=("walarchive-disk/walarchive.moved-*/old-system-identifier",))
        self.assertEqual(done.returncode, 0, done.stderr)
        self.assertEqual(files[self.OLD_IDENTIFIER_READ], "Database system identifier: 2222\n")

    def test_wal_line_does_not_mix_entries_when_the_last_entry_is_unreadable(self):
        # An earlier entry holds an identifier line and a timeline line. The last entry decides, so the file holds no timeline line.
        state = {**WAL_STATE, **pitr_state(pitr_line(identifier="3333", timeline="4")),
                 "change.log": OLD_ENTRY + self.UNREADABLE_ENTRY}
        tree = {**WAL_TREE, "walarchive-disk/walarchive/.system-identifier": "3333\n"}
        done, _, files = self.run_wal_line(state=state, tree=tree,
                                           read=("walarchive-disk/walarchive.moved-*/old-system-identifier",))
        self.assertEqual(done.returncode, 0, done.stderr)
        self.assertEqual(files[self.OLD_IDENTIFIER_READ], "Database system identifier: 3333\n")

    def test_wal_line_does_not_take_the_identifier_of_an_earlier_entry(self):
        # The last entry has no identifier line at all. The identifier line of the first entry must not serve.
        log_text = OLD_ENTRY + f"volume: {BACK_NEW}\ntag: {JOB_TAG}\n"
        done, _, files = self.run_wal_line(state={**WAL_STATE, "change.log": log_text})
        self.assertEqual(done.returncode, 1)
        self.assertIn("CI: wal-line: the change log holds no identifier line.", done.stderr)
        self.assertNotIn("old-system-identifier", files["tree.list"])

    def test_wal_line_stops_when_the_last_entry_is_unreadable_and_the_line_of_pitr_volume_is_missing(self):
        state = {**WAL_STATE, "change.log": self.UNREADABLE_ENTRY}
        done, _, files = self.run_wal_line(state=state)
        self.assertEqual(done.returncode, 1)
        self.assertIn("CI: wal-line: the last line of pitr-volume is wrong.", done.stderr)
        self.assertNotIn("old-system-identifier", files["tree.list"])

    def test_wal_line_moves_the_files_when_the_archive_names_the_new_identifier_of_an_unreadable_entry(self):
        # require_old_identity_file compares .system-identifier with the same identifier as the file old-system-identifier.
        tree = {**WAL_TREE, "walarchive-disk/walarchive/.system-identifier": "2222\n"}
        state = {**WAL_STATE, **pitr_state(), "change.log": self.UNREADABLE_ENTRY}
        done, _, _ = self.run_wal_line(state=state, tree=tree)
        self.assertEqual(done.returncode, 0, done.stderr)

    def test_wal_line_runs_the_steps_in_order_and_deletes_nothing(self):
        done, log, _ = self.run_wal_line()
        self.assertEqual(done.returncode, 0, done.stderr)
        self.assertInOrder(log, [
            r" ps -aq walarchive\n",
            r"sudo mkdir -m 0700 -- \S+walarchive\.moved-\d{8}_\d{6}\n",
            r"sudo mv -t \S+walarchive\.moved-\d{8}_\d{6} -- .+\n",
            r" up -d --no-deps walarchive\n",
            r"SELECT state, flush_lsn FROM pg_stat_replication WHERE usename = 'recipe_replicator' AND application_name = 'walarchive'\n",
            r" exec -T walarchive basebackup-once\n",
            r"SELECT pg_switch_wal\(\)\n",
            r"SELECT EXISTS \(SELECT 1 FROM pg_stat_replication WHERE application_name = 'walarchive' AND usename = 'recipe_replicator' AND flush_lsn >= '0/6000000'::pg_lsn\)\n",
            rf" exec -T walarchive pg_verifybackup --quiet --skip-checksums --ignore=backup_end_time --wal-directory=/wal /wal/base/{NEW_BASE}\n",
        ])
        self.assertNotRegex(log, r"sudo rm|volume rm|\bdown\b|\bkill\b| compose .* (stop|rm) ")
        for line in ("CI: wal-line: step 1,", "CI: wal-line: step 2,", "CI: wal-line: step 3,", "CI: wal-line: step 4,",
                     "CI: wal-line: step 5,", "CI: wal-line: step 6,", "CI: wal-line: step 7,", "CI: wal-line: done"):
            self.assertIn(line, done.stdout)

    def test_wal_line_always_runs_the_stat_check_and_has_no_mount_point_skip(self):
        body = shell_function("make_target")
        self.assertIn("stat -c %d", body)
        self.assertNotIn("mountpoint", body)
        self.assertNotIn("mount point", body)
        done, _, _ = self.run_wal_line({"STUB_MOUNTPOINT_STATUS": "0"})
        self.assertEqual(done.returncode, 0, done.stderr)
        self.assertNotIn("mount point", done.stdout)

    def test_wal_line_stops_when_the_target_is_on_another_filesystem(self):
        done, _, files = self.run_wal_line({"STUB_STAT_TARGET_DEVICE": "99"})
        self.assertEqual(done.returncode, 1)
        self.assertIn("CI: wal-line: the target directory is on another filesystem than the archive directory.", done.stderr)
        self.assertNotIn("wal-target.manifest", files)

    def test_ci_wal_disk_is_the_parent_of_the_archive_directory(self):
        common = (CI / "common.sh").read_text(encoding="utf-8")
        prepare = (CI / "prepare.sh").read_text(encoding="utf-8")
        self.assertIn("export RECIPE_WALARCHIVE_DIR=$CI_ROOT/walarchive-disk/walarchive", common)
        self.assertIn("for disk in backups walarchive-disk", prepare)
        self.assertIn('sudo install -d -o 999 -g 999 -m 0700 "$RECIPE_WALARCHIVE_DIR"', prepare)
        for name in ("cleanup.sh", "upgrade.sh", "switch-steps.sh"):
            self.assertIn("for disk in backups walarchive-disk", (CI / name).read_text(encoding="utf-8"), name)

    def test_wal_line_needs_the_files_of_switch(self):
        done, log, _ = self.run_wal_line(state={})
        self.assertEqual(done.returncode, 1)
        self.assertIn("CI: wal-line: run switch first.", done.stderr)
        self.assertEqual(log, "")

    def test_wal_line_stops_when_the_walarchive_container_exists(self):
        done, log, files = self.run_wal_line({"STUB_WALARCHIVE_ID": "abc123"})
        self.assertEqual(done.returncode, 1)
        self.assertIn("CI: wal-line: the walarchive container exists. The switch must delete it.", done.stderr)
        self.assertNotIn("walarchive.moved", files["tree.list"])

    def test_wal_line_stops_when_the_file_names_another_cluster(self):
        tree = {**WAL_TREE, "walarchive-disk/walarchive/.system-identifier": "9999\n"}
        done, log, files = self.run_wal_line(tree=tree)
        self.assertEqual(done.returncode, 1)
        self.assertIn("CI: wal-line: the file .system-identifier does not hold the identifier of the old cluster.", done.stderr)
        self.assertIn(f"walarchive-disk/walarchive/{SEGMENT_ONE}", files["tree.list"].splitlines())
        self.assertNotIn(" up ", log)

    def test_wal_line_stops_without_a_streaming_row(self):
        for row in ("", "startup|0/5000060", "streaming|0/5000060\nstreaming|0/5000060"):
            with self.subTest(row=row):
                done, log, _ = self.run_wal_line({"STUB_STREAM_ROW": row})
                self.assertEqual(done.returncode, 1)
                self.assertIn("CI: wal-line: the stream check did not find one streaming row.", done.stderr)
                self.assertNotIn("basebackup-once", log)

    def test_wal_line_stops_when_the_flush_check_never_returns_t(self):
        done, log, _ = self.run_wal_line({"STUB_FLUSH": "f"})
        self.assertEqual(done.returncode, 1)
        self.assertIn("CI: wal-line: the flush check did not return t.", done.stderr)
        self.assertNotIn("pg_verifybackup", log)

    def test_wal_line_stops_when_the_wal_check_fails(self):
        done, _, files = self.run_wal_line({"STUB_VERIFY_FAIL": "1"})
        self.assertEqual(done.returncode, 1)
        self.assertIn("CI: wal-line: the WAL check failed.", done.stderr)
        self.assertNotIn("wal-new.base", files)

    def test_wal_line_stops_when_the_guard_wrote_no_file(self):
        done, _, _ = self.run_wal_line({"STUB_NO_IDENTITY": "1"})
        self.assertEqual(done.returncode, 1)
        self.assertIn("CI: wal-line: the file .system-identifier of the new line is missing or wrong.", done.stderr)

    def test_wal_line_stops_when_the_file_holds_another_identifier(self):
        done, _, _ = self.run_wal_line({"STUB_NEW_ID": "7777"})
        self.assertEqual(done.returncode, 1)
        self.assertIn("CI: wal-line: the file .system-identifier of the new line is missing or wrong.", done.stderr)


# The stub gives the minute 20260101_0000 to the first STUB_DATE_SWITCH calls and 20260101_0001 after them.
# It logs each minute that it gives. Other formats go to the real date.
DATE_MINUTE_STUB = r"""
if [[ $* != "-u +%Y%m%d_%H%M" ]]; then exec /usr/bin/date "$@"; fi
count=$(( $(cat "$STUB_DIR/date.count" 2>/dev/null || echo 0) + 1 ))
echo "$count" >"$STUB_DIR/date.count"
if (( count <= ${STUB_DATE_SWITCH:-2} )); then stamp=20260101_0000; else stamp=20260101_0001; fi
echo "date $stamp" >>"$STUB_LOG"
echo "$stamp"
"""


class CheckBehaviorTests(StubbedSubcommandTests):
    def run_check(self, stub_env=None, state=None, tree=None, extra_stubs=None):
        env = {**WAL_ENV, "STUB_BASE_NAME": CHECK_BASE, **(stub_env or {})}
        return self.run_subcommand("check", CHECK_STATE if state is None else state, env, DOCKER_WAL_STUB,
                                   tree=CHECK_TREE if tree is None else tree, sudo_stub=SUDO_EXEC_STUB, stub_python=True,
                                   extra_stubs=extra_stubs)

    def test_check_waits_for_a_minute_without_a_set_before_backup_now(self):
        # backup-once refuses a name of an existing set. Prepare wrote a _now set in the same minute in CI run 41092760611.
        for suffix in ("_now", "_premigrate"):
            with self.subTest(suffix=suffix):
                tree = {**CHECK_TREE, f"backups/recipe_20260101_0000{suffix}.dump.sha256": "manifest"}
                done, log, _ = self.run_check(tree=tree, extra_stubs={"date": DATE_MINUTE_STUB})
                self.assertEqual(done.returncode, 0, done.stderr)
                self.assertInOrder(log, [
                    r" exec -T walarchive basebackup-once\n",
                    r"date 20260101_0001\n",
                    r" exec -T backup backup-now\n",
                ])

    def test_check_runs_every_step_in_order(self):
        done, log, _ = self.run_check()
        self.assertEqual(done.returncode, 0, done.stderr)
        self.assertInOrder(log, [
            r"-c TRUNCATE public\.session, public\.login_attempt\n",
            r" up -d --no-deps --wait --wait-timeout 180 app\n",
            r" up -d --no-deps --wait --wait-timeout 180 proxy backup\n",
            r" exec -T proxy wget -q -T 3 -O - http://app:9000/readyz\n",
            r"python3 -I -B \S+http_probe\.py relogin\n",
            r"python3 -I -B \S+http_probe\.py export export-after\.sha256\n",
            r"SELECT count\(\*\) FROM public\.app_user\n",
            r"SELECT count\(\*\) FROM public\.audit_log\n",
            r"psql -X -At -v ON_ERROR_STOP=1 -d recipe\n",
            r"SELECT chain_seq, encode\(entry_hash, 'hex'\) FROM public\.audit_log ORDER BY chain_seq DESC LIMIT 1\n",
            r"SELECT pg_current_wal_lsn\(\)\n",
            r"flush_lsn >= '0/7000060'::pg_lsn\)\n",
            r"SELECT pg_walfile_name\('0/7000060'::pg_lsn - 1\)\n",
            rf" exec -T walarchive pg_verifybackup --quiet --skip-checksums --ignore=backup_end_time --wal-directory=/wal /wal/base/{NEW_BASE}\n",
            r" run --rm --no-deps -T -e PITR_CI=1 -e PITR_MARKER_SEQ -e PITR_MARKER_HASH pitr-check\n",
            r" logs --no-color --no-log-prefix db\n",
            r" exec -T walarchive basebackup-once\n",
            r" exec -T backup backup-now\n",
            rf"(?m)^volume inspect {OLD_VOLUME}\n",
        ])
        self.assertNotRegex(log, r"sudo rm|volume rm|\bdown\b|\bkill\b| compose .* (stop|rm) ")

    def test_check_saves_the_export_hash_next_to_the_old_one(self):
        done, _, files = self.run_check()
        self.assertEqual(done.returncode, 0, done.stderr)
        self.assertEqual(files["export-after.sha256"], EXPORT_HASH + "\n")
        self.assertEqual(files["row-counts.after"], COUNTS_FILE)

    def test_check_logs_one_line_for_each_step(self):
        done, _, _ = self.run_check()
        for number in range(1, 9):
            self.assertIn(f"CI: check: step {number},", done.stdout)
        self.assertIn("CI: check: done", done.stdout)

    def test_check_needs_the_files_of_wal_line(self):
        done, log, _ = self.run_check(state={})
        self.assertEqual(done.returncode, 1)
        self.assertIn("CI: check: run wal-line first.", done.stderr)
        self.assertEqual(log, "")

    def test_check_stops_when_readyz_does_not_answer_ok(self):
        done, log, _ = self.run_check({"STUB_READYZ": "bad"})
        self.assertEqual(done.returncode, 1)
        self.assertIn("CI: check: /readyz did not answer ok.", done.stderr)
        self.assertNotIn("relogin", log)

    def test_check_stops_when_the_login_fails(self):
        done, log, _ = self.run_check({"STUB_PROBE_FAIL": "relogin"})
        self.assertEqual(done.returncode, 1)
        self.assertIn("CI: check: the login failed.", done.stderr)
        self.assertNotIn("export", log)

    def test_check_stops_when_the_export_hash_differs(self):
        done, log, _ = self.run_check({"STUB_EXPORT_HASH": "cd" * 32})
        self.assertEqual(done.returncode, 1)
        self.assertIn("CI: check: the export hash differs from the hash before the switch.", done.stderr)
        self.assertNotIn("SELECT count", log)

    def test_check_accepts_new_audit_entries_only(self):
        done, _, _ = self.run_check({"STUB_COUNTS": COUNTS.replace("audit_log|10", "audit_log|13")})
        self.assertEqual(done.returncode, 0, done.stderr)
        for counts, table in ((COUNTS.replace("audit_log|10", "audit_log|9"), "audit_log"),
                              (COUNTS.replace("app_user|2", "app_user|3"), "app_user"),
                              (COUNTS.replace("parameter|5", "parameter|4"), "parameter")):
            with self.subTest(table=table):
                done, log, _ = self.run_check({"STUB_COUNTS": counts})
                self.assertEqual(done.returncode, 1)
                self.assertIn(f"CI: check: the row count of {table} ", done.stderr)
                self.assertNotIn("pg_verifybackup", log)

    def test_check_stops_when_the_chain_query_fails(self):
        for result in ("1|t|t", "0|f|t", "0|t|f", ""):
            with self.subTest(result=result):
                done, log, _ = self.run_check({"STUB_CHAIN": result})
                self.assertEqual(done.returncode, 1)
                self.assertIn("CI: check: the audit chain check failed.", done.stderr)
                self.assertNotIn("pitr-check", log)

    def test_check_stops_when_the_flush_check_never_returns_t(self):
        done, log, _ = self.run_check({"STUB_FLUSH": "f"})
        self.assertEqual(done.returncode, 1)
        self.assertIn("CI: check: the flush check did not return t.", done.stderr)
        self.assertNotIn("pitr-check", log)

    def test_check_stops_without_the_partial_file_of_the_marker_segment(self):
        tree = {key: value for key, value in CHECK_TREE.items() if not key.endswith(".partial")}
        done, log, _ = self.run_check(tree=tree)
        self.assertEqual(done.returncode, 1)
        self.assertIn("CI: check: the marker segment has no .partial file.", done.stderr)
        self.assertNotIn("pitr-check", log)

    def test_check_stops_when_the_wal_check_or_pitr_check_fails(self):
        for name, message in (("STUB_VERIFY_FAIL", "CI: check: the WAL check failed."),
                              ("STUB_PITR_FAIL", "CI: check: pitr-check failed.")):
            with self.subTest(name=name):
                done, log, _ = self.run_check({name: "1"})
                self.assertEqual(done.returncode, 1)
                self.assertIn(message, done.stderr)
                self.assertNotIn(" logs ", log)

    def test_check_stops_when_the_log_of_db_holds_a_password(self):
        done, log, _ = self.run_check({"STUB_DB_LOG": f"LOG: statement with {SECRET_VALUE} inside"})
        self.assertEqual(done.returncode, 1)
        self.assertIn("CI: check: the log of db holds 1 lines with a password.", done.stderr)
        self.assertNotIn(SECRET_VALUE, done.stdout + done.stderr)
        self.assertNotIn("basebackup-once", log)

    def test_check_stops_when_the_base_directory_holds_an_old_base_backup(self):
        tree = {**CHECK_TREE, f"walarchive-disk/walarchive/base/{OLD_BASE}/backup_manifest": "{}"}
        done, log, _ = self.run_check(tree=tree)
        self.assertEqual(done.returncode, 1)
        self.assertIn("CI: check: /wal/base holds a base backup of the old line.", done.stderr)
        self.assertNotIn("volume inspect", log)

    def test_check_stops_when_the_base_directory_holds_an_unknown_base_backup(self):
        tree = {**CHECK_TREE, "walarchive-disk/walarchive/base/base_20250101_000000/backup_manifest": "{}"}
        done, _, _ = self.run_check(tree=tree)
        self.assertEqual(done.returncode, 1)
        self.assertIn("CI: check: /wal/base holds a base backup that is not of the new line.", done.stderr)

    def test_check_stops_when_a_file_of_the_target_directory_is_gone(self):
        tree = {key: value for key, value in CHECK_TREE.items() if not key.endswith(f"{MOVED}/{SEGMENT_ONE}")}
        done, log, _ = self.run_check(tree=tree)
        self.assertEqual(done.returncode, 1)
        self.assertIn("CI: check: a file of the target directory is missing.", done.stderr)

    LEGACY_OLD_LOG = f"volume: {OLD_VOLUME}\ntag: {JOB_TAG}\nDatabase system identifier: 1111\n"
    IDENTIFIER_CHANGED = "CI: check: the file old-system-identifier of the target directory changed."

    def test_check_accepts_a_legacy_identifier_file_for_a_legacy_change_log(self):
        # A change log entry from before ADR-0106 has no timeline line. The file then holds the identifier line only.
        tree = {**CHECK_TREE, f"{MOVED}/old-system-identifier": "Database system identifier: 1111\n"}
        done, _, _ = self.run_check(state={**CHECK_STATE, "change.log": self.LEGACY_OLD_LOG}, tree=tree)
        self.assertEqual(done.returncode, 0, done.stderr)

    def test_check_stops_when_the_timeline_line_of_old_system_identifier_differs(self):
        for content in (f"Database system identifier: 1111\n{TIMELINE_LABEL}: 2\n", "Database system identifier: 1111\n"):
            with self.subTest(content=content):
                tree = {**CHECK_TREE, f"{MOVED}/old-system-identifier": content}
                done, _, _ = self.run_check(tree=tree)
                self.assertEqual(done.returncode, 1)
                self.assertIn(self.IDENTIFIER_CHANGED, done.stderr)

    def test_check_stops_when_a_legacy_change_log_meets_a_file_with_a_timeline_line(self):
        done, _, _ = self.run_check(state={**CHECK_STATE, "change.log": self.LEGACY_OLD_LOG})
        self.assertEqual(done.returncode, 1)
        self.assertIn(self.IDENTIFIER_CHANGED, done.stderr)

    def test_check_accepts_the_identifier_file_of_an_unreadable_last_entry(self):
        # ADR-0106 (j), (k), (l). The file holds the identifier line only. The line of pitr-volume gives the identifier.
        content = "Database system identifier: 2222\n"
        tree = {**CHECK_TREE, f"{MOVED}/old-system-identifier": content}
        log_text = (OLD_ENTRY + f"volume: {OLD_VOLUME}\ntag: {JOB_TAG}\n"
                    f"Database system identifier: unreadable\n{TIMELINE_LABEL}: unreadable\n")
        done, _, _ = self.run_check(state={**CHECK_STATE, **pitr_state(), "change.log": log_text}, tree=tree)
        self.assertEqual(done.returncode, 0, done.stderr)

    def test_check_stops_when_the_old_volume_is_gone(self):
        done, _, _ = self.run_check({"STUB_VOLUME_MISSING": "1"})
        self.assertEqual(done.returncode, 1)
        self.assertIn("CI: check: the old volume is gone.", done.stderr)


class RefuseTextTests(unittest.TestCase):
    """Check the order and the forbidden commands in the text of refuse."""

    def test_refuse_runs_the_three_cases_and_then_the_final_checks(self):
        body = shell_function("refuse")
        markers = ["row-counts.after", "refuse_live_volume", "refuse_missing_volume", "refuse_foreign_volume",
                   "record_counts", "cmp -s", "db_volume"]
        found = [body.find(marker) for marker in markers]
        self.assertNotIn(-1, found, dict(zip(markers, found)))
        self.assertEqual(found, sorted(found), dict(zip(markers, found)))

    def test_refuse_deletes_and_stops_nothing(self):
        text = "".join(shell_function(name) for name in (
            "refuse", "refuse_live_volume", "refuse_missing_volume", "refuse_foreign_volume", "make_foreign_volume"))
        self.assertNotRegex(text, r"docker volume (rm|prune)")
        self.assertNotRegex(text, r"compose (\S+ )*(start|down|rm|stop)\b")
        self.assertNotRegex(text, r"\bkill\b")
        self.assertNotIn("pg_ctl", text)

    def test_refuse_reuses_the_step_four_check_of_switch(self):
        self.assertIn("check_new_volume", shell_function("refuse_foreign_volume"))
        self.assertNotIn("Database system identifier", shell_function("refuse_foreign_volume"))

    def test_the_foreign_cluster_is_made_without_a_network(self):
        body = shell_function("make_foreign_volume")
        self.assertIn('docker volume create --label "recipe.ci=$COMPOSE_PROJECT_NAME"', body)
        self.assertIn("--network none", body)
        self.assertIn("--entrypoint initdb", body)


NEW_VOLUME = "recipe-editor_pgdata_20260101_000000"
RV_ERROR = '{"timestamp":"2026-01-01T00:00:00Z","level":"ERROR","job":"restore-volume","message":"the target directory is not empty"}'

DOCKER_REFUSE_STUB = r"""
echo "$*" >>"$STUB_LOG"
args=" $* "
case "$args" in
  *" compose "*" ps -aq db "*)
    if [[ -e $STUB_DIR/up.ran ]]; then echo "${STUB_DB_ID_AFTER:-abc123def}"; else echo abc123def; fi
    ;;
  *" compose "*" ps -aq "*)
    printf 'abc123def\nfff000\n'
    if [[ -e $STUB_DIR/up.ran && -n ${STUB_LIST_EXTRA:-} ]]; then echo "$STUB_LIST_EXTRA"; fi
    ;;
  *" compose "*" run "*" restore-volume "*)
    printf '%s\n' "${STUB_RV_OUT-$STUB_RV_ERROR}"
    exit "${STUB_RV_STATUS:-1}"
    ;;
  *" compose "*" up "*)
    touch "$STUB_DIR/up.ran"
    echo "VOL=${RECIPE_PGDATA_VOLUME:-}" >>"$STUB_LOG.env"
    if [[ -n ${STUB_UP_OUT+x} ]]; then echo "$STUB_UP_OUT" >&2
    else echo "external volume \"$RECIPE_PGDATA_VOLUME\" not found" >&2; fi
    exit "${STUB_UP_STATUS:-1}"
    ;;
  " volume inspect "*) exit "${STUB_VOLUME_INSPECT_STATUS:-1}" ;;
  " run "*" --entrypoint initdb "*) exit "${STUB_INITDB_STATUS:-0}" ;;
  " run "*)
    for arg in "$@"; do
      if [[ $arg == *:/var/lib/postgresql/data:ro ]]; then volume=${arg%%:*}; fi
    done
    case "$volume" in
      *_foreign_*) state=${STUB_FOREIGN_STATE:-shut down}; id=${STUB_FOREIGN_ID:-3333} ;;
      *) state="shut down"; id=2222 ;;
    esac
    echo "Database system identifier:           $id"
    echo "Database cluster state:               $state"
    ;;
  " inspect "*) echo "${STUB_INSPECT_VOLUME:-$STUB_NEW}" ;;
  *" compose "*" exec "*" psql "*" -c "*)
    table=${args##*FROM public.}; table=${table%% *}
    tr ';' '\n' <<<"${STUB_COUNTS:-}" | sed -n "s/^$table|//p"
    ;;
esac
exit 0
"""

REFUSE_STATE = {
    "active.volume": NEW_VOLUME + "\n",
    "new.system-identifier": "2222\n",
    "row-counts.after": COUNTS_FILE,
}


class RefuseBehaviorTests(StubbedSubcommandTests):
    def run_refuse(self, stub_env=None, state=None):
        env = {"STUB_COUNTS": COUNTS, "STUB_NEW": NEW_VOLUME, "STUB_RV_ERROR": RV_ERROR, **(stub_env or {})}
        return self.run_subcommand("refuse", REFUSE_STATE if state is None else state, env, DOCKER_REFUSE_STUB)

    def test_refuse_runs_the_three_cases_in_order(self):
        done, log, files = self.run_refuse()
        self.assertEqual(done.returncode, 0, done.stderr)
        self.assertInOrder(log, [
            r"ps -aq db\n",
            rf" run --rm --no-deps -T -v {NEW_VOLUME}:/var/lib/postgresql/data restore-volume 20260101_0000_now\n",
            rf"(?m)^volume inspect {PROJECT}_missing_\d{{8}}_\d{{6}}\n",
            r" up -d --no-deps db\n",
            rf"volume create --label recipe\.ci={PROJECT} recipe-editor_pgdata_foreign_\d{{8}}_\d{{6}}\n",
            r"run --rm --network none .*--entrypoint initdb ",
            r"run --rm --network none .*-v recipe-editor_pgdata_foreign_\d{8}_\d{6}:/var/lib/postgresql/data:ro .*pg_controldata.*\n",
            r"SELECT count\(\*\) FROM public\.app_user\n",
            r"SELECT count\(\*\) FROM public\.audit_log\n",
            r"(?m)^inspect .*abc123def\n",
        ])
        self.assertNotRegex(log, r"\b(down|kill|volume rm|volume prune)\b| (stop|rm|start) ")
        self.assertRegex(files["env.log"], rf"VOL={PROJECT}_missing_\d{{8}}_\d{{6}}")

    def test_refuse_logs_each_case_and_saves_the_foreign_volume(self):
        done, _, files = self.run_refuse()
        self.assertEqual(done.returncode, 0, done.stderr)
        self.assertRegex(files["foreign.volume"], r"^recipe-editor_pgdata_foreign_\d{8}_\d{6}\n$")
        for text in (
            "CI: refuse: case 1, restore-volume on the live volume " + NEW_VOLUME,
            "CI: refuse: case 1 stopped as expected: the target directory is not empty.",
            "CI: refuse: case 2 stopped as expected: Compose reports the missing external volume.",
            "CI: refuse: case 3 stopped as expected: the system identifier differs.",
            "CI: refuse: the live cluster keeps its row counts and db keeps its volume.",
            "CI: refuse: done",
        ):
            self.assertIn(text, done.stdout)

    def test_refuse_needs_the_files_of_check(self):
        for name in REFUSE_STATE:
            with self.subTest(name=name):
                state = {key: value for key, value in REFUSE_STATE.items() if key != name}
                done, log, _ = self.run_refuse(state=state)
                self.assertEqual(done.returncode, 1)
                self.assertIn("CI: refuse: run check first.", done.stderr)
                self.assertEqual(log, "")

    def test_case_one_fails_when_restore_volume_succeeds(self):
        line = '{"level":"INFO","message":"restore volume completed"}'
        done, log, _ = self.run_refuse({"STUB_RV_OUT": line, "STUB_RV_STATUS": "0"})
        self.assertEqual(done.returncode, 1)
        self.assertIn("CI: refuse: restore-volume on the live volume gave the exit code 0. Expected 1.", done.stderr)
        self.assertNotIn(" up ", log)

    def test_case_one_fails_for_another_reason(self):
        other = RV_ERROR.replace("the target directory is not empty", "the lock file of the restore test is missing")
        for output in (other, "", RV_ERROR + "\n" + RV_ERROR):
            with self.subTest(output=output):
                done, log, _ = self.run_refuse({"STUB_RV_OUT": output})
                self.assertEqual(done.returncode, 1)
                self.assertIn("CI: refuse: restore-volume on the live volume stopped for another reason.", done.stderr)
                self.assertNotIn(" up ", log)

    def test_case_one_fails_on_another_exit_code(self):
        done, _, _ = self.run_refuse({"STUB_RV_STATUS": "125"})
        self.assertEqual(done.returncode, 1)
        self.assertIn("gave the exit code 125. Expected 1.", done.stderr)

    def test_case_two_fails_when_compose_accepts_the_missing_volume(self):
        done, _, _ = self.run_refuse({"STUB_UP_STATUS": "0"})
        self.assertEqual(done.returncode, 1)
        self.assertIn("CI: refuse: Compose accepted the missing external volume.", done.stderr)

    def test_case_two_fails_for_another_reason(self):
        done, log, _ = self.run_refuse({"STUB_UP_OUT": "Error response from daemon: no such image"})
        self.assertEqual(done.returncode, 1)
        self.assertIn("CI: refuse: Compose stopped for another reason than the missing external volume.", done.stderr)
        self.assertNotIn("initdb", log)

    def test_case_two_fails_when_the_missing_volume_exists(self):
        done, log, _ = self.run_refuse({"STUB_VOLUME_INSPECT_STATUS": "0"})
        self.assertEqual(done.returncode, 1)
        self.assertIn("CI: refuse: the volume for case 2 exists.", done.stderr)
        self.assertNotIn(" up ", log)

    def test_case_two_fails_when_a_container_appears(self):
        done, _, _ = self.run_refuse({"STUB_LIST_EXTRA": "new999"})
        self.assertEqual(done.returncode, 1)
        self.assertIn("CI: refuse: the list of containers changed.", done.stderr)

    def test_case_two_fails_when_db_is_replaced(self):
        done, _, _ = self.run_refuse({"STUB_DB_ID_AFTER": "def456"})
        self.assertEqual(done.returncode, 1)
        self.assertIn("CI: refuse: the db container changed.", done.stderr)

    def test_case_three_fails_when_initdb_fails(self):
        done, _, _ = self.run_refuse({"STUB_INITDB_STATUS": "1"})
        self.assertEqual(done.returncode, 1)
        self.assertIn("CI: refuse: initdb of the foreign volume failed.", done.stderr)

    def test_case_three_fails_when_the_check_accepts_the_volume(self):
        done, _, _ = self.run_refuse({"STUB_FOREIGN_ID": "2222"})
        self.assertEqual(done.returncode, 1)
        self.assertIn("CI: refuse: the check accepted the volume of another system.", done.stderr)

    def test_case_three_fails_for_another_reason(self):
        done, _, _ = self.run_refuse({"STUB_FOREIGN_STATE": "in production"})
        self.assertEqual(done.returncode, 1)
        self.assertIn("CI: refuse: the check of the foreign volume stopped for another reason.", done.stderr)

    def test_the_final_check_fails_when_a_row_count_changes(self):
        done, _, files = self.run_refuse({"STUB_COUNTS": COUNTS.replace("recipe|1", "recipe|2")})
        self.assertEqual(done.returncode, 1)
        self.assertIn("CI: refuse: the row counts of the live cluster changed.", done.stderr)
        self.assertIn("recipe|2", files["row-counts.refuse"])

    def test_the_final_check_fails_when_db_mounts_another_volume(self):
        done, _, _ = self.run_refuse({"STUB_INSPECT_VOLUME": "other"})
        self.assertEqual(done.returncode, 1)
        self.assertIn("CI: refuse: db does not use the live volume.", done.stderr)


class SwitchBackTextTests(unittest.TestCase):
    """Check the order and the forbidden commands in the text of switch-back."""

    def positions(self, body, markers):
        found = [body.find(marker) for marker in markers]
        self.assertNotIn(-1, found, dict(zip(markers, found)))
        self.assertEqual(found, sorted(found), dict(zip(markers, found)))

    def test_switch_back_runs_the_wrong_run_before_the_right_run(self):
        self.positions(shell_function("switch_back"), [
            "run check and refuse first",
            "capture_roles new",
            "run_wrong_switch_back",
            "switch_back_core",
            "capture_roles old",
            "start_wal_line",
            "up -d --no-deps --wait --wait-timeout 180 app",
            "up -d --no-deps --wait --wait-timeout 180 proxy backup",
            "verify_export",
            "compare_cluster_values",
            '>"$CI_STATE/active.volume"',
        ])

    def test_the_core_follows_the_steps_of_adr_0103_f(self):
        self.positions(shell_function("switch_back_core"), [
            "stop_services stop",
            "controldata",
            "require_compose_volume",
            "check_new_volume",
            "start_db",
        ])

    def test_the_wrong_run_runs_in_a_subshell_with_a_copy_of_the_change_log(self):
        body = shell_function("run_wrong_switch_back")
        self.assertIn("change.log.wrong", body)
        self.assertIn("( set -e; switch_back_core", body)
        self.assertIn("ps -aq db", body)

    def test_switch_back_never_starts_deletes_or_kills(self):
        text = "".join(shell_function(name) for name in (
            "switch_back", "switch_back_core", "run_wrong_switch_back", "capture_roles", "read_config_files",
            "compare_cluster_values", "compare_whole", "require_volumes_stay", "check_first_target_stays"))
        self.assertNotRegex(text, r"compose (\S+ )*(start|down|kill)\b")
        self.assertNotRegex(text, r"docker (volume (rm|prune|create)|kill)")
        self.assertNotRegex(text, r"sudo rm|rm -r|-delete")

    def test_the_comparison_reads_whole_files_without_a_filter(self):
        text = "".join(shell_function(name) for name in (
            "read_config_files", "capture_roles", "compare_cluster_values", "compare_whole"))
        self.assertNotRegex(text, r"\bgrep\b|\bsed\b|--ignore|diff -[a-zA-Z]*[IBwb]\b|\bcomm\b")
        body = shell_function("read_config_files")
        self.assertIn("--network none", body)
        self.assertIn(':/var/lib/postgresql/data:ro"', body)
        self.assertIn("--entrypoint cat", body)
        self.assertIn("postgresql.conf postgresql.auto.conf", script_text())
        self.assertIn("restore-roles.sql", shell_function("capture_roles"))
        self.assertIn("sql -q", shell_function("capture_roles"))


BACK_NEW = "recipe-editor_pgdata_20260101_000000"
BACK_MOVED = "walarchive-disk/walarchive.moved-29990101_000000"
BACK_BASE = "base_20260401_000000"
BACK_TREE = {key.replace(MOVED, BACK_MOVED): value for key, value in CHECK_TREE.items()}
BACK_OLD_TAG = "ci-old-tag"
BACK_STATE = {
    **CHECK_STATE,
    "change.log": OLD_ENTRY.replace(JOB_TAG, BACK_OLD_TAG),
    "wal-target.dir": f"@RUN@/{BACK_MOVED}\n",
    "wal-target.manifest": CHECK_MANIFEST.replace(MOVED, BACK_MOVED),
    "row-counts.refuse": COUNTS_FILE,
}
BACK_ENV = {
    **WAL_ENV,
    "STUB_NEW_ID": "1111",
    "STUB_BASE_NAME": BACK_BASE,
    "STUB_REAL_PYTHON": sys.executable,
}
BACK_CASES = r"""
  *" compose "*" up "*" db "*)
    echo "VER=${RECIPE_VERSION:-}" >>"$STUB_LOG.env"
    touch "$STUB_DIR/db.exists"
    echo "$RECIPE_PGDATA_VOLUME" >"$STUB_DIR/db.volume"
    ;;
  *" compose "*" rm -f "*)
    rm -f "$STUB_DIR/db.exists"
    ;;
  *" compose "*" ps -aq db "*)
    if [[ -e $STUB_DIR/db.exists ]]; then echo abc123def; fi
    ;;
  " inspect "*) echo "${STUB_INSPECT_VOLUME:-$(cat "$STUB_DIR/db.volume")}" ;;
  " volume inspect "*)
    if [[ " ${STUB_MISSING_VOLUMES:-} " == *" ${*: -1} "* ]]; then exit 1; fi
    ;;
  *" compose "*" config --format json "*)
    printf '{"volumes":{"pgdata":{"name":"%s"}}}\n' "${STUB_CONFIG_VOLUME:-$RECIPE_PGDATA_VOLUME}"
    ;;
  *" compose "*" exec "*" psql "*" -q "*)
    cat >/dev/null
    echo "ROLE|postgres|t"
    echo "HBA|local all all peer"
    if [[ $(cat "$STUB_DIR/db.volume") == "$STUB_OLD" ]]; then
      if [[ -n ${STUB_ROLES_OLD_EXTRA:-} ]]; then echo "$STUB_ROLES_OLD_EXTRA"; fi
    elif [[ -n ${STUB_ROLES_NEW_EXTRA:-} ]]; then
      echo "$STUB_ROLES_NEW_EXTRA"
    fi
    ;;
  " run "*" --entrypoint cat "*)
    for arg in "$@"; do
      if [[ $arg == *:/var/lib/postgresql/data:ro ]]; then volume=${arg%%:*}; fi
    done
    file=${*: -1}; file=${file##*/}
    echo "# $file"
    echo "listen_addresses = '*'"
    if [[ $volume != "$STUB_OLD" && $file == "${STUB_CONF_DIFF:-none}" ]]; then echo "max_connections = 5"; fi
    ;;
  " run "*)
    for arg in "$@"; do
      if [[ $arg == *:/var/lib/postgresql/data:ro ]]; then volume=${arg%%:*}; fi
    done
    if [[ $volume == "$STUB_OLD" ]]; then
      state=${STUB_OLD_STATE:-shut down}; id=${STUB_OLD_ID:-1111}; timeline=${STUB_OLD_TL-1}
    else
      if [[ -n ${STUB_NEW_CONTROL_FAIL:-} ]]; then exit 1; fi
      state=${STUB_NEW_STATE:-shut down}; id=2222; timeline=${STUB_NEW_TL-2}
    fi
    echo "Database system identifier:           $id"
    echo "Database cluster state:               $state"
    if [[ -n $timeline ]]; then echo "Latest checkpoint's TimeLineID:       $timeline"; fi
    ;;
"""
DOCKER_BACK_STUB = DOCKER_WAL_STUB.replace('case "$args" in\n', 'case "$args" in' + BACK_CASES, 1)
BACK_PYTHON_STUB = 'case " $* " in *" volume-name "*) exec "$STUB_REAL_PYTHON" "$@" ;; esac\n' + PYTHON_STUB.lstrip("\n")
BACK_RIGHT_CHANGE_LOG = (OLD_ENTRY.replace(JOB_TAG, BACK_OLD_TAG)
                         + f"volume: {BACK_NEW}\ntag: {JOB_TAG}\nDatabase system identifier: 2222\n{TIMELINE_LABEL}: 2\n")
# A change log entry from before ADR-0106 has no timeline line.
LEGACY_CHANGE_LOG = f"volume: {OLD_VOLUME}\ntag: {BACK_OLD_TAG}\nDatabase system identifier: 1111\n"


class SwitchBackBehaviorTests(StubbedSubcommandTests):
    def run_back(self, stub_env=None, state=None, tree=None, read=()):
        env = {**BACK_ENV, "STUB_MISSING_VOLUMES": "", **(stub_env or {})}
        return self.run_subcommand(
            "switch-back", BACK_STATE if state is None else state, env, DOCKER_BACK_STUB,
            tree=BACK_TREE if tree is None else tree, sudo_stub=SUDO_EXEC_STUB, stub_python=True,
            python_stub=BACK_PYTHON_STUB, read=read, stub_files={"db.volume": BACK_NEW + "\n"})

    def volume_run(self, volume, tool):
        return rf"run --rm --network none .*-v {volume}:/var/lib/postgresql/data:ro .*{tool}.*\n"

    def test_the_right_run_follows_adr_0103_g_in_order(self):
        done, log, _ = self.run_back()
        self.assertEqual(done.returncode, 0, done.stderr)
        stop = r" stop proxy app backup walarchive db\n"
        remove = r" rm -f proxy app backup walarchive db\n"
        cat = r"--entrypoint cat .*/var/lib/postgresql/data/{}\n"
        self.assertInOrder(log, [
            r"exec -T -u postgres db psql -X -At -v ON_ERROR_STOP=1 -d recipe -q\n",
            stop, remove, self.volume_run(BACK_NEW, "pg_controldata"), r" config --format json\n",
            self.volume_run(OLD_VOLUME, "pg_controldata"),
            stop, remove, self.volume_run(BACK_NEW, "pg_controldata"),
            self.volume_run(BACK_NEW, r"--entrypoint cat .*/postgresql\.conf"),
            self.volume_run(BACK_NEW, r"--entrypoint cat .*/postgresql\.auto\.conf"),
            r" config --format json\n",
            self.volume_run(OLD_VOLUME, "pg_controldata"),
            self.volume_run(OLD_VOLUME, r"--entrypoint cat .*/postgresql\.conf"),
            self.volume_run(OLD_VOLUME, r"--entrypoint cat .*/postgresql\.auto\.conf"),
            r" up -d --no-deps --wait --wait-timeout 180 db\n",
            r"(?m)^inspect .*abc123def\n",
            r"exec -T -u postgres db psql -X -At -v ON_ERROR_STOP=1 -d recipe -q\n",
            r" ps -aq walarchive\n",
            r"sudo mkdir -m 0700 -- \S+walarchive\.moved-\d{8}_\d{6}\n",
            r"sudo mv -t \S+walarchive\.moved-\d{8}_\d{6} -- .+\n",
            r" up -d --no-deps walarchive\n",
            r" exec -T walarchive basebackup-once\n",
            r" exec -T walarchive pg_verifybackup .*/wal/base/" + BACK_BASE + r"\n",
            r" up -d --no-deps --wait --wait-timeout 180 app\n",
            r" up -d --no-deps --wait --wait-timeout 180 proxy backup\n",
            r" exec -T proxy wget -q -T 3 -O - http://app:9000/readyz\n",
            r"python3 -I -B \S+http_probe\.py relogin\n",
            r"python3 -I -B \S+http_probe\.py export export-back\.sha256\n",
            r"SELECT count\(\*\) FROM public\.app_user\n",
        ])
        self.assertNotRegex(log, r"compose .* (start|down)\b|volume rm|\bkill\b|sudo rm")

    def test_the_wrong_run_starts_no_db_and_the_right_run_starts_one(self):
        done, log, files = self.run_back()
        self.assertEqual(done.returncode, 0, done.stderr)
        stop = " stop proxy app backup walarchive db\n"
        self.assertEqual(log.count(stop), 2)
        wrong_part = log.split(stop)[1]
        self.assertNotIn(" up ", wrong_part)
        self.assertIn(" ps -aq db\n", wrong_part)
        self.assertEqual(log.count(" up -d --no-deps --wait --wait-timeout 180 db\n"), 1)
        self.assertEqual(log.count("volume inspect"), 4)
        self.assertTrue(files["switch-back-wrong.log"].endswith(
            "CI: switch-back: the system identifier of the new volume differs from the change log.\n"))
        self.assertIn("Database system identifier: 1\n", files["change.log.wrong"])
        for line in ("CI: switch-back: part 2, run with a wrong identifier in a copy of the change log",
                     "CI: switch-back: part 2 stopped as expected: the system identifier differs.",
                     "CI: switch-back: part 3, the switch back with the right identifier",
                     "CI: switch-back: step 5, start db on the old volume",
                     "CI: switch-back: done"):
            self.assertIn(line, done.stdout)

    def test_the_right_run_writes_the_leaving_volume_into_the_change_log(self):
        done, _, files = self.run_back()
        self.assertEqual(done.returncode, 0, done.stderr)
        self.assertEqual(files["change.log"], BACK_RIGHT_CHANGE_LOG)
        self.assertEqual(files["active.volume"], OLD_VOLUME + "\n")

    def test_the_right_run_writes_the_timeline_of_the_leaving_volume(self):
        done, _, files = self.run_back({"STUB_NEW_TL": "5"})
        self.assertEqual(done.returncode, 0, done.stderr)
        self.assertEqual(files["change.log"], BACK_RIGHT_CHANGE_LOG.replace(f"{TIMELINE_LABEL}: 2", f"{TIMELINE_LABEL}: 5"))

    def test_the_run_stops_when_pg_controldata_shows_no_timeline_of_the_leaving_volume(self):
        done, log, files = self.run_back({"STUB_NEW_TL": ""})
        self.assertEqual(done.returncode, 1)
        self.assertIn("CI: switch-back: pg_controldata shows no timeline of the volume that the run leaves.", done.stdout)
        self.assertNotIn(" up ", log)
        self.assertEqual(files["active.volume"], BACK_NEW + "\n")

    def test_the_identifier_matches_but_the_timeline_differs_stops_the_run_before_db(self):
        # ADR-0106 (l). A recovery keeps the identifier and raises the timeline.
        for name, value in (("another timeline", "3"), ("no timeline on the volume", "")):
            with self.subTest(name=name):
                done, log, files = self.run_back({"STUB_OLD_TL": value})
                self.assertEqual(done.returncode, 1)
                self.assertTrue(done.stderr.strip().endswith(
                    "CI: switch-back: the timeline of the new volume differs from the change log."), done.stderr)
                self.assertNotIn(" up ", log)
                self.assertNotIn("volume rm", log)
                self.assertEqual(files["active.volume"], BACK_NEW + "\n")

    def test_an_entry_without_a_timeline_compares_the_identifier_only(self):
        # ADR-0106 (l). An entry from before ADR-0106 holds no timeline. Any timeline of the volume passes.
        for timeline in ("1", "7"):
            with self.subTest(timeline=timeline):
                done, _, files = self.run_back({"STUB_OLD_TL": timeline}, state={**BACK_STATE, "change.log": LEGACY_CHANGE_LOG})
                self.assertEqual(done.returncode, 0, done.stderr)
                self.assertEqual(files["active.volume"], OLD_VOLUME + "\n")
                self.assertTrue(files["change.log"].startswith(LEGACY_CHANGE_LOG))

    def test_an_entry_without_a_timeline_still_compares_the_identifier(self):
        done, log, _ = self.run_back({"STUB_OLD_ID": "9999"}, state={**BACK_STATE, "change.log": LEGACY_CHANGE_LOG})
        self.assertEqual(done.returncode, 1)
        self.assertTrue(done.stderr.strip().endswith(
            "CI: switch-back: the system identifier of the new volume differs from the change log."))
        self.assertNotIn(" up ", log)

    def test_the_timeline_comes_from_the_first_entry_only(self):
        # The first entry is legacy. A later entry with a timeline must not decide the check.
        later = LEGACY_CHANGE_LOG + f"volume: {BACK_NEW}\ntag: other-tag\nDatabase system identifier: 2222\n{TIMELINE_LABEL}: 9\n"
        done, _, files = self.run_back({"STUB_OLD_TL": "7"}, state={**BACK_STATE, "change.log": later})
        self.assertEqual(done.returncode, 0, done.stderr)
        self.assertEqual(files["active.volume"], OLD_VOLUME + "\n")

    def test_a_malformed_timeline_in_the_change_log_stops_the_run_before_any_container(self):
        base = f"volume: {OLD_VOLUME}\ntag: {BACK_OLD_TAG}\nDatabase system identifier: 1111\n"
        for name, line in (("empty value", f"{TIMELINE_LABEL}: \n"), ("letters", f"{TIMELINE_LABEL}: x1\n"),
                           ("no space", f"{TIMELINE_LABEL}:1\n"), ("too long", f"{TIMELINE_LABEL}: 12345678901\n")):
            with self.subTest(name=name):
                text = base + line
                done, log, files = self.run_back(state={**BACK_STATE, "change.log": text})
                self.assertEqual(done.returncode, 1)
                self.assertIn("CI: switch-back: the timeline in the change log is not valid.", done.stderr)
                self.assertEqual(log, "")
                self.assertEqual(files["change.log"], text)

    def test_the_right_run_sets_the_tag_of_the_old_volume_before_it_starts_db(self):
        done, log, files = self.run_back()
        self.assertEqual(done.returncode, 0, done.stderr)
        self.assertEqual(files["env.log"].count("VER="), 1)
        self.assertIn(f"VER={BACK_OLD_TAG}\n", files["env.log"])
        image = r"-v {}:/var/lib/postgresql/data:ro .*recipe-pgtools:{}\b"
        self.assertInOrder(log, [
            image.format(BACK_NEW, JOB_TAG),
            image.format(OLD_VOLUME, BACK_OLD_TAG),
            r" up -d --no-deps --wait --wait-timeout 180 db\n",
        ])
        self.assertNotRegex(log, image.format(OLD_VOLUME, JOB_TAG))

    def test_the_tag_of_the_old_volume_comes_from_the_first_entry(self):
        later = BACK_STATE["change.log"] + f"volume: {BACK_NEW}\ntag: other-tag\nDatabase system identifier: 2222\n"
        done, _, files = self.run_back(state={**BACK_STATE, "change.log": later})
        self.assertEqual(done.returncode, 0, done.stderr)
        self.assertIn(f"VER={BACK_OLD_TAG}\n", files["env.log"])

    def test_a_bad_tag_in_the_change_log_stops_the_run_before_any_container(self):
        first = f"volume: {OLD_VOLUME}\n{{}}Database system identifier: 1111\n"
        later_only = first.format("") + f"volume: {BACK_NEW}\ntag: {BACK_OLD_TAG}\nDatabase system identifier: 2222\n"
        missing = "CI: switch-back: the change log holds no tag line for the old volume."
        illegal = "CI: switch-back: the tag in the change log is not a legal tag."
        cases = {
            "no tag line": (first.format(""), missing),
            "empty tag": (first.format("tag: \n"), missing),
            "tag only in a later entry": (later_only, missing),
            "space in the tag": (first.format("tag: bad tag\n"), illegal),
            "slash in the tag": (first.format("tag: a/b\n"), illegal),
            "leading dash": (first.format("tag: -x\n"), illegal),
            "tag too long": (first.format("tag: " + "a" * 129 + "\n"), illegal),
        }
        for name, (log_text, message) in cases.items():
            with self.subTest(name=name):
                done, log, files = self.run_back(state={**BACK_STATE, "change.log": log_text})
                self.assertEqual(done.returncode, 1)
                self.assertIn(message, done.stderr)
                self.assertEqual(log, "")
                self.assertEqual(files["change.log"], log_text)

    def test_the_wrong_run_leaves_the_real_change_log_alone(self):
        done, _, files = self.run_back({"STUB_OLD_ID": "9999"})
        self.assertEqual(done.returncode, 1)
        self.assertEqual(files["change.log.wrong"].splitlines()[:3],
                         [f"volume: {OLD_VOLUME}", f"tag: {BACK_OLD_TAG}", "Database system identifier: 1"])
        self.assertTrue(files["change.log"].startswith(OLD_ENTRY.replace(JOB_TAG, BACK_OLD_TAG)))

    def test_the_new_line_gets_a_second_target_directory(self):
        done, _, files = self.run_back(read=(
            "walarchive-disk/walarchive/.system-identifier", "walarchive-disk/walarchive.moved-20*/old-system-identifier"))
        self.assertEqual(done.returncode, 0, done.stderr)
        listing = files["tree.list"]
        targets = re.findall(r"^walarchive-disk/walarchive\.moved-\d{8}_\d{6}$", listing, re.M)
        self.assertEqual(len(targets), 2, listing)
        self.assertIn(BACK_MOVED, targets)
        self.assertIn(f"{BACK_MOVED}/old-system-identifier", listing)
        self.assertIn(f"{BACK_MOVED}/{SEGMENT_ONE}", listing)
        self.assertEqual(files["read:walarchive-disk/walarchive/.system-identifier"], "1111\n")
        self.assertEqual(files["read:walarchive-disk/walarchive.moved-20*/old-system-identifier"],
                         f"Database system identifier: 2222\n{TIMELINE_LABEL}: 2\n")
        self.assertIn(f"walarchive-disk/walarchive/base/{BACK_BASE}", listing)
        self.assertNotIn(f"walarchive-disk/walarchive/base/{NEW_BASE}", listing)
        self.assertEqual(files["wal-new-back.base"], BACK_BASE + "\n")
        self.assertEqual(files["wal-old-back.bases"], NEW_BASE + "\n")
        self.assertTrue(files["wal-target-back.dir"].strip().endswith(tuple(t for t in targets if t != BACK_MOVED)))
        self.assertEqual(files["wal-target.dir"], f"@RUN@/{BACK_MOVED}\n".replace("@RUN@", files["wal-target.dir"].split("/walarchive-disk")[0]))

    def test_the_files_of_the_two_clusters_are_saved_after_each_stop(self):
        done, _, files = self.run_back()
        self.assertEqual(done.returncode, 0, done.stderr)
        for kind in ("old", "new"):
            self.assertEqual(files[f"state.{kind}"], "shut down\n")
            self.assertIn("# postgresql.conf\n", files[f"config.{kind}.postgresql.conf"])
            self.assertIn("# postgresql.auto.conf\n", files[f"config.{kind}.postgresql.auto.conf"])
            self.assertIn("HBA|local all all peer\n", files[f"roles.{kind}"])
        self.assertEqual(files["export-back.sha256"], EXPORT_HASH + "\n")
        self.assertEqual(files["row-counts.back"], COUNTS_FILE)

    def test_switch_back_needs_the_files_of_check_and_refuse(self):
        for name in ("change.log", "old.volume", "active.volume", "export.sha256", "row-counts", "wal-target.dir",
                     "wal-target.manifest", "new.system-identifier", "row-counts.refuse"):
            with self.subTest(name=name):
                state = {key: value for key, value in BACK_STATE.items() if key != name}
                done, log, _ = self.run_back(state=state)
                self.assertEqual(done.returncode, 1)
                self.assertIn("CI: switch-back: run check and refuse first.", done.stderr)
                self.assertEqual(log, "")

    def test_a_wrong_run_that_does_not_stop_fails_the_job(self):
        done, _, _ = self.run_back({"STUB_OLD_ID": "1"})
        self.assertEqual(done.returncode, 1)
        self.assertIn("CI: switch-back: the run with the wrong identifier did not stop.", done.stderr)

    def test_a_wrong_run_that_stops_for_another_reason_fails_the_job(self):
        done, log, _ = self.run_back({"STUB_OLD_STATE": "in production"})
        self.assertEqual(done.returncode, 1)
        self.assertIn("CI: switch-back: the run with the wrong identifier stopped for another reason.", done.stderr)
        self.assertNotIn(" up ", log)

    def test_a_wrong_run_needs_both_volumes(self):
        for volume in (OLD_VOLUME, BACK_NEW):
            with self.subTest(volume=volume):
                done, log, _ = self.run_back({"STUB_MISSING_VOLUMES": volume})
                self.assertEqual(done.returncode, 1)
                self.assertIn("CI: switch-back: a data volume is gone.", done.stderr)
                self.assertNotIn(" up ", log)

    def test_the_right_run_stops_on_a_wrong_identifier_and_starts_no_db(self):
        done, log, files = self.run_back({"STUB_OLD_ID": "9999"})
        self.assertEqual(done.returncode, 1)
        self.assertTrue(done.stderr.strip().endswith(
            "CI: switch-back: the system identifier of the new volume differs from the change log."))
        self.assertEqual(log.count(" stop proxy app backup walarchive db\n"), 2)
        self.assertNotIn(" up ", log)
        self.assertNotIn("volume rm", log)
        self.assertEqual(files["active.volume"], BACK_NEW + "\n")

    def test_the_right_run_stops_when_compose_shows_another_volume(self):
        done, log, _ = self.run_back({"STUB_CONFIG_VOLUME": "other"})
        self.assertEqual(done.returncode, 1)
        self.assertIn("CI: switch-back: Compose does not show the new volume for pgdata.", done.stdout)
        self.assertIn("stopped for another reason", done.stderr)
        self.assertNotIn(" up ", log)

    def test_the_run_needs_no_state_of_the_leaving_volume_but_needs_its_identifier(self):
        done, log, _ = self.run_back({"STUB_NEW_CONTROL_FAIL": "1"})
        self.assertEqual(done.returncode, 1)
        self.assertIn("CI: switch-back: pg_controldata shows no system identifier of the volume that the run leaves.",
                      done.stdout)
        self.assertIn("stopped for another reason", done.stderr)
        self.assertNotIn(" up ", log)

    def test_the_right_run_stops_when_db_mounts_another_volume(self):
        done, _, _ = self.run_back({"STUB_INSPECT_VOLUME": "other"})
        self.assertEqual(done.returncode, 1)
        self.assertIn("CI: switch-back: db does not use the new volume.", done.stderr)

    def test_walarchive_stops_the_run_when_its_container_exists(self):
        done, log, _ = self.run_back({"STUB_WALARCHIVE_ID": "abc123"})
        self.assertEqual(done.returncode, 1)
        self.assertIn("CI: switch-back wal-line: the walarchive container exists. The switch must delete it.", done.stderr)
        self.assertNotIn(" up -d --no-deps --wait --wait-timeout 180 app", log)

    def test_a_missing_file_of_the_first_target_directory_stops_the_run(self):
        tree = {key: value for key, value in BACK_TREE.items() if not key.endswith(f"{BACK_MOVED}/{SEGMENT_ONE}")}
        done, _, _ = self.run_back(tree=tree)
        self.assertEqual(done.returncode, 1)
        self.assertIn("CI: switch-back: a file of the first target directory is missing.", done.stderr)

    def test_the_data_checks_fail_on_each_difference(self):
        done, log, _ = self.run_back({"STUB_EXPORT_HASH": "cd" * 32})
        self.assertEqual(done.returncode, 1)
        self.assertIn("CI: switch-back: the export hash differs from the hash before the switch.", done.stderr)
        self.assertNotIn("SELECT count", log)
        for counts, table in ((COUNTS.replace("audit_log|10", "audit_log|9"), "audit_log"),
                              (COUNTS.replace("app_user|2", "app_user|3"), "app_user")):
            with self.subTest(table=table):
                done, _, _ = self.run_back({"STUB_COUNTS": counts})
                self.assertEqual(done.returncode, 1)
                self.assertIn(f"CI: switch-back: the row count of {table} ", done.stderr)

    def test_audit_log_may_only_grow_after_the_login_and_the_export(self):
        done, _, _ = self.run_back({"STUB_COUNTS": COUNTS.replace("audit_log|10", "audit_log|12")})
        self.assertEqual(done.returncode, 0, done.stderr)

    def test_readyz_and_login_failures_stop_the_run(self):
        done, _, _ = self.run_back({"STUB_READYZ": "bad"})
        self.assertEqual(done.returncode, 1)
        self.assertIn("CI: switch-back: /readyz did not answer ok.", done.stderr)
        done, _, _ = self.run_back({"STUB_PROBE_FAIL": "relogin"})
        self.assertEqual(done.returncode, 1)
        self.assertIn("CI: switch-back: the login failed.", done.stderr)

    def test_a_difference_in_a_configuration_file_fails_with_the_first_difference(self):
        for name in ("postgresql.conf", "postgresql.auto.conf"):
            with self.subTest(name=name):
                done, _, files = self.run_back({"STUB_CONF_DIFF": name})
                self.assertEqual(done.returncode, 1)
                self.assertIn(f"CI: switch-back: {name} of the old cluster and the new cluster differ.", done.stderr)
                self.assertIn("> max_connections = 5", done.stderr)
                self.assertIn("first difference", done.stderr)
                self.assertEqual(files["active.volume"], BACK_NEW + "\n")

    def test_a_difference_in_the_roles_output_fails_with_the_first_difference(self):
        done, _, _ = self.run_back({"STUB_ROLES_OLD_EXTRA": "ROLE|extra|t"})
        self.assertEqual(done.returncode, 1)
        self.assertIn("CI: switch-back: the output of restore-roles.sql of the old cluster and the new cluster differ.",
                      done.stderr)
        self.assertIn("< ROLE|extra|t", done.stderr)

    def test_the_comparison_needs_the_clean_stop_of_each_cluster(self):
        done, _, _ = self.run_back({"STUB_NEW_STATE": "in production"})
        self.assertEqual(done.returncode, 1)
        self.assertIn("CI: switch-back: the new cluster was not in the state 'shut down' when its files were read.",
                      done.stderr)


# The scan of deploy_values lists each line with the stop and the volume option. These tests build the text from parts.
DOWN_V = "down" + " -" + "v"
KEEP_DOWN_LINE = f'compose --profile restore --profile pitr {DOWN_V} --remove-orphans || fail "keep: the project stop failed."'
KEEP_STATE = {
    "old.volume": OLD_VOLUME + "\n",
    "new.volume": BACK_NEW + "\n",
    "active.volume": OLD_VOLUME + "\n",
}
DOCKER_KEEP_STUB = r"""
echo "$*" >>"$STUB_LOG"
echo "VOL=${RECIPE_PGDATA_VOLUME:-}" >>"$STUB_LOG.env"
args=" $* "
case "$args" in
  *" compose "*" down "*) exit "${STUB_DOWN_STATUS:-0}" ;;
  *" compose "*" ps -aq "*)
    if [[ -n ${STUB_PS_OUTPUT:-} ]]; then echo "$STUB_PS_OUTPUT"; fi
    ;;
  " volume inspect "*)
    if [[ " ${STUB_MISSING_VOLUMES:-} " == *" ${*: -1} "* ]]; then exit 1; fi
    echo "${STUB_LABEL:-recipe-ci-1-1}"
    ;;
esac
exit 0
"""


class KeepTextTests(unittest.TestCase):
    def test_keep_runs_the_exact_down_line_before_the_inspects(self):
        body = shell_function("keep")
        self.assertEqual(body.count(DOWN_V), 1)
        self.assertIn("  " + KEEP_DOWN_LINE + "\n", body + "\n")
        self.assertLess(body.index(DOWN_V), body.index("require_labelled_volume"))
        self.assertIn("docker volume inspect", shell_function("require_labelled_volume"))

    def test_keep_deletes_no_volume_itself(self):
        text = shell_function("keep") + shell_function("require_labelled_volume")
        self.assertNotIn("delete_ci_volumes", text)
        self.assertNotRegex(text, r"docker volume (rm|prune|create)|docker (kill|rm)\b|sudo rm")

    def test_cleanup_checks_for_a_labelled_volume_after_the_deletion(self):
        body = shell_function("cleanup")
        self.assertLess(body.index("delete_ci_volumes || status=$?"), body.index("require_no_labelled_volume || status=$?"))
        helper = shell_function("require_no_labelled_volume")
        self.assertIn('docker volume ls -q --filter "label=recipe.ci=$COMPOSE_PROJECT_NAME"', helper)
        self.assertIn("CI: cleanup: a volume with the label", helper)


class KeepBehaviorTests(StubbedSubcommandTests):
    def run_keep(self, stub_env=None, state=None):
        return self.run_subcommand(
            "keep", KEEP_STATE if state is None else state, stub_env or {}, DOCKER_KEEP_STUB, sudo_stub=SUDO_EXEC_STUB)

    def test_keep_stops_every_container_then_inspects_both_data_volumes(self):
        done, log, files = self.run_keep()
        self.assertEqual(done.returncode, 0, done.stderr)
        inspect = r"volume inspect --format \{\{index \.Labels \"recipe\.ci\"\}\} "
        self.assertInOrder(log, [
            r" --profile restore --profile pitr " + DOWN_V + r" --remove-orphans\n",
            r" --profile restore --profile pitr ps -aq\n",
            inspect + re.escape(OLD_VOLUME) + r"\n",
            inspect + re.escape(BACK_NEW) + r"\n",
        ])
        self.assertEqual(log.count("volume inspect"), 2)
        self.assertNotRegex(log, r"volume (rm|prune|create)|\bkill\b|\brm\b")
        self.assertIn(f"VOL={OLD_VOLUME}\n", files["env.log"])
        self.assertIn("CI: keep: done", done.stdout)

    def test_keep_fails_when_a_data_volume_is_gone(self):
        for volume in (OLD_VOLUME, BACK_NEW):
            with self.subTest(volume=volume):
                done, log, _ = self.run_keep({"STUB_MISSING_VOLUMES": volume})
                self.assertEqual(done.returncode, 1)
                self.assertIn(f"CI: keep: the data volume {volume} is gone.", done.stderr)
                self.assertIn(f" {DOWN_V} --remove-orphans\n", log)

    def test_keep_fails_when_a_data_volume_has_no_label_of_the_job(self):
        done, _, _ = self.run_keep({"STUB_LABEL": "other"})
        self.assertEqual(done.returncode, 1)
        self.assertIn(f"CI: keep: the data volume {OLD_VOLUME} has no label of this job.", done.stderr)

    def test_keep_fails_when_the_project_stop_fails(self):
        done, log, _ = self.run_keep({"STUB_DOWN_STATUS": "1"})
        self.assertEqual(done.returncode, 1)
        self.assertIn("CI: keep: the project stop failed.", done.stderr)
        self.assertNotIn("volume inspect", log)

    def test_keep_fails_when_a_container_stays(self):
        done, log, _ = self.run_keep({"STUB_PS_OUTPUT": "abc123"})
        self.assertEqual(done.returncode, 1)
        self.assertIn("CI: keep: a container of the project still exists.", done.stderr)
        self.assertNotIn("volume inspect", log)

    def test_keep_needs_the_state_that_switch_back_leaves(self):
        cases = {name: {key: value for key, value in KEEP_STATE.items() if key != name} for name in KEEP_STATE}
        cases["active is the new volume"] = {**KEEP_STATE, "active.volume": BACK_NEW + "\n"}
        for name, state in cases.items():
            with self.subTest(name=name):
                done, log, _ = self.run_keep(state=state)
                self.assertEqual(done.returncode, 1)
                self.assertIn("CI: keep: run switch-back first.", done.stderr)
                self.assertEqual(log, "")


def runbook_section(title):
    """Return the text of one level-3 section of the runbook, from its heading to the next level-3 heading."""
    text = (ROOT / "docs" / "ops" / "runbook.md").read_text(encoding="utf-8")
    parts = re.split(r"(?m)^(?=### )", text)
    for part in parts:
        if part.startswith(f"### {title}\n"):
            return part
    raise KeyError(title)


def script_assignment(name):
    """Return the quoted value of one assignment of switch-steps.sh, such as SEGMENT_REGEX='...' or NAME="..."."""
    found = re.search(rf"(?m)^{name}=(?:'([^']*)'|\"([^\"]*)\")$", switch_steps_text())
    if found is None:
        raise KeyError(name)
    return found.group(1) or found.group(2)


class RunbookWalLineTests(unittest.TestCase):
    """Check that the runbook and switch-steps.sh describe the same move and the same timeline rule (ADR-0106 (k), (l))."""

    def runbook_move_command(self):
        """Return the command of sub-step 4 of step 7 of "Restore into production"."""
        section = runbook_section("Restore into production")
        step = re.search(r"(?m)^7\. Start the new WAL line.*$", section)
        self.assertIsNotNone(step, "the step 7 is missing")
        found = re.search(
            r"(?ms)^   4\. Move the segment files[^\n]*\n\n```bash\n(.*?)\n```",
            section[step.start():],
        )
        self.assertIsNotNone(found, "sub-step 4 of step 7 has no command")
        return found.group(1)

    def script_move_command(self):
        """Build the expected command from the two find calls of switch-steps.sh.

        The substitution has four parts. The prefix `sudo ` and the tail `-printf ... | sort` of the find calls go away.
        The variable $SEGMENT_REGEX and the variable $HISTORY_REGEX become their quoted values from the script.
        The two tests join with -o and the test for the file .system-identifier joins as the third. The tail is the move.
        """
        calls = {}
        for name, variable in (("archive_segments", "SEGMENT_REGEX"), ("archive_history", "HISTORY_REGEX")):
            found = re.search(
                rf'sudo (find "\$RECIPE_WALARCHIVE_DIR" .*?) -regex "\${variable}" -printf',
                shell_function(name),
            )
            self.assertIsNotNone(found, name)
            calls[variable] = found.group(1)
        self.assertEqual(calls["SEGMENT_REGEX"], calls["HISTORY_REGEX"], "the two find calls differ in their options")
        segment = script_assignment("SEGMENT_REGEX")
        history = script_assignment("HISTORY_REGEX")
        return (
            f"{calls['SEGMENT_REGEX']} \\( -regex '{segment}' -o -regex '{history}' -o -name '.system-identifier' \\)"
            ' -exec mv -t "$target" -- {} +'
        )

    def test_the_move_command_equals_the_expression_of_switch_steps(self):
        self.assertEqual(self.runbook_move_command(), self.script_move_command())

    def test_the_move_command_has_no_glob_for_the_segments(self):
        # The old expression matched a segment by the glob [0-9A-F]???... and missed every history file.
        self.assertNotIn("-name '[0-9A-F]", self.runbook_move_command())

    def test_switch_back_names_the_timeline_of_the_left_volume(self):
        section = runbook_section("Switch back")
        label = script_assignment("TIMELINE_LABEL")
        self.assertIn("`<left timeline>`", section)
        self.assertIn("`<old timeline>`", section)
        self.assertIn(label, section)

    def test_switch_back_gives_the_rule_for_an_entry_without_a_timeline(self):
        section = runbook_section("Switch back")
        step = re.search(r"(?ms)^7\. Start `walarchive` before `app`.*?(?=^8\. )", section)
        self.assertIsNotNone(step, "step 7 of Switch back is missing")
        text = step.group(0)
        self.assertIn("`<left timeline>` in place of `<old timeline>`", text)
        self.assertIn("holds no timeline", text)
        self.assertIn("`unreadable`", text)
        self.assertIn("first line only", text)
        self.assertIn("ADR-0106 (l)", text)

    def test_switch_back_compares_the_timeline_of_the_old_volume(self):
        section = runbook_section("Switch back")
        step = re.search(r"(?ms)^4\. Run the command of sub-step 2.*?(?=^5\. )", section)
        self.assertIsNotNone(step, "step 4 of Switch back is missing")
        self.assertIn("The timeline must equal `<old timeline>`", step.group(0))
        self.assertIn("holds no timeline", step.group(0))

    def test_switch_back_writes_the_timeline_of_the_left_volume_into_the_change_log(self):
        section = runbook_section("Switch back")
        step = re.search(r"(?ms)^2\. Read the identifier.*?(?=^3\. )", section)
        self.assertIsNotNone(step, "step 2 of Switch back is missing")
        self.assertIn("Latest checkpoint's TimeLineID", step.group(0))
        self.assertIn("`<left timeline>`", step.group(0))


class ExportProbeTests(unittest.TestCase):
    def setUp(self):
        spec = importlib.util.spec_from_file_location("http_probe", CI / "http_probe.py")
        self.probe = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(self.probe)
        self.temp = tempfile.TemporaryDirectory()
        self.client = Mock()
        self.client.state = pathlib.Path(self.temp.name)

    def tearDown(self):
        self.temp.cleanup()

    def document(self, **changes):
        document = {"content_sha256": FIXTURE_HASH, "loadable": True}
        document.update(changes)
        return json.dumps(document)

    def test_export_writes_the_hash(self):
        self.client.request.return_value = (200, {}, self.document())
        self.probe.export(self.client, "export.sha256")
        self.client.request.assert_called_once_with("/export/v1/recipes/CI-UPGRADE/versions/1")
        self.assertEqual((self.client.state / "export.sha256").read_text(encoding="ascii"), FIXTURE_HASH + "\n")

    def test_export_rejects_each_bad_answer(self):
        cases = [
            (404, self.document()),
            (200, self.document(loadable=False)),
            (200, self.document(content_sha256="XYZ")),
            (200, "{}"),
        ]
        for status, body in cases:
            with self.subTest(status=status, body=body):
                self.client.request.return_value = (status, {}, body)
                with self.assertRaises(ValueError):
                    self.probe.export(self.client, "export.sha256")
                self.assertFalse((self.client.state / "export.sha256").exists())

    def test_export_rejects_a_file_name_with_a_path(self):
        self.client.request.return_value = (200, {}, self.document())
        for name in ("../x", "a/b", "", "x" * 65):
            with self.subTest(name=name):
                with self.assertRaises(ValueError):
                    self.probe.export(self.client, name)


if __name__ == "__main__":
    unittest.main()
