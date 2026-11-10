# Tasks

## 1. Decision and normative documents

- [ ] 1.1 (docs/adr) Set ADR-0154 to `Accepted` after the owner approves this change. Add its index row and update the decision map rows "Pages of machines, units, and UDT types" and "Export" in `docs/adr/README.md`. Check: `ste-lint.py` on the two files reports 0 hard violations.
- [ ] 1.2 (docs/architecture) [mechanical] Add row A47 to `overview.md` §9 with the text of ADR-0154. Amend row A24 with the audit action `PlcConnectionExported`. Copy both rows into `domain-model.md` §8. Check: `ste-lint.py` and `diff` of the two copies.
- [ ] 1.3 (docs/architecture) Add the ten rows of design D4 to `reliability.md` §1.2. Add `PlantErrors` and `CipherFault` to §1.1 and `security.plc_credential_key_file` to §7.11. Add the transfer route to the `timeout.export` row of §7.4. Check: `ste-lint.py`.
- [ ] 1.4 (docs/architecture) Add the table `plc_connection` and its two check constraints to `data-model.md` §3, its limits to §1.3, its grants to §5.2. Add the rows `SetPlcConnection` and `ExportPlcConnection` to §6, the second in the form of `ExportVersion`. Add the two actions to §10. Check: `ste-lint.py`.
- [ ] 1.5 (docs/architecture) Add the three routes of design D8 to `api.md` §2.2 and §2.7, with the query value `recipe`. Add the matrix rows to §3.3 and the connection document to §6. Add the value objects to `domain-model.md` §2, the command to §5, and the term to §7. Check: `ste-lint.py`.

## 2. Value objects (recipe-domain)

- [ ] 2.1 (recipe-domain) Write failing tests for `PlcHost::parse`: `192.168.1.20` and `plc-line1.plant.local` pass. `192.168.1.300`, `plc_01:102`, `fe80::1`, `010.0.0.1`, `1.2.3`, `plc.local.`, a label of 64 characters, a label with a hyphen at its start or end, and 254 characters fail. Then implement `PlcHost` and `HostReason`.
- [ ] 2.2 (recipe-domain) Write failing tests for `Rack::parse` (`0`, `7` pass, `8`, `-1`, `x` fail) and `Slot::parse` (`0`, `31` pass, `32` fails). Then implement both with `RangeReason`.
- [ ] 2.3 (recipe-domain) Write failing tests for `DataBlockNumber::parse` (`1`, `65535` pass, `0`, `65536` fail) and `TcpPort::parse` (`1`, `65535` pass, `0`, `65536` fail). Then implement both.
- [ ] 2.4 (recipe-domain) Write failing tests for `PlcAddress::parse` (1 and 200 characters pass, 201 characters and a control character fail). Write failing tests for `AmsNetId::parse` (`192.168.1.10.1.1` passes, five numbers and `256` fail). Then implement both.
- [ ] 2.5 (recipe-domain) Write failing tests for `CommMode::parse` (the four names pass, `Profinet` and `ads` fail). Write failing tests for `PlcLogin::parse` (1 and 64 characters pass, 0 and 65 characters fail). Then implement both.
- [ ] 2.6 (recipe-domain) Write failing tests for `PlcPassword::parse` (1 and 128 characters pass, 0 and 129 fail). Add a test that `format!("{:?}")` holds `<redacted>` and not the text. Then add `zeroize` to the workspace dependencies and implement the type without `Display`, `Serialize`, or `Clone`. Check: `cargo deny check`.

## 3. Platform shape and errors (recipe-domain)

