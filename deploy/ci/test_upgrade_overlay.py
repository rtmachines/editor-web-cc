"""Check the exit from the previous Compose overlay without Docker."""

import errno
import os
import pathlib
import re
import shutil
import subprocess
import tempfile
import unittest


def shell_function(name, filename="upgrade.sh"):
    source = pathlib.Path(__file__).with_name(filename).read_text(encoding="utf-8")
    marker = f"\n{name}() {{"
    if marker not in source:
        raise AssertionError(f"{filename} has no function {name}")
    return f"{name}() {{" + source.split(marker, 1)[1].split("\n}\n", 1)[0] + "\n}\n"


def run_shell(script, directory, **values):
    bash = shutil.which("bash")
    if bash is None:
        raise unittest.SkipTest("bash is not available")
    env = {**os.environ, "CASE_ROOT": str(directory), **values}
    return subprocess.run([bash, "-c", "umask 077\n" + script], env=env,
                          capture_output=True, text=True, check=False)


class OverlayFunctionTests(unittest.TestCase):
    CONTENT = (b"# Keep spaces and order.\nFIRST=  text  \nRECIPE_VERSION=ci-previous\n"
               b"COMPOSE_FILE=compose.yaml:compose.previous.yaml\n\nLAST=tail")
    IO_FAULT_MODULE = r'''
package OverlayIOFault::Output;
sub TIEHANDLE { bless {}, shift }
sub PRINT { $! = Errno::EIO(); return 0; }
sub CLOSE { return 1; }
sub FILENO { return 1; }
package OverlayIOFault;
use Errno qw(EACCES EIO);
BEGIN {
    my $fault = $ENV{OVERLAY_IO_FAULT};
    my $reads = 0;
    *CORE::GLOBAL::open = sub (*;$@) {
        if ($fault eq "open") { $! = EACCES; return 0; }
        return CORE::open($_[0], $_[1], $_[2]);
    };
    *CORE::GLOBAL::read = sub (*\$$;$) {
        if ($fault eq "read" && $reads++) { $! = EIO; return undef; }
        return CORE::read($_[0], ${$_[1]}, $_[2]);
    };
    *CORE::GLOBAL::close = sub (*) {
        my $input = fileno($_[0]) != 1;
        my $closed = CORE::close($_[0]);
        if ($fault eq "close" && $input) { $! = EIO; return 0; }
        return $closed;
    };
    if ($fault eq "write" || $fault eq "stdout-close") {
        CORE::open(STDOUT, ">", "/dev/full") or die "fault setup: open refused\n";
        if ($fault eq "write") { select STDOUT; $| = 1; }
    }
    if ($fault eq "write-once") {
        tie *STDOUT, "OverlayIOFault::Output" or die "fault setup: tie refused\n";
    }
}
1;
'''

    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = pathlib.Path(self.temp.name)
        self.copy = self.root / "deploy"
        self.copy.mkdir()
        self.env_file = self.copy / ".env"
        self.env_file.write_bytes(self.CONTENT)
        (self.copy / ".env.example").write_text("reference\n")
        self.stub = self.root / "stubs"
        self.stub.mkdir()
        self.function = shell_function("leave_previous_overlay", "upgrade-common.sh")

    def names(self):
        return sorted(p.name for p in self.copy.glob(".env.*"))

    def invoke(self, strict=False, write_tag="ci-upgrade"):
        script = self.function + '\nexport COMPOSE_FILE=compose.yaml:compose.previous.yaml\n'
        if strict:
            script += "set -euo pipefail\n"
        script += ('leave_previous_overlay "$CASE_ROOT/deploy" "$WRITE_TAG"\nstatus=$?\n'
                   'printf "%s|%s\\n" "$status" "${COMPOSE_FILE-unset}"\n')
        return run_shell(script, self.root, WRITE_TAG=write_tag,
                         PATH=str(self.stub) + os.pathsep + os.environ["PATH"])

    def write_stub(self, tool, body):
        path = self.stub / tool
        path.write_text("#!/usr/bin/env bash\n" + body, encoding="utf-8")
        path.chmod(0o755)

    def assert_failed_unchanged(self):
        before = self.env_file.read_bytes()
        names = self.names()
        done = self.invoke()
        self.assertEqual(done.returncode, 0, done.stderr)
        self.assertRegex(done.stdout, r"^[1-9][0-9]*\|compose.yaml:compose.previous.yaml\n$")
        self.assertTrue(done.stderr, "a failed step needs a diagnostic")
        self.assertEqual(self.env_file.read_bytes(), before)
        self.assertEqual(self.names(), names)
        return done

    def assert_success(self, strict=False, write_tag="ci-upgrade"):
        before = self.env_file.read_bytes()
        names = self.names()
        metadata = self.env_file.stat()
        done = self.invoke(strict, write_tag)
        self.assertEqual((done.returncode, done.stdout, done.stderr), (0, "0|unset\n", ""))
        expected = re.sub(rb"^(COMPOSE_FILE|RECIPE_VERSION)=[^\n]*(?:\n|$)", b"", before, flags=re.M)
        self.assertEqual(self.env_file.read_bytes(), f"RECIPE_VERSION={write_tag}\n".encode() + expected)
        after = self.env_file.stat()
        self.assertEqual((after.st_mode, after.st_uid, after.st_gid),
                         (metadata.st_mode, metadata.st_uid, metadata.st_gid))
        self.assertEqual(self.names(), names)

    def test_case_four_read_only_directory_keeps_file_and_shell(self):
        if os.geteuid() == 0:
            self.skipTest("root ignores the read-only directory mode")
        self.env_file.chmod(0o444)
        self.copy.chmod(0o555)
        try:
            self.assert_failed_unchanged()
        finally:
            self.copy.chmod(0o700)
            self.env_file.chmod(0o600)

    def test_unreadable_env_keeps_bytes_mode_and_overlay(self):
        if os.geteuid() == 0:
            self.skipTest("root can read a mode-000 .env; the injected open fault runs as root too")
        before = self.env_file.read_bytes()
        names = self.names()
        self.env_file.chmod(0o000)
        mode = self.env_file.stat().st_mode
        try:
            done = self.invoke()
            self.assertEqual((done.returncode, done.stdout),
                             (0, "1|compose.yaml:compose.previous.yaml\n"), done.stderr)
            self.assertIn("ERROR: cannot leave the previous Compose overlay.\n", done.stderr)
            self.assertEqual(self.env_file.stat().st_mode, mode)
        finally:
            self.env_file.chmod(0o600)
        self.assertEqual(self.env_file.read_bytes(), before)
        self.assertEqual(self.names(), names)

    def test_perl_io_faults_keep_bytes_mode_and_overlay(self):
        perl = shutil.which("perl")
        if perl is None:
            self.skipTest("perl is not available")
        for fault in ("open", "read", "close", "write", "write-once", "stdout-close"):
            with self.subTest(fault=fault):
                self.env_file.write_bytes(self.CONTENT)
                self.env_file.chmod(0o640)
                if fault in ("write", "stdout-close") and not pathlib.Path("/dev/full").exists():
                    self.skipTest("the output fault needs /dev/full")
                (self.stub / "OverlayIOFault.pm").write_text(self.IO_FAULT_MODULE)
                self.write_stub("perl", f'export OVERLAY_IO_FAULT={fault}\n'
                                f'exec "{perl}" -w -I"$CASE_ROOT/stubs" -MOverlayIOFault "$@"\n')
                mode = self.env_file.stat().st_mode
                done = self.assert_failed_unchanged()
                self.assertEqual(done.stdout, "1|compose.yaml:compose.previous.yaml\n")
                self.assertEqual(done.stderr, "ERROR: cannot leave the previous Compose overlay.\n")
                self.assertEqual(self.env_file.stat().st_mode, mode)

    def test_case_five_keeps_mode_owner_and_supplementary_group(self):
        groups = [group for group in os.getgroups() if group != os.getgid()]
        if not groups:
            self.skipTest("the host user has no supplementary group")
        try:
            os.chown(self.env_file, -1, groups[0])
        except OSError as error:
            if error.errno == errno.EINVAL:
                self.skipTest("the sandbox cannot map the supplementary group")
            raise
        self.env_file.chmod(0o644)
        self.assert_success()

    def test_case_five_keeps_mode_and_every_other_byte(self):
        self.env_file.chmod(0o644)
        self.assert_success()

    def test_case_six_accepts_no_compose_file_with_or_without_errexit(self):
        for strict in (False, True):
            with self.subTest(strict=strict):
                self.env_file.write_bytes(b"# Same tag.\nRECIPE_VERSION=ci-upgrade\nOTHER=unchanged\n")
                self.assert_success(strict)

    def test_success_uses_the_write_tag_and_removes_all_old_settings(self):
        self.env_file.write_bytes(self.CONTENT + b"\nRECIPE_VERSION=stale\nCOMPOSE_FILE=stale\n")
        self.assert_success(strict=True, write_tag="v1.2.3-operator")

    def test_empty_env_keeps_the_previous_success_output(self):
        self.env_file.write_bytes(b"")
        done = self.invoke()
        self.assertEqual((done.returncode, done.stdout, done.stderr), (0, "0|unset\n", ""))
        self.assertEqual(self.env_file.read_bytes(), b"")

    def test_case_seven_partial_write_keeps_original_and_removes_temp(self):
        self.write_stub("perl", 'input=${!#}\nhead -c "$(( $(wc -c <"$input") / 2 ))" -- "$input"\n'
                        'printf "stub: partial write.\\n" >&2\nexit 1\n')
        self.assert_failed_unchanged()

    def test_case_eight_rename_failure_keeps_original_and_removes_temp(self):
        self.write_stub("mv", 'printf "stub: rename refused.\\n" >&2\nexit 1\n')
        self.assert_failed_unchanged()

    def test_each_failed_step_stops_before_later_steps(self):
        steps = ("mktemp", "perl", "chown", "chmod", "mv")
        for failed in steps:
            with self.subTest(failed=failed):
                trace = self.root / "trace"
                trace.write_text("")
                for tool in steps:
                    real = shutil.which(tool)
                    if real is None:
                        self.skipTest(tool + " is not available")
                    body = f'printf "{tool}\\n" >>"$CASE_ROOT/trace"\n'
                    if tool == failed:
                        body += 'printf "stub: step refused.\\n" >&2\nexit 1\n'
                    else:
                        body += 'exec "' + real + '" "$@"\n'
                    self.write_stub(tool, body)
                self.assert_failed_unchanged()
                self.assertEqual(trace.read_text().splitlines(), list(steps[:steps.index(failed) + 1]))

    def test_function_has_only_operator_dependencies_and_sets_no_trap(self):
        shell = re.sub(r"perl -e '[^']*'", "perl", self.function)
        self.assertNotRegex(shell, r"\b(?:exit|trap|CI_STATE|GITHUB_\w+|UPGRADE_\w+|RUNNER_\w+)\b")
        source = pathlib.Path(__file__).with_name("upgrade-common.sh").read_text()
        for name in re.findall(r"^([a-z_]+)\(\) \{", source, re.M):
            if name != "leave_previous_overlay":
                self.assertNotRegex(self.function, r"\b" + name + r"\b")
        self.assertNotRegex(self.function, r"\b(?:fail|sudo)\b")
        variables = set(re.findall(r"\$(?:\{)?([A-Za-z_]\w*)", shell))
        self.assertFalse(variables - {"directory", "write_tag", "temp", "tag", "_"}, variables)
        upgrade = pathlib.Path(__file__).with_name("upgrade.sh").read_text()
        self.assertIn('source "$(dirname "${BASH_SOURCE[0]}")/upgrade-common.sh"', upgrade)
        self.assertNotIn("\nleave_previous_overlay() {", upgrade)


