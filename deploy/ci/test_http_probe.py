"""Check the HTTP assertions without a running stack."""

import importlib.util
import contextlib
import io
import pathlib
import runpy
import unittest
from unittest.mock import MagicMock, Mock, call, patch


HEALTH_PATHS = ("/healthz", "/readyz?x=1", "/HEALTHZ", "/healthz/", "//healthz", "/readyz%2F")


class HttpProbeTests(unittest.TestCase):
    def setUp(self):
        path = pathlib.Path(__file__).with_name("http_probe.py")
        spec = importlib.util.spec_from_file_location("http_probe", path)
        self.probe = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(self.probe)

    def test_token_comes_from_the_requested_form(self):
        html = '<form action="/lang"><input name="csrf_pre" value="wrong"></form>'
        html += '<form action="/login"><input value="right&amp;value" name="csrf_pre"></form>'
        self.assertEqual(self.probe.form_token(html, "/login", "csrf_pre"), "right&value")

    def test_missing_token_fails(self):
        with self.assertRaises(ValueError):
            self.probe.form_token('<form action="/login"></form>', "/login", "csrf_pre")

    def test_duplicate_token_fails_without_disclosing_it(self):
        html = '<form action="/login"><input name="csrf_pre" value="secret">'
        html += '<input name="csrf_pre" value="secret"></form>'
        with self.assertRaisesRegex(ValueError, "^expected one CSRF token$"):
            self.probe.form_token(html, "/login", "csrf_pre")

    def test_redirect_requires_both_status_and_destination(self):
        self.probe.check_redirect(303, {"location": "/recipes"}, "/recipes")
        for status, location in [(200, "/recipes"), (303, "/login"), (302, "/recipes")]:
            with self.subTest(status=status, location=location):
                with self.assertRaises(ValueError):
                    self.probe.check_redirect(status, {"location": location}, "/recipes")

    def test_health_accepts_caddy_404_for_all_six_paths(self):
        client = Mock()
        client.request.return_value = (404, {}, "")
        self.probe.health(client)
        self.assertEqual(client.request.call_args_list, [call(path) for path in HEALTH_PATHS])

    def test_health_rejects_request_id_on_each_path(self):
        for path in HEALTH_PATHS:
            for request_id in ("app-request-id", ""):
                with self.subTest(path=path, request_id=request_id):
                    client = Mock()
                    client.request.side_effect = [
                        (404, {"x-request-id": request_id} if item == path else {}, "")
                        for item in HEALTH_PATHS
                    ]
                    with self.assertRaisesRegex(ValueError, "^a public health path returned X-Request-Id: HTTP 404, code none$"):
                        self.probe.health(client)

    def test_health_rejects_nonempty_body_on_each_path(self):
        for path in HEALTH_PATHS:
            with self.subTest(path=path):
                client = Mock()
                client.request.side_effect = [
                    (404, {}, "app response" if item == path else "")
                    for item in HEALTH_PATHS
                ]
                with self.assertRaisesRegex(ValueError, "^a public health path returned a nonempty body: HTTP 404, code none$"):
                    self.probe.health(client)

    def test_health_rejects_non_404_on_each_path(self):
        for path in HEALTH_PATHS:
            with self.subTest(path=path):
                client = Mock()
                client.request.side_effect = [
                    (200 if item == path else 404, {}, "") for item in HEALTH_PATHS
                ]
                with self.assertRaisesRegex(ValueError, "^a public health path did not return 404: HTTP 200, code none$"):
                    self.probe.health(client)


