"""Check the source inventory with temporary Git repositories."""

import hashlib
import importlib.util
import pathlib
import subprocess
import tempfile
import unittest


MODULE = pathlib.Path(__file__).with_name("coverage_inventory.py").resolve()
PRODUCTION = "pub fn value() -> u8 { 1 }\n"


def load_inventory():
    spec = importlib.util.spec_from_file_location("coverage_inventory", MODULE)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


class InventoryTestCase(unittest.TestCase):
    def setUp(self):
        directory = tempfile.TemporaryDirectory()
        self.addCleanup(directory.cleanup)
        self.root = pathlib.Path(directory.name)
        self.inventory = load_inventory()

    def git(self, *arguments):
        subprocess.run(["git", "-C", str(self.root), *arguments], check=True, capture_output=True)

    def write(self, relative, content, track=True):
        path = self.root / relative
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(content.encode("utf-8"))
        if track:
            self.git("add", "--", relative)
        return path


class SourceInventoryTests(InventoryTestCase):
    def test_tracked_production_file_is_required_with_its_hash(self):
        self.git("init", "-q")
        self.write("crates/x/src/lib.rs", PRODUCTION)
        result = self.inventory.source_inventory(self.root, [])
        self.assertEqual(result["schema_version"], 1)
        self.assertEqual(len(result["files"]), 1)
        entry = result["files"][0]
        self.assertEqual(entry["path"], "crates/x/src/lib.rs")
        self.assertEqual(entry["sha256"], hashlib.sha256(PRODUCTION.encode()).hexdigest())
        self.assertEqual(entry["crate"], "x")
        self.assertEqual(entry["classification"], "required")

    def test_tracked_file_outside_src_is_not_listed(self):
        self.git("init", "-q")
        self.write("crates/x/tests/it.rs", PRODUCTION)
        self.write("crates/x/Cargo.toml", "[package]\n")
        self.assertEqual(self.inventory.source_inventory(self.root, [])["files"], [])

    def test_build_script_is_listed_apart_from_files(self):
        self.git("init", "-q")
        self.write("crates/sample/build.rs", PRODUCTION)
        self.write("crates/sample/src/build.rs", PRODUCTION)
        result = self.inventory.source_inventory(self.root, [])
        self.assertEqual(result["build_scripts"], ["crates/sample/build.rs"])
        self.assertEqual([item["path"] for item in result["files"]], ["crates/sample/src/build.rs"])

    def test_untracked_source_file_is_a_finding(self):
        self.git("init", "-q")
        self.write("crates/x/src/lib.rs", PRODUCTION)
        self.write("crates/x/src/extra.rs", PRODUCTION, track=False)
        result = self.inventory.source_inventory(self.root, [])
        self.assertEqual(result["untracked"], ["crates/x/src/extra.rs"])
        self.assertEqual([e["path"] for e in result["files"]], ["crates/x/src/lib.rs"])

    def test_directory_outside_git_raises_value_error(self):
        self.write("crates/x/src/lib.rs", PRODUCTION, track=False)
        with self.assertRaises(ValueError):
            self.inventory.source_inventory(self.root, [])


DECLARATION_ONLY = {
    "mod declaration": "mod parser;\npub(crate) mod config;\n",
    "inline mod": "pub mod inner {\n    use std::fmt;\n    pub struct A;\n}\n",
    "use": "use std::fmt::{self, Display};\n",
    "pub use": "pub use crate::value::Value;\npub(in crate::a) use super::b;\n",
    "extern crate": "extern crate alloc;\n",
    "attributes": "#![forbid(unsafe_code)]\n#[derive(Debug)]\npub struct A;\n",
    "comments": "//! Docs.\n/* block */\n/// Item.\npub struct A;\n\n",
    "struct": "pub struct A { pub x: u8 }\nstruct B(u8);\nstruct C;\n",
    "enum": "pub enum E { A, B(u8), C { x: u8 } }\n",
    "union": "pub union U { a: u8, b: u16 }\n",
    "type": "pub type Alias<T> = Vec<T>;\n",
    "trait without default bodies": (
        "pub trait T: Send {\n    type Out;\n    fn run(&self) -> u8;\n"
        "    const NAME: &'static str;\n}\n"
    ),
    "const literal": "pub const A: u8 = 1;\npub const B: &str = \"x\";\nconst C: f64 = -1.5e3;\n",
    "const path": "pub const A: u8 = u8::MAX;\nconst B: u8 = OTHER;\n",
    "const array": "pub const A: [u8; 3] = [1, 2, 3];\nconst B: [u8; 2] = [X, Y::Z];\n",
    "static literal": "pub static A: u8 = 1;\n",
    "static path": "pub static A: u8 = u8::MAX;\n",
    "enum discriminant": "pub enum E { A = 1, B = other::B as isize }\n",
    "array length": "pub type Bytes = [u8; 4];\npub type More = [u8; SIZE];\n",
    "tuple variants": "pub enum E { A(u8), B(Vec<u8>) }\npub struct S(pub u8);\n",
    "static array": "pub static A: [&str; 2] = [\"a\", \"b\"];\n",
}

