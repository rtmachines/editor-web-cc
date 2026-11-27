"""Check the value leg and report its results."""

import importlib.util
import json
import pathlib
import re
import time

# Isolated Python omits this directory from the import path.
_spec = importlib.util.spec_from_file_location(
    "_value_load_memory", pathlib.Path(__file__).with_name("load_memory.py"))
_memory = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(_memory)


REFERENCES = tuple(f"LOAD-V{number}" for number in range(1, 6))
KEY = re.compile(r"[0-9a-f]{8}-(?:[0-9a-f]{4}-){3}[0-9a-f]{12}")


def missed_barrier(number, reference):
    return f"round {number}: request {reference} did not reach the barrier"


def missing_barrier_anon(number):
    return f"round {number} barrier_anon is missing or invalid"


ROUND_ONE_MIN_BYTES = 16 * 1024 * 1024 - 64 * 1024
ROUND_ONE_MAX_BYTES = 16 * 1024 * 1024
# The seed uses U1 and a one-digit ordinal. The URL holds the reference and key.
# A header holds the CSRF token. The body can add 18 digits to the pinned one-digit row_version.
# A nonnegative BIGINT has 1 to 19 decimal digits.
ROUND_TWO_MIN_BYTES = 3_033_541
ROUND_TWO_MAX_BYTES = ROUND_TWO_MIN_BYTES + (19 - 1)
MEMORY_PHASES = ("seed", "precheck", "baseline-1", "round-1", "baseline-2", "round-2")
CONTAINER_ID = re.compile(r"[0-9a-f]{64}")


def json_count(value):
    return value if type(value) is int and value >= 0 else None


def display(value):
    return "unavailable" if value is None else str(value)


