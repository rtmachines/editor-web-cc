"""Check the login flood gates of ADR-0128 (c) and (t) to (x).

The runner command reads the host CPU count, meminfo, and cgroup controllers.
The preflight command reads cpu.max, memory.max, and a failed-login count file.
The body command reads token and password files, then writes the oha body file.
The check command reads oha JSON and the app log window from load.sh.
It writes JSON metrics to stdout and optionally to --output, also on bound failures.
It appends metrics to GITHUB_STEP_SUMMARY when that variable exists.
"""

import argparse
import json
import math
import os
import pathlib
import re
import secrets
import sys
import urllib.parse


# These limits follow reliability.md sections 6 and 7.11 and ADR-0128 (c), (u).
REQUESTS = 200
BODY_LINES = 2000
MAX_SECONDS = 5
APP_MEMORY = 512 * 1024 * 1024
HOST_MEMORY = 8 * 1024 * 1024 * 1024

# Match the default output of oha v1.16.0 without --stats-success-breakdown.
# https://github.com/hatoo/oha/blob/v1.16.0/src/printer.rs
# Task 3.3 pins the binary version and checksum in load.sh.
PERCENTILES = "p10 p25 p50 p75 p90 p95 p99 p99.9 p99.99".split()
TRIPLE = dict.fromkeys(("average", "fastest", "slowest"), "time")
SCHEMA = {
    "summary": {
        "successRate": "number", "total": "time", "slowest": "time",
        "fastest": "time", "average": "time", "requestsPerSec": "number",
        "totalData": "integer", "sizePerRequest": "integer", "sizePerSec": "number",
    },
    "metrics": {
        "success_rate": "number", "requests_per_sec": "number",
        "latency_ms": dict.fromkeys(("min", "mean", "p50", "p95", "p99", "max"), "time"),
    },
    "responseTimeHistogram": ("histogram", "integer"),
    "latencyPercentiles": dict.fromkeys(PERCENTILES, "time"),
    "firstByteHistogram": ("histogram", "integer"),
    "firstBytePercentiles": dict.fromkeys(PERCENTILES, "time"),
    "rps": {"mean": "number", "stddev": "nullable", "max": "number", "min": "number",
            "percentiles": dict.fromkeys(PERCENTILES, "number")},
    "details": {"DNSDialup": TRIPLE, "DNSLookup": TRIPLE, "firstByte": TRIPLE},
    "statusCodeDistribution": ("status", "integer"),
    "errorDistribution": ("map", "integer"),
}


class FloodError(ValueError):
    """Hold a safe failure message and optional flood metrics."""

    def __init__(self, message, report=None):
        super().__init__(message)
        self.report = report


def finite(value):
    if type(value) not in (int, float) or not math.isfinite(value):
        raise FloodError("oha schema: each numeric value and time must be finite")
    return value


def validate_schema(value, schema):
    """Reject missing fields, unknown fields, and wrong types without echoing input."""
    if isinstance(schema, dict):
        if not isinstance(value, dict) or value.keys() != schema.keys():
            raise FloodError("oha schema: unexpected fields")
        for key, child in schema.items():
            validate_schema(value[key], child)
    elif isinstance(schema, tuple):
        if not isinstance(value, dict):
            raise FloodError("oha schema: expected an object")
        for key, child in value.items():
            if schema[0] == "histogram":
                try:
                    finite(float(key))
                except ValueError:
                    raise FloodError("oha schema: histogram time must be finite") from None
            if schema[0] == "status" and not re.fullmatch(r"[0-9]{3}", key):
                raise FloodError("oha schema: invalid status code")
            validate_schema(child, schema[1])
    elif schema == "integer":
        if type(value) is not int or value < 0:
            raise FloodError("oha schema: expected a nonnegative integer")
    elif schema != "nullable" or value is not None:
        finite(value)


