"""Check the coverage gate with temporary Rust workspaces and synthetic reports."""

import contextlib
import importlib.util
import io
import json
import pathlib
import subprocess
import sys
import tempfile
import unittest
from unittest import mock


GATE = pathlib.Path(__file__).with_name("coverage-gate.py").resolve()
PRODUCTION = "pub fn value() -> u8 { 1 }\n"


class WorkspaceTestCase(unittest.TestCase):
    def setUp(self):
        directory = tempfile.TemporaryDirectory()
        self.addCleanup(directory.cleanup)
        self.root = pathlib.Path(directory.name)
        self.report = self.root / "coverage.json"

    def write(self, relative, content):
        path = self.root / relative
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(content, encoding="utf-8", newline="\n")
        return path

    def crate(self, files, name="sample"):
        self.write(
            f"crates/{name}/Cargo.toml",
            f'[package]\nname = "{name}"\nversion = "0.1.0"\nedition = "2024"\n',
        )
        (self.root / "crates" / name / "src").mkdir(exist_ok=True)
        for filename, content in files.items():
            self.write(f"crates/{name}/src/{filename}", content)

    def coverage(self, entries):
        files = [
            {
                "filename": str(self.root / relative),
                "summary": {"lines": {"count": count, "covered": covered}},
            }
            for relative, count, covered in entries
        ]
        self.report.write_text(
            json.dumps({"data": [{"files": files}]}), encoding="utf-8"
        )

    def run_gate(self, *arguments):
        return subprocess.run(
            [sys.executable, "-I", "-B", str(GATE), *map(str, arguments)],
            cwd=self.root,
            capture_output=True,
            text=True,
            timeout=30,
            check=False,
        )

    def assert_exit(self, result, expected):
        output = result.stdout + result.stderr
        self.assertEqual(result.returncode, expected, output)
        return output.replace("\\", "/")


class RegressionTests(WorkspaceTestCase):
    def test_crate_below_eighty_percent_fails(self):
        self.crate({"lib.rs": PRODUCTION})
        self.coverage([("crates/sample/src/lib.rs", 100, 79)])
        output = self.assert_exit(self.run_gate(self.report), 1)
        self.assertIn("sample", output)

    def test_missing_report_is_an_input_error(self):
        self.crate({"lib.rs": PRODUCTION})
        self.assert_exit(self.run_gate(self.report), 2)

    def test_bad_json_is_an_input_error(self):
        self.crate({"lib.rs": PRODUCTION})
        self.write("coverage.json", "{broken json")
        self.assert_exit(self.run_gate(self.report), 2)

    def test_bad_report_shape_is_an_input_error(self):
        self.crate({"lib.rs": PRODUCTION})
        self.write("coverage.json", '{"data": [{"files": [{}]}]}')
        self.assert_exit(self.run_gate(self.report), 2)

    def test_crate_without_production_files_has_no_code(self):
        self.crate({})
        self.coverage([])
        output = self.assert_exit(self.run_gate(self.report), 0)
        self.assertRegex(output, r"sample[^\n]*no code")


