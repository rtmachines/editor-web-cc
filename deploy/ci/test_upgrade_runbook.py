"""Check the runbook blocks of the upgrade without Docker."""

import importlib.util
import os
import pathlib
import re
import shlex
import shutil
import subprocess
import tempfile
import unittest
from unittest import mock

spec = importlib.util.spec_from_file_location(
    "docker_calls", pathlib.Path(__file__).with_name("docker_calls.py"))
docker_calls = importlib.util.module_from_spec(spec)
spec.loader.exec_module(docker_calls)


def shell_function(name, file="upgrade.sh"):
    """Return the text of one function of the selected shell file."""
    source = pathlib.Path(__file__).with_name(file).read_text(encoding="utf-8")
    marker = f"\n{name}() {{"
    if marker not in source:
        raise AssertionError(f"{file} has no function {name}")
    return f"{name}() {{" + source.split(marker, 1)[1].split("\n}\n", 1)[0] + "\n}\n"


def runbook_fence(line, fence):
    """Return the next fence state as its full marker and info string."""
    if fence is not None:
        marker, _ = fence
        closing = rf" {{0,3}}{re.escape(marker[0])}{{{len(marker)},}}\s*"
        return None if re.fullmatch(closing, line) else fence
    opening = re.match(r"^ {0,3}(`{3,}|~{3,})(.*)$", line)
    if opening is None:
        return None
    marker, info = opening.groups()
    if marker[0] == "`" and "`" in info:
        return None
    return marker, info.strip()


def runbook_heading_section(title, level):
    """Return one unique section, including its lower-level headings."""
    runbook = pathlib.Path(__file__).resolve().parents[2] / "docs" / "ops" / "runbook.md"
    lines = runbook.read_text(encoding="utf-8").splitlines()
    headings = []
    fence = None
    for index, line in enumerate(lines):
        previous = fence
        fence = runbook_fence(line, fence)
        if previous is not None or fence is not None:
            continue
        heading = re.match(r"^(#+) ", line)
        if heading:
            headings.append((index, len(heading.group(1))))
    target = f"{'#' * level} {title}"
    matches = [index for index, _ in headings if lines[index].rstrip() == target]
    if len(matches) != 1:
        raise AssertionError(f"the heading {target!r} must occur exactly once, found {len(matches)}")
    start = matches[0]
    end = next((index for index, depth in headings if index > start and depth <= level), len(lines))
    return "\n".join(lines[start:end])


def runbook_bash_blocks(section):
    """Return Bash blocks, excluding text inside other fences."""
    blocks = []
    fence = None
    body = []
    for line in section.splitlines():
        previous = fence
        fence = runbook_fence(line, fence)
        if previous is not None:
            if fence is None:
                if previous[1] == "bash":
                    blocks.append("\n".join(body))
            else:
                body.append(line)
        elif fence is not None:
            body = []
    return blocks


def runbook_code_span_offsets(block):
    """Yield full code span bounds within one prose block, including delimiters."""
    token = re.compile(r"\\[\\`]|`+")
    offset = 0
    while opening := token.search(block, offset):
        offset = opening.end()
        if opening.group().startswith("\\"):
            continue
        # Inside a span, backslashes are literal, including before its closer.
        closing = next((end for end in re.finditer(r"`+", block[offset:])
                        if end.group() == opening.group()), None)
        if closing is None:
            continue
        offset += closing.end()
        yield opening.start(), offset


def runbook_inline_code_spans(text):
    """Return spans with matching backtick runs in prose blocks outside fences."""
    spans = []
    fence = None
    blocks = []
    lines = []
    for line in text.splitlines():
        previous = fence
        fence = runbook_fence(line, fence)
        heading = re.match(r"^ {0,3}#{1,6}(?:[ \t]|$)", line)
        boundary = heading or re.match(r"^ *(?:\||(?:[-*+]|[0-9]+[.)])(?: |$))", line)
        if previous is not None or fence is not None or not line.strip() or boundary:
            if lines:
                blocks.append("\n".join(lines))
                lines = []
        if previous is None and fence is None and line.strip():
            if heading:
                blocks.append(line)
            else:
                lines.append(line)
    if lines:
        blocks.append("\n".join(lines))
    for block in blocks:
        for start, end in runbook_code_span_offsets(block):
            content = block[start:end].strip("`").replace("\n", " ")
            if content.startswith(" ") and content.endswith(" ") and content.strip(" "):
                content = content[1:-1]
            spans.append(content)
    return spans


