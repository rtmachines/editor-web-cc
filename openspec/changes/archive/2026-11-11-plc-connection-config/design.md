# Design

## Context

See `proposal.md`, section Why. These facts of the normative documents shape the design:

- R03 and A47: no crate holds a PLC driver (`overview.md` §7). ADR-0154 records the reading of R03.
- `machine` has `plc_platform` and the unique pair `(id, plc_platform)` (`data-model.md` §3.1). No command changes the platform, and I21 freezes it after the first recipe.
- `row_version` exists only on `recipe_version` (ADR-0008, `api.md` §4). The trigger function `bump_row_version` is generic (`data-model.md` §4.4).
- `api.md` §6 calls the export document the only machine-readable output. `content_sha256` is frozen at approval (ADR-0019).
- The transfer routes use HTTP Basic of an `ExportClient` account (ADR-0109, `api.md` §2.7, §7.7). A45 gives that role only `VersionExport`.
- Keys of 32 bytes come from Docker secret files (`reliability.md` §7.11, `security.login_hmac_key_file`).
- `E-PLT-022..029` and `E-WEB-013..019` are free (`reliability.md` §1.2).

## Goals / Non-Goals

**Goals:**

- One value object with one `parse` for each field (`reliability.md` §4.3).
- One domain function that checks the fields of a platform and lists every fault at once (ADR-0041).
- The password stays inside two types and two routes: the sealed column and the transfer answer.

**Non-Goals:**

- A second platform field, a connection test, or a PLC protocol client.
- Key rotation. The sealed form keeps a version byte so that a later ADR can add it.

## Decisions

### D1. Table `plc_connection`, one row per machine

Columns: `id`, `machine_id`, `plc_platform`, `host`, `rack`, `slot`, `db_number`, `port`, `address`, `comm_mode`, `login`, `password_sealed`, `row_version`, `created_at`, `updated_at`.

- `UNIQUE (machine_id)` gives one row per machine and serves as the index of the foreign key.
- The composite foreign key `(machine_id, plc_platform)` references `machine (id, plc_platform)`. A record therefore cannot hold another platform (I5 pattern).
- `CHECK plc_connection_shape`: an S7 row has `rack`, `slot`, `db_number` and no IEC column. An IEC row has `port`, `address`, `comm_mode` and no S7 column.
- `CHECK plc_connection_credentials`: `login` and `password_sealed` are both null or both set. `OpcUa` and `CodesysV3` need both. `ModbusTcp` permits neither. An S7 row has neither.
- Range checks mirror the value objects: rack `0..7`, slot `0..31`, `db_number` by platform, port `1..65535`, `comm_mode` by platform, text lengths of `data-model.md` §1.3.
- The database also mirrors the text grammars. A `CHECK` on `host` enforces the host format of `PlcHost`. A `CHECK` on `address` and on `login` refuses a control character. A `CHECK` on `address` enforces the AMS Net ID format when `comm_mode` is `Ads`.
- `BEFORE UPDATE` trigger `plc_connection_a_bump` runs `bump_row_version`.
- No trigger code is new. A check violation maps to `E-DB-003`, because the domain refuses the value first.
- Grants: `SELECT`, `INSERT`, and column `UPDATE` on the ten value columns for `recipe_app`. The ten columns are `host`, `rack`, `slot`, `db_number`, `port`, `address`, `comm_mode`, `login`, `password_sealed`, and `updated_at`. No `DELETE`. No `UPDATE` on `row_version`. The `UPDATE` grant permits `FOR UPDATE` and `FOR SHARE`.
- The restore check compares each function, trigger, and grant with `deploy/pgtools/restore-code.expected` and `restore-roles.expected` (`reliability.md` §7.7). The migration therefore updates both files in the same change.
- Alternative: columns on `machine`. Rejected: nine nullable columns on a hot table, and `machine` has no `row_version`.

### D2. Value objects in `recipe-domain`, module `plant::connection`