class AttributeTests(WorkspaceTestCase):
    def check_attribute(self, predicate, expected, between="", visibility=""):
        self.crate({
            "lib.rs": f"#[cfg({predicate})]\n{between}{visibility}mod fixtures;\n",
            "fixtures.rs": PRODUCTION,
        })
        self.coverage([
            ("crates/sample/src/lib.rs", 10, 10),
            ("crates/sample/src/fixtures.rs", 10, 0),
        ])
        self.assert_exit(self.run_gate(self.report), expected)

    def test_cfg_test_excludes_file(self):
        self.check_attribute("test", 0)

    def test_memory_feature_excludes_file(self):
        self.check_attribute('feature = "memory"', 0)

    def check_feature_coverage(self, literal, test_only):
        self.crate({
            "lib.rs": f"#[cfg(feature = {literal})] pub mod fixtures;\n" + PRODUCTION,
            "fixtures.rs": PRODUCTION,
        })
        self.coverage([
            ("crates/sample/src/lib.rs", 10, 5),
            ("crates/sample/src/fixtures.rs", 90, 90),
        ])
        output = self.assert_exit(self.run_gate(self.report), 1 if test_only else 0)
        row = (r"sample\s+5\s+10\s+50\.00\s+BELOW TARGET" if test_only else
               r"sample\s+95\s+100\s+95\.00\s+ok")
        self.assertRegex(output, row)

    def test_raw_memory_literals_do_not_hide_production_gap(self):
        for literal in ('r"memory"', 'r#"memory"#', 'r###"memory"###'):
            with self.subTest(literal=literal):
                self.check_feature_coverage(literal, test_only=True)

    def test_escaped_memory_literals_do_not_hide_production_gap(self):
        for literal in (r'"\x6demory"', r'"\u{6d}emory"', r'"\u{00006_d}emory"',
                        '"mem\\\n  \tory"'):
            with self.subTest(literal=literal):
                self.check_feature_coverage(literal, test_only=True)

    def test_other_literal_values_keep_production_coverage(self):
        for literal in ('"memory2"', 'r"memory2"', 'r#"memory2"#',
                        r'"memory\x32"', r'r"\x6demory"', r'r#"\u{6d}emory"#',
                        r'"memory\n"', r'"memory\0"', r'"mem\\ory"'):
            with self.subTest(literal=literal):
                self.check_feature_coverage(literal, test_only=False)

    def test_any_with_only_test_predicates_excludes_file(self):
        self.check_attribute('any(test, feature = "memory")', 0)

    def test_all_with_one_test_predicate_excludes_file(self):
        self.check_attribute("all(test, unix)", 0)

    def test_not_test_keeps_production_file(self):
        self.check_attribute("not(test)", 1)

    def test_any_with_unix_keeps_production_file(self):
        self.check_attribute("any(test, unix)", 1)

    def test_all_without_test_predicates_keeps_production_file(self):
        self.check_attribute('all(unix, feature = "storage")', 1)

    def test_nested_test_predicates_exclude_file(self):
        self.check_attribute('any(all(test, unix), feature = "memory")', 0)

    def test_pub_crate_visibility_excludes_file(self):
        self.check_attribute("test", 0, visibility="pub(crate) ")

    def test_doc_comments_and_blank_lines_keep_attribute(self):
        self.check_attribute("test", 0, between="/// Test fixtures.\n\n")

    def test_stacked_attributes_keep_test_predicate(self):
        self.check_attribute(
            "test", 0, between='#[allow(dead_code)]\n#[cfg(feature = "storage")]\n'
        )

    def test_attribute_does_not_leak_to_next_module(self):
        self.crate({
            "lib.rs": "#[cfg(test)]\nmod fixtures;\nmod service;\n",
            "fixtures.rs": PRODUCTION,
            "service.rs": PRODUCTION,
        })
        self.coverage([
            ("crates/sample/src/lib.rs", 10, 10),
            ("crates/sample/src/fixtures.rs", 100, 100),
            ("crates/sample/src/service.rs", 90, 0),
        ])
        self.assert_exit(self.run_gate(self.report), 1)


