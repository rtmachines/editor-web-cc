"""Check upgrade gates without Docker."""

import importlib.util
import copy
import os
import pathlib
import re
import shutil
import struct
import subprocess
import sys
import tempfile
import unittest


def wal_header(system_id, info=0x0002):
    """Return the first 32 bytes of a WAL segment: standard header, then xlp_sysid at offset 24."""
    standard = struct.pack("<HHIQI4x", 0xD113, info, 1, 0, 0)
    return standard + struct.pack("<Q", system_id)


def wal_identity_step():
    """Return the script that upgrade.sh runs in a new walarchive container."""
    source = pathlib.Path(__file__).with_name("upgrade.sh").read_text(encoding="utf-8")
    return source.split("WAL_IDENTITY_STEP=$(cat <<'STEP'\n", 1)[1].split("\nSTEP\n)", 1)[0]


class WalIdentityCase:
    """Run the real step against a temporary WAL directory with one complete segment."""

    def __init__(self, test, segment_sysid, info=0x0002, with_segment=True):
        self.test = test
        self.segment_sysid = segment_sysid
        self.info = info
        self.with_segment = with_segment

    def __enter__(self):
        bash = shutil.which("bash")
        if bash is None:
            self.test.skipTest("bash is not available")
        self.bash = bash
        self.temp = tempfile.TemporaryDirectory()
        self.directory = pathlib.Path(self.temp.name)
        self.file = self.directory / ".system-identifier"
        if self.with_segment:
            (self.directory / "000000010000000000000001").write_bytes(wal_header(self.segment_sysid, self.info))
        return self

    def __exit__(self, *exc):
        self.temp.cleanup()

    def run(self, identifier, prelude=""):
        env = {**os.environ, "WAL_DIR": str(self.directory)}
        # Bytes keep the stdin of the step free of CR on every host.
        # A function named sleep makes the wait for a segment end at once in the test.
        # On Windows, fsync of a file fails. A default sync stub keeps the test local. The prelude can replace it.
        sync_stub = "sync() { :; }\n" if os.name == "nt" else ""
        script = "sleep() { :; }\n" + sync_stub + prelude + wal_identity_step()
        done = subprocess.run([self.bash, "-c", script], input=(identifier + "\n").encode(),
                              capture_output=True, check=False, env=env)
        return subprocess.CompletedProcess(done.args, done.returncode, done.stdout.decode(), done.stderr.decode())


def shell_function(name):
    """Return the text of one function of upgrade.sh."""
    source = pathlib.Path(__file__).with_name("upgrade.sh").read_text(encoding="utf-8")
    return f"{name}() {{" + source.split(f"\n{name}() {{", 1)[1].split("\n}\n", 1)[0] + "\n}\n"


def runbook_identity_step():
    """Return the script block of step 1 of the segment check in the runbook section Upgrade."""
    runbook = pathlib.Path(__file__).resolve().parents[2] / "docs" / "ops" / "runbook.md"
    text = runbook.read_text(encoding="utf-8").replace("\r\n", "\n")
    section = text.split("\n## Upgrade", 1)[1].split("\n## ", 1)[0]
    blocks = re.findall(r"```bash\n(step=\$\(cat <<'STEP'\n.*?\nSTEP\n\))\n```", section, re.S)
    if len(blocks) != 1:
        raise AssertionError(f"the section Upgrade must hold exactly one step block, found {len(blocks)}")
    return blocks[0].split("\n", 1)[1].rsplit("\nSTEP\n)", 1)[0]


NAME_MESSAGE = ("CI: the pgdata volume of the new stack differs from the data volume of the running db."
                " Stop and report.")
EXTERNAL_MESSAGE = "CI: the pgdata volume of the new stack is not external. Stop and report."
VARIABLE_MESSAGE = ("CI: the pgdata volume name of the new stack differs from RECIPE_PGDATA_VOLUME."
                    " Stop and report.")
# The section volumes of a compose.yaml. The stub of compose prints the file and fills in the variable.
GOOD_COMPOSE = "services: {}\nvolumes:\n  pgdata:\n    external: true\n    name: ${RECIPE_PGDATA_VOLUME:?}\n  caddy: {}\n"
NAME_FIRST_COMPOSE = "volumes:\n  pgdata:\n    name: ${RECIPE_PGDATA_VOLUME:?}\n    external: true\n  caddy: {}\n"
OLD_COMPOSE = "services: {}\nvolumes:\n  pgdata: {}\n  caddy: {}\n"
NOT_EXTERNAL_COMPOSE = GOOD_COMPOSE.replace("external: true", "external: false")
OTHER_NAME_COMPOSE = GOOD_COMPOSE.replace("${RECIPE_PGDATA_VOLUME:?}", "other_pgdata")
INLINE_COMPOSE = "volumes:\n  pgdata: {external: true, name: ${RECIPE_PGDATA_VOLUME:?}}\n"


