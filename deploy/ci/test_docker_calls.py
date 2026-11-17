"""Check the shared shell lexer and Docker call parser without Docker."""

import ast
import importlib.util
import pathlib
import unittest

DOCKER_CALLS_SPEC = importlib.util.spec_from_file_location(
    "docker_calls", pathlib.Path(__file__).with_name("docker_calls.py"))
docker_calls = importlib.util.module_from_spec(DOCKER_CALLS_SPEC)


class ArgumentRecordsTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        DOCKER_CALLS_SPEC.loader.exec_module(docker_calls)

    def test_records_preserve_spaces_empty_arguments_and_newlines(self):
        data = b"3\x00a b\x00\x00x\ny\x00"
        self.assertEqual(docker_calls.argument_records(data), [["a b", "", "x\ny"]])

    def test_records_keep_call_boundaries_and_zero_arguments(self):
        data = b"0\x001\x00compose\x002\x00start\x00app\x00"
        self.assertEqual(docker_calls.argument_records(data),
                         [[], ["compose"], ["start", "app"]])
        self.assertEqual(docker_calls.argument_records(b""), [])

    def test_truncated_records_raise_value_error(self):
        for data in (b"1", b"1\x00", b"1\x00app", b"2\x00app\x00",
                     b"1\x00app\x002\x00backup\x00"):
            with self.subTest(data=data), self.assertRaises(ValueError):
                docker_calls.argument_records(data)

    def test_invalid_counts_raise_value_error(self):
        for data in (b"\x00", b"-1\x00", b"x\x00", b"+1\x00app\x00"):
            with self.subTest(data=data), self.assertRaises(ValueError):
                docker_calls.argument_records(data)


class ShellCommandsTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        DOCKER_CALLS_SPEC.loader.exec_module(docker_calls)

    def test_command_substitution_splits_into_simple_commands(self):
        self.assertEqual(docker_calls.shell_commands("$(docker compose start app)"),
                         [["$"], ["docker", "compose", "start", "app"]])

    def test_quoted_command_substitutions_expose_inner_commands(self):
        for command in ('out="$(docker compose start app)"',
                        'echo "$(docker compose start app)"'):
            with self.subTest(command=command):
                self.assertIn(["docker", "compose", "start", "app"],
                              docker_calls.shell_commands(command))

    def test_backtick_substitutions_expose_inner_commands(self):
        for command in ('x=`docker compose start app`',
                        'echo "`docker compose start app`"'):
            with self.subTest(command=command):
                self.assertIn(["docker", "compose", "start", "app"],
                              docker_calls.shell_commands(command))

    def test_nested_quoted_substitutions_expose_inner_commands(self):
        command = "echo \"$(echo '$(docker compose start app)')\""
        self.assertIn(["docker", "compose", "start", "app"],
                      docker_calls.shell_commands(command))

    def test_multiple_substitutions_expose_each_simple_command(self):
        command = 'echo "$(docker compose start app; docker compose restart backup)"'
        commands = docker_calls.shell_commands(command)
        self.assertIn(["docker", "compose", "start", "app"], commands)
        self.assertIn(["docker", "compose", "restart", "backup"], commands)

    def test_adjacent_control_operators_split_commands(self):
        self.assertEqual(docker_calls.shell_commands("docker compose start app&&echo"),
                         [["docker", "compose", "start", "app"], ["echo"]])

    def test_merged_control_runs_split_commands(self):
        self.assertEqual(docker_calls.shell_commands("true&&(docker compose start app);"),
                         [["true"], ["docker", "compose", "start", "app"]])

    def test_redirections_leave_later_operands(self):
        self.assertEqual(
            docker_calls.shell_commands("docker compose start 2>/dev/null >>log app"),
            [["docker", "compose", "start", "app"]])

    def test_quoted_operator_inside_a_word_stays_literal(self):
        self.assertEqual(docker_calls.shell_commands("docker compose -f 'a;b' start app"),
                         [["docker", "compose", "-f", "a;b", "start", "app"]])

    def test_unclosed_quote_uses_whitespace_fallback(self):
        self.assertEqual(docker_calls.shell_commands("docker compose start app 'unclosed"),
                         [["docker", "compose", "start", "app", "'unclosed"]])


class DockerCallTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        DOCKER_CALLS_SPEC.loader.exec_module(docker_calls)

    def test_compose_call_keeps_subcommand_operands(self):
        arguments = ["compose", "--compatibility", "start", "--attach", "app"]
        self.assertEqual(docker_calls.docker_call(arguments),
                         (True, "start", ["--attach", "app"]))

    def test_native_call_keeps_subcommand_operands(self):
        arguments = ["--context", "x", "container", "start", "app"]
        self.assertEqual(docker_calls.docker_call(arguments),
                         (False, "container", ["start", "app"]))
        self.assertEqual(docker_calls.docker_call(["start", "app"]),
                         (False, "start", ["app"]))

    def test_docker_value_options_skip_their_values(self):
        options = ("--context", "-c", "--host", "-H", "--config", "--log-level",
                   "-l", "--tlscacert", "--tlscert", "--tlskey")
        for option in options:
            with self.subTest(option=option):
                arguments = ["--debug", option, "start", "compose", "restart", "backup"]
                self.assertEqual(docker_calls.docker_call(arguments),
                                 (True, "restart", ["backup"]))

    def test_compose_value_options_skip_their_values(self):
        options = ("--project-name", "-p", "-f", "--file", "--profile", "--ansi",
                   "--env-file", "--project-directory", "--progress", "--parallel")
        for option in options:
            with self.subTest(option=option):
                arguments = ["compose", option, "start in checkout", "start", "app"]
                self.assertEqual(docker_calls.docker_call(arguments),
                                 (True, "start", ["app"]))

    def test_docker_short_option_clusters_skip_their_values(self):
        for options in (["-Dl", "debug"], ["-Dldebug"], ["-D"]):
            with self.subTest(options=options):
                arguments = options + ["compose", "start", "app"]
                self.assertEqual(docker_calls.docker_call(arguments),
                                 (True, "start", ["app"]))

    def test_equals_options_keep_the_next_token(self):
        cases = (["--host=unix:///x", "compose", "--file=x", "restart", "backup"],
                 ["--context=x", "compose", "--profile=x", "restart", "backup"])
        for arguments in cases:
            with self.subTest(arguments=arguments):
                self.assertEqual(docker_calls.docker_call(arguments),
                                 (True, "restart", ["backup"]))

    def test_compose_entry_starts_at_compose_options(self):
        arguments = ["--file", "start in checkout", "--compatibility", "start", "app"]
        self.assertEqual(docker_calls.docker_call(arguments, compose=True),
                         (True, "start", ["app"]))

    def test_missing_subcommand_returns_none(self):
        cases = ([], ["compose"], ["--context"], ["--context", "x"],
                 ["compose", "--file"], ["compose", "--file=x"])
        for arguments in cases:
            with self.subTest(arguments=arguments):
                self.assertIsNone(docker_calls.docker_call(arguments))
        self.assertIsNone(docker_calls.docker_call(["--file", "x"], compose=True))


class ForbiddenComposeCallTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        DOCKER_CALLS_SPEC.loader.exec_module(docker_calls)

    def test_short_option_clusters_consume_timeout_and_workdir_values(self):
        cases = (("up -dt 10 --no-deps", True),
                 ("up -dt 10 --no-deps proxy", False),
                 ("up -dt10 --no-deps", True),
                 ("run --rm --no-deps -Tw proxy", True))
        for command, expected in cases:
            subcommand, *arguments = command.split()
            with self.subTest(command=command):
                self.assertEqual(
                    docker_calls.forbidden_compose_call(subcommand, arguments), expected)

    def test_window_calls_require_no_deps_and_a_service(self):
        cases = (("up -d --no-deps --wait-timeout 180", True),
                 ("up -d --no-deps --wait-timeout 180 proxy", False),
                 ("up -d", True), ("up -d proxy", True),
                 ("run --rm --no-deps -w proxy", True),
                 ("up -d --no-deps -w proxy", False),
                 ("create --no-deps --pull=never proxy", False))
        for command, expected in cases:
            subcommand, *arguments = command.split()
            with self.subTest(command=command):
                self.assertEqual(
                    docker_calls.forbidden_compose_call(subcommand, arguments), expected)

    def test_window_calls_refuse_selected_services(self):
        for subcommand in ("up", "create", "run", "start", "restart"):
            for service in ("app", "backup"):
                with self.subTest(subcommand=subcommand, service=service):
                    self.assertTrue(docker_calls.forbidden_compose_call(
                        subcommand, ["--no-deps", service]))
            with self.subTest(subcommand=subcommand):
                self.assertFalse(docker_calls.forbidden_compose_call(
                    subcommand, ["--no-deps", "proxy"]))

    def test_no_deps_option_values_do_not_count_as_flags(self):
        cases = ("run --label --no-deps proxy", "run -l --no-deps proxy",
                 "up -d --scale --no-deps proxy", "create --scale --no-deps proxy")
        for command in cases:
            subcommand, *arguments = command.split()
            with self.subTest(command=command):
                self.assertTrue(docker_calls.forbidden_compose_call(subcommand, arguments))

    def test_real_no_deps_flags_allow_proxy_with_option_values(self):
        for subcommand, option in (("run", "--label"), ("run", "-l"),
                                   ("up", "--scale"), ("create", "--scale")):
            cases = (["--no-deps", option, "--no-deps", "proxy"],
                     [option, "--no-deps", "--no-deps", "proxy"])
            if option.startswith("--"):
                cases += ([f"{option}=--no-deps", "--no-deps", "proxy"],)
            for arguments in cases:
                with self.subTest(subcommand=subcommand, arguments=arguments):
                    self.assertFalse(docker_calls.forbidden_compose_call(
                        subcommand, arguments))

    def test_value_options_do_not_count_as_services(self):
        cases = {
            "up": ("--attach", "--exit-code-from", "--no-attach", "--pull",
                   "--scale", "-t", "--timeout", "--wait-timeout"),
            "create": ("--pull", "--scale"),
            "run": ("--cap-add", "--cap-drop", "--entrypoint", "-e", "--env",
                    "--env-from-file", "-l", "--label", "--name", "-p", "--publish",
                    "--pull", "-u", "--user", "-v", "--volume", "-w", "--workdir"),
        }
        for subcommand, options in cases.items():
            for option in options:
                with self.subTest(subcommand=subcommand, option=option):
                    self.assertTrue(docker_calls.forbidden_compose_call(
                        subcommand, ["--no-deps", option, "180"]))
                    self.assertFalse(docker_calls.forbidden_compose_call(
                        subcommand, ["--no-deps", option, "app", "proxy"]))
                    if option.startswith("--"):
                        self.assertTrue(docker_calls.forbidden_compose_call(
                            subcommand, ["--no-deps", f"{option}=180"]))
                        self.assertFalse(docker_calls.forbidden_compose_call(
                            subcommand, ["--no-deps", f"{option}=app", "proxy"]))

    def test_start_and_restart_allow_no_service(self):
        for subcommand in ("start", "restart"):
            with self.subTest(subcommand=subcommand):
                self.assertFalse(docker_calls.forbidden_compose_call(subcommand, []))

    def test_service_and_subcommand_selection(self):
        self.assertFalse(docker_calls.forbidden_compose_call("logs", ["app"]))
        self.assertFalse(docker_calls.forbidden_compose_call(
            "up", ["-d"], subcommands=("start", "restart")))
        self.assertFalse(docker_calls.forbidden_compose_call(
            "up", ["--no-deps", "app"], services=("backup",)))
        self.assertTrue(docker_calls.forbidden_compose_call(
            "restart", ["walarchive"], services=("walarchive",)))


class WindowCallsTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        DOCKER_CALLS_SPEC.loader.exec_module(docker_calls)

    def test_docker_calls_has_no_bare_assert(self):
        tree = ast.parse(pathlib.Path(docker_calls.__file__).read_text())
        self.assertEqual([node.lineno for node in ast.walk(tree)
                          if isinstance(node, ast.Assert)], [])

    def test_record_window_excludes_the_start_and_end_records(self):
        start = ("rm", ["-s", "-f", "walarchive"])
        end = ("up", ["-d", "--no-deps", "walarchive"])
        records = [["compose", "ps"], ["compose", start[0], *start[1]],
                   ["compose", "start", "app"], ["compose", end[0], *end[1]],
                   ["compose", "restart", "backup"]]
        cases = (({}, slice(None)), ({"start": start}, slice(2, None)),
                 ({"end": end}, slice(None, 3)),
                 ({"start": start, "end": end}, slice(2, 3)))
        for boundaries, expected in cases:
            with self.subTest(boundaries=boundaries):
                window = docker_calls.record_window(records, **boundaries)
                self.assertEqual(window, expected)
                self.assertEqual(records[window], records[expected])
        self.assertEqual(docker_calls.record_window([]), slice(None))

    def test_record_window_requires_each_boundary_exactly_once(self):
        boundaries = (("start", ("rm", ["-s", "-f", "walarchive"])),
                      ("end", ("up", ["-d", "--no-deps", "walarchive"])))
        for name, (subcommand, arguments) in boundaries:
            command = " ".join([subcommand, *arguments])
            duplicates = [["compose", subcommand, *arguments],
                          ["compose", "-f", "other.yaml", subcommand, *arguments]]
            for records, count in (([], 0), (duplicates, 2)):
                with self.subTest(boundary=name, count=count):
                    with self.assertRaisesRegex(AssertionError, rf"{command}.*{count}"):
                        docker_calls.record_window(
                            records, **{name: (subcommand, arguments)})

    def test_record_window_matches_parsed_compose_calls(self):
        start = ("rm", ["-s", "-f", "walarchive"])
        end = ("up", ["-d", "--no-deps", "walarchive"])
        records = [[], ["compose"], ["rm", "-s", "-f", "walarchive"],
                   ["compose", "rm", "-s", "-f", "backup"],
                   ["--context", "host", "compose", "-f", "a b.yaml",
                    "rm", "-s", "-f", "walarchive"],
                   ["compose", "ps"], ["compose", "up", "-d", "--no-deps", "backup"],
                   ["up", "-d", "--no-deps", "walarchive"],
                   ["-H", "host", "compose", "--project-name=recipe",
                    "--env-file", "recipe.env", "up", "-d", "--no-deps", "walarchive"]]
        self.assertEqual(docker_calls.record_window(records, start=start, end=end),
                         slice(5, 8))

    def test_window_returns_a_call_without_a_service_after_an_option_value(self):
        record = ["compose", "up", "-d", "--no-deps", "--wait-timeout", "180"]
        self.assertEqual(docker_calls.forbidden_window_calls([record]), [record])
        self.assertEqual(docker_calls.forbidden_window_calls([record + ["proxy"]]), [])

    def test_window_refuses_no_deps_values_in_argument_records(self):
        cases = ("compose run --label --no-deps proxy",
                 "compose up -d --scale --no-deps proxy",
                 "compose create --scale --no-deps proxy")
        for command in cases:
            record = command.split()
            fields = [str(len(record)), *record]
            data = b"\x00".join(word.encode() for word in fields) + b"\x00"
            records = docker_calls.argument_records(data)
            with self.subTest(command=command):
                self.assertEqual(docker_calls.forbidden_window_calls(records), [record])

    def test_window_rejects_an_empty_run_but_allows_an_empty_slice(self):
        with self.assertRaisesRegex(AssertionError, "The run has no Docker argument records"):
            docker_calls.forbidden_window_calls([])
        records = [["compose", "up", "-d", "--no-deps", "app"]]
        self.assertEqual(docker_calls.forbidden_window_calls(
            records, window=slice(0, 0)), [])

    def test_window_uses_docker_options_and_selected_services(self):
        record = ["--context", "x", "compose", "-f", "start in checkout",
                  "restart", "app"]
        records = [["start", "app"], ["compose"], record]
        self.assertEqual(docker_calls.forbidden_window_calls(records), [record])
        self.assertEqual(docker_calls.forbidden_window_calls(
            records, services=("backup",)), [])

    @staticmethod
    def check_records():
        return [
            ["compose", "exec", "-T", "db", "psql", "-c",
             "TRUNCATE public.session, public.login_attempt"],
            ["compose", "up", "-d", "--no-deps", "--wait", "--wait-timeout", "180", "app"],
            ["compose", "up", "-d", "--no-deps", "--wait", "--wait-timeout", "180", "proxy", "backup"],
        ]

    def test_check_allows_an_empty_prefix_and_calls_after_the_window(self):
        records = self.check_records()
        records.append(["compose", "start", "backup"])
        self.assertEqual(docker_calls.forbidden_check_window_calls(records), [])

    def test_check_returns_calls_before_truncate_and_backup_start(self):
        forbidden = ["compose", "up", "-d", "--no-deps", "app", "backup"]
        records = [forbidden, *self.check_records()]
        self.assertEqual(docker_calls.forbidden_check_window_calls(records), [forbidden])
        records = self.check_records()
        records.insert(2, ["compose", "start", "app"])
        records.insert(3, ["compose", "start", "backup"])
        self.assertEqual(docker_calls.forbidden_check_window_calls(records), [records[3]])

    def test_check_requires_the_truncate_and_both_exact_end_records(self):
        app_error = "The record after TRUNCATE must start app with the required options"
        backup_error = "check needs the end record that starts proxy and backup"
        cases = (([], "The run has no Docker argument records"),
                 (self.check_records()[1:], "check needs one TRUNCATE record"),
                 (self.check_records()[:-1], backup_error),
                 (self.check_records()[::2], app_error))
        for records, message in cases:
            with self.subTest(records=records), self.assertRaisesRegex(AssertionError, message):
                docker_calls.forbidden_check_window_calls(records)
        records = self.check_records()
        records.insert(1, ["compose", "ps"])
        with self.assertRaisesRegex(AssertionError, app_error):
            docker_calls.forbidden_check_window_calls(records)
        for index in (1, 2):
            records = self.check_records()
            records[index].remove("--no-deps")
            message = app_error if index == 1 else backup_error
            with self.subTest(index=index), self.assertRaisesRegex(AssertionError, message):
                docker_calls.forbidden_check_window_calls(records)


if __name__ == "__main__":
    unittest.main()
