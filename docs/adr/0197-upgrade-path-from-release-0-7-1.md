# ADR-0197: Upgrade path from release 0.7.1 to release 0.8.0

Status: Accepted. Amends ADR-0156 (h) and its rollback consequence, and ADR-0157 (the previous release of the expand period). Amends `docs/architecture/data-model.md` §3.18 and §4.13 (one linking sentence each)
Date: 2026-11-19
Deciders: project owner

## Context

Release 0.8.0 adds the PLC connection record of a machine (ADR-0154) and moves the data block number to the machine (ADR-0156). These facts shape the upgrade from 0.7.1:

- Tag `v0.7.1` holds the migrations `0001` to `0010`. `git grep plc_connection` and `git grep SetPlcConnection` find nothing in each tag from `v0.1.0` to `v0.7.1`.
- `migrations/0012_plc_connection.sql` line 6 creates the table `plc_connection`. The file has no `INSERT`, so on a database of 0.7.1 the table is empty after this file.
- `migrations/0013_machine_db_number.sql` lines 13 to 35 stop the file when a live S7 machine has no `plc_connection` row. The exception rolls back the whole file.
- A live S7 machine is a machine on `S7_300`, `S7_1200`, or `S7_1500` with `archived_at IS NULL` (`0013` lines 22 and 23).
- The message of `0013` line 29 says "Save the PLC connection of each listed machine with the previous release". Release 0.7.1 cannot save that record.
- At commit `6cc03ec`, `docs/ops/runbook.md` lines 1232 to 1266 and step 2.4 of the section "Upgrade" (lines 1301 to 1304) give the same instruction. The query of line 1251 reads `plc_connection`, so it fails on a database of 0.7.1.
- ADR-0156 item (h) on line 54 assumes a previous release that can save a PLC connection record. ADR-0156 line 70 assumes a previous release whose `CreateMachine` sets no `db_number`.
- ADR-0157 lines 11 to 17 and items (d) and (e) assume a previous release whose `SetPlcConnection` saves the number in `plc_connection` only.
- At commit `6cc03ec`, `data-model.md` §3.18 (line 1137), `data-model.md` §4.13 (line 1734), and runbook lines 1623 to 1625 also assume that previous release.
- `openspec/specs/plant-model/spec.md`, requirement "Copy of the data block number in the connection record" (lines 1394 to 1417), assumes the same.
- In 0.8.0, `SetPlcConnection` takes no `db_number`. The statements `INSERT`, `UPDATE_KEEP`, `UPDATE_SET`, and `UPDATE_CLEAR` of `crates/recipe-db/src/transaction/plc_connection.rs` copy `machine.db_number` into the record.
- In 0.8.0, `crates/recipe-db/src/transaction/machines.rs` line 56 inserts a machine with `db_number`.
- At `v0.7.1`, `crates/recipe-db/src/transaction/machines.rs` line 55 inserts a machine without `db_number`. The `NOT VALID` check `machine_db_number_shape` (`0013` lines 43 to 47) refuses that row for a live S7 machine.
- At `v0.7.1`, `reliability.md` §1.1 (line 31) maps SQLSTATE `23514` without a trigger code to `E-DB-003`. It maps SQLSTATE `55P03`, a lock wait that hit `lock_timeout`, to `StorageError::Deadlock`. That variant has the code `E-DB-001`.
- At `v0.7.1`, `crates/recipe-db/src/database.rs` line 31 sets `lock_timeout` on each new connection from the setting `db.lock_timeout`.
- At `v0.7.1`, `crates/recipe-db/src/operations/readiness.rs` lines 95 to 97 accept a migration row that the binary does not know (ADR-0059 (h), (i)). The readiness probe of 0.7.1 therefore accepts the schema of 0.8.0.
- At `v0.7.1`, no table `plc_connection` exists, so release 0.7.1 never writes a PLC connection record. The trigger of ADR-0157 fires only on a write of `plc_connection`.
- `deploy/compose.yaml` line 14 sets the project name `recipe-editor` at `v0.7.1` and in 0.8.0. From this fact the project infers that a stack of 0.8.0 on the host of 0.7.1 replaces the containers of 0.7.1.
- `deploy/compose.yaml` lines 432 to 443 declare the named volumes `backups` and `walarchive` as local bind volumes with `driver_opts`. The option `device` takes `RECIPE_BACKUP_DIR` and `RECIPE_WALARCHIVE_DIR`.
- `docker compose down` without `-v` keeps these volumes. They still bind to the directories of 0.7.1. No document gives the steps that bind them to new directories.
- The seed of the upgrade job of CI writes a `plc_connection` row (`deploy/ci/upgrade_smoke.py` line 61). From this fact the project infers that the job cannot seed the schema of 0.7.1.
- No production installation exists (owner, 2026-11-19).

