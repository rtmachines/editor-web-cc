"""Filter a breached-password list for the password policy of the recipe editor.

Run it as:

    python -I tools/breached-passwords/filter.py INPUT OUTPUT --source TEXT --commit SHA

The tool keeps the lines that have 12 to 128 characters after the line end is removed.
It drops duplicates and sorts the lines by byte order. It writes a header of comment
lines that start with "#", then one password for each line, to the OUTPUT path.

The tool treats INPUT as untrusted data. It reads the file as bytes, never runs it,
and skips every line that is not valid UTF-8, holds a control character, or starts with "#".
The loader of recipe-domain skips the lines that start with "#", so a password that starts
with "#" cannot stay in the list.

Only the Python standard library is used.
"""

import argparse
import sys
from pathlib import Path

MIN_CHARS = 12
MAX_CHARS = 128
LICENSE = "MIT"
COMMENT_PREFIX = "#"


def has_control_character(text):
    """Return True when the text holds a control character."""
    return any(ord(char) < 0x20 or ord(char) == 0x7F for char in text)


def decode_line(raw):
    """Return the text of one raw line without its line end, or None when it is not text."""
    line = raw.rstrip(b"\r\n")
    try:
        return line.decode("utf-8")
    except UnicodeDecodeError:
        return None


def keep(text):
    """Return True when the policy keeps this password in the list."""
    if text is None or text.startswith(COMMENT_PREFIX):
        return False
    if has_control_character(text):
        return False
    return MIN_CHARS <= len(text) <= MAX_CHARS


def filter_lines(raw_lines):
    """Return the kept passwords, without duplicates, in byte order."""
    kept = {text for text in map(decode_line, raw_lines) if keep(text)}
    return sorted(kept, key=lambda text: text.encode("utf-8"))


def header(source, commit, count):
    """Return the comment lines at the top of the output file."""
    return [
        "# Breached passwords for the password policy of the recipe editor (assumption A23).",
        f"# Source: {source}",
        f"# Commit: {commit}",
        f"# License: {LICENSE}",
        f"# Lines: {count}",
        f"# Filter: tools/breached-passwords/filter.py, {MIN_CHARS} to {MAX_CHARS} characters,"
        " unique, sorted by byte order",
    ]


def parse_arguments(argv):
    """Parse the command line."""
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("input", type=Path, help="path of the source list")
    parser.add_argument("output", type=Path, help="path of the filtered list")
    parser.add_argument("--source", required=True, help="path of the list in its repository")
    parser.add_argument("--commit", required=True, help="commit of the source repository")
    return parser.parse_args(argv)


def main(argv):
    """Read the source list and write the filtered list."""
    args = parse_arguments(argv)
    if args.input.resolve() == args.output.resolve():
        print("error: the input and the output are the same file", file=sys.stderr)
        return 2
    raw_lines = args.input.read_bytes().split(b"\n")
    passwords = filter_lines(raw_lines)
    lines = header(args.source, args.commit, len(passwords)) + passwords
    with args.output.open("w", encoding="utf-8", newline="\n") as handle:
        handle.write("\n".join(lines) + "\n")
    print(f"kept {len(passwords)} of {len(raw_lines)} lines")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