class ModuleResolutionTests(WorkspaceTestCase):
    def check_excluded(self, files, excluded, production="lib.rs"):
        self.crate(files)
        self.coverage(
            [(f"crates/sample/src/{production}", 10, 10)]
            + [(f"crates/sample/src/{name}", 10, 0) for name in excluded]
        )
        self.assert_exit(self.run_gate(self.report), 0)

    def test_lib_resolves_sibling_file(self):
        self.check_excluded({
            "lib.rs": "#[cfg(test)]\nmod fixtures;\n",
            "fixtures.rs": PRODUCTION,
        }, ["fixtures.rs"])

    def test_main_resolves_sibling_file(self):
        self.check_excluded({
            "main.rs": "#[cfg(test)]\nmod fixtures;\nfn main() {}\n",
            "fixtures.rs": PRODUCTION,
        }, ["fixtures.rs"], production="main.rs")

    def test_mod_rs_resolves_sibling_file(self):
        self.check_excluded({
            "lib.rs": "mod service;\n",
            "service/mod.rs": "#[cfg(test)]\nmod fixtures;\n",
            "service/fixtures.rs": PRODUCTION,
        }, ["service/fixtures.rs"])

    def test_named_file_resolves_child_file(self):
        self.check_excluded({
            "lib.rs": "mod service;\n",
            "service.rs": "#[cfg(test)]\nmod fixtures;\n",
            "service/fixtures.rs": PRODUCTION,
        }, ["service/fixtures.rs"])

    def test_lib_resolves_directory_module(self):
        self.check_excluded({
            "lib.rs": "#[cfg(test)]\nmod fixtures;\n",
            "fixtures/mod.rs": PRODUCTION,
        }, ["fixtures/mod.rs"])

    def test_main_resolves_directory_module(self):
        self.check_excluded({
            "main.rs": "#[cfg(test)]\nmod fixtures;\nfn main() {}\n",
            "fixtures/mod.rs": PRODUCTION,
        }, ["fixtures/mod.rs"], production="main.rs")

    def test_mod_rs_resolves_directory_module(self):
        self.check_excluded({
            "lib.rs": "mod service;\n",
            "service/mod.rs": "#[cfg(test)]\nmod fixtures;\n",
            "service/fixtures/mod.rs": PRODUCTION,
        }, ["service/fixtures/mod.rs"])

    def test_named_file_resolves_directory_module(self):
        self.check_excluded({
            "lib.rs": "mod service;\n",
            "service.rs": "#[cfg(test)]\nmod fixtures;\n",
            "service/fixtures/mod.rs": PRODUCTION,
        }, ["service/fixtures/mod.rs"])

    def test_test_directory_excludes_every_descendant(self):
        self.check_excluded({
            "lib.rs": "#[cfg(test)]\nmod fixtures;\n",
            "fixtures/mod.rs": "mod world;\n",
            "fixtures/world.rs": PRODUCTION,
            "fixtures/nested/support.rs": PRODUCTION,
        }, ["fixtures/mod.rs", "fixtures/world.rs", "fixtures/nested/support.rs"])

    def test_file_module_excludes_its_child_directory(self):
        self.check_excluded({
            "lib.rs": "#[cfg(test)]\nmod fixtures;\n",
            "fixtures.rs": "mod world;\n",
            "fixtures/world.rs": PRODUCTION,
            "fixtures/nested/support.rs": PRODUCTION,
        }, ["fixtures.rs", "fixtures/world.rs", "fixtures/nested/support.rs"])

    def test_nested_test_directory_excludes_descendants(self):
        self.check_excluded({
            "lib.rs": "mod service;\n",
            "service/mod.rs": "#[cfg(test)]\nmod fixtures;\n",
            "service/fixtures/mod.rs": "mod world;\n",
            "service/fixtures/world.rs": PRODUCTION,
            "service/fixtures/nested/support.rs": PRODUCTION,
        }, ["service/fixtures/mod.rs", "service/fixtures/world.rs",
            "service/fixtures/nested/support.rs"])

    def test_test_directory_does_not_hide_similar_production_directory(self):
        self.crate({
            "lib.rs": "#[cfg(test)]\nmod fixtures;\nmod fixtures_extra;\n",
            "fixtures/mod.rs": PRODUCTION,
            "fixtures_extra/mod.rs": PRODUCTION,
        })
        self.coverage([
            ("crates/sample/src/lib.rs", 10, 10),
            ("crates/sample/src/fixtures/mod.rs", 10, 0),
            ("crates/sample/src/fixtures_extra/mod.rs", 90, 0),
        ])
        self.assert_exit(self.run_gate(self.report), 1)