class NameCheckCase:
    """Run the real name check functions of upgrade.sh with stubs for Docker and Compose."""

    PRELUDE = r'''
set -euo pipefail
export RECIPE_PGDATA_VOLUME=project_pgdata
COMPOSE_PROJECT_NAME=project
compose() {
  printf '%s %s\n' "$UPGRADE_COMPOSE_DIR" "$*" >>"$CI_STATE/compose.log"
  if [[ ${REPLACE_DB:-0} == 1 ]]; then printf 'bbbb\n' >"$CI_STATE/db.id"; fi
  [[ ${1:-} == config ]] || return 1
  printf 'name: project\n'
  if [[ ${STUB_FIXED:-0} == 1 ]]; then printf 'volumes:\n  pgdata:\n    external: true\n    name: %s\n' "$RECIPE_PGDATA_VOLUME"; return 0; fi
  sed 's|\${RECIPE_PGDATA_VOLUME:?}|'"$RECIPE_PGDATA_VOLUME"'|' "$UPGRADE_COMPOSE_DIR/deploy/compose.yaml"
}
db_volume() { printf '%s\n' "$RUNNING"; }
container() { cat -- "$CI_STATE/db.id"; }
docker() {
  printf '%s\n' "$*" >>"$CI_STATE/docker.log"
  if [[ ${REPLACE_DB:-0} == 1 ]]; then printf 'bbbb\n' >"$CI_STATE/db.id"; fi
}
fail() { printf 'CI: %s\n' "$1" >&2; exit 1; }
upgrade_check() { "$PYTHON" -I -B "$CHECKS" "$@"; }
'''

    def __init__(self, test):
        self.test = test

    def run(self, call, running="project_pgdata", replace_db=False, compose_text=GOOD_COMPOSE, previous_text=GOOD_COMPOSE,
            fixed_config=False):
        bash = shutil.which("bash")
        if bash is None:
            self.test.skipTest("bash is not available")
        with tempfile.TemporaryDirectory() as temp:
            state = pathlib.Path(temp) / "state"
            state.mkdir()
            for name, text in (("new", compose_text), ("previous", previous_text)):
                directory = pathlib.Path(temp) / name / "deploy"
                directory.mkdir(parents=True)
                (directory / "compose.yaml").write_text(text, encoding="utf-8", newline="\n")
            (state / "db.id").write_text("aaaa\n")
            (state / "db.volume").write_text("project_pgdata\n")
            source = pathlib.Path(__file__).with_name("upgrade.sh").read_text(encoding="utf-8")
            messages = [line for line in source.splitlines() if line.startswith("VOLUME_") and "_MESSAGE=" in line]
            self.test.assertEqual(len(messages), 3)
            script = "\n".join([self.PRELUDE, *messages, shell_function("check_volume_name"),
                                shell_function("check_volume_name_mutation"),
                                shell_function("check_volume_external_mutation"), call])
            env = {**os.environ, "CI_STATE": state.as_posix(), "RUNNING": running, "PYTHON": sys.executable,
                   "UPGRADE_COMPOSE_DIR": (pathlib.Path(temp) / "new").as_posix(),
                   "UPGRADE_PREVIOUS_DIR": (pathlib.Path(temp) / "previous").as_posix(),
                   "STUB_FIXED": "1" if fixed_config else "0",
                   "CHECKS": pathlib.Path(__file__).with_name("upgrade_checks.py").as_posix(),
                   "REPLACE_DB": "1" if replace_db else "0"}
            done = subprocess.run([bash, "-c", script], input=b"", capture_output=True, check=False, env=env)
            docker_log = state / "docker.log"
            log = docker_log.read_text() if docker_log.exists() else ""
            compose_log = state / "compose.log"
            text = compose_log.read_text() if compose_log.exists() else ""
            self.compose_log = text.replace(state.as_posix(), "$CI_STATE").replace(pathlib.Path(temp).as_posix(), "$TEMP")
            return (done.returncode, done.stdout.decode().replace("\r", ""),
                    done.stderr.decode().replace("\r", ""), log)


