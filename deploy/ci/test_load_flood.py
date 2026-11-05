"""Check the flood gates without Docker or a network connection."""

import contextlib
import importlib.util
import io
import json
import os
import pathlib
import subprocess
import sys
import tempfile
import unittest
import urllib.parse
from unittest.mock import patch


# Model the default JSON output of oha v1.16.0, without success breakdown.
# https://github.com/hatoo/oha/blob/v1.16.0/src/printer.rs
# Summary, histogram, percentile, and detail times use seconds.
# Only metrics.latency_ms uses milliseconds.
OHA_JSON = r'''{
  "summary": {"successRate": 1.0, "total": 4.0, "slowest": 3.0,
    "fastest": 0.5, "average": 2.0, "requestsPerSec": 50.0,
    "totalData": 20000, "sizePerRequest": 100, "sizePerSec": 5000.0},
  "metrics": {"success_rate": 1.0, "requests_per_sec": 50.0,
    "latency_ms": {"min": 500.0, "mean": 2000.0, "p50": 2000.0,
      "p95": 2500.0, "p99": 2750.0, "max": 3000.0}},
  "responseTimeHistogram": {"0.5": 10, "2": 90, "3": 100},
  "latencyPercentiles": {"p10": 0.5, "p25": 1.0, "p50": 2.0,
    "p75": 2.25, "p90": 2.4, "p95": 2.5, "p99": 2.75,
    "p99.9": 3.0, "p99.99": 3.0},
  "firstByteHistogram": {"0.25": 10, "1": 90, "2": 100},
  "firstBytePercentiles": {"p10": 0.25, "p25": 0.5, "p50": 1.0,
    "p75": 1.25, "p90": 1.4, "p95": 1.5, "p99": 1.75,
    "p99.9": 2.0, "p99.99": 2.0},
  "rps": {"mean": 50.0, "stddev": null, "max": 80.0, "min": 20.0,
    "percentiles": {"p10": 20.0, "p25": 30.0, "p50": 50.0,
      "p75": 60.0, "p90": 70.0, "p95": 75.0, "p99": 80.0,
      "p99.9": 80.0, "p99.99": 80.0}},
  "details": {"DNSDialup": {"average": 0.02, "fastest": 0.01, "slowest": 0.03},
    "DNSLookup": {"average": 0.002, "fastest": 0.001, "slowest": 0.003},
    "firstByte": {"average": 1.0, "fastest": 0.25, "slowest": 2.0}},
  "statusCodeDistribution": {"401": 80, "429": 20, "503": 100},
  "errorDistribution": {}
}'''


def fixture(section=None, **changes):
    document = json.loads(OHA_JSON)
    if section:
        document[section].update(changes)
    return document


class FloodTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        path = pathlib.Path(__file__).with_name("load_flood.py")
        if not path.exists():
            raise AssertionError("load_flood.py does not exist")
        spec = importlib.util.spec_from_file_location("load_flood", path)
        cls.flood = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(cls.flood)

    def reject(self, document, message, count=100):
        with self.assertRaisesRegex(self.flood.FloodError, message):
            self.flood.check_flood(document, count)

    def test_reports_statuses_and_completed_logins_per_second(self):
        report = self.flood.check_flood(fixture(), 100)
        self.assertEqual(report["status_counts"], {"401": 80, "429": 20, "503": 100})
        self.assertEqual(report["logins_per_second"], 25.0)
        self.assertEqual(report["total_seconds"], 4.0)
        self.assertEqual(report["slowest_seconds"], 3.0)
        self.assertEqual(report["p99_seconds"], 2.75)

    def test_missing_429_reports_zero(self):
        document = fixture()
        document["statusCodeDistribution"] = {"401": 100, "503": 100}
        report = self.flood.check_flood(document, 100)
        self.assertEqual(report["status_counts"]["429"], 0)
        self.assertEqual(report["logins_per_second"], 25.0)

    def test_answer_count_below_200_fails(self):
        self.reject(fixture("statusCodeDistribution", **{"401": 79}), "exactly 200 answers")

    def test_answer_count_above_200_fails(self):
        self.reject(fixture("statusCodeDistribution", **{"401": 81}), "exactly 200 answers")

    def test_slowest_above_five_seconds_fails(self):
        self.reject(fixture("summary", slowest=5.001), "slowest request exceeds 5 s")

    def test_five_seconds_passes(self):
        self.flood.check_flood(fixture("summary", slowest=5.0), 100)

    def test_error_distribution_fails_without_printing_its_text(self):
        self.reject(fixture("errorDistribution", **{"private-value": 1}), "^oha reported errors$")

    def test_no_503_fails(self):
        document = fixture()
        document["statusCodeDistribution"] = {"401": 100, "429": 100}
        self.reject(document, "no 503 answer", 0)

    def test_unexpected_status_fails(self):
        document = fixture()
        document["statusCodeDistribution"] = {"401": 79, "429": 20, "503": 100, "403": 1}
        self.reject(document, "unexpected HTTP status: 403")

    def test_permit_log_count_must_equal_503_count(self):
        for count in (99, 101):
            with self.subTest(count=count):
                self.reject(fixture(), "password hash permit count differs from 503 count", count)

    def test_each_time_field_has_a_finite_value(self):
        paths = [("summary", key) for key in ("total", "slowest", "fastest", "average")]
        for section in ("latencyPercentiles", "firstBytePercentiles"):
            paths.extend((section, key) for key in fixture()[section])
        paths.extend(("metrics", "latency_ms", key) for key in fixture()["metrics"]["latency_ms"])
        for detail in ("DNSDialup", "DNSLookup", "firstByte"):
            paths.extend(("details", detail, key) for key in ("average", "fastest", "slowest"))
        for path in paths:
            for value in (float("nan"), float("inf"), -float("inf"), None):
                with self.subTest(path=path, value=value):
                    document = fixture()
                    target = document
                    for key in path[:-1]:
                        target = target[key]
                    target[path[-1]] = value
                    self.reject(document, "finite")

    def test_histogram_time_keys_must_be_finite(self):
        for section in ("responseTimeHistogram", "firstByteHistogram"):
            document = fixture()
            document[section] = {"NaN": 200}
            self.reject(document, "finite")

    def test_time_units_of_every_field(self):
        times = dict(self.flood.time_fields(fixture()))
        expected = {
            "summary.total": 4.0, "summary.slowest": 3.0,
            "summary.fastest": 0.5, "summary.average": 2.0,
            "metrics.latency_ms.min": 0.5, "metrics.latency_ms.mean": 2.0,
            "metrics.latency_ms.p50": 2.0, "metrics.latency_ms.p95": 2.5,
            "metrics.latency_ms.p99": 2.75, "metrics.latency_ms.max": 3.0,
            "responseTimeHistogram.0.5": 0.5, "responseTimeHistogram.2": 2.0,
            "responseTimeHistogram.3": 3.0, "firstByteHistogram.0.25": 0.25,
            "firstByteHistogram.1": 1.0, "firstByteHistogram.2": 2.0,
        }
        percentiles = ("p10", "p25", "p50", "p75", "p90", "p95", "p99", "p99.9", "p99.99")
        for section, values in (
            ("latencyPercentiles", (0.5, 1, 2, 2.25, 2.4, 2.5, 2.75, 3, 3)),
            ("firstBytePercentiles", (0.25, 0.5, 1, 1.25, 1.4, 1.5, 1.75, 2, 2)),
        ):
            expected.update((f"{section}.{key}", value) for key, value in zip(percentiles, values))
        for detail, values in (
            ("DNSDialup", (0.02, 0.01, 0.03)), ("DNSLookup", (0.002, 0.001, 0.003)),
            ("firstByte", (1.0, 0.25, 2.0)),
        ):
            expected.update((f"details.{detail}.{key}", value)
                            for key, value in zip(("average", "fastest", "slowest"), values))
        self.assertEqual(times, expected)

    def test_missing_or_extra_schema_fields_fail(self):
        for section in (None, "summary", "metrics", "latencyPercentiles", "rps", "details"):
            for change in ("missing", "extra"):
                document = fixture()
                target = document if section is None else document[section]
                if change == "missing":
                    del target[next(iter(target))]
                else:
                    target["unknown"] = 1
                with self.subTest(section=section, change=change):
                    self.reject(document, "oha schema")

    def test_wrong_schema_types_fail(self):
        for section, key, value in (
            ("summary", "total", "4s"), ("summary", "totalData", True),
            ("statusCodeDistribution", "401", "80"),
            ("statusCodeDistribution", "401", -1), ("rps", "mean", []),
        ):
            with self.subTest(section=section, key=key):
                self.reject(fixture(section, **{key: value}), "oha schema")

    def test_zero_or_negative_total_fails(self):
        for value in (0, -1):
            self.reject(fixture("summary", total=value), "total time must be positive")

    def test_log_count_requires_json_code_and_permit_stage(self):
        chain = "The request took too long and was stopped. Try again.: stage password hash permit"
        entry = {
            "timestamp": "2026-11-05T13:50:01.123Z", "level": "ERROR",
            "fields": {"message": "request failed", "error_code": "E-WEB-009",
                       "request_id": "0b6c...", "chain": chain},
            "target": "recipe_web::error::log",
            "span": {"route": "/login", "name": "request"},
        }
        matching = json.dumps(entry)
        self.assertEqual(self.flood.permit_log_count((matching + "\n") * 100), 100)
        rejected = [
            {**entry, "fields": {**entry["fields"], "chain": chain.replace("password hash permit", "request")}},
            {**entry, "fields": {**entry["fields"], "error_code": "E-WEB-001"}},
            {**entry, "fields": {**entry["fields"], "chain": chain + " extra"}},
            {**entry, "fields": {**entry["fields"], "chain": None}},
            {**entry, "fields": {"error_code": "E-WEB-009"}},
            {**entry, "fields": [entry["fields"]]},
            {**entry, "fields": None},
            {}, [entry], 9,
        ]
        lines = [json.dumps(value) for value in rejected]
        lines += ['E-WEB-009 cause="password hash permit"', "startup text", ""]
        for line in lines:
            with self.subTest(line=line):
                self.assertEqual(self.flood.permit_log_count(line), 0)
        self.assertEqual(self.flood.permit_log_count("\n".join([matching] + lines)), 1)

    def invoke(self, *args):
        output = io.StringIO()
        with contextlib.redirect_stdout(output):
            self.flood.main(list(args))
        return output.getvalue()

    def test_runner_checks_each_condition_at_its_boundary(self):
        with tempfile.TemporaryDirectory() as directory:
            root = pathlib.Path(directory)
            meminfo = root / "meminfo"
            controllers = root / "cgroup.controllers"
            for cpus, kib, v2, message in (
                (4, 8388608, True, None), (8, 16777216, True, None),
                (3, 8388608, True, "at least 4 vCPU"),
                (4, 8388607, True, "at least 8 GiB"),
                (4, 8388608, False, "cgroup v2"),
                (None, 8388608, True, "vCPU"),
            ):
                meminfo.write_text(f"MemTotal: {kib} kB\nMemFree: 1 kB\n", encoding="ascii")
                controllers.write_text("cpu memory\n", encoding="ascii")
                if not v2:
                    controllers.unlink()
                with self.subTest(cpus=cpus, kib=kib, v2=v2), patch.object(self.flood.os, "cpu_count", return_value=cpus):
                    args = ("runner", "--meminfo", str(meminfo), "--cgroup-root", str(root))
                    if message:
                        with self.assertRaisesRegex(self.flood.FloodError, message):
                            self.invoke(*args)
                    else:
                        self.assertIn("runner check passed", self.invoke(*args))

    def test_preflight_checks_each_condition(self):
        with tempfile.TemporaryDirectory() as directory:
            root = pathlib.Path(directory)
            cpu, memory, failures = [root / name for name in ("cpu.max", "memory.max", "failures")]
            cases = (
                ("100000 100000", "536870912", "0", None),
                ("50000 50000", "536870912", "0", None),
                ("200000 100000", "536870912", "0", "CPU limit must be 1"),
                ("50000 100000", "536870912", "0", "CPU limit must be 1"),
                ("max 100000", "536870912", "0", "CPU limit must be 1"),
                ("0 0", "536870912", "0", "CPU limit must be 1"),
                ("100000 100000", "536870911", "0", "memory.max must be 512 MiB"),
                ("100000 100000", "536870913", "0", "memory.max must be 512 MiB"),
                ("100000 100000", "max", "0", "memory.max must be 512 MiB"),
                ("100000 100000", "536870912", "1", "failed login count must be 0"),
                ("100000 100000", "536870912", "unavailable", "failed login count must be 0"),
            )
            for cpu_value, memory_value, count, message in cases:
                for path, value in ((cpu, cpu_value), (memory, memory_value), (failures, count)):
                    path.write_text(value + "\n", encoding="ascii")
                with self.subTest(cpu=cpu_value, memory=memory_value, count=count):
                    args = ("preflight", str(cpu), str(memory), str(failures))
                    if message:
                        with self.assertRaisesRegex(self.flood.FloodError, message):
                            self.invoke(*args)
                    else:
                        self.assertIn("preflight check passed", self.invoke(*args))

    def test_body_has_2000_random_encoded_logins_and_one_token(self):
        with tempfile.TemporaryDirectory() as directory:
            root = pathlib.Path(directory)
            token, password, body = [root / name for name in ("csrf_pre", "password", "body")]
            token.write_text("a+b/c=& token\n", encoding="ascii")
            password.write_text("not+a&real=password\n", encoding="ascii")
            output = self.invoke("body", str(token), str(password), str(body))
            lines = body.read_text(encoding="ascii").splitlines()
            self.assertEqual(len(lines), 2000)
            logins = set()
            for line in lines:
                self.assertNotIn(" ", line)
                fields = urllib.parse.parse_qs(line, strict_parsing=True)
                self.assertEqual(set(fields), {"login", "password", "csrf_pre"})
                self.assertEqual(fields["csrf_pre"], ["a+b/c=& token"])
                self.assertEqual(fields["password"], ["not+a&real=password"])
                self.assertRegex(fields["login"][0], r"\A[a-z0-9._-]{3,64}\Z")
                logins.add(fields["login"][0])
            self.assertEqual(len(logins), 2000)
            self.invoke("body", str(token), str(password), str(body))
            self.assertNotEqual(lines, body.read_text(encoding="ascii").splitlines())
            self.assertNotIn(password.read_text().strip(), output)
            self.assertNotIn(token.read_text().strip(), output)

    def test_check_cli_writes_metrics_and_fails_safely(self):
        with tempfile.TemporaryDirectory() as directory:
            root = pathlib.Path(directory)
            source, logs, report, summary = [root / name for name in ("oha.json", "app.log", "report.json", "summary")]
            line = json.dumps({"fields": {"error_code": "E-WEB-009", "chain":
                "The request took too long and was stopped. Try again.: stage password hash permit"}})
            logs.write_text((line + "\n") * 100, encoding="ascii")
            for document, status, message in (
                (fixture(), 0, ""),
                (fixture("errorDistribution", **{"private-response": 1}), 1, "oha reported errors"),
            ):
                source.write_text(json.dumps(document), encoding="ascii")
                result = subprocess.run(
                    [sys.executable, "-I", "-B", self.flood.__file__, "check", str(source),
                     str(logs), "--output", str(report)],
                    env={**os.environ, "GITHUB_STEP_SUMMARY": str(summary)},
                    capture_output=True, text=True, check=False,
                )
                self.assertEqual(result.returncode, status, result.stderr)
                self.assertIn(message, result.stderr)
                self.assertNotIn("private-response", result.stdout + result.stderr + report.read_text())
                self.assertEqual(json.loads(report.read_text())["logins_per_second"], 25.0)
                self.assertIn("429=20", summary.read_text())

    def test_bad_json_cli_fails_without_echoing_input(self):
        with tempfile.TemporaryDirectory() as directory:
            source = pathlib.Path(directory) / "bad.json"
            source.write_text('{"private-value":', encoding="ascii")
            result = subprocess.run(
                [sys.executable, "-I", "-B", self.flood.__file__, "check", str(source), str(source)],
                capture_output=True, text=True, check=False,
            )
            self.assertEqual(result.returncode, 1)
            self.assertNotIn("private-value", result.stderr)
            self.assertIn("flood input", result.stderr)


if __name__ == "__main__":
    unittest.main()