class ValueCheck:
    def __init__(self, directory):
        self.directory = directory
        self.failures = []
        state = self.read("load-value-state.json", "value state")
        drafts = state.get("drafts")
        rows = {}
        if isinstance(drafts, list):
            for draft in drafts:
                if isinstance(draft, dict) and draft.get("reference") in REFERENCES:
                    rows[draft["reference"]] = draft
        if not isinstance(drafts, list) or len(drafts) != len(rows) or len(rows) != len(REFERENCES):
            self.failures.append("The value state must contain the five drafts.")
        self.keys = {}
        self.seed_versions = {}
        for ref in REFERENCES:
            draft = rows.get(ref, {})
            key = draft.get("key")
            if isinstance(key, str) and KEY.fullmatch(key):
                self.keys[ref] = key
            else:
                self.failures.append(f"value state {ref} has an invalid draft key")
            self.seed_versions[ref] = self.version(draft, "row_version", f"value state {ref}")
        if len(set(self.keys.values())) != len(self.keys):
            self.failures.append("The value state must contain distinct draft keys.")

    def read(self, filename, label):
        value = _memory.read_json(self.directory / filename)
        if isinstance(value, dict):
            return value
        self.failures.append(f"{label} is missing or invalid")
        return {}

    def version(self, values, key, label):
        value = json_count(values.get(key))
        if value is None:
            self.failures.append(f"{label} row_version is missing or invalid")
        return value

    def read_versions(self, phase, refs):
        values = self.read(f"load-value-versions-{phase}.json", f"{phase} versions")
        rows = values.get("row_versions")
        if not isinstance(rows, dict):
            self.failures.append(f"{phase} row_versions are missing or invalid")
            return {}
        return {ref: self.version(rows, ref, f"versions {phase} {ref}") for ref in refs}

    def compare_version(self, label, current, base, increased):
        if current is None or base is None:
            return
        if increased and current <= base:
            self.failures.append(f"{label} row_version did not increase")
        if not increased and current != base:
            self.failures.append(f"{label} row_version changed")

    def check_answer(self, answer, number, ref, label):
        if answer.get("error") is not None:
            self.failures.append(f"{label} failed during transport")
        status = json_count(answer.get("status"))
        expected = 422 if number == 1 else 303
        if status != expected:
            if status is not None and 100 <= status <= 599:
                self.failures.append(f"{label} returned status {status}. Expected {expected}.")
            else:
                self.failures.append(f"{label} has no valid status. Expected {expected}.")
        elif number == 1:
            field = answer.get("field")
            if (answer.get("code"), field) != ("E-WEB-008", "description_en"):
                field = field if field in ("form", "description_en") else "unavailable or invalid"
                self.failures.append(f"{label} has field {field}. Expected E-WEB-008 under "
                                     "description_en as the first field line.")
        elif ref in self.keys:
            location = f"/recipes/{ref}/versions/1/parameters/{self.keys[ref]}"
            if answer.get("location") != location:
                self.failures.append(f"{label} has an incorrect Location")

    def check_precheck(self, summary=None):
        ref = REFERENCES[-1]
        for number in (1, 2):
            label = f"pre-check round {number} request {ref}"
            answer = self.read(f"load-value-precheck-{number}.json", label)
            if summary is not None and answer:
                summary.append(answer_summary(label, answer))
            if (answer.get("reference") != ref or answer.get("key") != self.keys.get(ref)
                    or json_count(answer.get("round")) != number):
                self.failures.append(f"{label} has an incorrect draft or round")
            self.check_answer(answer, number, ref, label)
        versions = self.read_versions("precheck", (ref,))
        version = versions.get(ref)
        self.compare_version(f"pre-check {ref}", version, self.seed_versions[ref], True)
        return version

    def check_round_answers(self, number, summary):
        label = f"round {number}"
        batch = self.read(f"load-value-answers-{number}.json", f"{label} answers")
        answers = batch.get("answers")
        refs = REFERENCES[:4]
        if json_count(batch.get("round")) != number:
            self.failures.append(f"{label} answers have an incorrect round")
        if not isinstance(answers, list) or len(answers) != len(refs):
            self.failures.append(f"{label} must contain four answers")
            answers = answers if isinstance(answers, list) else []
        rows = {}
        lengths = []
        for answer in answers:
            if not isinstance(answer, dict) or answer.get("reference") not in refs:
                self.failures.append(f"{label} contains an answer for an invalid draft")
                continue
            ref = answer["reference"]
            request = f"{label} request {ref}"
            if summary is not None:
                summary.append(answer_summary(request, answer))
            if ref in rows:
                self.failures.append(f"{request} has more than one answer")
            rows[ref] = answer.get("key")
            if json_count(answer.get("round")) != number:
                self.failures.append(f"{request} has an incorrect round")
            self.check_answer(answer, number, ref, request)
            size = json_count(answer.get("body_bytes"))
            if not size:
                self.failures.append(f"{label} {ref} body length is missing or invalid")
            else:
                lengths.append(size)
                if number == 1 and not ROUND_ONE_MIN_BYTES <= size <= ROUND_ONE_MAX_BYTES:
                    self.failures.append(f"{label} {ref} body length is outside the 16 MiB window")
                elif number == 2 and not ROUND_TWO_MIN_BYTES <= size <= ROUND_TWO_MAX_BYTES:
                    self.failures.append(f"{label} {ref} body length is outside the pinned window")
        if rows != {ref: self.keys.get(ref) for ref in refs}:
            self.failures.append(f"{label} does not use the four draft keys from the state file")
        return batch, lengths

    def check_round(self, number, precheck, summary=None):
        batch, lengths = self.check_round_answers(number, summary)
        versions = self.read_versions(number, REFERENCES)
        for ref in REFERENCES:
            measured = ref != REFERENCES[-1]
            base = self.seed_versions[ref] if measured else precheck
            self.compare_version(f"round {number} {ref}", versions.get(ref), base,
                                 number == 2 and measured)
        return batch, lengths

    def read_memory(self, phase):
        raw = self.read(f"load-memory-{phase}.json", f"{phase} reading")
        values, errors = _memory.check_reading(raw, phase)
        if phase == "value-round-1":
            errors = [f"{phase} {field} is unavailable or invalid"
                      for field, value in values.items() if value is None]
        self.failures.extend(errors)
        values["current"] = _memory.memory_counter(raw.get("current"))
        if values["current"] is None:
            self.failures.append(f"{phase} current is unavailable or invalid")
        container = raw.get("container_id")
        if not isinstance(container, str) or not CONTAINER_ID.fullmatch(container):
            self.failures.append(f"{phase} container ID is unavailable or invalid")
            container = None
        values["container_id"] = container
        return values, bool(raw)

    def check_overlap(self, number, batch, lengths, baseline):
        reference = json_count(batch.get("reference_anon"))
        raw_barrier = batch.get("barrier_anon")
        barrier = json_count(raw_barrier)
        need = reference + sum(lengths) if reference is not None and len(lengths) == 4 else None
        margin = barrier - need if barrier is not None and need is not None else None
        barrier_max = margin_max = None
        if reference is None or (baseline is not None and reference != baseline):
            self.failures.append(f"round {number} reference_anon must equal the baseline anon")
        if barrier is None:
            error = batch.get("barrier_failure")
            valid_errors = (missed_barrier(number, ref) for ref in REFERENCES[:4])
            if raw_barrier is None and error in valid_errors:
                self.failures.append(error)
            else:
                self.failures.append(missing_barrier_anon(number))
        elif margin is not None and margin < 0:
            self.failures.append(f"round {number}: the bodies did not overlap")
        elif number == 2 and margin is not None:
            maximum = json_count(batch.get("barrier_anon_max"))
            if maximum is None:
                self.failures.append(f"round {number} barrier_anon_max is missing or invalid")
            elif maximum < barrier:
                self.failures.append(f"round {number} barrier_anon_max must be at least barrier_anon")
            else:
                barrier_max, margin_max = maximum, maximum - need
        return {"reference_anon": reference, "barrier_anon": barrier, "overlap_margin": margin,
                "barrier_anon_max": barrier_max, "overlap_margin_max": margin_max}

    def elapsed(self, now, path, label):
        try:
            start = _memory.nonnegative_integer(path.read_text(encoding="ascii").strip())
        except (OSError, UnicodeError):
            start = None
        if start is not None and start <= now:
            return now - start
        self.failures.append(f"{label} start time is missing or invalid")
        return None

    def round_result(self, number, baseline, reading, batch, lengths):
        container = reading.get("container_id")
        if container and baseline.get("container_id") and container != baseline["container_id"]:
            self.failures.append(f"round {number} container ID differs from its baseline")
        overlap = self.check_overlap(number, batch, lengths, baseline.get("anon"))
        available = json_count(batch.get("mem_available_min_kib"))
        if available is None:
            self.failures.append(f"round {number} MemAvailable is missing or invalid")
        peak, current = reading.get("peak"), baseline.get("current")
        statistic = (peak - current) / 4 if peak is not None and current is not None else None
        return {"round": number, "container_id": container, "peak": peak,
                "body_bytes": max(lengths, default=None), "run_statistic": statistic,
                "mem_available_min_kib": available, **overlap}

    def has_round_files(self, number):
        names = (f"load-value-answers-{number}.json", f"load-memory-value-round-{number}.json",
                 f"load-value-versions-{number}.json")
        return any((self.directory / name).is_file() for name in names)

    def check_round_readings(self, readings):
        for before, after in (("precheck", "baseline-1"), ("round-1", "baseline-2")):
            retiring = readings[before][0]["container_id"]
            baseline = readings[after][0]["container_id"]
            if retiring is not None and retiring == baseline:
                self.failures.append(
                    f"value-{before} and value-{after} use the same container ID"
                )
        peak = readings["round-2"][0]["peak"]
        if peak is not None and peak >= _memory.SOAK_MEMORY_CEILING:
            self.failures.append("round 2 reading reached the ceiling of 384 MiB")

    def check_all(self, job_start, summary_path):
        stopped = not self.has_round_files(2)
        summary = []
        readings = {
            phase: self.read_memory(f"value-{phase}") for phase in MEMORY_PHASES
            if not stopped or (self.directory / f"load-memory-value-{phase}.json").is_file()
        }
        if stopped:
            self.failures.append("The value leg stopped before round 2.")
            for phase, (reading, _) in readings.items():
                if phase != "round-1":
                    summary.extend(reading_summary(f"value-{phase}", reading))
            precheck = self.check_precheck(summary)
            numbers = (1,) if self.has_round_files(1) else ()
        else:
            precheck = self.check_precheck()
            self.check_round_readings(readings)
            numbers = (1, 2)

        rounds = []
        for number in numbers:
            batch, lengths = self.check_round(number, precheck, summary if stopped else None)
            baseline, _ = readings.get(f"baseline-{number}", ({}, False))
            reading, present = readings.get(f"round-{number}", ({}, False))
            if stopped and not reading:
                self.failures.append(f"value-round-{number} reading is missing or invalid")
            result = self.round_result(number, baseline, reading, batch, lengths)
            if present:
                rounds.append(result)
            if present or (stopped and (reading or batch)):
                summary.extend(round_summary(result, baseline, reading))
        now = int(time.time())
        job = self.elapsed(now, job_start, "job")
        leg = self.elapsed(now, self.directory / "load-value-leg-start", "value leg")
        summary.extend((f"job elapsed: {display(job)} s", f"value leg elapsed: {display(leg)} s"))
        result = {"passed": not self.failures, "failures": self.failures, "rounds": rounds,
                  "job_elapsed_s": job, "leg_elapsed_s": leg}
        with summary_path.open("a", encoding="utf-8") as stream:
            stream.write("\n".join(summary + self.failures) + "\n")
        (self.directory / "load-value.json").write_text(
            json.dumps(result, indent=2, allow_nan=False) + "\n", encoding="utf-8",
        )