def time_fields(document):
    """Return each time in seconds, including histogram boundaries."""
    for key in ("total", "slowest", "fastest", "average"):
        yield f"summary.{key}", document["summary"][key]
    for section in ("latencyPercentiles", "firstBytePercentiles"):
        for key, value in document[section].items():
            yield f"{section}.{key}", value
    for section in ("responseTimeHistogram", "firstByteHistogram"):
        for key in document[section]:
            yield f"{section}.{key}", float(key)
    for detail, triple in document["details"].items():
        for key, value in triple.items():
            yield f"details.{detail}.{key}", value
    for key, value in document["metrics"]["latency_ms"].items():
        yield f"metrics.latency_ms.{key}", value / 1000


def check_flood(document, permit_count):
    validate_schema(document, SCHEMA)
    summary = document["summary"]
    if summary["total"] <= 0:
        raise FloodError("flood total time must be positive")
    if any(value < 0 for _, value in time_fields(document)):
        raise FloodError("oha schema: times must be nonnegative")
    counts = {"401": 0, "429": 0, "503": 0, **document["statusCodeDistribution"]}
    report = {
        "status_counts": counts,
        "logins_per_second": (counts["401"] + counts["429"]) / summary["total"],
        "total_seconds": summary["total"], "slowest_seconds": summary["slowest"],
        "p99_seconds": document["latencyPercentiles"]["p99"],
        "permit_log_count": permit_count,
    }
    failures = []
    if sum(counts.values()) != REQUESTS:
        failures.append("flood must have exactly 200 answers")
    if summary["slowest"] > MAX_SECONDS:
        failures.append("slowest request exceeds 5 s")
    if document["errorDistribution"]:
        failures.append("oha reported errors")
    if counts["503"] == 0:
        failures.append("flood has no 503 answer")
    for code, count in counts.items():
        if code not in ("401", "429", "503") and count > 0:
            failures.append(f"unexpected HTTP status: {code}")
    if type(permit_count) is not int or permit_count != counts["503"]:
        failures.append("password hash permit count differs from 503 count")
    report["failures"] = failures
    if failures:
        raise FloodError(". ".join(failures), report)
    return report


def permit_log_count(log_window):
    """Count JSON E-WEB-009 entries whose structured stage equals "password hash permit" in the supplied app log window."""
    count = 0
    for line in log_window.splitlines():
        try:
            entry = json.loads(line)
        except json.JSONDecodeError:
            continue
        if not isinstance(entry, dict):
            continue
        fields = entry.get("fields")
        if not isinstance(fields, dict):
            continue
        if (fields.get("error_code") == "E-WEB-009"
                and fields.get("stage") == "password hash permit"):
            count += 1
    return count


def runner(meminfo, cgroup_root):
    cpus = os.cpu_count()
    if cpus is None or cpus < 4:
        raise FloodError("runner needs at least 4 vCPU")
    try:
        match = re.search(r"^MemTotal:\s+([0-9]+) kB\s*$", meminfo.read_text(), re.M)
        memory = int(match[1]) * 1024 if match else 0
    except OSError:
        memory = 0
    if memory < HOST_MEMORY:
        raise FloodError("runner needs at least 8 GiB of memory")
    if not (cgroup_root / "cgroup.controllers").is_file():
        raise FloodError("runner needs cgroup v2")
    print("CI runner check passed.")


def read_value(path):
    try:
        return path.read_text(encoding="ascii").strip()
    except (OSError, UnicodeError):
        return "unavailable"


def preflight(cpu_path, memory_path, failures_path):
    fields = read_value(cpu_path).split()
    if (len(fields) != 2 or not all(re.fullmatch(r"[0-9]+", part) for part in fields)
            or int(fields[0]) <= 0 or int(fields[0]) != int(fields[1])):
        raise FloodError("app CPU limit must be 1")
    memory = read_value(memory_path)
    if not re.fullmatch(r"[0-9]+", memory) or int(memory) != APP_MEMORY:
        raise FloodError("app memory.max must be 512 MiB")
    failures = read_value(failures_path)
    if not re.fullmatch(r"[0-9]+", failures) or int(failures) != 0:
        raise FloodError("failed login count must be 0")
    print("CI flood preflight check passed.")


