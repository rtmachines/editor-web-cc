"""Check direct workspace dependencies against overview.md section 5."""

import argparse
import json
from pathlib import Path
import subprocess
import sys


# Source: docs/architecture/overview.md section 5, column "Forbidden".
# Review checks "any I/O" and "any port trait", which crate names cannot express.
# The session list names common libraries. Review checks other session libraries.
FORBIDDEN = {
    "recipe-domain": {"sqlx", "axum", "tokio"},
    "recipe-app": {"sqlx", "axum", "recipe-export", "recipe-db"},
    "recipe-db": {"axum"},
    "recipe-export": {"sqlx", "axum", "recipe-app"},
    "recipe-web": {
        "sqlx", "tower-sessions", "axum-sessions", "async-session", "axum-login",
    },
    "recipe-server": {"recipe-export", "sqlx"},
}


def read_metadata(path):
    """Read a JSON fixture or obtain metadata from the real workspace."""
    if path is not None:
        return json.loads(path.read_text(encoding="utf-8"))
    result = subprocess.run(
        ["cargo", "metadata", "--format-version", "1", "--locked"],
        cwd=Path(__file__).resolve().parents[2],
        capture_output=True, text=True, encoding="utf-8", check=True,
    )
    return json.loads(result.stdout)


def forbidden_edges(metadata):
    """Return each forbidden direct edge once, in a stable order."""
    members = set(metadata["workspace_members"])
    edges = set()
    for package in metadata["packages"]:
        if package["id"] not in members:
            continue
        crate = package["name"]
        for dependency in package["dependencies"]:
            # Dev dependencies are outside this rule. Cargo uses null for normal.
            if dependency["kind"] not in (None, "build"):
                continue
            # Cargo keeps the package name in "name", even for a renamed dependency.
            name = dependency["name"]
            if name in FORBIDDEN.get(crate, set()):
                edges.add((crate, name))
    return sorted(edges)


def main():
    """Exit 0 on success, 1 on forbidden edges, or 2 on input errors."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("metadata", nargs="?", type=Path, help="Cargo metadata JSON fixture")
    args = parser.parse_args()
    try:
        metadata = read_metadata(args.metadata)
        edges = forbidden_edges(metadata)
    except subprocess.CalledProcessError as error:
        print(f"Cannot read cargo metadata: {error.stderr.strip()}", file=sys.stderr)
        return 2
    except (OSError, ValueError, KeyError, TypeError) as error:
        print(f"Cannot read cargo metadata: {error}", file=sys.stderr)
        return 2
    if edges:
        for crate, dependency in edges:
            print(f"{crate}: forbidden dependency {dependency}")
        return 1
    print(f"Dependency rule passed for {len(metadata['workspace_members'])} workspace crates.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
