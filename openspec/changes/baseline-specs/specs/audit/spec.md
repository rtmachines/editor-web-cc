# audit

## Purpose

The audit capability records every business change in the audit log with its actor and state. It links the entries in a hash chain, checks the chain every night, and records security events for failed access and chain breaks.

## ADDED Requirements

### Requirement: Audit entry in the change transaction
The system SHALL write at least one audit entry in the same transaction as the change of every command. The system SHALL NOT commit a change without its audit entry (R20, I13).

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

### Requirement: Exceptions to the audit entry rule
The system SHALL write no audit entry for an `ApproveVersion` that returns `AlreadyApproved`. A failed or throttled `Login` SHALL write a security event and no audit entry (R20, A33, I13).

#### Scenario: Login without a matching account
- **GIVEN** no user with the login `jdoe`
- **WHEN** a user runs `Login` with the login `jdoe`
- **THEN** the system writes a security event of the kind `login_failed`
- **AND** the system writes no audit entry

### Requirement: Content of an audit entry
The system SHALL write each audit entry with its actor, action, aggregate, changed entity, and request id. The system SHALL also write the before state, the after state, the client address, and the session reference (R20, I13).

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
The system SHALL store the full row of the changed entity, without `password_hash`, for every other audit action. A create SHALL store the row in `after_state`. A delete SHALL store the row in `before_state` (R20).

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
- **GIVEN** the user `jkowalski` with a live session
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
- **THEN** the system writes a security event of the kind `forbidden`
- **AND** the security event stays in the database after the rejection

### Requirement: Kinds of security events
The system SHALL write security events only of the kinds `login_failed`, `login_rejected_inactive`, `login_throttled`, `csrf_rejected`, `forbidden`, `unlock`, and `audit_chain_broken` (A33).

#### Scenario: Throttled login
- **GIVEN** the login `jkowalski` inside its login delay
- **WHEN** a user runs `Login` with the login `jkowalski`
- **THEN** the system writes a security event of the kind `login_throttled`
- **AND** the security event holds the `login_hmac` of `jkowalski`

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
The system SHALL NOT allow the application role `recipe_app` to change or delete a security event. The only exception is the increment of `count` of an aggregated row (A33).

#### Scenario: Change of a security event
- **GIVEN** a security event of the kind `forbidden`
- **WHEN** an `UPDATE` statement as `recipe_app` changes the `detail` of that event
- **THEN** the database rejects the statement

#### Scenario: Increment of count
- **GIVEN** an aggregated `login_failed` row
- **WHEN** an `UPDATE` statement as `recipe_app` changes `count` of that row
- **THEN** the database accepts the statement

### Requirement: Purge of security events
The job `cleanup` SHALL delete security events older than 400 days through `security_event_purge()`. Each call SHALL delete at most 5 000 rows. The system SHALL keep audit entries forever (R20, A33).

#### Scenario: Old security event and old audit entry
- **GIVEN** a security event older than 400 days and an audit entry of the same age
- **WHEN** the job `cleanup` runs
- **THEN** the job deletes the security event through `security_event_purge()`
- **AND** the audit entry stays in the audit log
