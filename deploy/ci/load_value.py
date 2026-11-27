"""Build, send, and check the value bodies for the load job."""

import argparse
import concurrent.futures
import http.client
import importlib.util
import io
import json
import math
import os
import pathlib
import queue
import re
import socket
import ssl
import subprocess
import sys
import threading
import time
import urllib.parse

# Isolated Python omits this directory from the import path.
_spec = importlib.util.spec_from_file_location(
    "load_value_check", pathlib.Path(__file__).with_name("load_value_check.py"))
_checker = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(_checker)

REFERENCES = _checker.REFERENCES
KEY = _checker.KEY
missed_barrier = _checker.missed_barrier
missing_barrier_anon = _checker.missing_barrier_anon
run_checker = _checker.run_checker


ARRAY_LEVELS = 32
WIDE_LEVELS = 12
NARROW_LEVELS = ARRAY_LEVELS - WIDE_LEVELS
TYPE_TEXT = "ARRAY[10..11] OF " * WIDE_LEVELS + "ARRAY[10..10] OF " * NARROW_LEVELS + "LDT"
PADDING = 13_710_000
ALIAS_PREFIX = "LDATE_AND_TIME#"
MIN_TEXT = ALIAS_PREFIX + "1970-01-01-00:00:00.000000001"
MAX_TEXT = ALIAS_PREFIX + "2262-04-11-23:47:16.854775807"


def build_body(round_number: int, row_version: int, unit_path: str, ordinal: int) -> bytes:
    """Build the form of the selected round with the supplied draft fields."""
    if round_number not in (1, 2):
        raise ValueError("Round must be 1 or 2.")
    description = "x" * PADDING if round_number == 1 else "The largest edit form"
    pairs = [
        ("row_version", str(row_version)),
        ("unit_path", unit_path),
        ("tag", "BIG"),
        ("name_en", "Big"),
        ("description_en", description),
        ("s7_address", ""),
        ("ordinal", str(ordinal)),
        # The Rust fixture sends both type fields in this order.
        ("type", TYPE_TEXT),
        ("value_type", TYPE_TEXT),
    ]
    for leaf in range(1 << WIDE_LEVELS):
        bits = f"{leaf:0{WIDE_LEVELS}b}"
        path = "".join("[11]" if bit == "1" else "[10]" for bit in bits)
        path += "[10]" * NARROW_LEVELS
        value = f"{ALIAS_PREFIX}2262-04-11-23:47:16.854775{leaf % 800:03d}"
        pairs.extend((
            (f"value{path}", value),
            (f"limits[{leaf}].leaf_path", path),
            (f"limits[{leaf}].min", MIN_TEXT),
            (f"limits[{leaf}].max", MAX_TEXT),
        ))
    return "&".join(
        f"{urllib.parse.quote(key, safe='')}={urllib.parse.quote(value, safe='')}"
        for key, value in pairs
    ).encode("ascii")


BARRIER_TIMEOUT = 8
RECORD_WINDOW_S = 1.0
# Match http.request_timeout in reliability.md section 7.11.
REQUEST_TIMEOUT = 30
FIELD = re.compile(rb'<div class="field-error" id="([a-z0-9_.\[\]-]+)-error">')
CODE = re.compile(rb'<span class="field-error-code">(E-[A-Z]{3}-[0-9]{3})</span>')


def remaining(deadline):
    seconds = deadline - time.monotonic()
    if seconds <= 0:
        raise TimeoutError
    return seconds


def send_body(sock, body, deadline):
    """Send at most 1 MiB per write and check the deadline after partial writes."""
    while body:
        sock.settimeout(remaining(deadline))
        sent = sock.send(body[:1024 * 1024])
        if sent == 0:
            raise OSError
        body = body[sent:]
    remaining(deadline)


class AnswerSocket(io.RawIOBase):
    """Use the same deadline for all reads, including HTTP headers and chunk framing."""

    def __init__(self, sock, deadline):
        super().__init__()
        self.sock = sock
        self.deadline = deadline

    def makefile(self, _mode):
        return io.BufferedReader(self)

    def readable(self):
        return True

    def readinto(self, buffer):
        self.sock.settimeout(remaining(self.deadline))
        return self.sock.recv_into(buffer)


