"""Check upgrade manifest gates without Docker."""

import os
import pathlib
import shutil
import subprocess
import sys
import tempfile
import unittest


def shell_function(name, filename="upgrade.sh"):
    """Read one real function without the CI-only source initialization."""
    source = pathlib.Path(__file__).with_name(filename).read_text(encoding="utf-8")
    if f"\n{name}() {{" not in source:
        raise AssertionError(f"{filename} has no function {name}")
    return f"{name}() {{" + source.split(f"\n{name}() {{", 1)[1].split("\n}\n", 1)[0] + "\n}\n"


class ManifestUpgradeTests(unittest.TestCase):
    """Run the previous-image steps with local Compose and Docker stubs."""

    def run_step(self, call, probe="no-check", fault="", key=None):
        bash = shutil.which("bash")
        if bash is None:
            self.skipTest("bash is not available")
        root = pathlib.Path(__file__).resolve().parents[2]
        names = ("probe_previous_manifest", "previous_compose", "check_previous_manifest_key",
                 "check_rollback_manifest", "restore_previous")
        if "check_changed_rollback_set" in call:
            names += ("delete_rollback_set_copy", "check_changed_rollback_set")
        script = "set -euo pipefail\numask 077\n" + self.STUBS
        script += "\n".join(shell_function(name) for name in names) + "\n" + call
        with tempfile.TemporaryDirectory() as temp:
            directory = pathlib.Path(temp)
            backup = directory / "backups"
            backup.mkdir()
            stamp = "20261116_1200_premigrate"
            backup_set = "recipe_" + stamp
            (directory / "rollback.set").write_text(backup_set + "\n")
            for name in (backup_set + ".dump", backup_set + ".counts", "globals_" + stamp + ".sql",
                         backup_set + ".dump.sha256", backup_set + ".dump.sha256.hmac"):
                (backup / name).write_text("PGDMP fixture\n")
            env = {**os.environ, "CI_STATE": directory.as_posix(), "GITHUB_WORKSPACE": root.as_posix(),
                   "UPGRADE_COMPOSE_DIR": root.as_posix(), "RECIPE_BACKUP_DIR": backup.as_posix(),
                   "PROBE": probe, "FAULT": fault, "KEY_STATE": key or ("key" if probe == "check" else "no-key"),
                   "PYTHON": sys.executable, "RECIPE_VERSION": "ci-upgrade"}
            done = subprocess.run([bash, "-c", script], capture_output=True, text=True, check=False, env=env)
            self.calls = (directory / "calls").read_text() if (directory / "calls").exists() else ""
            self.copied = sorted(p.name for p in (backup / "upgrade-rollback").glob("*"))
            self.rollback_exists = (backup / "upgrade-rollback").exists()
            self.remaining = sorted(p.name for p in backup.iterdir())
            self.overlay = (directory / "rollback-key.compose.yaml").read_text() \
                if (directory / "rollback-key.compose.yaml").exists() else ""
            return done.returncode, done.stdout, done.stderr

    STUBS = r'''
fail() { printf 'CI: %s\n' "$1" >&2; exit 1; }
upgrade_check() {
  if [[ $FAULT == "$1" ]]; then fail "stub $1 failed"; fi
  "$PYTHON" -I -B "$GITHUB_WORKSPACE/deploy/ci/upgrade_checks.py" "$@"
}
docker() {
  printf 'docker %s\n' "$*" >>"$CI_STATE/calls"
  if [[ $FAULT == probe ]]; then fail 'stub probe failed'; fi
  printf '%s\n' "$PROBE"
}
compose() {
  printf 'compose %s %s %s\n' "$RECIPE_VERSION" "$UPGRADE_COMPOSE_DIR/deploy/compose.yaml" "$*" >>"$CI_STATE/calls"
  if [[ $FAULT == compose ]]; then fail 'stub compose failed'; fi
  if [[ " $* " == *' config '* ]]; then
    "$PYTHON" -I -c 'import json,sys; print(json.dumps({"services": {n: {"secrets":
      [{"source": "backup_manifest_key"}] if sys.argv[1] == "key" else []}
      for n in ("backup", "backup-premigrate", "restore-volume")}}))' "$KEY_STATE"
  elif [[ " $* " == *' --entrypoint sh '* ]]; then
    if [[ $FAULT == container ]]; then fail 'stub container failed'; fi
    printf '%s\n' "${CONTAINER_KEY_STATE:-$KEY_STATE}"
  elif [[ " $* " == *' restore-volume --check-only '* ]]; then
    local stamp=${!#} set=recipe_${!#}
    if [[ $stamp == 20261116_1201_premigrate ]]; then
      command cmp "$RECIPE_BACKUP_DIR/$set.dump.sha256.hmac" \
        "$RECIPE_BACKUP_DIR/recipe_20261116_1200_premigrate.dump.sha256.hmac" || fail 'stub tag changed'
      "$PYTHON" -I -c 'import pathlib,sys
new,old=(pathlib.Path(p).read_bytes() for p in sys.argv[1:])
assert new == b"\0" + old[1:]
' "$RECIPE_BACKUP_DIR/$set.dump" "$RECIPE_BACKUP_DIR/recipe_20261116_1200_premigrate.dump" \
        || fail 'stub dump byte did not change exactly once'
      command cmp "$RECIPE_BACKUP_DIR/$set.counts" "$RECIPE_BACKUP_DIR/recipe_20261116_1200_premigrate.counts" \
        || fail 'stub counts changed'
      command cmp "$RECIPE_BACKUP_DIR/globals_$stamp.sql" "$RECIPE_BACKUP_DIR/globals_20261116_1200_premigrate.sql" \
        || fail 'stub globals changed'
      (cd "$RECIPE_BACKUP_DIR"; sha256sum --check "$set.dump.sha256" >/dev/null) || fail 'stub manifest invalid'
      if [[ $FAULT == bad-accepted ]]; then return 0; fi
      if [[ $FAULT == bad-directory ]]; then mkdir "$RECIPE_BACKUP_DIR/upgrade-rollback"; fi
      if [[ $FAULT != bad-log ]]; then
        printf '{"level":"ERROR","job":"restore-volume","message":"the tag of the set does not match the manifest","set":"%s"}\n' "$set"
      fi
      if [[ $FAULT == bad-zero-error ]]; then return 0; fi
      return 23
    fi
    if [[ $RECIPE_VERSION == ci-upgrade && $FAULT == new-check ]]; then
      printf 'stub new-image check failed\n' >&2
      return 23
    fi
    if [[ $FAULT == tag ]]; then fail 'the tag of the set does not match the manifest'; fi
    if [[ $FAULT != tag-log ]]; then
      printf '%s\n' '{"level":"INFO","job":"restore-volume","message":"the set passed the check","set":"recipe_20261116_1200_premigrate"}'
    fi
  else
    printf '%s\n' '{"message":"restore test passed","set":"recipe_20261116_1200_premigrate"}'
  fi
}
sudo() {
  printf 'sudo %s\n' "$*" >>"$CI_STATE/calls"
  case $FAULT in
    bad-copy) if [[ $1 == install && $* == *1201*.dump ]]; then fail 'stub bad copy failed'; fi ;;
    bad-globals) if [[ $1 == install && $* == *globals_20261116_1201* ]]; then fail 'stub bad globals failed'; fi ;;
    bad-dd) if [[ $1 == dd ]]; then fail 'stub dd failed'; fi ;;
    bad-manifest) if [[ $1 == bash ]]; then fail 'stub manifest failed'; fi ;;
    bad-cleanup) if [[ $1 == rm ]]; then fail 'stub cleanup failed'; fi ;;
  esac
  if [[ $FAULT == copy && $* == *dump.sha256.hmac* ]]; then fail 'stub tag copy failed'; fi
  if [[ $FAULT == cmp && $1 == cmp && $* == *"$CMP_FILE"* ]]; then
    printf 'stub cmp failed\n' >&2
    return 1
  fi
  local arg
  local -a args=()
  while (( $# )); do
    arg=$1; shift
    case $arg in -o|-g) shift ;; *) args+=("$arg") ;; esac
  done
  command "${args[@]}"
}
'''

    def test_probe_and_plan_select_both_compose_paths(self):
        for probe, state in (("check", "key"), ("no-check", "no-key")):
            with self.subTest(probe=probe):
                code, output, error = self.run_step("probe_previous_manifest; check_previous_manifest_key", probe)
                self.assertEqual((code, error), (0, ""))
                self.assertIn("CI: previous manifest probe: " + probe + ".\n", output)
                self.assertIn("CI: previous manifest key state: " + state + ".\n", output)
                self.assertIn("--entrypoint sh recipe-pgtools:ci-previous", self.calls)
                self.assertIn("/etc/pgtools/manifest-tag.pl", self.calls)
                self.assertIn("compose ci-previous ", self.calls)
                self.assertEqual("/deploy/compose.previous.yaml" in self.calls, probe == "no-check")
                self.assertEqual(self.overlay, "services:\n  upgrade-rollback:\n    secrets:\n"
                                 "      - backup_manifest_key\n" if probe == "check" else "")
                self.assertIn("config --format json", self.calls)
                self.assertIn("--entrypoint sh backup", self.calls)
                self.assertIn("/run/secrets/backup_manifest_key", self.calls)

    def test_manifest_steps_refuse_probe_plan_config_and_container_faults(self):
        for probe, fault, key in (("other", "", None), ("check", "probe", None),
                                  ("check", "manifest-plan", None), ("check", "manifest-key", None),
                                  ("check", "compose", None), ("check", "container", None),
                                  ("check", "", "no-key"), ("no-check", "", "key")):
            with self.subTest(probe=probe, fault=fault, key=key):
                code, _, error = self.run_step("probe_previous_manifest; check_previous_manifest_key", probe, fault, key)
                self.assertEqual(code, 1)
                self.assertIn("CI:", error)
        code, _, error = self.run_step("probe_previous_manifest; CONTAINER_KEY_STATE=no-key; check_previous_manifest_key",
                                      "check")
        self.assertEqual(code, 1)
        self.assertIn("key state", error)
        code, _, error = self.run_step("probe_previous_manifest; UPGRADE_COMPOSE_DIR=/previous; previous_compose config")
        self.assertEqual(code, 1)
        self.assertIn("new Compose tree", error)

    def test_rollback_checks_original_before_copy_and_keeps_previous_image_check(self):
        for probe in ("check", "no-check"):
            with self.subTest(probe=probe):
                code, output, error = self.run_step("probe_previous_manifest; restore_previous 20261116_1200_premigrate", probe)
                self.assertEqual((code, error), (0, ""))
                self.assertIn("recipe_20261116_1200_premigrate.dump.sha256.hmac", self.copied)
                check = "run --rm --no-deps -T restore-volume --check-only 20261116_1200_premigrate"
                self.assertIn(check, self.calls)
                line = next(line for line in self.calls.splitlines() if check in line)
                self.assertIn("compose ci-upgrade ", line)
                self.assertIn(str(pathlib.Path(__file__).resolve().parents[1] / "compose.yaml"), line)
                self.assertNotIn("BACKUP_DIR", line)
                self.assertNotIn("-f ", line)
                self.assertLess(self.calls.index(check), self.calls.index("sudo install"))
                self.assertEqual(self.calls.count("restore-volume --check-only"), 2 if probe == "check" else 1)
                self.assertIn('"message":"the set passed the check"', output)
                self.assertEqual("rollback-key.compose.yaml" in self.calls, probe == "check")

    def test_rollback_refuses_copy_restore_and_tag_check_failures(self):
        for fault in ("copy", "compose", "tag", "tag-log"):
            with self.subTest(fault=fault):
                code, _, error = self.run_step("probe_previous_manifest; restore_previous", "check", fault)
                self.assertEqual(code, 1)
                self.assertIn("CI:", error)

    def test_new_image_check_failure_stops_before_copy_for_both_probe_results(self):
        for probe in ("check", "no-check"):
            with self.subTest(probe=probe):
                code, _, error = self.run_step("probe_previous_manifest; restore_previous 20261116_1200_premigrate",
                                               probe, "new-check")
                self.assertNotEqual(code, 0)
                self.assertIn("stub new-image check failed", error)
                self.assertNotIn("sudo install", self.calls)
                self.assertNotIn("-T upgrade-rollback", self.calls)
                self.assertFalse(self.rollback_exists)

    def test_rollback_compares_every_copied_file_and_refuses_each_cmp_failure(self):
        files = ("recipe_20261116_1200_premigrate.dump", "recipe_20261116_1200_premigrate.counts",
                 "globals_20261116_1200_premigrate.sql", "recipe_20261116_1200_premigrate.dump.sha256",
                 "recipe_20261116_1200_premigrate.dump.sha256.hmac")
        code, _, error = self.run_step("probe_previous_manifest; restore_previous 20261116_1200_premigrate")
        self.assertEqual((code, error), (0, ""))
        comparisons = [line for line in self.calls.splitlines() if line.startswith("sudo cmp ")]
        self.assertEqual(len(comparisons), len(files))
        for name in files:
            self.assertTrue(any("/" + name in line and "/upgrade-rollback/" + name in line for line in comparisons))
            for probe in ("check", "no-check"):
                with self.subTest(name=name, probe=probe):
                    code, _, error = self.run_step("probe_previous_manifest; CMP_FILE=" + name +
                                                   "; restore_previous 20261116_1200_premigrate", probe, "cmp")
                    self.assertNotEqual(code, 0)
                    self.assertIn("stub cmp failed", error)
                    self.assertNotIn("-T upgrade-rollback", self.calls)

    def test_bad_set_case_checks_tag_error_and_removes_copy_for_both_probe_results(self):
        for probe in ("check", "no-check"):
            with self.subTest(probe=probe):
                code, output, error = self.run_step("probe_previous_manifest; check_changed_rollback_set", probe)
                self.assertEqual(code, 0)
                self.assertEqual(error, "CI: the new image refused the rollback set\n")
                self.assertIn('"level":"ERROR","job":"restore-volume","message":'
                              '"the tag of the set does not match the manifest",'
                              '"set":"recipe_20261116_1201_premigrate"}', output)
                installs = [line for line in self.calls.splitlines() if line.startswith("sudo install ")]
                self.assertEqual(len(installs), 5)
                self.assertTrue(all(line.startswith("sudo install -o 999 -g 999 -m 0600 ") for line in installs))
                self.assertNotIn("-T upgrade-rollback", self.calls)
                self.assertFalse(self.rollback_exists)
                self.assertFalse(any("1201" in name for name in self.remaining))

    def test_bad_set_case_refuses_wrong_log_and_zero_exit_and_cleans_up(self):
        for fault in ("bad-accepted", "bad-log", "bad-zero-error", "bad-directory",
                      "bad-copy", "bad-globals", "bad-dd", "bad-manifest", "bad-cleanup"):
            with self.subTest(fault=fault):
                code, _, error = self.run_step("probe_previous_manifest; check_changed_rollback_set", fault=fault)
                self.assertNotEqual(code, 0)
                self.assertIn("CI:", error)
                if fault != "bad-cleanup":
                    self.assertFalse(any("1201" in name for name in self.remaining))

    def test_rollback_refuses_invalid_stamp_and_previous_tree_before_check(self):
        for call in ("restore_previous invalid", "UPGRADE_COMPOSE_DIR=/previous; restore_previous"):
            with self.subTest(call=call):
                code, _, error = self.run_step(call)
                self.assertNotEqual(code, 0)
                self.assertIn("CI:", error)
                self.assertNotIn("compose ", self.calls)
                self.assertFalse(self.rollback_exists)

    def test_bad_set_case_refuses_existing_copy_without_deleting_it(self):
        call = ('printf existing >"$RECIPE_BACKUP_DIR/recipe_20261116_1201_premigrate.dump"; '
                'probe_previous_manifest; check_changed_rollback_set')
        code, _, error = self.run_step(call)
        self.assertNotEqual(code, 0)
        self.assertIn("already exists", error)
        self.assertIn("recipe_20261116_1201_premigrate.dump", self.remaining)
        self.assertNotIn("sudo install", self.calls)

    def test_upgrade_runs_bad_case_before_good_set(self):
        source = pathlib.Path(__file__).with_name("upgrade.sh").read_text()
        branch = source.split("  upgrade)", 1)[1].split(";;", 1)[0]
        self.assertLess(branch.index("check_changed_rollback_set"), branch.index("restore_previous"))

    def test_previous_tree_and_upgrade_use_their_own_compose_paths(self):
        start = shell_function("start_previous")
        self.assertIn("UPGRADE_COMPOSE_DIR=$UPGRADE_PREVIOUS_DIR", start)
        self.assertNotIn("previous_compose", start)
        self.assertNotIn("compose.previous.yaml", start)
        upgrade = shell_function("upgrade")
        self.assertLess(upgrade.index("probe_previous_manifest"), upgrade.index("write_wal_identity"))
        self.assertLess(upgrade.index("check_previous_manifest_key"), upgrade.index("compose up -d backup-premigrate"))
        self.assertIn("previous_compose run --rm --no-deps -T backup restore-test", upgrade)
        self.assertIn("previous_compose run --rm --no-deps -T --entrypoint sh walarchive", shell_function("write_wal_identity"))
        overlay = pathlib.Path(__file__).with_name("upgrade.compose.yaml").read_text()
        self.assertIn("    secrets: []", overlay.split("  upgrade-rollback:", 1)[1])
        self.assertIn("      COMPOSE_SUBNET: ${COMPOSE_SUBNET:?}", overlay.split("  upgrade-rollback:", 1)[1])



