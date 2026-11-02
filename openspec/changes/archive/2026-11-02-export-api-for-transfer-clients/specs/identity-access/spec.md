## MODIFIED Requirements

### Requirement: Three roles
The system SHALL assign each user exactly one role: Administrator, ProcessEngineer, Operator, or ExportClient. Administrator, ProcessEngineer, and Operator SHALL be the roles of persons. ExportClient SHALL be the role of a technical account (R22, A45).

#### Scenario: Role of a new user
- **GIVEN** no user `lnowak` exists
- **WHEN** an Administrator runs `CreateUser` with the login `lnowak` and the role `ProcessEngineer`
- **THEN** the system creates the user `lnowak` with the role `ProcessEngineer`

#### Scenario: Role of a technical account
- **GIVEN** no user `plc-m12` exists
- **WHEN** an Administrator runs `CreateUser` with the login `plc-m12` and the role `ExportClient`
- **THEN** the system creates the user `plc-m12` with the role `ExportClient`

### Requirement: One-time password on user creation
The system SHALL create a user without a password in the request. The server SHALL generate a one-time password and show it once. It SHALL set `must_change_password` to true for every role except `ExportClient` (R21, A45).

#### Scenario: Create a user
- **GIVEN** no user `mhoffman` exists
- **WHEN** an Administrator runs `CreateUser` with the login `mhoffman` and the role `Operator`
- **THEN** the system creates the user `mhoffman` with the role `Operator`
- **AND** the system sets `must_change_password` to true
- **AND** the audit log holds the action `UserCreated`

#### Scenario: Create a technical account
- **GIVEN** no user `plc-m12` exists
- **WHEN** an Administrator runs `CreateUser` with the login `plc-m12` and the role `ExportClient`
- **THEN** the system shows the generated password once
- **AND** the system keeps `must_change_password` of `plc-m12` false
- **AND** the audit log holds the action `UserCreated`

### Requirement: One-time password on password reset
`ResetPassword` SHALL generate a one-time password on the server and show it once. It SHALL set `must_change_password` to true for every role except `ExportClient` (R21, A9, A45).

#### Scenario: Reset the password of a user
- **GIVEN** the active user `mhoffman`
- **WHEN** an Administrator runs `ResetPassword` on `mhoffman`
- **THEN** the system shows a new one-time password once
- **AND** the system sets `must_change_password` of `mhoffman` to true
- **AND** the audit log holds the action `PasswordReset`

#### Scenario: Rotate the password of a technical account
- **GIVEN** the active `ExportClient` account `plc-m12`
- **WHEN** an Administrator runs `ResetPassword` on `plc-m12`
- **THEN** the system shows a new password once
- **AND** `must_change_password` of `plc-m12` stays false
- **AND** a Basic check with the old password of `plc-m12` gives `401 E-IDN-003`

## ADDED Requirements

### Requirement: ExportClient permissions
The role ExportClient SHALL hold the permission `VersionExport` and no other permission (R28, A45).

#### Scenario: Export by a technical account
- **GIVEN** the `ExportClient` account `plc-m12` and the approved version 3 of the recipe `R-100`
- **WHEN** `plc-m12` runs `ExportVersion` for the approved version of `R-100`
- **THEN** the use case returns the export document

#### Scenario: View query by a technical account
- **GIVEN** the `ExportClient` account `plc-m12`
- **WHEN** `plc-m12` runs `ListRecipes`
- **THEN** the use case rejects the query with `E-WEB-002`
- **AND** the system writes one `security_event` of kind `forbidden`

### Requirement: Permission of the recipe list query
`ListExportableRecipes` SHALL check the permission `VersionExport` as its first statement. Administrator, ProcessEngineer, Operator, and ExportClient SHALL hold it (R28, A4).

#### Scenario: Role without the permission
- **GIVEN** a test role without `VersionExport`
- **WHEN** the generated matrix test runs `ListExportableRecipes` with that role
- **THEN** the use case returns `E-WEB-002` before it reads data

### Requirement: No UI login for a technical account
`Login` SHALL refuse an `ExportClient` account after the full password check. The answer SHALL be the `401 E-IDN-003` answer of a wrong password (A42, A45).

#### Scenario: Login form with client credentials
- **GIVEN** the active `ExportClient` account `plc-m12` with a correct password
- **WHEN** a person sends `POST /login` with the login `plc-m12` and its password
- **THEN** the system returns `401 E-IDN-003`
- **AND** the system creates no session
- **AND** the system writes one `security_event` of kind `login_failed` with the reason `technical_account`

### Requirement: No forced password change of a technical account
The system SHALL refuse a row of `app_user` with the role `ExportClient` and `must_change_password` set to true (R21, A45).

#### Scenario: Direct update of the flag
- **GIVEN** the `ExportClient` account `plc-m12`
- **WHEN** a statement sets `must_change_password` of `plc-m12` to true
- **THEN** the database refuses the statement with a `CHECK` violation

### Requirement: Role change of a technical account
`SetUserRole` SHALL refuse a change to the role ExportClient and a change from the role ExportClient with `409 E-IDN-011`. The command SHALL change nothing (R22, A45).

#### Scenario: Person to technical account
- **GIVEN** the active Operator `mhoffman`
- **WHEN** an Administrator runs `SetUserRole` on `mhoffman` with the role `ExportClient`
- **THEN** the system rejects the command with `E-IDN-011`
- **AND** `mhoffman` keeps the role `Operator` and its sessions

#### Scenario: Technical account to person
- **GIVEN** the `ExportClient` account `plc-m12`
- **WHEN** an Administrator runs `SetUserRole` on `plc-m12` with the role `Operator`
- **THEN** the system rejects the command with `E-IDN-011`