class UpgradeChecksTests(unittest.TestCase):
    def setUp(self):
        path = pathlib.Path(__file__).with_name("upgrade_checks.py")
        spec = importlib.util.spec_from_file_location("upgrade_checks", path)
        self.checks = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(self.checks)

    def test_push_selects_the_before_commit(self):
        before = "a" * 40
        self.assertEqual(self.checks.select_previous_ref("", "push", before), before)

    def test_push_rejects_the_zero_sha(self):
        with self.assertRaisesRegex(ValueError, "zero SHA"):
            self.checks.select_previous_ref("", "push", "0" * 40)

    def test_push_rejects_a_missing_or_invalid_before_sha(self):
        for before in ("", "main", "a" * 39, "g" * 40):
            with self.subTest(before=before), self.assertRaises(ValueError):
                self.checks.select_previous_ref("", "push", before)

    def test_explicit_ref_overrides_the_event(self):
        for event, before in (("push", "a" * 40), ("push", "0" * 40), ("workflow_dispatch", "")):
            with self.subTest(event=event, before=before):
                self.assertEqual(self.checks.select_previous_ref("v1.2.3", event, before), "v1.2.3")

    def test_no_ref_requests_the_merge_base(self):
        for event in ("", "workflow_dispatch"):
            with self.subTest(event=event):
                self.assertIsNone(self.checks.select_previous_ref("", event, "a" * 40))

    def test_previous_commit_must_differ_from_head(self):
        self.checks.check_previous_commit("a" * 40, "b" * 40)
        with self.assertRaisesRegex(ValueError, "previous commit equals HEAD"):
            self.checks.check_previous_commit("a" * 40, "a" * 40)

    def test_equal_commits_exit_with_one_clear_message(self):
        result = subprocess.run(
            [sys.executable, "-I", "-B", str(pathlib.Path(__file__).with_name("upgrade_checks.py")),
             "previous-commit", "a" * 40, "a" * 40],
            capture_output=True, text=True, check=False,
        )
        self.assertEqual(result.returncode, 1)
        self.assertEqual(result.stdout, "")
        self.assertEqual(result.stderr, "CI: the previous commit equals HEAD. Stop and report.\n")

    def test_extracts_only_the_guard_statement(self):
        statement = (
            "CREATE FUNCTION audit_guard.audit_objects_dropped() RETURNS event_trigger\n"
            "LANGUAGE plpgsql AS $$\nBEGIN\n    RETURN;\nEND;\n$$;"
        )
        source = "CREATE SCHEMA audit_guard;\n" + statement + "\nREVOKE ALL ON SCHEMA audit_guard FROM PUBLIC;"
        self.assertEqual(self.checks.guard_statement(source), statement.replace("CREATE FUNCTION", "CREATE OR REPLACE FUNCTION", 1))

    def test_missing_duplicate_and_unclosed_guards_fail(self):
        statement = "CREATE FUNCTION audit_guard.audit_objects_dropped() RETURNS event_trigger\n$$;"
        for source in ("", statement + "\n" + statement, statement.removesuffix("$$;")):
            with self.subTest(source=source), self.assertRaises(ValueError):
                self.checks.guard_statement(source)

    def test_only_one_exact_restore_success_passes(self):
        expected = "recipe_20261026_1201_now"
        passed = '{"message":"restore test passed","set":"' + expected + '"}\n'
        self.checks.check_restore(passed, expected)
        for output in ("", passed.replace("passed", "started"), passed.replace("_now", "_premigrate"), passed + passed):
            with self.subTest(output=output), self.assertRaises(ValueError):
                self.checks.check_restore(output, expected)

    def test_backup_result_rejects_the_wrong_suffix(self):
        passed = '{"message":"backup completed","set":"recipe_20261026_1201_premigrate"}\n'
        self.assertEqual(self.checks.backup_set(passed, "_premigrate"), "recipe_20261026_1201_premigrate")
        for output in (passed.replace("completed", "started"), passed.replace("_premigrate", "_now"), passed + passed):
            with self.subTest(output=output), self.assertRaises(ValueError):
                self.checks.backup_set(output, "_premigrate")

    def test_config_rejects_one_stale_image(self):
        services = {name: {"image": f"recipe-{image}:new"} for name, image in self.checks.IMAGES.items()}
        self.checks.check_images({"services": services}, "new")
        services["walarchive"]["image"] = "recipe-pgtools:old"
        with self.assertRaises(ValueError):
            self.checks.check_images({"services": services}, "new")

    def test_pgdata_volume_name_comes_from_the_config(self):
        config = {"volumes": {"pgdata": {"name": "recipe-ci-1-1_pgdata", "external": True}}}
        self.assertEqual(self.checks.pgdata_volume_name(config), "recipe-ci-1-1_pgdata")
        for bad in ({}, {"volumes": {}}, {"volumes": {"pgdata": {}}}, {"volumes": {"pgdata": {"name": ""}}},
                    {"volumes": {"pgdata": {"name": 7}}}):
            with self.subTest(config=bad), self.assertRaises((ValueError, KeyError)):
                self.checks.pgdata_volume_name(bad)

    def test_segment_check_log_must_match_the_state_of_the_identifier_file(self):
        passed = ("The segment check passed. The segment 000000010000000000000003"
                  " holds the system identifier of db.\n")
        skipped = "The segment check does not run. The file .system-identifier exists.\n"
        self.checks.check_wal_identity_log(passed, "ran")
        self.checks.check_wal_identity_log(passed.replace("0003 ", "0003.partial "), "ran")
        self.checks.check_wal_identity_log(skipped, "skipped")
        wrong_name = passed.replace("000000010000000000000003", "0000000100000000000003")
        for output, mode in ((skipped, "ran"), (passed, "skipped"), ("", "ran"), ("", "skipped"),
                             (passed + passed, "ran"), (skipped + passed, "skipped"), (wrong_name, "ran"),
                             (passed, "other")):
            with self.subTest(output=output, mode=mode), self.assertRaises(ValueError):
                self.checks.check_wal_identity_log(output, mode)

    def test_volume_name_and_log_actions_run_from_the_command_line(self):
        script = str(pathlib.Path(__file__).with_name("upgrade_checks.py"))
        with tempfile.TemporaryDirectory() as temp:
            config = pathlib.Path(temp) / "config.json"
            config.write_text('{"volumes": {"pgdata": {"name": "abc_pgdata"}}}', encoding="utf-8")
            log = pathlib.Path(temp) / "wal.log"
            log.write_text("The segment check does not run. The file .system-identifier exists.\n", encoding="utf-8")
            done = subprocess.run([sys.executable, "-I", "-B", script, "volume-name", str(config)],
                                  capture_output=True, text=True, check=False)
            self.assertEqual((done.returncode, done.stdout), (0, "abc_pgdata\n"))
            done = subprocess.run([sys.executable, "-I", "-B", script, "wal-identity", str(log), "skipped"],
                                  capture_output=True, text=True, check=False)
            self.assertEqual(done.returncode, 0, done.stderr)
            done = subprocess.run([sys.executable, "-I", "-B", script, "wal-identity", str(log), "ran"],
                                  capture_output=True, text=True, check=False)
            self.assertEqual(done.returncode, 1)
            self.assertIn("CI:", done.stderr)