| Type | Grammar | Reason type |
|---|---|---|
| `PlcHost` | IPv4 dotted decimal, or RFC 1123 host name of at most 253 characters. A name of digits and dots only must be IPv4 | `HostReason` |
| `Rack` | `0..=7` | `RangeReason` |
| `Slot` | `0..=31` | `RangeReason` |
| `DataBlockNumber` | `1..=65535`. The platform rule of D3 narrows it | `RangeReason` |
| `TcpPort` | `1..=65535` | `RangeReason` |
| `PlcAddress` | 1 to 200 characters, no control character | `ConnectionTextReason` |
| `AmsNetId` | six numbers `0..=255` with five dots | `AmsNetIdReason` |
| `CommMode` | `Ads`, `OpcUa`, `CodesysV3`, `ModbusTcp` | `ModeReason` |
| `PlcLogin` | 1 to 64 characters, no control character | `ConnectionTextReason` |
| `PlcPassword` | 1 to 128 characters | `ConnectionTextReason` |

- `PlcHost` refuses an IPv4 number with a leading zero (`010.0.0.1`) and fewer than four numbers (`1.2.3`). It also refuses a trailing dot (`plc.local.`), a label of more than 63 characters, and a hyphen at the start or end of a label.
- A reason owns no code. `recipe-web` wraps it into `WebError::BadRequest`, which gives `E-WEB-008` with a line under the field (`reliability.md` §1.1).
- Empty field rule. `recipe-web` turns an empty field text into no value before `parse`. A required field without a value gives `E-PLT-023`. An empty field of another platform has no effect. An empty password field means "keep" (D3).
- `PlcPassword` has a hand-written `Debug` that prints `PlcPassword(<redacted>)`. It has no `Display`, no `Serialize`, and no `Clone`. It clears its buffer on drop with `zeroize`.
- The request struct of `recipe-web` holds the password as `Option<SecretField>`. `SecretField` has a redacting `Debug`, `zeroize` on drop, and no `Serialize`. The key holder `PlcCredentialKey` of `recipe-server` follows the same rules.
- `PlcLogin` is not a secret. The page and the audit entry show it.

### D3. Platform shape `PlcConnection`

```rust
pub enum PlcConnection {
    S7 { host: PlcHost, rack: Rack, slot: Slot, db_number: DataBlockNumber },
    Iec { host: PlcHost, port: TcpPort, address: PlcAddress, mode: CommMode, login: Option<PlcLogin> },
}
```

- `PlcConnection::for_platform(machine: MachineCode, platform, ConnectionFields, stored: PasswordState) -> Result<ConnectionPlan, PlantErrors>` is the one constructor. `machine` names the machine in each error. `ConnectionFields` holds an `Option` of each parsed value, the password included. `ConnectionPlan` holds the `PlcConnection` and a `PasswordPlan`.
- The function checks in this order. First, a field not on the platform (`E-PLT-022`). Then a missing field (`E-PLT-023`) and the data block limit (`E-PLT-024`). Then the mode on the platform (`E-PLT-025`), the AMS Net ID for `Ads` (`E-PLT-026`), and the credentials of the mode (`E-PLT-028`). It collects every fault.
- The data block limit is 16 000 on `S7_300` and 59 999 on `S7_1200` and `S7_1500` (ADR-0116 (c), ADR-0140 (a)). The two values are facts of the platform, not settings (ADR-0116 (h)).
- `PlcPlatform::max_data_block() -> Option<u16>` holds the two limits. It gives `None` on `CodesysIec` and `TwinCat3`. The S7 address checks of `recipe/draft/s7.rs` and `s7/prop.rs` use it in place of their private constants.
- `db_number` is the default data block of the machine. The adapter uses it only for a parameter without `s7_address`. A parameter with `s7_address` keeps the data block of its address. The connection record never changes an address of the export document.
- `PlantErrors` is a new carrier of one or more `PlantError` values without a code, as `RecipeErrors` (ADR-0027). `AppError` gets `#[from] PlantErrors`.

