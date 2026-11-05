"""Send load requests through Caddy without exposing passwords or following redirects."""

import http.client
import http.cookies
import html.parser
import importlib.util
import json
import pathlib
import ssl
import urllib.parse


# Isolated Python omits this directory from the import path.
_spec = importlib.util.spec_from_file_location(
    "_load_http_probe", pathlib.Path(__file__).with_name("http_probe.py"))
_probe = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(_probe)
HttpCheckError = _probe.HttpCheckError
SESSION = "__Host-recipe_session"
ORIGIN = "https://localhost:8443"
# Match http.request_timeout in reliability.md section 7.11.
REQUEST_TIMEOUT = 30


class PasswordParser(html.parser.HTMLParser):
    """Read only the password element of the CreateUser page."""

    def __init__(self):
        super().__init__(convert_charrefs=True)
        self.values = []
        self.active = False

    def handle_starttag(self, tag, attributes):
        if tag == "code" and dict(attributes).get("id") == "one-time-password":
            self.values.append("")
            self.active = True

    def handle_data(self, data):
        if self.active:
            self.values[-1] += data

    def handle_endtag(self, tag):
        if tag == "code":
            self.active = False


def one_time_password(body, status):
    parser = PasswordParser()
    parser.feed(body)
    if len(parser.values) != 1 or not parser.values[0]:
        raise HttpCheckError("expected one password element", status, "")
    return parser.values[0]


def positive_version(value, status):
    if type(value) is not int or value < 1:
        raise HttpCheckError("invalid row_version", status, "")
    return value


class Client:
    """Keep one user's cookies and the tokens of the selected form in memory."""

    def __init__(self, state, connection_factory=None):
        self.state = pathlib.Path(state)
        self.connection_factory = connection_factory or http.client.HTTPSConnection
        self.context = ssl.create_default_context(cafile=str(self.state / "root.crt"))
        self.cookies = {}
        self.csrf_token = None
        self.row_version = None

    def request(self, path, fields=None, *, hx=False):
        """Return status, headers, and text after reading the last byte on a new connection."""
        headers = {"Accept": "text/html", "Sec-Fetch-Site": "same-origin"}
        headers["Cookie"] = "; ".join(f"{key}={value}" for key, value in self.cookies.items())
        body = None
        if fields is not None:
            headers["Origin"] = ORIGIN
            headers["Content-Type"] = "application/x-www-form-urlencoded"
            body = urllib.parse.urlencode(fields, doseq=True).encode("ascii")
        if hx:
            headers["HX-Request"] = "true"
        connection = None
        try:
            connection = self.connection_factory(
                "localhost", 8443, timeout=REQUEST_TIMEOUT, context=self.context)
            connection.request("GET" if fields is None else "POST", path, body, headers)
            response = connection.getresponse()
            status = response.status
            pairs = response.getheaders()
            response_headers = {key.lower(): value for key, value in pairs}
            text = response.read().decode("utf-8", errors="replace")
            for key, value in pairs:
                if key.lower() == "set-cookie":
                    self.store_cookie(value)
            if not (200 <= status < 300 or status == 303):
                raise HttpCheckError("the request failed", status, text)
            return status, response_headers, text
        except HttpCheckError:
            raise
        except Exception:
            raise ValueError("the HTTP connection failed") from None
        finally:
            if connection is not None:
                connection.close()

    def store_cookie(self, value):
        for name, cookie in http.cookies.SimpleCookie(value).items():
            if cookie["max-age"] == "0":
                self.cookies.pop(name, None)
            else:
                self.cookies[name] = cookie.value

    def secret(self, name):
        """Read a password from the state directory without consulting process arguments."""
        try:
            if pathlib.Path(name).name != name:
                raise ValueError("invalid file name")
            return (self.state / name).read_text(encoding="ascii").removesuffix("\n")
        except (OSError, ValueError):
            raise ValueError("cannot read the password state file") from None

    def get(self, path):
        status, _, body = self.request(path)
        if status != 200:
            raise HttpCheckError("the page did not return 200", status, body)
        return body

    def form(self, path, action=None):
        """Select the form by its action and keep its CSRF token and optional row_version."""
        self.csrf_token, self.row_version = None, None
        body = self.get(path)
        action = action or urllib.parse.urlsplit(path).path
        try:
            token = _probe.form_token(body, action, "csrf_token")
            parser = _probe.TokenParser(action, "row_version")
            parser.feed(body)
            if parser.values:
                if len(parser.values) != 1 or not parser.values[0].isascii() or not parser.values[0].isdecimal():
                    raise ValueError("invalid row_version")
                self.row_version = positive_version(int(parser.values[0]), 200)
        except ValueError:
            raise HttpCheckError("invalid form fields", 200, "") from None
        self.csrf_token = token
        return body

    def post(self, path, fields, *, hx=True, draft=False):
        """Post the selected form and retain the row_version of a successful draft command."""
        if self.csrf_token is None or (draft and self.row_version is None):
            raise ValueError("select a form before its command")
        payload = dict(fields, csrf_token=self.csrf_token)
        if draft:
            payload["row_version"] = self.row_version
        status, headers, body = self.request(path, payload, hx=hx)
        if draft or "hx-trigger" in headers:
            self.update_version(status, headers, body)
        return status, headers, body

    def update_version(self, status, headers, body):
        if status != 200:
            raise HttpCheckError("the draft command did not return 200", status, body)
        try:
            trigger = json.loads(headers.get("hx-trigger", "{}"))
            version = trigger.get("rowVersion")
            self.row_version = positive_version(version, status)
        except (ValueError, AttributeError):
            raise HttpCheckError("invalid row_version in HX-Trigger", status, "") from None

    def login(self, login, password_file="new_password"):
        """Log in with a password from a state file after the initial password change."""
        self._login_password(login, self.secret(password_file), "/recipes")

    def _login_password(self, login, password, destination):
        page = self.get("/login")
        try:
            pre = _probe.form_token(page, "/login", "csrf_pre")
        except ValueError:
            raise HttpCheckError("invalid login form", 200, "") from None
        previous = self.cookies.get(SESSION)
        status, headers, body = self.request(
            "/login", {"login": login, "password": password, "csrf_pre": pre})
        _probe.check_redirect(status, headers, destination, body)
        if not self.cookies.get(SESSION) or self.cookies.get(SESSION) == previous:
            raise HttpCheckError("the login did not issue a session cookie", status, body)
        self.csrf_token, self.row_version = None, None

    def create_user(self, login, password_file="approver_password"):
        """Create a ProcessEngineer and return its separate session after the password change."""
        replacement = self.secret(password_file)
        self.form("/users/new", "/users")
        status, _, body = self.post("/users", {"login": login, "role": "process_engineer"}, hx=False)
        if status != 200:
            raise HttpCheckError("CreateUser did not return 200", status, body)
        password = one_time_password(body, status)
        user = Client(self.state, connection_factory=self.connection_factory)
        user._login_password(login, password, "/me/password")
        user._change_password(password, replacement)
        return user

    def _change_password(self, current, replacement):
        self.form("/me/password")
        previous = self.cookies.get(SESSION)
        status, headers, body = self.post(
            "/me/password", {"current_password": current, "password": replacement})
        if status != 200 or headers.get("hx-refresh") != "true":
            raise HttpCheckError("the password change did not refresh the page", status, body)
        if not self.cookies.get(SESSION) or self.cookies.get(SESSION) == previous:
            raise HttpCheckError("the password change did not rotate the session cookie", status, body)
        self.csrf_token, self.row_version = None, None