class SharedModuleTests(WorkspaceTestCase):
    def check_shared_module(self, test_target, module_file, measured, both_test=False):
        other_target = "main.rs" if test_target == "lib.rs" else "lib.rs"
        self.crate({
            test_target: "#[cfg(test)] pub mod common;\n",
            other_target: ("#[cfg(test)] " if both_test else "") + "mod common;\n",
            module_file: "mod child;\n#[cfg(test)] mod fixtures;\n" + PRODUCTION,
            "common/child.rs": PRODUCTION,
            "common/fixtures.rs": PRODUCTION,
        })
        self.coverage([
            (f"crates/sample/src/{test_target}", 5, 5),
            (f"crates/sample/src/{other_target}", 5, 5),
            (f"crates/sample/src/{measured}", 90, 0),
            ("crates/sample/src/common/fixtures.rs", 900, 900),
        ])
        output = self.assert_exit(self.run_gate(self.report), 0 if both_test else 1)
        row = r"sample\s+10\s+10\s+100\.00\s+ok" if both_test else (
            r"sample\s+10\s+100\s+10\.00\s+BELOW TARGET"
        )
        self.assertRegex(output, row)

    def test_production_declaration_keeps_shared_module(self):
        for target in ("lib.rs", "main.rs"):
            for module_file in ("common.rs", "common/mod.rs"):
                with self.subTest(test_target=target, module_file=module_file):
                    self.check_shared_module(target, module_file, module_file)

    def test_production_declaration_keeps_shared_descendant(self):
        for target in ("lib.rs", "main.rs"):
            for module_file in ("common.rs", "common/mod.rs"):
                with self.subTest(test_target=target, module_file=module_file):
                    self.check_shared_module(target, module_file, "common/child.rs")

    def test_only_test_declarations_exclude_shared_module_and_descendants(self):
        for module_file in ("common.rs", "common/mod.rs"):
            for measured in (module_file, "common/child.rs"):
                with self.subTest(module_file=module_file, measured=measured):
                    self.check_shared_module("lib.rs", module_file, measured, both_test=True)


