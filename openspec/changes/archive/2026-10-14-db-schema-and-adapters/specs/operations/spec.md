# Spec Delta: operations

## ADDED Requirements

### Requirement: Roles from the database init script
The init script of the `db` service SHALL create the roles `recipe_migrator`, `recipe_app`, `recipe_backup`, `recipe_replicator`, and `audit_owner` and the database `recipe` once, on an empty volume. It SHALL read each password from a secret file. It SHALL revoke `CONNECT` and `TEMPORARY` of `PUBLIC` on `recipe`, `postgres`, and `template1`. A migration SHALL NOT create a role (R04, R34, A1).

#### Scenario: First start on an empty volume
- **GIVEN** an empty volume of the service `db` and the four password secret files
- **WHEN** the service `db` starts for the first time
- **THEN** the five roles and the database `recipe` exist
- **AND** `PUBLIC` holds no `CONNECT` and no `TEMPORARY` privilege on `recipe`

#### Scenario: Replication role
- **GIVEN** the database `recipe` after the first start
- **WHEN** `recipe_replicator` connects to `recipe` as a normal session
- **THEN** the database rejects the connection
- **AND** `recipe_replicator` holds no privilege on any table

#### Scenario: Normal session on the maintenance database
- **GIVEN** the database `postgres` after the first start
- **WHEN** `recipe_app` or `recipe_replicator` connects to `postgres` as a normal session
- **THEN** the database rejects the connection

### Requirement: Least privilege of the application role
The role `recipe_app` SHALL hold exactly the grants of `data-model.md` section 5.2. It SHALL NOT be a superuser, own a table, or switch to another role. An `UPDATE` of a column outside its grant SHALL fail with SQLSTATE `42501` before a trigger runs (R20, R29, R34).

#### Scenario: Update of a frozen version column
- **GIVEN** the approved version 2 of the recipe `R-100`
- **WHEN** `recipe_app` runs an `UPDATE` of `number`, `recipe_id`, `based_on_version_id`, or `row_version` of version 2
- **THEN** the database rejects the statement with SQLSTATE `42501`
- **AND** version 2 does not change

#### Scenario: Delete of a machine row
- **GIVEN** the live machine `M12`
- **WHEN** `recipe_app` runs a `DELETE` of the row of `M12`
- **THEN** the database rejects the statement with SQLSTATE `42501`
- **AND** the row of `M12` stays

#### Scenario: Role switch
- **GIVEN** a session of `recipe_app`
- **WHEN** the session runs `SET ROLE audit_owner`
- **THEN** the database rejects the statement

### Requirement: Trigger code of a storage error
When a database trigger rejects a write, the system SHALL return the code at the start of the trigger message. The error SHALL carry the user message of that code. When a `CHECK` constraint without a trigger code rejects a write, the system SHALL return `E-DB-003`. A deadlock SHALL return `E-DB-001` (R35, R36).

#### Scenario: Parameter write in an approved version
- **GIVEN** the approved version 2 of the recipe `R-100`
- **WHEN** a write of a parameter row of version 2 reaches the database
- **THEN** the system rejects the write with `E-RCP-031`
- **AND** the transaction of the write changes nothing

#### Scenario: Check constraint without a trigger code
- **GIVEN** the live machine `M12`
- **WHEN** a write of the machine name with 201 characters reaches the database
- **THEN** the system rejects the write with `E-DB-003`

### Requirement: Idle transaction limit
The database SHALL end a session of `app` that stays idle inside an open transaction for more than 15 seconds. The open transaction SHALL roll back (R30).

#### Scenario: Idle transaction
- **GIVEN** a connection of `app` with an open transaction
- **WHEN** the connection sends no statement for more than 15 seconds
- **THEN** the database ends the session
- **AND** the transaction writes nothing

### Requirement: Configuration of the migrator
The subcommand `recipe-server migrate` SHALL read the connection values from its configuration file and the password of `recipe_migrator` only from the file that `secrets.migrator_password_file` names. When it cannot read a value, it SHALL write one `ERROR` line and exit with code 1 before it connects (R34).

#### Scenario: Missing secret file
- **GIVEN** a configuration with `secrets.migrator_password_file` set to a path without a file
- **WHEN** the operator runs `recipe-server migrate`
- **THEN** the subcommand writes one `ERROR` line
- **AND** the subcommand exits with code 1 and applies no migration

#### Scenario: Migration of an empty database
- **GIVEN** the database `recipe` after the first start of `db` and a valid configuration
- **WHEN** the operator runs `recipe-server migrate`
- **THEN** the migrator applies `0001_schema.sql` to `0004_seed_plc_types.sql` as `recipe_migrator`
- **AND** the subcommand exits with code 0