Credentials of the mode:

| Platform and mode | Login | Password |
|---|---|---|
| `S7_300`, `S7_1200`, `S7_1500` | refused, `E-PLT-022` | refused, `E-PLT-022` |
| `OpcUa`, `CodesysV3` | required, `E-PLT-023` | required: a new password or a stored one |
| `Ads` | optional | with a login: as `OpcUa`. Without a login: refused, `E-PLT-023` for `login` |
| `ModbusTcp` | refused, `E-PLT-028` | refused, `E-PLT-028` |

- `PasswordPlan` is `Keep`, `Replace(PlcPassword)`, or `Clear`. An empty password field with a stored password gives `Keep`. An empty password field without a stored password gives `E-PLT-023` when the mode needs a password.
- A mode without credentials gives `Clear`. The command then removes a stored ciphertext. An S7 record never holds a password.
- `PasswordState` comes from the unlocked read of D6 step 2. The `row_version` compare of step 5 proves that the state did not change.

### D4. Error codes

The rows go into `reliability.md` §1.2 before the variants.

| Code | Variant | HTTP |
|---|---|---|
| `E-PLT-022` | `PlantError::ConnectionFieldNotOnPlatform { code, field, platform }` | 422 |
| `E-PLT-023` | `PlantError::ConnectionFieldMissing { code, field }` | 422 |
| `E-PLT-024` | `PlantError::ConnectionDataBlockOutOfRange { code, number, max }` | 422 |
| `E-PLT-025` | `PlantError::CommModeNotOnPlatform { code, mode, platform }` | 422 |
| `E-PLT-026` | `PlantError::AddressNotAmsNetId { code, address }` | 422 |
| `E-PLT-027` | `PlantError::ConnectionNotConfigured { code }` | 404 |
| `E-PLT-028` | `PlantError::ConnectionFieldNotInMode { code, field, mode }` | 422 |
| `E-PLT-029` | `PlantError::ConnectionRecipeMismatch { code, reference }` | 404 |
| `E-WEB-013` | `AppError::ConnectionConcurrentEdit { code, expected, actual }` | 409 |
| `E-WEB-014` | `AppError::SecretUnreadable { code }` | 500 |

- No variant carries the password or the ciphertext. `field` is the name of the form field.
- `E-PLT-023` names no platform, because a mode can also require a field.
- `E-WEB-013` is a new code and not `E-WEB-001`, because `ConcurrentEdit` names a recipe and a version.

### D5. Password encryption

- New port `CredentialCipher` in `recipe-app`: `seal(&self, machine: MachineId, password: &PlcPassword) -> Result<SealedSecret, CipherFault>` and `open(&self, machine: MachineId, sealed: &SealedSecret) -> Result<PlcPassword, CipherFault>`.
- `CipherFault` is a reason without a code, as `WebhookFault`. The use case maps a failed `open` to `AppError::SecretUnreadable` and a failed `seal` to `AppError::TaskFailed { stage: Token }`.
- The adapter `AesGcmCipher` lives in `recipe-server`, which loads the key. It uses the crate `aes-gcm` with a random nonce of 12 bytes for each `seal`.
- Sealed form: one version byte `0x01`, the nonce, the ciphertext, and the tag. The associated data is the machine id, so a copied ciphertext does not open on another machine.
- New setting `security.plc_credential_key_file` (`reliability.md` §7.11). Its value is `/run/secrets/plc_credential_key`. The file holds exactly one line. That line holds the 32 bytes of the key as 64 lower-case hex characters, as the other key files (`reliability.md` §7.1, ADR-0031, ADR-0035). A line feed at the end is permitted. There is no key ring.
- The startup check refuses a missing file and a file with a second line. It also refuses a file with 62 or 66 hex characters or with a character that is not lower-case hex.
- The startup check stays hard. The upgrade section of the runbook creates the key file before the new release. A check command of that section tests the file before `docker compose up`. `deploy/ci/prepare.sh` creates the file for CI.
- Alternative: `pgcrypto` in the database. Rejected: the key would travel in SQL text and could reach the database log.

