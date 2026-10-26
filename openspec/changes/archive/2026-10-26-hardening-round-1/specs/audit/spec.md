## MODIFIED Requirements

### Requirement: Append-only audit log for recipe_app
The system SHALL NOT allow the application role `recipe_app` to change or delete an audit entry. The role SHALL hold only the `SELECT` privilege on `audit_log` and the `INSERT` privilege on the columns that the application writes. The database SHALL set `id`, `occurred_at`, `chain_seq`, `prev_hash`, and `entry_hash` of a new audit entry (R20, I13).

#### Scenario: Change of an audit entry
- **GIVEN** the audit entry with the action `MachineRenamed`
- **WHEN** an `UPDATE` or `DELETE` statement as `recipe_app` changes that entry
- **THEN** the database rejects the statement
- **AND** the audit entry does not change

#### Scenario: Insert with a time in the past
- **GIVEN** the application role `recipe_app`
- **WHEN** an `INSERT` statement as `recipe_app` adds an audit entry with `occurred_at` set to 2025-01-01
- **THEN** the database rejects the statement with SQLSTATE `42501`
- **AND** the audit log holds no new entry

#### Scenario: Insert with a chosen id
- **GIVEN** the application role `recipe_app`
- **WHEN** an `INSERT` statement as `recipe_app` with `OVERRIDING SYSTEM VALUE` adds an audit entry with the `id` 1
- **THEN** the database rejects the statement with SQLSTATE `42501`
- **AND** the audit log holds no new entry

#### Scenario: Insert with a forged chain column
- **GIVEN** the audit chain head with the last `chain_seq` N
- **WHEN** an `INSERT` statement as `recipe_app` adds an audit entry with `chain_seq` set to 99
- **THEN** the database rejects the statement with SQLSTATE `42501`
- **AND** the audit log holds no new entry
- **AND** the chain head keeps the `chain_seq` N

#### Scenario: Normal insert of the application
- **GIVEN** the application role `recipe_app`
- **WHEN** a ProcessEngineer runs `RenameMachine` on `M12`
- **THEN** the database sets `id`, `occurred_at`, and the three chain columns of the new audit entry
- **AND** the command commits

## ADDED Requirements

### Requirement: Drop guard after a schema rename
The database SHALL refuse the drop of an audit object by every role except `audit_owner`. The guard SHALL hold also after a rename of the schema that holds the object (R20, I13).

#### Scenario: Migrator renames the schema and drops it
- **GIVEN** the database after `01-roles.sh` and every migration
- **WHEN** `recipe_migrator` renames the schema `public` and drops the renamed schema with `CASCADE` in one transaction
- **THEN** the database rejects the statement with SQLSTATE `42501`
- **AND** the table `audit_log` stays in the schema `public`