class UpgradeSmokeTests(unittest.TestCase):
    def setUp(self):
        path = pathlib.Path(__file__).with_name("upgrade_smoke.py")
        spec = importlib.util.spec_from_file_location("upgrade_smoke", path)
        self.smoke = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(self.smoke)

    def test_export_checks_data_hash_and_loadable_state(self):
        document = self.smoke.fixture_body()
        document.update(content_sha256=self.smoke.body_hash(document), loadable=True)
        self.smoke.check_export(document)
        for field, value in (("content_sha256", "0" * 64), ("loadable", False), ("parameters", [])):
            changed = copy.deepcopy(document)
            changed[field] = value
            with self.subTest(field=field), self.assertRaises(ValueError):
                self.smoke.check_export(changed)

    def test_export_rejects_a_different_recipe_even_with_its_correct_hash(self):
        document = self.smoke.fixture_body()
        document["recipe"]["reference_number"] = "WRONG-RECIPE"
        document.update(content_sha256=self.smoke.body_hash(document), loadable=True)
        with self.assertRaises(ValueError):
            self.smoke.check_export(document)

    def test_export_excludes_display_names_from_the_hash(self):
        document = self.smoke.fixture_body()
        document.update(content_sha256=self.smoke.body_hash(document), loadable=True)
        document["machine"]["name"] = "A new name"
        document["recipe"]["name"] = "Another name"
        document["parameters"][0]["name_en"] = "A display name"
        self.smoke.check_export(document)


