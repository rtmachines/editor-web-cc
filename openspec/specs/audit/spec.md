# audit Specification

## Purpose
The audit capability records every business change in the audit log with its actor and state. It links the entries in a hash chain, checks the chain every night, and records security events for failed access and chain breaks.

## Requirements

### Requirement: Audit entry in the change transaction
The system SHALL write at least one audit entry in the same transaction as the change of every command. `domain-model.md` section 5 names the two exceptions. The system SHALL NOT commit a change without its audit entry (R20, I13).

#### Scenario: Rename of a live machine
- **GIVEN** the live machine `M12`
- **WHEN** a ProcessEngineer runs `RenameMachine` on `M12` with a new name
- **THEN** the system commits the new name and one audit entry in one transaction
- **AND** the audit entry holds the action `MachineRenamed`

### Requirement: Failed audit write
The system SHALL reject a command with `E-AUD-001` when the audit entry cannot be written, and SHALL keep the change out of the database (R29, I13).

#### Scenario: Audit write fails
- **GIVEN** the live machine `M12`
- **WHEN** a ProcessEngineer runs `RenameMachine` on `M12` and the audit entry cannot be written
- **THEN** the system rejects the command with `E-AUD-001`
- **AND** the name of `M12` does not change

### Requirement: Content of an audit entry
The system SHALL write each audit entry with its action, aggregate, changed entity, request id, and a before state or an after state. The actor SHALL be empty only for `create-admin`, the startup settings comparison, and a data migration that changes catalog rows. The system SHALL write the client address and the session reference when the request has them (R20, I13).

#### Scenario: Content of a rename entry
- **GIVEN** the live machine `M12`
- **WHEN** a ProcessEngineer runs `RenameMachine` on `M12`
- **THEN** the audit entry holds the action `MachineRenamed` and the entity id of `M12`
- **AND** the session reference holds the first 16 hex characters of the session id

### Requirement: Diff state of five actions
The system SHALL store in `before_state` and `after_state` only the changed fields of the actions `ParameterDefinitionUpdated`, `ParameterValueSet`, `LimitsReplaced`, `TranslationUpserted`, and `TypeRevisionRebound` (R20).

#### Scenario: Value of a parameter
- **GIVEN** the draft of the recipe `R-100` with a parameter
- **WHEN** a ProcessEngineer runs `SetParameterValue` on that parameter with a new value
- **THEN** the audit entry holds the action `ParameterValueSet`
- **AND** the `before_state` and the `after_state` hold only the changed fields

### Requirement: Full row for other actions
The system SHALL store the full row of the changed entity, without `password_hash`, for every other audit action. A create SHALL store the row in `after_state`. A delete SHALL store the row in `before_state`. An update SHALL store the row in both (R20).

#### Scenario: Parameter added to a draft
- **GIVEN** the draft of the recipe `R-100`
- **WHEN** a ProcessEngineer runs `AddParameter` on the draft
- **THEN** the audit entry holds the action `ParameterAdded`
- **AND** the `after_state` holds the full row of the parameter
- **AND** the `before_state` holds no value

#### Scenario: New app user
- **GIVEN** no user with the login of the new user
- **WHEN** an Administrator runs `CreateUser` with a new login
- **THEN** the audit entry holds the action `UserCreated`
- **AND** the `after_state` holds the full row of the user without `password_hash`

### Requirement: State objects of four actions
The system SHALL store the `session_ref` for `UserLoggedIn` and `UserLoggedOut`. `VersionExported` SHALL store the version id, the schema version, and `content_sha256`. `VersionApproved` SHALL add `allow_outdated` to the full row (R20).

#### Scenario: Logout
- **GIVEN** the user `jkowalski` with an open session
- **WHEN** `jkowalski` runs `Logout`
- **THEN** the audit entry holds the action `UserLoggedOut` with the entity type `app_user`
- **AND** the `before_state` holds the `session_ref` of the deleted session

### Requirement: Entity type of limit and translation changes
The system SHALL write the entity type `parameter` and the id of the parameter for a change of a limit or a translation (R20).

#### Scenario: Limits of a parameter replaced
- **GIVEN** the parameter `P` of the recipe `R-100` in a live draft
- **WHEN** a ProcessEngineer runs `ReplaceLimits` on `P`
- **THEN** the audit entry holds the entity type `parameter` and the entity id of `P`
- **AND** the aggregate type is `recipe` and the aggregate id is the id of `R-100`

