# Tasks

## 1. Decision and normative documents

- [x] 1.1 (docs/adr) Set ADR-0156 to `Accepted` after the owner approves this change. Add the note "Amended by ADR-0156" to the Status line of ADR-0154. Add the index row and add ADR-0156 to the decision map row "Pages of machines, units, and UDT types" in `docs/adr/README.md`. Check: `ste-lint.py` on the three files reports 0 hard violations.
- [x] 1.2 (docs/architecture) [mechanical] Add row A48 to `overview.md` §9 with the text of ADR-0156 (a) and (c). Copy the row into `domain-model.md` §8. Check: `ste-lint.py` and `diff` of the two copies.
- [x] 1.3 (docs/architecture) Add the rows `E-PLT-050` and `E-PLT-051` of design D4 to `reliability.md` §1.2. Rename the variant of `E-PLT-024` to `DataBlockOutOfRange`. Check: `ste-lint.py`.
- [x] 1.4 (docs/architecture) Update `data-model.md`. Add the column and both checks to §3.1 and the mapping to §1.2. Add the copy note to §3.18, the grant to §5.2, and the files to §8. Check: `ste-lint.py`.
- [x] 1.5 (docs/architecture) Update `data-model.md` §6: the rows `CreateMachine`, `SetPlcConnection`, and `ExportPlcConnection`, and the new row `SetMachineDataBlock` of design D6. Add `MachineDataBlockSet` to §10. Check: `ste-lint.py`.
- [x] 1.6 (docs/architecture) Update `domain-model.md` §2 (the field of `Machine`, the module of `DataBlockNumber`, the S7 shape) and §5 (the commands). Update `api.md` §2.2, §3.3, and §6.5. Check: `ste-lint.py`.

## 2. Migrations (recipe-db)

- [x] 2.1 (recipe-db) Write a failing migration test. A live `S7_1500` machine without a record stops `0013` with its code in the message, and `machine.db_number` does not exist. Then write `0013_machine_db_number.sql` of design D2 with the column, both checks `NOT VALID`, and the grant. Check: `python deploy/ci/migration_rewrite.py migrations` reports no form.
- [x] 2.2 (recipe-db) [mechanical] Add `db_number` to each `INSERT INTO machine` of an S7 machine in `crates/recipe-db/tests/storage/` and in `src/transaction/machines.rs` test code. Check: `cargo test -p recipe-db` compiles and the old tests pass.
- [x] 2.3 (recipe-db) Write a failing migration test. The record number `100` of `M5` reaches `machine.db_number`. An archived `S7_300` machine without a record keeps `NULL`. Then write `0014_machine_db_number_copy.sql`.
- [x] 2.4 (recipe-db) Write failing constraint tests. A live S7 row without a number and an IEC row with a number give `23514`. `16001` on `S7_300` and `60000` on `S7_1500` also give `23514`. `16000` and `59999` pass. Then write `0015_machine_db_number_validate.sql`.
- [x] 2.5 (recipe-db) [mechanical] Add `machine.db_number|UPDATE` to `crates/recipe-db/tests/grants.expected` and `GRANT|machine.db_number|UPDATE` to `deploy/pgtools/restore-roles.expected`. Check: the grant test and the restore code test pass.

## 3. Domain (recipe-domain)

- [x] 3.1 (recipe-domain) Write a failing test that `error_codes.rs` holds `E-PLT-050` and `E-PLT-051`. Then add both variants of design D4 and rename `ConnectionDataBlockOutOfRange` to `DataBlockOutOfRange`. Widen the code range test of `error_codes/tests.rs` to `E-PLT-050..069`. Check: `E-PLT-024` keeps its code test.
- [x] 3.2 (recipe-domain) [mechanical] Move `DataBlockNumber` and its tests from `plant::connection` to `plant`. Keep the export from `plant`. Check: the tests of `data_block_number` pass unchanged.
- [x] 3.3 (recipe-domain) Write failing tests for `check_data_block`. `S7_300` without a number gives `E-PLT-050`. `TwinCat3` with `10` gives `E-PLT-051`. `16001` on `S7_300` gives `E-PLT-024` with `max` `16000`. `59999` on `S7_1500` and no number on `CodesysIec` pass. Then implement it.
- [x] 3.4 (recipe-domain) Write failing tests for `Machine::create` and `Machine::set_data_block` with the cases of 3.3. `set_data_block` on an archived machine gives `E-PLT-010`. Then add the field `db_number` and both functions.
- [x] 3.5 (recipe-domain) Write failing tests for `TryFrom<StoredMachine>`. A live S7 row without a number fails. An archived S7 row without a number passes. An IEC row with a number fails. Then implement the checks.
- [x] 3.6 (recipe-domain) Write a failing test that `PlcConnection::for_platform` on `S7_300` passes with the host, the rack, and the slot only. Then delete `db_number` from `PlcConnection::S7` and `ConnectionFields`. Check: `cargo test -p recipe-domain`.
- [x] 3.7 (recipe-domain) Write a failing test that the action list holds `MachineDataBlockSet` and that `RowAction::MachineDataBlockSet` maps to it. Then add the action.

