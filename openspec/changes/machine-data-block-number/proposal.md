# Proposal

## Why

The adapter needs the data block of each Siemens machine. Today the number lives in the optional PLC connection record. The owner wants a mandatory number on each machine on `S7_300`, `S7_1200`, or `S7_1500`.

The change satisfies R15, R20, R28, R29, R31, and R38. It uses A9, A14, A24, A32, A47, and the new A48.

## What Changes

- New column `machine.db_number`. Two `CHECK` constraints make it mandatory on a live S7 machine and refuse it on `CodesysIec` and `TwinCat3`. The platform limits 16 000 and 59 999 stay.
- **BREAKING** `CreateMachine` gets the form field `db_number`. An S7 machine without a number fails with a new code. An IEC machine with a number also fails with a new code.
- New command `SetMachineDataBlock` on `POST /machines/{code}/data-block`. It needs `MachineManage` and writes the new audit action `MachineDataBlockSet`.
- **BREAKING** `SetPlcConnection` loses the field `db_number`. The connection form no longer shows it. The machine page shows the number and its edit form.
- The transfer document of the connection keeps the field `connection.db_number`. Its value now comes from the machine.
- The number keeps its role of ADR-0154 (d): the default data block for a parameter without `s7_address`.
- Migration `0013` stops and names each live S7 machine without a connection record. Else it adds the column. `0014` copies the number from `plc_connection` and adds a sync trigger for the previous release (ADR-0157). `0015` runs `VALIDATE CONSTRAINT` on both.
- An archived S7 machine without a record keeps no number. No command can change an archived machine.
- The change is the expand step of `reliability.md` §5. The column `plc_connection.db_number` stays as a copy for a rollback, and each command writes both.
- Documents: ADR-0156, ADR-0157, A48, and the sections of `docs/architecture/` that the ADR names.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `plant-model`: new requirements for the data block number of a machine, its command, and the migration. Changed requirements for the machine creation and the connection fields.
- `web-ui`: the field `db_number` in the machine form, the number on the machine page, and its edit form.

## Impact

- Crates: `recipe-domain`, `recipe-app`, `recipe-db`, `recipe-web`. `recipe-export` and `recipe-server` do not change.
- Migrations `0013` to `0015`, `crates/recipe-db/tests/grants.expected`, `deploy/pgtools/restore-roles.expected`, and `deploy/pgtools/restore-code.expected`.
- The entry `lock table` of `0013` in the list `EXEMPT` of `deploy/ci/migration_rewrite.py` (ADR-0157).
- Each test fixture and E2E seed that inserts an S7 machine.
- `docs/ops/`: a query before the upgrade. It lists each live S7 machine without a connection record.

## Out of scope

- The contract step. A later change drops `plc_connection.db_number` and relaxes `plc_connection_shape`.
- A comparison of the address of a parameter with the number of the machine.
- An S7 address without a data block part.
- The number in the export documents v1 and v2 or in `content_sha256`.
- A `row_version` on the machine. `SetMachineDataBlock` works like `RenameMachine`.
