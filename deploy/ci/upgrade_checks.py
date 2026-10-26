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


def main():
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
    else:
        raise ValueError("unknown upgrade check")


if __name__ == "__main__":
    try:
        main()
    except (ValueError, KeyError, OSError):
        print("CI: the upgrade gate failed. Check the input and the preceding job output.", file=sys.stderr)
        sys.exit(1)
