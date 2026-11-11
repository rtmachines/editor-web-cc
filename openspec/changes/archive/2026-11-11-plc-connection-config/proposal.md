# Proposal

## Why

An external adapter loads approved recipes into the PLCs. The adapter needs the connection settings of each PLC, and today no document or table holds them. The editor already manages each machine and its `plc_platform` (R15). The editor stores them and never connects to a PLC, so R03 stays true. Assumption A47 and ADR-0154 record this reading of R03.

The change satisfies R03, R15, R20, R28, R29, R31, R36, and R38. It uses A9, A24, A32, A45, and the new A47.

## What Changes

- New table `plc_connection`: one optional connection record per machine. The record holds only the fields of the platform of the machine.
- New value objects in `recipe-domain`, each with one `parse`: `PlcHost`, `Rack`, `Slot`, `DataBlockNumber`, `TcpPort`, `PlcAddress`, `AmsNetId`, `CommMode`, `PlcLogin`, `PlcPassword`.
- New command `SetPlcConnection` and form query `GetPlcConnectionForm`. Both need `MachineManage`. The command compares a `row_version` of the record and writes an audit entry.
- AES-256-GCM encrypts the PLC password. The key comes from a new Docker secret `plc_credential_key`. The password never appears in HTML, a form value, an audit entry, a log line, an error, or `Debug` output.
- Login and password depend on the mode. `OpcUa` and `CodesysV3` need both. `Ads` permits both or neither. `ModbusTcp` and the S7 platforms permit neither.
- `db_number` is the default data block of the machine. An address of a parameter keeps its own data block.
- New transfer route `GET /export/v1/machines/{code}/connection?recipe={ref}`. It uses HTTP Basic of an `ExportClient` account and gives the record with the password as JSON. The recipe must belong to the live machine, so a reused machine code cannot give the wrong controller. The route commits its audit entry before the answer. It is outside the export document and outside `content_sha256`.
- New error codes in `reliability.md` §1.2. New audit actions `PlcConnectionSet` and `PlcConnectionExported`.
- Documents: ADR-0154, A47, an amended A24, and the sections of `api.md`, `data-model.md`, `domain-model.md`, and `reliability.md` that the ADR names.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `plant-model`: new requirements for the connection record of a machine, its fields per platform, the password secrecy, the edit command, and the transfer route.

## Impact

- Crates: `recipe-domain`, `recipe-app`, `recipe-db`, `recipe-web`, `recipe-server`. `recipe-export` does not change.
- Migration `0012_plc_connection.sql`, `crates/recipe-db/tests/grants.expected`, and the restore files `deploy/pgtools/restore-code.expected` and `restore-roles.expected`.
- Deploy: a new secret file in `deploy/compose.yaml` and `deploy/ci/prepare.sh`. The runbook gets its creation, backup, and a check before an upgrade.
- `recipe-domain`: `PlcPlatform::max_data_block` replaces the private data block limits of the S7 address checks.
- New dependency `aes-gcm` (RustCrypto). `cargo deny check` must pass.

## Out of scope

- A PLC driver, a connection test, or any network traffic to a PLC.
- The adapter itself.
- A server configuration. The owner removed it from this change.
- Key rotation of `plc_credential_key`.
- A command that changes `plc_platform`. No such command exists (I21).
- Connection data in the export documents v1 and v2 or in `content_sha256`.