- [ ] 3.1 (recipe-domain) Write a failing test that `error_codes.rs` holds `E-PLT-022..029`, `E-WEB-013`, and `E-WEB-014`. Then add the eight `PlantError` variants of design D4 and their table rows.
- [ ] 3.2 (recipe-domain) Write a failing test that `PlantErrors` keeps one or more errors in order and owns no code. Then implement the carrier as `RecipeErrors`.
- [ ] 3.3 (recipe-domain) Write failing tests for `PlcPlatform::max_data_block`: `16000` on `S7_300`, `59999` on `S7_1200` and `S7_1500`, `None` on `CodesysIec` and `TwinCat3`. Then add the function and replace the private limits of `recipe/draft/s7.rs` and `s7/prop.rs` with it. Check: the S7 address tests of `recipe-domain` pass unchanged.
- [ ] 3.4 (recipe-domain) Write failing tests for `PlcConnection::for_platform` on the S7 platforms. All fields pass. A port gives `E-PLT-022`. A missing slot gives `E-PLT-023`. A password gives `E-PLT-022`. Then implement the S7 branch.
- [ ] 3.5 (recipe-domain) Write failing tests for the data block limit: `16000` passes and `16001` fails on `S7_300`, `59999` passes and `60000` fails on `S7_1500`. Each fail gives `E-PLT-024` with `max`. Then implement the rule with `max_data_block`.
- [ ] 3.6 (recipe-domain) Write failing tests for the IEC branch: `Ads` on `CodesysIec` gives `E-PLT-025`, `Ads` with a text address gives `E-PLT-026`, a rack gives `E-PLT-022`. Then implement the branch.
- [ ] 3.7 (recipe-domain) Write failing tests for the credentials of design D3. `OpcUa` without a login gives `E-PLT-023`. `Ads` without both passes. `Ads` with a password and no login gives `E-PLT-023`. `ModbusTcp` with a login gives `E-PLT-028`. Then implement the rule.
- [ ] 3.8 (recipe-domain) Write failing tests for `PasswordPlan`. An empty field with a stored password gives `Keep`. An empty field without one gives `E-PLT-023` on `OpcUa`. `ModbusTcp` gives `Clear`. An S7 record gives no password plan. Then implement the plan.
- [ ] 3.9 (recipe-domain) Write a failing test for a request with the mode `OpcUa`, a rack, and no login on `TwinCat3`. It gives both errors in one `PlantErrors`. Then make `for_platform` collect every fault.
- [ ] 3.10 (recipe-domain) Write a failing test that `AuditAction` has `PlcConnectionSet` and `PlcConnectionExported` with these names. Then add the variants.

## 4. Storage (recipe-db, deploy)

- [ ] 4.1 (recipe-db) Write failing schema tests for `migrations/0012_plc_connection.sql`: a valid S7 row and a valid IEC row insert. A row of another platform violates the foreign key. A port on an S7 row violates `plc_connection_shape`. Then write the migration of design D1.
- [ ] 4.2 (recipe-db) Write a failing test for each range check. The cases are rack 8, slot 32, `db_number` 16001 on `S7_300`, `db_number` 60000 on `S7_1500`, port 0, mode `Ads` on `CodesysIec`, and each text limit. Then complete the checks.
- [ ] 4.3 (recipe-db) Write failing tests for `plc_connection_credentials`. Four cases each give a check violation. They are a login without a ciphertext, `OpcUa` without a login, `ModbusTcp` with a login, and an S7 row with a login. Then add the constraint.
- [ ] 4.4 (recipe-db) Write a failing test that an update increments `row_version` through `plc_connection_a_bump`. Write a negative test that `recipe_app` cannot set `row_version`. Then add the trigger.
- [ ] 4.5 (recipe-db) [mechanical] Add the grants of design D1 to `grants.expected`. Check: the grant test fails before and passes after the grants in the migration.
- [ ] 4.6 (deploy) Update `deploy/pgtools/restore-code.expected` with the trigger of the migration and `restore-roles.expected` with its grants. Check: the restore test of `deploy/pgtools/tests` passes after migration 0012 (`reliability.md` §7.7).
- [ ] 4.7 (recipe-db) Write a failing test of the adapter for the port `PlcConnections`. Cover these cases: read by machine, insert, update with `FOR UPDATE`, read with `FOR SHARE`, and no row. Then implement the adapter.
- [ ] 4.8 (recipe-db) Write a failing test for each boundary value, the host cases of task 2.1 included. A statement of the value gives the same result in the check constraint and in the value object of task group 2. Then fix any drift.
- [ ] 4.9 (recipe-db) Write failing race tests in `tests/race` on two connections. Two first saves give one record and one `E-WEB-013`. Two saves of one `row_version` give one `E-WEB-013`. A save against `ArchiveMachine` gives a commit before the archive or `E-PLT-010`. Then fix the locks.
- [ ] 4.10 (recipe-db) Write failing tests in `tests/atomic`. A forced failure of the audit insert of `SetPlcConnection` leaves no row and no change. A forced failure of the audit insert of `ExportPlcConnection` leaves no entry. Then fix the transaction.