### D6. Command `SetPlcConnection`

1. Check `MachineManage` (ADR-0013). Then the read-only gate gives `E-WEB-006` (A24).
2. Read `machine.plc_platform` and the `PasswordState` of the record without a lock. The platform never changes for a machine.
3. Run `PlcConnection::for_platform` and seal a new password outside the transaction (`reliability.md` §7.4).
4. Transaction: `machine FOR NO KEY UPDATE`, check `archived_at` (`E-PLT-010`), then `plc_connection FOR UPDATE`. This follows the order `machine` → children (ADR-0015).
5. Compare `row_version`. No row counts as 0. A mismatch gives `E-WEB-013`.
6. Insert or update the row with the `PasswordPlan`. Commit with one `PlcConnectionSet` entry (`UnitOfWork::commit`).

- The `machine` lock serializes two first saves, so the second save reads the new row and gets `E-WEB-013`.
- The `machine` lock also serializes a save with `ArchiveMachine`. A save after the archive gets `E-PLT-010`.
- An S7 record has no password. Its save never reads or writes `password_sealed`.

### D7. Audit

- New `AuditAction` variants `PlcConnectionSet` and `PlcConnectionExported` (`data-model.md` §10). `entity_type` is `plc_connection`. `aggregate_type` is `machine`.
- `PlcConnectionSet` stores the full row without `password_sealed`, plus `password_set` and `password_changed`. A create stores only `after_state`.
- `PlcConnectionExported` stores `machine_id`, `row_version`, `schema_version`, and the reference number of the recipe. The actor is the `ExportClient` account, with no session reference (ADR-0109 (mb)).
- `audit_log` needs no migration. Its checks accept the new names.

### D8. Pages and routes

| Method | Path | Use case | Permission |
|---|---|---|---|
| GET | `/machines/{code}/connection/edit` | `GetPlcConnectionForm` | `MachineManage` |
| POST | `/machines/{code}/connection` | `SetPlcConnection` | `MachineManage` |
| GET | `/export/v1/machines/{code}/connection?recipe={ref}` | `ExportPlcConnection` | `VersionExport` |

- A `409 E-WEB-013` answer uses the error fragment `error.html` in `#error-panel`. It shows the code, the sentence "Another user changed the connection. Reload the page.", and the `request_id`. It does not use `conflict.html`, because that fragment names a recipe version.
- The machine page shows the record without the password, with a text "password stored" or "no password". `GetMachine` reads it with `RecipeView`.
- The form shows only the fields of the platform. It has the hidden field `row_version` and `hx-sync="closest form:drop"`. The password input has `type="password"`, `autocomplete="new-password"`, and never a `value`.
- A success answers `200` with the fragment and `HX-Trigger: {"rowVersion": <new>}`. A request without `HX-Request` gets `303` to `/machines/{code}`.
- The request struct uses `#[serde(deny_unknown_fields)]` and holds every field of both shapes as an `Option<String>`. Thus a field of another platform with a value reaches the domain and gives `E-PLT-022`.
- New `fluent` keys in the bundles `pl` and `en` for the labels, the ten messages, and the password state.
- When the refused request held a password, the form shows a hint under the password input. The hint asks the user to type the password again.

### D9. Transfer route of the connection

- The route sits under `/export/`. It gets the navigation guard (ADR-0055), the Basic check of T1 and T2 (ADR-0109 (m)), `timeout.export`, and `Cache-Control: no-store`.
- Permission: `VersionExport`. A45 and ADR-0109 (b) stay unchanged. Only an `ExportClient` account passes the Basic check, so no person reaches the route.
- The request requires the query value `recipe`. It follows the rule of `{ref}` (`api.md` §1). A missing, malformed, or unknown reference gives `404 E-WEB-005`.
- The machine lookup follows the `{code}` rule of `api.md` §2.7 (live machine first, then the archived machine archived last). An archived machine gives `409 E-PLT-010`. A recipe of another machine gives `404 E-PLT-029`. Thus a reused machine code (A32) never gives the connection of a new controller for a recipe of the old machine.
- Body: `schema_version` `"1"`, `machine { code, plc_platform }`, `recipe { reference_number }`, `row_version`, and `connection` with the fields of the platform. The numbers are JSON integers. A body with credentials holds `login` and `password` as JSON strings.
- In read-only mode the route continues, as every export does (A24). ADR-0154 adds `PlcConnectionExported` to the audit actions of A24.

