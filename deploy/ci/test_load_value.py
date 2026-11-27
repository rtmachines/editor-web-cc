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

    def poll_round(self, samples, *, number=2, deadline=8.0, missing_reference=None):
        clock = SimpleNamespace(now=0.0, sleeps=[], reads=[], releases=[])
        drafts = [self.module.DraftBody(number, draft, b"body")
                  for draft in self.drafts[:4]]
        threshold = REFERENCE_ANON + sum(len(draft.bytes) for draft in drafts)
        samples = iter(samples)

        def sleep(seconds):
            clock.sleeps.append((clock.now, seconds))
            clock.now += seconds

        def read(command, *, timeout, **options):
            clock.reads.append((clock.now, timeout))
            duration, margin = next(samples, (0, None))
            if duration is None:
                clock.now += timeout
                raise subprocess.TimeoutExpired(command, timeout)
            clock.now += duration
            if margin is None:
                return subprocess.CompletedProcess(command, 0, b"file 90000000\n")
            self.set_anon(threshold + margin)
            return self.read_stat(command, timeout=timeout, **options)

        def send(draft, _start, barrier):
            if draft.reference == missing_reference:
                return
            barrier.arrive(draft.reference)
            clock.releases.append(clock.now)

        args = SimpleNamespace(round=number, deadline=deadline, cgroup=self.state,
                               reference_anon=REFERENCE_ANON)
        self.samples.side_effect = read
        fake_time = SimpleNamespace(monotonic=lambda: clock.now, sleep=sleep)
        with (patch.object(self.module, "time", fake_time),
              patch.object(self.module, "mem_available", return_value=10_000)):
            report, failure = self.module.run_round(args, SimpleNamespace(send=send), drafts)
        return report, failure, threshold, clock

    def assert_window(self, result, peak_margin, end):
        report, failure, threshold, clock = result
        self.assertIsNone(failure)
        self.assertEqual(clock.releases, [end] * 4)
        self.assertEqual(report["barrier_anon"], threshold)
        expected_max = None if peak_margin is None else threshold + peak_margin
        self.assertEqual(report["barrier_anon_max"], expected_max)
        self.assertNotIn("overlap_margin_max", report)

    def assert_window_budgets(self, clock, end, sleeps):
        for start, timeout in clock.reads[1:]:
            self.assertLess(start, end)
            self.assertAlmostEqual(timeout, end - start)
        self.assertEqual(len(clock.sleeps), len(sleeps))
        for (start, seconds), expected in zip(clock.sleeps, sleeps):
            self.assertAlmostEqual(seconds, expected)
            self.assertLessEqual(start + seconds, end)

    def test_round_two_records_the_largest_anon_until_the_window_ends(self):
        result = self.poll_round([(0.25, 0), (0, 4096), (0, 0), (0.82, 0)])
        self.assert_window(result, 4096, 1.25)
        self.assert_window_budgets(result[3], 1.25, (0.05, 0.05, 0.05, 0.03))

    def test_round_two_keeps_the_first_crossing_when_later_reads_are_smaller(self):
        result = self.poll_round([(0, 0), (0, -4096), (0.875, -2048)])
        self.assert_window(result, 0, 1)

    def test_round_two_discards_a_read_completed_after_the_window(self):
        result = self.poll_round([(0, 0), (0, 4096), (1, 8192)])
        self.assert_window(result, 4096, 1.1)

    def test_round_two_releases_at_the_barrier_deadline(self):
        result = self.poll_round([(7.5, 0), (0.44, 4096)])
        self.assert_window(result, 4096, 8)
        self.assert_window_budgets(result[3], 8, (0.05, 0.01))

    def test_round_two_skips_a_failed_read_in_the_window(self):
        result = self.poll_round([(0, 0), (0, None), (0, 4096)])
        self.assert_window(result, 4096, 1)

    def test_round_two_releases_when_a_read_stalls_until_the_window_end(self):
        result = self.poll_round([(0, 0), (0, 4096), (None, None)])
        self.assert_window(result, 4096, 1)
        self.assert_window_budgets(result[3], 1, (0.05, 0.05))

    def test_round_two_keeps_the_barrier_failures(self):
        cases = (
            ([(0, -1)], 8, -1, "round 2: the bodies did not overlap"),
            ([], 8, None, "round 2 barrier_anon is missing or invalid"),
        )
        for samples, deadline, margin, error in cases:
            with self.subTest(error=error):
                report, failure, threshold, clock = self.poll_round(samples, deadline=deadline)
                self.assertEqual(failure, error)
                self.assertEqual(report["barrier_failure"], error)
                self.assertEqual(report["barrier_anon"], None if margin is None else threshold - 1)
                self.assertTrue(all(start < deadline for start, _ in clock.reads))
                self.assertIsNone(report["barrier_anon_max"])

    def test_round_two_names_the_sender_that_missed_the_barrier(self):
        # Put LOAD-V1 last so that an early timeout names a different draft.
        self.drafts[:4] = [*self.drafts[1:4], self.drafts[0]]
        report, failure, _, clock = self.poll_round([], deadline=1, missing_reference="LOAD-V1")
        error = "round 2: request LOAD-V1 did not reach the barrier"
        self.assertEqual(failure, error)
        self.assertEqual(report["barrier_failure"], error)
        self.assertIsNone(report["barrier_anon"])
        self.assertIsNone(report["barrier_anon_max"])
        self.assertEqual(clock.reads, [])
        self.assertEqual(len(clock.releases), 3)

    def test_round_one_releases_at_the_first_crossing(self):
        result = self.poll_round([(0.25, 0), (0, 4096)], number=1)
        self.assert_window(result, None, 0.25)
        self.assertEqual(len(result[3].reads), 1)
        self.assertEqual(result[3].sleeps, [])

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


if __name__ == "__main__":
    unittest.main()
