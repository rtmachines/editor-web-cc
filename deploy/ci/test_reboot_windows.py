"""Check service starts inside the reboot windows without Docker."""

import importlib.util
import pathlib
import re
import unittest
from unittest import mock

RUNBOOK_SPEC = importlib.util.spec_from_file_location(
    "test_upgrade_runbook", pathlib.Path(__file__).with_name("test_upgrade_runbook.py"))
runbook = importlib.util.module_from_spec(RUNBOOK_SPEC)
RUNBOOK_SPEC.loader.exec_module(runbook)
DOCKER_CALLS_SPEC = importlib.util.spec_from_file_location(
    "docker_calls", pathlib.Path(__file__).with_name("docker_calls.py"))
docker_calls = importlib.util.module_from_spec(DOCKER_CALLS_SPEC)
DOCKER_CALLS_SPEC.loader.exec_module(docker_calls)

ROLLBACK_CHECK = (
    "env -u COMPOSE_FILE RECIPE_VERSION=<new tag> docker compose -f compose.yaml "
    "run --rm restore-volume --check-only <stamp>")


def window_steps(test, text, label, numbers, indent="", *, strict=True,
                 number_type=int, check_step=None):
    steps = {}
    pattern = rf"(?ms)^{indent}(\d+)\. .*?(?=^{indent}\d+\. |\Z)"
    for match in re.finditer(pattern, text):
        number = number_type(match.group(1))
        if strict or number in numbers:
            test.assertNotIn(number, steps, f"duplicate {label} step {number}")
            steps[number] = match.group(0)
    if not strict:
        for number in numbers:
            test.assertIn(number, steps, f"{label} step {number} must exist")
    if check_step is not None:
        for number, step in steps.items():
            check_step(number, step)
    if strict:
        expected = set(numbers)
        test.assertEqual(set(steps), expected, f"unexpected {label} steps {set(steps) - expected}")
    return steps


def start_span(test, text, command, label):
    spans = [(start, end) for start, end in runbook.runbook_code_span_offsets(text)
             if text[start:end].strip("`") == command]
    test.assertEqual(len(spans), 1, f"{label} must have one {command!r}")
    return spans[0]