def write_body(token_path, password_path, output_path):
    token = token_path.read_text(encoding="ascii").removesuffix("\n")
    password = password_path.read_text(encoding="ascii").removesuffix("\n")
    if not token or not password:
        raise FloodError("flood body input is empty")
    # Each random suffix stays in the TypedLogin grammar, also across runs.
    with open(output_path, "w", encoding="ascii", newline="\n",
              opener=lambda path, flags: os.open(path, flags, 0o600)) as output:
        for index in range(BODY_LINES):
            fields = {"login": f"load-{index}-{secrets.token_hex(16)}",
                      "password": password, "csrf_pre": token}
            output.write(urllib.parse.urlencode(fields) + "\n")
    print("CI flood body file contains 2000 lines.")


def publish(report, output_path):
    text = json.dumps(report, allow_nan=False, sort_keys=True) + "\n"
    if output_path:
        output_path.write_text(text, encoding="utf-8")
    print(text, end="")
    summary_path = os.environ.get("GITHUB_STEP_SUMMARY")
    if summary_path:
        statuses = " ".join(f"{code}={count}" for code, count in sorted(report["status_counts"].items()))
        with pathlib.Path(summary_path).open("a", encoding="utf-8") as summary:
            summary.write(f"Flood: {statuses}, logins/s={report['logins_per_second']:.6f}, "
                          f"p99={report['p99_seconds']:.6f} s, "
                          f"slowest={report['slowest_seconds']:.6f} s, "
                          f"total={report['total_seconds']:.6f} s\n")


class SafeParser(argparse.ArgumentParser):
    def error(self, message):
        # argparse can echo an accidental secret in an unknown argument.
        raise FloodError("invalid flood command arguments")


def arguments(argv):
    parser = SafeParser(description=__doc__)
    commands = parser.add_subparsers(dest="command", required=True)
    host = commands.add_parser("runner")
    host.add_argument("--meminfo", type=pathlib.Path, default=pathlib.Path("/proc/meminfo"))
    host.add_argument("--cgroup-root", type=pathlib.Path, default=pathlib.Path("/sys/fs/cgroup"))
    before = commands.add_parser("preflight")
    for name in ("cpu_max", "memory_max", "failed_logins"):
        before.add_argument(name, type=pathlib.Path)
    body = commands.add_parser("body")
    for name in ("csrf_pre", "password_file", "output"):
        body.add_argument(name, type=pathlib.Path)
    check = commands.add_parser("check")
    check.add_argument("oha_json", type=pathlib.Path)
    check.add_argument("app_log", type=pathlib.Path)
    check.add_argument("--output", type=pathlib.Path)
    return parser.parse_args(argv)


def main(argv=None):
    args = arguments(argv)
    if args.command == "runner":
        runner(args.meminfo, args.cgroup_root)
    elif args.command == "preflight":
        preflight(args.cpu_max, args.memory_max, args.failed_logins)
    elif args.command == "body":
        write_body(args.csrf_pre, args.password_file, args.output)
    else:
        document = json.loads(args.oha_json.read_text(encoding="utf-8"))
        count = permit_log_count(args.app_log.read_text(encoding="utf-8"))
        try:
            report = check_flood(document, count)
        except FloodError as error:
            if error.report is not None:
                publish(error.report, args.output)
            raise
        publish(report, args.output)


if __name__ == "__main__":
    os.umask(0o077)
    try:
        main()
    except FloodError as error:
        print(str(error), file=sys.stderr)
        sys.exit(1)
    except Exception:
        print("CI flood input could not be read.", file=sys.stderr)
        sys.exit(1)