Flow of `ExportPlcConnection` (`data-model.md` §6, as `ExportVersion`):

1. Check `VersionExport`. Parse `{code}` and `recipe`.
2. Take an `export.permits` permit. `timeout.export` covers the wait, the reads, the locks, and the `open` (`reliability.md` §7.4).
3. Transaction `READ COMMITTED`: `machine FOR SHARE`, read the recipe, then `plc_connection FOR SHARE`. Check the archive state, the recipe machine, and the record.
4. Run `open`. A failure gives `500 E-WEB-014`, rolls back, and writes no entry.
5. Insert `PlcConnectionExported` and commit. The audit insert and the commit run after the limit, so a timeout never hides a committed entry.
6. Build the JSON body only after the commit. A failed audit insert or a timeout gives an error answer without the password.

Change of the record. The connection document has no change marker, and a record change does not change `content_sha256`. The transfer client reads the document directly before each load and keeps no copy. `docs/ops/transfer-client.md` states this rule.

### D10. Documents of the change

- ADR-0154 `plc-connection-settings`: items D1 to D9, the reading of R03, and the amendment of A24.
- `overview.md` §9: new row A47, amended row A24. `domain-model.md` §8 gets identical copies.
- `api.md` §2.2, §2.7, §3.3 (matrix rows), §6 (first sentence names the connection document).
- `data-model.md` §1.3, §3 (new table), §5.2 (grants), §6 (operation row), §10 (actions).
- `domain-model.md` §2 (value objects), §5 (command row), §7 (term "PLC connection record").
- `reliability.md` §1.1 (`PlantErrors`, `CipherFault`), §1.2 (ten rows), §7.11 (key file setting).
- `docs/adr/README.md`: index row and the decision map rows "Pages of machines, units, and UDT types" and "Export".
- `docs/ops/runbook.md` (key creation, backup, upgrade check) and `docs/ops/transfer-client.md` (route, `recipe` value, read before each load).

## Risks / Trade-offs

- [A lost key makes every stored password unreadable] → The runbook backs up `plc_credential_key` with the other secret files. A re-entry of each password repairs the records.
- [The editor database now holds PLC credentials] → AES-256-GCM, the key outside the database, and one audited read route. A backup of the database without the key holds no readable password.
- [`VersionExport` also belongs to Operator] → Only an `ExportClient` account passes the Basic check, and no session route gives the password. A later ADR can add a separate permission.
- [The adapter needs the AMS port of TwinCAT and not the TCP port] → The field `port` is the TCP port of the endpoint. The adapter derives the AMS port. This stays a fact of A47.
- [A check constraint and a value object can drift] → One `recipe-db` test sends each boundary value through both.
- [A missing key file stops the server after an upgrade] → The upgrade section of the runbook creates and checks the file before `docker compose up`.
- [Each load writes one more audit entry] → One `PlcConnectionExported` per load is the price of an audited password read. The client never polls the route.

## Migration Plan

1. Create the secret file `plc_credential_key` with 32 random bytes on the host. Add it to `deploy/compose.yaml` for the service `app`.
2. Release with `migrations/0012_plc_connection.sql`, `restore-code.expected`, and `restore-roles.expected`. The table is new, so no rewrite and no lock of an existing table.
3. Rollback: the earlier release ignores the table. A forward migration can drop it.

## Open Questions

- The runbook names a key ceremony for `plc_credential_key`. The owner can choose the tool later without a change of the specs.
