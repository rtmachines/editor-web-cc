"""Measure the three load requests through Caddy and report their elapsed times."""

import argparse
import importlib.util
import json
import math
import os
import pathlib
import sys
import time
import urllib.parse


# Isolated Python omits this directory from the import path.
_spec = importlib.util.spec_from_file_location(
    "_timing_load_client", pathlib.Path(__file__).with_name("load_client.py"))
_client = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(_client)
Client = _client.Client
HttpCheckError = _client.HttpCheckError
# The Load row of reliability.md section 6 and ADR-0128 (r) set this bound.
TIME_LIMIT = 10


class TimingCheckError(ValueError):
    """Report a timing fault without response data or credentials."""


class TimedResponse:
    """Record the end of the body read before decoding or connection cleanup."""

    def __init__(self, response, finished):
        self.response = response
        self.finished = finished
        self.status = response.status

    def getheaders(self):
        return self.response.getheaders()

    def read(self):
        body = self.response.read()
        self.finished.append(time.monotonic())
        return body


class TimedConnection:
    """Keep the shared client's transport behavior and timestamp its complete response."""

    def __init__(self, connection, finished):
        self.connection = connection
        self.finished = finished

    def request(self, *args, **kwargs):
        return self.connection.request(*args, **kwargs)

    def getresponse(self):
        return TimedResponse(self.connection.getresponse(), self.finished)

    def close(self):
        self.connection.close()


def measure_request(client, name, path, fields=None, *, hx=False, redirect_to=None):
    """Measure one request from before the connection to the last response byte."""
    factory = client.connection_factory
    finished = []

    def connect(*args, **kwargs):
        return TimedConnection(factory(*args, **kwargs), finished)

    client.connection_factory = connect
    try:
        started = time.monotonic()
        status, headers, body = client.request(path, fields, hx=hx)
    finally:
        client.connection_factory = factory
    # CreateVersion returns a redirect even for an HTMX post. Never follow it.
    if redirect_to is not None:
        if status != 303 or headers.get("location") != redirect_to:
            raise HttpCheckError(f"{name} returned an unexpected redirect", status, body)
    elif status != 200:
        raise HttpCheckError(f"{name} did not return 200", status, body)
    return finished[0] - started


def check_bounds(timings):
    failures = []
    for name, seconds in timings.items():
        if not math.isfinite(seconds) or seconds < 0 or seconds >= TIME_LIMIT:
            failures.append(f"{name} must finish in less than 10 s")
    if failures:
        raise TimingCheckError("\n".join(failures))


def run_timings(admin, approver, reference, version):
    """Approve as the second user, copy as the author, and read the approved version."""
    versions = "/recipes/" + urllib.parse.quote(reference, safe="") + "/versions"
    page = f"{versions}/{version}"
    approve = page + "/approve"
    approver.form(approve)
    if approver.row_version is None:
        raise HttpCheckError("ApproveVersion requires row_version", 200, "")
    fields = {"csrf_token": approver.csrf_token, "row_version": approver.row_version}
    timings = {"ApproveVersion": measure_request(approver, "ApproveVersion", approve, fields, hx=True)}
    admin.form(versions + "/new", versions)
    fields = {"csrf_token": admin.csrf_token, "based_on": str(version),
              "change_note": "Copy the approved version for the load check."}
    timings["CreateVersion"] = measure_request(
        admin, "CreateVersion", versions, fields, hx=True, redirect_to=f"{versions}/{version + 1}")
    timings["GetVersion"] = measure_request(admin, "GetVersion", page)
    return timings


def write_report(seed_seconds, timings, output):
    """Write numeric results and append the seed time and request times to the job summary."""
    result = {"seed_seconds": seed_seconds, "timings_seconds": timings}
    output.write_text(json.dumps(result, indent=2, allow_nan=False) + "\n", encoding="utf-8")
    lines = [f"Seed: {seed_seconds:.3f} s"]
    lines.extend(f"{name}: {seconds:.3f} s" for name, seconds in timings.items())
    summary = "\n".join(lines) + "\n"
    summary_path = os.environ.get("GITHUB_STEP_SUMMARY")
    if summary_path:
        with pathlib.Path(summary_path).open("a", encoding="utf-8") as stream:
            stream.write(summary)
    print(summary, end="")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--reference", required=True)
    parser.add_argument("--version", type=int, required=True)
    parser.add_argument("--approver", required=True)
    parser.add_argument("--seed-seconds", type=float, required=True)
    parser.add_argument("--output", type=pathlib.Path, required=True)
    args = parser.parse_args()
    if args.version < 1 or not math.isfinite(args.seed_seconds) or args.seed_seconds < 0:
        raise TimingCheckError("The version or seed time is invalid.")
    state = os.environ.get("CI_STATE")
    if not state:
        raise TimingCheckError("CI_STATE is missing.")
    admin, approver = Client(state), Client(state)
    admin.login("ci-admin", "new_password")
    approver.login(args.approver, "approver_password")
    timings = run_timings(admin, approver, args.reference, args.version)
    write_report(args.seed_seconds, timings, args.output)
    check_bounds(timings)
    return 0


if __name__ == "__main__":
    try:
        sys.exit(main())
    except (HttpCheckError, TimingCheckError) as error:
        print(f"CI load timing failed: {error}", file=sys.stderr)
        sys.exit(1)
    except (OSError, ValueError):
        print("CI load timing could not complete the request or report.", file=sys.stderr)
        sys.exit(1)
