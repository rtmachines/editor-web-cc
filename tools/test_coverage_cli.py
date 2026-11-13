"""Check the four command-line forms of the coverage gate (design D2 root, design D5 CLI)."""

import hashlib
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
PERCENT_LINE = r"\d+\.\d\d"

LIB_SOURCE = "#[cfg(test)]\nmod support;\npub mod decl;\npub mod other;\npub fn value() -> u8 { 1 }\n"
DECL_SOURCE = "pub use crate::other::value;\n"
LIB = "crates/sample/src/lib.rs"
OTHER = "crates/sample/src/other.rs"
DECL = "crates/sample/src/decl.rs"
SUPPORT = "crates/sample/src/support.rs"
SOURCES = {LIB: LIB_SOURCE, OTHER: PRODUCTION, DECL: DECL_SOURCE, SUPPORT: PRODUCTION}
CLASSES = {LIB: "required", OTHER: "required", DECL: "declaration_only", SUPPORT: "test_only"}


def sha256(text):
    """Hash the fixture text the same way as the inventory, from its bytes."""
    return hashlib.sha256(text.encode("utf-8")).hexdigest()


class CliTestCase(WorkspaceTestCase):
    """A workspace with one required, one declaration-only, and one test-only file."""

    def setUp(self):
        super().setUp()
        self.crate({name.rsplit("/", 1)[1]: text for name, text in SOURCES.items()})
        self.inventory = self.root / "inventory.json"

    def own(self, relative):
        return str(self.root / relative)

    def write_report(self, lib=(10, 10), other=(10, 10), extra=None):
        entries = [{"filename": self.own(name),
                    "summary": {"lines": {"count": count, "covered": covered}}}
                   for name, (count, covered) in ((LIB, lib), (OTHER, other))]
        report = {"data": [{"files": entries}]}
        report.update(extra or {})
        self.report.write_text(json.dumps(report), encoding="utf-8")

    def write_inventory(self):
        self.assert_exit(self.run_gate("--write-inventory", self.inventory), 0)

    def edit_inventory(self, edit):
        stored = json.loads(self.inventory.read_text(encoding="utf-8"))
        edit(stored)
        self.inventory.write_text(json.dumps(stored), encoding="utf-8")

    def run_ci(self):
        return self.run_gate("--report", self.report, "--inventory", self.inventory)


class WriteInventoryTests(CliTestCase):
    def test_file_holds_literal_fields_of_the_fixture(self):
        self.write("crates/sample/src/extra.rs", PRODUCTION, track=False)
        self.write_inventory()
        stored = json.loads(self.inventory.read_text(encoding="utf-8"))
        expected_files = [{"path": name, "sha256": sha256(SOURCES[name]), "crate": "sample",
                           "classification": CLASSES[name]} for name in sorted(SOURCES)]
        self.assertEqual(stored, {
            "schema_version": 1,
            "root": self.root.resolve().as_posix(),
            "files": expected_files,
            "untracked": ["crates/sample/src/extra.rs"],
            "build_scripts": [],
        })

    def test_two_writes_of_one_tree_give_identical_bytes(self):
        self.write_inventory()
        first = self.inventory.read_bytes()
        self.write_inventory()
        self.assertEqual(self.inventory.read_bytes(), first)

    def test_file_has_sorted_keys_and_a_final_newline(self):
        self.write_inventory()
        keys = []

        def record(pairs):
            keys.append([key for key, _value in pairs])
            return dict(pairs)

        text = self.inventory.read_text(encoding="utf-8")
        json.loads(text, object_pairs_hook=record)
        self.assertTrue(text.endswith("}\n"))
        self.assertEqual(len(keys), 5)
        for names in keys:
            self.assertEqual(names, sorted(names))

    def test_unwritable_path_is_an_input_error(self):
        blocker = self.root / "blocker"
        blocker.write_text("not a directory", encoding="utf-8")
        output = self.assert_exit(self.run_gate("--write-inventory", blocker / "inventory.json"), 2)
        self.assertIn("coverage-gate:", output)

    def test_write_creates_missing_parent_directories(self):
        target = self.root / "missing" / "nested" / "inventory.json"
        self.assert_exit(self.run_gate("--write-inventory", target), 0)
        self.assertIsInstance(json.loads(target.read_text(encoding="utf-8")), dict)

    def test_write_prints_no_percentage(self):
        output = self.assert_exit(self.run_gate("--write-inventory", self.inventory), 0)
        self.assertNotRegex(output, PERCENT_LINE)


