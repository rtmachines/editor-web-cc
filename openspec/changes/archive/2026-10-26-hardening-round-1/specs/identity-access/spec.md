## ADDED Requirements

### Requirement: Serialized administrator changes
The system SHALL run `SetUserRole` and `DeactivateUser` in one READ COMMITTED transaction without a retry. Each command SHALL take the guard lock of the last Administrator before it reads a user row. Two such commands SHALL therefore run one after the other. A lock wait over `db.lock_timeout` SHALL return `E-DB-001` and SHALL change nothing (R29, R31, A9).

#### Scenario: Two parallel deactivations with three Administrators
- **GIVEN** the active Administrators `akowal`, `bnowak`, and `cwisniewski`
- **AND** `akowal` runs `DeactivateUser` on `bnowak` and holds the guard lock
- **WHEN** `cwisniewski` runs `DeactivateUser` on `akowal` at the same time
- **THEN** the command of `cwisniewski` waits for the guard lock
- **AND** after the commit of the first command, the second command commits
- **AND** the audit log holds two entries `UserDeactivated`, one after the other
- **AND** `cwisniewski` stays the one active Administrator

#### Scenario: Lock wait over the limit
- **GIVEN** a transaction that holds the guard lock of the last Administrator longer than `db.lock_timeout`
- **WHEN** an Administrator runs `SetUserRole` on `mhoffman` with the role `Operator`
- **THEN** the command returns `E-DB-001`
- **AND** the role of `mhoffman` does not change
- **AND** the audit log holds no new entry

### Requirement: Administrator change while the target sends requests
`SetUserRole` and `DeactivateUser` SHALL succeed while the target user sends requests. A session touch of the target SHALL NOT cause `E-DB-001`. After the commit, the target SHALL have no session (R28, R29, A9).

#### Scenario: Session touch during a deactivation
- **GIVEN** the user `mhoffman` with an open session
- **AND** `DeactivateUser` on `mhoffman` holds the user row and does not hold the session rows yet
- **WHEN** a request of `mhoffman` touches its session and commits
- **THEN** `DeactivateUser` commits in one run without `E-DB-001`
- **AND** `mhoffman` has no session
- **AND** the next request of `mhoffman` gets `E-WEB-004`

#### Scenario: Session touch during a demotion
- **GIVEN** the user `mhoffman` with the role `ProcessEngineer` and an open session
- **AND** `SetUserRole` on `mhoffman` with the role `Operator` holds the session rows of `mhoffman` and does not commit yet
- **WHEN** a request of `mhoffman` touches its session
- **THEN** the touch waits for the commit of `SetUserRole`
- **AND** the touch finds no session, and the request gets `E-WEB-004`
- **AND** the audit log holds one entry `UserRoleSet` for `mhoffman`

## MODIFIED Requirements

### Requirement: Concurrent demotion of administrators
When two Administrators demote or deactivate each other at the same time and no third active Administrator exists, the system SHALL complete exactly one command. The second command SHALL wait for the guard lock of the last Administrator. It SHALL then return `E-IDN-002` (A9, R31).

#### Scenario: Two administrators demote each other
- **GIVEN** the active Administrators `akowal` and `bnowak` and no other active Administrator
- **WHEN** `akowal` runs `SetUserRole` on `bnowak` with the role `Operator`
- **AND** `bnowak` runs `SetUserRole` on `akowal` with the role `Operator` at the same time
- **THEN** the second command waits for the guard lock until the first command commits
- **AND** exactly one of the two commands completes
- **AND** the other command returns `E-IDN-002`
- **AND** one active Administrator remains

## REMOVED Requirements

### Requirement: Serializable isolation for role and activation changes
**Reason**: ADR-0096 replaces SERIALIZABLE and the retry with READ COMMITTED and the guard lock of class 4. A target that sends requests can no longer cause two serialization failures.
**Migration**: The requirements "Serialized administrator changes", "Administrator change while the target sends requests", and "Concurrent demotion of administrators" state the new behavior.