class DiagnosticTests(unittest.TestCase):
    def run_script(self, name, action, connection=None, socket_connection=None, success=False):
        path = pathlib.Path(__file__).with_name(name)
        stderr, stdout = io.StringIO(), io.StringIO()
        with contextlib.ExitStack() as stack:
            stack.enter_context(patch("sys.argv", [str(path), action]))
            stack.enter_context(patch.dict("os.environ", {"CI_STATE": "unused", "PROXY_ADDRESS": "127.0.0.1"}))
            stack.enter_context(patch("os.umask"))
            for method, value in (("exists", False), ("read_text", "password-secret"), ("write_text", None), ("touch", None)):
                stack.enter_context(patch.object(pathlib.Path, method, return_value=value))
            stack.enter_context(patch("ssl.create_default_context"))
            stack.enter_context(patch("http.client.HTTPSConnection", return_value=connection))
            stack.enter_context(patch("http.client.HTTPConnection", return_value=connection))
            stack.enter_context(patch("socket.create_connection", return_value=socket_connection))
            stack.enter_context(contextlib.redirect_stderr(stderr))
            stack.enter_context(contextlib.redirect_stdout(stdout))
            if success:
                runpy.run_path(str(path), run_name="__main__")
            else:
                with self.assertRaises(SystemExit) as raised:
                    runpy.run_path(str(path), run_name="__main__")
                self.assertEqual(raised.exception.code, 1)
        self.assertEqual(stdout.getvalue(), "CI request completed during shutdown.\n" if success else "")
        return stderr.getvalue()

    def response(self, status, body):
        return Mock(status=status, read=Mock(return_value=body.encode()), getheaders=Mock(return_value=[
            ("Set-Cookie", "session=cookie-secret"), ("Location", "/token-secret/E-IDN-999")
        ]))

    def test_command_failure_logs_only_status_and_body_code(self):
        for action, path, token in (("login", "/login", "csrf_pre"), ("password", "/me/password", "csrf_token")):
            for status, code in ((401, "E-WEB-004"), (503, "E-WEB-006")):
                with self.subTest(action=action, status=status):
                    form = f'<form action="{path}"><input name="{token}" value="token-secret"></form>'
                    connection = Mock()
                    connection.getresponse.side_effect = [self.response(200, form), self.response(status, f"body-secret {code} password-secret")]
                    output = self.run_script("http_probe.py", action, connection)
                    self.assertEqual(output, f"unexpected redirect: HTTP {status}, code {code}\n")

    def test_form_failure_logs_no_body_when_code_is_absent_or_invalid(self):
        for body in ("", "body-secret E-web-006 E-WEB-0067 token-secret"):
            with self.subTest(body=body):
                connection = Mock()
                connection.getresponse.return_value = self.response(502, body)
                output = self.run_script("http_probe.py", "login", connection)
                self.assertEqual(output, "the form did not return 200: HTTP 502, code none\n")

    def test_redirect_failure_logs_only_status_and_body_code(self):
        connection = Mock()
        connection.getresponse.return_value = self.response(503, "body-secret E-WEB-006 token-secret")
        output = self.run_script("http_probe.py", "redirect", connection)
        self.assertEqual(output, "HTTP did not redirect to the same HTTPS path: HTTP 503, code E-WEB-006\n")

    def test_shutdown_failure_logs_only_status_and_body_code(self):
        for body, code in ((b"body-secret E-WEB-006 token-secret", "E-WEB-006"), (b"body-secret", "none")):
            with self.subTest(code=code):
                final = b"HTTP/1.1 503 Unavailable\r\nSet-Cookie: session=cookie-secret E-IDN-999\r\n"
                final += f"Content-Length: {len(body)}\r\n\r\n".encode() + body
                connection = Mock()
                connection.getresponse.return_value = Mock(status=503)
                socket_connection = MagicMock()
                socket_connection.__enter__.return_value = socket_connection
                socket_connection.makefile.side_effect = [io.BytesIO(b"HTTP/1.1 100 Continue\r\n\r\n" + final)]
                output = self.run_script("stop_probe.py", "127.0.0.1", connection, socket_connection)
                self.assertEqual(output, f"the server did not complete the request during its drain: HTTP 503, code {code}\n")

    def test_untrusted_exception_messages_remain_hidden(self):
        for name, action in (("http_probe.py", "login"), ("stop_probe.py", "127.0.0.1")):
            with self.subTest(name=name):
                connection = Mock()
                connection.getresponse.side_effect = ValueError("password-secret token-secret cookie-secret")
                socket_connection = MagicMock()
                socket_connection.__enter__.side_effect = ValueError("password-secret token-secret cookie-secret")
                output = self.run_script(name, action, connection, socket_connection)
                expected = "CI HTTP check failed.\n" if name == "http_probe.py" else "CI shutdown request failed.\n"
                self.assertEqual(output, expected)

    def test_shutdown_accepts_chunked_csrf_rejection(self):
        response = b"HTTP/1.1 100 Continue\r\n\r\nHTTP/1.1 403 Forbidden\r\n"
        response += b"Transfer-Encoding: chunked\r\n\r\n5\r\nE-WEB\r\n4\r\n-007\r\n0\r\n\r\n"
        connection = Mock()
        connection.getresponse.return_value = Mock(status=503)
        socket_connection = MagicMock()
        socket_connection.__enter__.return_value = socket_connection
        socket_connection.makefile.side_effect = [io.BytesIO(response)]
        output = self.run_script("stop_probe.py", "127.0.0.1", connection, socket_connection, success=True)
        self.assertEqual(output, "")


if __name__ == "__main__":
    unittest.main()
