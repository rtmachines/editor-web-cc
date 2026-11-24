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
    "_value_load_memory", pathlib.Path(__file__).with_name("load_memory.py"))
_memory = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(_memory)


ARRAY_LEVELS = 32
WIDE_LEVELS = 14
NARROW_LEVELS = ARRAY_LEVELS - WIDE_LEVELS
TYPE_TEXT = "ARRAY[10..11] OF " * WIDE_LEVELS + "ARRAY[10..10] OF " * NARROW_LEVELS + "LDT"
PADDING = 4_600_000
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


REFERENCES = tuple(f"LOAD-V{number}" for number in range(1, 6))
BARRIER_TIMEOUT = 8
# Match http.request_timeout in reliability.md section 7.11.
REQUEST_TIMEOUT = 30
FIELD = re.compile(rb'<div class="field-error" id="([a-z0-9_.\[\]-]+)-error">')
CODE = re.compile(rb'<span class="field-error-code">(E-[A-Z]{3}-[0-9]{3})</span>')
KEY = re.compile(r"[0-9a-f]{8}-(?:[0-9a-f]{4}-){3}[0-9a-f]{12}")


def remaining(deadline):
    seconds = deadline - time.monotonic()
    if seconds <= 0:
        raise TimeoutError
    return seconds


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
        return f"round {self.answer['round']}: request {self.reference} did not reach the barrier"


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
                        sock.settimeout(remaining(deadline))
                        sock.sendall(body[:-1])
                        remaining(deadline)
                        at_barrier = True
                        deadline = barrier.arrive(draft.reference)
                        if deadline is None:
                            answer["error"] = barrier.failure
                            return
                        body = body[-1:]
                    sock.settimeout(remaining(deadline))
                    sock.sendall(body)
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


def overlap_anon(args, drafts, deadline):
    threshold = args.reference_anon + sum(len(draft.bytes) for draft in drafts)
    last_anon = None
    try:
        while True:
            anon = read_anon(args.cgroup, remaining(deadline))
            if anon is not None:
                last_anon = anon
            if anon is not None and anon >= threshold and time.monotonic() < deadline:
                return last_anon, None
            time.sleep(min(0.05, remaining(deadline)))
    except TimeoutError:
        return last_anon, f"round {args.round}: the bodies did not overlap"


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
    with concurrent.futures.ThreadPoolExecutor(max_workers=len(drafts)) as pool:
        try:
            tasks = [pool.submit(sender.send, draft, start, barrier) for draft in drafts]
            missing = barrier.missing(drafts)
            if missing is not None:
                barrier.failure = missing.barrier_error()
            else:
                samples.append(mem_available())
                anon, barrier.failure = overlap_anon(args, drafts, barrier.deadline)
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
        "mem_available_min_kib": min(samples), "answers": answers,
    }, failure


ROUND_ONE_MIN_BYTES = 16 * 1024 * 1024 - 64 * 1024
ROUND_ONE_MAX_BYTES = 16 * 1024 * 1024
MEMORY_PHASES = ("seed", "precheck", "baseline-1", "round-1", "baseline-2", "round-2")
CONTAINER_ID = re.compile(r"[0-9a-f]{64}")


def json_count(value):
    return value if type(value) is int and value >= 0 else None


def display(value):
    return "unavailable" if value is None else str(value)


