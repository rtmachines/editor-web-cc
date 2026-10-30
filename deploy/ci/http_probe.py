"""Send CI requests without printing credentials or following redirects."""

import http.client
import http.cookies
import html.parser
import json
import os
import pathlib
import re
import ssl
import sys
import urllib.parse


class HttpCheckError(ValueError):
    def __init__(self, message, status, body):
        match = re.search(r"\bE-[A-Z]+-[0-9]{3}\b", body)
        code = match.group() if match else "none"
        super().__init__(f"{message}: HTTP {status}, code {code}")


class TokenParser(html.parser.HTMLParser):
    def __init__(self, action, name):
        super().__init__(convert_charrefs=True)
        self.action = action
        self.name = name
        self.active = False
        self.values = []

    def handle_starttag(self, tag, attributes):
        fields = dict(attributes)
        if tag == "form":
            self.active = fields.get("action") == self.action
        if tag == "input" and self.active and fields.get("name") == self.name:
            self.values.append(fields.get("value", ""))

    def handle_endtag(self, tag):
        if tag == "form":
            self.active = False


def form_token(body, action, name):
    parser = TokenParser(action, name)
    parser.feed(body)
    if len(parser.values) != 1 or not parser.values[0]:
        raise ValueError("expected one CSRF token")
    return parser.values[0]


def check_redirect(status, headers, destination, body=""):
    if status != 303 or headers.get("location") != destination:
        raise HttpCheckError("unexpected redirect", status, body)


class Client:
    def __init__(self, state, connection_factory=None):
        self.state = state
        self.connection_factory = connection_factory or http.client.HTTPSConnection
        self.cookies_path = state / "cookies.json"
        self.cookies = {}
        if self.cookies_path.exists():
            self.cookies = json.loads(self.cookies_path.read_text())
        self.context = ssl.create_default_context(cafile=str(state / "root.crt"))

    def request(self, path, fields=None):
        connection = self.connection_factory("localhost", 8443, timeout=40, context=self.context)
        headers = {"Cookie": "; ".join(f"{key}={value}" for key, value in self.cookies.items())}
        body = None
        if fields is not None:
            headers["Origin"] = "https://localhost:8443"
            headers["Content-Type"] = "application/x-www-form-urlencoded"
            body = urllib.parse.urlencode(fields).encode("ascii")
        try:
            connection.request("GET" if fields is None else "POST", path, body, headers)
            response = connection.getresponse()
            status = response.status
            response_headers = {key.lower(): value for key, value in response.getheaders()}
            for key, value in response.getheaders():
                if key.lower() == "set-cookie":
                    self.store_cookie(value)
            text = response.read().decode("utf-8")
            self.cookies_path.write_text(json.dumps(self.cookies), encoding="utf-8")
            return status, response_headers, text
        finally:
            connection.close()

    def store_cookie(self, value):
        for name, cookie in http.cookies.SimpleCookie(value).items():
            if cookie["max-age"] == "0":
                self.cookies.pop(name, None)
            else:
                self.cookies[name] = cookie.value

    def token(self, path, name):
        status, _, body = self.request(path)
        if status != 200:
            raise HttpCheckError("the form did not return 200", status, body)
        return form_token(body, path, name)

    def secret(self, name):
        return (self.state / name).read_text(encoding="ascii").removesuffix("\n")


def login(client):
    fields = {"login": "ci-admin", "password": client.secret("admin_password")}
    fields["csrf_pre"] = client.token("/login", "csrf_pre")
    status, headers, body = client.request("/login", fields)
    check_redirect(status, headers, "/me/password", body)
    if "__Host-recipe_session" not in client.cookies:
        raise HttpCheckError("the session cookie is missing", status, body)


def password(client):
    fields = {"current_password": client.secret("admin_password"), "password": client.secret("new_password")}
    fields["csrf_token"] = client.token("/me/password", "csrf_token")
    previous = client.cookies.get("__Host-recipe_session")
    status, headers, body = client.request("/me/password", fields)
    check_redirect(status, headers, "/recipes", body)
    current = client.cookies.get("__Host-recipe_session")
    if not current or current == previous:
        raise HttpCheckError("the command did not rotate the session cookie", status, body)


def relogin(client):
    """Log in after the password change. The session cookie is new and the answer is the recipe list."""
    fields = {"login": "ci-admin", "password": client.secret("new_password")}
    fields["csrf_pre"] = client.token("/login", "csrf_pre")
    status, headers, body = client.request("/login", fields)
    check_redirect(status, headers, "/recipes", body)
    if "__Host-recipe_session" not in client.cookies:
        raise HttpCheckError("the session cookie is missing", status, body)


def health(client):
    for path in ("/healthz", "/readyz?x=1", "/HEALTHZ", "/healthz/", "//healthz", "/readyz%2F"):
        status, headers, body = client.request(path)
        if status != 404:
            raise HttpCheckError("a public health path did not return 404", status, body)
        if "x-request-id" in headers:
            raise HttpCheckError("a public health path returned X-Request-Id", status, body)
        if body:
            raise HttpCheckError("a public health path returned a nonempty body", status, body)


EXPORT_PATH = "/export/v1/recipes/CI-UPGRADE/versions/1"


def export(client, name):
    """Write the content hash of the fixture export to a file of the state directory."""
    if not re.fullmatch(r"[a-z][a-z0-9.-]{0,63}", name):
        raise ValueError("the export file name is invalid")
    status, _, body = client.request(EXPORT_PATH)
    if status != 200:
        raise HttpCheckError("the export did not return 200", status, body)
    document = json.loads(body)
    digest = document.get("content_sha256")
    if document.get("loadable") is not True or not isinstance(digest, str) or not re.fullmatch(r"[0-9a-f]{64}", digest):
        raise HttpCheckError("the export has no loadable content hash", status, "")
    (client.state / name).write_text(digest + "\n", encoding="ascii")


def redirect():
    connection = http.client.HTTPConnection(os.environ["PROXY_ADDRESS"], 80, timeout=5)
    try:
        connection.request("GET", "/ci-redirect?probe=1", headers={"Host": "localhost"})
        response = connection.getresponse()
        if response.status != 308 or response.getheader("Location") != "https://localhost/ci-redirect?probe=1":
            raise HttpCheckError("HTTP did not redirect to the same HTTPS path", response.status, response.read().decode("utf-8", errors="replace"))
        if response.getheader("Strict-Transport-Security") is not None:
            raise HttpCheckError("HTTP sent HSTS", response.status, response.read().decode("utf-8", errors="replace"))
    finally:
        connection.close()


def main():
    os.umask(0o077)
    action = sys.argv[1]
    if action == "redirect":
        redirect()
    elif action == "export":
        export(Client(pathlib.Path(os.environ["CI_STATE"])), sys.argv[2])
    else:
        client = Client(pathlib.Path(os.environ["CI_STATE"]))
        {"login": login, "relogin": relogin, "password": password, "health": health}[action](client)
    print("CI HTTP check passed.")


if __name__ == "__main__":
    try:
        main()
    except HttpCheckError as error:
        print(str(error), file=sys.stderr)
        sys.exit(1)
    except Exception:
        # Response bodies, cookie values, and exceptions can contain secrets.
        print("CI HTTP check failed.", file=sys.stderr)
        sys.exit(1)