class FailedUpgradeTests(unittest.TestCase):
    """Exercise the real failure trap, wrapper, and CI checks with persisted stub state."""

    def run_case(self, call, probe="no-check", fault=""):
        bash = shutil.which("bash")
        if bash is None:
            self.skipTest("bash is not available")
        script = "set -euo pipefail\numask 077\n" + self.STUBS
        script += shell_function("compose", "upgrade-common.sh")
        script += shell_function("restore_tag_on_failure", "upgrade-common.sh")
        if any(name in call for name in ("check_failed_upgrade", "check_early_upgrade_failure",
                                         "reset_failed_upgrade_backup")):
            for name in ("check_failed_upgrade", "check_early_upgrade_failure", "check_previous_backup_schedule",
                         "check_backup_without_key", "reset_failed_upgrade_backup"):
                script += shell_function(name)
        with tempfile.TemporaryDirectory() as temp:
            directory = pathlib.Path(temp)
            (directory / "backup.id").write_text("old-backup\n")
            (directory / "crontab").write_text("/dev/null\n")
            (directory / "tree").write_text("/previous\n")
            env = {**os.environ, "CI_STATE": temp, "GITHUB_WORKSPACE": "/new",
                   "UPGRADE_PREVIOUS_DIR": "/previous", "UPGRADE_COMPOSE_DIR": "/new",
                   "RECIPE_BACKUP_DIR": "/backups", "RECIPE_VERSION": "ci-upgrade",
                   "COMPOSE_PROJECT_NAME": "project", "PREVIOUS_MANIFEST_PROBE": probe, "FAULT": fault}
            done = subprocess.run([bash, "-c", script + "\n" + call], env=env,
                                  capture_output=True, text=True, check=False)
            self.calls = (directory / "calls").read_text() if (directory / "calls").exists() else ""
            self.backup_id = (directory / "backup.id").read_text().strip()
            self.crontab = (directory / "crontab").read_text().strip()
            return done.returncode, done.stdout, done.stderr

    STUBS = r'''
fail() { printf 'CI: %s\n' "$1" >&2; exit 1; }
container() { cat "$CI_STATE/backup.id"; }
check_images() { printf 'images %s %s\n' "$RECIPE_VERSION" "$UPGRADE_COMPOSE_DIR" >>"$CI_STATE/calls"; }
check_image() {
  printf 'image %s %s\n' "$1" "$2" >>"$CI_STATE/calls"
  [[ $FAULT != image ]] || fail 'stub wrong backup image'
}
wait_job() {
  printf 'wait %s\n' "$1" >>"$CI_STATE/calls"
  if [[ $FAULT == accepted ]]; then return 0; fi
  if [[ $FAULT == published ]]; then touch "$CI_STATE/published"; fi
  printf '%s\n' '{"level":"ERROR","job":"backup-once","message":"the manifest key file is missing"}' \
    >"$CI_STATE/backup-premigrate.log"
  fail 'backup-premigrate did not exit with code 0'
}
sudo() {
  if [[ $1 == find ]]; then
    printf 'recipe_existing.dump.sha256\n'
    if [[ -e $CI_STATE/published ]]; then printf 'recipe_new.dump.sha256\n'; fi
    return 0
  fi
  printf 'compose %s %s %s\n' "$RECIPE_VERSION" "$UPGRADE_COMPOSE_DIR" "$*" >>"$CI_STATE/calls"
  local cron=/dev/null arg
  for arg in "$@"; do
    case $arg in
      */upgrade.compose.yaml) cron=/dev/null ;;
      */compose.previous.yaml) cron=/etc/pgtools/crontab.previous ;;
    esac
  done
  if [[ " $* " == *' up -d --no-deps --force-recreate backup ' ]]; then
    if [[ $FAULT == recreate ]]; then printf 'stub recreate failed\n' >&2; return 9; fi
    printf 'new-%s\n' "$(cat "$CI_STATE/backup.id")" >"$CI_STATE/backup.id"
    printf '%s\n' "$cron" >"$CI_STATE/crontab"
    printf '%s\n' "$UPGRADE_COMPOSE_DIR" >"$CI_STATE/tree"
  elif [[ " $* " == *' exec -T backup cat '* ]]; then
    if [[ $FAULT == daily ]]; then printf 'CRON_TZ=UTC\n'; return 0; fi
    printf 'CRON_TZ=UTC\n0 2 * * * backup-once\n'
    case $FAULT in
      schedule) printf '0 3 * * 0 restore-test\n' ;;
    esac
  elif [[ " $* " == *' exec -T backup '* ]]; then
    [[ $FAULT != key ]] || fail 'stub key file exists'
  fi
}
docker() {
  printf 'docker %s\n' "$*" >>"$CI_STATE/calls"
  if [[ $* == *'.Config.Env'* ]]; then
    if [[ $FAULT == crontab ]]; then printf 'BACKUP_CRONTAB=/wrong\n'
    else printf 'BACKUP_CRONTAB=%s\n' "$(cat "$CI_STATE/crontab")"; fi
  elif [[ $* == *'.Mounts'* ]]; then
    local mode=false
    if [[ $FAULT == mount ]]; then mode=true; fi
    printf '/new/deploy/crontab.previous|/etc/pgtools/crontab.previous|%s\n' "$mode"
  else
    fail 'stub unexpected Docker call'
  fi
}
'''

    def test_marker_follows_name_check_and_immediately_precedes_command_one(self):
        lines = [line.strip() for line in shell_function("upgrade").splitlines()]
        self.assertEqual(lines.count("UPGRADE_STEP3_STARTED=true"), 1)
        marker = lines.index("UPGRADE_STEP3_STARTED=true")
        self.assertGreater(marker, lines.index("check_volume_name"))
        self.assertEqual(lines[marker + 1], "compose up -d backup-premigrate")

    def test_trap_recreates_only_keyless_previous_backup_after_step_three(self):
        call = '(trap restore_tag_on_failure EXIT; UPGRADE_STEP3_STARTED=true; exit 7)'
        code, _, error = self.run_case(call)
        self.assertEqual(code, 7)
        self.assertIn("previous image tag is selected", error)
        self.assertEqual(self.backup_id, "new-old-backup")
        self.assertEqual(self.crontab, "/etc/pgtools/crontab.previous")
        command = next(line for line in self.calls.splitlines() if "--force-recreate" in line)
        self.assertTrue(command.startswith("compose ci-previous /new "))
        self.assertLess(command.index("/new/deploy/ci/upgrade.compose.yaml"),
                        command.index("/new/deploy/compose.previous.yaml"))
        self.assertIn("up -d --no-deps --force-recreate backup", command)
        self.assertLess(self.calls.index(command), self.calls.index("images ci-previous /previous"))

    def test_trap_keeps_backup_for_check_missing_probe_missing_marker_and_success(self):
        cases = (("check", "UPGRADE_STEP3_STARTED=true; exit 7", 7),
                 ("", "UPGRADE_STEP3_STARTED=true; exit 7", 7),
                 ("no-check", "exit 7", 7), ("no-check", "UPGRADE_STEP3_STARTED=false; exit 7", 7),
                 ("no-check", "UPGRADE_STEP3_STARTED=true; true", 0))
        for probe, step, expected in cases:
            with self.subTest(probe=probe, step=step):
                code, _, _ = self.run_case("(trap restore_tag_on_failure EXIT; " + step + ")", probe)
                self.assertEqual(code, expected)
                self.assertEqual(self.backup_id, "old-backup")
                self.assertNotIn("--force-recreate", self.calls)

    def test_trap_reports_a_failed_recreation_and_keeps_a_failure_status(self):
        code, _, error = self.run_case('(trap restore_tag_on_failure EXIT; UPGRADE_STEP3_STARTED=true; exit 7)',
                                       fault="recreate")
        self.assertNotEqual(code, 0)
        self.assertEqual(error.count("CI: cannot recreate the previous backup after the failed upgrade.\n"), 1)
        self.assertIn("stub recreate failed", error)
        self.assertIn("images ci-previous /previous", self.calls)

    def test_failed_upgrade_case_checks_both_probe_results_and_resets_schedule(self):
        for probe in ("check", "no-check"):
            with self.subTest(probe=probe):
                code, output, error = self.run_case("check_failed_upgrade; reset_failed_upgrade_backup", probe)
                self.assertEqual((code, error), (0, ""))
                self.assertIn("forced upgrade failure", output)
                self.assertIn("no new set", output)
                self.assertIn("before step 3 kept the backup container", output)
                self.assertIn("up -d backup-premigrate", self.calls)
                self.assertIn("wait backup-premigrate", self.calls)
                self.assertEqual(self.crontab, "/dev/null")
                if probe == "no-check":
                    self.assertIn("0 2 * * * backup-once", output)
                    self.assertIn("BACKUP_CRONTAB=/dev/null", output)
                    self.assertIn("compose ci-previous /previous ", self.calls)
                    self.assertIn("image backup recipe-pgtools:ci-previous", self.calls)
                    self.assertIn("/run/secrets/backup_manifest_key", self.calls)
                else:
                    self.assertEqual(self.backup_id, "old-backup")
                    self.assertNotIn("--force-recreate", self.calls)

    def test_failed_upgrade_case_refuses_success_new_set_and_wrong_recovery_state(self):
        for fault in ("accepted", "published", "image", "crontab", "mount", "schedule", "daily", "key"):
            with self.subTest(fault=fault):
                code, _, error = self.run_case("check_failed_upgrade", fault=fault)
                self.assertNotEqual(code, 0)
                self.assertIn("CI:", error)

    def test_early_failure_case_refuses_replaced_container(self):
        call = 'container() { if [[ -e $CI_STATE/early ]]; then echo replaced; else touch "$CI_STATE/early"; echo old; fi; }; check_early_upgrade_failure'
        code, _, error = self.run_case(call)
        self.assertNotEqual(code, 0)
        self.assertIn("before step 3 replaced the backup container", error)

    def test_reset_case_refuses_wrong_crontab_image_and_key(self):
        for fault in ("crontab", "image", "key", "recreate"):
            with self.subTest(fault=fault):
                code, _, error = self.run_case("reset_failed_upgrade_backup", fault=fault)
                self.assertNotEqual(code, 0)
                self.assertIn("CI:", error)

    def test_upgrade_runs_failure_case_then_reset_before_the_real_marker(self):
        lines = [line.strip() for line in shell_function("upgrade").splitlines()]
        self.assertLess(lines.index("check_failed_upgrade"), lines.index("reset_failed_upgrade_backup"))
        self.assertIn("UPGRADE_STEP3_STARTED=true", lines)
        self.assertLess(lines.index("reset_failed_upgrade_backup"), lines.index("UPGRADE_STEP3_STARTED=true"))
        reset = shell_function("reset_failed_upgrade_backup")
        self.assertIn("RECIPE_VERSION=ci-previous", reset)
        self.assertIn("UPGRADE_COMPOSE_DIR=$UPGRADE_PREVIOUS_DIR", reset)
        self.assertIn("compose up -d --no-deps --force-recreate backup", reset)
        self.assertNotIn("compose.previous.yaml", reset)


if __name__ == "__main__":
    unittest.main()