def runbook_deleted_service_starts(section, services=("app", "backup"),
                                   subcommands=("start", "restart")):
    """Return inline spans and fenced lines with forbidden Compose calls."""
    commands = runbook_inline_code_spans(section)
    fence = None
    continuation = False
    for line in section.splitlines():
        previous = fence
        fence = runbook_fence(line, fence)
        if previous is not None and fence is not None:
            if continuation:
                commands[-1] = commands[-1][:-1] + line
            else:
                commands.append(line)
            continuation = line.endswith("\\")
        else:
            continuation = False
    forbidden = []
    for command in commands:
        normalized = re.sub(r"<[A-Za-z][^<>`|;&()$'\"]*>", "PLACEHOLDER", command)
        for tokens in docker_calls.shell_commands(normalized):
            for index, token in enumerate(tokens):
                if token not in ("docker", "docker-compose"):
                    continue
                call = docker_calls.docker_call(tokens[index + 1:], compose=token == "docker-compose")
                if call is None:
                    continue
                is_compose, subcommand, operands = call
                if is_compose and docker_calls.forbidden_compose_call(
                        subcommand, operands, services=services, subcommands=subcommands):
                    forbidden.append(command)
                    break
            else:
                continue
            break
    return forbidden


class RunbookHostRebootTests(unittest.TestCase):
    required_sentences = (
        "From step 2 of [Restore into production](#restore-into-production) "
        "until the procedure starts app and backup again, the two services can have no container.",
        "The same holds from step 1 of [Recover from the WAL archive](#recover-from-the-wal-archive).",
        "From step 1 of [Major upgrade](#major-upgrade) until the procedure starts app again, app can have no container.",
        "In [Rollback](#rollback), step 1 of the restore of the _premigrate set deletes backup.",
        "From that step until the procedure starts app and backup again, backup can have no container.",
        "From the pass of command 1 in that step, app can have no container too.",
        "While app has no container, proxy can stay unhealthy, because its healthcheck calls http://app:9000/healthz.",
        "Skip step 4 in each window.",
        "Step 4 runs docker compose exec app.",
        "In each window, step 1 waits only for db and walarchive, "
        "and only for a service that docker compose ps -a lists.",
        "Step 4 of the recovery deletes the container of walarchive.",
        "The switch deletes the containers of db and walarchive, and later steps create them again.",
        "Until step 8 of the recovery deletes the container of db, "
        "skip this wait when db did not run at step 1 of the recovery.",
        "Such a db cannot become healthy, and the healthcheck of walarchive needs a running db.",
        "After step 1 of this section, continue the procedure at the step where it stopped.",
        "Two cases need an action before you continue:",
        "If the reboot stopped step 4 of the restore or step 6 of the recovery, the new volume is not complete.",
        # Two literals, so that deploy_values does not read this test text as a volume deletion.
        "Delete it with docker volume"
        " rm <new volume>.",
        "Then start again at step 3 of the restore or at step 5 of the recovery.",
        "If the reboot stopped step 7 of the restore or step 10 of the recovery, the shell lost the variable target.",
        "Set target to the directory that sub-step 1 made.",
        "If sub-step 1 made no directory, run sub-step 1 again.",
    )

    def prose_paragraphs(self):
        section = runbook_heading_section("After a host reboot", 3)
        prose = []
        fence = None
        for line in section.splitlines():
            previous = fence
            fence = runbook_fence(line, fence)
            if previous is not None:
                continue
            if fence is not None:
                prose.append("")
                continue
            prose.append(line)
        return re.split(r"\n\s*\n", "\n".join(prose))

    @staticmethod
    def sentence_count(paragraph):
        for start, end in reversed(list(runbook_code_span_offsets(paragraph))):
            paragraph = paragraph[:start] + paragraph[end:]
        return len(re.findall(r"\.(?:\s|$)", paragraph))

    def test_sentence_count_ignores_url_dots_and_code_spans(self):
        cases = (
            ("Read https://example.org/status. Run `one. two.` now.\n"
             "Call `http://app:9000/healthz`.", 3),
            ("Run ``a`b. c`` now.", 1),
            ("Run ``a`b.\nc`` now.", 1),
            ("Run ``a. b`c`` now.", 1),
            ("Run ``a.\nb`c`` now.", 1),
            ("Run `one.\ntwo` now.", 1),
            (r"Keep \`first. second` in prose.", 2),
        )
        for paragraph, expected in cases:
            with self.subTest(paragraph=paragraph):
                self.assertEqual(self.sentence_count(paragraph), expected,
                                 "sentence counter must ignore URL dots and periods inside code spans")

    def test_restore_and_recovery_window_has_conditional_wait_and_skip(self):
        text = " ".join("\n".join(self.prose_paragraphs()).replace("`", "").split())
        links = (
            "[Restore into production](#restore-into-production)",
            "[Recover from the WAL archive](#recover-from-the-wal-archive)",
            "[Major upgrade](#major-upgrade)",
            "[Rollback](#rollback)",
        )
        for link in links:
            self.assertIn(link, text, f"After a host reboot: missing link {link}")
        phrase = "until the procedure starts app and backup again"
        with self.subTest(phrase=phrase):
            self.assertIn(phrase, text, f"After a host reboot: missing phrase: {phrase}")
        for sentence in self.required_sentences:
            with self.subTest(sentence=sentence):
                self.assertIn(sentence, text, f"After a host reboot: missing sentence: {sentence}")
        old_texts = (
            "If db did not run at step 1 of the recovery, step 1 waits for no service.",
            "can never show healthy",
            "waits only until db and walarchive show healthy",
            "Before step 8 of the recovery, skip this wait when db did not run at step 1 of the recovery.",
            "If the reboot came between step 4 and step 8 of the recovery, run step 4 again first.",
            "Then continue the procedure at the step where it stopped.",
            "If the reboot came between step 4 and step 8 of the recovery, walarchive runs again.",
            "Run step 4 of the recovery again.",
            "Three cases need an action before you continue:",
            "In that window, proxy cannot become healthy",
            "Skip step 4 in that window.",
            "In that window, step 1 waits only for",
        )
        for old_text in old_texts:
            with self.subTest(old_text=old_text):
                self.assertNotIn(old_text, text, f"After a host reboot: obsolete text remains: {old_text}")

    def test_recovery_restart_instruction_follows_the_wait(self):
        text = " ".join("\n".join(self.prose_paragraphs()).replace("`", "").split())
        markers = (
            "In each window, step 1 waits only for",
            "After step 1 of this section, continue the procedure at the step where it stopped.",
            "If the reboot stopped step 4 of the restore or step 6 of the recovery, the new volume is not complete.",
        )
        for marker in markers:
            self.assertIn(marker, text, f"After a host reboot: missing order marker: {marker}")
        self.assertLess(text.index(markers[0]), text.index(markers[1]),
                        "After a host reboot: wait before continuing the procedure")
        self.assertLess(text.index(markers[1]), text.index(markers[2]),
                        "After a host reboot: handle the cases after the wait")

    def assert_block_sentence_limits(self, paragraph):
        if paragraph.startswith("- "):
            for item in re.split(r"\n(?=- )", paragraph):
                self.assertLessEqual(self.sentence_count(item), 3,
                                     f"After a host reboot: list item exceeds three sentences: {item}")
            return
        self.assertLessEqual(self.sentence_count(paragraph), 6,
                             f"After a host reboot: paragraph exceeds six sentences: {paragraph}")

    def test_list_sentence_limit_applies_to_each_item(self):
        self.assert_block_sentence_limits(
            "- First. Second. Third.\n"
            "- Fourth. Fifth. Sixth.\n"
            "- Seventh. Eighth. Ninth."
        )
        self.assert_block_sentence_limits("- First.\n  Second. Third.")
        with self.assertRaisesRegex(AssertionError, "list item exceeds three sentences"):
            self.assert_block_sentence_limits("- First.\n  Second. Third. Fourth.")
        self.assert_block_sentence_limits("First. Second. Third. Fourth. Fifth. Sixth.")
        with self.assertRaisesRegex(AssertionError, "paragraph exceeds six sentences"):
            self.assert_block_sentence_limits("First. Second. Third. Fourth. Fifth. Sixth. Seventh.")

    def test_window_paragraphs_have_at_most_six_sentences(self):
        for paragraph in self.prose_paragraphs():
            text = " ".join(paragraph.replace("`", "").split())
            if any(sentence in text for sentence in self.required_sentences):
                with self.subTest(paragraph=text):
                    self.assert_block_sentence_limits(paragraph)


