# ADR-0154: PLC connection settings of a machine

Status: Accepted. Amended by ADR-0155 (`app_setting` records `security.plc_credential_key_file`)
Date: 2026-11-10
Deciders: project owner

## Context

An external adapter loads approved recipes into the PLCs. The adapter needs the connection settings of each PLC. No document and no table holds them.

These facts shape the decision:

- R03 says "No machine communication". `overview.md` §7 reads it as "No PLC driver in any crate".
- `machine.plc_platform` is the type of the controller (R15). No command changes it, and I21 freezes it after the first recipe.
- `api.md` §6 calls the export document the only machine-readable output. `content_sha256` is frozen at approval (ADR-0019).
- `row_version` exists only on `recipe_version` (ADR-0008).
- The adapter needs the PLC password as plain text. A one-way hash therefore cannot store it.
- The project owner decided four points on 2026-11-10. The points are AES-256-GCM with a key from a Docker secret, a separate JSON route, no server configuration, and the field ranges of this ADR.

## Decision drivers

- The editor never communicates with a PLC (R03).
- Data safety has priority (R31). A PLC password is a secret.
- The domain checks all external input (R38).
- Every change has an audit entry (R20). Every command is atomic (R29).
- The export document and its hash stay stable (ADR-0019).

## Considered options

### Option A: A connection table, an encrypted password, and a separate transfer route

- Pros: the export documents and their hash do not change. The password has one audited exit.
- Cons: a new key to keep. A new route.

### Option B: Connection data in a new export document v3, outside the hash

- Pros: one file for the adapter.
- Cons: a new schema version. The password would be in every recipe file.

### Option C: The editor stores no password

- Pros: no key.
- Cons: the adapter needs a second store for each machine. The owner chose to edit the password in the editor.

## Decision

The project uses **Option A**.

- (a) Reading of R03. The editor stores and edits the connection settings of a PLC. No crate opens a connection to a PLC or holds a PLC protocol client. Assumption A47 records this reading.
- (b) Table. `plc_connection` holds at most one row per machine. A composite foreign key binds `(machine_id, plc_platform)` to `machine`. A check constraint permits only the fields of the platform.
- (c) Fields. `S7_300`, `S7_1200`, and `S7_1500` use the host, the rack `0..7`, the slot `0..31`, and the data block number. `CodesysIec` and `TwinCat3` use the host, the TCP port `1..65535`, the address, the mode, the login, and the password. An empty field text is no value.
- (d) Data block number. The limit is 16 000 on `S7_300` and 59 999 on `S7_1200` and `S7_1500`, as in ADR-0116 (c) and ADR-0140 (a). `PlcPlatform::max_data_block` holds both limits for every check. The number is the default data block for a parameter without `s7_address`. It never changes the address of a parameter.
- (e) Modes. `CodesysIec` permits `OpcUa`, `CodesysV3`, and `ModbusTcp`. `TwinCat3` permits `Ads`, `OpcUa`, and `ModbusTcp`. The mode `Ads` needs an AMS Net ID as the address.
- (ea) Credentials. `OpcUa` and `CodesysV3` need a login and a password. `Ads` permits both or neither. `ModbusTcp` and the S7 platforms permit neither. A check constraint enforces the same rule.
- (f) Host. The host is an IPv4 address or an RFC 1123 host name of at most 253 characters. The editor refuses IPv6.
- (g) Password. AES-256-GCM encrypts the password with a random nonce. The key has 32 bytes and comes from the Docker secret `plc_credential_key`. The machine id is the associated data.
- (h) Secrecy. The password never appears in HTML, a form value, an audit entry, a log line, an error, or `Debug` output. An empty password field keeps the stored password. A mode without credentials removes it.
- (ha) Key file. The server refuses to start without a key of 32 bytes. The upgrade section of the runbook creates the file and checks it before `docker compose up`.
- (i) Command. `SetPlcConnection` needs `MachineManage`. It compares a `row_version` of the record. No record counts as 0. It writes the audit action `PlcConnectionSet`.
- (j) Transfer route. `GET /export/v1/machines/{code}/connection?recipe={ref}` gives the record and the password as JSON to an `ExportClient` account with HTTP Basic. It needs `VersionExport` and writes the audit action `PlcConnectionExported`.
- (ja) Machine identity. The recipe `{ref}` must belong to the live machine `{code}`. A recipe of another machine gives `404 E-PLT-029`, and an archived machine gives `409 E-PLT-010`. Thus a reused code (A32) never gives the controller of a new machine.
- (jb) Audit first. The route commits `PlcConnectionExported` before it builds the answer. A failed insert or a timeout gives an error answer without the password.
- (jc) No change marker. The transfer client reads the connection document directly before each load and keeps no copy.
- (k) The connection record is not part of the export documents v1 and v2 or of `content_sha256`. `api.md` §6 names the connection document as the second machine-readable output.
- (l) Read-only mode. The transfer route continues. This ADR amends A24: `PlcConnectionExported` is an audit action in read-only mode.
- (m) The editor has no command that changes `plc_platform`. A later command of that kind needs a new ADR for the connection record.

Assumption A47 (text of the row in `overview.md` §9):

> The editor stores the PLC connection settings of each machine for an external adapter. It never opens a connection to a PLC. No crate holds a PLC protocol client. AES-256-GCM encrypts the password at rest.

## Consequences

### Positive

- The adapter gets the settings and the recipes from one system.
- Each read of a password has an audit entry.
- The export documents and their hash do not change.

### Negative

- A new secret file to create, keep, and back up.
- Ten new error codes with texts in the bundles `pl` and `en`.
- One more audit entry for each load of a recipe.

### Risks and mitigations

- Risk: a lost key makes every stored password unreadable. Mitigation: the runbook backs up the key with the other secret files. A user can enter each password again.
- Risk: Operator also holds `VersionExport`. Mitigation: only an `ExportClient` account passes the Basic check, and no session route gives the password.

## Related

- ADR-0008 (`row_version`), ADR-0013 (permissions), ADR-0015 (lock order), ADR-0019 (export), ADR-0041 (error lists), ADR-0055 (navigation guard), ADR-0109 (HTTP Basic), ADR-0116 and ADR-0140 (data block number)
- `overview.md` §7: R03, R15, R20, R29, R31, R38. §9: A24, A32, A45, A47
- Change `openspec/changes/plc-connection-config`
