# Spec Delta: audit

## ADDED Requirements

### Requirement: Owner of the audit objects
The role `audit_owner` SHALL own `audit_log`, `audit_chain_head`, `security_event`, and their functions. The role `recipe_migrator` SHALL hold no privilege on these objects. A migration file SHALL NOT run `DROP`, `ALTER`, or `TRUNCATE` on an audit object outside the five steps of `data-model.md` section 5.1 (R20, I13).

#### Scenario: Migrator writes an audit row
- **GIVEN** the database after `0003_grants.sql`
- **WHEN** `recipe_migrator` runs an `INSERT` into `audit_log` without `SET ROLE`
- **THEN** the database rejects the statement with SQLSTATE `42501`

#### Scenario: Migration file drops an audit table
- **GIVEN** a migration file with `DROP TABLE audit_chain_head` outside the five steps
- **WHEN** the test suite checks the migration files
- **THEN** the check rejects the file

#### Scenario: Application role writes the chain head
- **GIVEN** the database after `0003_grants.sql`
- **WHEN** `recipe_app` runs an `UPDATE` of `audit_chain_head`
- **THEN** the database rejects the statement with SQLSTATE `42501`
- **AND** the chain head does not change

### Requirement: Concurrent audit inserts
Audit inserts of concurrent transactions SHALL get consecutive `chain_seq` values in commit order. A second writer SHALL wait for the commit or the rollback of the first writer (R20, I13).

#### Scenario: Two commands at the same time
- **GIVEN** the audit chain head with the last `chain_seq` N
- **WHEN** two ProcessEngineers run `RenameMachine` on two machines at the same time
- **THEN** the two audit entries have the `chain_seq` values N plus 1 and N plus 2
- **AND** the recomputed hash of each entry equals its stored `entry_hash`
