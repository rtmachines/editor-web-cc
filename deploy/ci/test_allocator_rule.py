"""Check the allocator rule with Cargo, Rust, and deploy fixtures."""

import json
import os
import shlex
import subprocess
import sys
import tempfile
import unittest
from copy import deepcopy
from pathlib import Path

SCRIPT = Path(__file__).with_name("allocator_rule.py")
MUSL = "x86_64-unknown-linux-musl"
GNU = "x86_64-unknown-linux-gnu"
TREE = (0, "mimalloc v0.1.52\n└── recipe-server v0.1.0 (/workspace/crates/recipe-server)\n", "")
EMPTY_TREE = (
    0, "", (
        "warning: nothing to print.\n\n"
        "To find dependencies that require specific target platforms, try to use option "
        "`--target all` first, and then narrow your search scope accordingly.\n"
    ),
)
MISSING_PACKAGE = (
    101, "", "error: package ID specification `mimalloc` did not match any packages\n",
)
CARGO_ERROR = (101, "", "error: failed to load manifest for workspace member\n")
MIMALLOC_ID = "registry+https://github.com/rust-lang/crates.io-index#mimalloc@0.1.52"
LIBMIMALLOC_ID = "registry+https://github.com/rust-lang/crates.io-index#libmimalloc-sys@0.1.49"
CFG = '#[cfg(target_env = "musl")]\n'
UNWIND_CFG = '#![cfg(target_env = "musl")]\n'
MAIN_RS = "crates/recipe-server/src/main.rs"
UNWIND_RS = "crates/recipe-server/tests/mimalloc_unwind.rs"
ALLOCATOR = "#[global_allocator]\n"
STATIC = "static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;\n"
SHADOWING_IMPORTS = (
    "use crate::compat::mimalloc;",
    "use recipe_server::mimalloc;",
    "use recipe_server::r#mimalloc;",
    "use crate::compat::{mimalloc};",
    "pub(crate) use recipe_server::{\n    compat::{r#mimalloc},\n};",
    "use crate::compat::mimalloc::{Other, self};",
    "use crate::compat as r#mimalloc;",
)
GLOB_IMPORTS = (
    "use recipe_server::*;",
    "use recipe_server::{compat::{Other, *}};",
)

CARGO_STUB = """\
import json
import os
from pathlib import Path
import sys

prefix = ["tree", "--locked", "-p", "recipe-server", "-e", "normal", "-i", "mimalloc",
          "--charset", "utf8", "--target"]
metadata = ["metadata", "--locked", "--format-version", "1", "--filter-platform",
            "x86_64-unknown-linux-musl"]
metadata_runs = {tuple(metadata): "default", (*metadata, "--all-features"): "all features"}
if os.environ.get("CARGO_TERM_COLOR") != "never":
    sys.exit("Set CARGO_TERM_COLOR to never for Cargo.")
args = tuple(sys.argv[1:])
if args in metadata_runs:
    run = metadata_runs[args]
    with Path("metadata-called").open("a", encoding="utf-8") as calls:
        print(run, file=calls)
    fixture = json.loads(Path("metadata.json").read_text(encoding="utf-8"))
    status, stdout, stderr = fixture[run]
elif sys.argv[1:-1] == prefix:
    fixture = json.loads(Path("cargo.json").read_text(encoding="utf-8"))
    status, stdout, stderr = fixture[sys.argv[-1]]
else:
    sys.exit(f"Unexpected cargo arguments {sys.argv[1:]!r}.")
sys.stdout.write(stdout)
sys.stderr.write(stderr)
sys.exit(status)
"""


