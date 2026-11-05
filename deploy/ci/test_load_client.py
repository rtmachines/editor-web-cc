"""Check the load client with fake HTTP connections and real state files."""

import contextlib
import importlib.util
import io
import pathlib
import tempfile
import unittest
import urllib.parse
from unittest.mock import Mock, patch


SESSION = "__Host-recipe_session"
PRE_SESSION = "__Host-recipe_csrf"


def form(action, token="form-token", row_version=None, name="csrf_token"):
    fields = f'<input name="{name}" value="{token}">'
    if row_version is not None:
        fields += f'<input name="row_version" value="{row_version}">'
    return f'<form action="{action}">{fields}</form>'


def response(status=200, body="", headers=()):
    return Mock(status=status, read=Mock(return_value=body.encode()),
                getheaders=Mock(return_value=list(headers)))


class LoadClientTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        path = pathlib.Path(__file__).with_name("load_client.py")
        spec = importlib.util.spec_from_file_location("load_client", path)
        cls.module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(cls.module)

    def setUp(self):
        directory = self.enterContext(tempfile.TemporaryDirectory())
        self.state = pathlib.Path(directory)
        self.secret = "file-value-" + "a" * 16
        (self.state / "new_password").write_text(self.secret + "\n", encoding="ascii")
        self.context = self.enterContext(patch("ssl.create_default_context"))
        self.factory = Mock()
        self.client = self.module.Client(self.state, connection_factory=self.factory)

    def answers(self, *answers):
        connections = [Mock(getresponse=Mock(return_value=answer)) for answer in answers]
        self.factory.side_effect = connections
        return connections

    def sent(self, connection):
        method, path, body, headers = connection.request.call_args.args
        fields = urllib.parse.parse_qs(body.decode("ascii")) if body else {}
        return method, path, fields, headers

    def test_login_reads_state_file_and_sends_pre_session_token_and_cookie(self):
        page = form("/lang", "wrong", name="csrf_pre")
        page += form("/login", "pre&amp;token", name="csrf_pre")
        connections = self.answers(
            response(body=page, headers=[("Set-Cookie", f"{PRE_SESSION}=pre-cookie; Path=/; Secure")]),
            response(303, headers=[("Location", "/recipes"),
                                   ("Set-Cookie", f"{SESSION}=admin-session; Path=/; Secure"),
                                   ("Set-Cookie", f"{PRE_SESSION}=; Max-Age=0; Path=/")]),
        )
        with patch("sys.argv", ["load_client.py", "ignored-argument"]):
            self.client.login("ci-admin", "new_password")
        method, path, fields, headers = self.sent(connections[1])
        self.assertEqual((method, path), ("POST", "/login"))
        self.assertEqual(fields["login"], ["ci-admin"])
        self.assertTrue(fields["password"] == [self.secret])
        self.assertEqual(fields["csrf_pre"], ["pre&token"])
        self.assertEqual(headers["Cookie"], f"{PRE_SESSION}=pre-cookie")
        self.assertEqual(headers["Origin"], "https://localhost:8443")
        self.assertNotIn("HX-Request", headers)
        self.assertEqual(self.client.cookies, {SESSION: "admin-session"})
        self.assertEqual(len(self.factory.call_args_list), 2)
        self.context.assert_called_once_with(cafile=str(self.state / "root.crt"))
        for connection in connections:
            connection.close.assert_called_once()

    def test_login_requires_expected_redirect_and_a_new_session_cookie(self):
        for status, headers in (
            (200, []), (303, [("Location", "/me/password")]),
            (303, [("Location", "/recipes")]),
        ):
            with self.subTest(status=status):
                self.answers(response(body=form("/login", name="csrf_pre")),
                             response(status, headers=headers))
                with self.assertRaises(self.module.HttpCheckError):
                    self.client.login("ci-admin", "new_password")

    def test_form_uses_its_action_and_draft_posts_use_updated_row_version(self):
        action = "/recipes/R-1/versions/1/parameters"
        page = form("/lang", "wrong", 99) + form(action, "right&amp;token", 7)
        connections = self.answers(
            response(body=page), response(headers=[("HX-Trigger", '{"rowVersion":8}')]),
            response(headers=[("HX-Trigger", '{"rowVersion":9}')]),
        )
        self.client.form(action + "/new?unit=U1", action)
        self.client.post(action, {"tag": "P1"}, draft=True)
        self.client.post(action, {"tag": "P2"}, draft=True)
        for connection, version in zip(connections[1:], ("7", "8")):
            method, path, fields, headers = self.sent(connection)
            self.assertEqual((method, path), ("POST", action))
            self.assertEqual(fields["csrf_token"], ["right&token"])
            self.assertEqual(fields["row_version"], [version])
            self.assertEqual(headers["HX-Request"], "true")
            self.assertEqual(headers["Sec-Fetch-Site"], "same-origin")
            self.assertEqual(headers["Content-Type"], "application/x-www-form-urlencoded")
        self.assertEqual(self.client.row_version, 9)

    def test_bad_form_or_row_version_stops_before_post_without_body_in_error(self):
        for page in (form("/other"), form("/users") + form("/users"),
                     form("/users", row_version="bad-value")):
            with self.subTest():
                self.answers(response(body=page))
                with self.assertRaises(self.module.HttpCheckError) as raised:
                    self.client.form("/users/new", "/users")
                self.assertIn("HTTP 200", str(raised.exception))
                self.assertNotIn(page, str(raised.exception))

    def test_draft_requires_initial_and_returned_row_version(self):
        self.client.csrf_token = "token"
        with self.assertRaises(ValueError):
            self.client.post("/draft", {}, draft=True)
        self.factory.assert_not_called()
        for trigger in (None, "not-json", "[]", '{"rowVersion":true}',
                        '{"rowVersion":0}', '{"rowVersion":"8"}', '{}'):
            with self.subTest(trigger=trigger):
                self.client.row_version = 7
                headers = [] if trigger is None else [("HX-Trigger", trigger)]
                self.answers(response(headers=headers))
                with self.assertRaises(self.module.HttpCheckError):
                    self.client.post("/draft", {}, draft=True)

    def user_answers(self, changed_cookie="new-session", refresh="true"):
        return (
            response(body=form("/users", "admin-csrf")),
            response(body='<code>decoy</code><code class="data" id="one-time-password">once&amp;value-12345</code>'),
            response(body=form("/login", "pre-user", name="csrf_pre")),
            response(303, headers=[("Location", "/me/password"),
                                   ("Set-Cookie", f"{SESSION}=first-session; Path=/")]),
            response(body=form("/me/password", "user-csrf")),
            response(headers=[("HX-Refresh", refresh),
                              ("Set-Cookie", f"{SESSION}={changed_cookie}; Path=/")]),
        )

    def test_create_user_replaces_one_time_password_and_keeps_sessions_separate(self):
        self.client.cookies[SESSION] = "admin-session"
        (self.state / "approver_password").write_text(self.secret + "\n", encoding="ascii")
        connections = self.answers(*self.user_answers())
        output = self.enterContext(contextlib.redirect_stdout(io.StringIO()))
        errors = self.enterContext(contextlib.redirect_stderr(io.StringIO()))
        user = self.client.create_user("load-approver", "approver_password")
        self.assertEqual(self.sent(connections[0])[:2], ("GET", "/users/new"))
        _, path, fields, headers = self.sent(connections[1])
        self.assertEqual(path, "/users")
        self.assertEqual(fields, {"login": ["load-approver"], "role": ["process_engineer"],
                                  "csrf_token": ["admin-csrf"]})
        self.assertNotIn("HX-Request", headers)
        self.assertNotIn("admin-session", self.sent(connections[2])[3].get("Cookie", ""))
        self.assertTrue(self.sent(connections[3])[2]["password"] == ["once&value-12345"])
        _, path, fields, headers = self.sent(connections[5])
        self.assertEqual(path, "/me/password")
        self.assertTrue(fields["current_password"] == ["once&value-12345"])
        self.assertTrue(fields["password"] == [self.secret])
        self.assertEqual(fields["csrf_token"], ["user-csrf"])
        self.assertEqual(headers["HX-Request"], "true")
        self.assertEqual(user.cookies[SESSION], "new-session")
        self.assertEqual(self.client.cookies[SESSION], "admin-session")
        self.assertIsNone(user.csrf_token)
        self.assertEqual(output.getvalue() + errors.getvalue(), "")
        self.assertEqual(sorted(item.name for item in self.state.iterdir()),
                         ["approver_password", "new_password"])

    def test_password_change_requires_rotation_and_refresh(self):
        (self.state / "approver_password").write_text(self.secret, encoding="ascii")
        for cookie, refresh in (("first-session", "true"), ("new-session", "false")):
            with self.subTest():
                self.answers(*self.user_answers(cookie, refresh))
                with self.assertRaises(self.module.HttpCheckError):
                    self.client.create_user("load-approver", "approver_password")

    def test_missing_or_duplicate_one_time_password_fails_safely(self):
        element = '<code id="one-time-password">hidden-value-12345</code>'
        for body in ("", element + element):
            with self.subTest():
                self.answers(response(body=form("/users")), response(body=body))
                with self.assertRaises(self.module.HttpCheckError) as raised:
                    self.client.create_user("load-approver", "new_password")
                self.assertNotIn("hidden-value", str(raised.exception))

    def test_http_failure_exposes_status_and_code_only(self):
        self.client.csrf_token = "token"
        for body, code in ((self.secret + " E-IDN-006", "E-IDN-006"),
                           (self.secret + " E-idn-006 E-IDN-0067", "none")):
            with self.subTest(code=code):
                connections = self.answers(response(422, body))
                with self.assertRaises(self.module.HttpCheckError) as raised:
                    self.client.post("/users", {"login": "load-approver"})
                self.assertEqual(str(raised.exception), f"the request failed: HTTP 422, code {code}")
                self.assertNotIn(self.secret, str(raised.exception))
                connections[0].close.assert_called_once()

    def test_redirect_returns_without_another_request(self):
        self.answers(response(303, headers=[("Location", "/recipes")]))
        status, headers, _ = self.client.request("/recipes", {})
        self.assertEqual((status, headers["location"]), (303, "/recipes"))
        self.assertEqual(len(self.factory.call_args_list), 1)

    def test_transport_failure_hides_exception_text_and_closes_connection(self):
        connection = self.answers(response())[0]
        connection.getresponse.side_effect = OSError(self.secret)
        with self.assertRaisesRegex(ValueError, "^the HTTP connection failed$") as raised:
            self.client.request("/recipes")
        self.assertTrue(raised.exception.__suppress_context__)
        connection.close.assert_called_once()

    def test_state_file_is_required_even_if_arguments_exist(self):
        with patch("sys.argv", ["load_client.py", "unused-value"]):
            with self.assertRaisesRegex(ValueError, "^cannot read the password state file$"):
                self.client.login("ci-admin", "missing")
        self.factory.assert_not_called()


if __name__ == "__main__":
    unittest.main()
