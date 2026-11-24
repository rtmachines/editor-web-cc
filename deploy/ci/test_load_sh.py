"""Check the load runner with local programs instead of Docker and HTTP."""

import json
import os
import pathlib
import runpy
import shutil
import subprocess
import sys
import tempfile
import unittest


CI = pathlib.Path(__file__).resolve().parent
ROOT = CI.parent.parent
SCRIPT = CI / "load.sh"

STUB_PROGRAM = r'''
import json
import os
import pathlib
import shutil
import sys

name, *args = sys.argv[1:]
state = pathlib.Path(os.environ["STUB_STATE"])
event_path = pathlib.Path(os.environ["STUB_EVENTS"])
with event_path.open("a") as stream:
    stream.write(json.dumps([name, args]) + "\n")

def option(flag):
    return args[args.index(flag) + 1]

def app_id():
    return (state / "recreated").read_text() if (state / "recreated").exists() else "a" * 64

def app_cgroup():
    return pathlib.Path(os.environ["STUB_CGROUP"]).with_name(app_id())

if name == "curl":
    output = pathlib.Path(option("--output"))
    if any("releases/download/" in arg for arg in args):
        shutil.copyfile(pathlib.Path(os.environ["STUB_BIN"]) / "oha", output)
    else:
        pathlib.Path(option("--cookie-jar")).write_text(
            "# Netscape HTTP Cookie File\n"
            "#HttpOnly_localhost\tFALSE\t/\tTRUE\t0\t__Host-recipe_csrf\tpre-cookie\n")
        output.write_text('<form action="/login"><input name="csrf_pre" value="pre-token"></form>')
elif name == "sha256sum":
    text = sys.stdin.read()
    digest, path = text.strip().split(maxsplit=1)
    assert len(digest) == 64 and all(char in "0123456789abcdef" for char in digest)
    assert pathlib.Path(path).is_file()
    sys.exit(1 if os.environ.get("STUB_BAD_SHA") == "1" else 0)
elif name == "date":
    if args == ["+%s"]:
        print("1793870400")
    else:
        count = sum(json.loads(line)[0] == "date" for line in event_path.read_text().splitlines())
        print(f"2026-11-05T12:00:0{count}.000000000Z")
elif name == "find":
    full_id = app_id()
    names = [args[index + 1] for index, arg in enumerate(args) if arg == "-name"]
    if full_id in names or f"docker-{full_id}.scope" in names:
        print(app_cgroup())
elif name == "load_seed":
    assert args == ["value"]
    (state / "load-value-state.json").write_text("{}\n")
elif name == "load_value":
    assert pathlib.Path(option("--ci-state")) == state
    if args[0] == "round":
        baseline = state / f"load-memory-value-baseline-{option('--round')}.json"
        assert baseline.is_file()
        assert pathlib.Path(option("--cgroup")) == app_cgroup()
        (app_cgroup() / "memory.peak").write_text("150000000\n")
        (app_cgroup() / "memory.current").write_text("130000000\n")
        if option("--round") == "1":
            (app_cgroup() / "memory.events").write_text("oom 1\noom_kill 1\nmax 1\n")
            if os.environ.get("STUB_VALUE_FAILURE") == "round":
                sys.exit(1)
    elif os.environ.get("STUB_VALUE_FAILURE") == args[0]:
        sys.exit(1)
elif name == "oha":
    assert (state / "recreated").is_file()
    assert (state / "ready").is_file()
    assert (state / "preflight").is_file()
    body = pathlib.Path(option("-Z")).read_text()
    assert body.count("\n") == 2000
    assert "csrf_pre=pre-token" in body
    print(pathlib.Path(os.environ["STUB_OHA_JSON"]).read_text())
    if os.environ.get("STUB_OHA_STATUS") != "0":
        print((state / "new_password").read_text(), file=sys.stderr)
    sys.exit(int(os.environ["STUB_OHA_STATUS"]))
elif name == "docker":
    if args[0] == "compose":
        if "up" in args:
            assert "--force-recreate" in args and args[-1] == "app"
            previous = app_cgroup()
            (state / "recreated").write_text(chr(ord(app_id()[0]) + 1) * 64)
            shutil.copytree(previous, app_cgroup())
            (app_cgroup() / "memory.peak").write_text("120000000\n")
            anon = {"b": 90000000, "c": 95000000}[app_id()[0]]
            (app_cgroup() / "memory.stat").write_text(f"anon {anon}\nfile 20000000\n")
            (app_cgroup() / "memory.current").write_text(f"{anon + 20000000}\n")
            (app_cgroup() / "memory.events").write_text("oom 0\noom_kill 0\nmax 0\n")
        elif "ps" in args:
            full_id = app_id()
            print(full_id[:12] if os.environ.get("STUB_SHORT_ID") == "1" else full_id)
        elif "exec" in args and "proxy" in args:
            assert (state / "recreated").is_file()
            assert "http://app:9000/readyz" in args
            (state / "ready").touch()
            print("ok")
        elif "psql" in args:
            if "login_attempt" in " ".join(args):
                assert "succeeded" in " ".join(args)
                print(os.environ.get("STUB_FAILED_LOGINS", "0"))
                (state / "preflight").touch()
            else:
                query = option("-c") if "-c" in args else sys.stdin.read()
                assert "public.recipe_version" in query and "row_version" in query
                assert "v.number = 1" in query and "v.status = 'draft'" in query
                assert "reference_number" in query and all(
                    f"'LOAD-V{number}'" in query for number in range(1, 6))
                version = 4 if app_id()[0] == "c" else 3
                print(json.dumps({"row_versions": {
                    f"LOAD-V{number}": 4 if number == 5 else version
                    for number in range(1, 6)}}))
        else:
            raise AssertionError("unexpected Compose command")
    elif args[0] == "inspect":
        template = option("--format")
        full_id = app_id()
        values = {"{{.Id}}": os.environ.get("STUB_INSPECT_ID", full_id),
                  "{{.HostConfig.Memory}}": "536870912",
                  "{{.State.OOMKilled}}": "false", "{{.RestartCount}}": "0"}
        assert template in values
        print(values[template])
    elif args[0] == "logs":
        assert args[-1] == "b" * 64
        count = 100 if "--since" in args and "--until" in args else 101
        line = json.dumps({
            "timestamp": "2026-11-05T13:50:01.123Z", "level": "ERROR",
            "fields": {"message": "request failed", "error_code": "E-WEB-009",
                       "request_id": "0b6c...",
                       "chain": "The request took too long and was stopped. Try again.",
                       "stage": "password hash permit"},
            "target": "recipe_web::error::log",
            "span": {"route": "/login", "name": "request"},
        })
        print((line + "\n") * count, end="")
    else:
        raise AssertionError("unexpected Docker command")
else:
    raise AssertionError("unexpected stub program")
'''


