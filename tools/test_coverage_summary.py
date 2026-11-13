"""Check the line results, the summary, and the exit precedence of the coverage gate (design D2, D3)."""

import hashlib
import importlib.util
import json
import pathlib
import re
import unittest


def load_shared_helpers():
    """Load the shared test helpers by path, because `python3 -I` drops the script directory."""
    path = pathlib.Path(__file__).with_name("test_coverage_gate.py").resolve()
    spec = importlib.util.spec_from_file_location("test_coverage_gate_helpers", path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


HELPERS = load_shared_helpers()
WorkspaceTestCase = HELPERS.WorkspaceTestCase
PRODUCTION = HELPERS.PRODUCTION
PERCENT_LINE = r"\d+\.\d\d"
LIB = "crates/sample/src/lib.rs"
OTHER = "crates/sample/src/other.rs"
GOAL_CLAIM = re.compile(r"100 %[^\n]*(met|reached|complete|achieved)|goal[^\n]*(met|reached)", re.IGNORECASE)


def section(output, title):
    """Return the lines under a summary title up to the next blank line. Return None without the title."""
    lines = output.splitlines()
    starts = [i for i, line in enumerate(lines) if title in line]
    if not starts:
        return None
    body = []
    for line in lines[starts[0] + 1:]:
        if not line.strip():
            break
        body.append(line)
    return body


class SummaryTestCase(WorkspaceTestCase):
    """A one-crate workspace. Each test writes the report counters as literals."""

    def setUp(self):
        super().setUp()
        self.crate({"lib.rs": PRODUCTION})

    def check_lib(self, count, covered, exit_code):
        self.coverage([(LIB, count, covered)])
        return self.assert_exit(self.run_gate(self.report), exit_code)


class LineFloorTests(SummaryTestCase):
    def test_exactly_eighty_percent_passes(self):
        output = self.check_lib(100, 80, 0)
        self.assertRegex(output, r"sample\s+80\s+100\s+80\.00\s+20\s+ok")

    def test_just_below_eighty_percent_fails(self):
        output = self.check_lib(100, 79, 1)
        self.assertRegex(output, r"sample\s+79\s+100\s+79\.00\s+21\s+BELOW TARGET")

    def test_eight_hundred_of_one_thousand_passes(self):
        output = self.check_lib(1000, 800, 0)
        self.assertRegex(output, r"sample\s+800\s+1000\s+80\.00\s+200\s+ok")

    def test_seven_hundred_ninety_nine_of_one_thousand_fails(self):
        output = self.check_lib(1000, 799, 1)
        self.assertRegex(output, r"sample\s+799\s+1000\s+79\.90\s+201\s+BELOW TARGET")

    def test_fully_covered_crate_shows_zero_uncovered_lines(self):
        output = self.check_lib(10, 10, 0)
        self.assertRegex(output, r"sample\s+10\s+10\s+100\.00\s+0\s+ok")


class CrateRowTests(WorkspaceTestCase):
    def test_each_crate_row_shows_covered_total_percent_and_uncovered(self):
        self.crate({"lib.rs": PRODUCTION}, name="alpha")
        self.crate({"lib.rs": PRODUCTION}, name="beta")
        self.crate({}, name="gamma")
        self.coverage([("crates/alpha/src/lib.rs", 40, 36), ("crates/beta/src/lib.rs", 8, 8)])
        output = self.assert_exit(self.run_gate(self.report), 0)
        self.assertRegex(output, r"alpha\s+36\s+40\s+90\.00\s+4\s+ok")
        self.assertRegex(output, r"beta\s+8\s+8\s+100\.00\s+0\s+ok")
        self.assertRegex(output, r"gamma[^\n]*no code")

    def test_failing_crate_row_shows_its_uncovered_lines(self):
        self.crate({"lib.rs": PRODUCTION}, name="alpha")
        self.crate({"lib.rs": PRODUCTION}, name="beta")
        self.coverage([("crates/alpha/src/lib.rs", 40, 30), ("crates/beta/src/lib.rs", 8, 8)])
        output = self.assert_exit(self.run_gate(self.report), 1)
        self.assertRegex(output, r"alpha\s+30\s+40\s+75\.00\s+10\s+BELOW TARGET")
        self.assertRegex(output, r"beta\s+8\s+8\s+100\.00\s+0\s+ok")


class RequiredFileTests(WorkspaceTestCase):
    TITLE = "required files with uncovered lines"

    def setUp(self):
        super().setUp()
        self.crate({"lib.rs": PRODUCTION, "other.rs": PRODUCTION})

    def test_summary_lists_each_required_file_with_uncovered_lines(self):
        self.coverage([(LIB, 100, 90), (OTHER, 50, 50)])
        output = self.assert_exit(self.run_gate(self.report), 0)
        body = section(output, self.TITLE)
        self.assertIsNotNone(body, output)
        self.assertEqual(len(body), 1, body)
        self.assertRegex(body[0], r"^crates/sample/src/lib\.rs\s+10 uncovered lines$")

    def test_summary_lists_two_files_with_their_own_counts(self):
        self.coverage([(LIB, 100, 95), (OTHER, 50, 40)])
        output = self.assert_exit(self.run_gate(self.report), 0)
        body = "\n".join(section(output, self.TITLE) or [])
        self.assertRegex(body, r"crates/sample/src/lib\.rs\s+5 uncovered lines")
        self.assertRegex(body, r"crates/sample/src/other\.rs\s+10 uncovered lines")

    def test_fully_covered_files_and_empty_list_are_not_shown(self):
        self.coverage([(LIB, 10, 10), (OTHER, 10, 10)])
        output = self.assert_exit(self.run_gate(self.report), 0)
        self.assertNotIn(self.TITLE, output)
        self.assertNotIn("uncovered lines", output)

    def test_failing_run_lists_the_files_that_cause_the_deficit(self):
        self.coverage([(LIB, 100, 10), (OTHER, 100, 100)])
        output = self.assert_exit(self.run_gate(self.report), 1)
        body = section(output, self.TITLE)
        self.assertEqual(len(body or []), 1, output)
        self.assertRegex(body[0], r"lib\.rs\s+90 uncovered lines")


class ListedPathTests(WorkspaceTestCase):
    """The summary holds the four lists of design D3."""

    def setUp(self):
        super().setUp()
        self.crate({"lib.rs": "mod names;\nmod other;\n" + PRODUCTION,
                    "names.rs": "pub const A: u8 = 1;\n",
                    "other.rs": PRODUCTION})
        digest = hashlib.sha256(PRODUCTION.encode()).hexdigest()
        entry = {"path": OTHER, "sha256": digest, "configuration": "windows",
                 "reason": "Compiled on Windows only.", "evidence": "other.rs line 1: cfg(windows)"}
        self.write("tools/coverage-policy.json", json.dumps({"schema_version": 1, "entries": [entry]}))

    def report_with(self, extra):
        files = [(str(self.root / LIB), 10, 10), *extra]
        entries = [{"filename": name, "summary": {"lines": {"count": count, "covered": covered}}}
                   for name, count, covered in files]
        self.report.write_text(json.dumps({"data": [{"files": entries}]}), encoding="utf-8")

    def test_summary_lists_all_four_kinds_of_path(self):
        self.report_with([(str(self.root / "crates/sample/build.rs"), 37, 0),
                          ("/opt/vendor/lib.rs", 100, 100)])
        output = self.assert_exit(self.run_gate(self.report), 0)
        expected = {
            "declaration-only files": r"crates/sample/src/names\.rs",
            "other-configuration files": r"crates/sample/src/other\.rs \(windows\)",
            "outside the metric": r"crates/sample/build\.rs\s+37 lines",
            "third-party code": r"/opt/vendor/lib\.rs\s+100 lines",
        }
        for title, pattern in expected.items():
            with self.subTest(title):
                body = section(output, title)
                self.assertIsNotNone(body, output)
                self.assertRegex("\n".join(body), pattern)

    def test_ci_form_keeps_the_configuration_of_an_other_configuration_file(self):
        self.report_with([])
        inventory = self.root / "inventory.json"
        self.assert_exit(self.run_gate("--write-inventory", inventory), 0)
        stored = json.loads(inventory.read_text(encoding="utf-8"))
        configurations = [f.get("configuration") for f in stored["files"] if f["path"] == OTHER]
        self.assertEqual(configurations, ["windows"])
        output = self.assert_exit(self.run_gate("--report", self.report, "--inventory", inventory), 0)
        self.assertRegex("\n".join(section(output, "other-configuration files") or []),
                         r"crates/sample/src/other\.rs \(windows\)")

    def test_declaration_only_file_with_reported_lines_moves_to_the_required_list(self):
        self.report_with([(str(self.root / "crates/sample/src/names.rs"), 10, 0)])
        output = self.assert_exit(self.run_gate(self.report), 1)
        self.assertIsNone(section(output, "declaration-only files"), output)
        body = section(output, "required files with uncovered lines")
        self.assertRegex("\n".join(body or []), r"names\.rs\s+10 uncovered lines")

    def test_each_list_is_absent_when_it_has_no_path(self):
        self.crate({"lib.rs": PRODUCTION})
        self.coverage([(LIB, 10, 10)])
        self.git("rm", "-q", "--cached", "crates/sample/src/names.rs", "crates/sample/src/other.rs")
        (self.root / "crates/sample/src/names.rs").unlink()
        (self.root / "crates/sample/src/other.rs").unlink()
        self.write("tools/coverage-policy.json", '{"schema_version": 1, "entries": []}')
        output = self.assert_exit(self.run_gate(self.report), 0)
        for title in ("declaration-only files", "other-configuration files",
                      "outside the metric", "third-party code"):
            with self.subTest(title):
                self.assertNotIn(title, output)


class GoalClaimTests(WorkspaceTestCase):
    def setUp(self):
        super().setUp()
        self.crate({"lib.rs": PRODUCTION, "other.rs": PRODUCTION})

    def test_passing_run_with_uncovered_lines_does_not_claim_the_goal(self):
        self.coverage([(LIB, 10, 9), (OTHER, 10, 10)])
        output = self.assert_exit(self.run_gate(self.report), 0)
        self.assertNotRegex(output, GOAL_CLAIM)

    def test_failing_run_does_not_claim_the_goal(self):
        self.coverage([(LIB, 10, 1), (OTHER, 10, 10)])
        output = self.assert_exit(self.run_gate(self.report), 1)
        self.assertNotRegex(output, GOAL_CLAIM)
        self.assertNotIn("every crate meets", output)

    def test_missing_required_file_does_not_claim_the_goal(self):
        self.coverage([(LIB, 10, 10)])
        output = self.assert_exit(self.run_gate(self.report), 1)
        self.assertIn("other.rs", output)
        self.assertNotRegex(output, GOAL_CLAIM)
        self.assertNotIn("every crate meets", output)

    def test_zero_line_required_file_does_not_claim_the_goal(self):
        self.coverage([(LIB, 10, 10), (OTHER, 0, 0)])
        output = self.assert_exit(self.run_gate(self.report), 1)
        self.assertNotRegex(output, GOAL_CLAIM)
        self.assertNotIn("every crate meets", output)


class BuildScriptTests(WorkspaceTestCase):
    TITLE = "build-time code outside the metric, not counted"

    def test_summary_names_a_build_script_and_keeps_the_crate_totals(self):
        self.crate({"lib.rs": PRODUCTION})
        self.write("crates/sample/build.rs", PRODUCTION)
        self.coverage([(LIB, 10, 10)])
        output = self.assert_exit(self.run_gate(self.report), 0)
        self.assertEqual(section(output, self.TITLE), ["crates/sample/build.rs"])
        self.assertRegex(output, r"sample\s+10\s+10\s+100\.00\s+0\s+ok")


class ExitPrecedenceTests(WorkspaceTestCase):
    def setUp(self):
        super().setUp()
        self.crate({"lib.rs": PRODUCTION})

    def write_report(self, entries):
        files = [{"filename": name, "summary": {"lines": counters}} for name, counters in entries]
        self.report.write_text(json.dumps({"data": [{"files": files}]}), encoding="utf-8")

    def assert_input_error(self):
        output = self.assert_exit(self.run_gate(self.report), 2)
        self.assertNotRegex(output, PERCENT_LINE)
        self.assertNotIn("every crate meets", output)
        self.assertNotIn("BELOW TARGET", output)
        return output

    def test_unknown_source_path_and_a_deficit_give_exit_two(self):
        self.write_report([(str(self.root / LIB), {"count": 10, "covered": 1}),
                           (str(self.root / "crates/sample/src/ghost.rs"), {"count": 5, "covered": 5})])
        self.assert_input_error()

    def test_bad_counter_in_a_later_entry_and_a_deficit_give_exit_two(self):
        self.write_report([(str(self.root / LIB), {"count": 100, "covered": 1}),
                           ("/opt/vendor/lib.rs", {"count": 5, "covered": 9})])
        self.assert_input_error()

    def test_input_error_beats_an_untracked_source_file_and_a_deficit(self):
        self.write("crates/sample/src/loose.rs", PRODUCTION, track=False)
        self.write_report([(str(self.root / LIB), {"count": 10, "covered": 1}),
                           (str(self.root / LIB), {"count": 10, "covered": 1})])
        self.assert_input_error()

    def test_unused_policy_entry_and_a_deficit_give_exit_two(self):
        self.crate({"lib.rs": "mod other;\n" + PRODUCTION, "other.rs": PRODUCTION})
        digest = hashlib.sha256(PRODUCTION.encode()).hexdigest()
        entry = {"path": OTHER, "sha256": digest, "configuration": "windows",
                 "reason": "Compiled on Windows only.", "evidence": "other.rs line 1: cfg(windows)"}
        self.write("tools/coverage-policy.json", json.dumps({"schema_version": 1, "entries": [entry]}))
        self.write_report([(str(self.root / LIB), {"count": 10, "covered": 1}),
                           (str(self.root / OTHER), {"count": 10, "covered": 10})])
        self.assertNotRegex(self.assert_exit(self.run_gate(self.report), 2), PERCENT_LINE)

    def test_untracked_source_file_with_a_deficit_gives_exit_one(self):
        self.write("crates/sample/src/loose.rs", PRODUCTION, track=False)
        self.write_report([(str(self.root / LIB), {"count": 10, "covered": 1})])
        self.assert_exit(self.run_gate(self.report), 1)

    def test_untracked_source_file_alone_gives_exit_one(self):
        self.write("crates/sample/src/loose.rs", PRODUCTION, track=False)
        self.write_report([(str(self.root / LIB), {"count": 10, "covered": 10})])
        self.assert_exit(self.run_gate(self.report), 1)


class DocstringTests(unittest.TestCase):
    def test_docstring_holds_the_exit_code_table_of_design_d2(self):
        doc = HELPERS.load_gate().__doc__
        rows = {code: next((line for line in doc.splitlines()
                            if re.match(rf"\s*\|?\s*{code}\s*\|", line)), "") for code in (0, 1, 2)}
        self.assertIn("pass", rows[0].lower())
        self.assertIn("coverage deficit", rows[1].lower())
        self.assertIn("incomplete", rows[1].lower())
        self.assertIn("input or policy error", rows[2].lower())
        self.assertIn("Exit 2 wins over exit 1", doc)


if __name__ == "__main__":
    unittest.main()
