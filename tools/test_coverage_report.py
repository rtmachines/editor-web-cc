"""Check the report shape and the line counters of the coverage gate (design D2)."""

import importlib.util
import json
import pathlib
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
load_gate = HELPERS.load_gate
PERCENT_LINE = r"\d+\.\d\d"


class ReportTestCase(WorkspaceTestCase):
    """A workspace with two production files and a report that each test breaks on purpose."""

    LIB = "crates/sample/src/lib.rs"
    OTHER = "crates/sample/src/other.rs"

    def setUp(self):
        super().setUp()
        self.crate({"lib.rs": PRODUCTION, "other.rs": PRODUCTION})

    def entry(self, filename, count, covered):
        return {"filename": filename, "summary": {"lines": {"count": count, "covered": covered}}}

    def own(self, relative):
        return str(self.root / relative)

    def valid(self):
        return {"data": [{"files": [self.entry(self.own(self.LIB), 10, 10),
                                    self.entry(self.own(self.OTHER), 10, 10)]}]}

    def write_report(self, report):
        self.report.write_text(json.dumps(report), encoding="utf-8")

    def expect_input_error(self, report):
        self.write_report(report)
        output = self.assert_exit(self.run_gate(self.report), 2)
        self.assertNotRegex(output, PERCENT_LINE)
        self.assertNotIn("every crate meets", output)
        return output


class BaselineTests(ReportTestCase):
    def test_valid_report_passes(self):
        self.write_report(self.valid())
        output = self.assert_exit(self.run_gate(self.report), 0)
        self.assertRegex(output, r"sample\s+20\s+20\s+100\.00\s+0\s+ok")


class DuplicateEntryTests(ReportTestCase):
    def test_absolute_and_relative_form_of_one_file_is_an_input_error(self):
        report = self.valid()
        report["data"][0]["files"].append(self.entry(self.LIB, 10, 10))
        output = self.expect_input_error(report)
        self.assertIn(self.LIB, output)

    def test_same_filename_twice_is_an_input_error(self):
        report = self.valid()
        report["data"][0]["files"].append(self.entry(self.own(self.LIB), 10, 10))
        self.expect_input_error(report)

    def test_duplicate_of_a_file_outside_the_metric_is_an_input_error(self):
        report = self.valid()
        report["data"][0]["files"] += [self.entry("build.rs", 3, 3),
                                       self.entry(self.own("build.rs"), 3, 3)]
        self.expect_input_error(report)

    def test_duplicate_of_a_third_party_file_is_an_input_error(self):
        report = self.valid()
        report["data"][0]["files"] += [self.entry("/opt/vendor/lib.rs", 3, 3)] * 2
        self.expect_input_error(report)

    def test_covered_duplicate_cannot_hide_an_uncovered_file(self):
        report = {"data": [{"files": [self.entry(self.own(self.LIB), 100, 100),
                                      self.entry(self.LIB, 100, 100),
                                      self.entry(self.own(self.OTHER), 10, 0)]}]}
        self.expect_input_error(report)


class ExportObjectTests(ReportTestCase):
    def test_two_export_objects_are_an_input_error(self):
        report = self.valid()
        report["data"].append({"files": [self.entry(self.own("crates/sample/other"), 1, 1)]})
        self.expect_input_error(report)

    def test_two_export_objects_with_split_files_are_an_input_error(self):
        files = self.valid()["data"][0]["files"]
        self.expect_input_error({"data": [{"files": files[:1]}, {"files": files[1:]}]})

    def test_zero_export_objects_are_an_input_error(self):
        self.expect_input_error({"data": []})


class MetadataShapeTests(ReportTestCase):
    def test_cargo_llvm_cov_that_is_not_an_object_is_an_input_error(self):
        for value in (None, [], "x", 1):
            with self.subTest(value=value):
                report = self.valid()
                report["cargo_llvm_cov"] = value
                output = self.expect_input_error(report)
                self.assertIn("coverage-gate:", output)

    def test_empty_manifest_path_is_an_input_error(self):
        report = self.valid()
        report["cargo_llvm_cov"] = {"manifest_path": ""}
        output = self.expect_input_error(report)
        self.assertIn("coverage-gate:", output)

    def test_missing_cargo_llvm_cov_key_still_passes(self):
        self.write_report(self.valid())
        self.assert_exit(self.run_gate(self.report), 0)


