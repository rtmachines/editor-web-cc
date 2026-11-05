"""Check smoke artifacts before upload, as ADR-0128 (ak) requires.

Pass trace and report paths as arguments. Read both passwords from the environment.
Unpack archives in memory to keep their contents outside the upload directory.
"""

import io
import os
import pathlib
import sys
import zipfile


PASSWORD_NAMES = ("E2E_ADMIN_PASSWORD", "E2E_APPROVER_PASSWORD")


def read_passwords():
    passwords = tuple(os.environ.get(name, "") for name in PASSWORD_NAMES)
    if not all(passwords):
        raise ValueError("The secret scan requires both password variables.")
    return tuple(password.encode("utf-8") for password in passwords)


def scan_bytes(data, name, passwords):
    if any(password in data for password in passwords):
        raise ValueError("The secret scan found a password.")
    source = io.BytesIO(data)
    if pathlib.PurePosixPath(name).suffix.lower() == ".zip" or zipfile.is_zipfile(source):
        with zipfile.ZipFile(source) as archive:
            for member in archive.infolist():
                if not member.is_dir():
                    scan_bytes(archive.read(member), member.filename, passwords)


def scan_path(path, passwords):
    if path.is_symlink():
        raise ValueError("The secret scan refuses symbolic links.")
    if path.is_dir():
        for child in path.iterdir():
            scan_path(child, passwords)
    elif path.is_file():
        scan_bytes(path.read_bytes(), path.name, passwords)
    else:
        raise ValueError("The secret scan cannot read an input.")


def main():
    try:
        passwords = read_passwords()
        if not sys.argv[1:]:
            raise ValueError("The secret scan requires artifact paths.")
        for argument in sys.argv[1:]:
            scan_path(pathlib.Path(argument), passwords)
    except Exception:
        # Paths, archive names, and exception messages can contain passwords.
        print("CI secret scan failed. Do not upload the artifacts.", file=sys.stderr)
        return 1
    print("CI secret scan passed.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
