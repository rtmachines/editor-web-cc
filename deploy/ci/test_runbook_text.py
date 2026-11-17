"""Check the runbook text helpers without Docker."""

import importlib.util
import pathlib
import unittest
from unittest import mock


RUNBOOK_SPEC = importlib.util.spec_from_file_location(
    "test_upgrade_runbook", pathlib.Path(__file__).with_name("test_upgrade_runbook.py"))
runbook = importlib.util.module_from_spec(RUNBOOK_SPEC)
RUNBOOK_SPEC.loader.exec_module(runbook)

runbook_deleted_service_starts = runbook.runbook_deleted_service_starts
runbook_inline_code_spans = runbook.runbook_inline_code_spans
runbook_bash_blocks = runbook.runbook_bash_blocks
runbook_heading_section = runbook.runbook_heading_section


class RunbookDeletedServiceStartsTests(unittest.TestCase):
    def test_command_substitutions_keep_forbidden_calls(self):
        cases = (('out="$(docker compose start app)"', 'Run `{command}`.'),
                 ('x=`docker compose start app`', '```bash\n{command}\n```'),
                 ('echo "$(docker compose start app)"', 'Run `{command}`.'))
        for command, template in cases:
            with self.subTest(command=command):
                self.assertEqual(runbook_deleted_service_starts(
                    template.format(command=command)), [command])

    def test_docker_global_options_keep_forbidden_compose_calls(self):
        commands = ("docker --context x compose start app",
                    "docker -c x compose start app",
                    "docker --host=unix:///x compose restart backup",
                    "docker -H x compose start app")
        for command in commands:
            with self.subTest(command=command):
                self.assertEqual(runbook_deleted_service_starts(f"Run `{command}`."), [command])

    def test_docker_global_options_allow_compose_logs(self):
        command = "docker --context x compose logs app"
        self.assertEqual(runbook_deleted_service_starts(f"Run `{command}`."), [])

    def test_global_options_before_start_and_restart(self):
        options = ("--project-name", "-p", "-f", "--file", "--profile", "--ansi",
                   "--env-file", "--project-directory", "--progress", "--parallel")
        for option in options:
            forms = (f"{option} x", f"{option}=x") if option.startswith("--") else (f"{option} x",)
            for form in forms:
                for subcommand in ("start", "restart"):
                    command = f"docker compose {form} {subcommand} proxy backup"
                    with self.subTest(command=command):
                        self.assertEqual(runbook_deleted_service_starts(f"Run `{command}`."), [command])

    def test_forbidden_commands_and_each_compose_pair(self):
        commands = ("docker compose -f x start app",
                    "docker compose --profile=x start backup",
                    "docker compose restart app",
                    "docker compose -p x restart proxy backup",
                    "docker compose start proxy backup && echo ok",
                    "docker-compose --compatibility restart backup",
                    "docker compose logs app && docker compose restart app",
                    "docker-compose start app && docker compose restart backup")
        for command in commands:
            with self.subTest(command=command):
                self.assertEqual(runbook_deleted_service_starts(f"Run `{command}`."), [command])

    def test_commands_in_fences_with_any_info_string(self):
        command = "docker compose start app"
        for info in ("", "text", "sql", "yaml", "bash"):
            for marker in ("```", "~~~"):
                with self.subTest(info=info, marker=marker):
                    section = f"{marker}{info}\n{command}\n{marker}"
                    self.assertEqual(runbook_deleted_service_starts(section), [command])

    def test_fenced_lines_join_at_a_final_backslash(self):
        command = "docker compose \\\n  start app"
        for info in ("text", "bash"):
            with self.subTest(info=info):
                section = f"```{info}\n{command}\n```"
                self.assertEqual(runbook_deleted_service_starts(section), [command.replace("\\\n", "")])

    def test_allowed_commands(self):
        commands = ("docker compose start", "docker compose restart",
                    "docker compose up -d --no-deps app", "docker compose start proxy",
                    "docker compose logs --no-color app backup > start.txt",
                    "docker compose -f /srv/start/app/compose.yaml rm -s -f app",
                    "docker compose -f 'start app' rm -s -f app",
                    "docker compose start app-copy backup-copy")
        for command in commands:
            with self.subTest(command=command):
                self.assertEqual(runbook_deleted_service_starts(f"Run `{command}`."), [])

    def test_shell_operators_end_the_service_list(self):
        for operator in ("&&", "||", ";", "|"):
            command = f"docker compose start proxy {operator} echo app backup"
            with self.subTest(operator=operator):
                self.assertEqual(runbook_deleted_service_starts(f"Run `{command}`."), [])

    def test_redirections_keep_later_services(self):
        for operator in (">", ">start.txt", "<", "<start.txt"):
            command = f"docker compose start proxy {operator} echo app backup"
            with self.subTest(operator=operator):
                self.assertEqual(runbook_deleted_service_starts(f"Run `{command}`."), [command])

    def test_adjacent_shell_operators_keep_forbidden_commands(self):
        commands = ("docker compose start backup;", "docker compose start app;",
                    "docker compose start app&&echo", "$(docker compose start app)",
                    "true&&(docker compose start app);")
        for command in commands:
            with self.subTest(command=command):
                self.assertEqual(runbook_deleted_service_starts(f"Run `{command}`."), [command])

    def test_redirection_before_a_service_keeps_the_hit(self):
        command = "docker compose start >/dev/null app"
        self.assertEqual(runbook_deleted_service_starts(f"Run `{command}`."), [command])

    def test_hash_inside_a_word_keeps_the_later_command(self):
        command = "docker compose ps --filter label=a#b && docker compose start app"
        self.assertEqual(runbook_deleted_service_starts(f"Run `{command}`."), [command])

    def test_placeholders_and_input_redirections_keep_current_hits(self):
        commands = ("RECIPE_VERSION=<previous tag> docker compose start app",
                    "docker compose --env-file <file> start app",
                    "docker compose start 2>/dev/null app",
                    "docker compose exec -T db psql <dump.sql && docker compose start app >/dev/null",
                    "cat <in.txt | docker compose start app >/dev/null")
        for command in commands:
            with self.subTest(command=command):
                self.assertEqual(runbook_deleted_service_starts(f"Run `{command}`."), [command])

    def test_shell_quotes_and_whitespace_fallback(self):
        for command in ("docker compose -f 'compose file.yaml' start 'app'",
                        "docker compose start app 'unclosed"):
            with self.subTest(command=command):
                self.assertEqual(runbook_deleted_service_starts(f"Run `{command}`."), [command])