class MissingKeyTests(ReportTestCase):
    def check_removed(self, mutate):
        report = self.valid()
        mutate(report)
        self.expect_input_error(report)

    def first_entry(self, report):
        return report["data"][0]["files"][0]

    def test_missing_data_is_an_input_error(self):
        self.check_removed(lambda report: report.pop("data"))

    def test_missing_files_is_an_input_error(self):
        self.check_removed(lambda report: report["data"][0].pop("files"))

    def test_missing_filename_is_an_input_error(self):
        self.check_removed(lambda report: self.first_entry(report).pop("filename"))

    def test_missing_summary_is_an_input_error(self):
        self.check_removed(lambda report: self.first_entry(report).pop("summary"))

    def test_missing_lines_is_an_input_error(self):
        self.check_removed(lambda report: self.first_entry(report)["summary"].pop("lines"))

    def test_missing_covered_is_an_input_error(self):
        self.check_removed(lambda report: self.first_entry(report)["summary"]["lines"].pop("covered"))

    def test_missing_count_is_an_input_error(self):
        self.check_removed(lambda report: self.first_entry(report)["summary"]["lines"].pop("count"))

    def test_missing_counter_in_the_last_entry_stops_before_any_percentage(self):
        self.check_removed(lambda report: report["data"][0]["files"][1]["summary"]["lines"].pop("count"))

    def test_wrong_types_are_input_errors(self):
        for name, mutate in {
            "data object": lambda report: report.update(data={"files": []}),
            "export string": lambda report: report.update(data=["files"]),
            "files object": lambda report: report["data"][0].update(files={}),
            "entry string": lambda report: report["data"][0].update(files=["lib.rs"]),
            "filename number": lambda report: self.first_entry(report).update(filename=7),
            "summary list": lambda report: self.first_entry(report).update(summary=[]),
            "lines list": lambda report: self.first_entry(report)["summary"].update(lines=[]),
            "report list": lambda report: report.clear(),
        }.items():
            with self.subTest(name=name):
                self.check_removed(mutate)


class CounterFaultTests(ReportTestCase):
    def check_counters(self, count, covered):
        report = self.valid()
        report["data"][0]["files"][0]["summary"]["lines"] = {"count": count, "covered": covered}
        self.expect_input_error(report)

    def test_boolean_counters_are_input_errors(self):
        for count, covered in ((10, True), (True, 1), (True, True), (10, False), (False, False)):
            with self.subTest(count=count, covered=covered):
                self.check_counters(count, covered)

    def test_negative_counters_are_input_errors(self):
        for count, covered in ((10, -1), (-10, -10), (-1, 0)):
            with self.subTest(count=count, covered=covered):
                self.check_counters(count, covered)

    def test_fractional_counters_are_input_errors(self):
        for count, covered in ((10, 1.5), (10.5, 5), (10.0, 10), (10, 10.0)):
            with self.subTest(count=count, covered=covered):
                self.check_counters(count, covered)

    def test_non_finite_counters_are_input_errors(self):
        nan, inf = float("nan"), float("inf")
        for count, covered in ((10, nan), (nan, 0), (inf, 5), (10, inf), (inf, inf), (10, -inf)):
            with self.subTest(count=count, covered=covered):
                self.check_counters(count, covered)

    def test_non_json_number_literals_are_input_errors(self):
        text = json.dumps(self.valid()).replace('"covered": 10', '"covered": NaN', 1)
        self.assertIn("NaN", text)
        self.report.write_text(text, encoding="utf-8")
        self.assert_exit(self.run_gate(self.report), 2)
        self.report.write_text(text.replace("NaN", "Infinity"), encoding="utf-8")
        self.assert_exit(self.run_gate(self.report), 2)

    def test_other_counter_types_are_input_errors(self):
        for count, covered in ((10, "5"), ("10", 5), (10, None), (None, 0), (10, [5]), ({}, 0)):
            with self.subTest(count=count, covered=covered):
                self.check_counters(count, covered)

    def test_covered_above_count_is_an_input_error(self):
        self.check_counters(10, 11)

    def test_covered_above_count_cannot_lift_a_crate_over_the_floor(self):
        report = {"data": [{"files": [self.entry(self.own(self.LIB), 10, 200),
                                      self.entry(self.own(self.OTHER), 100, 0)]}]}
        self.expect_input_error(report)

    def test_zero_counters_are_not_a_counter_fault(self):
        report = self.valid()
        report["data"][0]["files"][0]["summary"]["lines"] = {"count": 0, "covered": 0}
        self.write_report(report)
        output = self.assert_exit(self.run_gate(self.report), 1)
        self.assertIn(self.LIB, output)


