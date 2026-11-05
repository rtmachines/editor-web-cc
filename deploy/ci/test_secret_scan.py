"""Check that smoke artifacts never expose either password."""

import base64
import io
import json
import os
import pathlib
import secrets
import subprocess
import sys
import tempfile
import unittest
import zipfile


SCRIPT = pathlib.Path(__file__).with_name("secret_scan.py")
PASSWORD_NAMES = ("E2E_ADMIN_PASSWORD", "E2E_APPROVER_PASSWORD")


class SecretScanTests(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        self.root = pathlib.Path(self.directory.name)
        self.traces = self.root / "test-results"
        self.report = self.root / "playwright-report"
        self.traces.mkdir()
        self.report.mkdir()
        self.passwords = {name: secrets.token_hex(16) for name in PASSWORD_NAMES}
        self.environment = {**os.environ, **self.passwords}
        self.environment.pop("E2E_ONE_TIME_FILE", None)

    def scan(self, paths=None, environment=None):
        return subprocess.run(
            [sys.executable, "-I", "-B", str(SCRIPT),
             *map(str, paths if paths is not None else (self.traces, self.report))],
            env=self.environment if environment is None else environment,
            capture_output=True, check=False,
        )

    def assert_failure(self, result):
        self.assertEqual(result.returncode, 1)
        self.assertEqual(
            result.stderr.splitlines(), [b"CI secret scan failed. Do not upload the artifacts."])
        self.assertEqual(result.stdout, b"")
        for password in self.passwords.values():
            self.assertFalse(password.encode() in result.stderr, "Failure text exposed a password.")
        self.assertFalse(b"Traceback" in result.stderr)

    def trace(self, path, password=None):
        path.parent.mkdir(parents=True, exist_ok=True)
        request = {"request": {"postData": {"text": "login=ci-admin"}}}
        if password is not None:
            request["request"]["postData"]["text"] += "&password=" + password
        with zipfile.ZipFile(path, "w", compression=zipfile.ZIP_DEFLATED) as archive:
            archive.writestr("test.trace", '{"type":"context-options"}')
            archive.writestr("resources/request-body", json.dumps(request))

    def test_compressed_request_body_rejects_each_password(self):
        self.trace(self.traces / "first" / "trace.zip")
        for name in PASSWORD_NAMES:
            with self.subTest(variable=name):
                self.trace(self.traces / "second" / "trace.zip", self.passwords[name])
                self.assert_failure(self.scan())

    def test_report_rejects_each_password(self):
        for name in PASSWORD_NAMES:
            with self.subTest(variable=name):
                (self.report / "index.html").write_text(
                    "<pre>" + self.passwords[name] + "</pre>", encoding="utf-8")
                self.assert_failure(self.scan())

    def report_payload(self, password=None):
        title = 'Fill "' + (password if password is not None else "public value") + '"'
        report = {"tests": [{"results": [{"steps": [{"title": title}]}]}]}
        source = io.BytesIO()
        with zipfile.ZipFile(source, "w", compression=zipfile.ZIP_DEFLATED) as archive:
            archive.writestr("report.json", json.dumps(report))
        return base64.b64encode(source.getvalue()).decode("ascii")

    def embedded_report(self, payload):
        return '<!DOCTYPE html><template id="playwrightReportBase64">' + payload + '</template>'

    def test_embedded_report_rejects_each_password(self):
        for name in PASSWORD_NAMES:
            with self.subTest(variable=name):
                payload = "data:application/zip;base64," + self.report_payload(self.passwords[name])
                (self.report / "index.html").write_text(
                    self.embedded_report(payload), encoding="utf-8")
                self.assert_failure(self.scan())

    def test_clean_embedded_report_passes(self):
        for prefix in ("data:application/zip;base64,", ""):
            with self.subTest(data_uri=bool(prefix)):
                (self.report / "index.html").write_text(
                    self.embedded_report(prefix + self.report_payload()), encoding="utf-8")
                result = self.scan()
                self.assertEqual(result.returncode, 0)
                self.assertEqual(result.stderr, b"")

    def test_template_without_data_uri_rejects_password(self):
        payload = self.report_payload(self.passwords[PASSWORD_NAMES[0]])
        (self.report / "index.html").write_text(
            self.embedded_report("\n" + payload[:40] + "\n" + payload[40:] + "\n"),
            encoding="utf-8")
        self.assert_failure(self.scan())

    def test_clean_template_with_wrapped_data_uri_passes(self):
        payload = self.report_payload()
        (self.report / "index.html").write_text(
            self.embedded_report("data:application/zip;base64,\n" + payload[:40]
                                 + "\n" + payload[40:] + "\n"), encoding="utf-8")
        result = self.scan()
        self.assertEqual(result.returncode, 0)
        self.assertEqual(result.stderr, b"")

    def test_every_zip_data_uri_is_scanned(self):
        clean = "data:application/zip;base64," + self.report_payload()
        leaked = "data:application/zip;base64," + self.report_payload(self.passwords[PASSWORD_NAMES[1]])
        (self.report / "index.html").write_text(
            '<a href="' + clean + '"></a><a href="' + leaked + '"></a>', encoding="utf-8")
        self.assert_failure(self.scan())

    def test_zip_data_uri_in_processing_instruction_is_scanned(self):
        payload = "data:application/zip;base64," + self.report_payload(self.passwords[PASSWORD_NAMES[0]])
        (self.report / "index.html").write_text(
            '<?report payload="' + payload + '"?>', encoding="utf-8")
        self.assert_failure(self.scan())

    def test_invalid_embedded_payload_fails(self):
        for prefix in ("data:application/zip;base64,", ""):
            for payload in ("", "%%%", "UEs", "bm90IGEgemlw", self.report_payload() + "!"):
                with self.subTest(data_uri=bool(prefix), malformed=True):
                    (self.report / "index.html").write_text(
                        self.embedded_report(prefix + payload), encoding="utf-8")
                    self.assert_failure(self.scan())

    def test_embedded_archives_are_scanned_recursively(self):
        leaked = self.embedded_report(
            "data:application/zip;base64," + self.report_payload(self.passwords[PASSWORD_NAMES[0]]))
        source = io.BytesIO()
        with zipfile.ZipFile(source, "w", compression=zipfile.ZIP_DEFLATED) as archive:
            archive.writestr("nested.html", leaked)
        outer = self.embedded_report(
            "data:application/zip;base64," + base64.b64encode(source.getvalue()).decode("ascii"))
        with zipfile.ZipFile(self.traces / "trace.zip", "w") as archive:
            archive.writestr("index.html", outer)
        self.assert_failure(self.scan())

    def test_one_time_password_file_adds_stripped_secret(self):
        password = secrets.token_hex(16)
        self.passwords["one_time"] = password
        secret_file = self.root / "one-time"
        secret_file.write_text(" \n" + password + "\n ", encoding="utf-8")
        environment = {**self.environment, "E2E_ONE_TIME_FILE": str(secret_file)}
        self.assertEqual(self.scan(environment=environment).returncode, 0)
        for embedded in (False, True):
            with self.subTest(embedded=embedded):
                content = self.embedded_report(self.report_payload(password)) if embedded else password
                (self.report / "index.html").write_text(content, encoding="utf-8")
                self.assert_failure(self.scan(environment=environment))

    def test_missing_or_empty_one_time_password_file_passes(self):
        secret_file = self.root / "one-time"
        environment = {**self.environment, "E2E_ONE_TIME_FILE": str(secret_file)}
        for content in (None, "", " \n\t "):
            with self.subTest(missing=content is None):
                if content is not None:
                    secret_file.write_text(content, encoding="utf-8")
                result = self.scan(environment=environment)
                self.assertEqual(result.returncode, 0)
                self.assertEqual(result.stderr, b"")

    def test_binary_attachment_rejects_password(self):
        (self.report / "attachment.bin").write_bytes(
            b"\xff\x00" + self.passwords[PASSWORD_NAMES[1]].encode() + b"\x00\xfe")
        self.assert_failure(self.scan())

    def test_clean_files_and_archives_pass_without_changes(self):
        self.trace(self.traces / "first" / "trace.zip")
        self.trace(self.report / "data" / "trace.zip")
        (self.report / "index.html").write_text("<p>All checks passed.</p>", encoding="utf-8")
        (self.report / "attachment.bin").write_bytes(b"\xff\x00\xfe")
        before = {path: path.read_bytes() for path in self.root.rglob("*") if path.is_file()}
        result = self.scan()
        self.assertEqual(result.returncode, 0)
        self.assertEqual(result.stderr, b"")
        after = {path: path.read_bytes() for path in self.root.rglob("*") if path.is_file()}
        self.assertEqual(before, after)

    def test_individual_file_argument_rejects_password(self):
        path = self.report / "index.html"
        path.write_text(self.passwords[PASSWORD_NAMES[0]], encoding="utf-8")
        self.assert_failure(self.scan([path]))

    def test_missing_or_empty_password_fails(self):
        for name in PASSWORD_NAMES:
            for value in (None, ""):
                with self.subTest(variable=name, empty=value == ""):
                    environment = {**os.environ, **self.passwords}
                    if value is None:
                        environment.pop(name, None)
                    else:
                        environment[name] = value
                    self.assert_failure(self.scan(environment=environment))

    def test_missing_input_fails(self):
        self.assert_failure(self.scan([self.root / "missing"]))

    def test_no_arguments_fails(self):
        self.assert_failure(self.scan([]))

    def test_corrupt_archive_fails_without_its_contents(self):
        (self.traces / "trace.zip").write_bytes(
            b"broken zip " + self.passwords[PASSWORD_NAMES[0]].encode())
        self.assert_failure(self.scan())

    def test_failure_does_not_print_archive_member_name(self):
        with zipfile.ZipFile(self.traces / "trace.zip", "w") as archive:
            archive.writestr(self.passwords[PASSWORD_NAMES[0]] + ".zip", b"broken zip")
        self.assert_failure(self.scan())


if __name__ == "__main__":
    unittest.main()