### Requirement: Aggregate type of an audit entry
The system SHALL write the aggregate type `machine` for a change of a machine, a unit, or a PLC type. The system SHALL write the aggregate type `recipe` for a change of a recipe, a version, or a parameter (R20).

#### Scenario: Unit added to a machine
- **GIVEN** the live machine `M12` with the unit `U1`
- **WHEN** a ProcessEngineer runs `AddUnit` on `M12` with the parent path `U1` and the segment `2`
- **THEN** the audit entry holds the aggregate type `machine` and the aggregate id of `M12`

### Requirement: Append-only audit log for recipe_app
The system SHALL NOT allow the application role `recipe_app` to change or delete an audit entry. The role SHALL hold only the `INSERT` and `SELECT` privileges on `audit_log` (R20, I13).

#### Scenario: Change of an audit entry
- **GIVEN** the audit entry with the action `MachineRenamed`
- **WHEN** an `UPDATE` or `DELETE` statement as `recipe_app` changes that entry
- **THEN** the database rejects the statement
- **AND** the audit entry does not change

### Requirement: Hash chain of audit entries
The system SHALL give each audit entry the next `chain_seq` and an `entry_hash` computed over the previous hash and the entry content. A rolled back command SHALL leave no gap in `chain_seq` (R20, I13).

#### Scenario: Next entry in the chain
- **GIVEN** the audit chain head with the last `chain_seq` N and the entry hash H
- **WHEN** a ProcessEngineer runs `RenameMachine` on `M12`
- **THEN** the new audit entry has the `chain_seq` N plus 1
- **AND** the `prev_hash` of the new audit entry equals H

#### Scenario: Rolled back command
- **GIVEN** the audit chain head with the last `chain_seq` N
- **WHEN** a command rolls back after its audit insert
- **THEN** the next committed audit entry has the `chain_seq` N plus 1

### Requirement: Nightly audit chain check
The system SHALL run the job `audit_chain_check` every night at 03:00 UTC. The job SHALL recompute the hash of every audit entry up to the head that it reads at the start (R20, I13).

#### Scenario: Chain without a break
- **GIVEN** an audit chain without a break
- **WHEN** the job `audit_chain_check` runs
- **THEN** the job records `audit.last_check_seq` and `audit.last_check_at` in `app_setting`
- **AND** the job writes no audit entry

### Requirement: Chain break
The system SHALL write `E-AUD-002` to the event log and a security event of the kind `audit_chain_broken` when the job `audit_chain_check` finds a break. The job SHALL stop at the break (A33, I13).

#### Scenario: Entry hash does not match
- **GIVEN** an audit entry whose stored hash does not match its content
- **WHEN** the job `audit_chain_check` runs
- **THEN** the job writes `E-AUD-002` to the event log
- **AND** the system writes the security event `audit_chain_broken` without a client address
- **AND** the job stops

### Requirement: Security event transaction
The system SHALL write each security event in its own short transaction. The system SHALL keep the security event when the request fails (A33).

#### Scenario: Forbidden command
- **GIVEN** the live machine `M12`
- **WHEN** an Operator runs `RenameMachine` on `M12` and the request fails with `Forbidden`
- **THEN** the security event of the kind `forbidden` stays in the database after the rejection

### Requirement: Kinds of security events
The system SHALL write security events only of the kinds `login_failed`, `login_rejected_inactive`, `login_throttled`, `csrf_rejected`, `forbidden`, `unlock`, and `audit_chain_broken` (A33).

#### Scenario: Unknown kind
- **GIVEN** the constraint `security_event_kind_value` on the table `security_event`
- **WHEN** an `INSERT` statement as `recipe_app` adds a security event of the kind `login_ok`
- **THEN** the database rejects the statement with the constraint `security_event_kind_value`

### Requirement: Aggregated security events
The system SHALL aggregate the events of the kinds `login_failed`, `login_throttled`, and `csrf_rejected` per client address, kind, and minute. The system SHALL increment `count` for each later event of the same minute (A33).

