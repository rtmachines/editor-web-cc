"""Check direct workspace dependency rules with Cargo metadata fixtures."""

import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest


SCRIPT = Path(__file__).with_name("dependency_rule.py")
CRATES = (
    "recipe-domain", "recipe-app", "recipe-db", "recipe-export",
    "recipe-web", "recipe-server",
)


class DependencyRuleTests(unittest.TestCase):
    def setUp(self):
        self.assertTrue(SCRIPT.is_file(), "D1-1 must add dependency_rule.py")
        self.metadata = {
            "packages": [
                {"id": name, "name": name, "dependencies": []}
                for name in CRATES
            ],
            "workspace_members": list(CRATES),
        }
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        self.fixture = Path(temporary.name) / "metadata.json"

    def add_dependency(self, crate, dependency, kind=None, **fields):
        package = next(p for p in self.metadata["packages"] if p["name"] == crate)
        package["dependencies"].append({"name": dependency, "kind": kind, **fields})

    def run_cli(self):
        self.fixture.write_text(
            json.dumps(self.metadata), encoding="utf-8", newline="\n",
        )
        return subprocess.run(
            [sys.executable, "-I", "-B", str(SCRIPT), str(self.fixture)],
            capture_output=True, text=True, encoding="utf-8", check=False, timeout=30,
        )

    def assert_passes(self):
        result = self.run_cli()
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertEqual(result.stdout.splitlines(), [
            "Dependency rule passed for 6 workspace crates.",
        ])
        self.assertEqual(result.stderr, "")

    def assert_fails(self, *edges):
        result = self.run_cli()
        self.assertEqual(result.returncode, 1, result.stdout + result.stderr)
        self.assertEqual(result.stdout.splitlines(), [
            f"{crate}: forbidden dependency {dependency}" for crate, dependency in edges
        ])
        self.assertEqual(result.stderr, "")

    def test_clean_graph_passes(self):
        for crate, dependency in (
            ("recipe-domain", "serde"), ("recipe-app", "recipe-domain"),
            ("recipe-app", "tokio"), ("recipe-db", "sqlx"),
            ("recipe-export", "recipe-domain"), ("recipe-web", "axum"),
            ("recipe-web", "recipe-export"), ("recipe-server", "recipe-db"),
        ):
            self.add_dependency(crate, dependency)
        self.assert_passes()

    def test_axum_under_recipe_app_fails(self):
        self.add_dependency("recipe-app", "axum")
        self.assert_fails(("recipe-app", "axum"))

    def test_tokio_under_recipe_domain_fails(self):
        self.add_dependency("recipe-domain", "tokio")
        self.assert_fails(("recipe-domain", "tokio"))

    def test_session_crates_under_recipe_web_fail(self):
        for dependency in ("tower-sessions", "axum-sessions", "async-session", "axum-login"):
            with self.subTest(dependency=dependency):
                self.metadata["packages"][4]["dependencies"] = []
                self.add_dependency("recipe-web", dependency)
                self.assert_fails(("recipe-web", dependency))

    def test_dev_dependency_passes(self):
        self.add_dependency("recipe-domain", "tokio", kind="dev")
        self.assert_passes()

    def test_transitive_edge_passes(self):
        self.add_dependency("recipe-app", "bridge")
        self.metadata["packages"].append({
            "id": "bridge", "name": "bridge",
            "dependencies": [{"name": "axum", "kind": None}],
        })
        self.assert_passes()

    def test_build_dependency_fails(self):
        self.add_dependency("recipe-app", "axum", kind="build")
        self.assert_fails(("recipe-app", "axum"))

    def test_renamed_dependency_fails(self):
        self.add_dependency("recipe-app", "axum", rename="http_adapter")
        self.assert_fails(("recipe-app", "axum"))

    def test_optional_target_dependency_fails(self):
        self.add_dependency("recipe-domain", "tokio", optional=True, target="cfg(unix)")
        self.assert_fails(("recipe-domain", "tokio"))

    def test_all_named_forbidden_edges_fail(self):
        expected = {
            "recipe-domain": ("sqlx", "axum", "tokio"),
            "recipe-app": ("sqlx", "axum", "recipe-export", "recipe-db"),
            "recipe-db": ("axum",),
            "recipe-export": ("sqlx", "axum", "recipe-app"),
            "recipe-web": ("sqlx",),
            "recipe-server": ("recipe-export", "sqlx"),
        }
        edges = []
        for crate, dependencies in expected.items():
            for dependency in dependencies:
                self.add_dependency(crate, dependency)
                edges.append((crate, dependency))
        self.assert_fails(*sorted(edges))

    def test_duplicate_edge_reports_once(self):
        self.add_dependency("recipe-app", "axum")
        self.add_dependency("recipe-app", "axum", kind="build")
        self.assert_fails(("recipe-app", "axum"))

    def test_nonmember_package_passes(self):
        self.metadata["workspace_members"].remove("recipe-domain")
        self.add_dependency("recipe-domain", "tokio")
        result = self.run_cli()
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertEqual(result.stdout, "Dependency rule passed for 5 workspace crates.\n")


if __name__ == "__main__":
    unittest.main()