class RunbookMajorUpgradeWindowTests(unittest.TestCase):
    def test_major_upgrade_window_keeps_app_without_a_container(self):
        section = runbook.runbook_heading_section("Major upgrade", 2)
        start = "1. Delete the container of `app`."
        end = "Then run its sub-steps 4 to 7."
        for anchor in (start, end):
            self.assertEqual(section.count(anchor), 1, f"major upgrade must have one {anchor!r}")
        self.assertLess(section.index(start), section.index(end))
        window = section[section.index(start):section.index(end) + len(end)]

        def check_step(number, step):
            services = ("app",) if number in map(str, range(1, 8)) else ("app", "backup")
            parts = self.major_step_parts(step) if number == "4" else (step,)
            for part in parts:
                self.assertEqual(runbook.runbook_deleted_service_starts(
                    part, services=services, subcommands=("up", "create", "run", "start", "restart")),
                    [], f"major upgrade step {number} must keep {services!r} without a container")

        steps = window_steps(self, window, "major upgrade", tuple(map(str, range(1, 13))),
                             number_type=str, check_step=check_step)
        self.assertEqual(steps["12"].count(end), 1, f"major upgrade step 12 must have one {end!r}")

    def major_step_parts(self, step):
        paragraphs = list(re.finditer(
            r"(?m)^If a step before step 8 fails[^\n]*(?:\n[^\n]+)*", step))
        self.assertEqual(len(paragraphs), 1, "major upgrade must have one failed-step paragraph")
        paragraph = paragraphs[0]
        text = paragraph.group(0)
        command = "docker compose up -d --no-deps app"
        span_start, span_end = start_span(self, text, command, "major upgrade failed-step paragraph")
        return step[:paragraph.start() + span_start], step[paragraph.start() + span_end:]

    def test_major_upgrade_window_guard_rejects_runbook_mutants(self):
        section = runbook.runbook_heading_section("Major upgrade", 2)
        cases = (
            ("g_major_step3_app",
             "3. Run `docker compose exec backup restore-test`. Continue only after it passes.",
             " Then run `docker compose up -d --no-deps app`.", False,
             "docker compose up -d --no-deps app"),
            ("g_major_step12_up_all", "Then run its sub-steps 4 to 7.",
             "Run `docker compose up -d`. ", True, "docker compose up -d"),
            ("g_major_unexpected_step13", "\n4. Check that the line",
             "\n13. Run `docker compose up -d --no-deps app`.\n", True,
             "docker compose up -d --no-deps app"),
        )
        for name, anchor, insertion, before, command in cases:
            with self.subTest(mutant=name):
                self.assertEqual(section.count(anchor), 1, anchor)
                replacement = insertion + anchor if before else anchor + insertion
                mutant = section.replace(anchor, replacement, 1)
                with mock.patch.object(runbook, "runbook_heading_section", return_value=mutant):
                    with self.assertRaisesRegex(AssertionError, re.escape(command)):
                        self.test_major_upgrade_window_keeps_app_without_a_container()

    def test_major_upgrade_window_guard_rejects_duplicate_step_numbers(self):
        section = runbook.runbook_heading_section("Major upgrade", 2)
        anchor = "\n4. Check that the line"
        self.assertEqual(section.count(anchor), 1, anchor)
        mutant = section.replace(anchor, "\n3. Run the check." + anchor, 1)
        with mock.patch.object(runbook, "runbook_heading_section", return_value=mutant):
            with self.assertRaisesRegex(AssertionError, r"duplicate major upgrade step 3\b"):
                self.test_major_upgrade_window_keeps_app_without_a_container()


class RunbookRollbackWindowTests(unittest.TestCase):
    def test_section_has_no_stop_app_or_backup_command(self):
        section = runbook.runbook_heading_section("Rollback", 3)
        self.assertEqual(runbook.runbook_deleted_service_starts(
            section, services=("app", "backup"), subcommands=("stop",)), [])

    def test_rollback_stop_guard_parses_timeout(self):
        command = "docker compose stop -t 30 backup"
        with mock.patch.object(runbook, "runbook_heading_section",
                               return_value=f" Run `{command}`. "):
            with self.assertRaisesRegex(AssertionError, re.escape(command)):
                self.test_section_has_no_stop_app_or_backup_command()

    def test_rollback_window_keeps_app_and_backup_without_a_container(self):
        section = runbook.runbook_heading_section("Rollback", 3)
        start = "1. Delete the container of `backup`."
        end_step = '\n8. Record the values of sub-steps 1 to 3 of step 9 of "Restore into production".'
        for anchor in (start, end_step):
            self.assertEqual(section.count(anchor), 1, f"rollback must have one {anchor!r}")
        self.assertLess(section.index(start), section.index(end_step))
        tail = section[section.index(end_step):]
        end_block = re.search(r"(?ms)^```bash\n(.*?)^```[ \t]*(?:\n|\Z)", tail)
        self.assertIsNotNone(end_block, "rollback step 8 must have a Bash block")
        self.assertEqual(
            [line.strip() for line in end_block.group(1).strip().splitlines()],
            ["docker compose up -d --no-deps app", "docker compose up -d --no-deps proxy backup"])
        end = section.index(end_step) + end_block.start()
        window = section[section.index(start):end]

        def check_step(number, step):
            self.assertEqual(runbook.runbook_deleted_service_starts(
                step, services=("app", "backup"),
                subcommands=("up", "create", "run", "start", "restart")),
                [ROLLBACK_CHECK] if number == "1" else [],
                f"rollback step {number} must keep app and backup without a container")

        window_steps(self, window, "rollback", tuple(map(str, range(1, 9))),
                     number_type=str, check_step=check_step)

    def test_rollback_window_guard_rejects_runbook_mutants(self):
        section = runbook.runbook_heading_section("Rollback", 3)
        cases = (
            ("g_rollback_step2_backup",
             "2. Set `RECIPE_VERSION` in `.env` to the previous tag. "
             "Do this only after command 1 of step 1 passes.",
             " Then run `docker compose up -d --no-deps backup`.",
             "docker compose up -d --no-deps backup"),
            ("g_rollback_step7_app",
             "7. Truncate `session` and `login_attempt`, and re-apply the user changes.",
             " Run `docker compose up -d --no-deps app`.",
             "docker compose up -d --no-deps app"),
            ("g_rollback_unexpected_step9", "\n2. Set `RECIPE_VERSION` in `.env` to the previous tag. "
             "Do this only after command 1 of step 1 passes.",
             "\n9. Run `docker compose up -d --no-deps backup`.\n",
             "docker compose up -d --no-deps backup"),
        )
        for name, anchor, insertion, command in cases:
            with self.subTest(mutant=name):
                self.assertEqual(section.count(anchor), 1, anchor)
                mutant = section.replace(anchor, anchor + insertion, 1)
                with mock.patch.object(runbook, "runbook_heading_section", return_value=mutant):
                    with self.assertRaisesRegex(AssertionError, re.escape(command)):
                        self.test_rollback_window_keeps_app_and_backup_without_a_container()

    def test_restore_volume_has_no_service_dependencies(self):
        compose = pathlib.Path(__file__).resolve().parents[1] / "compose.yaml"
        blocks = re.findall(
            r"(?ms)^  restore-volume:\n(.*?)(?=^  [\w-]+:|^\S|\Z)",
            compose.read_text(encoding="utf-8"))
        self.assertEqual(len(blocks), 1, "compose must have one restore-volume block")
        block = blocks[0]
        self.assertNotRegex(
            block, r'''(?m)^[ \t]+['"]?(?:depends_on|links|volumes_from)['"]?[ \t]*:''')
        self.assertNotRegex(
            block, r'''(?m)^[ \t]+['"]?network_mode['"]?[ \t]*:[ \t]*['"]?service:''')


