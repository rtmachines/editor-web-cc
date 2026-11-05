"""Check two memory readings from the load job (ADR-0128 (y) to (ac)).

Pass the timings JSON path, the flood JSON path, and --output with an artifact path.
Each JSON object holds peak, limit, anon, file, oom, oom_kill, max, OOMKilled, and RestartCount.
The caller collects cgroup values with memory.sh and adds the remaining memory.events counters and Docker state.
Byte values and counters accept nonnegative integers or decimal strings.
OOMKilled accepts a JSON boolean or the Docker strings true and false.
GITHUB_STEP_SUMMARY selects the summary file to append.
"""

import argparse
import json
import os
import pathlib
import re
import sys


# reliability.md section 7.11: container.app_memory and soak.memory_ceiling.
APP_MEMORY = 512 * 1024 * 1024
SOAK_MEMORY_CEILING = 384 * 1024 * 1024
NUMERIC_FIELDS = ("peak", "limit", "anon", "file", "oom", "oom_kill", "max", "RestartCount")


def nonnegative_integer(value):
    if type(value) is int and value >= 0:
        return value
    if isinstance(value, str) and re.fullmatch(r"[0-9]+", value):
        try:
            return int(value)
        except ValueError:
            pass
    return None


def check_reading(reading, phase):
    errors = []
    values = {}
    if not isinstance(reading, dict):
        reading = {}
    for field in NUMERIC_FIELDS:
        value = nonnegative_integer(reading.get(field))
        values[field] = value
        if value is None:
            errors.append(f"{phase} {field}: unavailable or not a nonnegative integer")
    killed = reading.get("OOMKilled")
    if type(killed) is bool:
        values["OOMKilled"] = killed
    elif isinstance(killed, str) and killed in ("true", "false"):
        values["OOMKilled"] = killed == "true"
    else:
        values["OOMKilled"] = None
        errors.append(f"{phase} OOMKilled: unavailable or not a boolean")
    for field in ("oom", "oom_kill", "max", "RestartCount"):
        if values[field] is not None and values[field] > 0:
            errors.append(f"{phase} {field}: above 0")
    if values["peak"] is not None and values["peak"] >= APP_MEMORY:
        errors.append(f"{phase} memory.peak: reached the limit of 512 MiB")
    if values["OOMKilled"] is True:
        errors.append(f"{phase} OOMKilled: Docker reports an OOM kill")
    return values, errors


def check_memory(timings, flood):
    timings, timing_errors = check_reading(timings, "timings")
    flood, flood_errors = check_reading(flood, "flood")
    peaks = (timings["peak"], flood["peak"])
    peak = max(peaks) if all(value is not None for value in peaks) else None
    # Item (ac) applies only the OOM kill, restart, and peak bounds to the flood.
    condition_met = (
        flood["OOMKilled"] is False and flood["oom_kill"] == 0
        and flood["RestartCount"] == 0
        and flood["peak"] is not None and flood["peak"] < APP_MEMORY
    )
    errors = timing_errors + flood_errors
    return {
        "timings": timings, "flood": flood, "peak": peak,
        "container_app_memory": APP_MEMORY,
        "soak_memory_ceiling": SOAK_MEMORY_CEILING,
        "adr_0064_o_memory_condition": "met" if condition_met else "not met",
        "passed": not errors, "errors": errors,
    }


def format_summary(result):
    lines = []
    for phase in ("timings", "flood"):
        reading = result[phase]
        peak = reading["peak"] if reading["peak"] is not None else "unavailable"
        anon = reading["anon"] if reading["anon"] is not None else "unavailable"
        lines.append(f"{phase} memory.peak: {peak} bytes (container limit: 512 MiB)")
        lines.append(f"{phase} anon: {anon} bytes (soak ceiling: 384 MiB, information only)")
    peak = result["peak"] if result["peak"] is not None else "unavailable"
    lines.append(f"whole run memory.peak: {peak} bytes")
    lines.append("ADR-0064 (o) memory condition: " + result["adr_0064_o_memory_condition"])
    lines.extend(result["errors"])
    return "\n".join(lines) + "\n"


def read_json(path):
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except (OSError, UnicodeError, ValueError):
        # Never echo input values or parser exceptions.
        return None


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("timings", type=pathlib.Path)
    parser.add_argument("flood", type=pathlib.Path)
    parser.add_argument("--output", type=pathlib.Path, required=True)
    args = parser.parse_args()
    result = check_memory(read_json(args.timings), read_json(args.flood))
    summary = format_summary(result)
    args.output.write_text(json.dumps(result, indent=2, allow_nan=False) + "\n", encoding="utf-8")
    summary_path = os.environ.get("GITHUB_STEP_SUMMARY")
    if summary_path:
        with pathlib.Path(summary_path).open("a", encoding="utf-8") as stream:
            stream.write(summary)
    print(summary, end="")
    for error in result["errors"]:
        print(error, file=sys.stderr)
    return 0 if result["passed"] else 1


if __name__ == "__main__":
    try:
        sys.exit(main())
    except OSError:
        print("CI memory check could not write its report.", file=sys.stderr)
        sys.exit(1)