class RunbookOverlayTests(unittest.TestCase):
    def test_leave_previous_overlay_block_equals_the_function(self):
        section = runbook_heading_section("Leave the file compose.previous.yaml", 4)
        blocks = runbook_bash_blocks(section)
        self.assertEqual(len(blocks), 1, "the section must hold exactly one bash block")
        script = shell_function("leave_previous_overlay", "upgrade-common.sh")
        self.assertEqual(blocks[0].strip(), script.strip())


FAILED_TITLE = "Failed rollback restore after a failed upgrade"
FAILED_LINK = f"[{FAILED_TITLE}](#failed-rollback-restore-after-a-failed-upgrade)"
BACKUP_START = "docker compose up -d --no-deps backup"
APP_REMOVE = "docker compose rm -s -f app"
BACKUP_REMOVE = "docker compose rm -s -f backup"
BACKUP_LOG = "docker compose logs --no-color backup > <file>"
ROLLBACK_CHECK = (
    "env -u COMPOSE_FILE RECIPE_VERSION=<new tag> docker compose -f compose.yaml "
    "run --rm restore-volume --check-only <stamp>"
)
DUMP_CHECK = (
    "docker compose exec -T -e LC_ALL=C backup sh -c "
    "'ls /backups/recipe_$(date -u +%Y%m%d)_[0-9][0-9][0-9][0-9].dump.sha256'"
)