#### Scenario: Second failed login in one minute
- **GIVEN** a `login_failed` row for a client address and the current minute
- **WHEN** a user runs `Login` with a wrong password from the same client address in the same minute
- **THEN** the system increments `count` of that row
- **AND** the system inserts no new row

### Requirement: HMAC of the login in a security event
The system SHALL store the login of a security event only as an HMAC in `login_hmac`. The system SHALL NOT store the plain login text in a security event (A33).

#### Scenario: Login of an unknown account
- **GIVEN** no user with the login `jdoe`
- **WHEN** a user runs `Login` with the login `jdoe`
- **THEN** the security event holds the `login_hmac` of the typed login
- **AND** the security event holds no plain login text

### Requirement: Append-only security events
The system SHALL NOT allow the application role `recipe_app` to change or delete a security event. The only exception is the increment of `count` of an aggregated row by exactly 1. Any other update SHALL fail with `E-AUD-003` (A33).

#### Scenario: Change of a security event
- **GIVEN** a security event of the kind `forbidden`
- **WHEN** an `UPDATE` statement as `recipe_app` changes the `detail` of that event
- **THEN** the database rejects the statement

#### Scenario: Increment of count
- **GIVEN** an aggregated `login_failed` row
- **WHEN** an `UPDATE` statement as `recipe_app` adds 1 to `count` of that row
- **THEN** the database accepts the statement

#### Scenario: Decrease of count
- **GIVEN** an aggregated `login_failed` row with `count` 5
- **WHEN** an `UPDATE` statement as `recipe_app` sets `count` of that row to 1
- **THEN** the database rejects the statement with `E-AUD-003`
- **AND** `count` of that row stays 5

#### Scenario: Increment by more than 1
- **GIVEN** an aggregated `csrf_rejected` row with `count` 2
- **WHEN** an `UPDATE` statement as `recipe_app` sets `count` of that row to 4
- **THEN** the database rejects the statement with `E-AUD-003`

### Requirement: Deletion of old security events
The job `cleanup` SHALL delete security events older than 400 days through `security_event_purge()`. Each call SHALL delete at most 5 000 rows. The system SHALL keep audit entries forever (R20, A33).

#### Scenario: Old security event and old audit entry
- **GIVEN** a security event older than 400 days and an audit entry of the same age
- **WHEN** the job `cleanup` runs
- **THEN** the job deletes the security event through `security_event_purge()`
- **AND** the audit entry stays in the audit log

### Requirement: Full row with child rows
The full row of a parameter in the audit actions `ParameterAdded` and `ParameterRemoved` SHALL hold the parameter row, its limit rows, and its translation rows. The full row of a struct in the audit action `UdtDefined` SHALL hold the type row and its member rows. A command SHALL write no separate audit entry for these child rows (R20).

#### Scenario: Parameter added with limits
- **GIVEN** the draft version 3 of `R-100`
- **WHEN** a ProcessEngineer runs `AddParameter` on version 3 with the tag `SPEED` and the limit `min 10, max 250` on the leaf path `""`
- **THEN** the system writes one audit entry with the action `ParameterAdded`
- **AND** the `after_state` holds the row of `SPEED` and its limit row with the leaf path `""`

#### Scenario: `RemoveParameter` with a translation
- **GIVEN** the parameter `SPEED` of the draft version 3 of `R-100` with a translation for `pl`
- **WHEN** a ProcessEngineer runs `RemoveParameter` on `SPEED`
- **THEN** the system writes one audit entry with the action `ParameterRemoved`
- **AND** the `before_state` holds the row of `SPEED` and its translation row for `pl`

#### Scenario: Struct defined with two members
- **GIVEN** the machine `M12` with the platform `TwinCat3`
- **WHEN** a ProcessEngineer runs `DefineUdt` for `ST_Zone` with the members `setpoint` of type `REAL` and `label` of type `STRING(16)`
- **THEN** the audit entry `UdtDefined` holds the type row of `ST_Zone@1`
- **AND** the `after_state` holds the member rows `setpoint` and `label` in their order

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

### Requirement: Time and count of a new security event
The database SHALL set `occurred_at` and `count` of a new security event. An `INSERT` statement of `recipe_app` that names `occurred_at` or `count` SHALL fail (R20, A33).

