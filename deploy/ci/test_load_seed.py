"""Check the load seed through fake connections and database answers."""

import contextlib
import importlib.util
import io
import json
import pathlib
import tempfile
import unittest
import urllib.parse
from unittest.mock import Mock, patch


def response(status=200, body="", headers=()):
    return Mock(status=status, read=Mock(return_value=body.encode()),
                getheaders=Mock(return_value=list(headers)))


def form(action, version=None):
    fields = '<input name="csrf_token" value="seed-token">'
    if version is not None:
        fields += f'<input name="row_version" value="{version}">'
    return response(body=f'<form action="{action}">{fields}</form>')


class LoadSeedTests(unittest.TestCase):
    def setUp(self):
        path = pathlib.Path(__file__).with_name("load_seed.py")
        self.assertTrue(path.exists(), "load_seed.py does not exist")
        spec = importlib.util.spec_from_file_location("load_seed", path)
        self.seed = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(self.seed)
        self.state = pathlib.Path(self.enterContext(tempfile.TemporaryDirectory()))
        self.enterContext(patch("ssl.create_default_context"))
        self.sent = []
        self.additions = 0
        self.factory = Mock(side_effect=self.connection)
        self.client = self.seed.Client(self.state, connection_factory=self.factory)

    def connection(self, *args, **kwargs):
        connection = Mock()
        connection.request.side_effect = self.request
        connection.getresponse.side_effect = lambda: self.answer
        return connection

    def request(self, method, path, body, headers):
        fields = urllib.parse.parse_qs(body.decode()) if body else {}
        self.sent.append((method, path, fields, headers))
        if method == "GET":
            action = path.split("?")[0].removesuffix("/new")
            self.answer = form(action, 1 if path.endswith("?unit=U1") else None)
        elif path.endswith("/parameters"):
            self.additions += 1
            self.answer = response(headers=[("HX-Trigger", json.dumps({"rowVersion": self.additions + 1}))])
        else:
            destination = {"/machines": "/machines/M9000001",
                           "/machines/M9000001/types": "/machines/M9000001/types/ST_Load",
                           "/recipes": "/recipes/LOAD-1/versions/1"}[path]
            self.answer = response(303, headers=[("Location", destination)])

    def run_seed(self):
        output = io.StringIO()
        with contextlib.redirect_stdout(output):
            result = self.seed.seed(self.client, clock=Mock(side_effect=range(20)))
        return result, output.getvalue()

    def test_seed_defines_codesys_udt_with_eight_array_members(self):
        self.run_seed()
        posts = {path: fields for method, path, fields, _ in self.sent if method == "POST"}
        self.assertEqual(posts["/machines"]["plc_platform"], ["CodesysIec"])
        fields = posts["/machines/M9000001/types"]
        self.assertEqual(fields["name"], ["ST_Load"])
        members = {key: value for key, value in fields.items() if key.startswith("members[")}
        expected = {}
        for index in range(8):
            expected[f"members[{index}].name"] = [f"m{index + 1}"]
            expected[f"members[{index}].type"] = ["ARRAY[1..8] OF INT"]
        self.assertEqual(members, expected)
        self.assertEqual(fields["csrf_token"], ["seed-token"])

    def test_each_parameter_post_carries_values_limits_and_current_version(self):
        self.run_seed()
        posts = [item for item in self.sent if item[0] == "POST" and item[1].endswith("/parameters")]
        self.assertEqual(len(posts), 4000)
        for index, (_, _, fields, headers) in enumerate(posts, 1):
            self.assertEqual(fields["row_version"], [str(index)])
            self.assertEqual(fields["tag"], [f"P{index}"])
            self.assertEqual(headers["HX-Request"], "true")
        fields = posts[0][2]
        self.assertEqual(fields["type"], ["ST_Load@1"])
        self.assertEqual(fields["value_type"], ["ST_Load@1"])
        self.assertEqual(fields["unit_path"], ["U1"])
        self.assertNotIn("s7_address", fields)
        self.assertNotIn("fields", fields)
        values = {key: value for key, value in fields.items() if key.startswith("value.")}
        self.assertEqual(values, {f"value.m{m}[{i}]": ["1"] for m in range(1, 9) for i in range(1, 9)})
        limits = {key: value for key, value in fields.items() if key.startswith("limits[")}
        expected = {}
        for index in range(8):
            expected.update({f"limits[{index}].leaf_path": [f".m{index + 1}[*]"],
                             f"limits[{index}].min": ["0"], f"limits[{index}].max": ["100"]})
        self.assertEqual(limits, expected)
        self.assertFalse(any("/limits" in path for _, path, _, _ in self.sent))

    def test_progress_reports_each_500_completed_requests_and_elapsed_time(self):
        checkpoints = []
        output = Mock()
        output.write.side_effect = lambda text: checkpoints.append((self.additions, text)) if text.strip() else None
        with contextlib.redirect_stdout(output):
            result = self.seed.seed(self.client, clock=Mock(side_effect=range(20)))
        self.assertEqual([count for count, _ in checkpoints], list(range(500, 4001, 500)))
        for elapsed, (count, line) in enumerate(checkpoints, 1):
            self.assertIn(str(count), line)
            self.assertIn(f"{elapsed:.3f} s", line)
        self.assertEqual(result["seed_seconds"], 9)
        self.assertEqual(result["draft_path"], "/recipes/LOAD-1/versions/1")

    def test_count_check_accepts_exact_counts_and_names_each_wrong_count(self):
        counts = {"parameters": 4000, "leaves": 256000, "limit_rows": 32000}
        self.seed.check_counts(counts)
        for name in counts:
            for wrong in (counts[name] - 1, counts[name] + 1, None, True):
                with self.subTest(name=name, wrong=wrong):
                    with self.assertRaisesRegex(ValueError, name):
                        self.seed.check_counts(dict(counts, **{name: wrong}))

    def test_count_reader_uses_database_output_and_rejects_bad_answers(self):
        with patch.object(self.seed.subprocess, "run") as run:
            run.return_value = Mock(returncode=0, stdout="4000|256000|32000\n")
            self.assertEqual(self.seed.read_counts("LOAD-1"),
                             {"parameters": 4000, "leaves": 256000, "limit_rows": 32000})
            args, options = run.call_args
            self.assertIn("LOAD-1", args[0])
            self.assertIn("recipe_version", options["input"])
            self.assertIn("parameter_limit", options["input"])
            self.assertIn("jsonb_path_query", options["input"])
            self.assertIn("strict", options["input"])
            self.assertIn("v.status = 'draft'", options["input"])
            self.assertIn("v.deleted_at IS NULL", options["input"])
            for answer in ("", "4000|256000", "4000|bad|32000", "4000|256000|32000\n0|0|0"):
                run.return_value = Mock(returncode=0, stdout=answer)
                with self.assertRaisesRegex(ValueError, "counts"):
                    self.seed.read_counts("LOAD-1")
            run.return_value = Mock(returncode=1, stdout="hidden-value", stderr="hidden-value")
            with self.assertRaisesRegex(ValueError, "^cannot read draft counts$"):
                self.seed.read_counts("LOAD-1")

    def test_failed_parameter_stops_before_progress_and_hides_response_body(self):
        original = self.request

        def fail(method, path, body, headers):
            original(method, path, body, headers)
            if path.endswith("/parameters") and method == "POST":
                self.answer = response(422, "password-like-value E-RCP-021")

        self.request = fail
        output = io.StringIO()
        with contextlib.redirect_stdout(output):
            with self.assertRaisesRegex(self.seed.HttpCheckError, "HTTP 422, code E-RCP-021") as raised:
                self.seed.seed(self.client)
        self.assertNotIn("password-like-value", str(raised.exception))
        self.assertEqual(self.additions, 1)
        self.assertEqual(output.getvalue(), "")

    def test_main_checks_database_before_writing_seed_artifact(self):
        with patch.dict("os.environ", {"CI_STATE": str(self.state)}), \
                patch.object(self.seed, "Client", return_value=self.client), \
                patch.object(self.client, "login") as login, \
                patch.object(self.seed, "read_counts", return_value={"parameters": 4000, "leaves": 256000, "limit_rows": 31999}):
            with contextlib.redirect_stdout(io.StringIO()):
                with self.assertRaisesRegex(ValueError, "limit_rows"):
                    self.seed.main()
            login.assert_called_once_with("ci-admin", "new_password")
        self.assertFalse((self.state / "load-seed.json").exists())

    def test_cli_names_wrong_count_but_hides_untrusted_errors(self):
        self.assertTrue(hasattr(self.seed, "cli"), "the CLI must report count failures")
        try:
            self.seed.check_counts({"parameters": 4000, "leaves": 256000, "limit_rows": 31999})
        except ValueError as error:
            count_error = error
        for error, wanted in ((count_error, "limit_rows"),
                              (OSError("password-like-value"), "CI load seed failed.")):
            with patch.object(self.seed, "main", side_effect=error), \
                    contextlib.redirect_stderr(io.StringIO()) as output:
                self.assertEqual(self.seed.cli(), 1)
            self.assertIn(wanted, output.getvalue())
            self.assertNotIn("password-like-value", output.getvalue())

    def test_main_writes_only_checked_counts_and_seed_metadata(self):
        counts = {"parameters": 4000, "leaves": 256000, "limit_rows": 32000}
        with patch.dict("os.environ", {"CI_STATE": str(self.state)}), \
                patch.object(self.seed, "Client", return_value=self.client), \
                patch.object(self.client, "login"), \
                patch.object(self.seed, "read_counts", return_value=counts), \
                contextlib.redirect_stdout(io.StringIO()):
            self.seed.main()
        result = json.loads((self.state / "load-seed.json").read_text())
        self.assertEqual(result["counts"], counts)
        self.assertEqual(result["row_version"], 4001)
        self.assertGreaterEqual(result["seed_seconds"], 0)
        self.assertEqual(result["draft_path"], "/recipes/LOAD-1/versions/1")


if __name__ == "__main__":
    unittest.main()