class CiFormResultTests(CliTestCase):
    def compare_with_legacy(self, expected):
        self.write_inventory()
        legacy = self.assert_exit(self.run_gate(self.report), expected)
        ci = self.assert_exit(self.run_ci(), expected)
        self.assertEqual(ci, legacy)
        return ci

    def test_pass_matches_legacy_form(self):
        self.write_report()
        output = self.compare_with_legacy(0)
        self.assertRegex(output, r"sample\s+20\s+20\s+100\.00\s+0\s+ok")

    def test_deficit_matches_legacy_form(self):
        self.write_report(lib=(10, 5), other=(10, 5))
        output = self.compare_with_legacy(1)
        self.assertRegex(output, r"sample\s+10\s+20\s+50\.00")

    def test_input_error_matches_legacy_form(self):
        self.report.write_text(json.dumps({"data": []}), encoding="utf-8")
        output = self.compare_with_legacy(2)
        self.assertNotRegex(output, PERCENT_LINE)

    def test_build_script_list_is_the_same_in_both_forms(self):
        self.write("crates/sample/build.rs", PRODUCTION)
        self.write_report()
        output = self.compare_with_legacy(0)
        self.assertIn("build-time code outside the metric, not counted:\ncrates/sample/build.rs\n", output)

    def test_untracked_source_file_gives_exit_1_in_both_forms(self):
        self.write("crates/sample/src/extra.rs", PRODUCTION, track=False)
        self.write_report()
        output = self.compare_with_legacy(1)
        self.assertIn("crates/sample/src/extra.rs:1: untracked source file", output)

    def test_untracked_source_file_with_report_entry_gives_exit_2_in_both_forms(self):
        draft = "crates/sample/src/draft.rs"
        self.write(draft, PRODUCTION, track=False)
        self.write_report()
        report = json.loads(self.report.read_text(encoding="utf-8"))
        report["data"][0]["files"].append(
            {"filename": self.own(draft), "summary": {"lines": {"count": 4, "covered": 4}}})
        self.report.write_text(json.dumps(report), encoding="utf-8")
        output = self.compare_with_legacy(2)
        self.assertIn(f"{draft}: the report holds an untracked source file", output)
        self.assertNotRegex(output, PERCENT_LINE)

    def test_missing_report_is_an_input_error(self):
        self.write_inventory()
        self.assert_exit(self.run_ci(), 2)


class StaleInventoryTests(CliTestCase):
    def setUp(self):
        super().setUp()
        self.write_report()
        self.write_inventory()

    def expect_stale(self, fragment=None):
        output = self.assert_exit(self.run_ci(), 2)
        self.assertNotRegex(output, PERCENT_LINE)
        self.assertNotIn("every crate meets", output)
        if fragment:
            self.assertIn(fragment, output)

    def test_unchanged_inventory_passes(self):
        self.assert_exit(self.run_ci(), 0)

    def test_source_edit_after_the_write_changes_the_hash(self):
        self.write(OTHER, PRODUCTION + "// edit\n")
        self.expect_stale(OTHER)

    def test_added_tracked_file_is_stale(self):
        self.write("crates/sample/src/added.rs", PRODUCTION)
        self.expect_stale("crates/sample/src/added.rs")

    def test_removed_tracked_file_is_stale(self):
        self.git("rm", "-q", "-f", "--", OTHER)
        self.expect_stale(OTHER)

    def test_added_untracked_file_is_stale(self):
        self.write("crates/sample/src/extra.rs", PRODUCTION, track=False)
        self.expect_stale("crates/sample/src/extra.rs")

    def test_changed_classification_is_stale(self):
        def edit(stored):
            for item in stored["files"]:
                if item["path"] == OTHER:
                    item["classification"] = "declaration_only"

        self.edit_inventory(edit)
        self.expect_stale(OTHER)

    def test_wrong_schema_version_is_an_input_error(self):
        self.edit_inventory(lambda stored: stored.update(schema_version=2))
        self.expect_stale("schema_version")

    def test_boolean_schema_version_is_an_input_error(self):
        self.edit_inventory(lambda stored: stored.update(schema_version=True))
        self.expect_stale("schema_version")

    def test_missing_key_is_an_input_error(self):
        self.edit_inventory(lambda stored: stored.pop("untracked"))
        self.expect_stale("untracked")

    def expect_bad_entry(self, edit):
        self.edit_inventory(edit)
        output = self.assert_exit(self.run_ci(), 2)
        self.assertIn("coverage-gate:", output)
        self.assertNotIn("Traceback", output)

    def test_path_that_is_a_list_is_an_input_error(self):
        self.expect_bad_entry(lambda stored: stored["files"][0].update(path=[]))

    def test_file_item_that_is_a_string_is_an_input_error(self):
        self.expect_bad_entry(lambda stored: stored["files"].__setitem__(0, "text"))

    def test_file_item_without_a_field_is_an_input_error(self):
        self.expect_bad_entry(lambda stored: stored["files"][0].pop("sha256"))

    def test_untracked_item_that_is_a_number_is_an_input_error(self):
        self.expect_bad_entry(lambda stored: stored["untracked"].append(5))

    def test_build_scripts_that_is_not_a_list_of_strings_is_an_input_error(self):
        self.expect_bad_entry(lambda stored: stored.update(build_scripts="crates/sample/build.rs"))
        self.expect_bad_entry(lambda stored: stored.update(build_scripts=[5]))

    def test_added_build_script_is_stale(self):
        self.write("crates/sample/build.rs", PRODUCTION)
        self.expect_stale("crates/sample/build.rs")

    def test_changed_root_is_an_input_error(self):
        self.edit_inventory(lambda stored: stored.update(root="/elsewhere"))
        self.expect_stale("root")

    def test_malformed_file_is_an_input_error(self):
        self.inventory.write_text("{not json", encoding="utf-8")
        self.expect_stale("inventory")

    def test_non_object_file_is_an_input_error(self):
        self.inventory.write_text("[]", encoding="utf-8")
        self.expect_stale("inventory")

    def test_missing_file_is_an_input_error(self):
        self.inventory.unlink()
        self.expect_stale("inventory")

    def test_directory_instead_of_file_is_an_input_error(self):
        self.inventory.unlink()
        self.inventory.mkdir()
        self.expect_stale("inventory")