class RunbookRecoveryWalarchiveWindowTests(unittest.TestCase):
    def test_recovery_deletes_new_volume_only_in_step_7(self):
        section = runbook.runbook_heading_section("Recover from the WAL archive", 3)
        steps = window_steps(self, section, "recovery", (6, 7), strict=False)
        self.assertNotRegex(steps[6], r"docker\s+volume\s+(?:rm|remove)\b")
        self.assertIn("To end the attempt, delete the new volume.", steps[7])

    def test_recovery_delete_guard_rejects_variants(self):
        section = runbook.runbook_heading_section("Recover from the WAL archive", 3)
        anchor = "\n6. Fill the new volume."
        self.assertEqual(section.count(anchor), 1, anchor)
        for command in (
                "docker volume remove <new volume>",
                "docker  volume " "rm <new volume>"):
            with self.subTest(command=command):
                mutant = section.replace(anchor, anchor + f" Run `{command}`.", 1)
                with mock.patch.object(runbook, "runbook_heading_section", return_value=mutant):
                    with self.assertRaisesRegex(AssertionError, re.escape(command)):
                        self.test_recovery_deletes_new_volume_only_in_step_7()

    def test_section_has_no_stop_walarchive_command(self):
        section = runbook.runbook_heading_section("Recover from the WAL archive", 3)
        self.assertEqual(runbook.runbook_deleted_service_starts(
            section, services=("walarchive",), subcommands=("stop",)), [])

    def test_recovery_stop_guard_parses_compose_options(self):
        command = "docker compose -f compose.yaml stop walarchive"
        with mock.patch.object(runbook, "runbook_heading_section",
                               return_value=f" Run `{command}`. "):
            with self.assertRaisesRegex(AssertionError, re.escape(command)):
                self.test_section_has_no_stop_walarchive_command()

    def test_recovery_window_keeps_walarchive_without_a_container(self):
        recovery = runbook.runbook_heading_section("Recover from the WAL archive", 3)
        restore = runbook.runbook_heading_section("Restore into production", 3)
        start = "4. Delete the container of `walarchive`."
        reference = 'Run step 7 of "Restore into production".'
        for anchor in (start, reference):
            self.assertEqual(recovery.count(anchor), 1, f"recovery must have one {anchor!r}")
        self.assertLess(recovery.index(start), recovery.index(reference))
        window = recovery[recovery.index(start):recovery.index(reference) + len(reference)]

        def check_step(number, step):
            if number == 7:
                span_start, span_end = start_span(
                    self, step, "docker compose up -d --no-deps walarchive", "recovery step 7")
                parts = (step[:span_start], step[span_end:])
            elif number == 10:
                self.assertEqual(step.count(reference), 1, "recovery step 10 must have one restore reference")
                parts = (step[:step.index(reference)],)
            else:
                parts = (step,)
            for part in parts:
                self.assert_walarchive_absent(part, f"recovery step {number}")

        window_steps(self, window, "recovery", range(4, 11), check_step=check_step)
        steps = window_steps(self, restore, "restore", (3, 5, 6, 7), strict=False)
        for number in (3, 5, 6):
            self.assert_walarchive_absent(steps[number], f"restore step {number}")
        substep = window_steps(self, steps[7], "restore step 7", (7,), indent="   ", strict=False)[7]
        span_start, _ = start_span(
            self, substep, "docker compose up -d --no-deps walarchive", "restore sub-step 7.7")
        end = steps[7].index(substep) + span_start
        self.assert_walarchive_absent(steps[7][:end], "restore step 7 prefix")

    def assert_walarchive_absent(self, text, label):
        self.assertEqual(runbook.runbook_deleted_service_starts(
            text, services=("walarchive",), subcommands=("up", "create", "run", "start", "restart")),
            [], f"{label} must keep walarchive without a container")

    def test_recovery_window_guard_rejects_runbook_mutants(self):
        heading_section = runbook.runbook_heading_section
        title = "Recover from the WAL archive"
        section = heading_section(title, 3)
        command = "docker compose up -d --no-deps walarchive"
        cases = (
            ("h_recovery_unexpected_step11", "\n5. Create the new volume.",
             f"\n11. Run `{command}`.\n5. Create the new volume."),
            ("h_recovery_step5_walarchive",
             '5. Create the new volume. Run step 3 of "Restore into production". '
             "The command prints the name of the volume.",
             '5. Create the new volume. Run step 3 of "Restore into production". '
             f"The command prints the name of the volume. Run `{command}`."),
            ("h_recovery_step9_walarchive",
             '9. Set the role passwords. Run step 6 of "Restore into production". '
             "The cluster already holds",
             f'9. Set the role passwords. Run `{command}`. '
             'Run step 6 of "Restore into production". The cluster already holds'),
        )
        for name, anchor, replacement in cases:
            with self.subTest(mutant=name):
                self.assertEqual(section.count(anchor), 1, anchor)
                mutant = section.replace(anchor, replacement, 1)

                def mutated_section(requested_title, level):
                    return mutant if requested_title == title else heading_section(requested_title, level)

                with mock.patch.object(runbook, "runbook_heading_section", side_effect=mutated_section):
                    with self.assertRaisesRegex(AssertionError, re.escape(command)):
                        self.test_recovery_window_keeps_walarchive_without_a_container()

    def test_recovery_window_guard_rejects_restore_mutants(self):
        heading_section = runbook.runbook_heading_section
        title = "Restore into production"
        section = heading_section(title, 3)
        command = "docker compose up -d --no-deps walarchive"
        cases = (
            ("h_restore_step3_walarchive", "3. Create the new volume."),
            ("h_restore_step5_walarchive", "5. Run the switch (ADR-0103 (f))."),
            ("h_restore_step6_walarchive",
             "6. Write the start time into the change log ([rule](#change-log-of-the-site))."),
            ("h_restore_step7_prefix_walarchive",
             "7. Start the new WAL line before `app` (ADR-0103 (r), (s))."),
        )
        for name, anchor in cases:
            with self.subTest(mutant=name):
                self.assertEqual(section.count(anchor), 1, anchor)
                mutant = section.replace(anchor, anchor + f" Run `{command}`.", 1)

                def mutated_section(requested_title, level):
                    return mutant if requested_title == title else heading_section(requested_title, level)

                with mock.patch.object(runbook, "runbook_heading_section", side_effect=mutated_section):
                    with self.assertRaisesRegex(AssertionError, re.escape(command)):
                        self.test_recovery_window_keeps_walarchive_without_a_container()


