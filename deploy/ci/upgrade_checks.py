"""Check the guard statement, image tags, and exact backup results."""

import json
import pathlib
import re
import sys


IMAGES = {
    "app": "server",
    "migrate": "server",
    "backup": "pgtools",
    "backup-premigrate": "pgtools",
    "walarchive": "pgtools",
}


class PreviousRefError(ValueError):
    """Reject a previous ref before the image build."""


def select_previous_ref(upgrade_from, event_name, before):
    """Return the explicit ref, the push SHA, or None for the merge base."""
    if upgrade_from:
        return upgrade_from
    if event_name == "push":
        if before == "0" * 40:
            raise PreviousRefError("the push before commit is the zero SHA. Stop and report.")
        if re.fullmatch(r"[0-9a-fA-F]{40}", before) is None:
            raise PreviousRefError("the push before commit is not a valid SHA. Stop and report.")
        return before
    return None


def check_previous_commit(previous, head):
    """Compare commit IDs after Git resolves each ref."""
    if previous == head:
        raise PreviousRefError("the previous commit equals HEAD. Stop and report.")


def guard_statement(source):
    start = "CREATE FUNCTION audit_guard.audit_objects_dropped() RETURNS event_trigger"
    if source.count(start) != 1:
        raise ValueError("expected one guard function in 01-roles.sh")
    match = re.search(r"^" + re.escape(start) + r"\n.*?^\$\$;$", source, re.M | re.S)
    if match is None:
        raise ValueError("the guard function has no closing delimiter")
    return match.group().replace("CREATE FUNCTION", "CREATE OR REPLACE FUNCTION", 1)


def result_sets(output, message):
    rows = [json.loads(line) for line in output.splitlines() if line.startswith("{")]
    return [row.get("set") for row in rows if row.get("message") == message]


def check_restore(output, expected):
    if result_sets(output, "restore test passed") != [expected]:
        raise ValueError("restore-test did not pass for the exact expected set")


def backup_set(output, suffix):
    sets = result_sets(output, "backup completed")
    if len(sets) != 1 or not isinstance(sets[0], str):
        raise ValueError("expected one completed backup set")
    if re.fullmatch(r"recipe_[0-9]{8}_[0-9]{4}" + re.escape(suffix), sets[0]) is None:
        raise ValueError("the backup set has the wrong name or suffix")
    return sets[0]


def check_images(config, tag):
    for service, image in IMAGES.items():
        if config["services"][service]["image"] != f"recipe-{image}:{tag}":
            raise ValueError(f"the image tag of {service} differs from the release tag")


def pgdata_volume_name(config):
    """Return the volume name that the new stack gives to pgdata."""
    name = config["volumes"]["pgdata"]["name"]
    if not isinstance(name, str) or not name:
        raise ValueError("the pgdata volume has no name")
    return name


SEGMENT_PASSED = re.compile(r"CI: the segment check passed\. The segment [0-9A-F]{24} holds the system identifier of db\.")
SEGMENT_SKIPPED = "CI: the segment check does not run. The file .system-identifier exists."


def check_wal_identity_log(output, mode):
    """Check the log of the step of design D8 against the state of the file before the step."""
    lines = [line for line in output.splitlines() if line.startswith("CI: the segment check")]
    if mode == "ran":
        ok = len(lines) == 1 and SEGMENT_PASSED.fullmatch(lines[0]) is not None
    elif mode == "skipped":
        ok = lines == [SEGMENT_SKIPPED]
    else:
        raise ValueError("unknown segment check mode")
    if not ok:
        raise ValueError("the log of the segment check does not match the state of the identifier file")


def main():
    if sys.argv[1] == "previous-ref":
        print(select_previous_ref(*sys.argv[2:]) or "")
        return
    if sys.argv[1] == "previous-commit":
        check_previous_commit(*sys.argv[2:])
        return
    action, path, *arguments = sys.argv[1:]
    source = pathlib.Path(path).read_text(encoding="utf-8")
    if action == "guard":
        print(guard_statement(source))
    elif action == "restore":
        check_restore(source, arguments[0])
    elif action == "backup":
        print(backup_set(source, arguments[0]))
    elif action == "images":
        check_images(json.loads(source), arguments[0])
    elif action == "volume-name":
        print(pgdata_volume_name(json.loads(source)))
    elif action == "wal-identity":
        check_wal_identity_log(source, arguments[0])
    else:
        raise ValueError("unknown upgrade check")


if __name__ == "__main__":
    try:
        main()
    except PreviousRefError as error:
        print(f"CI: {error}", file=sys.stderr)
        sys.exit(1)
    except (ValueError, KeyError, OSError):
        print("CI: the upgrade gate failed. Check the input and the preceding job output.", file=sys.stderr)
        sys.exit(1)