class InventoryRootTests(CliTestCase):
    def setUp(self):
        super().setUp()
        self.write_inventory()

    def test_manifest_directory_equal_to_inventory_root_passes(self):
        self.write_report(extra={"cargo_llvm_cov": {"manifest_path": self.own("Cargo.toml")}})
        self.assert_exit(self.run_ci(), 0)

    def test_manifest_directory_that_differs_from_inventory_root_is_an_input_error(self):
        self.write_report(extra={"cargo_llvm_cov": {"manifest_path": "/elsewhere/Cargo.toml"}})
        output = self.assert_exit(self.run_ci(), 2)
        self.assertIn("/elsewhere", output)
        self.assertNotRegex(output, PERCENT_LINE)

    def test_report_path_outside_inventory_root_is_third_party(self):
        self.write_report()
        report = json.loads(self.report.read_text(encoding="utf-8"))
        outside = "/elsewhere/crates/sample/src/lib.rs"
        report["data"][0]["files"].append(
            {"filename": outside, "summary": {"lines": {"count": 7, "covered": 0}}})
        self.report.write_text(json.dumps(report), encoding="utf-8")
        output = self.assert_exit(self.run_ci(), 0)
        self.assertIn(f"{outside} 7 lines", output)


class BadArgumentTests(CliTestCase):
    def expect_usage_error(self, *arguments):
        result = self.run_gate(*arguments)
        output = self.assert_exit(result, 2)
        self.assertEqual(result.stdout, "")
        self.assertIn("usage", result.stderr.lower())
        self.assertNotRegex(output, PERCENT_LINE)

    def test_no_arguments(self):
        self.expect_usage_error()

    def test_unknown_flag(self):
        self.expect_usage_error("--bogus")

    def test_unknown_flag_after_valid_form(self):
        self.expect_usage_error("--report", self.report, "--inventory", self.inventory, "--bogus")

    def test_report_without_inventory(self):
        self.expect_usage_error("--report", self.report)

    def test_inventory_without_report(self):
        self.expect_usage_error("--inventory", self.inventory)

    def test_report_flag_without_value(self):
        self.expect_usage_error("--report")

    def test_inventory_flag_without_value(self):
        self.expect_usage_error("--report", self.report, "--inventory")

    def test_write_inventory_without_value(self):
        self.expect_usage_error("--write-inventory")

    def test_extra_argument_after_legacy_report(self):
        self.expect_usage_error(self.report, "extra")

    def test_extra_argument_after_ci_form(self):
        self.expect_usage_error("--report", self.report, "--inventory", self.inventory, "extra")

    def test_legacy_report_with_inventory(self):
        self.expect_usage_error(self.report, "--inventory", self.inventory)

    def test_source_only_with_report(self):
        self.expect_usage_error("--source-only", "--report", self.report, "--inventory", self.inventory)

    def test_source_only_with_extra_argument(self):
        self.expect_usage_error("--source-only", self.report)

    def test_write_inventory_with_report(self):
        self.expect_usage_error("--write-inventory", self.inventory, "--report", self.report)

    def test_write_inventory_with_extra_argument(self):
        self.expect_usage_error("--write-inventory", self.inventory, "extra")


class ExistingFormTests(CliTestCase):
    def test_source_only_keeps_its_result(self):
        self.assert_exit(self.run_gate("--source-only"), 0)

    def test_source_only_still_fails_on_untracked_file(self):
        self.write("crates/sample/src/extra.rs", PRODUCTION, track=False)
        self.assert_exit(self.run_gate("--source-only"), 1)

    def test_legacy_form_keeps_its_result(self):
        self.write_report()
        output = self.assert_exit(self.run_gate(self.report), 0)
        self.assertIn("every crate meets 80 % line coverage", output)


if __name__ == "__main__":
    unittest.main()