class OverlayStepTests(unittest.TestCase):
    FUNCTIONS = ("prepare_overlay_copy", "check_overlay_key_state", "check_overlay_step_two",
                 "check_backup_with_key", "check_overlay_step_six", "overlay_temp_names",
                 "run_overlay_exit", "prepare_overlay_file_case", "write_overlay_fault_stub",
                 "check_overlay_file_result", "check_overlay_file_case", "check_overlay_file_cases")

    STUBS = r'''
fail() { printf 'CI: %s\n' "$1" >&2; exit 1; }
container() {
  if [[ $FAULT == changed-id && -e $CI_STATE/config-read ]]; then printf 'changed\n'
  else cat "$CI_STATE/backup.id"; fi
}
check_image() {
  [[ $(cat "$CI_STATE/image") == "$2" && $FAULT != image ]] || fail 'stub wrong image'
}
sleep() { SECONDS=$((SECONDS + 181)); }
compose() {
  printf 'compose %s %s\n' "$RECIPE_VERSION" "$*" >>"$CI_STATE/calls"
  if [[ " $* " == *' up '* ]]; then
    if [[ $RECIPE_VERSION == ci-previous ]]; then
      [[ $FAULT != previous ]] || fail 'stub previous start refused'
      printf 'previous-backup\n' >"$CI_STATE/backup.id"
      printf 'recipe-pgtools:ci-previous\n' >"$CI_STATE/image"
      rm -f "$CI_STATE/key"
    else
      [[ $FAULT != recreate ]] || fail 'stub new start refused'
      printf 'new-backup\n' >"$CI_STATE/backup.id"
      printf 'recipe-pgtools:ci-upgrade\n' >"$CI_STATE/image"
      touch "$CI_STATE/key"
    fi
  else
    [[ -e $CI_STATE/key && $FAULT != missing-key ]] || fail 'stub missing key'
  fi
}
previous_compose() { RECIPE_VERSION=ci-previous compose "$@"; }
upgrade_check() { "$PYTHON" -I -B "$CHECKER" "$@"; }
'''

    DOCKER = '''#!/usr/bin/env python3
import json, os, pathlib, sys
state = pathlib.Path(os.environ["CI_STATE"])
fault = os.environ["FAULT"]
with (state / "calls").open("a") as calls:
    calls.write("docker " + " ".join(sys.argv[1:]) + "\\n")
if "config" in sys.argv:
    if fault == "config":
        print("stub: config refused", file=sys.stderr)
        sys.exit(1)
    assert "RECIPE_VERSION" not in os.environ
    assert sys.argv[1:] == ["compose", "--profile", "restore", "config", "--format", "json"]
    data = pathlib.Path(".env").read_text()
    overlay = os.environ.get("COMPOSE_FILE") or next(
        (line.split("=", 1)[1] for line in data.splitlines() if line.startswith("COMPOSE_FILE=")), "")
    key = "compose.previous.yaml" not in overlay
    if fault == "wrong-key":
        key = not key
    (state / "config-read").touch()
    print(json.dumps({"services": {name: {"secrets": [{"source": "backup_manifest_key"}] if key else []}
                     for name in ("backup", "backup-premigrate", "restore-volume")}}))
else:
    if fault == "inspect":
        print("stub: inspect refused", file=sys.stderr)
        sys.exit(1)
    assert sys.argv[1:3] == ["inspect", "--format"]
    print("unhealthy" if fault == "health" else "healthy")
'''

    def run_case(self, call, fault="", primary_group_only=False, probe="no-check"):
        functions = "".join(shell_function(name) for name in self.FUNCTIONS)
        functions += shell_function("leave_previous_overlay", "upgrade-common.sh")
        with tempfile.TemporaryDirectory() as temp:
            directory = pathlib.Path(temp)
            deploy = directory / "source" / "deploy"
            deploy.mkdir(parents=True)
            (deploy / ".env.example").write_text("# Keep this.\nRECIPE_VERSION=old\nOTHER=value\n")
            (directory / "backup.id").write_text("old-backup\n")
            (directory / "image").write_text("recipe-pgtools:ci-upgrade\n")
            (directory / "key").touch()
            stub = directory / "bin"
            stub.mkdir()
            (stub / "docker").write_text(self.DOCKER)
            (stub / "docker").chmod(0o755)
            values = {"CI_STATE": temp, "GITHUB_WORKSPACE": str(deploy.parent), "FAULT": fault,
                      "RECIPE_VERSION": "ci-upgrade", "PREVIOUS_MANIFEST_PROBE": probe,
                      "PYTHON": shutil.which("python3"), "CHECKER": str(pathlib.Path(__file__).with_name("upgrade_checks.py")),
                      "PATH": str(stub) + os.pathsep + os.environ["PATH"]}
            stubs = self.STUBS
            if primary_group_only:
                stubs += '\nid() { command id -g; }\n'
            if fault == "leave":
                functions += "\nleave_previous_overlay() { printf 'stub: exit refused.\\n' >&2; return 1; }\n"
            done = run_shell("set -euo pipefail\n" + stubs + functions + "\n" + call, directory, **values)
            self.calls = (directory / "calls").read_text() if (directory / "calls").exists() else ""
            return done

    def test_case_one_selects_key_without_replacing_backup(self):
        done = self.run_case("check_overlay_step_two")
        self.assertEqual((done.returncode, done.stderr), (0, ""))
        self.assertIn("overlay case 1:", done.stdout)
        self.assertEqual(self.calls.count("config --format json"), 2)
        self.assertNotIn(" up ", self.calls)

    def test_case_one_refuses_config_wrong_key_and_changed_backup(self):
        for fault in ("config", "wrong-key", "changed-id", "leave"):
            with self.subTest(fault=fault):
                done = self.run_case("check_overlay_step_two", fault)
                self.assertNotEqual(done.returncode, 0)
                self.assertIn("CI:", done.stderr)

    def test_case_one_refused_exit_stops_before_the_final_config_read(self):
        done = self.run_case("check_overlay_step_two", "leave")
        self.assertNotEqual(done.returncode, 0)
        self.assertIn("the step 2 overlay exit failed", done.stderr)
        self.assertEqual(self.calls.count("config --format json"), 1)
        self.assertNotIn(" up ", self.calls)

    def test_case_two_checks_the_key_and_health(self):
        done = self.run_case("check_backup_with_key 2")
        self.assertEqual((done.returncode, done.stderr), (0, ""))
        self.assertIn("overlay case 2:", done.stdout)
        self.assertIn("exec -T backup test -f /run/secrets/backup_manifest_key", self.calls)
        self.assertIn(".State.Health.Status", self.calls)

    def test_case_two_refuses_missing_key_unhealthy_backup_and_wrong_image(self):
        for fault in ("missing-key", "health", "image", "inspect"):
            with self.subTest(fault=fault):
                done = self.run_case("check_backup_with_key 2", fault)
                self.assertNotEqual(done.returncode, 0)
                self.assertIn("CI:", done.stderr)

    def test_case_three_recreates_previous_then_new_backup_and_checks_key(self):
        done = self.run_case("check_overlay_step_six")
        self.assertEqual((done.returncode, done.stderr), (0, ""))
        self.assertIn("overlay case 3:", done.stdout)
        lines = self.calls.splitlines()
        previous = next(i for i, line in enumerate(lines) if line.startswith("compose ci-previous "))
        new = next(i for i, line in enumerate(lines) if line.startswith("compose ci-upgrade up "))
        self.assertIn("/deploy/compose.previous.yaml up -d --no-deps --force-recreate backup", lines[previous])
        self.assertEqual(lines[new], "compose ci-upgrade up -d --no-deps --force-recreate backup")
        self.assertLess(previous, new)
        self.assertEqual(self.calls.count("config --format json"), 2)

    def test_case_three_refuses_previous_start_recreation_and_missing_key(self):
        for fault in ("previous", "recreate", "missing-key", "config"):
            with self.subTest(fault=fault):
                done = self.run_case("check_overlay_step_six", fault)
                self.assertNotEqual(done.returncode, 0)
                self.assertIn("CI:", done.stderr)

    def test_case_three_refused_exit_starts_no_new_backup(self):
        done = self.run_case("check_overlay_step_six", "leave")
        self.assertNotEqual(done.returncode, 0)
        self.assertIn("the step 6 overlay exit failed", done.stderr)
        self.assertEqual(self.calls.count("config --format json"), 1)
        self.assertNotIn("compose ci-upgrade up ", self.calls)
        self.assertNotIn("exec -T backup", self.calls)

    def test_case_three_also_accepts_a_previous_image_with_the_check(self):
        done = self.run_case("check_overlay_step_six", probe="check")
        self.assertEqual((done.returncode, done.stderr), (0, ""))
        self.assertIn("compose ci-previous up -d --no-deps --force-recreate backup", self.calls)
        self.assertIn("compose ci-upgrade up -d --no-deps --force-recreate backup", self.calls)
        self.assertIn("overlay case 3:", done.stdout)

    def test_ci_file_cases_run_the_pasted_function_and_check_faults(self):
        done = self.run_case("check_overlay_file_cases", primary_group_only=True)
        self.assertEqual(done.returncode, 0, done.stderr)
        for number in range(4, 9):
            self.assertIn(f"overlay case {number}:", done.stdout)

    def test_cases_follow_upgrade_and_rollback_order(self):
        upgrade = shell_function("upgrade")
        order = ("check_failed_upgrade\n", "check_overlay_step_two\n", "check_overlay_file_cases\n",
                 "reset_failed_upgrade_backup\n", "UPGRADE_STEP3_STARTED=true\n", "compose up -d backup-premigrate\n")
        positions = [upgrade.index(step) for step in order]
        self.assertEqual(positions, sorted(positions))
        self.assertLess(upgrade.index("check_smoke"), upgrade.index("check_upgraded_backup"))
        backup = shell_function("check_upgraded_backup")
        start = backup.index("compose up -d --no-deps --wait --wait-timeout 180 backup walarchive")
        self.assertGreater(backup.index("check_backup_with_key 2"), start)
        source = pathlib.Path(__file__).with_name("upgrade.sh").read_text()
        branch = source.split("  upgrade)", 1)[1].split(";;", 1)[0]
        self.assertLess(branch.index("check_changed_rollback_set"), branch.index("restore_previous"))
        self.assertLess(branch.index("restore_previous"), branch.index("check_overlay_step_six"))


if __name__ == "__main__":
    unittest.main()
