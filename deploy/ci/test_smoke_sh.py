"""Check the smoke runner and its artifact gate without a browser."""

import hashlib
import json
import os
import pathlib
import shutil
import subprocess
import tempfile
import unittest


CI = pathlib.Path(__file__).resolve().parent
ROOT = CI.parent.parent
SCRIPT = CI / "smoke.sh"

NPX_STUB = r'''
import hashlib
import json
import os
import pathlib
import re
import sys
import zipfile

args = sys.argv[1:]
admin = os.environ.get("E2E_ADMIN_PASSWORD", "")
approver = os.environ.get("E2E_APPROVER_PASSWORD", "")
state = pathlib.Path(os.environ["CI_STATE"])
observed = {
    "admin_matches": admin == (state / "new_password").read_text().strip(),
    "approver_valid": re.fullmatch(r"[0-9a-f]{32}", approver) is not None,
    "secret_in_argv": any(value and value in arg for value in (admin, approver) for arg in args),
    "hashes": [hashlib.sha256(value.encode()).hexdigest() for value in (admin, approver)],
    "args": args,
    "cwd": str(pathlib.Path.cwd()),
    "base_url": os.environ.get("E2E_BASE_URL"),
    "session_dir": os.environ.get("E2E_STATE_DIR"),
}
pathlib.Path(os.environ["STUB_OBSERVED"]).write_text(json.dumps(observed))
output = pathlib.Path(args[args.index("--output") + 1])
report = pathlib.Path(os.environ["PLAYWRIGHT_HTML_OUTPUT_DIR"])
sessions = pathlib.Path(os.environ["E2E_STATE_DIR"])
for directory in (output, report, sessions):
    directory.mkdir(parents=True, exist_ok=True)
(sessions / "admin.json").write_text('{"cookies": []}')
mode = os.environ["STUB_MODE"]
password = {"admin": admin, "approver": approver}[os.environ["STUB_SECRET"]]
body = "password=" + password if mode == "trace" else "clean request"
with zipfile.ZipFile(output / "trace.zip", "w", zipfile.ZIP_DEFLATED) as archive:
    archive.writestr("test.trace", body)
(report / "index.html").write_text(password if mode == "report" else "clean report")
if mode == "badzip":
    (output / "trace.zip").write_bytes(b"broken archive")
print(password if mode == "log" else "Browser stub finished.")
sys.exit(int(os.environ["STUB_STATUS"]))
'''


