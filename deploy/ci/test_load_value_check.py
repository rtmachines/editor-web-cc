"""Check the reports and validation of the value leg."""

import json
import pathlib
import subprocess
import sys
import tempfile
import time
import unittest
import uuid

BODY_BYTES = 16_743_514
WRITE_BYTES = 3_033_541
COOKIE = "__Host-recipe_session=fixture-session"
TOKEN = "fixture-token"
REFERENCE_ANON = 4096


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
            barrier = REFERENCE_ANON + 4 * size
            self.write(f"precheck-{number}", self.answer(number, 4, size))
            self.write(f"answers-{number}", {
                "round": number, "reference_anon": REFERENCE_ANON,
                "barrier_anon": barrier, "barrier_anon_max": barrier if number == 2 else None,
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

    def read_report(self):
        return json.loads((self.state / "load-value.json").read_text(encoding="utf-8"))

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
        batch["barrier_anon_max"] = batch["barrier_anon"]
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
             "reference_anon": 4096, "barrier_anon": 66_978_152, "overlap_margin": 0,
             "barrier_anon_max": None, "overlap_margin_max": None},
            {"round": 2, "container_id": "d" * 64, "peak": self.ceiling - 1,
             "body_bytes": WRITE_BYTES, "run_statistic": 95_663_295.5,
             "mem_available_min_kib": 10_000, "reference_anon": 4096,
             "barrier_anon": 12_138_289, "overlap_margin": 29,
             "barrier_anon_max": 12_138_289, "overlap_margin_max": 29},
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

    def test_round_two_reports_the_max_margin_for_four_unequal_bodies(self):
        batch = self.read("answers-2")
        for answer, extra in zip(batch["answers"], (0, 2, 5, 11), strict=True):
            answer["body_bytes"] += extra
        batch.update(barrier_anon=12_138_281, barrier_anon_max=12_138_319)
        self.write("answers-2", batch)

        self.assertEqual(self.run_check(), "")
        first, second = self.read_report()["rounds"]
        self.assertIsNone(first["barrier_anon_max"])
        self.assertIsNone(first["overlap_margin_max"])
        self.assertEqual(second["overlap_margin"], 3)
        self.assertEqual(second["barrier_anon_max"], 12_138_319)
        self.assertEqual(second["overlap_margin_max"], 41)
        summary = self.summary.read_text(encoding="utf-8").splitlines()
        for line in ("round 1 barrier_anon_max: unavailable bytes",
                     "round 1 overlap_margin_max: unavailable bytes",
                     "round 2 barrier_anon_max: 12138319 bytes",
                     "round 2 overlap_margin_max: 41 bytes"):
            self.assertIn(line, summary)

    def test_round_two_requires_a_max_count_after_crossing(self):
        batch = self.read("answers-2")
        del batch["barrier_anon_max"]
        max_fields = [{}] + [{"barrier_anon_max": value}
                             for value in (None, True, -1, "12138260")]
        for fields in max_fields:
            with self.subTest(fields=fields):
                self.write("answers-2", batch | fields)
                self.summary.write_text("", encoding="utf-8")
                failure = "round 2 barrier_anon_max is missing or invalid"
                self.assertEqual(self.run_check(expected=1).splitlines(), [failure])
                self.assertEqual(self.read_report()["failures"], [failure])
                self.assertIn(failure, self.summary.read_text(encoding="utf-8").splitlines())

    def test_round_two_rejects_a_max_below_the_crossing(self):
        batch = self.read("answers-2")
        batch["barrier_anon_max"] = batch["barrier_anon"] - 1
        self.write("answers-2", batch)

        self.check_failures("round 2 barrier_anon_max must be at least barrier_anon")

    def test_round_two_reports_null_max_values_without_a_crossing(self):
        self.change_answer(2, body_bytes=WRITE_BYTES + 7)

        self.run_check(expected=1)
        for row in self.read_report()["rounds"]:
            with self.subTest(round=row["round"]):
                self.assertIsNone(row["barrier_anon_max"])
                self.assertIsNone(row["overlap_margin_max"])

    def test_round_two_reports_a_negative_overlap_margin_for_unequal_bodies(self):
        batch = self.read("answers-2")
        batch["answers"][0]["body_bytes"] += 7
        batch["barrier_anon_max"] = None
        self.write("answers-2", batch)

        failure = "round 2: the bodies did not overlap"
        self.assertEqual(self.run_check(expected=1).splitlines(), [failure])
        report = self.read_report()
        self.assertEqual(report["failures"], [failure])
        self.assertEqual(report["rounds"][1].get("overlap_margin"), -7)
        summary = self.summary.read_text(encoding="utf-8").splitlines()
        self.assertEqual(summary.count(failure), 1)
        self.assertIn("round 2 overlap_margin: -7 bytes", summary)

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
        batch["barrier_anon_max"] = batch["barrier_anon"]
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
             "barrier_anon": 66_978_152, "overlap_margin": 0,
             "barrier_anon_max": None, "overlap_margin_max": None},
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