`sqlx` applies each migration file once to a database. A database of 0.8.0 or later therefore never runs `0013` again. From these facts the project infers that the stop check of `0013` can find a machine only on a database of a release before 0.8.0. On such a database, `plc_connection` is empty.

## Decision drivers

- No machine gets an invented data block number. A wrong number sends a recipe to the wrong data block (ADR-0156).
- The owner decision of 2026-11-19 changes no code, no test, and no migration. Each migration file keeps its text.
- The operator gets one supported path to 0.8.0 for each database of 0.7.1.
- ADR-0156 and ADR-0157 keep their bodies.

## Considered options

### Option A: In-place upgrade only without a live S7 machine, else a new installation

- Pros: no change of code, tests, or migrations. On such a database, the stop check of `0013` finds no machine.
- Cons: a database of 0.7.1 with a live S7 machine has no in-place upgrade.

### Option B: A bridge release 0.7.2 with `SetPlcConnection`

- Pros: every database of 0.7.1 gets an in-place upgrade.
- Cons: a new release with a new command and a new migration. The owner decision changes no code. No production installation needs the bridge.

### Option C: Archive each live S7 machine with 0.7.1 before the upgrade

- Pros: `0013` passes an archived machine.
- Cons: no command restores or changes an archived machine (ADR-0156 line 14). The plant loses each machine for new work.

## Decision

The project uses **Option A**. The owner decided the items below on 2026-11-19. They are the rules.

- (a) Release 0.7.1 is the only supported start release of the upgrade to 0.8.0. An older installation first upgrades to 0.7.1. The project supports the in-place upgrade from 0.7.1 to 0.8.0 only when the database holds no live S7 machine. Then the stop check of `0013` finds no machine. The other stops of the upgrade still apply, for example the stop of `0011` (ADR-0143). Step 2 of the runbook section "Upgrade" lists the other checks before step 3.
- (b) The operator runs a query that reads only `machine` before each other step of the runbook section "Upgrade", and again before its step 3. The query must return 0 rows. Until `migrate` ends, no user creates a live S7 machine with 0.7.1.
- (c) When the database holds a live S7 machine, the operator stops the upgrade and changes nothing on the host of 0.7.1. Release 0.7.1 keeps running. The operator makes a new installation of 0.8.0 on another host only. The project does not support the host of 0.7.1 for it.

  The runbook sections "Host preparation" and "First start, first administrator, and image transfer" give the steps. Step 4 of the steps for 0.7.1 in the runbook section "Check the live S7 machines for a PLC connection record" gives this rule. This ADR defines no transfer of data from the database of 0.7.1.
- (c1) The owner decided the host rule of (c) on 2026-11-19. Release 0.7.1 and release 0.8.0 use the same project name and the same volumes `backups` and `walarchive`. New values of `RECIPE_BACKUP_DIR` and `RECIPE_WALARCHIVE_DIR` therefore do not protect the backup sets and the WAL archive of 0.7.1.
- (c2) The owner decided on 2026-11-19 to keep the path of (c) short. The project gives no step on the host of 0.7.1 for this path. The runbook has no decommission steps, no final backup set, and no off-site copy for it.
- (d) The texts on a `SetPlcConnection` of the previous release assume a release with the PLC connection record of ADR-0154. That release does not write the column `machine.db_number` of ADR-0156. The `SetPlcConnection` of that release saves the number in `plc_connection` only. No release has that state. Release 0.7.1 has neither part, and release 0.8.0 has both.
- (e) `SetPlcConnection` of 0.8.0 copies `machine.db_number` into the record, so the trigger `plc_connection_sync_db_number` changes no row on its writes. The trigger and the texts of (d) therefore cover only a direct write of `plc_connection.db_number` and a client that no release shipped. The texts of (d) are ADR-0157 lines 11 to 17, its items (d) and (e), and the sentences of its item (g) on `SetPlcConnection`. They also include `data-model.md` §3.18 and §4.13 and the requirement "Copy of the data block number in the connection record" of `plant-model`.
- (f) Migration `0013` keeps its message. By the inference of the Context, the message can appear only on a database of a release before 0.8.0. No such release can save a PLC connection record. Under (a) and (b), the operator does not see the message.
- (g) The migrations `0011` to `0016` do not follow the expand pattern as one release. After a rollback, the project does not support 0.7.1 on the schema of 0.8.0. A rollback from 0.8.0 to 0.7.1 restores the `_premigrate` set into a new volume. Use the subsection "Roll back with the _premigrate set" of the runbook section "Rollback" (`reliability.md` §7.10 step 6). The restore loses every write after the dump.

  The limit of ADR-0156 line 70 applies to 0.7.1 only on the schema of 0.8.0. There the check `machine_db_number_shape` refuses the `INSERT` of 0.7.1 with `E-DB-003`. `CreateMachine` of 0.8.0 inserts `db_number`, so line 70 does not describe 0.8.0.
