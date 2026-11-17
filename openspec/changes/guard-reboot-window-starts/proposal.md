# Proposal

## Why

ADR-0176 deletes `app` in the major upgrade, `backup` and `app` in the rollback, and `walarchive` in the recovery. A reboot then starts no deleted container.
The tests of `deploy/ci/` refuse only `start` and `restart` in these windows. An `up`, `create`, or `run` of a deleted service passes.
On `main` 39f1e15, fourteen mutants with such a command left 598 tests green (design.md "Evidence").
A start inside a window can write to the old cluster or change the `.partial` file. Writes then get lost (R31, R34, A35).

The reviews of PR #73 and PR #74 also left LOW findings: F3 to F7 and L-1 to L-6.

## What Changes

- The new file `deploy/ci/test_reboot_windows.py` guards the runbook windows of ADR-0176 (g) and (h): the major upgrade, the rollback, and the recovery for `walarchive`.
  Each window refuses `up`, `create`, `run`, `start`, and `restart` of a deleted service. Only its allowed end start names the service.
  The recovery window of `walarchive` also reads the restore steps that the recovery runs, up to the real start in restore sub-step 7.7.
- The same file guards the copy of the rollback window in `check_rollback_runbook` of `upgrade.sh`. It joins continuation lines before it reads a command.
- `test_pitr_recovery.py` guards the records of the `walarchive` window from `crash` to `wal-line`.
- `docker_calls.py`: explicit errors replace `assert` (F4). A short option cluster such as `-dt 10` takes its value (F5). A new helper cuts a record window.
- The mutant tests assert the exact hits (F3). A tab after a list marker ends a code span block (F6). Names and sizes of F7 change.
- The stop guard of the major upgrade parses the command (L-1). The test name of L-6 changes.
- `docs/adr/README.md` names the items that ADR-0176 amends (L-4). L-5 stays unfixed (owner decision 2026-11-17, design.md D12).
- L-2 and L-3 change a procedure step. The owner chose option A for each (design.md "Decisions of the owner (2026-11-17)").
- `CHANGELOG.md` gets one line.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

None. The change hardens tests and fixes words, so `.openspec.yaml` sets `skip_specs: true` (design.md D1).

## Out of scope

- A change of the commands of a procedure, except the owner decisions Q2 and Q3 (design.md).
- The window of "A gap in the WAL stream" (ADR-0176 (k)).
- A stub harness for `upgrade.sh` (design.md D6).
- A complete shell parser.

## Impact

- `deploy/ci/docker_calls.py`, `test_docker_calls.py`, `test_upgrade_runbook.py`, `test_runbook_text.py`, `test_restore_switch.py`, `test_pitr_recovery.py`.
- The new file `deploy/ci/test_reboot_windows.py`.
- `docs/ops/runbook.md`, `docs/adr/README.md`, `CHANGELOG.md`.
- No crate, migration, script of `deploy/ci/`, ADR file, `docs/architecture/` file, or spec changes.