## 5. Use cases (recipe-app)

- [ ] 5.1 (recipe-app) Write a failing test that `SetPlcConnection` without `MachineManage` returns `Forbidden` before any read and writes one security event. Then add the use case shell, the port `PlcConnections`, and the permission check.
- [ ] 5.2 (recipe-app) Write a failing test that `SetPlcConnection` in read-only mode returns `ReadOnlyMode` and writes nothing. Then add the gate.
- [ ] 5.3 (recipe-app) Write failing tests for the port `CredentialCipher` with an in-memory fake: `seal` runs before the transaction, a failed `seal` gives `TaskFailed { stage: Token }`. Then define the port and `CipherFault`.
- [ ] 5.4 (recipe-app) Write failing tests for the transaction of design D6. An archived machine gives `E-PLT-010`. A stale `row_version` gives `E-WEB-013`. A new record starts at `row_version` 1. Then implement the transaction.
- [ ] 5.5 (recipe-app) Write failing tests for the `PasswordPlan` in the command. `Keep` keeps the ciphertext. `Clear` removes it. A first save and an update of an S7 record pass without a password. Then implement the plan.
- [ ] 5.6 (recipe-app) Write a failing test that the audit entry `PlcConnectionSet` holds the full row, `password_set`, and `password_changed`, and no password or ciphertext. Then build the entry.
- [ ] 5.7 (recipe-app) Write a failing test that `GetPlcConnectionForm` checks `MachineManage` first and returns the record without a password, with `password_set`. Then implement the query.
- [ ] 5.8 (recipe-app) Write failing tests for `ExportPlcConnection`. `VersionExport` runs first. A missing record gives `E-PLT-027`. A recipe of another machine gives `E-PLT-029`. An archived machine gives `E-PLT-010`. A failed `open` gives `E-WEB-014`. Then implement the checks.
- [ ] 5.9 (recipe-app) Write failing tests for the flow of design D9. The use case commits `PlcConnectionExported` before it returns the record. A failed audit insert and a timeout return an error without the password. Then implement the flow.
- [ ] 5.10 (recipe-app) [mechanical] Add the three operations to the generated permission matrix test. Check: the test fails before and passes after the rows of `api.md` §3.3.

## 6. Key and wiring (recipe-server, deploy)

- [ ] 6.1 (recipe-server) Write failing tests for `AesGcmCipher`: `seal` then `open` gives the password. A ciphertext of another machine id fails. Two `seal` calls give two nonces. A wrong version byte fails. Then add `aes-gcm` to the workspace dependencies and implement the adapter. Check: `cargo deny check`.
- [ ] 6.2 (recipe-server) Write failing tests that the startup check refuses a missing key file and a file of 31 or 33 bytes. The check writes one `ERROR` line and exits with code 1. Add a test that `PlcCredentialKey` has a redacting `Debug`. Then load `security.plc_credential_key_file`.
- [ ] 6.3 (recipe-server) Write a failing wiring test that `serve` builds the two use cases and the query with the cipher. Then wire them.
- [ ] 6.4 (deploy) Add the secret `plc_credential_key` to `deploy/compose.yaml` for the service `app` and to `deploy/ci/prepare.sh`. Check: `docker compose -f deploy/compose.yaml config`.
- [ ] 6.5 (docs/ops) Add the creation and the backup of `plc_credential_key` to `docs/ops/runbook.md`. Add a step to the section "Upgrade" that creates the file and checks its size before `docker compose up`. Check: `ste-lint.py`.
- [ ] 6.6 (docs/ops) Add the connection route to `docs/ops/transfer-client.md`. State the query value `recipe`, the default data block, and the rule to read the document directly before each load. Check: `ste-lint.py`.