class ValueCheck:
    def __init__(self, directory):
        self.directory = directory
        self.failures = []
        state = self.read("load-value-state.json", "value state")
        drafts = state.get("drafts")
        rows = {}
        if isinstance(drafts, list):
            for draft in drafts:
                if isinstance(draft, dict) and draft.get("reference") in REFERENCES:
                    rows[draft["reference"]] = draft
        if not isinstance(drafts, list) or len(drafts) != len(rows) or len(rows) != len(REFERENCES):
            self.failures.append("The value state must contain the five drafts.")
        self.keys = {}
        self.seed_versions = {}
        for ref in REFERENCES:
            draft = rows.get(ref, {})
            key = draft.get("key")
            if isinstance(key, str) and KEY.fullmatch(key):
                self.keys[ref] = key
            else:
                self.failures.append(f"value state {ref} has an invalid draft key")
            self.seed_versions[ref] = self.version(draft, "row_version", f"value state {ref}")
        if len(set(self.keys.values())) != len(self.keys):
            self.failures.append("The value state must contain distinct draft keys.")

    def read(self, filename, label):
        value = _memory.read_json(self.directory / filename)
        if isinstance(value, dict):
            return value
        self.failures.append(f"{label} is missing or invalid")
        return {}

    def version(self, values, key, label):
        value = json_count(values.get(key))
        if value is None:
            self.failures.append(f"{label} row_version is missing or invalid")
        return value

    def read_versions(self, phase, refs):
        values = self.read(f"load-value-versions-{phase}.json", f"{phase} versions")
        rows = values.get("row_versions")
        if not isinstance(rows, dict):
            self.failures.append(f"{phase} row_versions are missing or invalid")
            return {}
        return {ref: self.version(rows, ref, f"versions {phase} {ref}") for ref in refs}

    def compare_version(self, label, current, base, increased):
        if current is None or base is None:
            return
        if increased and current <= base:
            self.failures.append(f"{label} row_version did not increase")
        if not increased and current != base:
            self.failures.append(f"{label} row_version changed")

    def check_answer(self, answer, number, ref, label):
        if answer.get("error") is not None:
            self.failures.append(f"{label} failed during transport")
        status = json_count(answer.get("status"))
        expected = 422 if number == 1 else 303
        if status != expected:
            if status is not None and 100 <= status <= 599:
                self.failures.append(f"{label} returned status {status}. Expected {expected}.")
            else:
                self.failures.append(f"{label} has no valid status. Expected {expected}.")
        elif number == 1:
            field = answer.get("field")
            if (answer.get("code"), field) != ("E-WEB-008", "description_en"):
                field = field if field in ("form", "description_en") else "unavailable or invalid"
                self.failures.append(f"{label} has field {field}. Expected E-WEB-008 under "
                                     "description_en as the first field line.")
        elif ref in self.keys:
            location = f"/recipes/{ref}/versions/1/parameters/{self.keys[ref]}"
            if answer.get("location") != location:
                self.failures.append(f"{label} has an incorrect Location")

    def check_precheck(self, summary=None):
        ref = REFERENCES[-1]
        for number in (1, 2):
            label = f"pre-check round {number} request {ref}"
            answer = self.read(f"load-value-precheck-{number}.json", label)
            if summary is not None and answer:
                summary.append(answer_summary(label, answer))
            if (answer.get("reference") != ref or answer.get("key") != self.keys.get(ref)
                    or json_count(answer.get("round")) != number):
                self.failures.append(f"{label} has an incorrect draft or round")
            self.check_answer(answer, number, ref, label)
        versions = self.read_versions("precheck", (ref,))
        version = versions.get(ref)
        self.compare_version(f"pre-check {ref}", version, self.seed_versions[ref], True)
        return version

    def check_round_answers(self, number, summary):
        label = f"round {number}"
        batch = self.read(f"load-value-answers-{number}.json", f"{label} answers")
        answers = batch.get("answers")
        refs = REFERENCES[:4]
        if json_count(batch.get("round")) != number:
            self.failures.append(f"{label} answers have an incorrect round")
        if not isinstance(answers, list) or len(answers) != len(refs):
            self.failures.append(f"{label} must contain four answers")
            answers = answers if isinstance(answers, list) else []
        rows = {}
        lengths = []
        for answer in answers:
            if not isinstance(answer, dict) or answer.get("reference") not in refs:
                self.failures.append(f"{label} contains an answer for an invalid draft")
                continue
            ref = answer["reference"]
            request = f"{label} request {ref}"
            if summary is not None:
                summary.append(answer_summary(request, answer))
            if ref in rows:
                self.failures.append(f"{request} has more than one answer")
            rows[ref] = answer.get("key")
            if json_count(answer.get("round")) != number:
                self.failures.append(f"{request} has an incorrect round")
            self.check_answer(answer, number, ref, request)
            size = json_count(answer.get("body_bytes"))
            if not size:
                self.failures.append(f"{label} {ref} body length is missing or invalid")
            else:
                lengths.append(size)
                if number == 1 and not ROUND_ONE_MIN_BYTES <= size <= ROUND_ONE_MAX_BYTES:
                    self.failures.append(f"{label} {ref} body length is outside the 16 MiB window")
        if rows != {ref: self.keys.get(ref) for ref in refs}:
            self.failures.append(f"{label} does not use the four draft keys from the state file")
        return batch, lengths

    def check_round(self, number, precheck, summary=None):
        batch, lengths = self.check_round_answers(number, summary)
        versions = self.read_versions(number, REFERENCES)
        for ref in REFERENCES:
            measured = ref != REFERENCES[-1]
            base = self.seed_versions[ref] if measured else precheck
            self.compare_version(f"round {number} {ref}", versions.get(ref), base,
                                 number == 2 and measured)
        return batch, lengths

    def read_memory(self, phase):
        raw = self.read(f"load-memory-{phase}.json", f"{phase} reading")
        values, errors = _memory.check_reading(raw, phase)
        if phase == "value-round-1":
            errors = [f"{phase} {field} is unavailable or invalid"
                      for field, value in values.items() if value is None]
        self.failures.extend(errors)
        values["current"] = _memory.nonnegative_integer(raw.get("current"))
        if values["current"] is None:
            self.failures.append(f"{phase} current is unavailable or invalid")
        container = raw.get("container_id")
        if not isinstance(container, str) or not CONTAINER_ID.fullmatch(container):
            self.failures.append(f"{phase} container ID is unavailable or invalid")
            container = None
        values["container_id"] = container
        return values, bool(raw)

    def check_overlap(self, number, batch, lengths, baseline):
        reference = json_count(batch.get("reference_anon"))
        barrier = json_count(batch.get("barrier_anon"))
        if reference is None or (baseline is not None and reference != baseline):
            self.failures.append(f"round {number} reference_anon must equal the baseline anon")
        if barrier is None or (reference is not None and len(lengths) == 4
                               and barrier - reference < sum(lengths)):
            self.failures.append(f"round {number}: the bodies did not overlap")

    def elapsed(self, now, path, label):
        try:
            start = _memory.nonnegative_integer(path.read_text(encoding="ascii").strip())
        except (OSError, UnicodeError):
            start = None
        if start is not None and start <= now:
            return now - start
        self.failures.append(f"{label} start time is missing or invalid")
        return None

    def round_result(self, number, baseline, reading, batch, lengths):
        container = reading.get("container_id")
        if container and baseline.get("container_id") and container != baseline["container_id"]:
            self.failures.append(f"round {number} container ID differs from its baseline")
        self.check_overlap(number, batch, lengths, baseline.get("anon"))
        available = json_count(batch.get("mem_available_min_kib"))
        if available is None:
            self.failures.append(f"round {number} MemAvailable is missing or invalid")
        peak, current = reading.get("peak"), baseline.get("current")
        statistic = (peak - current) / 4 if peak is not None and current is not None else None
        return {"round": number, "container_id": container, "peak": peak,
                "body_bytes": max(lengths, default=None), "run_statistic": statistic,
                "mem_available_min_kib": available}

    def has_round_files(self, number):
        names = (f"load-value-answers-{number}.json", f"load-memory-value-round-{number}.json",
                 f"load-value-versions-{number}.json")
        return any((self.directory / name).is_file() for name in names)

    def check_round_readings(self, readings):
        round_one = readings["round-1"][0]["container_id"]
        round_two = readings["round-2"][0]["container_id"]
        if round_one is not None and round_one == round_two:
            self.failures.append("Both rounds use the same container ID")
        peak = readings["round-2"][0]["peak"]
        if peak is not None and peak >= _memory.SOAK_MEMORY_CEILING:
            self.failures.append("round 2 reading reached the ceiling of 384 MiB")

    def check_all(self, job_start, summary_path):
        stopped = not self.has_round_files(2)
        summary = []
        readings = {
            phase: self.read_memory(f"value-{phase}") for phase in MEMORY_PHASES
            if not stopped or (self.directory / f"load-memory-value-{phase}.json").is_file()
        }
        if stopped:
            self.failures.append("The value leg stopped before round 2.")
            for phase, (reading, _) in readings.items():
                if phase != "round-1":
                    summary.extend(reading_summary(f"value-{phase}", reading))
            precheck = self.check_precheck(summary)
            numbers = (1,) if self.has_round_files(1) else ()
        else:
            precheck = self.read_versions("precheck", REFERENCES[-1:]).get(REFERENCES[-1])
            self.check_round_readings(readings)
            numbers = (1, 2)

        rounds = []
        for number in numbers:
            batch, lengths = self.check_round(number, precheck, summary if stopped else None)
            baseline, _ = readings.get(f"baseline-{number}", ({}, False))
            reading, present = readings.get(f"round-{number}", ({}, False))
            if stopped and not reading:
                self.failures.append(f"value-round-{number} reading is missing or invalid")
            result = self.round_result(number, baseline, reading, batch, lengths)
            if present:
                rounds.append(result)
            if present or (stopped and (reading or batch)):
                summary.extend(round_summary(result, baseline, reading))
        now = int(time.time())
        job = self.elapsed(now, job_start, "job")
        leg = self.elapsed(now, self.directory / "load-value-leg-start", "value leg")
        summary.extend((f"job elapsed: {display(job)} s", f"value leg elapsed: {display(leg)} s"))
        result = {"passed": not self.failures, "failures": self.failures, "rounds": rounds,
                  "job_elapsed_s": job, "leg_elapsed_s": leg}
        with summary_path.open("a", encoding="utf-8") as stream:
            stream.write("\n".join(summary + self.failures) + "\n")
        (self.directory / "load-value.json").write_text(
            json.dumps(result, indent=2, allow_nan=False) + "\n", encoding="utf-8",
        )


