## ADDED Requirements

### Requirement: Role ExportClient in the user form
The user form SHALL offer the role ExportClient after the three roles of persons. The label SHALL come from the bundles `pl` and `en` (R10, A45).

#### Scenario: Administrator opens the user form
- **GIVEN** an Administrator with the UI language `en`
- **WHEN** the Administrator opens `GET /users/new`
- **THEN** the role list holds Administrator, ProcessEngineer, Operator, and ExportClient in this order
- **AND** each option has a label from the bundle `en`

### Requirement: Password notice of a technical account
The page of a generated password of an `ExportClient` account SHALL tell the Administrator that the password is the permanent credential of the client. The page SHALL NOT tell the Administrator that the user changes the password at the first login (R39, A45).

#### Scenario: New technical account
- **GIVEN** an Administrator with the UI language `pl`
- **WHEN** the Administrator runs `CreateUser` with the login `plc-m12` and the role `ExportClient`
- **THEN** the page shows the password once with the notice of a technical account from the bundle `pl`
- **AND** the page shows no notice of a password change at the first login

#### Scenario: Rotation of the password
- **GIVEN** the `ExportClient` account `plc-m12`
- **WHEN** the Administrator runs `ResetPassword` on `plc-m12`
- **THEN** the fragment shows the new password once with the notice of a technical account

### Requirement: Role form of a technical account
The set-role form SHALL NOT offer the role ExportClient. The user page of an `ExportClient` account SHALL show no set-role form (R28, A45).

#### Scenario: Role form of a person
- **GIVEN** an Administrator and the active Operator `mhoffman`
- **WHEN** the Administrator opens `GET /users/mhoffman`
- **THEN** the set-role form offers Administrator, ProcessEngineer, and Operator
- **AND** the form offers no option ExportClient

#### Scenario: Page of a technical account
- **GIVEN** an Administrator and the `ExportClient` account `plc-m12`
- **WHEN** the Administrator opens `GET /users/plc-m12`
- **THEN** the page shows no set-role form
