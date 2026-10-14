## ADDED Requirements

### Requirement: No row values in the log of a storage error
The log line of a storage error SHALL hold the error code and the messages of the error chain. It SHALL NOT hold the row values of a database detail. The database server SHALL log the errors of the application role without their detail (R35, R37).

#### Scenario: Not-null failure on a user row
- **GIVEN** a write of an `app_user` row that fails on a `NOT NULL` column
- **WHEN** the error reaches the HTTP mapping
- **THEN** the event log holds the code of the error
- **AND** the event log and the server log of `db` hold no `$argon2id$`

## MODIFIED Requirements

### Requirement: First administrator
`recipe-server create-admin` SHALL read the login from `--login` and the password from stdin or from the file of `--password-file`. It SHALL connect as the role of `db.app_user`. It SHALL do nothing when an active Administrator exists. It SHALL check the password policy, set `must_change_password`, and write `UserCreated` with an empty actor (R21, R34, A23).

#### Scenario: First run of create-admin
- **GIVEN** a database with no active Administrator
- **WHEN** the operator runs `recipe-server create-admin` with the login `admin` and a password on stdin
- **THEN** the subcommand creates the Administrator `admin` with `must_change_password` set to true
- **AND** the audit log holds the action `UserCreated` with an empty actor

#### Scenario: Second run of create-admin
- **GIVEN** an active Administrator
- **WHEN** the operator runs `recipe-server create-admin` with a new login
- **THEN** the subcommand does nothing
- **AND** the subcommand writes one `INFO` line and exits with code 0

#### Scenario: Password outside the policy
- **GIVEN** a database with no active Administrator
- **WHEN** the operator runs `recipe-server create-admin` with the login `admin` and a password of 11 characters on stdin
- **THEN** the subcommand creates no user
- **AND** the subcommand writes one `ERROR` line with `E-IDN-006` and exits with code 1

#### Scenario: Password from a terminal
- **GIVEN** a database with no active Administrator
- **WHEN** the operator runs `recipe-server create-admin` with the login `admin` and stdin connected to a terminal
- **THEN** the subcommand reads no password, creates no user, and exits with code 1

#### Scenario: Password file with two lines
- **GIVEN** a password file with two lines
- **WHEN** the operator runs `recipe-server create-admin` with the login `admin` and `--password-file` set to that file
- **THEN** the subcommand creates no user and exits with code 1
- **AND** the output holds no line of the file
