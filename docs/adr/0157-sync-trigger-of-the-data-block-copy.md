# ADR-0157: Sync trigger for the data block copy in the expand period

Status: Accepted. Amends ADR-0156 (h), (i), (j), and (k)
Date: 2026-11-12
Deciders: project owner

## Context

ADR-0156 moves the data block number to `machine.db_number`. Migration `0014` copies `plc_connection.db_number` into the machine one time. The column `plc_connection.db_number` stays as a copy for a rollback (ADR-0156 (j)).

A review found that the previous release can change the copy after the copy step. These facts cause the fault:

- The previous release keeps running during the upgrade until step 18 of the runbook section "Upgrade", which replaces the container of `app`. The upgrade does not stop the running stack.
- After a rollback, the previous release runs again. It can still read and save the PLC connection record.
- `SetPlcConnection` of the previous release writes the number into `plc_connection` only. Then `machine.db_number` keeps the old number.
- `ExportPlcConnection` of the new release reads `machine.db_number` (ADR-0156 (g)). It sends the old data block to the adapter.
- The next `SetPlcConnection` of the new release writes the copy from the machine. The new number of the previous release is then lost, and no error shows the loss.

The review also found a gap in migration `0013`. A `CreateMachine` of the previous release can insert a live S7 machine without a record after the stop check and before the `ALTER TABLE`. The `NOT VALID` check does not read that row.

## Decision drivers

- No data block number changes without a trace. A wrong number sends a recipe to the wrong data block.
- The previous release stays unchanged. Its image shipped.
- A migration that has not shipped can still change. A shipped migration never changes (`data-model.md` §8).
- The lock order `machine` → `plc_connection` of ADR-0015 applies also to a migration that runs next to the previous release.

## Considered options

### Option A: A sync trigger on `plc_connection` until the contract step

- Pros: each write of the previous release reaches the machine in its own transaction. The new release needs no change.
- Cons: one more trigger and function until the contract step. The machine write has no audit entry of its own.

### Option B: `ExportPlcConnection` reads `plc_connection.db_number` until the contract step

- Pros: no trigger.
- Cons: the next `SetPlcConnection` of the new release still overwrites the number of the previous release. Two sources of the number exist.

### Option C: Revoke the `UPDATE` grant on `plc_connection.db_number` for the expand period

- Pros: the previous release cannot change the copy.
- Cons: the previous release then cannot save any PLC connection. ADR-0156 promises that it can.

## Decision

The project uses **Option A**. The items below are the rules.

- (a) Migration `0014` creates the function `plc_connection_sync_db_number()` and the trigger `plc_connection_sync_db_number` after its copy. The trigger is `AFTER INSERT OR UPDATE OF db_number ON plc_connection FOR EACH ROW`.
- (b) When `NEW.db_number IS NOT NULL`, the function runs `UPDATE machine SET db_number = NEW.db_number WHERE id = NEW.machine_id AND db_number IS DISTINCT FROM NEW.db_number`. Else it does nothing.
- (c) The function is `SECURITY INVOKER` with `SET search_path = pg_catalog, public, pg_temp`. `recipe_app` has `UPDATE (db_number)` on `machine` from `0013`. The function raises no error of its own and has no trigger code.
- (d) A number outside the platform range fails a range `CHECK` with SQLSTATE `23514`. The previous release then gets `E-DB-003` and writes nothing.
- (e) `SetPlcConnection` of the previous release locks `machine FOR NO KEY UPDATE` before it writes the record (`data-model.md` §6, ADR-0015). The machine write of the trigger therefore needs no new lock. The trigger `machine_set_updated_at` sets `updated_at`. The entry `PlcConnectionSet` of the previous release holds the number, so the machine write adds no audit entry.
- (f) The new release writes the copy from `machine.db_number`. The trigger therefore changes no row on a write of the new release.
- (g) Migration `0014` first locks every `machine` row with `FOR NO KEY UPDATE` in the order of `id`. Then it runs the copy, and then it creates the trigger. A `SetPlcConnection` of the previous release that runs at the same time then ends before the copy reads its record. A later one waits for the end of `0014` and fires the trigger.
- (h) The copy of `0014` changes only the rows where `plc_connection.db_number IS NOT NULL`. An IEC record therefore causes no write and no new `updated_at` on its machine.
- (i) Migration `0013` runs `LOCK TABLE machine IN ACCESS EXCLUSIVE MODE` as its first statement. The lock blocks every other statement on `machine` until the file ends. Its stop check therefore reads every machine that the `ALTER TABLE` sees. A weaker mode such as `SHARE ROW EXCLUSIVE` deadlocks with a previous-release command. That command holds a machine row lock and then runs its `UPDATE`, while the `ALTER TABLE` waits for that command. PostgreSQL then aborts one side with `40P01`. `ACCESS EXCLUSIVE` makes `0013` wait until such a command ends, so the deadlock does not occur.
- (i1) The list `EXEMPT` of `deploy/ci/migration_rewrite.py` gets the entry `("0013_machine_db_number.sql", "lock table")` with the reason `ADR-0157`. The entry follows the rules of ADR-0136. It permits the `LOCK TABLE` statement of item (i).
- (j) `SetMachineDataBlock` writes the copy only when the number of the record differs from the new number. With the current number, the record and its `row_version` stay unchanged. The command still updates `machine` and writes `MachineDataBlockSet`, as `RenameMachine` with the current name.
- (k) The contract change that drops `plc_connection.db_number` also drops the trigger and its function, before the column.

## Consequences

### Positive

- A number that the previous release saves reaches the machine in the same transaction.
- `ExportPlcConnection` sends the number that the operator saved last, in both releases.
- No `CreateMachine` of the previous release can pass the stop check of `0013` without a number.

### Negative

- During `0014`, each command of the previous release that locks a machine waits until the file ends.
- During `0013`, `CreateMachine` of the previous release waits until the file ends.
- Between the commit of `0013` and the row lock of `0014`, a previous-release command that updates a live S7 machine row fails with `E-DB-003`. For example, `RenameMachine` fails. The `NOT VALID` check `machine_db_number_shape` checks each updated row, and `db_number` is still NULL. No data is lost.
- The restore test of `recipe-pgtools` lists one more function and one more trigger until the contract step.
- The machine write of the trigger has no audit entry of its own.

### Risks and mitigations

- Risk: the contract change forgets the trigger. Mitigation: the trigger names the column in `UPDATE OF db_number`, so `DROP COLUMN` without `CASCADE` fails until the trigger is gone.
- Risk: the entry `lock table` of item (i1) also permits any other `LOCK TABLE` statement in `0013`. Mitigation: the review of the entry checks the whole file (ADR-0136, Risks).

## Related

- ADR-0015 (lock order), ADR-0136 (`EXEMPT`), ADR-0143 (k1) (`lock table` of `0011`), ADR-0154 (PLC connection), ADR-0156 (data block number)
- `overview.md` §7: R28, R31. §9: A48
- Change `openspec/changes/machine-data-block-number`