def posix_path(path):
    text = path.as_posix()
    return "/" + text[0].lower() + text[2:] if os.name == "nt" else text


class LoadScriptTests(unittest.TestCase):
    def setUp(self):
        self.assertTrue(SCRIPT.is_file(), "deploy/ci/load.sh is missing")
        self.bash = shutil.which("bash")
        self.assertIsNotNone(self.bash, "bash is required")
        self.directory = pathlib.Path(self.enterContext(tempfile.TemporaryDirectory(dir=ROOT)))
        self.state = self.directory / "recipe-ci-1-1" / "state"
        self.state.mkdir(parents=True)
        (self.state / "new_password").write_text("fixture-private-password\n")
        (self.state / "root.crt").write_text("fixture certificate\n")
        self.github_env = self.directory / "github-env"
        self.github_env.write_text("sentinel\n")
        self.events_path = self.directory / "events.jsonl"
        self.events_path.touch()
        self.summary = self.directory / "summary"
        self.make_stubs()
        self.make_cgroup()
        fixture = runpy.run_path(str(CI / "test_load_flood.py"))["OHA_JSON"]
        self.oha_json = self.directory / "oha.json"
        self.oha_json.write_text(fixture)

    def make_stubs(self):
        self.stubs = self.directory / "bin"
        self.stubs.mkdir()
        self.program = self.directory / "stub.py"
        self.program.write_text(STUB_PROGRAM)
        for name in ("oha", "docker", "curl", "sha256sum", "date", "find"):
            path = self.stubs / name
            path.write_text('#!/usr/bin/env bash\nexec python3 -I -B "$STUB_PROGRAM" '
                            + name + ' "$@"\n', newline="\n")
            path.chmod(0o700)
        sudo = self.stubs / "sudo"
        sudo.write_text('#!/usr/bin/env bash\nset -euo pipefail\n'
                        'if [[ ${1:-} == --preserve-env=* || ${1:-} == -n ]]; then shift; fi\n'
                        'exec "$@"\n', newline="\n")
        sudo.chmod(0o700)
        python = self.stubs / "python3"
        python.write_text(
            '#!/usr/bin/env bash\n'
            'case ${3:-} in\n'
            '  */load_seed.py|*/load_value.py)\n'
            '    name=${3##*/}\n'
            '    exec "$STUB_PYTHON" "$1" "$2" "$STUB_PROGRAM" "${name%.py}" "${@:4}"\n'
            '    ;;\n'
            'esac\n'
            'exec "$STUB_PYTHON" "$@"\n', newline="\n")
        python.chmod(0o700)

    def make_cgroup(self):
        self.cgroup = self.directory / ("a" * 64)
        self.cgroup.mkdir()
        for name, text in {
            "cpu.max": "100000 100000\n", "memory.max": "536870912\n",
            "memory.peak": "100000000\n", "memory.stat": "anon 80000000\nfile 20000000\n",
            "memory.events": "oom 0\noom_kill 0\nmax 0\n", "memory.current": "100000000\n",
        }.items():
            (self.cgroup / name).write_text(text)

    def run_load(self, *arguments, **overrides):
        env = os.environ.copy()
        for name in ("BASH_ENV", "SHELLOPTS"):
            env.pop(name, None)
        env.update({
            "GITHUB_WORKSPACE": ROOT.as_posix(), "RUNNER_TEMP": self.directory.as_posix(),
            "COMPOSE_PROJECT_NAME": "recipe-ci-1-1", "GITHUB_ACTIONS": "true",
            "GITHUB_ENV": self.github_env.as_posix(), "GITHUB_STEP_SUMMARY": self.summary.as_posix(),
            "STUB_STATE": self.state.as_posix(), "STUB_BIN": posix_path(self.stubs),
            "STUB_PROGRAM": self.program.as_posix(), "STUB_EVENTS": self.events_path.as_posix(),
            "STUB_PYTHON": sys.executable,
            "STUB_CGROUP": self.cgroup.as_posix(), "STUB_OHA_JSON": self.oha_json.as_posix(),
            "STUB_OHA_STATUS": "0", "LOAD_SCRIPT": SCRIPT.as_posix(), **overrides,
        })
        command = 'export PATH="$STUB_BIN:$PATH"\nexec bash "$LOAD_SCRIPT" "$@"'
        return subprocess.run([self.bash, "-c", command, "load-test", *arguments],
                              cwd=ROOT, env=env, text=True, capture_output=True, check=False)

    def events(self, name):
        return [args for program, args in map(json.loads, self.events_path.read_text().splitlines())
                if program == name]

    def test_bad_sha256_stops_before_running_the_download(self):
        result = self.run_load(STUB_BAD_SHA="1")
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("SHA-256", result.stderr)
        self.assertEqual(self.events("oha"), [])
        self.assertFalse((self.state / "recreated").exists())

    def test_oha_failure_preserves_status_and_reads_memory(self):
        result = self.run_load(STUB_OHA_STATUS="7")
        self.assertEqual(result.returncode, 7, result.stderr)
        self.assertIn("oha", result.stderr)
        self.assertTrue((self.state / "load-memory-flood.json").is_file())
        self.assertEqual(len([args for args in self.events("docker") if args[0] == "logs"]), 1)
        self.assertNotIn("fixture-private-password", result.stdout + result.stderr)

    def test_flood_uses_verified_tls_and_exact_request_arguments(self):
        result = self.run_load()
        self.assertEqual(result.returncode, 0, result.stderr)
        calls = self.events("oha")
        self.assertEqual(len(calls), 1)
        args = calls[0]
        self.assertIn("--ipv4", args)
        for flag, value in (("-n", "200"), ("-c", "200"), ("-m", "POST"),
                            ("--output-format", "json")):
            self.assertEqual(args[args.index(flag) + 1], value)
        self.assertIn("https://localhost:8443/login", args)
        self.assertEqual(pathlib.Path(args[args.index("--cacert") + 1]), self.state / "root.crt")
        headers = [args[index + 1] for index, arg in enumerate(args) if arg == "-H"]
        self.assertIn("Origin: https://localhost:8443", headers)
        self.assertIn("Cookie: __Host-recipe_csrf=pre-cookie", headers)
        self.assertIn("Content-Type: application/x-www-form-urlencoded", headers)
        self.assertTrue(pathlib.Path(args[args.index("-Z") + 1]).is_file())
        for flag in ("-t", "--insecure", "--debug", "-d"):
            self.assertNotIn(flag, args)
        self.assertFalse(any("fixture-private-password" in arg for arg in args))
        self.assertEqual(self.github_env.read_text(), "sentinel\n")

    def test_download_pins_the_checker_version_and_checks_before_oha(self):
        result = self.run_load()
        self.assertEqual(result.returncode, 0, result.stderr)
        downloads = [args for args in self.events("curl") if any("releases/download/" in arg for arg in args)]
        self.assertEqual(len(downloads), 1)
        self.assertIn("https://github.com/hatoo/oha/releases/download/v1.16.0/oha-linux-amd64", downloads[0])
        names = [json.loads(line)[0] for line in self.events_path.read_text().splitlines()]
        self.assertLess(names.index("sha256sum"), names.index("oha"))

    def test_recreate_and_ready_check_precede_the_flood(self):
        result = self.run_load()
        self.assertEqual(result.returncode, 0, result.stderr)
        recreate = [args for args in self.events("docker") if "--force-recreate" in args]
        self.assertEqual(len(recreate), 1)
        self.assertIn("--no-deps", recreate[0])
        self.assertIn("--no-build", recreate[0])
        ready = [args for args in self.events("docker") if "http://app:9000/readyz" in args]
        self.assertEqual(len(ready), 1)
        gets = [args for args in self.events("curl") if "https://localhost:8443/login" in args]
        self.assertEqual(len(gets), 1)
        self.assertIn("--cacert", gets[0])

    def test_only_the_flood_log_window_reaches_the_checker(self):
        result = self.run_load()
        self.assertEqual(result.returncode, 0, result.stderr)
        logs = [args for args in self.events("docker") if args[0] == "logs"]
        self.assertEqual(len(logs), 1)
        self.assertEqual(logs[0][logs[0].index("--since") + 1], "2026-11-05T12:00:01.000000000Z")
        self.assertEqual(logs[0][logs[0].index("--until") + 1], "2026-11-05T12:00:02.000000000Z")
        names = [json.loads(line)[0] for line in self.events_path.read_text().splitlines()]
        first, second = [index for index, name in enumerate(names) if name == "date"]
        self.assertEqual(names[first + 1:second], ["oha"])
        self.assertEqual(json.loads((self.state / "load-flood.json").read_text())["permit_log_count"], 100)

    def test_nonempty_failure_count_blocks_oha(self):
        result = self.run_load(STUB_FAILED_LOGINS="1")
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("failed login count must be 0", result.stderr)
        self.assertEqual(self.events("oha"), [])

    def test_memory_readings_cover_both_containers_and_write_the_summary(self):
        result = self.run_load()
        self.assertEqual(result.returncode, 0, result.stderr)
        report = json.loads((self.state / "load-memory.json").read_text())
        self.assertTrue(report["passed"])
        self.assertEqual(report["timings"]["peak"], 100000000)
        self.assertEqual(report["flood"]["peak"], 120000000)
        self.assertEqual(report["peak"], 120000000)
        self.assertIn("ADR-0064 (o) memory condition: met", self.summary.read_text())
        ids = [args[-1] for args in self.events("docker")
               if args[:3] == ["inspect", "--format", "{{.State.OOMKilled}}"]]
        self.assertIn("a" * 64, ids)
        self.assertIn("b" * 64, ids)

    def test_memory_counter_failure_fails_the_runner(self):
        (self.cgroup / "memory.events").write_text("oom 1\noom_kill 0\nmax 0\n")
        result = self.run_load()
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("oom: above 0", result.stderr)

    def test_short_container_id_uses_full_id_for_memory(self):
        result = self.run_load("memory-timings", STUB_SHORT_ID="1")
        self.assertEqual(result.returncode, 0, result.stderr)
        reading = json.loads((self.state / "load-memory-timings.json").read_text())
        self.assertEqual(reading["peak"], "100000000")
        self.assertIn(["inspect", "--format", "{{.Id}}", "a" * 12], self.events("docker"))
        self.assertIn(["inspect", "--format", "{{.State.OOMKilled}}", "a" * 64],
                      self.events("docker"))

    def test_invalid_inspected_container_id_fails_without_secrets(self):
        for value in ("a" * 12, "a" * 63, "a" * 65, "A" * 64, "g" * 64,
                      "fixture-private-password"):
            with self.subTest(value=value):
                result = self.run_load("memory-timings", STUB_SHORT_ID="1", STUB_INSPECT_ID=value)
                self.assertNotEqual(result.returncode, 0)
                self.assertIn("CI: expected a 64-character lowercase hexadecimal app container ID",
                              result.stderr.splitlines())
                self.assertNotIn("fixture-private-password", result.stdout + result.stderr)
                self.assertEqual(self.events("find"), [])

    def test_separate_steps_keep_both_memory_readings(self):
        result = self.run_load("memory-timings")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(self.events("oha"), [])
        result = self.run_load("flood")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.write_value_readings()
        result = self.run_load("memory-check")
        self.assertEqual(result.returncode, 0, result.stderr)
        report = json.loads((self.state / "load-memory.json").read_text())
        self.assertTrue(report["passed"])
        self.assertEqual(report["timings"]["peak"], 100000000)
        self.assertEqual(report["flood"]["peak"], 120000000)
        self.assertEqual({phase: reading["peak"]
                          for phase, reading in report.get("value", {}).items()}, {
            "value-seed": 130000000, "value-precheck": 140000000,
            "value-baseline-1": 150000000, "value-round-1": 160000000,
            "value-baseline-2": 170000000, "value-round-2": 180000000,
        })

    def write_value_readings(self):
        fixture = json.loads((self.state / "load-memory-timings.json").read_text())
        for phase, peak in (("seed", 130000000), ("precheck", 140000000),
                            ("baseline-1", 150000000), ("round-1", 160000000),
                            ("baseline-2", 170000000), ("round-2", 180000000)):
            path = self.state / f"load-memory-value-{phase}.json"
            path.write_text(json.dumps(fixture | {"peak": str(peak)}))

    def test_memory_check_names_a_missing_value_reading(self):
        result = self.run_load("memory-timings")
        self.assertEqual(result.returncode, 0, result.stderr)
        result = self.run_load("flood")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.write_value_readings()
        (self.state / "load-memory-value-round-2.json").unlink()
        result = self.run_load("memory-check")
        self.assertEqual(result.returncode, 1, result.stderr)
        self.assertIn("value-round-2", result.stderr)

    def value_events(self):
        events = []
        for name, args in map(json.loads, self.events_path.read_text().splitlines()):
            if name == "load_value":
                number = args[args.index("--round") + 1] if "--round" in args else ""
                events.append((args[0], number))
            elif "--force-recreate" in args:
                events.append(("recreate", ""))
            elif "psql" in args:
                events.append(("versions", ""))
            elif args[:3] == ["inspect", "--format", "{{.State.OOMKilled}}"]:
                events.append(("reading", args[-1][0]))
        return events

    def test_value_reads_the_retiring_app_and_each_baseline_in_order(self):
        result = self.run_load("value")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(self.value_events(), [
            ("single", "1"), ("single", "2"), ("versions", ""), ("check-precheck", ""),
            ("reading", "a"), ("recreate", ""), ("reading", "b"), ("round", "1"),
            ("reading", "b"), ("versions", ""), ("check-round", "1"), ("recreate", ""),
            ("reading", "c"), ("round", "2"), ("reading", "c"), ("versions", ""),
        ])
        for phase, container, current in (
            ("precheck", "a", "100000000"), ("baseline-1", "b", "110000000"),
            ("round-1", "b", "130000000"), ("baseline-2", "c", "115000000"),
            ("round-2", "c", "130000000"),
        ):
            reading = json.loads((self.state / f"load-memory-value-{phase}.json").read_text())
            self.assertEqual(reading["container_id"], container * 64)
            self.assertEqual(reading["current"], current)
            self.assertEqual(reading["oom_kill"], "1" if phase == "round-1" else "0")
        for phase, version in (("precheck", 3), ("1", 3), ("2", 4)):
            versions = json.loads((self.state / f"load-value-versions-{phase}.json").read_text())
            self.assertEqual(versions, {"row_versions": {
                "LOAD-V1": version, "LOAD-V2": version, "LOAD-V3": version,
                "LOAD-V4": version, "LOAD-V5": 4,
            }})
        for args in self.events("load_value"):
            if args[0] in ("single", "round"):
                self.assertIn("https://localhost:8443", args)
                self.assertIn((self.state / "root.crt").as_posix(), args)
            if args[0] == "single":
                self.assertEqual(args[args.index("--reference") + 1], "LOAD-V5")
            if args[0] == "round":
                number = int(args[args.index("--round") + 1])
                self.assertEqual(args[args.index("--reference-anon") + 1],
                                 ("90000000", "95000000")[number - 1])

    def test_value_stops_before_recreate_when_the_precheck_fails(self):
        result = self.run_load("value", STUB_VALUE_FAILURE="check-precheck")
        self.assertEqual(result.returncode, 1, result.stderr)
        self.assertEqual(self.value_events()[-1:], [("check-precheck", "")], result.stderr)
        self.assertFalse((self.state / "recreated").exists())
        self.assertFalse((self.state / "load-memory-value-precheck.json").exists())

    def assert_round_one_stop(self, failure):
        result = self.run_load("value", STUB_VALUE_FAILURE=failure)
        self.assertEqual(result.returncode, 1, result.stderr)
        recreated = self.state / "recreated"
        self.assertTrue(recreated.is_file(), result.stderr)
        self.assertEqual(recreated.read_text(), "b" * 64)
        reading_path = self.state / "load-memory-value-round-1.json"
        self.assertTrue(reading_path.is_file(), result.stderr)
        reading = json.loads(reading_path.read_text())
        self.assertEqual(reading["peak"], "150000000")
        self.assertFalse((self.state / "load-memory-value-baseline-2.json").exists())

    def test_value_keeps_the_round_one_reading_when_the_client_fails(self):
        self.assert_round_one_stop("round")
        self.assertEqual(self.value_events()[-1], ("reading", "b"))

    def test_value_keeps_the_round_one_reading_when_the_check_fails(self):
        self.assert_round_one_stop("check-round")
        self.assertEqual(self.value_events()[-1], ("check-round", "1"))

    def test_value_seed_writes_the_start_time_and_reads_memory(self):
        result = self.run_load("value-seed")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(self.events("load_seed"), [["value"]])
        self.assertEqual((self.state / "load-value-leg-start").read_text(), "1793870400\n")
        reading = json.loads((self.state / "load-memory-value-seed.json").read_text())
        self.assertEqual(reading["container_id"], "a" * 64)
        names = [name for name, _ in map(json.loads, self.events_path.read_text().splitlines())]
        self.assertLess(names.index("load_seed"), names.index("find"))

    def test_value_check_passes_the_job_start_and_summary_paths(self):
        result = self.run_load("value-check")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(self.events("load_value"), [[
            "check", "--ci-state", self.state.as_posix(),
            "--job-start", (self.directory / "load-job-start").as_posix(),
            "--summary", self.summary.as_posix(),
        ]])


if __name__ == "__main__":
    unittest.main()