class UpgradeScriptTests(unittest.TestCase):
    def test_build_previous_gives_the_db_build_context_to_the_pgtools_build(self):
        source = pathlib.Path(__file__).with_name("upgrade.sh").read_text(encoding="utf-8")
        body = source.split("build_previous() {", 1)[1].split("\n}\n", 1)[0].replace("\\\n", " ")
        builds = [part for part in body.split("docker buildx build") if "recipe-pgtools:ci-previous" in part]
        self.assertEqual(len(builds), 1)
        self.assertIn('--build-context db="$UPGRADE_PREVIOUS_DIR/deploy/db"', builds[0])

    def test_upgrade_records_and_compares_the_data_volume_and_the_system_identifier(self):
        source = pathlib.Path(__file__).with_name("upgrade.sh").read_text(encoding="utf-8")
        self.assertNotIn("old-db.id", source)
        self.assertNotIn("the upgrade replaced the database container", source)
        body = source.split("\nupgrade() {", 1)[1].split("\n}\n", 1)[0]
        up = body.index("compose up -d backup-premigrate")
        self.assertLess(body.index('db_volume >"$CI_STATE/db.volume"'), up)
        self.assertLess(body.index('db_system_identifier >"$CI_STATE/db.system-id"'), up)
        self.assertRegex(
            body,
            r'\[\[ \$\(db_volume\) == "\$\(cat "\$CI_STATE/db\.volume"\)" \]\]'
            r' \|\| fail "the upgrade changed the data volume of the database"',
        )
        self.assertRegex(
            body,
            r'\[\[ \$\(db_system_identifier\) == "\$\(cat "\$CI_STATE/db\.system-id"\)" \]\]'
            r' \|\| fail "the upgrade changed the system identifier of the database"',
        )
        self.assertGreater(body.index("the upgrade changed the data volume"), up)
        self.assertGreater(body.index("the upgrade changed the system identifier"), up)
    def test_the_pgdata_volume_name_comes_from_the_project_and_reaches_compose(self):
        common = pathlib.Path(__file__).with_name("common.sh").read_text(encoding="utf-8")
        self.assertEqual(common.count("RECIPE_PGDATA_VOLUME="), 1)
        self.assertIn("export RECIPE_PGDATA_VOLUME=${COMPOSE_PROJECT_NAME}_pgdata", common)
        for name in ("common.sh", "upgrade-common.sh"):
            source = pathlib.Path(__file__).with_name(name).read_text(encoding="utf-8")
            options = source.split("sudo --preserve-env=", 1)[1].split(" ", 1)[0].split(",")
            with self.subTest(script=name):
                self.assertIn("RECIPE_PGDATA_VOLUME", options)

    def test_every_ci_volume_has_the_label_of_the_project(self):
        common = pathlib.Path(__file__).with_name("common.sh").read_text(encoding="utf-8")
        create = common.split("create_pgdata_volume() {", 1)[1].split("\n}\n", 1)[0]
        self.assertIn('docker volume create --label "recipe.ci=$COMPOSE_PROJECT_NAME" "$RECIPE_PGDATA_VOLUME"', create)
        prepare = pathlib.Path(__file__).with_name("prepare.sh").read_text(encoding="utf-8")
        self.assertIn("create_pgdata_volume", prepare)

    def test_cleanup_deletes_only_labelled_volumes(self):
        common = pathlib.Path(__file__).with_name("common.sh").read_text(encoding="utf-8")
        delete = common.split("delete_ci_volumes() {", 1)[1].split("\n}\n", 1)[0]
        self.assertIn('docker volume ls -q --filter "label=recipe.ci=$COMPOSE_PROJECT_NAME"', delete)
        self.assertIn('delete_labelled_volume "$volume"', delete)
        self.assertNotIn("docker volume rm", delete)
        shared = common.split("delete_labelled_volume() {", 1)[1].split("\n}\n", 1)[0]
        self.assertEqual(common.count("docker volume rm"), 1)
        self.assertEqual(common.count('rm "$volume"'), 0)
        label = common.split("has_job_label() {", 1)[1].split("\n}\n", 1)[0]
        self.assertIn("docker volume inspect --format", label)
        self.assertIn("recipe.ci", label.split("docker volume inspect", 1)[1])
        self.assertLess(shared.index("has_job_label"), shared.index("docker volume rm"))
        self.assertNotIn("fail ", shared)
        cleanup = pathlib.Path(__file__).with_name("cleanup.sh").read_text(encoding="utf-8")
        self.assertIn("delete_ci_volumes", cleanup)
        self.assertNotIn("docker volume rm", cleanup)

    def test_start_previous_creates_the_volume_only_for_an_external_pgdata(self):
        source = pathlib.Path(__file__).with_name("upgrade.sh").read_text(encoding="utf-8")
        body = source.split("\nstart_previous() {", 1)[1].split("\n}\n", 1)[0]
        guard = body.index("previous_pgdata_is_external")
        self.assertLess(guard, body.index("create_pgdata_volume"))
        self.assertLess(body.index("create_pgdata_volume"), body.index("compose up -d --wait"))
        self.assertRegex(
            body,
            r'\[\[ \$\(db_volume\) == "\$RECIPE_PGDATA_VOLUME" \]\]'
            r' \|\| fail "the previous database does not use the volume of the upgrade"',
        )
        helper = source.split("\nprevious_pgdata_is_external() {", 1)[1].split("\n}\n", 1)[0]
        self.assertIn("external:[[:space:]]*true", helper)
        self.assertIn("$UPGRADE_PREVIOUS_DIR/deploy/compose.yaml", helper)

    def test_upgrade_cleanup_deletes_only_labelled_volumes(self):
        source = pathlib.Path(__file__).with_name("upgrade.sh").read_text(encoding="utf-8")
        body = source.split("\ncleanup() {", 1)[1].split("\n}\n", 1)[0]
        self.assertLess(body.index("down -v"), body.index("delete_ci_volumes"))
        self.assertNotIn("com.docker.compose.project", body)
        self.assertNotIn("docker volume rm", body)

    def test_wal_identity_step_runs_in_a_new_walarchive_container_before_the_db_replacement(self):
        source = pathlib.Path(__file__).with_name("upgrade.sh").read_text(encoding="utf-8")
        self.assertFalse("compose exec -T walarchive" in source, "upgrade.sh must not use compose exec into walarchive")
        step = source.split("\nwrite_wal_identity() {", 1)[1].split("\n}\n", 1)[0]
        run_form = r"RECIPE_VERSION=ci-previous\s+compose run --rm --no-deps -T --entrypoint sh walarchive"
        run = re.search(run_form, step)
        self.assertIsNotNone(run, "the step must run in a new container with the previous tag")
        self.assertLess(step.index("db_system_identifier"), step.index("pg_switch_wal"))
        wait = step[step.index("pg_switch_wal"):run.start()]
        self.assertIn("RECIPE_WALARCHIVE_DIR", wait)
        self.assertRegex(wait, r"(?m)^\s*(for|while|until)\b")
        self.assertIn("sleep", wait)
        self.assertIn("$WAL_IDENTITY_STEP", step[run.start():])
        self.assertLess(run.start(), step.index("sudo cat"))
        body = source.split("\nupgrade() {", 1)[1].split("\n}\n", 1)[0]
        self.assertEqual(body.count("write_wal_identity"), 1)
        name_check = body.index("\n  check_volume_name\n")
        replacement = body.index("compose up -d backup-premigrate")
        self.assertLess(name_check, body.index("write_wal_identity"))
        self.assertLess(body.index("write_wal_identity"), replacement)

    def test_restore_test_runs_in_a_new_previous_container_before_the_premigrate_backup(self):
        source = pathlib.Path(__file__).with_name("upgrade.sh").read_text(encoding="utf-8")
        body = source.split("\nupgrade() {", 1)[1].split("\n}\n", 1)[0]
        run_form = "RECIPE_VERSION=ci-previous compose run --rm --no-deps -T backup restore-test"
        self.assertEqual(body.count(run_form), 1)
        start = body.index(run_form)
        self.assertLess(body.index("\n  write_wal_identity\n"), start)
        self.assertLess(start, body.index("compose up -d backup-premigrate"))
        self.assertNotRegex(source, r"compose exec( -T)? backup restore-test")
        self.assertNotIn("restore-test", source.split("\nstart_previous() {", 1)[1].split("\n}\n", 1)[0])
        self.assertRegex(
            body[start:],
            r'(?s)^[^\n]*\| tee "\$CI_STATE/old-restore\.log"\n\s*upgrade_check restore "\$CI_STATE/old-restore\.log" "\$\(cat "\$CI_STATE/old-now\.set"\)"',
        )

    def test_wal_identity_step_writes_the_file_from_a_partial_file_of_the_same_system(self):
        with WalIdentityCase(self, segment_sysid=5, with_segment=False) as case:
            (case.directory / "000000010000000000000001.partial").write_bytes(wal_header(5))
            result = case.run("5")
            self.assertEqual(result.returncode, 0, result.stderr)
            lines = [line for line in result.stdout.splitlines() if line.strip()]
            self.assertEqual(len(lines), 1, result.stdout)
            self.assertIn("segment check passed", lines[0])
            self.assertEqual(case.file.read_bytes(), b"5\n")
            self.assertEqual(sorted(path.name for path in case.directory.iterdir()),
                             [".system-identifier", "000000010000000000000001.partial"])

    def test_wal_identity_step_reads_the_newest_partial_file(self):
        with WalIdentityCase(self, segment_sysid=5, with_segment=False) as case:
            (case.directory / "000000010000000000000001.partial").write_bytes(wal_header(9))
            (case.directory / "000000010000000000000002.partial").write_bytes(wal_header(5))
            result = case.run("5")
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertEqual(case.file.read_bytes(), b"5\n")

    def test_wal_identity_step_refuses_a_partial_file_of_another_system(self):
        with WalIdentityCase(self, segment_sysid=5, with_segment=False) as case:
            (case.directory / "000000010000000000000001.partial").write_bytes(wal_header(7))
            result = case.run("5")
            self.assertEqual(result.returncode, 1)
            lines = [line for line in result.stderr.splitlines() if line.strip()]
            self.assertEqual(len(lines), 1, result.stderr)
            self.assertIn("holds another system identifier", lines[0])
            self.assertEqual(result.stdout, "")
            self.assertEqual(sorted(path.name for path in case.directory.iterdir()),
                             ["000000010000000000000001.partial"])

    def test_wal_identity_step_refuses_a_partial_file_without_a_long_header(self):
        with WalIdentityCase(self, segment_sysid=5, with_segment=False) as case:
            (case.directory / "000000010000000000000001.partial").write_bytes(wal_header(5, info=0x0001))
            result = case.run("5")
            self.assertEqual(result.returncode, 1)
            lines = [line for line in result.stderr.splitlines() if line.strip()]
            self.assertEqual(len(lines), 1, result.stderr)
            self.assertIn("has no long header", lines[0])
            self.assertEqual(result.stdout, "")
            self.assertEqual(sorted(path.name for path in case.directory.iterdir()),
                             ["000000010000000000000001.partial"])

    def test_wal_identity_step_ignores_a_matching_partial_file_when_a_complete_segment_exists(self):
        with WalIdentityCase(self, segment_sysid=6) as case:
            (case.directory / "000000010000000000000002.partial").write_bytes(wal_header(5))
            result = case.run("5")
            self.assertEqual(result.returncode, 1)
            self.assertIn("holds another system identifier", result.stderr)
            self.assertFalse(case.file.exists())

    def test_wal_identity_step_runs_sync_on_the_temp_file_before_ln_and_on_the_directory_after_it(self):
        with WalIdentityCase(self, segment_sysid=5) as case:
            log = case.directory.parent / (case.directory.name + ".calls")
            self.addCleanup(lambda: log.unlink(missing_ok=True))
            prelude = (f'CALLS="{log.as_posix()}"\n'
                       'sync() { printf "sync %s\\n" "$*" >>"$CALLS"; }\n'
                       'ln() { printf "ln %s\\n" "$*" >>"$CALLS"; command ln "$@"; }\n')
            result = case.run("5", prelude)
            self.assertEqual(result.returncode, 0, result.stderr)
            calls = [line.rstrip("\r").split(" ", 1) for line in log.read_text().splitlines()] if log.exists() else []
            # Options such as -- or -d do not change which path the call names.
            kinds = [(name, [part for part in rest.split() if not part.startswith("-")]) for name, rest in calls]
            self.assertEqual([name for name, _ in kinds], ["sync", "ln", "sync"], calls)
            directory = str(case.directory)
            temp = kinds[0][1]
            self.assertEqual(len(temp), 1, calls)
            self.assertTrue(temp[0].startswith(directory + "/.system-identifier.tmp."), calls)
            self.assertIn(temp[0], kinds[1][1])
            self.assertEqual(kinds[2][1], [directory], calls)

    def test_wal_identity_step_names_the_published_file_when_only_the_directory_sync_fails(self):
        with WalIdentityCase(self, segment_sysid=5) as case:
            prelude = 'sync() { local last; for last; do :; done; [ ! -d "$last" ]; }\n'
            result = case.run("5", prelude)
            self.assertEqual(result.returncode, 1)
            self.assertEqual(result.stderr.strip().splitlines(),
                             ["ERROR: the file .system-identifier exists, but the sync of the directory failed."])
            self.assertEqual(case.file.read_text().strip(), "5")
            self.assertEqual([p.name for p in case.directory.iterdir() if ".tmp." in p.name], [])

    def test_runbook_step_equals_wal_identity_step_of_upgrade_sh(self):
        runbook = runbook_identity_step().replace("\r\n", "\n").rstrip("\n")
        script = wal_identity_step().replace("\r\n", "\n").rstrip("\n")
        self.assertEqual(runbook, script)

    def test_wal_identity_step_skips_when_the_file_exists(self):
        with WalIdentityCase(self, segment_sysid=5) as case:
            case.file.write_text("99\n")
            result = case.run("5")
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertIn("The file .system-identifier exists.", result.stdout)
            self.assertNotIn("segment check passed", result.stdout)
            self.assertEqual(case.file.read_text(), "99\n")

    def test_wal_identity_step_refuses_a_segment_with_another_system_identifier(self):
        with WalIdentityCase(self, segment_sysid=5) as case:
            result = case.run("6")
            self.assertEqual(result.returncode, 1)
            self.assertIn("holds another system identifier", result.stderr)
            self.assertFalse(case.file.exists())
            self.assertEqual(list(case.directory.glob(".system-identifier*")), [])

    def test_wal_identity_step_refuses_a_page_without_a_long_header(self):
        with WalIdentityCase(self, segment_sysid=5, info=0x0001) as case:
            result = case.run("5")
            self.assertEqual(result.returncode, 1)
            self.assertIn("has no long header", result.stderr)
            self.assertFalse(case.file.exists())

    def test_wal_identity_step_writes_one_line_with_mode_600_for_the_same_system(self):
        with WalIdentityCase(self, segment_sysid=1234567890123456789) as case:
            (case.directory / "000000010000000000000002.partial").write_bytes(wal_header(7))
            result = case.run("1234567890123456789")
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertIn("segment 000000010000000000000001 holds", result.stdout)
            self.assertEqual(case.file.read_bytes(), b"1234567890123456789\n")
            if os.name == "posix":
                self.assertEqual(case.file.stat().st_mode & 0o777, 0o600)
            self.assertEqual(sorted(path.name for path in case.directory.iterdir()),
                             [".system-identifier", "000000010000000000000001", "000000010000000000000002.partial"])

    def test_name_check_passes_for_the_same_volume_and_stops_for_another(self):
        case = NameCheckCase(self)
        code, _, stderr, _ = case.run("check_volume_name")
        self.assertEqual((code, stderr), (0, ""))
        code, _, stderr, _ = case.run("check_volume_name", running="other_pgdata")
        self.assertEqual(code, 1)
        self.assertEqual(stderr, NAME_MESSAGE + "\n")

    def test_name_check_refuses_a_volume_that_is_not_external(self):
        for text in (OLD_COMPOSE, NOT_EXTERNAL_COMPOSE, INLINE_COMPOSE.replace("external: true", "external: false")):
            with self.subTest(text=text):
                code, _, stderr, _ = NameCheckCase(self).run("check_volume_name", compose_text=text)
                self.assertEqual((code, stderr), (1, EXTERNAL_MESSAGE + "\n"))

    def test_name_check_refuses_a_name_other_than_the_variable(self):
        code, _, stderr, _ = NameCheckCase(self).run("check_volume_name", compose_text=OTHER_NAME_COMPOSE,
                                                      running="other_pgdata")
        self.assertEqual((code, stderr), (1, VARIABLE_MESSAGE + "\n"))

    def test_name_check_reads_name_and_external_in_any_order(self):
        code, _, stderr, _ = NameCheckCase(self).run("check_volume_name", compose_text=NAME_FIRST_COMPOSE)
        self.assertEqual((code, stderr), (0, ""))

    def test_name_check_runs_one_config_command_and_nothing_else(self):
        case = NameCheckCase(self)
        for text in (OLD_COMPOSE, OTHER_NAME_COMPOSE, GOOD_COMPOSE):
            with self.subTest(text=text):
                case.run("check_volume_name", compose_text=text, running="other_pgdata")
                self.assertEqual(case.compose_log, "$TEMP/new config\n")

    def test_external_mutation_expects_case_1_on_a_copy_and_keeps_db(self):
        for previous in (GOOD_COMPOSE, NAME_FIRST_COMPOSE, OLD_COMPOSE):
            with self.subTest(previous=previous):
                case = NameCheckCase(self)
                code, stdout, stderr, log = case.run("check_volume_external_mutation", previous_text=previous)
                self.assertEqual(code, 0, stderr)
                self.assertIn("the name check stopped the upgrade for a pgdata volume that is not external", stdout)
                self.assertEqual(case.compose_log, "$CI_STATE/previous-no-external config\n")
                self.assertEqual(log, "")

    def test_external_mutation_fails_when_the_check_does_not_stop(self):
        code, _, stderr, _ = NameCheckCase(self).run("check_volume_external_mutation", fixed_config=True)
        self.assertEqual(code, 1)
        self.assertIn("the name check did not stop the upgrade for a pgdata volume that is not external", stderr)

    def test_external_mutation_fails_when_db_changes(self):
        code, _, stderr, _ = NameCheckCase(self).run("check_volume_external_mutation", replace_db=True)
        self.assertEqual(code, 1)
        self.assertIn("replaced the database container", stderr)
        code, _, stderr, _ = NameCheckCase(self).run("check_volume_external_mutation", running="other_pgdata")
        self.assertEqual(code, 1)
        self.assertIn("changed the data volume of the database", stderr)

    def test_name_check_mutation_expects_the_stop_and_keeps_db(self):
        code, stdout, stderr, log = NameCheckCase(self).run("check_volume_name_mutation")
        self.assertEqual(code, 0, stderr)
        self.assertIn("the name check stopped the upgrade", stdout)
        self.assertIn("volume create --label recipe.ci=project project_pgdata_wrong", log)

    def test_name_check_mutation_fails_when_the_check_does_not_stop(self):
        code, _, stderr, _ = NameCheckCase(self).run("check_volume_name_mutation", running="project_pgdata_wrong")
        self.assertEqual(code, 1)
        self.assertIn("the name check did not stop the upgrade", stderr)

    def test_name_check_mutation_fails_when_db_is_replaced(self):
        code, _, stderr, _ = NameCheckCase(self).run("check_volume_name_mutation", replace_db=True)
        self.assertEqual(code, 1)
        self.assertIn("replaced the database container", stderr)

    def test_upgrade_checks_the_volume_name_before_any_compose_start(self):
        source = pathlib.Path(__file__).with_name("upgrade.sh").read_text(encoding="utf-8")
        body = source.split("\nupgrade() {", 1)[1].split("\n}\n", 1)[0]
        record = body.index('db_volume >"$CI_STATE/db.volume"')
        mutation = body.index("\n  check_volume_name_mutation\n")
        external = body.index("\n  check_volume_external_mutation\n")
        gate = body.index("\n  check_volume_name\n")
        self.assertLess(record, mutation)
        self.assertLess(mutation, external)
        self.assertLess(external, gate)
        self.assertLess(external, body.index("write_wal_identity"))
        self.assertEqual(body.count("\n  check_volume_external_mutation\n"), 1)
        for command in ("compose up", "wait_job", "replace_guard"):
            with self.subTest(command=command):
                self.assertLess(gate, body.index(command))
        self.assertEqual(body.count("\n  check_volume_name\n"), 1)

    def test_name_check_compares_compose_config_with_the_running_db(self):
        check = shell_function("check_volume_name")
        runbook = pathlib.Path(__file__).resolve().parents[2] / "docs" / "ops" / "runbook.md"
        text = runbook.read_text(encoding="utf-8").replace("\r\n", "\n")
        section = text.split("\n## Upgrade", 1)[1].split("\n## ", 1)[0]
        commands = re.findall(r"^docker compose config \| (awk '.*')$", section, re.M)
        self.assertEqual(len(commands), 1)
        self.assertIn(commands[0], check)
        self.assertIn("compose config", check)
        self.assertIn("db_volume", check)
        self.assertNotIn("compose up", check)

    def test_every_ci_volume_has_the_label_and_only_common_deletes_volumes(self):
        for script in sorted(pathlib.Path(__file__).parent.glob("*.sh")):
            text = script.read_text(encoding="utf-8")
            for line in text.splitlines():
                code = line.split("#", 1)[0]
                if "docker volume create" in code:
                    with self.subTest(script=script.name, line=line):
                        self.assertIn('--label "recipe.ci=$COMPOSE_PROJECT_NAME"', code)
            if script.name != "common.sh":
                with self.subTest(script=script.name):
                    self.assertNotIn("docker volume rm", text)
                    self.assertNotIn("docker volume prune", text)

    def test_segment_check_log_is_captured_and_checked_against_the_file_state(self):
        source = pathlib.Path(__file__).with_name("upgrade.sh").read_text(encoding="utf-8")
        step = source.split("\nwrite_wal_identity() {", 1)[1].split("\n}\n", 1)[0]
        state = step.index("sudo test -e")
        run = step.index('"$WAL_IDENTITY_STEP"')
        self.assertLess(state, run)
        self.assertRegex(step[run:], r'\| tee "\$CI_STATE/wal-identity\.log"')
        check = step.index('upgrade_check wal-identity "$CI_STATE/wal-identity.log"')
        self.assertLess(run, check)
        self.assertLess(check, step.index("sudo cat"))

if __name__ == "__main__":
    unittest.main()