## 4. Use cases (recipe-app)

- [x] 4.1 (recipe-app) Write failing tests for `CreateMachine`. `S7_300` without a number gives `E-PLT-050` and opens no transaction. `CodesysIec` with a number gives `E-PLT-051`. An S7 machine with `100` reaches the port. Then add `db_number` to `CreateMachineInput` and the memory store.
- [x] 4.2 (recipe-app) Write failing tests that `Operation::SetMachineDataBlock` needs `MachineManage` and is not a read-only exception. Then add the operation to the permission matrix.
- [x] 4.3 (recipe-app) Write failing tests for the port `set_machine_data_block` in the memory store: the machine and the copy of a record change together. Then add the port and its memory adapter.
- [x] 4.4 (recipe-app) Write failing tests for `SetMachineDataBlock`. Operator gets `Forbidden` and one security event. An IEC machine gives `E-PLT-051` before the transaction. An archived machine gives `E-PLT-010`. A success commits one `MachineDataBlockSet`. Then implement the use case.
- [x] 4.5 (recipe-app) Write failing tests that `SetPlcConnection` and `GetPlcConnectionForm` hold no `db_number`. `ExportPlcConnection` gives the number of the machine. Then update the three use cases.

## 5. Storage adapters (recipe-db)

- [x] 5.1 (recipe-db) Write a failing contract test. `insert_machine` stores `db_number`, the image of `MachineCreated` holds it, and the machine loader reads it. Then update the adapter.
- [x] 5.2 (recipe-db) Write a failing contract test for `set_machine_data_block`. Both columns change, and `plc_connection.row_version` rises by 1. A machine without a record changes only `machine`. Then implement the adapter of design D6.
- [x] 5.3 (recipe-db) Write a failing contract test. A new and a changed S7 record of `SetPlcConnection` copy `machine.db_number`. Then update the adapter of design D7.
- [x] 5.4 (recipe-db) Write a failing contract test. The connection read of `ExportPlcConnection` gives `machine.db_number` after `set_machine_data_block`. Then update the query.
- [x] 5.5 (recipe-db) Write a failing test for `SetMachineDataBlock` with a failed audit insert. Neither `machine` nor `plc_connection` changes. Then fix the adapter if the test finds a partial write.

## 6. Pages and routes (recipe-web)

- [x] 6.1 (recipe-web) Write failing tests for the machine form. The field `db_number` exists after the platform list with its hint. An empty field is no value. `0` gives one `E-WEB-008` line. `S7_300` without a number gives `422 E-PLT-050` with the out-of-band form. Then update the request, `machine-new.html`, and both `fluent` bundles.
- [x] 6.2 (recipe-web) Write failing tests for the machine page. An S7 machine shows its number. An archived S7 machine without a number shows `machine-db-number-none`. An IEC machine, an Operator, and an archived machine get no data block form. Then update `machine.html` and the page query.
- [x] 6.3 (recipe-web) Write failing tests for `POST /machines/{code}/data-block`. HTMX gets `200`, the fragment, and a toast. A request without `HX-Request` gets `303`. `16001` on `S7_300` gives `422 E-PLT-024` with `#error-panel` and the out-of-band form. Then add the route and `machine-data-block-set.html`.
- [x] 6.4 (recipe-web) Write failing tests for the new route. An unknown field gives `E-WEB-008`. A bad CSRF token gives `403 E-WEB-007`. Read-only mode gives `503 E-WEB-006`. Add the field to the XSS corpus test. Then fix the route.
- [x] 6.5 (recipe-web) Write failing tests that `machine-connection.html` has no input `db_number`. A save with `db_number` gives one `E-WEB-008` line and no out-of-band form. Then delete the field from the request, the form, and the connection facts.

## 7. End-to-end suite and operator documents

- [ ] 7.1 (e2e) [mechanical] Fill `db_number` for each S7 machine in `e2e/tests/support/pages.ts` and `e2e/tests/jcs.spec.ts`. Check: `cd e2e && npm test` passes on the dev stack.
- [x] 7.2 (docs/ops) Add a query before the upgrade to `docs/ops/runbook.md`. It lists each live S7 machine without a record. Also add the rollback limit of design D2. Check: `ste-lint.py` and the query on the dev stack.
- [x] 7.3 (docs/ops) Name the machine as the source of `db_number` in `docs/ops/transfer-client.md`. Add the change to `CHANGELOG.md`. Check: `ste-lint.py`.
- [x] 7.4 (docs/user) Add the data block field to the machine steps of `docs/user/tutorial-first-recipe.md` and `docs/user/reference-screens.md`. Check: `ste-lint.py`.

## 8. Integration checks

