"""Check load memory decisions without a running stack."""

import importlib.util
import json
import os
import pathlib
import subprocess
import sys
import tempfile
import unittest


SCRIPT = pathlib.Path(__file__).with_name("load_memory.py")


def reading(**changes):
    values = {
        "peak": 419430400, "limit": 536870912, "anon": 408944640,
        "file": 10485760, "oom": 0, "oom_kill": 0, "max": 0,
        "OOMKilled": False, "RestartCount": 0,
    }
    return values | changes


class LoadMemoryTests(unittest.TestCase):
    def setUp(self):
        self.assertTrue(SCRIPT.is_file(), "load_memory.py does not exist")
        spec = importlib.util.spec_from_file_location("load_memory", SCRIPT)
        self.memory = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(self.memory)

    def check_failure(self, fixture, field):
        for phase in ("timings", "flood"):
            with self.subTest(phase=phase):
                pair = {"timings": reading(), "flood": reading()}
                pair[phase] = fixture
                result = self.memory.check_memory(**pair)
                self.assertFalse(result["passed"])
                self.assertTrue(any(phase in error and field in error
                                    for error in result["errors"]), result)

    def test_unavailable_fails_for_each_value(self):
        for field in reading():
            with self.subTest(field=field):
                self.check_failure(reading(**{field: "unavailable"}), field)

    def test_oom_fails(self):
        self.check_failure(reading(oom=1), "oom")

    def test_oom_kill_fails(self):
        self.check_failure(reading(oom_kill=1), "oom_kill")

    def test_max_fails(self):
        self.check_failure(reading(max=1), "max")

    def test_peak_at_or_above_512_mib_fails(self):
        for peak in (536870912, 536870913):
            with self.subTest(peak=peak):
                self.check_failure(reading(peak=peak), "512 MiB")

    def test_docker_oom_killed_fails(self):
        self.check_failure(reading(OOMKilled=True), "OOMKilled")

    def test_restart_fails(self):
        self.check_failure(reading(RestartCount=1), "RestartCount")

    def test_missing_and_nonnumeric_values_fail_without_echoing_values(self):
        for field in reading():
            for value in (None, "PRIVATE-INVALID-VALUE", [], {}):
                with self.subTest(field=field, value=value):
                    fixture = reading(**{field: value})
                    self.check_failure(fixture, field)
                    report = self.memory.check_memory(reading(), fixture)
                    self.assertNotIn("PRIVATE-INVALID-VALUE", json.dumps(report))
            fixture = reading()
            del fixture[field]
            self.check_failure(fixture, field)

    def test_counts_and_bytes_require_nonnegative_integers(self):
        for field in reading().keys() - {"OOMKilled"}:
            for value in (-1, 0.5, True, float("nan"), float("inf"), "1e3"):
                with self.subTest(field=field, value=value):
                    self.check_failure(reading(**{field: value}), field)

    def test_pair_reports_both_peaks_and_maximum_in_either_order(self):
        for timings, flood in ((419430400, 503316480), (503316480, 419430400)):
            with self.subTest(timings=timings):
                result = self.memory.check_memory(reading(peak=timings), reading(peak=flood))
                self.assertTrue(result["passed"], result)
                self.assertEqual(result["timings"]["peak"], timings)
                self.assertEqual(result["flood"]["peak"], flood)
                self.assertEqual(result["peak"], 503316480)
                self.assertEqual(result["errors"], [])

    def test_one_byte_below_limit_and_anon_above_soak_ceiling_pass(self):
        result = self.memory.check_memory(reading(), reading(peak=536870911, anon=419430400))
        self.assertTrue(result["passed"], result)
        self.assertEqual(result["flood"]["anon"], 419430400)
        self.assertEqual(result["soak_memory_ceiling"], 402653184)
        summary = self.memory.format_summary(result)
        self.assertIn("419430400", summary)
        self.assertIn("384 MiB", summary)
        self.assertIn("information only", summary)

    def test_shell_decimal_strings_and_docker_boolean_strings(self):
        fixture = {key: str(value) for key, value in reading().items()}
        fixture["OOMKilled"] = "false"
        self.assertTrue(self.memory.check_memory(fixture, fixture)["passed"])
        fixture["OOMKilled"] = "true"
        self.check_failure(fixture, "OOMKilled")

    def test_flood_memory_condition_met_and_not_met(self):
        for changes in ({}, {"oom_kill": 1}, {"OOMKilled": True},
                        {"RestartCount": 1}, {"peak": 536870912}, {"peak": "unavailable"}):
            with self.subTest(changes=changes):
                result = self.memory.check_memory(reading(), reading(**changes))
                condition = "not met" if changes else "met"
                self.assertEqual(result["adr_0064_o_memory_condition"], condition)
                self.assertIn("ADR-0064 (o) memory condition: " + condition,
                              self.memory.format_summary(result))

    def test_fallback_condition_uses_only_flood_bounds_of_item_ac(self):
        for timings, flood in ((reading(oom_kill=1), reading()),
                               (reading(), reading(oom=1)),
                               (reading(), reading(max=1))):
            with self.subTest(timings=timings, flood=flood):
                result = self.memory.check_memory(timings, flood)
                self.assertFalse(result["passed"])
                self.assertEqual(result["adr_0064_o_memory_condition"], "met")

    def run_cli(self, directory, flood_text):
        timings = directory / "timings.json"
        flood = directory / "flood.json"
        output = directory / "memory.json"
        summary = directory / "summary.md"
        timings.write_text(json.dumps(reading()), encoding="utf-8")
        flood.write_text(flood_text, encoding="utf-8")
        summary.write_text("Existing summary\n", encoding="utf-8")
        result = subprocess.run(
            [sys.executable, "-I", "-B", str(SCRIPT), str(timings), str(flood),
             "--output", str(output)], capture_output=True, text=True, check=False,
            env=os.environ | {"GITHUB_STEP_SUMMARY": str(summary)},
        )
        return result, json.loads(output.read_text(encoding="utf-8")), summary.read_text(encoding="utf-8")

    def test_cli_writes_summary_and_artifact_before_failure_exit(self):
        for changes, status, condition in (({}, 0, "met"), ({"max": 1}, 1, "met"),
                                           ({"oom_kill": 1}, 1, "not met")):
            with self.subTest(changes=changes), tempfile.TemporaryDirectory() as temp:
                result, artifact, summary = self.run_cli(
                    pathlib.Path(temp), json.dumps(reading(peak=503316480, **changes)))
                self.assertEqual(result.returncode, status, result.stderr)
                self.assertEqual(artifact["flood"]["peak"], 503316480)
                self.assertEqual(artifact["peak"], 503316480)
                self.assertTrue(summary.startswith("Existing summary\n"))
                self.assertIn("timings memory.peak: 419430400 bytes", summary)
                self.assertIn("flood memory.peak: 503316480 bytes", summary)
                self.assertIn("whole run memory.peak: 503316480 bytes", summary)
                self.assertIn("ADR-0064 (o) memory condition: " + condition, summary)
                self.assertIn("ADR-0064 (o) memory condition: " + condition, result.stdout)
                if status:
                    self.assertIn(next(iter(changes)), result.stderr)

    def test_cli_rejects_malformed_readings_without_echoing_input(self):
        for text in ('{"peak": "PRIVATE-INVALID-VALUE"', '[]', 'null'):
            with self.subTest(text=text), tempfile.TemporaryDirectory() as temp:
                result, artifact, summary = self.run_cli(pathlib.Path(temp), text)
                self.assertEqual(result.returncode, 1)
                self.assertFalse(artifact["passed"])
                self.assertIn("flood", result.stderr)
                self.assertIn("ADR-0064 (o) memory condition: not met", summary)
                self.assertNotIn("PRIVATE-INVALID-VALUE", result.stdout + result.stderr
                                 + summary + json.dumps(artifact))


if __name__ == "__main__":
    unittest.main()
