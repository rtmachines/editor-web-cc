"""Check the encoded value bodies against the Rust route test pins."""

import contextlib
import hashlib
import http.server
import importlib.util
import io
import json
import os
import pathlib
import queue
import select
import socket
import subprocess
import sys
import tempfile
import threading
import time
import unittest
import urllib.parse
import uuid
from types import SimpleNamespace
from unittest.mock import patch


class LoadValueTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        path = pathlib.Path(__file__).with_name("load_value.py")
        spec = importlib.util.spec_from_file_location("load_value", path)
        cls.module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(cls.module)

    def test_body_pins_match_the_rust_route_tests(self):
        pins = (
            (1, BODY_BYTES, BODY_SHA256),
            (2, WRITE_BYTES, WRITE_SHA256),
        )
        for round_number, expected_length, expected_sha256 in pins:
            body = self.module.build_body(round_number, row_version=1, unit_path="U1", ordinal=0)
            with self.subTest(round_number=round_number, pin="length"):
                self.assertEqual(len(body), expected_length)
            with self.subTest(round_number=round_number, pin="sha256"):
                self.assertEqual(hashlib.sha256(body).hexdigest(), expected_sha256)


BODY_BYTES = 16_743_514
BODY_SHA256 = "354b1a3e4c5ff56400dc92f6f3ddf84fc104390e3334269d852b3ebae599ccc9"
WRITE_BYTES = 3_033_541
WRITE_SHA256 = "631700fb82002bb379b37cc2cc9a7d6d9ef2dd628769e4ac3dd9a517f3076022"
COOKIE = "__Host-recipe_session=fixture-session"
TOKEN = "fixture-token"
REFERENCE_ANON = 4096
ANSWER_KEYS = {
    "round", "reference", "key", "body_bytes", "status", "location", "code", "field",
    "last_byte_s", "answer_s", "error",
}


class ValuePeer(http.server.BaseHTTPRequestHandler):
    def do_POST(self):
        size = int(self.headers["Content-Length"])
        remaining = size
        digest = hashlib.sha256()
        while remaining:
            chunk = self.rfile.read1(min(remaining, 64 * 1024))
            if not chunk:
                return
            digest.update(chunk)
            remaining -= len(chunk)
        self.server.requests.append((self.path, self.headers, size, digest.hexdigest()))
        status, body = self.server.answer
        self.send_response(status)
        if status == 303:
            self.send_header("Location", self.path.removesuffix("/definition"))
        self.send_header("Set-Cookie", COOKIE)
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        if body:
            self.wfile.write(body)

    def log_message(self, *_args):
        pass


class FixtureSocket(socket.socket):
    def send(self, data):
        return self.write(data, time.monotonic() + self.gettimeout())

    def sendall(self, data):
        deadline = time.monotonic() + self.gettimeout()
        data = memoryview(data)
        while data:
            data = data[self.write(data, deadline):]

    def write(self, data, deadline):
        # Use os.write because the sandbox can block sends on local sockets.
        while True:
            timeout = max(0, deadline - time.monotonic())
            if not select.select([], [self], [], timeout)[1] or timeout == 0:
                raise TimeoutError
            try:
                return os.write(self.fileno(), data)
            except BlockingIOError:
                continue


class SlowTlsSocket(contextlib.AbstractContextManager):
    def __init__(self):
        self.now = 0
        self.timeout = None
        self.written = 0
        self.attempt_sizes = []

    def __exit__(self, *_exc):
        pass

    def settimeout(self, timeout):
        self.timeout = timeout

    def send(self, data):
        self.attempt_sizes.append(len(data))
        self.now += min(self.timeout, 0.75)
        if self.timeout < 0.75:
            raise TimeoutError
        size = min(len(data), 256 * 1024)
        self.written += size
        return size

    def sendall(self, data):
        # Model the separate timeout for each write in TLS sendall.
        while data:
            data = data[self.send(data):]