class SourceScanTests(WorkspaceTestCase):
    def check_path_attribute(self, files):
        self.crate(files)
        self.coverage([("crates/sample/src/lib.rs", 10, 10)])
        self.assert_exit(self.run_gate(self.report), 2)

    def test_path_attribute_in_production_is_an_input_error(self):
        self.check_path_attribute({
            "lib.rs": '#[path = "service.rs"]\nmod service;\n',
            "service.rs": PRODUCTION,
        })

    def test_path_attribute_in_test_file_is_an_input_error(self):
        self.check_path_attribute({
            "lib.rs": "#[cfg(test)]\nmod fixtures;\n",
            "fixtures.rs": '#[path = "support.rs"]\nmod support;\n',
            "support.rs": PRODUCTION,
        })

    def test_path_attribute_in_unreferenced_file_is_an_input_error(self):
        self.check_path_attribute({
            "lib.rs": PRODUCTION,
            "unused.rs": '#[path = "service.rs"]\nmod service;\n',
            "service.rs": PRODUCTION,
        })

    def test_inline_test_module_names_file_and_line(self):
        self.crate({
            "lib.rs": PRODUCTION + "#[cfg(test)]\nmod tests {\n}\n",
        })
        self.coverage([("crates/sample/src/lib.rs", 10, 10)])
        output = self.assert_exit(self.run_gate(self.report), 1)
        # Accept the attribute line or the declaration line as the location.
        self.assertRegex(output, r"crates/sample/src/lib\.rs:(?:2|3)\b")

    def test_inline_module_in_test_file_passes(self):
        self.crate({
            "lib.rs": "#[cfg(test)]\nmod fixtures;\n",
            "fixtures.rs": "#[cfg(test)]\nmod tests {\n}\n",
        })
        self.coverage([
            ("crates/sample/src/lib.rs", 10, 10),
            ("crates/sample/src/fixtures.rs", 10, 0),
        ])
        self.assert_exit(self.run_gate(self.report), 0)

    def check_production_name(self, filename):
        self.crate({"lib.rs": PRODUCTION, filename: PRODUCTION})
        # The coverage tool omits this file before the gate reads the report.
        self.coverage([("crates/sample/src/lib.rs", 10, 10)])
        for argument in (self.report, "--source-only"):
            with self.subTest(argument=argument):
                output = self.assert_exit(self.run_gate(argument), 1)
                self.assertIn(f"crates/sample/src/{filename}:1:", output)

    def test_production_underscore_tests_name_fails(self):
        self.check_production_name("foo_tests.rs")

    def test_production_hyphen_tests_name_fails(self):
        self.check_production_name("foo-tests.rs")

    def test_production_tests_name_fails(self):
        self.check_production_name("tests.rs")

    def test_production_file_in_tests_directory_fails(self):
        self.check_production_name("service/tests/support.rs")

    def test_production_files_in_examples_and_benches_fail(self):
        for filename in ("examples/mod.rs", "benches/hot.rs",
                         "service/examples/helper.rs", "service/benches/hot.rs"):
            with self.subTest(filename=filename):
                self.check_production_name(filename)

    def test_test_only_examples_and_benches_pass(self):
        self.crate({
            "lib.rs": "#[cfg(test)] mod examples;\n#[cfg(test)] mod benches;\n" + PRODUCTION,
            "examples/mod.rs": PRODUCTION,
            "benches/mod.rs": "mod hot;\n",
            "benches/hot.rs": PRODUCTION,
        })
        self.coverage([("crates/sample/src/lib.rs", 10, 10)])
        for argument in (self.report, "--source-only"):
            with self.subTest(argument=argument):
                self.assert_exit(self.run_gate(argument), 0)

    def test_similar_paths_outside_default_filter_pass(self):
        filenames = ("examples_extra/mod.rs", "my_benches/hot.rs", "tests_extra/mod.rs",
                     "examples.rs", "benches.rs", "_tests.rs", "-tests.rs",
                     "foo.tests.rs", "café_tests.rs")
        self.crate({"lib.rs": PRODUCTION, **dict.fromkeys(filenames, PRODUCTION)})
        self.coverage([(f"crates/sample/src/{name}", 10, 10)
                       for name in ("lib.rs", *filenames)])
        for argument in (self.report, "--source-only"):
            with self.subTest(argument=argument):
                self.assert_exit(self.run_gate(argument), 0)

    def test_test_only_file_with_tests_name_passes(self):
        self.crate({
            "lib.rs": "#[cfg(test)]\nmod tests;\n",
            "tests.rs": "#[test]\nfn works() {}\n",
        })
        self.coverage([("crates/sample/src/lib.rs", 10, 10)])
        self.assert_exit(self.run_gate(self.report), 0)

    def test_source_only_without_report_passes(self):
        self.crate({"lib.rs": PRODUCTION})
        self.assert_exit(self.run_gate("--source-only"), 0)

    def test_source_only_lists_each_violation_on_one_line(self):
        self.crate({
            "lib.rs": PRODUCTION + "#[cfg(test)]\nmod tests {\n}\n",
            "foo_tests.rs": PRODUCTION,
        })
        output = self.assert_exit(self.run_gate("--source-only"), 1)
        for filename in ("lib.rs", "foo_tests.rs"):
            matches = [line for line in output.splitlines() if filename in line]
            self.assertEqual(len(matches), 1, output)
            self.assertIn(f"crates/sample/src/{filename}", matches[0])
        self.assertRegex(output, r"lib\.rs:(?:2|3)\b")

    def test_source_only_with_path_attribute_is_an_input_error(self):
        self.crate({
            "lib.rs": '#[path = "service.rs"]\nmod service;\n',
            "service.rs": PRODUCTION,
        })
        output = self.assert_exit(self.run_gate("--source-only"), 2)
        self.assertIn("lib.rs", output)

    def test_unreadable_source_is_an_input_error(self):
        self.crate({"lib.rs": PRODUCTION})
        self.coverage([("crates/sample/src/lib.rs", 10, 10)])
        spec = importlib.util.spec_from_file_location("coverage_gate", GATE)
        gate = importlib.util.module_from_spec(spec)
        with mock.patch.dict(sys.modules, {"coverage_gate": gate}):
            spec.loader.exec_module(gate)
        original_open = pathlib.Path.open

        def open_source(path, *args, **kwargs):
            if path.suffix == ".rs":
                raise PermissionError(f"Cannot read {path}")
            return original_open(path, *args, **kwargs)

        # Windows permissions cannot reliably make a temporary file unreadable.
        output = io.StringIO()
        with (
            contextlib.chdir(self.root),
            contextlib.redirect_stdout(output),
            contextlib.redirect_stderr(output),
            mock.patch.object(pathlib.Path, "open", open_source),
        ):
            result = gate.main([str(self.report)])
        self.assertEqual(result, 2, output.getvalue())