class SmokeScriptTests(unittest.TestCase):
    def setUp(self):
        self.assertTrue(SCRIPT.is_file(), "deploy/ci/smoke.sh is missing")
        self.bash = shutil.which("bash")
        self.assertIsNotNone(self.bash, "bash is required")
        self.directory = pathlib.Path(self.enterContext(tempfile.TemporaryDirectory()))
        self.state = self.directory / "recipe-ci-1-1" / "state"
        self.state.mkdir(parents=True)
        (self.state / "new_password").write_text("fixture-admin-password\n")
        self.upload = self.state / "smoke-upload"
        self.github_env = self.directory / "github-env"
        self.github_env.write_text("sentinel\n")
        self.observed = self.directory / "observed.json"
        self.stubs = self.directory / "bin"
        self.stubs.mkdir()
        stub = self.stubs / "npx"
        stub.write_text('#!/usr/bin/env bash\nexec python3 -I -B "$STUB_PROGRAM" "$@"\n', newline="\n")
        stub.chmod(0o700)
        (self.directory / "npx.py").write_text(NPX_STUB)

    def run_smoke(self, mode="clean", status=0, secret="admin"):
        env = os.environ.copy()
        for name in ("E2E_ADMIN_PASSWORD", "E2E_APPROVER_PASSWORD", "BASH_ENV", "SHELLOPTS"):
            env.pop(name, None)
        env.update({
            "GITHUB_WORKSPACE": ROOT.as_posix(),
            "RUNNER_TEMP": self.directory.as_posix(),
            "COMPOSE_PROJECT_NAME": "recipe-ci-1-1",
            "GITHUB_ACTIONS": "true",
            "GITHUB_ENV": self.github_env.as_posix(),
            "STUB_PROGRAM": (self.directory / "npx.py").as_posix(),
            "STUB_OBSERVED": self.observed.as_posix(),
            "STUB_MODE": mode,
            "STUB_STATUS": str(status),
            "STUB_SECRET": secret,
            "SMOKE_SCRIPT": SCRIPT.as_posix(),
            "STUB_BIN": self.stubs.as_posix(),
        })
        # Git Bash needs a POSIX path in PATH, but Python uses native file paths.
        if os.name == "nt":
            env["STUB_BIN"] = "/" + self.stubs.drive[0].lower() + self.stubs.as_posix()[2:]
        program = 'export PATH="$STUB_BIN:$PATH"\nexec bash "$SMOKE_SCRIPT"'
        return subprocess.run([self.bash, "-c", program], env=env, cwd=ROOT,
                              capture_output=True, text=True, check=False)

    def assert_upload_empty(self):
        self.assertFalse(list(self.upload.rglob("*")))

    def assert_masked_output(self, result):
        observed = json.loads(self.observed.read_text())
        masks = [line.removeprefix("::add-mask::") for line in result.stdout.splitlines()
                 if line.startswith("::add-mask::")]
        self.assertEqual(len(masks), 2)
        self.assertEqual([hashlib.sha256(value.encode()).hexdigest() for value in masks],
                         observed["hashes"])
        ordinary = "\n".join(line for line in result.stdout.splitlines()
                             if not line.startswith("::add-mask::")) + result.stderr
        self.assertFalse(any(value in ordinary for value in masks))
        self.assertTrue(result.stdout.startswith("::add-mask::"))

    def test_password_in_trace_blocks_upload(self):
        result = self.run_smoke(mode="trace")
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("CI secret scan failed", result.stderr)
        self.assert_upload_empty()
        self.assert_masked_output(result)

    def test_approver_password_in_trace_blocks_upload_after_suite_failure(self):
        result = self.run_smoke(mode="trace", status=7, secret="approver")
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("CI secret scan failed", result.stderr)
        self.assert_upload_empty()
        self.assert_masked_output(result)

    def test_password_in_report_blocks_upload(self):
        result = self.run_smoke(mode="report", secret="approver")
        self.assertNotEqual(result.returncode, 0)
        self.assert_upload_empty()
        self.assert_masked_output(result)

    def test_password_in_child_output_stays_out_of_logs_and_upload(self):
        result = self.run_smoke(mode="log")
        self.assertNotEqual(result.returncode, 0)
        self.assert_upload_empty()
        self.assert_masked_output(result)

    def test_unreadable_trace_blocks_upload(self):
        result = self.run_smoke(mode="badzip")
        self.assertNotEqual(result.returncode, 0)
        self.assert_upload_empty()

    def test_clean_run_publishes_traces_and_report_without_sessions(self):
        result = self.run_smoke()
        self.assertEqual(result.returncode, 0)
        self.assertTrue((self.upload / "test-results" / "trace.zip").is_file())
        self.assertEqual((self.upload / "playwright-report" / "index.html").read_text(), "clean report")
        self.assertFalse(list(self.upload.rglob("admin.json")))
        self.assert_masked_output(result)

    def test_clean_artifacts_survive_suite_failure_and_preserve_its_status(self):
        result = self.run_smoke(status=7)
        self.assertEqual(result.returncode, 7)
        self.assertTrue((self.upload / "test-results" / "trace.zip").is_file())
        self.assertTrue((self.upload / "playwright-report" / "index.html").is_file())

    def test_child_gets_passwords_only_through_environment(self):
        result = self.run_smoke()
        self.assertEqual(result.returncode, 0)
        observed = json.loads(self.observed.read_text())
        self.assertTrue(observed["admin_matches"])
        self.assertTrue(observed["approver_valid"])
        self.assertFalse(observed["secret_in_argv"])
        self.assertEqual(self.github_env.read_text(), "sentinel\n")
        self.assertEqual(observed["args"][:4],
                         ["playwright", "test", "-c", "playwright.compose.config.ts"])
        self.assertEqual(pathlib.Path(observed["cwd"]), ROOT / "e2e")
        self.assertEqual(observed["base_url"], "https://localhost:8443")
        self.assertFalse(pathlib.Path(observed["session_dir"]).is_relative_to(self.upload))
        self.assert_masked_output(result)


if __name__ == "__main__":
    unittest.main()