#### Scenario: Insert with a time in the past
- **GIVEN** the application role `recipe_app`
- **WHEN** an `INSERT` statement as `recipe_app` adds a security event with `occurred_at` set to 2025-01-01
- **THEN** the database rejects the statement with SQLSTATE `42501`
- **AND** the table holds no new row

### Requirement: Full row of an unlock
The entry `LoginUnlocked` SHALL hold the full row of the user without `password_hash`, with the child array `login_attempts`. The before-state SHALL hold the newest 100 deleted attempts and the field `login_attempt_count` with the number of all deleted attempts. The after-state SHALL hold an empty array (R20, A22).

#### Scenario: Unlock a login with two failures
- **GIVEN** 2 failed attempts for `jkowalski`
- **WHEN** an Administrator runs `UnlockLogin` on `jkowalski`
- **THEN** the before-state of `LoginUnlocked` holds 2 entries in `login_attempts`
- **AND** the after-state of `LoginUnlocked` holds an empty `login_attempts`
- **AND** neither state holds `password_hash`

#### Scenario: Unlock a login with many failures
- **GIVEN** 250 failed attempts for `jkowalski` from many client addresses
- **WHEN** an Administrator runs `UnlockLogin` on `jkowalski`
- **THEN** the before-state of `LoginUnlocked` holds the newest 100 attempts
- **AND** its field `login_attempt_count` is 250

### Requirement: Subject of a login security event
A `login_failed` event SHALL hold the user when an account matches, and the `login_hmac` of the typed login when no account matches. A `login_throttled` event SHALL always hold the `login_hmac`. A `login_rejected_inactive` event SHALL hold the user (A33, A42).

#### Scenario: Wrong password of an existing user
- **GIVEN** the active user `jkowalski`
- **WHEN** a user runs `Login` with the login `jkowalski` and a wrong password
- **THEN** the security event of kind `login_failed` holds the user `jkowalski`
- **AND** the security event holds no `login_hmac`

#### Scenario: Throttled login
- **GIVEN** the login delay is active for `jkowalski` and the client address
- **WHEN** a user runs `Login` with the login `jkowalski` from that address
- **THEN** the security event of kind `login_throttled` holds the `login_hmac` of `jkowalski`

### Requirement: Audit list query
`ListAuditEntries` SHALL check its permission first. It SHALL return at most `page.audit` entries, newest first, with the id below the cursor `before`. It SHALL say whether an older entry exists. It SHALL read no state of an entry (R20, R28, ADR-0051).

#### Scenario: Cursor of the second page
- **GIVEN** 150 audit entries with the ids 1 to 150
- **WHEN** an Administrator runs `ListAuditEntries` with `before` 51
- **THEN** the result holds the entries 50 to 1, newest first
- **AND** the result says that no older entry exists

### Requirement: Scope of the audit queries
For a user with only `AuditViewRecipes`, `ListAuditEntries` SHALL return only the entries with `aggregate_type` set to `recipe`. A filter value of `entity_type` or `action` outside that scope SHALL give an empty list. `GetAuditEntry` of an entry outside that scope SHALL return `NotFound`. The audit list and the entry SHALL include the entries of a deleted recipe (A20, R28, ADR-0051).

#### Scenario: ProcessEngineer filters by the action of a machine
- **GIVEN** the entries `MachineRenamed` of `M12` and `RecipeRenamed` of `R-100`
- **WHEN** a ProcessEngineer runs `ListAuditEntries` with the action `MachineRenamed`
- **THEN** the result is empty

#### Scenario: ProcessEngineer reads an entry of a machine
- **GIVEN** the entry `MachineRenamed` of `M12`
- **WHEN** a ProcessEngineer runs `GetAuditEntry` on that entry
- **THEN** the system returns `NotFound`

#### Scenario: ProcessEngineer lists the entries of a deleted recipe
- **GIVEN** the deleted recipe `R-300` with the entry `RecipeDeleted`
- **WHEN** a ProcessEngineer runs `ListAuditEntries` with the action `RecipeDeleted`
- **THEN** the result holds the entry of `R-300`

### Requirement: Filters of the audit list
`ListAuditEntries` SHALL filter by `aggregate` (an aggregate id), `entity_type`, `entity_id` (only with `entity_type`), `actor` (a login), `action`, `from`, and `to`. `from` and `to` SHALL be dates in UTC, and both days SHALL be inside the range. An `actor` that names no user SHALL give the empty list, so the answer does not show which logins exist (R20, R38, A42, ADR-0051).

