"""Check upgrade gates without Docker."""

import importlib.util
import copy
import pathlib
import subprocess
import sys
import unittest


class UpgradeChecksTests(unittest.TestCase):
    def setUp(self):
        path = pathlib.Path(__file__).with_name("upgrade_checks.py")
        spec = importlib.util.spec_from_file_location("upgrade_checks", path)
        self.checks = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(self.checks)

    def test_push_selects_the_before_commit(self):
        before = "a" * 40
        self.assertEqual(self.checks.select_previous_ref("", "push", before), before)

    def test_push_rejects_the_zero_sha(self):
        with self.assertRaisesRegex(ValueError, "zero SHA"):
            self.checks.select_previous_ref("", "push", "0" * 40)

    def test_push_rejects_a_missing_or_invalid_before_sha(self):
        for before in ("", "main", "a" * 39, "g" * 40):
            with self.subTest(before=before), self.assertRaises(ValueError):
                self.checks.select_previous_ref("", "push", before)

    def test_explicit_ref_overrides_the_event(self):
        for event, before in (("push", "a" * 40), ("push", "0" * 40), ("workflow_dispatch", "")):
            with self.subTest(event=event, before=before):
                self.assertEqual(self.checks.select_previous_ref("v1.2.3", event, before), "v1.2.3")

    def test_no_ref_requests_the_merge_base(self):
        for event in ("", "workflow_dispatch"):
            with self.subTest(event=event):
                self.assertIsNone(self.checks.select_previous_ref("", event, "a" * 40))

    def test_previous_commit_must_differ_from_head(self):
        self.checks.check_previous_commit("a" * 40, "b" * 40)
        with self.assertRaisesRegex(ValueError, "previous commit equals HEAD"):
            self.checks.check_previous_commit("a" * 40, "a" * 40)

    def test_equal_commits_exit_with_one_clear_message(self):
        result = subprocess.run(
            [sys.executable, "-I", "-B", str(pathlib.Path(__file__).with_name("upgrade_checks.py")),
             "previous-commit", "a" * 40, "a" * 40],
            capture_output=True, text=True, check=False,
        )
        self.assertEqual(result.returncode, 1)
        self.assertEqual(result.stdout, "")
        self.assertEqual(result.stderr, "CI: the previous commit equals HEAD. Stop and report.\n")

    def test_extracts_only_the_guard_statement(self):
        statement = (
            "CREATE FUNCTION audit_guard.audit_objects_dropped() RETURNS event_trigger\n"
            "LANGUAGE plpgsql AS $$\nBEGIN\n    RETURN;\nEND;\n$$;"
        )
        source = "CREATE SCHEMA audit_guard;\n" + statement + "\nREVOKE ALL ON SCHEMA audit_guard FROM PUBLIC;"
        self.assertEqual(self.checks.guard_statement(source), statement.replace("CREATE FUNCTION", "CREATE OR REPLACE FUNCTION", 1))

    def test_missing_duplicate_and_unclosed_guards_fail(self):
        statement = "CREATE FUNCTION audit_guard.audit_objects_dropped() RETURNS event_trigger\n$$;"
        for source in ("", statement + "\n" + statement, statement.removesuffix("$$;")):
            with self.subTest(source=source), self.assertRaises(ValueError):
                self.checks.guard_statement(source)

    def test_only_one_exact_restore_success_passes(self):
        expected = "recipe_20261026_1201_now"
        passed = '{"message":"restore test passed","set":"' + expected + '"}\n'
        self.checks.check_restore(passed, expected)
        for output in ("", passed.replace("passed", "started"), passed.replace("_now", "_premigrate"), passed + passed):
            with self.subTest(output=output), self.assertRaises(ValueError):
                self.checks.check_restore(output, expected)

    def test_backup_result_rejects_the_wrong_suffix(self):
        passed = '{"message":"backup completed","set":"recipe_20261026_1201_premigrate"}\n'
        self.assertEqual(self.checks.backup_set(passed, "_premigrate"), "recipe_20261026_1201_premigrate")
        for output in (passed.replace("completed", "started"), passed.replace("_premigrate", "_now"), passed + passed):
            with self.subTest(output=output), self.assertRaises(ValueError):
                self.checks.backup_set(output, "_premigrate")

    def test_config_rejects_one_stale_image(self):
        services = {name: {"image": f"recipe-{image}:new"} for name, image in self.checks.IMAGES.items()}
        self.checks.check_images({"services": services}, "new")
        services["walarchive"]["image"] = "recipe-pgtools:old"
        with self.assertRaises(ValueError):
            self.checks.check_images({"services": services}, "new")


class UpgradeSmokeTests(unittest.TestCase):
    def setUp(self):
        path = pathlib.Path(__file__).with_name("upgrade_smoke.py")
        spec = importlib.util.spec_from_file_location("upgrade_smoke", path)
        self.smoke = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(self.smoke)

    def test_export_checks_data_hash_and_loadable_state(self):
        document = self.smoke.fixture_body()
        document.update(content_sha256=self.smoke.body_hash(document), loadable=True)
        self.smoke.check_export(document)
        for field, value in (("content_sha256", "0" * 64), ("loadable", False), ("parameters", [])):
            changed = copy.deepcopy(document)
            changed[field] = value
            with self.subTest(field=field), self.assertRaises(ValueError):
                self.smoke.check_export(changed)

    def test_export_rejects_a_different_recipe_even_with_its_correct_hash(self):
        document = self.smoke.fixture_body()
        document["recipe"]["reference_number"] = "WRONG-RECIPE"
        document.update(content_sha256=self.smoke.body_hash(document), loadable=True)
        with self.assertRaises(ValueError):
            self.smoke.check_export(document)

    def test_export_excludes_display_names_from_the_hash(self):
        document = self.smoke.fixture_body()
        document.update(content_sha256=self.smoke.body_hash(document), loadable=True)
        document["machine"]["name"] = "A new name"
        document["recipe"]["name"] = "Another name"
        document["parameters"][0]["name_en"] = "A display name"
        self.smoke.check_export(document)


if __name__ == "__main__":
    unittest.main()
