"""Check smoke artifacts before upload, as ADR-0128 (ak) requires.

Pass trace and report paths as arguments. Read passwords from the environment and the optional file.
Unpack archives in memory to keep their contents outside the upload directory.
"""

import base64
from html.parser import HTMLParser
import io
import os
import pathlib
import re
import sys
import zipfile


PASSWORD_NAMES = ("E2E_ADMIN_PASSWORD", "E2E_APPROVER_PASSWORD")
ZIP_DATA_URI = "data:application/zip;base64,"


class ReportTemplates(HTMLParser):
    def __init__(self):
        super().__init__()
        self.parts = None
        self.payloads = []
        self.outside = []

    def handle_starttag(self, tag, attrs):
        self.outside.append(self.get_starttag_text())
        if tag == "template" and dict(attrs).get("id") == "playwrightReportBase64":
            if self.parts is not None:
                raise ValueError("The secret scan cannot read a report template.")
            self.parts = []

    def handle_data(self, data):
        if self.parts is not None:
            self.parts.append(data)
        else:
            self.outside.append(data)

    def handle_endtag(self, tag):
        self.outside.append("</" + tag + ">")
        if tag == "template" and self.parts is not None:
            self.payloads.append("".join(self.parts).strip())
            self.parts = None

    def handle_comment(self, data):
        self.outside.append("<!--" + data + "-->")

    def handle_decl(self, decl):
        self.outside.append("<!" + decl + ">")

    def handle_pi(self, data):
        self.outside.append("<?" + data + ">")

    def unknown_decl(self, data):
        self.outside.append("<![" + data + "]>")


def read_passwords():
    passwords = tuple(os.environ.get(name, "") for name in PASSWORD_NAMES)
    if not all(passwords):
        raise ValueError("The secret scan requires both password variables.")
    one_time_file = os.environ.get("E2E_ONE_TIME_FILE")
    if one_time_file:
        try:
            one_time = pathlib.Path(one_time_file).read_text(encoding="utf-8").strip()
        except FileNotFoundError:
            one_time = ""
        if one_time:
            passwords += (one_time,)
    return tuple(password.encode("utf-8") for password in passwords)


def scan_html(data, passwords):
    text = data.decode("utf-8")
    templates = ReportTemplates()
    templates.feed(text)
    templates.close()
    if templates.parts is not None:
        raise ValueError("The secret scan cannot read a report template.")
    payloads = set()
    for payload in templates.payloads:
        if payload.lower().startswith(ZIP_DATA_URI):
            payload = payload[len(ZIP_DATA_URI):]
        payloads.add("".join(payload.split()))
    outside = "".join(templates.outside)
    for match in re.finditer(r'data:application/zip;base64,([^\s<>"\'`]*)', outside, re.IGNORECASE):
        payloads.add(match.group(1))
    for payload in payloads:
        scan_bytes(base64.b64decode(payload, validate=True), "embedded.zip", passwords)


def scan_bytes(data, name, passwords):
    if any(password in data for password in passwords):
        raise ValueError("The secret scan found a password.")
    suffix = pathlib.PurePosixPath(name).suffix.lower()
    if suffix in (".html", ".htm"):
        scan_html(data, passwords)
    source = io.BytesIO(data)
    if suffix == ".zip" or zipfile.is_zipfile(source):
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