class RollbackCheckScriptTests(unittest.TestCase):
    def test_check_rollback_runbook_keeps_backup_without_a_container(self):
        source = runbook.shell_function("check_rollback_runbook")
        delete = '  compose rm -s -f backup || fail "cannot delete backup before the runbook check"'
        restart = ('  compose up -d --no-deps --wait --wait-timeout 180 backup '
                   '|| fail "cannot restart backup after the runbook check"')
        lines = source.splitlines()
        for anchor in (delete, restart):
            self.assertEqual(lines.count(anchor), 1, f"rollback check must have one {anchor!r}")
        self.assertLess(lines.index(delete), lines.index(restart))
        window = "\n".join(lines[lines.index(delete) + 1:lines.index(restart)])
        hits = []
        for line in window.replace("\\\n", "").splitlines():
            for tokens in docker_calls.shell_commands(line):
                calls = [docker_calls.docker_call(tokens[index + 1:])
                         for index, token in enumerate(tokens) if token == "docker"]
                while tokens and (tokens[0] in (
                        "if", "then", "elif", "else", "while", "until", "do", "!", "command", "time")
                        or re.match(r"^[A-Za-z_][A-Za-z0-9_]*=", tokens[0])):
                    tokens = tokens[1:]
                if tokens and tokens[0] == "compose":
                    calls.append(docker_calls.docker_call(tokens[1:], compose=True))
                if any(call is not None and call[0] and docker_calls.forbidden_compose_call(
                        call[1], call[2], services=("backup",)) for call in calls):
                    hits.append(line.strip())
                    break
        self.assertEqual(hits, [
            "env -u COMPOSE_FILE RECIPE_VERSION=ci-upgrade docker compose -f compose.yaml "
            'run --rm restore-volume --check-only "$stamp"'])

    def assert_rollback_mutant_rejected(self, insertion):
        source = runbook.shell_function("check_rollback_runbook")
        anchor = '  compose rm -s -f backup || fail "cannot delete backup before the runbook check"'
        self.assertEqual(source.count(anchor), 1, anchor)
        mutant = source.replace(anchor, anchor + insertion, 1)
        with mock.patch.object(runbook, "shell_function", return_value=mutant):
            with self.assertRaisesRegex(AssertionError, re.escape("up -d --no-deps backup")):
                self.test_check_rollback_runbook_keeps_backup_without_a_container()

    def test_rollback_check_guard_rejects_backup_start_mutant(self):
        self.assert_rollback_mutant_rejected("\n  compose up -d --no-deps backup || true")

    def test_rollback_check_guard_rejects_continued_backup_start_mutant(self):
        self.assert_rollback_mutant_rejected("\n  compose \\\n    up -d --no-deps backup || true")

    def test_rollback_check_guard_rejects_prefixed_backup_start_mutants(self):
        for command in (
                "if compose up -d --no-deps backup; then :; fi",
                "if ! compose up -d --no-deps backup; then fail x; fi",
                "COMPOSE_X=1 compose up -d --no-deps backup || true",
                "command compose up -d --no-deps backup || true"):
            with self.subTest(command=command):
                self.assert_rollback_mutant_rejected("\n  " + command)


if __name__ == "__main__":
    unittest.main()
