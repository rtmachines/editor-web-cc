"""Fail when a release with a new PostgreSQL major version adds a migration (ADR-0103 (p))."""

import re
import subprocess
import sys


COMPOSE_PATH = "deploy/compose.yaml"
MIGRATIONS_DIR = "migrations/"
IMAGE_LINE = re.compile(r"^\s*image:\s*postgres:(\d+)\.", re.M)


def image_major(compose_text):
    """Return the major version of the one `postgres:<major>.` image line."""
    majors = IMAGE_LINE.findall(compose_text)
    if len(majors) != 1:
        raise ValueError("expected one postgres image line in compose.yaml")
    return majors[0]


def check(head_compose, tag_compose, added_files):
    """Return the added migration files when the major version differs."""
    if image_major(head_compose) == image_major(tag_compose):
        return []
    return list(added_files)


def git(*arguments):
    result = subprocess.run(["git", *arguments], capture_output=True, text=True, check=True)
    return result.stdout


def main():
    tag = git("describe", "--tags", "--abbrev=0", "--match", "v*", "HEAD^").strip()
    head_compose = git("show", f"HEAD:{COMPOSE_PATH}")
    tag_compose = git("show", f"{tag}:{COMPOSE_PATH}")
    added = git("diff", "--name-only", "--diff-filter=A", tag, "HEAD", "--", MIGRATIONS_DIR).split()
    failures = check(head_compose, tag_compose, added)
    for name in failures:
        print(f"CI: the major version of db changed since {tag}. New migration: {name}", file=sys.stderr)
    if failures:
        sys.exit(1)
    print(f"CI: migration freeze passed against {tag}.")


if __name__ == "__main__":
    try:
        main()
    except (ValueError, OSError, subprocess.CalledProcessError) as error:
        print(f"CI: the migration freeze check failed to run: {error}", file=sys.stderr)
        sys.exit(1)