REQUIRED = {
    "closure in const": "const A: fn() -> u8 = || 1;\n",
    "block in const": "const A: u8 = { 1 };\n",
    "call in const": "const A: u8 = make(1);\n",
    "macro call in static": "static A: &str = concat!(\"a\", \"b\");\n",
    "call in array": "const A: [u8; 1] = [make()];\n",
    "trait default fn": "pub trait T {\n    fn run(&self) -> u8 { 1 }\n}\n",
    "trait default const": "pub trait T {\n    const N: u8 = 1;\n}\n",
    "call in discriminant": "pub enum E { A = crate::compute() }\n",
    "call in array length": "pub type Bytes = [u8; crate::compute()];\n",
    "macro in array length": "pub type Bytes = [u8; size_of!()];\n",
    "macro in discriminant": "pub enum E { A = make!() }\n",
    "fn": "use a::b;\npub fn run() {}\n",
    "impl": "pub struct A;\nimpl A {}\n",
    "macro_rules": "macro_rules! m { () => {}; }\n",
    "item macro call": "thread_local! { static A: u8 = 1; }\n",
    "unknown form": "pub gadget A;\n",
    "fn inside inline mod": "mod m {\n    pub fn run() {}\n}\n",
    "const fn": "pub const fn run() -> u8 { 1 }\n",
}


class DeclarationOnlyTests(InventoryTestCase):
    def classification(self, content):
        self.git("init", "-q")
        self.write("crates/x/src/lib.rs", content)
        result = self.inventory.source_inventory(self.root, [])
        return result["files"][0]["classification"]

    def test_each_declaration_kind_is_declaration_only(self):
        for name, content in DECLARATION_ONLY.items():
            with self.subTest(name), tempfile.TemporaryDirectory() as directory:
                self.root = pathlib.Path(directory)
                self.assertEqual(self.classification(content), "declaration_only")

    def test_each_executable_form_is_required(self):
        for name, content in REQUIRED.items():
            with self.subTest(name), tempfile.TemporaryDirectory() as directory:
                self.root = pathlib.Path(directory)
                self.assertEqual(self.classification(content), "required")


class PolicyTests(InventoryTestCase):
    PATH = "crates/x/src/lib.rs"

    def entry(self, content=PRODUCTION, **changes):
        entry = {
            "path": self.PATH,
            "sha256": hashlib.sha256(content.encode()).hexdigest(),
            "configuration": "windows",
            "reason": "Compiled on Windows only.",
            "evidence": "lib.rs line 1: cfg(windows)",
        }
        return {**entry, **changes}

    def inventory_with(self, policy, content=PRODUCTION):
        self.git("init", "-q")
        self.write(self.PATH, content)
        return self.inventory.source_inventory(self.root, policy)

    def assert_policy_fault(self, policy, content=PRODUCTION):
        with self.assertRaises(ValueError):
            self.inventory_with(policy, content)

    def test_valid_entry_gives_other_configuration(self):
        result = self.inventory_with([self.entry()])
        self.assertEqual(result["files"][0]["classification"], "other_configuration")

    def test_stale_hash_is_a_fault(self):
        self.assert_policy_fault([self.entry(sha256="0" * 64)])

    def test_duplicate_path_is_a_fault(self):
        self.assert_policy_fault([self.entry(), self.entry()])

    def test_wildcard_is_a_fault(self):
        for path in ("crates/x/src/*.rs", "crates/x/src/li?.rs", "crates/x/src/[l]ib.rs"):
            with self.subTest(path), tempfile.TemporaryDirectory() as directory:
                self.root = pathlib.Path(directory)
                self.assert_policy_fault([self.entry(path=path)])

    def test_unused_entry_is_a_fault(self):
        self.assert_policy_fault([self.entry(path="crates/x/src/other.rs")])

    def test_empty_field_is_a_fault(self):
        for key in ("path", "sha256", "configuration", "reason", "evidence"):
            with self.subTest(key), tempfile.TemporaryDirectory() as directory:
                self.root = pathlib.Path(directory)
                self.assert_policy_fault([self.entry(**{key: "  "})])

    def test_missing_or_extra_key_is_a_fault(self):
        missing = self.entry()
        del missing["reason"]
        for entry in (missing, {**self.entry(), "note": "x"}):
            with self.subTest(sorted(entry)), tempfile.TemporaryDirectory() as directory:
                self.root = pathlib.Path(directory)
                self.assert_policy_fault([entry])

    def test_declaration_only_file_is_a_fault(self):
        content = "pub struct A;\n"
        self.assert_policy_fault([self.entry(content)], content)

    def test_test_only_file_is_a_fault(self):
        self.git("init", "-q")
        self.write(self.PATH, PRODUCTION)
        with self.assertRaises(ValueError):
            self.inventory.source_inventory(
                self.root, [self.entry()], lambda paths: {pathlib.Path(self.PATH)}
            )


if __name__ == "__main__":
    unittest.main()