- (g1) `sqlx` runs each migration file in its own transaction (ADR-0156). When `0013` stops on a database of 0.7.1, `0011` and `0012` stay committed. Then the operator does all four parts of step 12 of the runbook section "Upgrade". Its part 4 restores the `_premigrate` set of step 8 of the same upgrade with the subsection "Roll back with the _premigrate set". That restore loses every write after the dump.

  The `app` of 0.7.1 keeps running on the partly migrated schema until step 1 of that subsection deletes it. The restore discards every write that the `app` of 0.7.1 accepts in that time. After a failed check, `app` keeps running, as `reliability.md` 7.10 step 6 and the runbook section "Failed rollback restore after a failed upgrade" say.
- (h) `0013` holds its lock on `machine` until the file ends (ADR-0157 (i)). A statement of 0.7.1 on `machine` waits for that lock. When the wait is longer than `db.lock_timeout`, the statement fails with `E-DB-001`. Its command then writes nothing. A `CreateMachine` of 0.7.1 that commits a live S7 machine before `0013` takes the lock makes the stop check find that machine.
- (i) Release 0.7.1 never writes `plc_connection`. During the upgrade, the trigger `plc_connection_sync_db_number` therefore never fires on a write of 0.7.1.
- (j) After the `_premigrate` restore of (g), the database holds no table `plc_connection` and no column `machine.db_number`. The restore loses the PLC connection records and the data block numbers that 0.8.0 saved, with the other writes after the dump.
- (k) ADR-0157 line 74 has no case under (a). No live S7 machine exists when `0013` commits, and the check refuses an `INSERT` of 0.7.1 after the commit.
- (l) This ADR states no other limit of a rollback from 0.8.0 to 0.7.1.
- (m) In `docs/architecture/`, only `data-model.md` §3.18 and §4.13 change. Each gets one sentence that links to (d) and (e). ADR-0156 and ADR-0157 get only a note "Amended by ADR-0197" on their Status lines. The requirement of `plant-model` in (e) still describes the client of (d). A later OpenSpec change fixes it. Until then, the release notes list it as a known gap.
- (n) The fix rule of the upgrade checks covers only a live draft and an `Approved` version of a live recipe on a live machine. The block "Fix rule of the upgrade checks" of the runbook section "Check the stored data against the platform ranges" gives the rule.

## Consequences

### Positive

- The operator gets a check that runs on a database of 0.7.1.
- No code, test, or migration changes.
- `data-model.md` §3.18 and §4.13 name the behavior that 0.8.0 ships.
- The upgrade to 0.8.0 has one start release.

### Negative

- A database of 0.7.1 with a live S7 machine has no in-place upgrade. The new installation does not hold the data of that database.
- The new installation of (c) needs a second host. The project gives no step for the host of 0.7.1 on this path (item (c2)).
- An installation before 0.7.1 first upgrades to 0.7.1 and then to 0.8.0.
- The message of `0013` names an action that no supported path offers.
- A rollback from 0.8.0 to 0.7.1 loses the writes after the `_premigrate` dump. These writes include each PLC connection record and each data block number.
- Until a later OpenSpec change, the requirement of `plant-model` in (e) describes a client that no release shipped.

### Risks and mitigations

- Risk: a user of 0.7.1 creates a live S7 machine after the query of (b). Mitigation: `0013` locks `machine` first and stops with the machine code (ADR-0157 (i)). The operator then does step 12 of the runbook section "Upgrade" with the `_premigrate` restore. Item (g1) gives the state of the database.
- Risk: an operator makes the new installation of (c) on the host of 0.7.1. Then 0.8.0 replaces the containers of 0.7.1 and can write into its backup and WAL directories. Mitigation: (c) and the runbook permit only another host. The operator does no step on the host of 0.7.1, so that host keeps its volume, backup sets, WAL archive, and secret files.
- Risk: CI does not run the upgrade from the schema of 0.7.1. Mitigation: none. The owner decision of 2026-11-19 changes no test.

## Related

- ADR-0059 (readiness on a newer schema), ADR-0083 (forward fix or `_premigrate` restore), ADR-0143 (stop of `0011`)
- ADR-0154 (PLC connection settings), ADR-0156 (data block number), ADR-0157 (sync trigger, lock of `0013`)
- `data-model.md` §3.18 and §4.13, `reliability.md` §7.10
- `docs/ops/runbook.md`, sections "Check the live S7 machines for a PLC connection record", "Upgrade", and "Rollback"
- `openspec/specs/plant-model/spec.md`, requirement "Copy of the data block number in the connection record"
- `overview.md` §7: R31, R34. §9: A48
