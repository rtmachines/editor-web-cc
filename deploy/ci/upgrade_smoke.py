"""Run Upgrade steps 21 and 22 with a fixed recipe fixture."""

import hashlib
import http.client
import importlib.util
import ipaddress
import json
import os
import pathlib
import socket
import sys


PARAMETER_KEY = "6e982764-4669-4d97-bc46-fbe01ce5dd1b"
PARAMETER_FIELDS = ("unit_path", "tag", "key", "type_id", "s7_address", "limits", "value")


def fixture_body():
    return {
        "machine": {"code": "M1", "plc_platform": "CodesysIec"},
        "recipe": {"reference_number": "CI-UPGRADE"},
        "version": {"number": 1},
        "types": [],
        "parameters": [{
            "unit_path": "U1", "tag": "SPEED", "key": PARAMETER_KEY,
            "type_id": "INT", "s7_address": None, "value": "7",
            "limits": [{"leaf_path": "", "min": "0", "max": "10", "max_length": None}],
        }],
    }


def body_hash(body):
    # This fixture contains ASCII strings and one small integer, so this encoding equals JCS.
    return hashlib.sha256(json.dumps(body, sort_keys=True, separators=(",", ":")).encode("ascii")).hexdigest()


def fixture_sql():
    # Seed test data before the old app starts. Keep every database trigger active.
    return f"""BEGIN;
INSERT INTO app_user (login, password_hash, role, must_change_password)
SELECT 'ci-author', password_hash, 'process_engineer', true FROM app_user WHERE login = 'ci-admin';
INSERT INTO machine (code, name, plc_platform) VALUES ('M1', 'CI machine', 'CodesysIec');
INSERT INTO unit (machine_id, path, name) SELECT id, 'U1', 'CI unit' FROM machine WHERE code = 'M1';
INSERT INTO recipe (machine_id, plc_platform, reference_number, name, created_by)
SELECT m.id, m.plc_platform, 'CI-UPGRADE', 'CI upgrade recipe', u.id
FROM machine m CROSS JOIN app_user u WHERE m.code = 'M1' AND u.login = 'ci-author';
INSERT INTO recipe_version (recipe_id, plc_platform, number, change_note, created_by)
SELECT id, plc_platform, 1, 'CI upgrade fixture', created_by FROM recipe WHERE reference_number = 'CI-UPGRADE';
INSERT INTO parameter (parameter_key, recipe_version_id, plc_platform, unit_id, tag,
    name_en, description_en, plc_type_id, value)
SELECT '{PARAMETER_KEY}', v.id, v.plc_platform, u.id, 'SPEED', 'Speed', 'CI speed setting', t.id, '"7"'::jsonb
FROM recipe_version v JOIN recipe r ON r.id = v.recipe_id
JOIN unit u ON u.machine_id = r.machine_id CROSS JOIN plc_type t
WHERE r.reference_number = 'CI-UPGRADE' AND u.path = 'U1' AND t.name = 'INT' AND t.machine_id IS NULL;
INSERT INTO parameter_limit (parameter_id, recipe_version_id, leaf_path, min_value, max_value)
SELECT id, recipe_version_id, '', 0, 10 FROM parameter WHERE parameter_key = '{PARAMETER_KEY}';
UPDATE recipe_version SET status = 'approved', approved_at = now(),
    approved_by = (SELECT id FROM app_user WHERE login = 'ci-admin'),
    content_sha256 = '{body_hash(fixture_body())}'
WHERE recipe_id = (SELECT id FROM recipe WHERE reference_number = 'CI-UPGRADE');
COMMIT;"""


class ProxyConnection(http.client.HTTPSConnection):
    def connect(self):
        # Use the backend bridge directly. TLS still checks the name localhost and the CI root.
        self.sock = socket.create_connection((os.environ["PROXY_ADDRESS"], 443), self.timeout)
        self.sock = self._context.wrap_socket(self.sock, server_hostname="localhost")


def load_probe():
    path = pathlib.Path(__file__).with_name("http_probe.py")
    spec = importlib.util.spec_from_file_location("http_probe", path)
    probe = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(probe)
    return probe


def check_export(document):
    body = {
        "machine": {key: document["machine"][key] for key in ("code", "plc_platform")},
        "recipe": {"reference_number": document["recipe"]["reference_number"]},
        "version": {"number": document["version"]["number"]},
        "types": document["types"],
        "parameters": [{key: row[key] for key in PARAMETER_FIELDS} for row in document["parameters"]],
    }
    if body != fixture_body() or document["content_sha256"] != body_hash(body) or document["loadable"] is not True:
        raise ValueError("the exported fixture or its content hash differs")


def smoke():
    probe = load_probe()
    state = pathlib.Path(os.environ["CI_STATE"])
    client = probe.Client(state, connection_factory=ProxyConnection)
    # Step 21. Send a wrong password, then read the event as the administrator.
    fields = {"login": "ci-admin", "password": "CI-wrong-password", "csrf_pre": client.token("/login", "csrf_pre")}
    status, _, body = client.request("/login", fields)
    if status != 401 or "E-IDN-003" not in body:
        raise ValueError("the wrong password did not return E-IDN-003")
    probe.login(client)
    probe.password(client)
    address = str(ipaddress.ip_address(os.environ["UPGRADE_CLIENT_IP"]))
    status, _, body = client.request("/security-events?kind=login_failed&login=ci-admin")
    if status != 200 or f'<td class="data">{address}</td>' not in body:
        raise ValueError("the security event does not show the runner address")
    # Step 22. Open the recipe and its version, then check the exported hash.
    for path in ("/recipes/CI-UPGRADE", "/recipes/CI-UPGRADE/versions/1"):
        status, _, body = client.request(path)
        if status != 200 or "CI-UPGRADE" not in body:
            raise ValueError("the recipe page did not show the fixture")
    status, _, body = client.request("/export/v1/recipes/CI-UPGRADE/versions/1")
    if status != 200:
        raise ValueError("the export did not return 200")
    check_export(json.loads(body))
    print("The login address, recipe pages, and export hash passed.")


if __name__ == "__main__":
    try:
        if sys.argv[1:] == ["seed"]:
            print(fixture_sql())
        elif sys.argv[1:] == ["check"]:
            smoke()
        else:
            raise ValueError("unknown upgrade smoke action")
    except Exception:
        # A response or an exception can contain a cookie. Print neither.
        print("CI: the upgrade smoke check failed.", file=sys.stderr)
        sys.exit(1)