## 7. Edit form and route (recipe-web)

- [ ] 7.1 (recipe-web) Write a failing HTTP test for `GET /machines/{code}/connection/edit`. The page shows only the fields of the platform, an empty password input without `value`, and the hidden `row_version`. Then add the route and the template.
- [ ] 7.2 (recipe-web) Write failing HTTP tests for `POST /machines/{code}/connection`: success gives `200` with `HX-Trigger`, no `HX-Request` gives `303`. Then add the handler.
- [ ] 7.3 (recipe-web) Write failing tests for the request struct. An empty field text gives no value. An empty host gives `E-PLT-023`. An empty port on `S7_1500` has no effect. The password field is an `Option<SecretField>` with a redacting `Debug`. Then implement the struct.
- [ ] 7.4 (recipe-web) Write failing HTTP tests for a bad host, a rack of 8, and a port on `S7_1500`. The answer is one `422` with all their field lines. Then map the reasons and `PlantErrors` to field lines.
- [ ] 7.5 (recipe-web) Write failing HTTP tests with the password `S3cret-PLC` for a refused save, a successful save, and a transfer read. The text appears in no answer body except the transfer answer, no log line, and no audit entry. Then fix any leak.
- [ ] 7.6 (recipe-web) Write failing HTTP tests for `403 E-WEB-002` (Operator), `503 E-WEB-006` (read-only mode), and `409 E-WEB-013` with its fragment. Then complete the error mapping.
- [ ] 7.7 (recipe-web) Write a failing HTTP test that the machine page shows the record without the password and with the password state. Then extend the machine page.
- [ ] 7.8 (recipe-web) Write a failing test that each new `fluent` key exists in `pl` and `en`. Then add the labels, the messages of the ten codes, and the password state texts.
- [ ] 7.9 (recipe-web) Write a failing test that the XSS payload corpus goes through the host, address, and login fields without markup. Then fix any escape fault.

## 8. Transfer route (recipe-web)

- [ ] 8.1 (recipe-web) Write failing HTTP tests for `GET /export/v1/machines/{code}/connection?recipe={ref}`: an `ExportClient` account gets `200` JSON with `Cache-Control: no-store`. A person account gets `401 E-IDN-003`. Then add the route.
- [ ] 8.2 (recipe-web) Write failing HTTP tests that the guard refuses `HX-Request: true` with `403 E-WEB-007`. A machine without a record gives `404 E-PLT-027`. A missing `recipe` gives `404 E-WEB-005`. Then complete the route.
- [ ] 8.3 (recipe-web) Write a failing HTTP test that archives `M8` and creates a new `M8`. A read with a recipe of the old `M8` gives `404 E-PLT-029`. Then fix the lookup.
- [ ] 8.4 (recipe-web) Write failing HTTP tests that a failed audit insert gives `500` and a timeout gives `503 E-WEB-009`. Neither answer holds the password. Then fix the order of commit and answer.
- [ ] 8.5 (recipe-web) Write a failing HTTP test that the route answers `200` in read-only mode and writes `PlcConnectionExported`. Then fix the gate if necessary.
- [ ] 8.6 (recipe-web) Write a failing test that the export documents v1 and v2 hold no connection field. They also keep `content_sha256` and every `s7_address` after a connection change. Then fix any leak.

## 9. Browser regression and checks

- [ ] 9.1 (e2e) Write a failing Playwright test. A ProcessEngineer saves a `TwinCat3` connection and reloads. The test sees an empty password field with the stored state. Then fix the page. Check: `npm test` in `e2e`.
- [ ] 9.2 (workspace) Run `cargo clippy --all-targets --all-features -- -D warnings`, `cargo test --workspace --all-features --locked`, `cargo fmt --check`, and `cargo deny check`. All four pass.
- [ ] 9.3 (openspec) Run `openspec validate plc-connection-config --strict` and the `ste-lint.py` command of `CLAUDE.md`. Both report no error.

## Workflow follow-up

- `code-review` after the apply, then `superpowers:verification-before-completion`.
- `openspec-archive-change` after `cargo test` and `openspec validate` pass.