class RunbookDeletedServiceSelectionTests(unittest.TestCase):
    def test_window_subcommands_refuse_unsafe_calls(self):
        commands = ("docker compose up -d --no-deps app backup",
                    "docker compose run --rm backup sh", "docker compose up -d",
                    "docker compose up -d proxy",
                    "docker compose up -d --no-deps --wait-timeout 180",
                    "docker compose create --no-deps --pull never",
                    "docker compose run --rm --no-deps -v x:/data", "(docker compose up -d);")
        for command in commands:
            with self.subTest(command=command):
                self.assertEqual(runbook_deleted_service_starts(
                    f"Run `{command}`.", subcommands=("up", "create", "run", "start", "restart")),
                    [command])

    def test_window_subcommands_allow_safe_calls(self):
        commands = ("docker compose up -d --no-deps proxy", "docker compose start",
                    "docker compose up -d --no-deps --wait --wait-timeout 180 proxy",
                    'docker compose run --rm --no-deps -T -v '
                    '"<new volume>:/var/lib/postgresql/data" restore-volume <stamp>')
        for command in commands:
            with self.subTest(command=command):
                self.assertEqual(runbook_deleted_service_starts(
                    f"Run `{command}`.", subcommands=("up", "create", "run", "start", "restart")), [])

    def test_default_subcommands_allow_up(self):
        for command in ("docker compose up -d --no-deps app", "docker compose up -d"):
            with self.subTest(command=command):
                self.assertEqual(runbook_deleted_service_starts(f"Run `{command}`."), [])

    def test_selected_walarchive_start_has_one_hit(self):
        command = "docker compose start walarchive"
        self.assertEqual(
            runbook_deleted_service_starts(f"Run `{command}`.", services=("walarchive",)),
            [command],
        )

    def test_default_services_allow_walarchive_start(self):
        self.assertEqual(
            runbook_deleted_service_starts("Run `docker compose start walarchive`."),
            [],
        )

    def test_selected_services_allow_proxy_start(self):
        self.assertEqual(
            runbook_deleted_service_starts(
                "Run `docker compose start proxy`.", services=("app", "backup", "walarchive")),
            [],
        )