class ValueServer:
    def __init__(self, serial, answer):
        self.serial = serial
        self.answer = answer
        self.requests = []
        self.threads = []
        self.pending = queue.Queue()
        if serial:
            self.start(self.serve_serial)

    def start(self, target, *args):
        thread = threading.Thread(target=target, args=args)
        self.threads.append(thread)
        thread.start()

    def serve_serial(self):
        for peer in iter(self.pending.get, None):
            self.serve(peer)

    def serve(self, peer):
        with peer:
            ValuePeer(peer, (), self)

    def connect(self, _address, timeout):
        # A socket pair gives the peer a real stream without a listening port.
        pair = socket.socketpair()
        client, peer = [FixtureSocket(socket.AF_UNIX, socket.SOCK_STREAM, 0, sock.detach())
                        for sock in pair]
        client.settimeout(timeout)
        peer.settimeout(30)
        if self.serial:
            for sock, option in ((peer, socket.SO_RCVBUF), (client, socket.SO_SNDBUF)):
                try:
                    sock.setsockopt(socket.SOL_SOCKET, option, 16 * 1024)
                except PermissionError:
                    # The default buffer also blocks a body of this size.
                    pass
            self.pending.put(peer)
        else:
            self.start(self.serve, peer)
        return client

    def close(self):
        if self.serial:
            self.pending.put(None)
        for thread in self.threads:
            thread.join()