#### Scenario: Range of one day
- **GIVEN** the entries of 2026-10-15 23:59 UTC and of 2026-10-16 00:00 UTC
- **WHEN** an Administrator runs `ListAuditEntries` with `from` and `to` set to 2026-10-16
- **THEN** the result holds only the entry of 2026-10-16 00:00 UTC

#### Scenario: Entries of one parameter
- **GIVEN** the entries `ParameterValueSet` and `LimitsReplaced` of the parameter `SPEED_TABLE` and an entry of `HOLD`
- **WHEN** an Administrator runs `ListAuditEntries` with `entity_type` `parameter` and the id of `SPEED_TABLE`
- **THEN** the result holds the two entries of `SPEED_TABLE`

#### Scenario: Actor that names no user
- **GIVEN** no user with the login `ghost`
- **WHEN** a ProcessEngineer runs `ListAuditEntries` with `actor` set to `ghost`
- **THEN** the result is empty

### Requirement: Audit entry with its states
`GetAuditEntry` SHALL return the entry with `before` and `after`. The states SHALL hold no password hash, no session token, and no cookie value (R20, `reliability.md` 1.1, ADR-0051).

#### Scenario: Entry of a new user
- **GIVEN** the entry `UserCreated` of the user `jsmith`
- **WHEN** an Administrator runs `GetAuditEntry` on that entry
- **THEN** the state `after` holds the login `jsmith`
- **AND** the state holds no field `password_hash`

### Requirement: Recipe history query
`GetRecipeHistory` SHALL check `RecipeView` first. It SHALL return the entries with the `aggregate_id` of the recipe, newest first, at most `page.history` entries, without `before` and `after`. For a deleted recipe, it SHALL return the entries only to a user with `RecipeRestore` and `E-RCP-012` to every other user (R24, A14, A20, ADR-0051).

#### Scenario: History of a recipe
- **GIVEN** the recipe `R-100` with the entries `RecipeCreated`, `VersionCreated`, and `VersionApproved`
- **WHEN** an Operator runs `GetRecipeHistory` on `R-100`
- **THEN** the result holds the three entries, newest first, without states

### Requirement: Security event list query
`ListSecurityEvents` SHALL check `AuditView` first. It SHALL return at most `page.security_events` events, newest first, with the cursor `before`. It SHALL filter by `kind`, `login`, `client_ip`, `from`, and `to`. `login` SHALL match the events of the user with that login. It SHALL also match the events whose `login_hmac` equals the HMAC of the login under the current key of the key ring. The result SHALL hold no `login_hmac` (R35, A33, ADR-0051).

#### Scenario: Events of an unknown login
- **GIVEN** two `login_failed` events of the typed login `ghost` from two different client addresses, and `ghost` matches no account
- **WHEN** an Administrator runs `ListSecurityEvents` with `login` set to `ghost`
- **THEN** the result holds the two events
- **AND** no field of the result holds the `login_hmac`

#### Scenario: Event under an older key
- **GIVEN** a `login_failed` event of the typed login `ghost` with the HMAC under an older key of the key ring
- **WHEN** an Administrator runs `ListSecurityEvents` with `login` set to `ghost`
- **THEN** the result is empty

#### Scenario: Events of a known user
- **GIVEN** a `forbidden` event of the user `mhoffman`
- **WHEN** an Administrator runs `ListSecurityEvents` with `login` set to `mhoffman`
- **THEN** the result holds the event

### Requirement: Login filter of an aggregated row
An aggregated row of `ListSecurityEvents` SHALL keep the subject of its first event. The filter `login` SHALL match an aggregated row by that first event only. The filter `client_ip` SHALL find every attempt of an address (R35, A33, ADR-0051).

#### Scenario: Aggregated row of two logins
- **GIVEN** a `login_failed` event of the typed login `decoy` and then one of `target`, both from one client address in one minute
- **WHEN** an Administrator runs `ListSecurityEvents` with `login` set to `target`, and then with `client_ip` set to that address
- **THEN** the first result is empty
- **AND** the second result holds one row with the count 2