def runbook_step(title, start, level=3):
    """Return a step up to its next unindented step number."""
    section = runbook_heading_section(title, level)
    matches = list(re.finditer(r"^" + re.escape(start) + r"[^\n]*", section, re.M))
    if len(matches) != 1:
        raise AssertionError(f"{title}: {start!r} must occur exactly once")
    number = int(start.split(".", 1)[0])
    end = re.search(rf"^{number + 1}\.", section[matches[0].end():], re.M)
    if end is None:
        raise AssertionError(f"{title}: {start!r} has no next step")
    return section[matches[0].start():matches[0].end() + end.start()]


class RunbookStepTests(unittest.TestCase):
    def test_step_reads_a_level_two_heading(self):
        section = "## Major upgrade\n\n1. Delete app.\nKeep the dump.\n\n2. Continue.\n"
        with mock.patch(f"{__name__}.runbook_heading_section", return_value=section) as heading:
            self.assertEqual(runbook_step("Major upgrade", "1. Delete", level=2),
                             "1. Delete app.\nKeep the dump.\n\n")
        heading.assert_called_once_with("Major upgrade", 2)

    def test_step_defaults_to_a_level_three_heading(self):
        section = "### Rollback\n\n1. Stop backup.\nCheck the set.\n\n2. Continue.\n"
        with mock.patch(f"{__name__}.runbook_heading_section", return_value=section) as heading:
            self.assertEqual(runbook_step("Rollback", "1. Stop"),
                             "1. Stop backup.\nCheck the set.\n\n")
        heading.assert_called_once_with("Rollback", 3)


def stripped_bash_blocks(section):
    """Strip each command line, including tabs and carriage returns."""
    return [[line.strip() for line in block.splitlines()]
            for block in runbook_bash_blocks(section)]


class RunbookMajorUpgradeDeleteTests(unittest.TestCase):
    def test_step_one_has_only_the_remove_command(self):
        step = runbook_step("Major upgrade", "1. Delete the container of `app`.", level=2)
        self.assertEqual(stripped_bash_blocks(step), [[APP_REMOVE]])

    def test_step_one_has_optional_log_copy(self):
        step = runbook_step("Major upgrade", "1. Delete the container of `app`.", level=2)
        self.assertIn("docker compose logs --no-color app > <file>",
                      runbook_inline_code_spans(step))

    def test_step_one_log_copy_precedes_app_remove(self):
        step = runbook_step("Major upgrade", "1. Delete the container of `app`.", level=2)
        log = "docker compose logs --no-color app > <file>"
        self.assertIn(log, runbook_inline_code_spans(step))
        blocks = runbook_bash_blocks(step)
        self.assertEqual(stripped_bash_blocks(step), [[APP_REMOVE]])
        self.assertLess(step.index(log), step.index(blocks[0]),
                        "step 1: copy the app log before deleting app")

    def test_failed_path_creates_app_on_the_old_volume(self):
        section = runbook_heading_section("Major upgrade", 2)
        paragraphs = [paragraph for paragraph in section.split("\n\n")
                      if paragraph.startswith("If a step before step 8 fails")]
        self.assertEqual(len(paragraphs), 1)
        self.assertIn("docker compose up -d --no-deps app",
                      runbook_inline_code_spans(paragraphs[0]))
        text = " ".join(paragraphs[0].replace("`", "").split())
        markers = ("change into the directory of <old tag>",
                   "docker compose up -d --no-deps app")
        positions = [text.find(marker) for marker in markers]
        message = "failed upgrade must select the old directory before creating app"
        self.assertNotIn(-1, positions, message)
        self.assertLess(positions[0], positions[1], message)

    def test_section_has_no_start_app_or_start_backup_command(self):
        section = runbook_heading_section("Major upgrade", 2)
        self.assertEqual(runbook_deleted_service_starts(section), [],
                         "major upgrade must use up -d --no-deps after the delete")

    def test_section_has_no_stop_app_command(self):
        section = runbook_heading_section("Major upgrade", 2)
        self.assertNotIn("docker compose stop app", section)