class RunbookInlineCodeSpansTests(unittest.TestCase):
    def test_list_items_end_code_spans(self):
        command = "docker compose start app"
        for marker in ("-", "*", "+", "1.", "1)", "   2."):
            for text in (f"{marker} Type a lone ` here.\n{marker} Run `{command}`.",
                         f"Text ` here.\n{marker}\nRun `{command}`."):
                with self.subTest(marker=marker, text=text):
                    self.assertEqual(runbook_inline_code_spans(text), [command])
                    self.assertEqual(runbook_deleted_service_starts(text), [command])

    def test_headings_and_table_rows_end_code_spans(self):
        command = "docker compose start app"
        for prefix in ("Text ` here\n## Heading ` x", "## Heading ` x",
                       "Text ` here\n## Heading", "| a ` b |", "   | a ` b |"):
            with self.subTest(prefix=prefix):
                tail = f"| Run `{command}` |" if "|" in prefix else f"Run `{command}`."
                text = f"{prefix}\n{tail}"
                self.assertEqual(runbook_inline_code_spans(text), [command])
                self.assertEqual(runbook_deleted_service_starts(text), [command])

    def test_wrapped_nonmarkers_keep_one_code_span(self):
        for continuation in ("--no-deps app", "-text", "*text", "+text", "1.text", "1)text"):
            with self.subTest(continuation=continuation):
                text = f"Run `docker compose up\n{continuation}`."
                self.assertEqual(runbook_inline_code_spans(text), [f"docker compose up {continuation}"])

    def test_two_spans_on_one_line(self):
        self.assertEqual(runbook_inline_code_spans("Run `first` and `second`."),
                         ["first", "second"])

    def test_spans_inside_fenced_blocks_do_not_count(self):
        for fence in ("```", "~~~"):
            with self.subTest(fence=fence):
                text = f"`before`\n  {fence}bash\nprintf `hidden`\n  {fence}\n`after`"
                self.assertEqual(runbook_inline_code_spans(text), ["before", "after"])

    def test_line_without_a_backtick_has_no_span(self):
        self.assertEqual(runbook_inline_code_spans("No code on this line."), [])

    def test_double_backtick_span(self):
        self.assertEqual(runbook_inline_code_spans("Run ``docker compose start app``."),
                         ["docker compose start app"])

    def test_span_can_hold_a_backtick(self):
        self.assertEqual(runbook_inline_code_spans("Run `` a`b ``."), ["a`b"])

    def test_unmatched_backtick_run_has_no_span(self):
        self.assertEqual(runbook_inline_code_spans("Run ``unmatched ` tail."), [])

    def test_only_an_exact_backtick_run_closes_a_span(self):
        self.assertEqual(runbook_inline_code_spans("`a``b`"), ["a``b"])

    def test_space_normalization_preserves_all_spaces_and_single_ends(self):
        self.assertEqual(runbook_inline_code_spans("`  ` ` a` `b ` `  c  `"),
                         ["  ", " a", "b ", " c "])

    def test_span_can_cross_a_line_break(self):
        for ending in ("\n", "\r\n", "\r"):
            with self.subTest(ending=ending):
                text = f"Run `docker compose{ending}start proxy backup`."
                self.assertEqual(runbook_inline_code_spans(text),
                                 ["docker compose start proxy backup"])

    def test_line_endings_become_spaces_without_collapsing_other_whitespace(self):
        text = "Run `\nfoo  \n bar\t\n` now."
        self.assertEqual(runbook_inline_code_spans(text), ["foo    bar\t"])

    def test_span_cannot_cross_a_paragraph_or_fence(self):
        for separator in ("\n\n", "\n \t\n", "\n```bash\n`hidden`\n```\n"):
            with self.subTest(separator=separator):
                self.assertEqual(runbook_inline_code_spans(f"`first{separator}second`"), [])

    def test_escaped_backtick_without_another_opener_has_no_span(self):
        # CommonMark 2.4: the escape leaves only one unescaped backtick, with no closer.
        self.assertEqual(runbook_inline_code_spans(r"\`docker compose start app`"), [])

    def test_escaped_backtick_before_an_unescaped_opener(self):
        # The first backtick is literal. The next backtick opens the span (CommonMark 2.4).
        self.assertEqual(runbook_inline_code_spans(r"Run \``docker compose start app`."),
                         ["docker compose start app"])

    def test_escaped_backslash_does_not_escape_the_opener(self):
        for count in (2, 4):
            with self.subTest(count=count):
                self.assertEqual(runbook_inline_code_spans("\\" * count + "`code`"), ["code"])
        self.assertEqual(runbook_inline_code_spans("\\" * 3 + "`code`"), [])

    def test_backslashes_inside_a_span_stay_literal(self):
        # CommonMark 6.1: a backslash cannot escape the closing backtick inside a span.
        self.assertEqual(runbook_inline_code_spans(r"`foo\`bar`"), ["foo\\"])
        self.assertEqual(runbook_inline_code_spans(r"``foo\`bar``"), [r"foo\`bar"])


