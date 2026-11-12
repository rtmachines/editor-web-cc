# Design

## Context

See `proposal.md`, section Why. These facts of the normative documents shape the design:

- ADR-0154 (c) and (d) put `db_number` into the optional record `plc_connection` (`data-model.md` §3.18). The record holds the number only for an S7 platform.
- `DataBlockNumber` and `PlcPlatform::max_data_block` already exist (`domain-model.md` §2). `E-PLT-024` already names the platform limit (`reliability.md` §1.2).
- `machine` has no `row_version` (ADR-0008). `RenameMachine` locks `machine FOR NO KEY UPDATE` and compares nothing (`data-model.md` §6).
- No command restores an archived machine, and `SetPlcConnection` refuses an archived machine (A14, ADR-0154).
- A breaking change uses expand and contract across releases (`reliability.md` §5). A migration never rewrites or scans a table under an exclusive lock (`data-model.md` §8, ADR-0127, ADR-0136).
- `sqlx` runs each migration file in its own transaction. A failed file leaves the earlier files applied.
- `E-PLT-001..029` is full, and `E-PLT-030..049` belongs to the trigger codes (`reliability.md` §1.2). The change opens the block `E-PLT-050..069`, as `E-RCP-050..069`.

## Goals / Non-Goals

**Goals:**

- Each live S7 machine has one data block number from its creation on.
- The domain, the form, and a `CHECK` give the same answer for each value.
- The previous release keeps working after a rollback to it.

**Non-Goals:**

- The contract step. A later change drops `plc_connection.db_number`.
- A new role of the number. It stays the default of the adapter (ADR-0154 (d)).

## Decisions

### D1. Column `machine.db_number` and two checks

- `db_number INTEGER`, nullable, no default. A plain `INTEGER` without a default rewrites no table (ADR-0153).
- `CHECK machine_db_number_shape`: an IEC row has no number. A live S7 row has a number. An archived S7 row may have none.
- `CHECK machine_db_number_range`: `1..16000` on `S7_300`, `1..59999` on `S7_1200` and `S7_1500`. The limits equal `PlcPlatform::max_data_block`.
- Grant: `UPDATE (db_number)` on `machine` for `recipe_app`. The grant goes into `grants.expected` and `deploy/pgtools/restore-roles.expected`.
- A check violation maps to `E-DB-003`, because the domain refuses the value first.
- Alternative: a `NOT NULL` column with a default. Rejected: no default is a correct number, and a wrong number sends recipes to the wrong data block.

### D2. Three migration files

1. `0013_machine_db_number.sql`. The first statement is `LOCK TABLE machine IN ACCESS EXCLUSIVE MODE`, as `0011` locks its tables (D10). Then a temporary check function finds each live S7 machine without a `plc_connection` row. The file creates the function with `CREATE FUNCTION`, runs a `SELECT`, and drops it with `DROP FUNCTION`. The function raises an exception that names their codes, so the file changes nothing. Then the file adds the column and both checks `NOT VALID`.
2. `0014_machine_db_number_copy.sql`. The file locks the `machine` rows in the order of `id`. Then `UPDATE machine ... FROM plc_connection` copies each number that is not `NULL`. The statement takes `ROW EXCLUSIVE`, not an exclusive lock. The trigger `machine_set_updated_at` sets `updated_at`. Then the file creates the sync trigger of D10.
3. `0015_machine_db_number_validate.sql`. `VALIDATE CONSTRAINT` on both checks, as `0010` does.

- After file 1, a `NOT VALID` check already refuses a new S7 machine without a number. So no gap can open between file 1 and file 3.
- The copy writes no audit entry. The ADR records this. The next `MachineDataBlockSet` entry shows the copied value as its "before" image.

### D3. Domain

- `DataBlockNumber` moves from `plant::connection` to `plant`. Its grammar `1..=65535` stays.
- `Machine` gets a private field `db_number: Option<DataBlockNumber>` and the accessor `db_number()`.
- One free function `check_data_block(code, platform, Option<DataBlockNumber>)` applies the rule of D1 and the platform limit. `Machine::create`, `Machine::set_data_block`, and `TryFrom<StoredMachine>` call it.
- `TryFrom<StoredMachine>` permits no number only for an archived S7 machine.
- `PlcConnection::S7` loses `db_number`. `ConnectionFields` loses the field.

### D4. Error codes

| Code | Variant | Status | Message |
|---|---|---|---|
| `E-PLT-050` | `PlantError::DataBlockMissing { code, platform }` | 422 | Machine {code} on platform {platform} needs a data block number. Enter a number. |
| `E-PLT-051` | `PlantError::DataBlockNotOnPlatform { code, platform }` | 422 | Platform {platform} of machine {code} has no data block number. Delete the number. |

- `E-PLT-024` keeps its code and message. Its variant becomes `PlantError::DataBlockOutOfRange { code, number, max }`, because the machine now owns the number.
- `E-PLT-022` and `E-PLT-023` no longer name the field `db_number`. A request of `SetPlcConnection` with `db_number` gets `E-WEB-008` from `deny_unknown_fields` (ADR-0138).

### D5. `CreateMachine`

