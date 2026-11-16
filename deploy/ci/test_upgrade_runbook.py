"""Check the runbook blocks of the upgrade without Docker."""

import os
import pathlib
import re
import shlex
import shutil
import subprocess
import tempfile
import unittest
from unittest import mock


def shell_function(name, file="upgrade.sh"):
    """Return the text of one function of the selected shell file."""
    source = pathlib.Path(__file__).with_name(file).read_text(encoding="utf-8")
    marker = f"\n{name}() {{"
    if marker not in source:
        raise AssertionError(f"{file} has no function {name}")
    return f"{name}() {{" + source.split(marker, 1)[1].split("\n}\n", 1)[0] + "\n}\n"


def runbook_heading_section(title, level):
    """Return one unique section, including its lower-level headings."""
    runbook = pathlib.Path(__file__).resolve().parents[2] / "docs" / "ops" / "runbook.md"
    lines = runbook.read_text(encoding="utf-8").splitlines()
    headings = []
    fence = None
    for index, line in enumerate(lines):
        stripped = line.strip()
        if fence is not None:
            if stripped.startswith(fence):
                fence = None
            continue
        if stripped.startswith(("```", "~~~")):
            fence = stripped[:3]
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
    bash = False
    for line in section.splitlines():
        stripped = line.strip()
        if fence is not None:
            if stripped.startswith(fence):
                if bash:
                    blocks.append("\n".join(body))
                fence = None
            else:
                body.append(line)
        elif stripped.startswith(("```", "~~~")):
            fence = stripped[:3]
            bash = stripped[3:].strip() == "bash"
            body = []
    return blocks


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
BACKUP_STOP = "docker compose stop backup"
ROLLBACK_CHECK = (
    "env -u COMPOSE_FILE RECIPE_VERSION=<new tag> docker compose -f compose.yaml "
    "run --rm restore-volume --check-only <stamp>"
)
DUMP_CHECK = (
    "docker compose exec -T -e LC_ALL=C backup sh -c "
    "'ls /backups/recipe_$(date -u +%Y%m%d)_[0-9][0-9][0-9][0-9].dump.sha256'"
)


def runbook_step(title, start):
    """Return a step up to its next unindented step number."""
    section = runbook_heading_section(title, 3)
    matches = list(re.finditer(r"^" + re.escape(start) + r"[^\n]*", section, re.M))
    if len(matches) != 1:
        raise AssertionError(f"{title}: {start!r} must occur exactly once")
    number = int(start.split(".", 1)[0])
    end = re.search(rf"^{number + 1}\.", section[matches[0].end():], re.M)
    if end is None:
        raise AssertionError(f"{title}: {start!r} has no next step")
    return section[matches[0].start():matches[0].end() + end.start()]


def stripped_bash_blocks(section):
    """Strip each command line, including tabs and carriage returns."""
    return [[line.strip() for line in block.splitlines()]
            for block in runbook_bash_blocks(section)]


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
        return runbook_step("Rollback", "1. Stop")

    def test_ci_rollback_check_equals_the_runbook_command(self):
        blocks = stripped_bash_blocks(self.rollback_stop())
        checks = [block[1] for block in blocks if len(block) == 2 and block[0] == BACKUP_STOP]
        self.assertEqual(len(checks), 1, "step 1: exactly one command-1 block")
        expected = checks[0].replace("<new tag>", "ci-upgrade").replace("<stamp>", '"$stamp"')
        script = shell_function("check_rollback_runbook")
        commands = [line.strip() for line in script.splitlines()
                    if "docker compose" in line and "restore-volume" in line]
        self.assertEqual(commands, [expected], "CI must run the exact runbook check command")

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
                            {BACKUP_STOP, ROLLBACK_CHECK, APP_REMOVE}, "step 1")

    def test_each_recovery_step_links_to_the_failed_section(self):
        for title, start in (("Restore into production", "4. Fill the new volume."),
                             ("Rollback", "3. Fill the new volume."),
                             ("Switch back", "8. Start"), ("Rollback", "1. Stop")):
            with self.subTest(section=title, step=start):
                self.assertIn(FAILED_LINK, runbook_step(title, start), "missing failed-restore link")

    def test_rollback_stop_has_one_remove_line_in_a_bash_block(self):
        blocks = stripped_bash_blocks(self.rollback_stop())
        count = sum(block.count(APP_REMOVE) for block in blocks)
        self.assertEqual(count, 1, "step 1: exactly one rm line in a bash block")

    def test_rollback_check_precedes_remove(self):
        blocks = stripped_bash_blocks(self.rollback_stop())
        check = [BACKUP_STOP, ROLLBACK_CHECK]
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