def answer_summary(label, answer):
    status = json_count(answer.get("status"))
    if status is not None and not 100 <= status <= 599:
        status = None
    size = json_count(answer.get("body_bytes"))
    return f"{label} status={display(status)} body length={display(size)} bytes"


def reading_summary(label, reading):
    return [
        f"{label} container ID: {display(reading['container_id'])}",
        f"{label} memory.peak={display(reading['peak'])} bytes",
        f"{label} memory.current={display(reading['current'])} bytes",
        f"{label} anon={display(reading['anon'])} bytes",
    ]


def round_summary(result, baseline, reading):
    label = f"round {result['round']}"
    size = result["body_bytes"]
    triple = 3 * size if size is not None else None
    lines = reading_summary(label, reading) if reading else []
    lines.extend([
        f"{label} baseline memory.current: {display(baseline.get('current'))} bytes",
        f"{label} body length: {display(size)} bytes",
        (f"{label} run statistic: {display(result['run_statistic'])} bytes "
         f"(upper bound, 3 × body length: {display(triple)} bytes)"),
        f"{label} smallest MemAvailable: {display(result['mem_available_min_kib'])} KiB",
    ])
    if reading:
        killed = reading["OOMKilled"]
        killed = str(killed).lower() if killed is not None else None
        lines.append(
            f"{label} oom={display(reading['oom'])} oom_kill={display(reading['oom_kill'])} "
            f"max={display(reading['max'])} OOMKilled={display(killed)} "
            f"RestartCount={display(reading['RestartCount'])}"
        )
    return lines


def run_checker(args):
    checker = ValueCheck(args.ci_state)
    if args.command == "check-precheck":
        checker.check_precheck()
    elif args.command == "check-round":
        precheck = checker.read_versions("precheck", REFERENCES[-1:]).get(REFERENCES[-1])
        checker.check_round(args.round, precheck)
    else:
        checker.check_all(args.job_start, args.summary)
    return checker.failures


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
