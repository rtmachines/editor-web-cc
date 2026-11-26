"""Check memory readings from the load job (ADR-0128 and ADR-0222).

Pass the timings JSON path, the flood JSON path, and --output with an artifact path.
Use --value PHASE=PATH for each of the six value phases.
The value readings also require current and container_id.
Byte values and counters accept integers or decimal strings from 0 to 2^63 - 1.
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
MAX_COUNTER = 2**63 - 1
NUMERIC_FIELDS = ("peak", "limit", "anon", "file", "oom", "oom_kill", "max", "RestartCount")
BOUND_COUNTERS = ("oom", "oom_kill", "max", "RestartCount")
VALUE_PHASES = (
    "value-seed", "value-precheck", "value-baseline-1", "value-round-1",
    "value-baseline-2", "value-round-2",
)


def nonnegative_integer(value):
    if type(value) is int and value >= 0:
        return value
    if isinstance(value, str) and re.fullmatch(r"[0-9]+", value):
        try:
            return int(value)
        except ValueError:
            pass
    return None


def memory_counter(value):
    count = nonnegative_integer(value)
    return count if count is not None and count <= MAX_COUNTER else None


def check_reading(reading, phase, *, enforce_bounds=True):
    errors = []
    values = {}
    if not isinstance(reading, dict):
        reading = {}
    for field in NUMERIC_FIELDS:
        value = memory_counter(reading.get(field))
        values[field] = value
        if value is None:
            errors.append(f"{phase} {field}: unavailable or invalid (integer from 0 to 2^63 - 1)")
    killed = reading.get("OOMKilled")
    if type(killed) is bool:
        values["OOMKilled"] = killed
    elif isinstance(killed, str) and killed in ("true", "false"):
        values["OOMKilled"] = killed == "true"
    else:
        values["OOMKilled"] = None
        errors.append(f"{phase} OOMKilled: unavailable or not a boolean")
    if not enforce_bounds:
        return values, errors
    for field in BOUND_COUNTERS:
        if values[field] is not None and values[field] > 0:
            errors.append(f"{phase} {field}: above 0")
    if values["peak"] is not None and values["peak"] >= APP_MEMORY:
        errors.append(f"{phase} memory.peak: reached the limit of 512 MiB")
    if values["OOMKilled"] is True:
        errors.append(f"{phase} OOMKilled: Docker reports an OOM kill")
    return values, errors


def check_value_reading(reading, phase):
    if not isinstance(reading, dict):
        return None, [f"{phase} reading is missing or invalid"]
    values, errors = check_reading(reading, phase, enforce_bounds=phase != "value-round-1")
    current = memory_counter(reading.get("current"))
    if current is None:
        errors.append(f"{phase} current: unavailable or invalid (integer from 0 to 2^63 - 1)")
    container_id = reading.get("container_id")
    if not isinstance(container_id, str) or not re.fullmatch(r"[0-9a-f]{64}", container_id):
        errors.append(f"{phase} container_id: unavailable or invalid")
        container_id = None
    values.update(current=current, container_id=container_id)
    return values, errors


def check_memory(timings, flood, value=None):
    timings, timing_errors = check_reading(timings, "timings")
    flood, flood_errors = check_reading(flood, "flood")
    errors = timing_errors + flood_errors
    readings = {}
    peaks = [timings["peak"], flood["peak"]]
    if value is not None:
        for phase in VALUE_PHASES:
            reading, reading_errors = check_value_reading(value.get(phase), phase)
            readings[phase] = reading
            errors.extend(reading_errors)
            peaks.append(reading["peak"] if reading is not None else None)
    peak = max(peaks) if all(value is not None for value in peaks) else None
    # Item (ac) applies only the OOM kill, restart, and peak bounds to the flood.
    condition_met = (
        flood["OOMKilled"] is False and flood["oom_kill"] == 0
        and flood["RestartCount"] == 0
        and flood["peak"] is not None and flood["peak"] < APP_MEMORY
    )
    return {
        "timings": timings, "flood": flood, "value": readings, "peak": peak,
        "container_app_memory": APP_MEMORY,
        "soak_memory_ceiling": SOAK_MEMORY_CEILING,
        "adr_0064_o_memory_condition": "met" if condition_met else "not met",
        "passed": not errors, "errors": errors,
    }


def format_summary(result):
    lines = []
    readings = {"timings": result["timings"], "flood": result["flood"], **result["value"]}
    for phase, reading in readings.items():
        if reading is None:
            continue
        peak = reading["peak"] if reading["peak"] is not None else "unavailable"
        anon = reading["anon"] if reading["anon"] is not None else "unavailable"
        round_one = phase == "value-round-1"
        note = "information only" if round_one else "container limit: 512 MiB"
        lines.append(f"{phase} memory.peak: {peak} bytes ({note})")
        lines.append(f"{phase} anon: {anon} bytes (soak ceiling: 384 MiB, information only)")
        if round_one:
            for field in (*BOUND_COUNTERS, "OOMKilled"):
                value = reading[field]
                text = json.dumps(value) if value is not None else "unavailable"
                lines.append(f"{phase} {field}: {text} (information only)")
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


def value_argument(argument):
    phase, _, path = argument.partition("=")
    if phase not in VALUE_PHASES or not path:
        raise argparse.ArgumentTypeError("use PHASE=PATH with a known value phase")
    return phase, pathlib.Path(path)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("timings", type=pathlib.Path)
    parser.add_argument("flood", type=pathlib.Path)
    parser.add_argument("--value", action="append", type=value_argument, metavar="PHASE=PATH")
    parser.add_argument("--output", type=pathlib.Path, required=True)
    args = parser.parse_args()
    value = None
    if args.value is not None:
        paths = dict(args.value)
        if len(paths) != len(args.value):
            parser.error("each value phase must occur once")
        value = {phase: read_json(path) for phase, path in paths.items()}
    result = check_memory(read_json(args.timings), read_json(args.flood), value)
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