def assert_raw_commands(test, section, allowed, scope):
    """Check raw lines without interpreting Markdown or shell options."""
    for raw in section.splitlines():
        line = raw.strip()
        if re.search(r"\bdocker\b", line, re.ASCII | re.IGNORECASE):
            test.assertIn(line, allowed, f"{scope}: rule 1: unlisted command")
        test.assertFalse(line.endswith("\\"), f"{scope}: rule 2: continued line")


class FailedRollbackRunbookTests(unittest.TestCase):
    def failed_section(self):
        return runbook_heading_section(FAILED_TITLE, 4)

    def rollback_stop(self):
        return runbook_step("Rollback", "1. Delete the container of `backup`.")

    def test_ci_rollback_check_equals_the_runbook_command(self):
        blocks = stripped_bash_blocks(self.rollback_stop())
        checks = [block[1] for block in blocks if len(block) == 2 and block[0] == BACKUP_REMOVE]
        self.assertEqual(len(checks), 1, "step 1: exactly one command-1 block")
        expected = checks[0].replace("<new tag>", "ci-upgrade").replace("<stamp>", '"$stamp"')
        script = shell_function("check_rollback_runbook")
        commands = [line.strip() for line in script.splitlines()
                    if "docker compose" in line and "restore-volume" in line]
        self.assertEqual(commands, [expected], "CI must run the exact runbook check command")

    def test_ci_rollback_check_deletes_backup_and_creates_it_after_the_check(self):
        script = shell_function("check_rollback_runbook")
        lines = [line.strip() for line in script.splitlines()]
        remove = 'compose rm -s -f backup || fail "cannot delete backup before the runbook check"'
        self.assertTrue(remove in lines, "CI must delete backup before the runbook check")
        self.assertNotIn("compose stop backup", script)
        start = "compose up -d --no-deps --wait --wait-timeout 180 backup"
        self.assertIn(start, script)
        check = ('env -u COMPOSE_FILE RECIPE_VERSION=ci-upgrade docker compose -f compose.yaml '
                 'run --rm restore-volume --check-only "$stamp"')
        self.assertLess(script.index(remove), script.index(check))
        self.assertLess(script.index(') | tee "$CI_STATE/rollback-runbook.log"'), script.index(start))
        self.assertNotRegex(script, r"compose\s+(?:start|restart)\b[^\n]*\bbackup\b")

    def test_failed_section_has_three_bash_blocks(self):
        self.assertEqual(len(stripped_bash_blocks(self.failed_section())), 3)

    def test_failed_check_block(self):
        blocks = stripped_bash_blocks(self.failed_section())
        self.assertEqual(blocks[0], [BACKUP_START], "first block: failed check")

    def test_failed_restore_block(self):
        blocks = stripped_bash_blocks(self.failed_section())
        self.assertEqual(blocks[1], [APP_REMOVE, BACKUP_START], "second block: failed restore")

    def test_dump_check_block(self):
        blocks = stripped_bash_blocks(self.failed_section())
        self.assertEqual(blocks[2], [DUMP_CHECK], "third block: dump check")

    def test_failed_section_raw_commands(self):
        assert_raw_commands(self, self.failed_section(),
                            {BACKUP_START, APP_REMOVE, DUMP_CHECK}, "section")

    def test_rollback_stop_raw_commands(self):
        assert_raw_commands(self, self.rollback_stop(),
                            {BACKUP_REMOVE, BACKUP_LOG, ROLLBACK_CHECK, APP_REMOVE}, "step 1")

    def test_rollback_log_copy_precedes_command_one(self):
        blocks = stripped_bash_blocks(self.rollback_stop())
        log = [BACKUP_LOG]
        check = [BACKUP_REMOVE, ROLLBACK_CHECK]
        self.assertEqual(blocks.count(log), 1, "step 1: exactly one backup log block")
        self.assertEqual(blocks.count(check), 1, "step 1: missing command-1 block")
        self.assertLess(blocks.index(log), blocks.index(check))

    def test_rollback_backup_remove_equals_the_ci_command(self):
        blocks = stripped_bash_blocks(self.rollback_stop())
        checks = [block for block in blocks if ROLLBACK_CHECK in block]
        self.assertEqual(len(checks), 1, "step 1: exactly one command-1 block")
        self.assertEqual(checks[0][0], BACKUP_REMOVE)
        script = shell_function("check_rollback_runbook")
        self.assertIn(BACKUP_REMOVE.removeprefix("docker "), script)
        self.assertNotIn("compose stop backup", script)

    def test_rollback_has_no_start_of_a_deleted_service(self):
        section = runbook_heading_section("Rollback", 3)
        self.assertEqual(runbook_deleted_service_starts(section), [],
                         "rollback must use up -d --no-deps after the delete")

    def test_each_recovery_step_links_to_the_failed_section(self):
        for title, start in (("Restore into production", "4. Fill the new volume."),
                             ("Rollback", "3. Fill the new volume."),
                             ("Switch back", "8. Start"),
                             ("Rollback", "1. Delete the container of `backup`.")):
            with self.subTest(section=title, step=start):
                self.assertIn(FAILED_LINK, runbook_step(title, start), "missing failed-restore link")

    def test_rollback_stop_has_one_remove_line_in_a_bash_block(self):
        blocks = stripped_bash_blocks(self.rollback_stop())
        count = sum(block.count(APP_REMOVE) for block in blocks)
        self.assertEqual(count, 1, "step 1: exactly one rm line in a bash block")

    def test_rollback_check_precedes_remove(self):
        blocks = stripped_bash_blocks(self.rollback_stop())
        check = [BACKUP_REMOVE, ROLLBACK_CHECK]
        self.assertEqual(blocks.count(check), 1, "step 1: missing command-1 block")
        remove = next((index for index, block in enumerate(blocks) if APP_REMOVE in block), len(blocks))
        self.assertLess(blocks.index(check), remove, "step 1: command 1 must precede rm")

    def test_failed_section_boundaries(self):
        rollback = runbook_heading_section("Rollback", 3)
        leave = runbook_heading_section("Leave the file compose.previous.yaml", 4)
        failed = self.failed_section()
        self.assertTrue(rollback.endswith(failed), "failed section must end at Switch back")
        boundary = leave + "\n" + failed
        self.assertIn(boundary, rollback, "leave section must end at the failed section heading")
        self.assertNotIn("### Switch back", failed)

    def test_dump_glob_accepts_only_four_digits_without_a_suffix(self):
        shell = shutil.which("sh")
        if shell is None:
            self.skipTest("sh is not available")
        command = shlex.split(DUMP_CHECK)[-1].replace("/backups", ".")
        day = "20260102"
        with tempfile.TemporaryDirectory() as tools, tempfile.TemporaryDirectory() as cases:
            date = pathlib.Path(tools) / "date"
            date.write_text(f"#!/bin/sh\nprintf '%s\\n' '{day}'\n", encoding="utf-8")
            date.chmod(0o755)
            env = dict(os.environ, LC_ALL="C", PATH=tools + os.pathsep + os.environ.get("PATH", os.defpath))
            for stamp, accepted in (("0200", True), ("abcd", False), ("0200_premigrate", False)):
                with self.subTest(stamp=stamp):
                    directory = pathlib.Path(cases) / stamp
                    directory.mkdir()
                    name = f"recipe_{day}_{stamp}.dump.sha256"
                    (directory / name).touch()
                    done = subprocess.run([shell, "-c", command], cwd=directory, env=env,
                                          text=True, capture_output=True, check=False)
                    if accepted:
                        self.assertEqual(done.returncode, 0, done.stderr)
                        self.assertEqual(done.stdout.strip(), "./" + name)
                    else:
                        self.assertNotEqual(done.returncode, 0, f"glob accepted {name}")


if __name__ == "__main__":
    unittest.main()
