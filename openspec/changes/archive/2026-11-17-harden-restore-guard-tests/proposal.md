# Proposal

## Why

The review of 2026-11-13 and the delta review of `restore-delete-app-backup` found gaps in the guard tests of `deploy/ci/`.
These tests keep a forbidden Docker command out of the restore, the recovery, and their runbook sections (ADR-0175, ADR-0176).
On `main` 2324454, six findings still let a forbidden command pass. Mutants with such a command left 532 tests green:

- RDAB-D2 and T-01: the runbook guard misses `start backup;`, `start app&&echo`, `$(...)`, `docker --context x compose`, and a fenced command split with `\`.
- RDAB-D3: the Docker stub logs `$*`. A quoted option value with spaces then hides `start` from the stub-log guard.
- T-02: no test keeps `app` and `backup` without a container from the delete until the procedure starts them again. `compose up -d` and `compose up -d proxy` also create them.
- T-03: an unmatched backtick in one list item pairs with a backtick of the next item and hides its command.
- T-05: the stub-log guard ignores `start`, `container start`, and `rm -f` outside Compose. It also ignores `stop db` and `rm -s -f db` of Compose.

A missed command can start `app` or `backup` on the old volume during a restore. Writes then get lost (R31, R34).
The recovery from the WAL archive uses A35.

## What Changes

- `deploy/ci/test_upgrade_runbook.py`: the runbook guard splits shell operators, skips redirections, and skips the global options of `docker`.
  It reads a runbook placeholder such as `<previous tag>` as one word. It joins continued fenced lines, and a code span ends at a list item.
- One shared parser reads the call of `docker` for the runbook guard and for the stub-log guards. It lives in the new file `deploy/ci/docker_calls.py`.
  The new file `deploy/ci/test_docker_calls.py` holds the unit tests of the shared helpers (design.md D3).
- The Docker stubs write each argument list as a lossless record. The stub-log guards read these records.
- The stub-log guard of the step `restore` also refuses `start`, `restart`, `kill`, and `rm` outside Compose. In Compose, it refuses `stop` and an `rm` of a service other than `app` and `backup`.
- New tests keep `app` and `backup` without a container in the restore and recovery windows, in the CI scripts and in the runbook.
  A window ends at a concrete record or at a concrete runbook text, for example the allowed start of `app`. So it includes the part of the step `check` before the start.
  The window guards separate the values of Compose options from the service names.
- `CHANGELOG.md` gets one line.

No production script, runbook text, or ADR changes.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

None. The change hardens tests only, so `.openspec.yaml` sets `skip_specs: true`.

## Out of scope

- T-04. The change `restore-reboot-hazards` closed it. design.md gives the evidence.
- A change of `restore-switch.sh`, `pitr-recovery.sh`, or `docs/ops/runbook.md`. Each new test passes on them now.
- A complete shell parser. The guards cover the forms of the findings and of design.md.
- Guards for `up`, `create`, and `run` in the windows of ADR-0176 (g) and (h). A later separate change adds them (design.md, owner decision Q2).

## Impact

- `deploy/ci/test_upgrade_runbook.py`, `deploy/ci/test_restore_switch.py`, `deploy/ci/test_pitr_recovery.py`.
- The new files `deploy/ci/docker_calls.py` and `deploy/ci/test_docker_calls.py` (design.md, owner decision Q1).
- `CHANGELOG.md`.
- No crate, migration, ADR, `docs/architecture/` file, or spec changes.
