## ADDED Requirements

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

## MODIFIED Requirements

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
