"""Replay archived load runs through the final value checker."""

import importlib.util
import json
import pathlib
import unittest

TEST_DIR = pathlib.Path(__file__).parent
REPLAY_DATA = TEST_DIR / "testdata" / "load-replay"
GREEN_RUNS = ("run-45748785867", "run-45776554175")
MUTATION_B = "run-45768560058"
LEGS = ("load-musl", "load-cc-debian13")
CEILING_FAILURE = "round 2 reading reached the ceiling of 384 MiB"
ROUND_FIELDS = (
    "round", "container_id", "peak", "body_bytes", "run_statistic", "mem_available_min_kib",
)

# Isolated Python does not put the script directory on the import path.
CHECKER_SPEC = importlib.util.spec_from_file_location(
    "test_load_value", TEST_DIR / "test_load_value.py",
)
CHECKER_TESTS = importlib.util.module_from_spec(CHECKER_SPEC)
CHECKER_SPEC.loader.exec_module(CHECKER_TESTS)


class LoadReplayTests(unittest.TestCase):
    def checker_fixture(self):
        fixture = CHECKER_TESTS.ValueCheckerTests()
        self.addCleanup(fixture.doCleanups)
        fixture.setUp()
        return fixture

    def test_archived_runs(self):
        for run in (*GREEN_RUNS, MUTATION_B):
            for leg in LEGS:
                with self.subTest(run=run, leg=leg):
                    self.replay(run, leg)

    def replay(self, run, leg):
        archive = REPLAY_DATA / run / leg
        expected = json.loads((archive / "load-value.json").read_text(encoding="utf-8"))
        memory = json.loads((archive / "load-memory.json").read_text(encoding="utf-8"))
        readings = memory["value"]
        fixture = self.checker_fixture()
        for phase, reading in readings.items():
            fixture.write(f"memory-{phase}", reading)
        for row in expected["rounds"]:
            number, size = row["round"], row["body_bytes"]
            baseline = readings[f"value-baseline-{number}"]["anon"]
            fixture.write(f"answers-{number}", {
                "round": number,
                "reference_anon": baseline,
                "barrier_anon": baseline + 4 * size,
                "mem_available_min_kib": row["mem_available_min_kib"],
                "answers": [fixture.answer(number, index, size) for index in range(4)],
            })

        failures = [CEILING_FAILURE] if run == MUTATION_B else []
        stderr = fixture.run_check(expected=int(bool(failures)))
        actual = json.loads((fixture.state / "load-value.json").read_text(encoding="utf-8"))
        self.assertEqual(actual["passed"], not failures)
        self.assertEqual(actual["failures"], failures)
        self.assertEqual(stderr.splitlines(), failures)
        summary = fixture.summary.read_text(encoding="utf-8").splitlines()
        self.assertEqual(summary.count(CEILING_FAILURE), len(failures))
        self.assertEqual(len(actual["rounds"]), len(expected["rounds"]))
        for actual_row, expected_row in zip(actual["rounds"], expected["rounds"], strict=True):
            self.assertEqual(
                {field: actual_row[field] for field in ROUND_FIELDS},
                {field: expected_row[field] for field in ROUND_FIELDS},
            )

    def test_mutation_a_reports_one_missed_barrier(self):
        """Model the stopped sender from the musl log of run 45755380782."""
        fixture = self.checker_fixture()
        cause = "round 1: request LOAD-V1 did not reach the barrier"
        batch = fixture.read("answers-1")
        batch.update(barrier_anon=None, barrier_failure=cause)
        transport_failures = []
        for answer in batch["answers"]:
            reference = answer["reference"]
            transport_failure = f"round 1 request {reference} failed during transport"
            transport_failures.append(transport_failure)
            answer.update(status=None, code=None, field=None, last_byte_s=None, answer_s=None,
                          error=cause if reference == "LOAD-V1" else transport_failure)
        fixture.write("answers-1", batch)
        for name in ("answers-2", "versions-2", "memory-value-baseline-2", "memory-value-round-2"):
            fixture.path(name).unlink()

        stderr = fixture.run_check(expected=1).splitlines()
        actual = json.loads((fixture.state / "load-value.json").read_text(encoding="utf-8"))
        self.assertFalse(actual["passed"])
        self.assertEqual(actual["failures"], stderr)
        for failure in transport_failures:
            self.assertIn(failure, stderr)
        summary = fixture.summary.read_text(encoding="utf-8").splitlines()
        for lines in (stderr, summary):
            self.assertEqual(lines.count(cause), 1, lines)
            self.assertNotIn("round 1: the bodies did not overlap", lines)


if __name__ == "__main__":
    unittest.main()