- [x] 8.1 (workspace) Run `cargo clippy --all-targets --all-features -- -D warnings`, `cargo test --workspace --all-features --locked`, `cargo fmt --check`, and `cargo llvm-cov --workspace` with the gate of 80 % lines per crate. Check: each command passes.
- [x] 8.2 (openspec) Run `openspec validate machine-data-block-number --strict` and `ste-lint.py` on the scope of ADR-0023 and ADR-0135. Check: both report 0 errors.

## 9. Review fixes

- [x] 9.1 (recipe-db) Write failing migration tests of design D10 and ADR-0157. As `recipe_app`, an `UPDATE` of `plc_connection.db_number` in the style of the previous release sets `machine.db_number` to the same number. An out-of-range number gives `23514` and changes neither table. An equal number changes no `machine` row. An IEC record keeps `updated_at` of its machine through `0014`. Then change `0014_machine_db_number_copy.sql`: lock the `machine` rows in `id` order, copy only a number that is not `NULL`, then create the function and the trigger of `data-model.md` §4.13. Add the function and the trigger to `deploy/pgtools/restore-code.expected`. Check: the restore code test passes.
- [x] 9.2 (recipe-db) Write a failing migration test on the `0012` schema. An open transaction inserts a live S7 machine without a record. `0013` runs at the same time. The insert commits, and then `0013` fails with the stop message that names the machine. Then add `LOCK TABLE machine IN ACCESS EXCLUSIVE MODE` as the first statement of `0013_machine_db_number.sql`. Check: `python deploy/ci/migration_rewrite.py migrations` reports no form.
- [x] 9.3 (recipe-web) Write a failing test if no test covers each field of `MachinePage`. Then split `MachinePage::new` into functions below 50 lines. Check: the machine page tests pass unchanged.
- [x] 9.4 (recipe-web) Write a failing test that the data block form of `machine.html` shows its own hint. The hint names only the limit of the platform of the machine and has no text about other platforms. Then add the `fluent` key in `pl` and `en`. Check: the hint of `machine-new.html` stays unchanged.
- [x] 9.5 (recipe-db) Write constraint tests. `db_number` `0` on `S7_300` and `60000` on `S7_1200` give `23514`. Check: both tests pass on `0015`.
- [x] 9.6 (recipe-web) Write a failing test for the scenario "Connection form after a new data block number" of `specs/web-ui/spec.md`. A connection form loaded before `SetMachineDataBlock` and saved after it gets `409 E-WEB-013`. Then fix the route if the test fails for another reason.
- [x] 9.7 (deploy/ci) [codex] Seed an S7 machine with a PLC connection record in `deploy/ci/upgrade_smoke.py`. After the upgrade, check that `machine.db_number` equals the number of the record. In `crates/recipe-db/tests/storage/migration_0013.rs`, add a test that a live IEC machine without a record passes the stop check. Implementer: Codex, as for each deploy and CI task.
- [x] 9.8 (recipe-app, recipe-db) Write failing tests for `SetMachineDataBlock` with the current number. In the memory store and in `recipe-db`, the record and its `row_version` stay unchanged. The command still writes one `MachineDataBlockSet`, as `RenameMachine` with the current name. Then add `AND db_number IS DISTINCT FROM $n` to the copy statement of `data-model.md` §6 and the same rule to the memory store (ADR-0157 (j)).
- [x] 9.9 (recipe-app) Write a failing test with the memory store. `ExportPlcConnection` of an S7 record whose machine has no number gives `E-DB-008` and writes no entry. Then refuse that case in the use case with the corrupt-row error of `memory/rows.rs` (`StorageError::Other`). Check: `data-model.md` §6, row `ExportPlcConnection`.

## 10. Second review fixes

- [x] 10.1 (deploy/ci) [codex] The seed of upgrade_smoke.py runs on the previous and the head schema. restore-switch passes on the head schema.
- [x] 10.2 (recipe-app, recipe-web) SetMachineDataBlockOutput carries the platform. The data block answers build the hint from it. A failed read on a refused path gives an error answer, not an empty hint.
- [x] 10.3 (recipe-db) A positive test covers the INSERT arm of plc_connection_sync_db_number.
- [x] 10.4 (recipe-db, recipe-web) Split the two tests over 50 lines in migration_0013.rs and data_block_commands_tests.rs.
- [x] 10.5 (docs) ADR-0157 names step 18, the window between 0013 and 0014, runbook copy sentence, task 9.2 lock mode.
- [x] 10.6 (recipe-domain, recipe-web) Split the functions that this change grew over 50 lines. The tests and the seeded data stay the same.

## Workflow follow-up

- Review the code with `code-review` and fix each CRITICAL and HIGH finding.
- Archive the change with `openspec-archive-change` after `cargo test` and `openspec validate` pass.
- Propose the contract change: drop `plc_connection.db_number` and relax `plc_connection_shape`.
