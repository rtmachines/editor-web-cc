"""Redact generated secrets before CI prints the Compose log."""

import json
import pathlib
import re
import sys


def main():
    root = pathlib.Path(sys.argv[1])
    values = []
    paths = list((root / "secrets").glob("*"))
    paths.extend((root / "state" / name) for name in ("admin_password", "new_password"))
    for path in paths:
        if path.is_file():
            values.extend(line for line in path.read_text().splitlines() if line)
    cookies = root / "state" / "cookies.json"
    if cookies.exists():
        values.extend(json.loads(cookies.read_text()).values())
    for name in sys.argv[2:] or ["compose.log"]:
        path = root / "state" / name
        if not path.exists():
            continue
        text = path.read_text(errors="replace")
        for value in sorted(values, key=len, reverse=True):
            text = text.replace(value, "[REDACTED]")
        text = re.sub(r"\b[0-9a-fA-F]{32,}\b", "[REDACTED]", text)
        # Stop a log line from becoming a GitHub workflow command.
        label = "compose" if name == "compose.log" else name
        for line in text.splitlines():
            print(label + " | " + line)


if __name__ == "__main__":
    try:
        main()
    except Exception:
        print("CI could not redact the Compose log.", file=sys.stderr)
        sys.exit(1)