- The input gets `db_number: Option<DataBlockNumber>`. An empty text is no value.
- The use case calls `check_data_block` before the transaction (`reliability.md` §7.4). The `INSERT` writes the column, so `MachineCreated` holds the number.
- The domain check runs only when every field parses. A parse error of another field stays one `E-WEB-008` line of the list (ADR-0041).

### D6. Command `SetMachineDataBlock`

- Route `POST /machines/{code}/data-block` with the field `db_number`. Permission `MachineManage`. `Operation::SetMachineDataBlock`.
- Before the transaction: read `machine.plc_platform` without a lock and call `check_data_block`. The platform never changes.
- Transaction: `machine FOR NO KEY UPDATE`, `archived_at IS NOT NULL` → `E-PLT-010`. `UPDATE machine SET db_number`. Then `UPDATE plc_connection SET db_number` of the machine, when the row exists.
- Audit: one entry `MachineDataBlockSet` with the machine image before and after. The copy in `plc_connection` writes no entry (D7).
- No `row_version`. The command works like `RenameMachine`. Each change has its audit entry.
- A new number bumps `plc_connection.row_version`. An open connection form then gets `409 E-WEB-013` on save. The new form shows the stored record.
- The current number changes no record and no `row_version`. The command still writes `MachineDataBlockSet`, as `RenameMachine` with the current name (D10).
- Read-only mode gives `503 E-WEB-006` (A24).

### D7. Copy in `plc_connection` (expand step)

- `recipe-db` writes the copy in SQL. The domain never sees it.
- `SetPlcConnection` writes `db_number` from `SELECT db_number FROM machine` under the machine lock that it already holds.
- `SetMachineDataBlock` writes the copy of D6.
- `ExportPlcConnection` reads `machine.db_number`. The field `connection.db_number` of `api.md` §6.5 keeps its name and type.
- An S7 record of a machine without a number is a corrupt stored row. `ExportPlcConnection` then refuses with `E-DB-008` and writes no entry (`data-model.md` §6).
- `plc_connection_shape` stays unchanged. The previous release can thus read and write the record after a rollback.

### D8. Pages

- `machine.html`: the facts list shows the number of an S7 machine, or the text of `machine-db-number-none`. The connection facts lose the number.
- A data block form sits next to the rename form inside the same `details` element. Its block renders alone as the out-of-band form of an error answer (ADR-0041 (e)).
- A success answers `200` with the fragment `machine-data-block-set.html` and a toast. A request without `HX-Request` gets `303` to `/machines/{code}`.
- `machine-new.html` gets the field `db_number` after the platform list. It is always visible, with a hint that names the three S7 platforms. No script hides it (`api.md` §9).
- `machine-connection.html` loses the field `db_number`.
- New `fluent` keys in `pl` and `en`. The UI text follows `design-system.md`.

### D9. Documents of the change

- ADR-0156 amends ADR-0154 (c) and (d). A48 joins `overview.md` §9 and `domain-model.md` §8.
- `reliability.md` §1.2: two rows and the renamed variant of `E-PLT-024`.
- `data-model.md`: §1.2, §3.1, §3.18, §5.2, §6, §8, §10.
- `domain-model.md`: §2 and §5. `api.md`: §2.2, §3.3, §6.5.
- `docs/ops/`: the upgrade check and the rollback note. `docs/ops/transfer-client.md` names the machine as the source of `db_number`.

### D10. Sync trigger and migration locks

- ADR-0157 gives the rules. `data-model.md` §4.13 gives the trigger, and §8 gives the order of `0013` and `0014`.
- The trigger keeps `machine.db_number` equal to each number that the previous release saves. It acts in the upgrade window and after a rollback.
- `0013` locks `machine` first, so no `CreateMachine` of the previous release slips past the stop check.
- The contract change drops the trigger and its function with the column.

## Risks / Trade-offs

- [The upgrade stops on a live S7 machine without a record] → The runbook gives a query that lists such machines before the upgrade. The operator saves their records with the previous release.
- [After a rollback, the previous release cannot create an S7 machine] → The `NOT VALID` check refuses its `INSERT` with `E-DB-003`. No wrong row appears. The runbook names this limit.
- [Two users set the number at the same time] → The last write wins, as for `RenameMachine`. The audit log holds both entries.
- [The copy write adds a second `UPDATE` to each command] → The table `plc_connection` has at most one row per machine. The contract change deletes the copy.
- [The copy of `0014` has no audit entry] → The ADR records the copy. The values come from audited `PlcConnectionSet` entries.

## Migration Plan

1. Before the upgrade, run the query of the runbook. Save the record of each listed machine with the previous release.
2. Release with `0013`, `0014`, `0015`, `grants.expected`, and `restore-roles.expected`.
3. On a stop in `0013`, the schema stays at `0012`. Repeat step 1 and the upgrade.
4. Rollback: the previous release runs on the new schema, without the creation of an S7 machine.
5. A later change drops `plc_connection.db_number` and relaxes `plc_connection_shape` (contract step).

## Open Questions

None. The owner chose the mandatory number on the machine, the unchanged role, the stop of the migration, and the expand step.