def read_fields(response):
    """Drain the answer and keep only the first field line."""
    tail = b""
    field = None
    result = (None, None)
    scan = True
    while chunk := response.read(8192):
        if not scan:
            continue
        window = tail + chunk
        if field is None and (match := FIELD.search(window)):
            field = match[1].decode("ascii")
            window = window[match.end():]
        if field is not None:
            end = window.find(b"</div>")
            match = CODE.search(window, 0, len(window) if end == -1 else end)
            if match:
                result = (match[1].decode("ascii"), field)
            scan = match is None and end == -1
        tail = window[-4096:] if scan else b""
    if response.length:
        raise http.client.IncompleteRead(b"")
    return result


class DraftBody:
    def __init__(self, round_number, draft, body):
        self.bytes = body
        self.reference = draft["reference"]
        self.path = f"/recipes/{self.reference}/versions/1/parameters/{draft['key']}/definition"
        self.answer = {
            "round": round_number, "reference": self.reference, "key": draft["key"],
            "body_bytes": len(body), "status": None, "location": None, "code": None,
            "field": None, "last_byte_s": None, "answer_s": None, "error": None,
        }

    def barrier_error(self):
        return missed_barrier(self.answer["round"], self.reference)


def draft_bodies(round_number, drafts, references):
    bodies = {}
    for reference in references:
        draft = drafts[reference]
        if not KEY.fullmatch(draft["key"]):
            raise ValueError
        fields = draft["row_version"], draft["unit_path"], draft["ordinal"]
        if fields not in bodies:
            bodies[fields] = build_body(round_number, *fields)
        yield DraftBody(round_number, draft, bodies[fields])


class ValueSender:
    def __init__(self, args, state):
        self.origin = args.url
        self.host = self.origin.hostname
        self.port = self.origin.port or 443
        self.context = ssl.create_default_context(cafile=str(args.cacert))
        self.cookie = state["cookie"]
        self.token = state["csrf_token"]
        if not isinstance(self.cookie, str) or not isinstance(self.token, str):
            raise ValueError
        self.secrets = [self.token]
        self.secrets.extend(part.partition("=")[2].strip() for part in self.cookie.split(";"))
        endpoints = socket.getaddrinfo(self.host, self.port, type=socket.SOCK_STREAM)
        self.addresses = [address[:2] for _, _, _, _, address in endpoints]

    def connect(self, deadline):
        for address in self.addresses:
            try:
                return socket.create_connection(address, timeout=remaining(deadline))
            except OSError:
                continue
        raise OSError

    def send_headers(self, sock, draft, deadline):
        connection = http.client.HTTPConnection(self.host, self.port)
        connection.sock = sock
        connection.putrequest("POST", draft.path)
        connection.putheader("Content-Type", "application/x-www-form-urlencoded")
        connection.putheader("Content-Length", str(len(draft.bytes)))
        connection.putheader("X-CSRF-Token", self.token)
        connection.putheader("Origin", f"https://{self.origin.netloc}")
        connection.putheader("Cookie", self.cookie)
        connection.putheader("Connection", "close")
        sock.settimeout(remaining(deadline))
        connection.endheaders()

    def receive(self, sock, deadline, answer):
        with http.client.HTTPResponse(AnswerSocket(sock, deadline), method="POST") as response:
            response.begin()
            answer["status"] = response.status
            location = response.getheader("Location")
            if location is not None and self.safe_location(location):
                answer["location"] = location
            answer["code"], answer["field"] = read_fields(response)

    def safe_location(self, location):
        text = urllib.parse.unquote(location)
        return (not any(secret and secret in text for secret in self.secrets)
                and not any(char.isspace() or char in "<>" for char in text))

    def send(self, draft, start, barrier=None):
        answer = draft.answer
        deadline = start + REQUEST_TIMEOUT if barrier is None else barrier.deadline
        at_barrier = False
        try:
            with self.connect(deadline) as raw:
                raw.settimeout(remaining(deadline))
                with self.context.wrap_socket(raw, server_hostname=self.host) as sock:
                    self.send_headers(sock, draft, deadline)
                    body = memoryview(draft.bytes)
                    if barrier is not None:
                        send_body(sock, body[:-1], deadline)
                        at_barrier = True
                        deadline = barrier.arrive(draft.reference)
                        if deadline is None:
                            answer["error"] = barrier.failure
                            return
                        body = body[-1:]
                    send_body(sock, body, deadline)
                    answer["last_byte_s"] = time.monotonic() - start
                    self.receive(sock, deadline, answer)
                    remaining(deadline)
                    answer["answer_s"] = time.monotonic() - start
        except (OSError, http.client.HTTPException, ValueError):
            if barrier is not None and not at_barrier:
                answer["error"] = draft.barrier_error()
            else:
                label = f"round {answer['round']}: request {draft.reference}"
                answer["error"] = f"{label} failed during answer collection"