class AllocatorRuleTests(unittest.TestCase):
    def setUp(self):
        self.assertTrue(SCRIPT.is_file(), "Task 2.1 must add allocator_rule.py.")
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        self.root = Path(temporary.name)
        self.trees = {MUSL: TREE, GNU: EMPTY_TREE}
        self.nodes = [
            {"id": LIBMIMALLOC_ID, "features": []},
            {"id": MIMALLOC_ID, "features": ["default"]},
        ]
        self.mimalloc_features = {
            "default": [],
            "secure": ["libmimalloc-sys/secure"],
            "v2": ["libmimalloc-sys/v2"],
        }
        self.metadata = {
            "packages": [
                {"id": MIMALLOC_ID, "name": "mimalloc", "features": self.mimalloc_features},
                {"id": LIBMIMALLOC_ID, "name": "libmimalloc-sys", "features": {"secure": []}},
            ],
            "resolve": {"nodes": self.nodes},
        }
        self.all_features_metadata = deepcopy(self.metadata)
        self.metadata_failures = {}
        self.parent_env = {}
        self.source = CFG + ALLOCATOR + STATIC
        self.unwind_source = UNWIND_CFG + ALLOCATOR + STATIC
        self.script = self.root / "deploy/ci/allocator_rule.py"
        self.write("deploy/ci/allocator_rule.py", SCRIPT.read_text(encoding="utf-8"))
        self.write("bin/cargo_stub.py", CARGO_STUB)
        self.write("bin/cargo", f'#!/bin/sh\nexec {shlex.quote(sys.executable)} '
                   '-I -B "${0%/*}/cargo_stub.py" "$@"\n')
        (self.root / "bin/cargo").chmod(0o700)

    def write(self, relative_path, contents):
        path = self.root / relative_path
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(contents, encoding="utf-8", newline="\n")

    def run_cli(self):
        self.write("cargo.json", json.dumps(self.trees))
        metadata_results = {
            "default": (0, json.dumps(self.metadata), ""),
            "all features": (0, json.dumps(self.all_features_metadata), ""),
            **self.metadata_failures,
        }
        self.write("metadata.json", json.dumps(metadata_results))
        self.write(MAIN_RS, self.source)
        self.write(UNWIND_RS, self.unwind_source)
        env = {
            **os.environ,
            "PATH": os.pathsep.join((str(self.root / "bin"), os.defpath)),
            **self.parent_env,
        }
        return subprocess.run(
            [sys.executable, "-I", "-B", str(self.script)],
            cwd=self.root, env=env, capture_output=True, text=True, encoding="utf-8",
            check=False, timeout=30,
        )

    def assert_passes(self):
        result = self.run_cli()
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertEqual(result.stderr, "")

    def assert_fails(self, *diagnostics):
        result = self.run_cli()
        output = result.stdout + result.stderr
        self.assertEqual(result.returncode, 1, output)
        for diagnostic in diagnostics:
            self.assertIn(diagnostic, output)

    def test_musl_tree_and_empty_gnu_tree_pass(self):
        self.assert_passes()

    def test_parent_color_always_passes(self):
        self.parent_env["CARGO_TERM_COLOR"] = "always"
        self.assert_passes()

    def test_parent_unicode_false_passes(self):
        self.parent_env["CARGO_TERM_UNICODE"] = "false"
        self.assert_passes()

    def test_musl_tree_without_mimalloc_fails(self):
        self.trees[MUSL] = (0, "recipe-server v0.1.0 (/workspace/crates/recipe-server)\n", "")
        self.assert_fails(MUSL, "mimalloc")

    def test_musl_tree_without_recipe_server_fails(self):
        self.trees[MUSL] = (0, "mimalloc v0.1.52\n└── other-crate v0.1.0\n", "")
        self.assert_fails(MUSL, "recipe-server")

    def test_gnu_tree_with_mimalloc_fails(self):
        self.trees[GNU] = TREE
        self.assert_fails(GNU, "mimalloc")

    def test_empty_musl_tree_fails(self):
        self.trees[MUSL] = EMPTY_TREE
        self.assert_fails(MUSL, "mimalloc")

    def test_missing_package_on_gnu_passes(self):
        self.trees[GNU] = MISSING_PACKAGE
        self.assert_passes()

    def test_missing_package_on_musl_fails(self):
        self.trees[MUSL] = MISSING_PACKAGE
        self.assert_fails(MUSL, "mimalloc")

    def test_other_cargo_error_fails_on_both_targets(self):
        for target in (MUSL, GNU):
            with self.subTest(target=target):
                self.trees = {MUSL: TREE, GNU: EMPTY_TREE, target: CARGO_ERROR}
                self.assert_fails(target, "failed to load manifest")

    def test_gnu_absence_diagnostic_requires_matching_streams_and_status(self):
        for result in (
            (0, EMPTY_TREE[2], ""),
            (0, TREE[1], EMPTY_TREE[2]),
            (101, "", EMPTY_TREE[2]),
            (0, "", MISSING_PACKAGE[2]),
        ):
            with self.subTest(result=result):
                self.trees[GNU] = result
                self.assert_fails(GNU)

    def test_musl_tree_with_cargo_error_fails(self):
        self.trees[MUSL] = (101, TREE[1], CARGO_ERROR[2])
        self.assert_fails(MUSL, "failed to load manifest")

    def test_measured_metadata_passes(self):
        self.assert_passes()
        calls = (self.root / "metadata-called").read_text(encoding="utf-8").splitlines()
        self.assertEqual(calls, ["default", "all features"])

    def test_mimalloc_v2_feature_fails(self):
        self.nodes[1]["features"] = ["default", "v2"]
        self.assert_fails(f"{MUSL} (default)", "mimalloc:", "['default', 'v2']")

    def test_mimalloc_secure_feature_fails(self):
        self.nodes[1]["features"] = ["default", "secure"]
        self.nodes[0]["features"] = ["secure"]
        self.assert_fails("mimalloc:", "['default', 'secure']")

    def test_mimalloc_feature_only_in_all_features_run_fails(self):
        nodes = self.all_features_metadata["resolve"]["nodes"]
        nodes[0]["features"] = ["secure"]
        nodes[1]["features"] = ["default", "secure"]
        self.assert_fails(f"{MUSL} (all features)", "mimalloc:", "['default', 'secure']")

    def test_libmimalloc_secure_feature_fails(self):
        self.nodes[0]["features"] = ["secure"]
        self.assert_fails("libmimalloc-sys:", "['secure']")

    def test_mimalloc_default_feature_with_members_fails(self):
        self.mimalloc_features["default"] = ["v2"]
        self.assert_fails("mimalloc:", "default", "['v2']")

    def test_missing_mimalloc_node_fails(self):
        self.nodes.pop(1)
        self.assert_fails("mimalloc:", "resolve node")

    def test_missing_libmimalloc_node_fails(self):
        self.nodes.pop(0)
        self.assert_fails("libmimalloc-sys:", "resolve node")

    def test_missing_resolve_graph_fails(self):
        del self.metadata["resolve"]
        self.assert_fails("Cargo metadata", "resolve graph")

    def assert_metadata_command_error(self, run):
        self.metadata_failures[run] = CARGO_ERROR
        result = self.run_cli()
        self.assertEqual(result.returncode, 2, result.stdout + result.stderr)
        self.assertIn(f"{MUSL} ({run}): Cannot read Cargo metadata", result.stderr)
        self.assertIn("failed to load manifest", result.stderr)

    def test_metadata_command_fails_with_cargo_error(self):
        self.assert_metadata_command_error("default")

    def test_all_features_metadata_command_fails_with_cargo_error(self):
        self.assert_metadata_command_error("all features")

    def test_missing_musl_attribute_fails(self):
        self.source = ALLOCATOR + STATIC
        self.assert_fails("main.rs", 'cfg(target_env = "musl")')

    def test_two_allocator_attributes_fail(self):
        self.source += ALLOCATOR + STATIC.replace("GLOBAL", "OTHER")
        self.assert_fails("main.rs", "global_allocator")

    def test_conditional_allocator_fails_in_both_files(self):
        other = "static OTHER: std::alloc::System = std::alloc::System;\n"
        fixtures = (
            ("Codex", '#[cfg_attr(not(target_env = "musl"), '
             'doc = concat!["Host"], global_allocator)]\n' + other),
            ("inner", 'mod extra {\n    #![cfg_attr(not(target_env = "musl"), '
             'global_allocator)]\n}\n'),
            ("outer", '#[cfg_attr(not(target_env = "musl"), '
             'cfg_attr(all(), doc = concat!["Host"], global_allocator))]\n' + other),
        )
        rule = "Declare exactly one #[global_allocator] attribute."
        for name, fixture in fixtures:
            with self.subTest(path=MAIN_RS, fixture=name):
                self.source = CFG + ALLOCATOR + STATIC + fixture
                self.assert_fails(f"{MAIN_RS}: {rule}")
            self.source = CFG + ALLOCATOR + STATIC
            with self.subTest(path=UNWIND_RS, fixture=name):
                self.unwind_source = UNWIND_CFG + ALLOCATOR + STATIC + fixture
                self.assert_fails(f"{UNWIND_RS}: {rule}")
            self.unwind_source = UNWIND_CFG + ALLOCATOR + STATIC

    def test_allocator_name_in_attribute_comments_and_strings_passes(self):
        for attribute in (
            '#[doc /* global_allocator */ = "Host"]\n',
            '#[doc = "global_allocator"]\n',
        ):
            with self.subTest(attribute=attribute):
                item = attribute + "const SAMPLE: bool = true;\n"
                self.source = CFG + ALLOCATOR + STATIC + item
                self.unwind_source = UNWIND_CFG + ALLOCATOR + STATIC + item
                self.assert_passes()

    def test_allocator_name_in_cfg_attr_comments_and_strings_passes(self):
        self.source += (
            '#[cfg_attr(test, /* global_allocator */ doc = "global_allocator")]\n'
            'const SAMPLE: bool = true;\n'
        )
        self.assert_passes()

    def test_system_allocator_type_fails_with_mimalloc_in_tree(self):
        self.source = CFG + ALLOCATOR + "static GLOBAL: std::alloc::System = std::alloc::System;\n"
        self.assert_fails("main.rs", "std::alloc::System")

    def test_wrong_allocator_value_fails(self):
        self.source = CFG + ALLOCATOR + STATIC.replace("= mimalloc::MiMalloc", "= OTHER")
        self.assert_fails("main.rs", "OTHER")

    def test_second_cfg_attribute_fails(self):
        self.source = CFG + ALLOCATOR + "#[cfg(debug_assertions)]\n" + STATIC
        self.assert_fails("main.rs", "cfg")

    def test_second_cfg_before_nested_doc_attribute_fails(self):
        self.source = '#[cfg(debug_assertions)]\n#[doc = concat!["Allocator"]]\n' + self.source
        self.assert_fails("main.rs", 'Put only #[cfg(target_env = "musl")]')

    def test_module_inner_cfg_passes(self):
        self.source = "mod extra { #![cfg(test)] }\n" + self.source
        self.assert_passes()

    def test_cfg_attr_that_can_disable_the_static_fails(self):
        self.source = '#[cfg_attr(test, cfg(any()))]\n' + self.source
        self.assert_fails("main.rs", "cfg")

    def test_commented_allocator_fails(self):
        source = self.source
        comments = ("// " + source.replace("\n", "\n// "), "/*\n/* nested */\n" + source + "*/")
        for comment in comments:
            with self.subTest(comment=comment):
                self.source = comment
                self.assert_fails("main.rs", "global_allocator")

    def test_allocator_inside_string_fails(self):
        self.source = 'const SAMPLE: &str = r#"\n' + self.source + '"#;\n'
        self.assert_fails("main.rs", "global_allocator")

    def test_allocator_inside_macro_input_fails(self):
        self.source = (
            "macro_rules! discard { ($($tokens:tt)*) => {} }\n"
            f"discard!(\n{self.source});\n"
        )
        self.assert_fails("main.rs", "global_allocator")

    def test_documented_allocator_passes(self):
        self.source = f"/// Describes the allocator.\n{CFG}/* Comment. */\n{ALLOCATOR}{STATIC}"
        self.assert_passes()

    def test_allocator_shadowing_fails_in_both_files(self):
        for declaration, rule in (
            ("mod mimalloc { pub use std::alloc::System as MiMalloc; }\n", "mod mimalloc"),
            ("use std::alloc as mimalloc;\n", "as mimalloc"),
            ("use std::alloc::System as MiMalloc;\n", "as MiMalloc"),
        ):
            with self.subTest(path=MAIN_RS, rule=rule):
                self.source = declaration + CFG + ALLOCATOR + STATIC
                self.assert_fails(f"{MAIN_RS}:", "Remove allocator shadowing.", rule)
            self.source = CFG + ALLOCATOR + STATIC
            with self.subTest(path=UNWIND_RS, rule=rule):
                self.unwind_source = UNWIND_CFG + declaration + ALLOCATOR + STATIC
                self.assert_fails(f"{UNWIND_RS}:", "Remove allocator shadowing.", rule)
            self.unwind_source = UNWIND_CFG + ALLOCATOR + STATIC

    def test_allocator_imports_fail_in_both_files(self):
        for imports, rule in (
            (SHADOWING_IMPORTS, "Remove imports that bind mimalloc."),
            (GLOB_IMPORTS, "Replace glob imports with explicit imports."),
        ):
            for item in imports:
                with self.subTest(path=MAIN_RS, item=item):
                    self.source = item + "\n" + CFG + ALLOCATOR + STATIC
                    self.assert_fails(f"{MAIN_RS}: {rule}")
                self.source = CFG + ALLOCATOR + STATIC
                with self.subTest(path=UNWIND_RS, item=item):
                    self.unwind_source = UNWIND_CFG + item + "\n" + ALLOCATOR + STATIC
                    self.assert_fails(f"{UNWIND_RS}: {rule}")
                self.unwind_source = UNWIND_CFG + ALLOCATOR + STATIC

    def test_allocator_imports_in_comments_and_strings_pass(self):
        imports = "\n".join(SHADOWING_IMPORTS + GLOB_IMPORTS)
        for ignored in (
            "\n".join(f"// {line}" for line in imports.splitlines()) + "\n",
            f"/*\n{imports}\n*/\n",
            f"const SAMPLE: &str = {json.dumps(imports)};\n",
            f'const SAMPLE: &str = r#"\n{imports}\n"#;\n',
        ):
            with self.subTest(ignored=ignored):
                self.source = ignored + CFG + ALLOCATOR + STATIC
                self.unwind_source = UNWIND_CFG + ignored + ALLOCATOR + STATIC
                self.assert_passes()

    def test_explicit_imports_that_bind_other_names_pass(self):
        imports = (
            "use mimalloc::MiMalloc;\n"
            "use crate::compat::mimalloc as other;\n"
            "use recipe_server::mimalloc::{self as allocator, Other};\n"
        )
        self.source = imports + self.source
        self.unwind_source = UNWIND_CFG + imports + ALLOCATOR + STATIC
        self.assert_passes()

    def test_lint_attribute_in_allocator_stack_fails_in_both_files(self):
        for name in ("allow", "expect", "r#expect"):
            attribute = f"#[{name}(dead_code)]\n"
            rule = f"Remove #[{name.removeprefix('r#')}] from the allocator static."
            for position, stack in (
                ("before", attribute + ALLOCATOR),
                ("after", ALLOCATOR + attribute),
            ):
                with self.subTest(path=MAIN_RS, name=name, position=position):
                    self.source = CFG + stack + STATIC
                    self.assert_fails(f"{MAIN_RS}: {rule}")
                self.source = CFG + ALLOCATOR + STATIC
                with self.subTest(path=UNWIND_RS, name=name, position=position):
                    self.unwind_source = UNWIND_CFG + stack + STATIC
                    self.assert_fails(f"{UNWIND_RS}: {rule}")
                self.unwind_source = UNWIND_CFG + ALLOCATOR + STATIC

    def test_shadowing_in_comments_and_strings_passes(self):
        for ignored in (
            "// mod mimalloc as mimalloc as MiMalloc\n",
            'const SAMPLE: &str = "mod mimalloc as mimalloc as MiMalloc";\n',
        ):
            with self.subTest(ignored=ignored):
                self.source = ignored + CFG + ALLOCATOR + STATIC
                self.unwind_source = UNWIND_CFG + ignored + ALLOCATOR + STATIC
                self.assert_passes()

    def test_lint_attribute_on_another_item_passes(self):
        for name in ("allow", "expect", "r#expect"):
            with self.subTest(name=name):
                item = f"#[{name}(dead_code)]\nconst SAMPLE: bool = true;\n"
                self.source = item + CFG + ALLOCATOR + STATIC
                self.unwind_source = UNWIND_CFG + item + ALLOCATOR + STATIC
                self.assert_passes()

    def test_unwind_without_allocator_fails(self):
        self.unwind_source = UNWIND_CFG
        self.assert_fails(f"{UNWIND_RS}:", "global_allocator")

    def test_unwind_without_crate_cfg_fails(self):
        self.unwind_source = ALLOCATOR + STATIC
        self.assert_fails(f"{UNWIND_RS}:", UNWIND_CFG.strip())

    def test_unwind_with_another_cfg_fails(self):
        for extra_cfg in (
            "#![cfg(debug_assertions)]\n",
            "#[cfg_attr(test, cfg(any()))]\n",
            "mod extra { #![cfg(test)] }\n",
        ):
            with self.subTest(extra_cfg=extra_cfg):
                self.unwind_source = UNWIND_CFG + extra_cfg + ALLOCATOR + STATIC
                self.assert_fails(f"{UNWIND_RS}:", UNWIND_CFG.strip())

    def test_unwind_with_musl_crate_cfg_passes(self):
        self.unwind_source = (
            UNWIND_CFG + "//! Unwind proof.\n\n" + ALLOCATOR + STATIC
        )
        self.assert_passes()

    def test_compose_allocator_variable_fails(self):
        self.write("deploy/compose.yaml", "environment:\n  - MIMALLOC_PURGE_DELAY=0\n")
        self.assert_fails("deploy/compose.yaml", "MIMALLOC_PURGE_DELAY")

    def test_lowercase_compose_allocator_variable_fails(self):
        self.write("deploy/compose.yaml", "environment:\n  - mimalloc_purge_delay=0\n")
        self.assert_fails("deploy/compose.yaml:2: Remove mimalloc_purge_delay.")

    def test_compose_unwind_variable_fails(self):
        self.write("deploy/compose.yaml", "environment:\n  - mimalloc_unwind=0\n")
        self.assert_fails("deploy/compose.yaml:2: Remove mimalloc_unwind.")

    def test_script_allocator_variable_fails(self):
        self.write("deploy/start.sh", "docker run -e MIMALLOC_PURGE_DELAY=0 app\n")
        self.assert_fails("deploy/start.sh", "MIMALLOC_PURGE_DELAY")

    def test_script_allocator_variable_attached_to_short_option_fails(self):
        self.write("deploy/start.sh", "docker run -eMIMALLOC_PURGE_DELAY=0 app\n")
        self.assert_fails("deploy/start.sh:1: Remove MIMALLOC_PURGE_DELAY.")

    def test_python_identifier_with_allocator_prefix_passes(self):
        self.write("deploy/ci/pin.py", "def check_mimalloc_unwind_step():\n    pass\n")
        self.assert_passes()

    def test_cargo_unwind_target_passes(self):
        for argument in (
            "--test mimalloc_unwind",
            "--test=mimalloc_unwind",
            '--test "mimalloc_unwind"',
            "--test 'mimalloc_unwind'",
        ):
            with self.subTest(argument=argument):
                self.write("deploy/ci/rust-tests.sh", f"cargo test {argument} --locked\n")
                self.assert_passes()

    def test_unwind_target_exception_keeps_rejecting_variables(self):
        path = "deploy/start.sh"
        for command, variable in (
            ("mimalloc_unwind=0", "mimalloc_unwind"),
            ("MIMALLOC_UNWIND=1", "MIMALLOC_UNWIND"),
            ("docker run -e MIMALLOC_X app", "MIMALLOC_X"),
            ("cargo test --test=mimalloc_unwind=0", "mimalloc_unwind"),
            ('cargo test --test "mimalloc_unwind"=0', "mimalloc_unwind"),
            ('cargo test --test "mimalloc_unwind_extra"', "mimalloc_unwind_extra"),
        ):
            with self.subTest(command=command):
                self.write(path, command + "\n")
                self.assert_fails(f"{path}:1: Remove {variable}.")

    def test_script_comment_with_allocator_variable_passes(self):
        self.write("deploy/start.sh", "  # Keep the default for MIMALLOC_PURGE_DELAY.\n")
        self.assert_passes()

    def test_scan_skips_only_the_checker_and_its_test(self):
        self.write("deploy/ci/test_allocator_rule.py", 'VARIABLE = "MIMALLOC_PURGE_DELAY"\n')
        self.write("deploy/ci/allocator_rule.py", self.script.read_text(encoding="utf-8")
                   + '\nVARIABLE = "MIMALLOC_PURGE_DELAY"\n')
        self.assert_passes()
        self.write("deploy/other/allocator_rule.py", 'VARIABLE = "MIMALLOC_PURGE_DELAY"\n')
        self.assert_fails("deploy/other/allocator_rule.py", "MIMALLOC_PURGE_DELAY")


if __name__ == "__main__":
    unittest.main()