def answer_summary(label, answer):
    status = json_count(answer.get("status"))
    if status is not None and not 100 <= status <= 599:
        status = None
    size = json_count(answer.get("body_bytes"))
    return f"{label} status={display(status)} body length={display(size)} bytes"


def reading_summary(label, reading):
    killed = reading["OOMKilled"]
    killed = str(killed).lower() if killed is not None else None
    return [
        f"{label} container ID: {display(reading['container_id'])}",
        f"{label} memory.peak={display(reading['peak'])} bytes",
        f"{label} memory.current={display(reading['current'])} bytes",
        f"{label} anon={display(reading['anon'])} bytes",
        (f"{label} oom={display(reading['oom'])} oom_kill={display(reading['oom_kill'])} "
         f"max={display(reading['max'])} OOMKilled={display(killed)} "
         f"RestartCount={display(reading['RestartCount'])}"),
    ]


def round_summary(result, baseline, reading):
    label = f"round {result['round']}"
    size = result["body_bytes"]
    triple = 3 * size if size is not None else None
    lines = reading_summary(label, reading) if reading else []
    lines.extend([
        f"{label} baseline memory.current: {display(baseline.get('current'))} bytes",
        f"{label} body length: {display(size)} bytes",
        (f"{label} run statistic: {display(result['run_statistic'])} bytes "
         f"(upper bound, 3 × body length: {display(triple)} bytes)"),
        f"{label} reference_anon: {display(result['reference_anon'])} bytes",
        f"{label} barrier_anon: {display(result['barrier_anon'])} bytes",
        f"{label} overlap_margin: {display(result['overlap_margin'])} bytes",
        f"{label} barrier_anon_max: {display(result['barrier_anon_max'])} bytes",
        f"{label} overlap_margin_max: {display(result['overlap_margin_max'])} bytes",
        f"{label} smallest MemAvailable: {display(result['mem_available_min_kib'])} KiB",
    ])
    return lines


def run_checker(args):
    checker = ValueCheck(args.ci_state)
    if args.command == "check-precheck":
        checker.check_precheck()
    elif args.command == "check-round":
        precheck = checker.read_versions("precheck", REFERENCES[-1:]).get(REFERENCES[-1])
        checker.check_round(args.round, precheck)
    else:
        checker.check_all(args.job_start, args.summary)
    return checker.failures