class RoundBarrier:
    def __init__(self, deadline):
        self.deadline = deadline
        self.ready = queue.Queue()
        self.release = threading.Event()
        self.answer_deadline = None
        self.failure = None

    def arrive(self, reference):
        self.ready.put(reference)
        self.release.wait()
        return self.answer_deadline

    def missing(self, drafts):
        pending = {draft.reference: draft for draft in drafts}
        try:
            while pending:
                del pending[self.ready.get(timeout=remaining(self.deadline))]
        except (TimeoutError, queue.Empty):
            return next(iter(pending.values()))
        return None


def read_anon(cgroup, timeout):
    try:
        data = subprocess.run(
            ["sudo", "-n", "cat", str(cgroup / "memory.stat")], check=True,
            stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, timeout=timeout,
        ).stdout
    except (OSError, subprocess.SubprocessError):
        return None
    match = re.search(rb"^anon ([0-9]+)$", data, re.M)
    return int(match[1]) if match else None


def record_anon_max(cgroup, largest, deadline):
    try:
        while True:
            time.sleep(min(0.05, remaining(deadline)))
            anon = read_anon(cgroup, remaining(deadline))
            remaining(deadline)
            if anon is not None:
                largest = max(largest, anon)
    except TimeoutError:
        return largest


def overlap_anon(args, drafts, deadline):
    threshold = args.reference_anon + sum(len(draft.bytes) for draft in drafts)
    last_anon = None
    try:
        while True:
            anon = read_anon(args.cgroup, remaining(deadline))
            read_at = time.monotonic()
            if read_at >= deadline:
                break
            if anon is not None:
                if anon >= threshold:
                    anon_max = None
                    if args.round == 2:
                        window_end = min(read_at + RECORD_WINDOW_S, deadline)
                        anon_max = record_anon_max(args.cgroup, anon, window_end)
                    return anon, anon_max, None
                last_anon = anon
            time.sleep(min(0.05, remaining(deadline)))
    except TimeoutError:
        pass
    if last_anon is None:
        return None, None, missing_barrier_anon(args.round)
    return last_anon, None, f"round {args.round}: the bodies did not overlap"


def mem_available():
    data = pathlib.Path("/proc/meminfo").read_bytes()
    match = re.search(rb"^MemAvailable:\s+([0-9]+) kB$", data, re.M)
    if not match:
        raise ValueError
    return int(match[1])


def run_round(args, sender, drafts):
    start = time.monotonic()
    barrier = RoundBarrier(start + args.deadline)
    samples = []
    anon = None
    anon_max = None
    with concurrent.futures.ThreadPoolExecutor(max_workers=len(drafts)) as pool:
        try:
            tasks = [pool.submit(sender.send, draft, start, barrier) for draft in drafts]
            missing = barrier.missing(drafts)
            if missing is not None:
                barrier.failure = missing.barrier_error()
            else:
                samples.append(mem_available())
                anon, anon_max, barrier.failure = overlap_anon(args, drafts, barrier.deadline)
                if barrier.failure is None:
                    barrier.answer_deadline = time.monotonic() + REQUEST_TIMEOUT
        finally:
            barrier.release.set()
        for task in tasks:
            task.result()
    samples.append(mem_available())
    answers = [draft.answer for draft in drafts]
    failures = (answer["error"] for answer in answers if answer["error"])
    failure = barrier.failure or next(failures, None)
    return {
        "round": args.round, "reference_anon": args.reference_anon, "barrier_anon": anon,
        "barrier_anon_max": anon_max,
        "barrier_failure": barrier.failure,
        "mem_available_min_kib": min(samples), "answers": answers,
    }, failure