class ValueSenderTests(unittest.TestCase):
    def setUp(self):
        path = pathlib.Path(__file__).with_name("load_value.py")
        spec = importlib.util.spec_from_file_location("load_value", path)
        self.module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(self.module)
        self.state = pathlib.Path(self.enterContext(tempfile.TemporaryDirectory()))
        self.drafts = [
            {"reference": f"LOAD-V{number}", "key": str(uuid.UUID(int=number)),
             "machine": f"M900000{number + 1}", "row_version": 1, "unit_path": "U1", "ordinal": 0}
            for number in range(1, 6)
        ]
        (self.state / "load-value-state.json").write_text(json.dumps({
            "cookie": COOKIE, "csrf_token": TOKEN, "drafts": self.drafts,
        }), encoding="utf-8")
        self.stat = self.state / "memory.stat"
        self.set_anon(REFERENCE_ANON + 1)
        self.context = self.enterContext(patch("ssl.create_default_context"))
        self.context.return_value.wrap_socket.side_effect = lambda sock, **_options: sock
        self.samples = self.enterContext(patch("subprocess.run", side_effect=self.read_stat))

    def set_anon(self, value):
        self.stat.write_text(f"anon {value}\nfile 90000000\n", encoding="ascii")

    def read_stat(self, command, **options):
        self.assertEqual(command, ["sudo", "-n", "cat", str(self.stat)])
        self.assertGreater(options["timeout"], 0)
        return subprocess.CompletedProcess(command, 0, self.stat.read_bytes())

    def start_peer(self, *, serial=False, status=422, body=b""):
        server = ValueServer(serial, (status, body))
        self.addCleanup(server.close)
        self.enterContext(patch("socket.create_connection", side_effect=server.connect))
        return server

    def run_sender(self, command, round_number, *options):
        self.origin = "https://localhost:8443"
        self.ca = self.state / "root.crt"
        error = io.StringIO()
        with contextlib.redirect_stderr(error):
            status = self.module.cli([
                command, "--ci-state", str(self.state), "--round", str(round_number),
                "--url", self.origin, "--cacert", str(self.ca), *options,
            ])
        return status, error.getvalue()

    def run_round(self, deadline="2"):
        return self.run_sender(
            "round", 1, "--cgroup", str(self.state), "--reference-anon",
            str(REFERENCE_ANON), "--deadline", deadline,
        )

    def read_report(self, filename):
        text = (self.state / filename).read_text(encoding="utf-8")
        for private in (COOKIE, TOKEN, "<div", "<html"):
            self.assertNotIn(private, text)
        return json.loads(text)

    def assert_requests(self, server, drafts, size, digest):
        expected = {f"/recipes/{d['reference']}/versions/1/parameters/{d['key']}/definition"
                    for d in drafts}
        self.assertEqual(len(server.requests), len(expected))
        self.assertEqual({path for path, *_ in server.requests}, expected)
        for _, headers, length, sha256 in server.requests:
            self.assertEqual((length, sha256), (size, digest))
            self.assertEqual(headers["Cookie"], COOKIE)
            self.assertEqual(headers["X-CSRF-Token"], TOKEN)
            self.assertEqual(headers["Origin"], self.origin)
            self.assertNotIn("HX-Request", headers)
        self.context.assert_called_once_with(cafile=str(self.ca))

    def test_slow_body_stops_at_the_send_deadline(self):
        args = SimpleNamespace(url=urllib.parse.urlsplit("https://127.0.0.1"),
                               cacert=self.state / "root.crt")
        sender = self.module.ValueSender(args, {"cookie": COOKIE, "csrf_token": TOKEN})
        barrier = self.module.RoundBarrier(1)
        barrier.release.set()
        for phase, gate, error in (
            ("barrier", barrier, "round 1: request LOAD-V1 did not reach the barrier"),
            ("single", None, "round 1: request LOAD-V1 failed during answer collection"),
        ):
            with self.subTest(phase=phase):
                sock = SlowTlsSocket()
                draft = self.module.DraftBody(1, self.drafts[0], b"x" * (2 * 1024 * 1024))
                with (patch.object(self.module.time, "monotonic", lambda sock=sock: sock.now),
                      patch.object(self.module, "REQUEST_TIMEOUT", 1),
                      patch.object(sender, "connect", return_value=sock),
                      patch.object(sender, "send_headers"),
                      patch.object(sender, "receive") as receive):
                    sender.send(draft, 0, gate)

                self.assertEqual(draft.answer["error"], error)
                self.assertEqual(sock.now, 1)
                self.assertEqual(sock.written, 256 * 1024)
                self.assertLessEqual(max(sock.attempt_sizes), 1024 * 1024)
                self.assertIsNone(draft.answer["last_byte_s"])
                receive.assert_not_called()
        self.assertTrue(barrier.ready.empty())

    def test_round_fails_when_a_serial_peer_blocks_the_barrier(self):
        server = self.start_peer(serial=True)
        status, error = self.run_round()
        self.assertEqual(status, 1)
        self.assertRegex(error, r"^round 1: request LOAD-V[1-4] did not reach the barrier\n$")
        self.samples.assert_not_called()
        self.assertEqual(server.requests, [])
        report = self.read_report("load-value-answers-1.json")
        self.assertIsNone(report["barrier_anon"])
        self.assertEqual(report.get("barrier_failure"), error.strip())

    def test_round_fails_when_no_anon_sample_exists(self):
        self.start_peer()
        self.stat.write_text("file 90000000\n", encoding="ascii")
        self.assertEqual(self.run_round(), (1, "round 1 barrier_anon is missing or invalid\n"))
        self.samples.assert_called()

    def test_round_fails_when_anon_does_not_rise_enough(self):
        server = self.start_peer()
        self.assertEqual(self.run_round(), (1, "round 1: the bodies did not overlap\n"))
        self.assertGreater(self.samples.call_count, 1)
        report = self.read_report("load-value-answers-1.json")
        self.assertEqual(report["barrier_anon"], REFERENCE_ANON + 1)
        self.assertTrue(all(answer["last_byte_s"] is None for answer in report["answers"]))
        self.assertEqual(server.requests, [])

    def test_round_releases_four_pinned_bodies_after_anon_rises(self):
        server = self.start_peer()

        def rising_anon(command, **options):
            result = self.read_stat(command, **options)
            self.assertEqual(server.requests, [])
            self.set_anon(REFERENCE_ANON + 4 * BODY_BYTES)
            return result

        self.samples.side_effect = rising_anon
        with patch.object(self.module, "mem_available", side_effect=(20_000, 10_000)) as samples:
            self.assertEqual(self.run_round(deadline="4"), (0, ""))
        self.assertEqual(samples.call_count, 2)
        self.assertGreaterEqual(self.samples.call_count, 2)
        self.assert_requests(server, self.drafts[:4], BODY_BYTES, BODY_SHA256)
        report = self.read_report("load-value-answers-1.json")
        self.assertEqual(report["round"], 1)
        self.assertEqual(report["reference_anon"], REFERENCE_ANON)
        self.assertEqual(report["barrier_anon"], REFERENCE_ANON + 4 * BODY_BYTES)
        self.assertEqual(report["mem_available_min_kib"], 10_000)
        self.assertEqual([answer["reference"] for answer in report["answers"]],
                         [draft["reference"] for draft in self.drafts[:4]])
        for answer in report["answers"]:
            self.assertEqual(answer.keys(), ANSWER_KEYS)
            self.assertEqual(answer["status"], 422)
            self.assertIsNone(answer["error"])
            self.assertLessEqual(0, answer["last_byte_s"])
            self.assertLessEqual(answer["last_byte_s"], answer["answer_s"])

    def test_round_keeps_the_first_field_line_of_a_large_422(self):
        page = b"<html>".ljust(8 * 1024 - 12, b"x") + (
            b'<div class="field-error" id="description_en-error"><p>'
            b'<span class="field-error-code">E-WEB-008</span></p></div>'
            b'<div class="field-error" id="name_en-error"><p>'
            b'<span class="field-error-code">E-WEB-001</span></p></div>'
        ) + b"x" * (6 * 1024 * 1024) + TOKEN.encode() + b"</html>"
        self.start_peer(body=page)
        self.set_anon(REFERENCE_ANON + 4 * BODY_BYTES)
        self.assertEqual(self.run_round(deadline="4"), (0, ""))
        report = self.read_report("load-value-answers-1.json")
        for answer in report["answers"]:
            self.assertEqual((answer["status"], answer["code"], answer["field"]),
                             (422, "E-WEB-008", "description_en"))

    def test_single_records_the_selected_draft_and_redirect(self):
        server = self.start_peer(status=303)
        draft = self.drafts[4]
        self.assertEqual(self.run_sender("single", 2, "--reference", "LOAD-V5"), (0, ""))
        self.assert_requests(server, [draft], WRITE_BYTES, WRITE_SHA256)
        self.samples.assert_not_called()
        answer = self.read_report("load-value-precheck-2.json")
        self.assertEqual(answer.keys(), ANSWER_KEYS)
        self.assertEqual(
            (answer["round"], answer["reference"], answer["key"], answer["body_bytes"]),
            (2, "LOAD-V5", draft["key"], WRITE_BYTES),
        )
        self.assertEqual((answer["status"], answer["location"]),
                         (303, f"/recipes/LOAD-V5/versions/1/parameters/{draft['key']}"))
        for name in ("code", "field", "error"):
            self.assertIsNone(answer[name])
        self.assertLessEqual(0, answer["last_byte_s"])
        self.assertLessEqual(answer["last_byte_s"], answer["answer_s"])


