# ADR-0156: Data block number of a machine

Status: Accepted. Amends ADR-0154 (c) and (d). Amended by ADR-0157 (a sync trigger keeps the machine number equal to the copy in the expand period)
Date: 2026-11-12
Deciders: project owner

## Context

ADR-0154 put the data block number into the PLC connection record of a machine. The record is optional, so a machine on `S7_300`, `S7_1200`, or `S7_1500` can exist without a number. The owner wants a number on each S7 machine.

These facts shape the decision:

- The table `machine` has no `row_version` (ADR-0008). `RenameMachine` compares nothing.
- No command restores or changes an archived machine (A14). `SetPlcConnection` refuses an archived machine (ADR-0154).
- A breaking change uses expand and contract across releases (`reliability.md` §5).
- A migration never rewrites or scans a table under an exclusive lock (`data-model.md` §8).
- `sqlx` runs each migration file in its own transaction.

## Decision drivers

- No machine gets an invented number. A wrong number sends a recipe to the wrong data block.
- The domain, the form, and the database give the same answer for each value.
- The previous release keeps working after a rollback.
- The adapter sees no change in the transfer document.

## Considered options

### Option A: A column on `machine`, mandatory for a live S7 machine

- Pros: the rule lives on the row that owns the platform. `CreateMachine` sets the number. The database enforces the rule.
- Cons: three migration files, and a copy in `plc_connection` until the contract step.

### Option B: A mandatory connection record for each S7 machine

- Pros: no new column.
- Cons: `CreateMachine` then needs the host, the rack, and the slot. The owner often does not know those values when a new machine starts.

### Option C: A gate at approval or export

- Pros: no migration.
- Cons: a machine still exists without a number. The fault appears late, at the end of the work on a recipe.

## Decision

The project uses **Option A**. The items below are the rules.

- (a) The column `machine.db_number` holds the data block number. A live S7 machine has one. A `CodesysIec` or `TwinCat3` machine has none. An archived S7 machine from before this ADR may have none.
- (b) The limits of ADR-0154 (d) stay: 16 000 on `S7_300` and 59 999 on `S7_1200` and `S7_1500`. `PlcPlatform::max_data_block` holds both.
- (c) The role of ADR-0154 (d) stays. The number is the default data block for a parameter without `s7_address`. It never changes the address of a parameter.
- (d) `CreateMachine` takes the field `db_number`. A missing number on an S7 platform is `E-PLT-050`. A number on an IEC platform is `E-PLT-051`. The two codes open the block `E-PLT-050..069` of `PlantError`, as `E-RCP-050..069` of `RecipeError`.
- (e) The new command `SetMachineDataBlock` changes the number of a live S7 machine. It needs `MachineManage` and writes the audit action `MachineDataBlockSet`. It compares no `row_version`, as `RenameMachine`.
- (f) `SetPlcConnection` no longer takes `db_number`. This amends the field list of ADR-0154 (c).
- (g) The field `connection.db_number` of the transfer document stays. Its value comes from the machine.
- (h) The migration stops when a live S7 machine has no connection record. It names the code of each such machine and changes nothing. The operator saves the records with the previous release first.
- (i) The migration copies each stored number into its machine. The copy writes no audit entry. The audited entries `PlcConnectionSet` already hold the values.
- (j) Expand step: `plc_connection.db_number` stays as a copy for a rollback. Each command that writes the number also writes the copy. The copy writes no audit entry.
- (k) A later change drops `plc_connection.db_number` and relaxes `plc_connection_shape`. That change is the contract step.

## Consequences

### Positive

- Each new S7 machine has its number from the start.
- The database refuses a live S7 machine without a number.
- The adapter keeps its document.

### Negative

- An upgrade can stop until the operator saves the missing records.
- After a rollback, the previous release cannot create an S7 machine. The check refuses its `INSERT`.
- Two users can set the number at the same time. The last write wins, and the audit log holds both entries.

### Risks and mitigations

- Risk: the operator does not know which machines block the upgrade. Mitigation: the runbook gives a query that lists them before the upgrade.
- Risk: the copy in `plc_connection` drifts from the machine. Mitigation: one `recipe-db` test runs each command and compares both columns.

## Related

- ADR-0008 (`row_version`), ADR-0041 (error lists), ADR-0127 and ADR-0136 (migration check), ADR-0138 (changed form body), ADR-0153 (added column), ADR-0154 (PLC connection settings)
- `overview.md` §7: R14, R15, R20, R28, R29, R31, R38. §9: A9, A14, A24, A47, A48
- Change `openspec/changes/machine-data-block-number`