class RootAndPathTests(WorkspaceTestCase):
    """Design D2: one root, no crates suffix match, strict path mapping."""

    THIRD_PARTY = "/opt/vendor/crates/sample/src/lib.rs"

    def raw_report(self, entries, manifest=None):
        files = [
            {"filename": name, "summary": {"lines": {"count": count, "covered": covered}}}
            for name, count, covered in entries
        ]
        report = {"data": [{"files": files}]}
        if manifest is not None:
            report["cargo_llvm_cov"] = {"manifest_path": manifest}
        self.report.write_text(json.dumps(report), encoding="utf-8")

    def own(self, relative="crates/sample/src/lib.rs"):
        return str(self.root / relative)

    def test_third_party_suffix_path_is_not_added_to_a_crate(self):
        self.crate({"lib.rs": PRODUCTION})
        self.raw_report([(self.own(), 10, 5), (self.THIRD_PARTY, 100, 100)])
        output = self.assert_exit(self.run_gate(self.report), 1)
        self.assertRegex(output, r"sample\s+5\s+10\s+50\.00\s+5\s+BELOW TARGET")

    def test_third_party_path_is_listed_with_its_line_count(self):
        self.crate({"lib.rs": PRODUCTION})
        self.raw_report([(self.own(), 10, 10), (self.THIRD_PARTY, 100, 100)])
        output = self.assert_exit(self.run_gate(self.report), 0)
        self.assertRegex(output, r"sample\s+10\s+10\s+100\.00\s+0\s+ok")
        self.assertRegex(output, r"third-party[^\n]*\n(?:[^\n]*\n)*?[^\n]*"
                                 r"/opt/vendor/crates/sample/src/lib\.rs[^\n]*\b100\b")

    def test_root_is_the_manifest_directory(self):
        self.crate({"lib.rs": PRODUCTION})
        self.raw_report([(self.own(), 10, 10)], manifest=str(self.root / "Cargo.toml"))
        self.assert_exit(self.run_gate(self.report), 0)

    def test_manifest_directory_other_than_the_current_directory_is_an_input_error(self):
        self.crate({"lib.rs": PRODUCTION})
        self.raw_report([(self.own(), 10, 10)], manifest="/elsewhere/repo/Cargo.toml")
        self.assert_exit(self.run_gate(self.report), 2)

    def test_root_is_the_current_directory_without_a_manifest_field(self):
        self.crate({"lib.rs": PRODUCTION})
        self.raw_report([(self.own(), 10, 10)])
        self.assert_exit(self.run_gate(self.report), 0)

    def test_relative_path_resolves_against_the_root(self):
        self.crate({"lib.rs": PRODUCTION})
        self.raw_report([("crates/sample/src/lib.rs", 10, 10)])
        self.assert_exit(self.run_gate(self.report), 0)

    def test_path_with_parent_segment_is_an_input_error(self):
        self.crate({"lib.rs": PRODUCTION})
        for name in (self.own("crates/sample/../sample/src/lib.rs"),
                     "crates/sample/../sample/src/lib.rs", "../repo/crates/sample/src/lib.rs"):
            with self.subTest(name=name):
                self.raw_report([(self.own(), 10, 10), (name, 1, 1)])
                self.assert_exit(self.run_gate(self.report), 2)

    def test_unknown_path_under_crate_source_is_an_input_error(self):
        self.crate({"lib.rs": PRODUCTION})
        self.raw_report([(self.own(), 10, 10), (self.own("crates/sample/src/ghost.rs"), 5, 5)])
        output = self.assert_exit(self.run_gate(self.report), 2)
        self.assertIn("ghost.rs", output)

    def test_path_inside_the_root_and_outside_crate_source_is_listed_not_added(self):
        self.crate({"lib.rs": PRODUCTION})
        self.raw_report([(self.own(), 10, 10), (self.own("crates/sample/build.rs"), 37, 0)])
        output = self.assert_exit(self.run_gate(self.report), 0)
        self.assertRegex(output, r"sample\s+10\s+10\s+100\.00\s+0\s+ok")
        self.assertRegex(output, r"outside the metric[^\n]*\n(?:[^\n]*\n)*?[^\n]*"
                                 r"crates/sample/build\.rs[^\n]*\b37\b")

    def test_empty_filename_is_an_input_error(self):
        self.crate({"lib.rs": PRODUCTION})
        self.raw_report([(self.own(), 10, 10), ("", 1, 1)])
        output = self.assert_exit(self.run_gate(self.report), 2)
        self.assertNotRegex(output, PERCENT_LINE)

    def test_anchored_path_outside_the_root_is_third_party(self):
        self.crate({"lib.rs": PRODUCTION})
        self.raw_report([(self.own(), 10, 10), ("/crates/sample/src/lib.rs", 100, 0)])
        output = self.assert_exit(self.run_gate(self.report), 0)
        self.assertRegex(output, r"sample\s+10\s+10\s+100\.00\s+0\s+ok")

    def test_drive_path_without_a_root_is_an_input_error(self):
        gate = load_gate()
        root = pathlib.PureWindowsPath("C:/work/repo")
        with self.assertRaises(ValueError):
            gate.report_path("C:crates\\x\\src\\lib.rs", root, {pathlib.Path("crates/x/src/lib.rs")})

    def test_windows_drive_paths_map_to_workspace_paths(self):
        gate = load_gate()
        root = pathlib.PureWindowsPath("C:/work/repo")
        known = {pathlib.Path("crates/x/src/lib.rs")}
        for name in ("C:\\work\\repo\\crates\\x\\src\\lib.rs",
                     "C:/work/repo/crates/x/src/lib.rs", "c:\\work\\repo\\crates\\x\\src\\lib.rs"):
            with self.subTest(name=name):
                found = gate.report_path(name, root, known)
                self.assertEqual((found.kind, found.path),
                                 ("source", pathlib.Path("crates/x/src/lib.rs")))

    def test_windows_path_on_another_drive_is_third_party(self):
        gate = load_gate()
        root = pathlib.PureWindowsPath("C:/work/repo")
        found = gate.report_path("D:\\work\\repo\\crates\\x\\src\\lib.rs", root, set())
        self.assertEqual((found.kind, found.path), ("third_party", None))

    def test_windows_manifest_gives_the_windows_root(self):
        gate = load_gate()
        inventory_root = pathlib.PureWindowsPath("C:\\work\\repo")
        report = {"cargo_llvm_cov": {"manifest_path": "C:\\work\\repo\\Cargo.toml"}}
        self.assertEqual(gate.resolve_root(report, inventory_root), inventory_root)
        with self.assertRaises(ValueError):
            gate.resolve_root(report, pathlib.PureWindowsPath("C:\\other"))

    def test_report_path_rejects_parent_segments_and_unknown_sources(self):
        gate = load_gate()
        root = pathlib.PureWindowsPath("C:/work/repo")
        for name in ("C:\\work\\repo\\crates\\x\\..\\x\\src\\lib.rs",
                     "C:\\work\\repo\\crates\\x\\src\\other.rs",
                     "D:\\elsewhere\\..\\lib.rs"):
            with self.subTest(name=name), self.assertRaises(ValueError):
                gate.report_path(name, root, {pathlib.Path("crates/x/src/lib.rs")})


if __name__ == "__main__":
    unittest.main()