class ReportTests(WorkspaceTestCase):
    def test_production_crate_missing_from_nonempty_report_fails(self):
        self.crate({"lib.rs": PRODUCTION})
        self.crate({"lib.rs": PRODUCTION}, name="missing")
        self.coverage([("crates/sample/src/lib.rs", 10, 10)])
        output = self.assert_exit(self.run_gate(self.report), 1)
        self.assertIn("missing", output)

    def test_production_crate_with_empty_report_fails(self):
        self.crate({"lib.rs": PRODUCTION})
        self.coverage([])
        output = self.assert_exit(self.run_gate(self.report), 1)
        self.assertIn("sample", output)

    def test_production_crate_with_zero_reported_lines_fails(self):
        self.crate({"lib.rs": PRODUCTION})
        self.coverage([("crates/sample/src/lib.rs", 0, 0)])
        output = self.assert_exit(self.run_gate(self.report), 1)
        self.assertIn("sample", output)

    def test_only_test_lines_do_not_satisfy_production_crate(self):
        self.crate({
            "lib.rs": "#[cfg(test)]\nmod fixtures;\n" + PRODUCTION,
            "fixtures.rs": PRODUCTION,
        })
        self.coverage([("crates/sample/src/fixtures.rs", 10, 10)])
        output = self.assert_exit(self.run_gate(self.report), 1)
        self.assertIn("sample", output)

    def test_test_lines_do_not_lower_production_coverage(self):
        self.crate({
            "lib.rs": "#[cfg(test)]\nmod fixtures;\n" + PRODUCTION,
            "fixtures.rs": PRODUCTION,
        })
        # Raw coverage is 50 percent, production coverage is 100 percent.
        self.coverage([
            ("crates/sample/src/lib.rs", 10, 10),
            ("crates/sample/src/fixtures.rs", 10, 0),
        ])
        self.assert_exit(self.run_gate(self.report), 0)

    def test_covered_test_lines_do_not_hide_production_gap(self):
        self.crate({
            "lib.rs": "#[cfg(test)]\nmod fixtures;\n" + PRODUCTION,
            "fixtures.rs": PRODUCTION,
        })
        self.coverage([
            ("crates/sample/src/lib.rs", 10, 5),
            ("crates/sample/src/fixtures.rs", 90, 90),
        ])
        self.assert_exit(self.run_gate(self.report), 1)

    def test_exactly_eighty_percent_production_passes(self):
        self.crate({"lib.rs": PRODUCTION})
        self.coverage([("crates/sample/src/lib.rs", 10, 8)])
        self.assert_exit(self.run_gate(self.report), 0)


if __name__ == "__main__":
    unittest.main()
