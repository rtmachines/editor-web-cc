"""Check load timings with a monotonic clock and fake HTTP connections."""

import contextlib
import importlib.util
import io
import json
import pathlib
import runpy
import tempfile
import unittest
import urllib.parse
from unittest.mock import Mock, patch


def response(status=200, body="", headers=()):
    return Mock(status=status, read=Mock(return_value=body.encode()),
                getheaders=Mock(return_value=list(headers)))


def form(action, token, row_version=None):
    fields = f'<input name="csrf_token" value="{token}">'
    if row_version is not None:
        fields += f'<input name="row_version" value="{row_version}">'
    return f'<form action="{action}">{fields}</form>'


class LoadTimingTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        path = pathlib.Path(__file__).with_name("load_timing.py")
        spec = importlib.util.spec_from_file_location("load_timing", path)
        cls.module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(cls.module)

    def setUp(self):
        self.state = pathlib.Path(self.enterContext(tempfile.TemporaryDirectory()))
        self.enterContext(patch("ssl.create_default_context"))
        self.factory = Mock()
        self.admin = self.module.Client(self.state, connection_factory=self.factory)
        self.approver = self.module.Client(self.state, connection_factory=self.factory)
        self.admin.cookies = {"__Host-recipe_session": "author-session"}
        self.approver.cookies = {"__Host-recipe_session": "approver-session"}

    def answers(self, *answers):
        connections = [Mock(getresponse=Mock(return_value=answer)) for answer in answers]
        self.factory.side_effect = connections
        return connections

    def test_time_below_ten_seconds_passes(self):
        self.answers(response())
        with patch.object(self.module.time, "monotonic", side_effect=[100, 109.9]):
            seconds = self.module.measure_request(self.admin, "ApproveVersion", "/test")
        self.assertAlmostEqual(seconds, 9.9)
        self.module.check_bounds({"ApproveVersion": seconds})

    def test_time_at_ten_seconds_fails_with_request_name(self):
        for name in ("ApproveVersion", "CreateVersion", "GetVersion"):
            with self.subTest(name=name):
                self.answers(response())
                with patch.object(self.module.time, "monotonic", side_effect=[100, 110]):
                    seconds = self.module.measure_request(self.admin, name, "/test")
                with self.assertRaisesRegex(ValueError, name):
                    self.module.check_bounds({name: seconds})

    def test_nonfinite_and_negative_times_fail(self):
        for seconds in (float("nan"), float("inf"), -1):
            with self.subTest(seconds=seconds):
                with self.assertRaisesRegex(ValueError, "GetVersion"):
                    self.module.check_bounds({"GetVersion": seconds})

    def test_failed_answer_reports_status_without_body(self):
        self.answers(response(503, "private-value E-WEB-009"))
        with self.assertRaises(self.module.HttpCheckError) as raised:
            self.module.measure_request(self.admin, "GetVersion", "/test")
        message = str(raised.exception)
        self.assertIn("HTTP 503", message)
        self.assertIn("E-WEB-009", message)
        self.assertNotIn("private-value", message)
        self.assertIs(self.admin.connection_factory, self.factory)

    def test_redirect_fails_without_following_location(self):
        for status in (301, 302, 303, 307, 308):
            with self.subTest(status=status):
                self.factory.reset_mock()
                connections = self.answers(response(status, headers=[("Location", "/login")]))
                with self.assertRaisesRegex(self.module.HttpCheckError, f"HTTP {status}"):
                    self.module.measure_request(self.admin, "GetVersion", "/test")
                self.assertEqual(self.factory.call_count, 1)
                connections[0].close.assert_called_once()

    def test_create_redirect_succeeds_without_following_the_new_draft(self):
        destination = "/recipes/R-LOAD/versions/2"
        connections = self.answers(response(303, headers=[("Location", destination)]))
        with patch.object(self.module.time, "monotonic", side_effect=[100, 109.9]):
            seconds = self.module.measure_request(
                self.admin, "CreateVersion", "/recipes/R-LOAD/versions", {}, hx=True,
                redirect_to=destination)
        self.assertAlmostEqual(seconds, 9.9)
        self.assertEqual(self.factory.call_count, 1)
        connections[0].close.assert_called_once()

    def test_create_requires_the_success_redirect_and_its_destination(self):
        for status, location in ((200, ""), (303, "/login"), (303, ""),
                                 (303, "/recipes/R-LOAD/versions/1")):
            with self.subTest(status=status, location=location):
                self.answers(response(status, headers=[("Location", location)]))
                with self.assertRaisesRegex(self.module.HttpCheckError, f"HTTP {status}"):
                    self.module.measure_request(
                        self.admin, "CreateVersion", "/recipes/R-LOAD/versions", {}, hx=True,
                        redirect_to="/recipes/R-LOAD/versions/2")

    def test_clock_covers_connect_and_last_byte_but_not_close(self):
        events = []
        now = [100.0]
        answer = response()

        def read():
            events.append("last byte")
            now[0] += 7.9
            return b"complete body"

        def connect(*args, **kwargs):
            events.append("connect")
            now[0] += 2
            return connection

        def close():
            events.append("close")
            now[0] += 50

        def clock():
            events.append("clock")
            return now[0]

        answer.read.side_effect = read
        connection = Mock(getresponse=Mock(return_value=answer), close=Mock(side_effect=close))
        self.factory.side_effect = connect
        with patch.object(self.module.time, "monotonic", side_effect=clock):
            seconds = self.module.measure_request(self.admin, "GetVersion", "/test")
        self.assertAlmostEqual(seconds, 9.9)
        self.assertEqual(events, ["clock", "connect", "last byte", "clock", "close"])
        self.assertIs(self.admin.connection_factory, self.factory)

    def run_timings(self, approve_seconds=1):
        page = "/recipes/R-LOAD/versions/1"
        connections = self.answers(
            response(body=form(page + "/approve", "approve-token", 4001)),
            response(headers=[("HX-Trigger", '{"rowVersion":4002}')]),
            response(body=form("/recipes/R-LOAD/versions", "create-token")),
            response(303, headers=[("Location", "/recipes/R-LOAD/versions/2")]), response(),
        )
        with patch.object(self.module.time, "monotonic",
                          side_effect=[100, 100 + approve_seconds, 200, 202, 300, 303]):
            timings = self.module.run_timings(self.admin, self.approver, "R-LOAD", 1)
        return timings, connections

    def test_three_requests_use_separate_connections_sessions_and_browser_headers(self):
        timings, connections = self.run_timings()
        self.assertEqual(timings, {"ApproveVersion": 1, "CreateVersion": 2, "GetVersion": 3})
        expected = [
            ("GET", "/recipes/R-LOAD/versions/1/approve"),
            ("POST", "/recipes/R-LOAD/versions/1/approve"),
            ("GET", "/recipes/R-LOAD/versions/new"),
            ("POST", "/recipes/R-LOAD/versions"),
            ("GET", "/recipes/R-LOAD/versions/1"),
        ]
        self.assertEqual(self.factory.call_count, 5)
        for index, connection in enumerate(connections):
            method, path, body, headers = connection.request.call_args.args
            self.assertEqual((method, path), expected[index])
            self.assertEqual(headers["Sec-Fetch-Site"], "same-origin")
            self.assertEqual(headers["Accept"], "text/html")
            session = "approver-session" if index < 2 else "author-session"
            self.assertEqual(headers["Cookie"], "__Host-recipe_session=" + session)
            connection.close.assert_called_once()
            if method == "POST":
                self.assertEqual(headers["HX-Request"], "true")
                self.assertEqual(headers["Origin"], "https://localhost:8443")
                self.assertEqual(headers["Content-Type"], "application/x-www-form-urlencoded")
            else:
                self.assertNotIn("HX-Request", headers)
        approve = urllib.parse.parse_qs(connections[1].request.call_args.args[2].decode())
        self.assertEqual(approve, {"csrf_token": ["approve-token"], "row_version": ["4001"]})
        create = urllib.parse.parse_qs(connections[3].request.call_args.args[2].decode())
        self.assertEqual(create["csrf_token"], ["create-token"])
        self.assertEqual(create["based_on"], ["1"])
        self.assertTrue(create["change_note"][0])
        self.assertNotIn("row_version", create)

    def test_slow_success_still_collects_all_three_times_before_the_bound_check(self):
        timings, _ = self.run_timings(approve_seconds=10)
        self.assertEqual(len(timings), 3)
        with self.assertRaisesRegex(ValueError, "ApproveVersion"):
            self.module.check_bounds(timings)

    def test_report_appends_seed_and_three_times_and_writes_json(self):
        summary = self.state / "summary.md"
        summary.write_text("Earlier step\n", encoding="utf-8")
        output = self.state / "timings.json"
        timings = {"ApproveVersion": 1.25, "CreateVersion": 2.5, "GetVersion": 9.9}
        stdout = io.StringIO()
        with patch.dict("os.environ", {"GITHUB_STEP_SUMMARY": str(summary)}):
            with contextlib.redirect_stdout(stdout):
                self.module.write_report(123.5, timings, output)
        text = summary.read_text(encoding="utf-8")
        self.assertTrue(text.startswith("Earlier step\n"))
        for line in ("Seed: 123.500 s", "ApproveVersion: 1.250 s",
                     "CreateVersion: 2.500 s", "GetVersion: 9.900 s"):
            self.assertIn(line, text)
            self.assertIn(line, stdout.getvalue())
        self.assertEqual(json.loads(output.read_text(encoding="utf-8")),
                         {"seed_seconds": 123.5, "timings_seconds": timings})

    def test_cli_reads_password_files_and_reports_before_failing_a_bound(self):
        for name in ("new_password", "approver_password"):
            (self.state / name).write_text("test-file-value-only", encoding="ascii")
        login_page = '<form action="/login"><input name="csrf_pre" value="pre"></form>'
        login_headers = [("Location", "/recipes"),
                         ("Set-Cookie", "__Host-recipe_session=session; Path=/; Secure")]
        page = "/recipes/R-LOAD/versions/1"
        connections = self.answers(
            response(body=login_page), response(303, headers=login_headers),
            response(body=login_page), response(303, headers=login_headers),
            response(body=form(page + "/approve", "approve-token", 4001)),
            response(), response(body=form("/recipes/R-LOAD/versions", "create-token")),
            response(303, headers=[("Location", "/recipes/R-LOAD/versions/2")]), response(),
        )
        path = pathlib.Path(__file__).with_name("load_timing.py")
        output, summary = self.state / "result.json", self.state / "summary.md"
        args = [str(path), "--reference", "R-LOAD", "--version", "1",
                "--approver", "ci-approver", "--seed-seconds", "123.5", "--output", str(output)]
        stdout, stderr = io.StringIO(), io.StringIO()
        with contextlib.ExitStack() as stack:
            stack.enter_context(patch("sys.argv", args))
            stack.enter_context(patch.dict("os.environ", {
                "CI_STATE": str(self.state), "GITHUB_STEP_SUMMARY": str(summary)}))
            stack.enter_context(patch("http.client.HTTPSConnection", self.factory))
            stack.enter_context(patch("time.monotonic", side_effect=[100, 110, 200, 202, 300, 303]))
            stack.enter_context(contextlib.redirect_stdout(stdout))
            stack.enter_context(contextlib.redirect_stderr(stderr))
            with self.assertRaises(SystemExit) as raised:
                runpy.run_path(str(path), run_name="__main__")
        self.assertEqual(raised.exception.code, 1)
        self.assertIn("ApproveVersion", stderr.getvalue())
        self.assertIn("GetVersion: 3.000 s", summary.read_text(encoding="utf-8"))
        self.assertEqual(json.loads(output.read_text())["seed_seconds"], 123.5)
        self.assertNotIn("test-file-value-only", stdout.getvalue() + stderr.getvalue())
        for index, login in ((1, "ci-admin"), (3, "ci-approver")):
            fields = urllib.parse.parse_qs(connections[index].request.call_args.args[2].decode())
            self.assertEqual(fields["login"], [login])
            self.assertTrue(fields["password"] == ["test-file-value-only"])


if __name__ == "__main__":
    unittest.main()
