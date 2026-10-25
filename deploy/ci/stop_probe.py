"""Hold a request body until the server starts its graceful shutdown."""

import http.client
import ipaddress
import os
import pathlib
import re
import socket
import sys
import time


class HttpCheckError(ValueError):
    def __init__(self, message, status, body):
        match = re.search(r"\bE-[A-Z]+-[0-9]{3}\b", body)
        code = match.group() if match else "none"
        super().__init__(f"{message}: HTTP {status}, code {code}")


class ResponseSocket:
    """Keep buffered response bytes after the interim status line."""

    def __init__(self, stream):
        self.stream = stream

    def makefile(self, mode):
        return self.stream


def wait_for_shutdown(address):
    deadline = time.monotonic() + 5
    while time.monotonic() < deadline:
        probe = http.client.HTTPConnection(address, 9000, timeout=1)
        try:
            probe.request("GET", "/readyz")
            if probe.getresponse().status == 503:
                return
        finally:
            probe.close()
        time.sleep(0.1)
    raise ValueError("the server did not enter shutdown")


def run():
    os.umask(0o077)
    address = str(ipaddress.IPv4Address(sys.argv[1]))
    state = pathlib.Path(os.environ["CI_STATE"])
    body = b"login=ci-stop&password=invalid&csrf_pre=invalid"
    headers = (
        "POST /login HTTP/1.1\r\nHost: localhost:8443\r\n"
        "Origin: https://localhost:8443\r\nConnection: close\r\n"
        "Content-Type: application/x-www-form-urlencoded\r\n"
        f"Content-Length: {len(body)}\r\nExpect: 100-continue\r\n\r\n"
    )
    with socket.create_connection((address, 8080), timeout=8) as connection:
        connection.sendall(headers.encode("ascii"))
        with connection.makefile("rb") as stream:
            if stream.readline() != b"HTTP/1.1 100 Continue\r\n":
                raise ValueError("the server did not start reading the request body")
            if stream.readline() != b"\r\n":
                raise ValueError("the interim response has unexpected headers")
            (state / "inflight-ready").touch()
            wait_for_shutdown(address)
            connection.sendall(body)
            # The invalid pre-session token must reach the normal CSRF rejection.
            with http.client.HTTPResponse(ResponseSocket(stream)) as response:
                response.begin()
                text = response.read().decode("utf-8", errors="replace")
                if response.status != 403:
                    raise HttpCheckError("the server did not complete the request during its drain", response.status, text)
    print("CI request completed during shutdown.")


if __name__ == "__main__":
    try:
        run()
    except HttpCheckError as error:
        print(str(error), file=sys.stderr)
        sys.exit(1)
    except Exception:
        print("CI shutdown request failed.", file=sys.stderr)
        sys.exit(1)
