"""Seed the load draft through HTTP and check its database counts (ADR-0128 (m) to (o)).

Run with CI_STATE after the administrator changes its password.
The fresh stack receives one machine, one UDT, and one recipe.
The file load-seed.json holds the draft path, counts, and seed_seconds for the timings.
"""

import importlib.util
import json
import os
import pathlib
import re
import subprocess
import sys
import time


# Isolated Python omits this directory from the import path.
_spec = importlib.util.spec_from_file_location(
    "_seed_load_client", pathlib.Path(__file__).with_name("load_client.py"))
_client = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(_client)
Client = _client.Client
HttpCheckError = _client.HttpCheckError

# Match budget.parameters_per_version in reliability.md section 7.11.
PARAMETERS = 4000
# ADR-0128 (m) and (o) define the shape and the progress interval.
MEMBERS = 8
ELEMENTS = 8
PROGRESS_INTERVAL = 500
MACHINE = "M9000001"
REFERENCE = "LOAD-1"
UDT = "ST_Load"
DRAFT_PATH = f"/recipes/{REFERENCE}/versions/1"
EXPECTED_COUNTS = {"parameters": PARAMETERS, "leaves": PARAMETERS * MEMBERS * ELEMENTS,
                   "limit_rows": PARAMETERS * MEMBERS}

# Count each table separately so a join cannot multiply the counts.
COUNT_SQL = """
WITH draft AS (
    SELECT v.id FROM public.recipe_version v
    JOIN public.recipe r ON r.id = v.recipe_id
    WHERE r.reference_number = :'reference' AND v.number = 1
      AND v.status = 'draft' AND v.deleted_at IS NULL
), parameters AS (
    SELECT p.value FROM public.parameter p JOIN draft d ON d.id = p.recipe_version_id
)
SELECT (SELECT count(*) FROM parameters),
       (SELECT count(*) FROM parameters p,
        LATERAL pg_catalog.jsonb_path_query(p.value,
          'strict $.** ? (@.type() != "object" && @.type() != "array")') leaf),
       (SELECT count(*) FROM public.parameter_limit l WHERE l.recipe_version_id = d.id)
FROM draft d;
"""


class SeedCheckError(ValueError):
    """Report a count failure without database output or credentials."""


def define_udt_body():
    fields = {"name": UDT}
    for index in range(MEMBERS):
        fields[f"members[{index}].name"] = f"m{index + 1}"
        fields[f"members[{index}].type"] = "ARRAY[1..8] OF INT"
    return fields


def parameter_body(number):
    fields = {"unit_path": "U1", "tag": f"P{number}", "name_en": f"Parameter {number}",
              "description_en": "Load seed parameter", "type": f"{UDT}@1",
              "value_type": f"{UDT}@1", "ordinal": str(number)}
    for member in range(1, MEMBERS + 1):
        for element in range(1, ELEMENTS + 1):
            fields[f"value.m{member}[{element}]"] = "1"
        prefix = f"limits[{member - 1}]"
        fields.update({f"{prefix}.leaf_path": f".m{member}[*]",
                       f"{prefix}.min": "0", f"{prefix}.max": "100"})
    return fields


def create(client, page, action, fields, destination):
    client.form(page, action)
    status, headers, body = client.post(action, fields, hx=False)
    if status != 303 or headers.get("location") != destination:
        raise HttpCheckError("the seed command did not redirect to its page", status, body)


def seed(client, *, clock=time.monotonic):
    """Create the draft and report each completed batch of AddParameter requests."""
    started = clock()
    create(client, "/machines/new", "/machines",
           {"code": MACHINE, "name": "Load machine", "plc_platform": "CodesysIec",
            "main_unit_name": "Line"}, f"/machines/{MACHINE}")
    types = f"/machines/{MACHINE}/types"
    create(client, types + "/new", types, define_udt_body(), types + f"/{UDT}")
    create(client, "/recipes/new", "/recipes",
           {"machine_code": MACHINE, "reference_number": REFERENCE,
            "name": "Load recipe", "change_note": "Seed the load draft"}, DRAFT_PATH)
    action = DRAFT_PATH + "/parameters"
    client.form(action + "/new?unit=U1", action)
    for number in range(1, PARAMETERS + 1):
        client.post(action, parameter_body(number), draft=True)
        if number % PROGRESS_INTERVAL == 0:
            print(f"Seed: {number} AddParameter requests, {clock() - started:.3f} s elapsed.", flush=True)
    return {"reference": REFERENCE, "draft_path": DRAFT_PATH,
            "row_version": client.row_version, "seed_seconds": clock() - started}


def check_counts(counts):
    for name, expected in EXPECTED_COUNTS.items():
        actual = counts.get(name)
        if type(actual) is not int or actual != expected:
            observed = str(actual) if type(actual) is int else "invalid"
            raise SeedCheckError(f"draft {name}: expected {expected}, got {observed}")


def read_counts(reference):
    """Read the live draft through the local postgres socket of the CI database."""
    if not re.fullmatch(r"[A-Z0-9][A-Z0-9_-]{0,49}", reference):
        raise SeedCheckError("invalid recipe reference for draft counts")
    common = pathlib.Path(__file__).with_name("common.sh").resolve()
    script = 'set -euo pipefail\nsource "$1"\nsql -q -v reference="$2"'
    result = subprocess.run(["bash", "-c", script, "load-seed-counts", str(common), reference],
                            input=COUNT_SQL, text=True, capture_output=True, check=False)
    if result.returncode != 0:
        raise SeedCheckError("cannot read draft counts")
    if not re.fullmatch(r"[0-9]+\|[0-9]+\|[0-9]+\n?", result.stdout):
        raise SeedCheckError("invalid draft counts")
    return dict(zip(EXPECTED_COUNTS, map(int, result.stdout.strip().split("|"))))


def main():
    os.umask(0o077)
    state = pathlib.Path(os.environ["CI_STATE"])
    client = Client(state)
    client.login("ci-admin", "new_password")
    result = seed(client)
    counts = read_counts(result["reference"])
    check_counts(counts)
    result["counts"] = counts
    (state / "load-seed.json").write_text(json.dumps(result) + "\n", encoding="utf-8")
    print(f"Seed completed in {result['seed_seconds']:.3f} s. Draft counts passed.")


def cli():
    try:
        main()
        return 0
    except (HttpCheckError, SeedCheckError) as error:
        print(str(error), file=sys.stderr)
        return 1
    except Exception:
        # Database errors and transport errors can contain secrets.
        print("CI load seed failed.", file=sys.stderr)
        return 1


if __name__ == "__main__":
    sys.exit(cli())