class RunbookFenceTests(unittest.TestCase):
    def test_fences_keep_their_body_out_of_prose_and_inside_bash(self):
        cases = (
            ("````bash", "```\n`hidden`\n### Hidden", "````"),
            ("```bash", "```bash\n`hidden`\n### Hidden", "```"),
            ("~~~~bash", "~~~\n```\n`hidden`\n### Hidden", "~~~~"),
            ("   ````bash", "  ```\n`hidden`\n### Hidden", "   ````` \t"),
        )
        for opener, body, closer in cases:
            with self.subTest(opener=opener, body=body):
                text = f"#### Target\n`before`\n{opener}\n{body}\n{closer}\n`after`"
                self.assertEqual(runbook_inline_code_spans(text), ["before", "after"])
                self.assertEqual(runbook_bash_blocks(text), [body])
                with mock.patch.object(pathlib.Path, "read_text", return_value=text + "\n### Next"):
                    self.assertEqual(runbook_heading_section("Target", 4), text)

    def test_four_spaces_do_not_open_a_fence(self):
        text = "    ```bash\n`visible`"
        self.assertEqual(runbook_inline_code_spans(text), ["visible"])
        self.assertEqual(runbook_bash_blocks(text), [])


class RunbookHeadingSectionTests(unittest.TestCase):
    def section(self, text, title="Target", level=4):
        with mock.patch.object(pathlib.Path, "read_text", return_value=text):
            return runbook_heading_section(title, level)

    def test_text_fence_before_the_bash_block_keeps_the_scope(self):
        for fence in ("```", "~~~"):
            with self.subTest(fence=fence):
                section = f"#### Target\n  {fence}text\n# note\n  {fence}\n```bash\nprintf ok\n```"
                self.assertEqual(self.section(section + "\n### Next\n"), section)
                self.assertEqual(runbook_bash_blocks(self.section(section)), ["printf ok"])

    def test_crlf_copy_gives_the_same_section(self):
        text = "### Parent\n#### Target\n```text\n# note\n```\n##### Note\nbody\n#### Next\n"
        self.assertEqual(self.section(text.replace("\n", "\r\n")), self.section(text))

    def test_lower_level_heading_stays_in_the_scope(self):
        section = "#### Target\nbody\n##### Note\n```bash\nprintf ok\n```"
        self.assertEqual(self.section(section + "\n#### Next\n"), section)

    def test_same_or_higher_level_heading_ends_the_scope(self):
        for level in (1, 2, 3, 4):
            with self.subTest(level=level):
                self.assertEqual(self.section(f"#### Target\nbody\n{'#' * level} Next\ntail"),
                                 "#### Target\nbody")

    def test_each_level_matches_with_trailing_white_space(self):
        for level in range(1, 7):
            with self.subTest(level=level):
                section = f"{'#' * level} Target \t\nbody"
                self.assertEqual(self.section(section, level=level), section)

    def test_fenced_duplicate_and_fenced_end_do_not_change_the_scope(self):
        text = ("```text\n#### Target\n```\n#### Target\n~~~text\n"
                "#### Target\n### End\n~~~\nbody\n### Next\ntail")
        expected = "#### Target\n~~~text\n#### Target\n### End\n~~~\nbody"
        self.assertEqual(self.section(text), expected)

    def test_unclosed_fence_keeps_the_rest_of_the_file(self):
        text = "#### Target\n```text\n### Next\ntail"
        self.assertEqual(self.section(text), text)

    def test_missing_heading_fails(self):
        for text in ("### Target\n", " #### Target\n", "#### Target extra\n",
                     "```text\n#### Target\n```\n"):
            with self.subTest(text=text), self.assertRaisesRegex(AssertionError, "exactly once, found 0"):
                self.section(text)

    def test_duplicate_heading_fails_even_outside_the_first_scope(self):
        text = "#### Target\nbody\n### Next\n#### Target \t\n"
        with self.assertRaisesRegex(AssertionError, "exactly once, found 2"):
            self.section(text)



if __name__ == "__main__":
    unittest.main()