class UsageError(ValueError):
    pass


class SafeParser(argparse.ArgumentParser):
    def error(self, _message):
        raise UsageError("Invalid value command arguments.")


def arguments(argv):
    parser = SafeParser(description=__doc__)
    commands = parser.add_subparsers(dest="command", required=True)
    single = commands.add_parser("single")
    single.add_argument("--reference", required=True, choices=REFERENCES)
    round_parser = commands.add_parser("round")
    round_parser.add_argument("--cgroup", required=True, type=pathlib.Path)
    round_parser.add_argument("--reference-anon", required=True, type=int)
    round_parser.add_argument("--deadline", type=float, default=BARRIER_TIMEOUT)
    precheck = commands.add_parser("check-precheck")
    check_round = commands.add_parser("check-round")
    check_round.add_argument("--round", required=True, type=int, choices=(1,))
    check = commands.add_parser("check")
    check.add_argument("--job-start", required=True, type=pathlib.Path)
    check.add_argument("--summary", required=True, type=pathlib.Path)
    for command in (precheck, check_round, check):
        command.add_argument("--ci-state", required=True, type=pathlib.Path)
    for command in (single, round_parser):
        command.add_argument("--ci-state", required=True, type=pathlib.Path)
        command.add_argument("--round", required=True, type=int, choices=(1, 2))
        command.add_argument("--url", required=True)
        command.add_argument("--cacert", required=True, type=pathlib.Path)
    args = parser.parse_args(argv)
    if args.command in ("check-precheck", "check-round", "check"):
        return args
    try:
        args.url = urllib.parse.urlsplit(args.url)
        port = args.url.port
    except ValueError:
        parser.error(None)
    if (args.url.scheme != "https" or not args.url.hostname or args.url.username is not None
            or args.url.path not in ("", "/") or args.url.query or args.url.fragment
            or port == 0):
        parser.error(None)
    if args.command == "round" and (
            args.reference_anon < 0 or not math.isfinite(args.deadline) or args.deadline <= 0):
        parser.error(None)
    return args


def run_sender(args):
    state = json.loads((args.ci_state / "load-value-state.json").read_text(encoding="utf-8"))
    drafts = {draft["reference"]: draft for draft in state["drafts"]}
    sender = ValueSender(args, state)
    if args.command == "single":
        draft, = draft_bodies(args.round, drafts, (args.reference,))
        sender.send(draft, time.monotonic())
        report, failure = draft.answer, draft.answer["error"]
        name = f"load-value-precheck-{args.round}.json"
    else:
        batch = list(draft_bodies(args.round, drafts, REFERENCES[:4]))
        report, failure = run_round(args, sender, batch)
        name = f"load-value-answers-{args.round}.json"
    (args.ci_state / name).write_text(json.dumps(report, allow_nan=False) + "\n", encoding="utf-8")
    return failure


def cli(argv=None):
    try:
        args = arguments(argv)
    except UsageError as error:
        print(error, file=sys.stderr)
        return 2
    try:
        if args.command in ("single", "round"):
            failure = run_sender(args)
            failures = [failure] if failure is not None else []
        else:
            failures = run_checker(args)
    except (OSError, ValueError, KeyError, TypeError):
        failures = ["CI value input could not be read or the result could not be written."]
    for failure in failures:
        print(failure, file=sys.stderr)
    return 1 if failures else 0


if __name__ == "__main__":
    os.umask(0o077)
    sys.exit(cli())