class ValueCheckerTests(unittest.TestCase):
    ceiling = 402_653_184

    def setUp(self):
        self.state = pathlib.Path(self.enterContext(tempfile.TemporaryDirectory()))
        self.summary = self.state / "summary"
        self.summary.write_text("Earlier summary.\n", encoding="utf-8")
        self.job_start = self.state / "job-start"
        self.job_start.write_text(f"{int(time.time()) - 100}\n", encoding="ascii")
        (self.state / "load-value-leg-start").write_text(
            f"{int(time.time()) - 40}\n", encoding="ascii",
        )
        self.refs = [f"LOAD-V{number}" for number in range(1, 6)]
        self.keys = [str(uuid.UUID(int=number)) for number in range(1, 6)]
        drafts = [
            {"reference": ref, "key": key, "machine": f"M900000{number + 2}",
             "row_version": 1, "unit_path": "U1", "ordinal": 0}
            for number, (ref, key) in enumerate(zip(self.refs, self.keys))
        ]
        self.write("state", {"drafts": drafts, "cookie": COOKIE, "csrf_token": TOKEN})
        for phase, versions in (("precheck", [1, 1, 1, 1, 2]), ("1", [1, 1, 1, 1, 2]),
                                ("2", [2, 2, 2, 2, 2])):
            self.write(f"versions-{phase}", {"row_versions": dict(zip(self.refs, versions))})
        for number, size in ((1, BODY_BYTES), (2, WRITE_BYTES)):
            answers = [self.answer(number, index, size) for index in range(4)]
            self.write(f"precheck-{number}", self.answer(number, 4, size))
            self.write(f"answers-{number}", {
                "round": number, "reference_anon": REFERENCE_ANON,
                "barrier_anon": REFERENCE_ANON + 4 * size,
                "mem_available_min_kib": 10_000, "answers": answers,
            })
        for phase, digit in (("seed", "a"), ("precheck", "b"), ("baseline-1", "c"),
                             ("round-1", "c"), ("baseline-2", "d"), ("round-2", "d")):
            self.write(f"memory-value-{phase}", {
                "container_id": digit * 64, "peak": str(self.ceiling - 1),
                "current": "20000001", "anon": str(REFERENCE_ANON), "file": "1024",
                "limit": "536870912", "oom": "0", "oom_kill": "0", "max": "0",
                "OOMKilled": "false", "RestartCount": "0",
            })

    def answer(self, number, index, size):
        ref, key = self.refs[index], self.keys[index]
        refused = number == 1
        return {
            "round": number, "reference": ref, "key": key, "body_bytes": size,
            "status": 422 if refused else 303,
            "location": None if refused else f"/recipes/{ref}/versions/1/parameters/{key}",
            "code": "E-WEB-008" if refused else None,
            "field": "description_en" if refused else None,
            "last_byte_s": 0.5, "answer_s": 1.0, "error": None,
        }

    def path(self, name):
        prefix = "load-" if name.startswith("memory-") else "load-value-"
        return self.state / f"{prefix}{name}.json"

    def write(self, name, data):
        self.path(name).write_text(json.dumps(data), encoding="utf-8")

    def read(self, name):
        return json.loads(self.path(name).read_text(encoding="utf-8"))

    def change_answer(self, number, **changes):
        name = f"answers-{number}"
        batch = self.read(name)
        batch["answers"][0].update(changes)
        self.write(name, batch)

    def change_reading(self, phase, **changes):
        name = f"memory-value-{phase}"
        self.write(name, self.read(name) | changes)

    def run_check(self, command="check", expected=0):
        options = {
            "check": ["--job-start", str(self.job_start), "--summary", str(self.summary)],
            "check-round": ["--round", "1"],
        }
        script = pathlib.Path(__file__).with_name("load_value.py")
        result = subprocess.run(
            [sys.executable, "-I", "-B", str(script), command, "--ci-state", str(self.state),
             *options.get(command, [])], capture_output=True, text=True, timeout=5,
        )
        self.assertEqual(result.returncode, expected, result.stderr.strip())
        return result.stderr

    def check_failures(self, *messages):
        error = self.run_check(expected=1)
        report = json.loads(
            (self.state / "load-value.json").read_text(encoding="utf-8"),
        )
        self.assertFalse(report["passed"])
        self.assertEqual(error.splitlines(), report["failures"])
        for message in messages:
            self.assertTrue(any(message in failure for failure in report["failures"]), error)

    def test_precheck_accepts_both_answers_and_the_larger_version(self):
        self.assertEqual(self.run_check("check-precheck"), "")

    def test_precheck_requires_description_code_location_and_larger_version(self):
        self.write("precheck-1", self.read("precheck-1") | {"code": "E-WEB-001"})
        self.write("precheck-2", self.read("precheck-2") | {"location": f"/login?x={TOKEN}"})
        versions = self.read("versions-precheck")
        versions["row_versions"]["LOAD-V5"] = 1
        self.write("versions-precheck", versions)
        error = self.run_check("check-precheck", expected=1)
        for field in ("E-WEB-008", "Location", "row_version"):
            self.assertIn(field, error)
        self.assertNotIn(TOKEN, error)

    def test_final_check_requires_both_precheck_answers(self):
        self.path("precheck-1").unlink()
        self.write("precheck-2", self.read("precheck-2") | {"location": "/login"})

        self.check_failures("pre-check round 1 request LOAD-V5 is missing or invalid",
                            "pre-check round 2 request LOAD-V5 has an incorrect Location")

    def test_round_one_uses_precheck_version_for_load_v5(self):
        self.assertEqual(self.run_check("check-round"), "")

    def test_round_one_rejects_wrong_field_status_body_length_and_versions(self):
        batch = self.read("answers-1")
        batch["answers"][0]["field"] = "form"
        batch["answers"][1]["status"] = 303
        batch["answers"][2]["body_bytes"] = 16_711_679
        batch["answers"][3]["body_bytes"] = 16_777_217
        self.write("answers-1", batch)
        versions = self.read("versions-1")
        versions["row_versions"].update({"LOAD-V1": 2, "LOAD-V5": 1})
        self.write("versions-1", versions)
        error = self.run_check("check-round", expected=1)
        for message in ("field form", "LOAD-V2 returned status 303", "LOAD-V3 body length",
                        "LOAD-V4 body length", "LOAD-V1 row_version", "LOAD-V5 row_version"):
            self.assertIn(message, error)

    def test_round_one_at_ceiling_reports_both_rounds_and_passes(self):
        self.change_reading("round-1", peak=str(self.ceiling))
        batch = self.read("answers-2")
        batch["barrier_anon"] += 29
        self.write("answers-2", batch)
        self.assertEqual(self.run_check(), "")
        report = json.loads((self.state / "load-value.json").read_text(encoding="utf-8"))
        self.assertTrue(report["passed"])
        self.assertEqual(report["failures"], [])
        self.assertGreaterEqual(report["job_elapsed_s"], 100)
        self.assertGreaterEqual(report["leg_elapsed_s"], 40)
        self.assertEqual(report["rounds"], [
            {"round": 1, "container_id": "c" * 64, "peak": self.ceiling, "body_bytes": BODY_BYTES,
             "run_statistic": 95_663_295.75, "mem_available_min_kib": 10_000,
             "reference_anon": 4096, "barrier_anon": 66_978_152, "overlap_margin": 0},
            {"round": 2, "container_id": "d" * 64, "peak": self.ceiling - 1,
             "body_bytes": WRITE_BYTES, "run_statistic": 95_663_295.5,
             "mem_available_min_kib": 10_000, "reference_anon": 4096,
             "barrier_anon": 12_138_289, "overlap_margin": 29},
        ])
        summary = self.summary.read_text(encoding="utf-8")
        self.assertTrue(summary.startswith("Earlier summary.\n"))
        for value in (str(self.ceiling), "c" * 64, "d" * 64, "20000001", "MemAvailable",
                      "job elapsed", "leg elapsed", str(BODY_BYTES), str(WRITE_BYTES)):
            self.assertIn(value, summary)
        self.assertRegex(summary, rf"(?m)^.*95663295\.75.*{3 * BODY_BYTES}.*$")
        self.assertRegex(summary, rf"(?m)^.*95663295\.5.*{3 * WRITE_BYTES}.*$")
        for number, barrier, margin in ((1, 66_978_152, 0), (2, 12_138_289, 29)):
            self.assertIn(f"round {number} reference_anon: 4096 bytes\n", summary)
            self.assertIn(f"round {number} barrier_anon: {barrier} bytes\n", summary)
            self.assertIn(f"round {number} overlap_margin: {margin} bytes\n", summary)

    def test_round_two_reports_a_negative_overlap_margin_for_unequal_bodies(self):
        self.change_answer(2, body_bytes=WRITE_BYTES + 7)

        self.assertEqual(self.run_check(expected=1), "round 2: the bodies did not overlap\n")
        report = json.loads((self.state / "load-value.json").read_text(encoding="utf-8"))
        self.assertEqual(report["rounds"][1].get("overlap_margin"), -7)
        summary = self.summary.read_text(encoding="utf-8")
        self.assertIn("round 2 overlap_margin: -7 bytes\n", summary)

    def test_round_two_at_ceiling_fails(self):
        self.change_reading("round-2", peak=str(self.ceiling))
        self.check_failures("round 2 reading reached the ceiling of 384 MiB")

    def test_round_two_rejects_four_short_bodies(self):
        batch = self.read("answers-2")
        for answer in batch["answers"]:
            answer["body_bytes"] = 256
        batch["barrier_anon"] = REFERENCE_ANON + 4 * 256
        self.write("answers-2", batch)
        error = self.run_check(expected=1)
        self.assertEqual(error.splitlines(), [
            f"round 2 {ref} body length is outside the pinned window" for ref in self.refs[:4]
        ])

    def test_round_two_accepts_only_lengths_within_the_pinned_window(self):
        batch = self.read("answers-2")
        sizes = (WRITE_BYTES - 1, WRITE_BYTES, WRITE_BYTES + 18, WRITE_BYTES + 19)
        for answer, size in zip(batch["answers"], sizes):
            answer["body_bytes"] = size
        batch["barrier_anon"] = REFERENCE_ANON + sum(sizes)
        self.write("answers-2", batch)
        error = self.run_check(expected=1)
        self.assertEqual(error.splitlines(), [
            f"round 2 {ref} body length is outside the pinned window"
            for ref in ("LOAD-V1", "LOAD-V4")
        ])

    def test_round_one_reports_each_memory_bound(self):
        conditions = (("oom", "1"), ("oom_kill", "1"), ("max", "1"), ("OOMKilled", "true"),
                      ("RestartCount", "1"), ("peak", "536870912"))
        original = self.read("memory-value-round-1")
        for field, value in conditions:
            with self.subTest(field=field):
                self.summary.write_text("", encoding="utf-8")
                self.write("memory-value-round-1", original | {field: value})
                self.assertEqual(self.run_check(), "")
                summary = self.summary.read_text(encoding="utf-8")
                self.assertIn(f"{field}={value}", summary)

    def test_round_one_unavailable_peak_fails(self):
        self.change_reading("round-1", peak="unavailable")
        self.check_failures("value-round-1 peak")

    def test_oversized_memory_counters_preserve_the_reports(self):
        self.change_reading("round-1", peak="9" * 400)
        self.change_reading("baseline-2", current=2**63)
        self.change_answer(2, status=409)

        failures = self.run_check(expected=1).splitlines()
        artifact = self.state / "load-value.json"
        self.assertTrue(artifact.is_file(), "\n".join(failures))
        report = json.loads(artifact.read_text(encoding="utf-8"))
        self.assertFalse(report["passed"])
        self.assertEqual(report["failures"], failures)
        summary = self.summary.read_text(encoding="utf-8").splitlines()
        for prefix in ("value-round-1 peak", "value-baseline-2 current",
                       "round 2 request LOAD-V1 returned status 409"):
            self.assertTrue(any(line.startswith(prefix) for line in failures), failures)
        for failure in failures:
            self.assertIn(failure, summary)
        self.assertIsNone(report["rounds"][0]["peak"])
        self.assertEqual([row["run_statistic"] for row in report["rounds"]], [None, None])

    def test_round_one_stop_reports_the_missed_barrier(self):
        cause = "round 1: request LOAD-V1 did not reach the barrier"
        batch = self.read("answers-1")
        batch.update(barrier_anon=None, barrier_failure=cause)
        for answer in batch["answers"]:
            answer.update(status=None, code=None, field=None, last_byte_s=None, answer_s=None,
                          error=cause)
        self.write("answers-1", batch)
        for name in ("answers-2", "versions-2", "memory-value-baseline-2", "memory-value-round-2"):
            self.path(name).unlink()

        self.check_failures("stopped before round 2")
        report = json.loads((self.state / "load-value.json").read_text(encoding="utf-8"))
        summary = self.summary.read_text(encoding="utf-8").splitlines()
        result = report["rounds"][0]
        expected = {"reference_anon": 4096, "barrier_anon": None, "overlap_margin": None}
        self.assertEqual({key: result.get(key, "missing") for key in expected}, expected)
        self.assertIn("round 1 barrier_anon: unavailable bytes", summary)
        self.assertIn("round 1 overlap_margin: unavailable bytes", summary)
        with self.subTest("sender cause"):
            self.assertEqual(report["failures"].count(cause), 1, report["failures"])
            self.assertEqual(summary.count(cause), 1)
        with self.subTest("missing sample"):
            shortfall = "round 1: the bodies did not overlap"
            self.assertNotIn(shortfall, report["failures"])
            self.assertNotIn(shortfall, summary)

    def test_final_check_requires_a_sample_or_a_valid_missed_barrier(self):
        batch = self.read("answers-2")
        batch.update(barrier_anon=None, barrier_failure=f"invalid barrier failure {TOKEN}")
        self.write("answers-2", batch)

        self.assertEqual(self.run_check(expected=1).splitlines(), [
            "round 2 barrier_anon is missing or invalid",
        ])
        self.assertNotIn(TOKEN, self.summary.read_text(encoding="utf-8"))

    def test_round_one_without_statuses_reports_the_stop(self):
        batch = self.read("answers-1")
        for answer in batch["answers"]:
            answer.update(status=None, code=None, field=None, answer_s=None)
        self.write("answers-1", batch)
        for name in ("versions-1", "answers-2", "versions-2", "memory-value-baseline-2",
                     "memory-value-round-2"):
            self.path(name).unlink()
        self.change_reading("round-1", peak="536870912", oom="2", oom_kill="1", max="3",
                            OOMKilled="true", RestartCount="1")

        self.check_failures("stopped before round 2", *(
            f"round 1 request {ref} has no valid status" for ref in self.refs[:4]
        ))
        report = json.loads((self.state / "load-value.json").read_text(encoding="utf-8"))
        self.assertEqual(report["rounds"], [
            {"round": 1, "container_id": "c" * 64, "peak": 536_870_912,
             "body_bytes": BODY_BYTES, "run_statistic": 129_217_727.75,
             "mem_available_min_kib": 10_000, "reference_anon": 4096,
             "barrier_anon": 66_978_152, "overlap_margin": 0},
        ])
        summary = self.summary.read_text(encoding="utf-8")
        for line in ("value-seed memory.peak=", "value-precheck memory.peak=",
                     "value-baseline-1 memory.peak=",
                     "value-seed oom=0 oom_kill=0 max=0 OOMKilled=false RestartCount=0",
                     "value-baseline-1 oom=0 oom_kill=0 max=0 OOMKilled=false RestartCount=0",
                     "pre-check round 1 request LOAD-V5 status=422",
                     "pre-check round 2 request LOAD-V5 status=303",
                     "stopped before round 2", "round 1 memory.peak=536870912 bytes",
                     "round 1 oom=2 oom_kill=1 max=3 OOMKilled=true RestartCount=1"):
            self.assertIn(line, summary)
        for ref in self.refs[:4]:
            self.assertIn(f"round 1 request {ref} status=unavailable", summary)
            self.assertIn(f"round 1 request {ref} has no valid status", summary)

    def test_precheck_failure_reports_the_available_files(self):
        self.write("precheck-1", self.read("precheck-1") | {"status": 500})
        self.change_reading("precheck", oom="2", oom_kill="1", max="3", OOMKilled="true",
                            RestartCount="1")
        for name in ("memory-value-baseline-1", "memory-value-round-1",
                     "memory-value-baseline-2", "memory-value-round-2", "answers-1", "answers-2",
                     "versions-1", "versions-2"):
            self.path(name).unlink()

        cause = "pre-check round 1 request LOAD-V5 returned status 500. Expected 422."
        self.assertEqual(self.run_check("check-precheck", expected=1).splitlines(), [cause])
        self.check_failures("stopped before round 2", cause)
        report = json.loads((self.state / "load-value.json").read_text(encoding="utf-8"))
        self.assertEqual(report["failures"].count(cause), 1)
        self.assertEqual(report["rounds"], [])
        summary = self.summary.read_text(encoding="utf-8")
        self.assertIn("value-seed memory.peak=", summary)
        self.assertIn("value-precheck oom=2 oom_kill=1 max=3 OOMKilled=true RestartCount=1",
                      summary.splitlines())
        self.assertIn("pre-check round 1 request LOAD-V5 status=500", summary)
        self.assertIn(cause, summary)

    def test_round_two_keeps_each_memory_bound(self):
        conditions = (("oom", "1"), ("oom_kill", "1"), ("max", "1"), ("OOMKilled", "true"),
                      ("RestartCount", "1"), ("peak", "536870912"))
        original = self.read("memory-value-round-2")
        for field, value in conditions:
            with self.subTest(field=field):
                self.write("memory-value-round-2", original | {field: value})
                message = "512 MiB" if field == "peak" else f"value-round-2 {field}"
                self.check_failures(message)

    def test_other_value_readings_also_gate_and_require_valid_values(self):
        self.change_reading("precheck", RestartCount="1")
        self.change_reading("baseline-2", current="invalid")
        self.path("memory-value-seed").unlink()
        self.check_failures("value-precheck RestartCount", "value-baseline-2 current",
                            "value-seed reading")

    def test_collects_round_two_status_and_ceiling_failures(self):
        self.change_reading("round-2", peak=str(self.ceiling))
        self.change_answer(2, status=409)
        self.check_failures("round 2 reading reached the ceiling of 384 MiB",
                            "round 2 request LOAD-V1 returned status 409")

    def test_round_two_rejects_login_redirect_and_wrong_versions(self):
        self.change_answer(2, location="/login")
        versions = self.read("versions-2")
        versions["row_versions"].update({"LOAD-V1": 1, "LOAD-V5": 3})
        self.write("versions-2", versions)
        self.check_failures("round 2 request LOAD-V1", "Location", "LOAD-V1 row_version",
                            "LOAD-V5 row_version")

    def test_round_one_rejects_the_precheck_container(self):
        self.change_reading("baseline-1", container_id="b" * 64)
        self.change_reading("round-1", container_id="b" * 64)
        self.check_failures("value-precheck and value-baseline-1 use the same container ID")

    def test_round_two_rejects_the_round_one_container(self):
        self.change_reading("baseline-2", container_id="c" * 64)
        self.change_reading("round-2", container_id="c" * 64)
        self.check_failures("value-round-1 and value-baseline-2 use the same container ID")

    def test_round_two_container_must_match_its_baseline(self):
        self.change_reading("round-2", container_id="e" * 64)
        self.check_failures("round 2 container ID differs from its baseline")

    def test_rounds_require_the_same_four_distinct_draft_keys(self):
        for key in (self.keys[1], self.keys[4]):
            with self.subTest(key=key):
                self.change_answer(2, key=key)
                self.check_failures("round 2 does not use the four draft keys from the state file")

    def test_final_check_rejects_insufficient_anon_and_a_wrong_baseline(self):
        batch = self.read("answers-1")
        batch["barrier_anon"] -= 1
        self.write("answers-1", batch)
        self.change_reading("baseline-2", anon=str(REFERENCE_ANON + 1))
        self.check_failures("round 1: the bodies did not overlap", "round 2 reference_anon")


if __name__ == "__main__":
    unittest.main()
