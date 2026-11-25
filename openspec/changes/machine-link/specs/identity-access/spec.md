## MODIFIED Requirements

### Requirement: ExportClient permissions
The role ExportClient SHALL hold the permissions `VersionExport` and `MachineLinkReport` and no other permission (R28, A45, A55).

#### Scenario: Export by a technical account
- **GIVEN** the `ExportClient` account `plc-m12` and the approved version 3 of the recipe `R-100`
- **WHEN** `plc-m12` runs `ExportVersion` for the approved version of `R-100`
- **THEN** the use case returns the export document

#### Scenario: View query by a technical account
- **GIVEN** the `ExportClient` account `plc-m12`
- **WHEN** `plc-m12` runs `ListRecipes`
- **THEN** the use case rejects the query with `E-WEB-002`
- **AND** the system writes one `security_event` of kind `forbidden`

#### Scenario: Status report by a technical account
- **GIVEN** the `ExportClient` account `plc-m12` and the live machine `M12`
- **WHEN** `plc-m12` runs `ReportMachineState` for `M12`
- **THEN** the use case stores the link state of `M12`

#### Scenario: Load request by a technical account
- **GIVEN** the `ExportClient` account `plc-m12` and the approved version 3 of the recipe `R-100`
- **WHEN** `plc-m12` runs `RequestRecipeLoad` for the version 3 of `R-100`
- **THEN** the use case rejects the command with `E-WEB-002`
- **AND** the system stores no load request

### Requirement: Operator permissions
The system SHALL give an Operator only the permissions `RecipeView`, `VersionExport`, and `RecipeLoadRequest` (R24, A4, A55).

#### Scenario: Operator exports an approved version
- **GIVEN** the approved version 3 of the recipe `R-100`
- **WHEN** an Operator runs `ExportVersion` for the approved version of `R-100`
- **THEN** the system returns the export document of version 3

#### Scenario: Operator exports a retired version
- **GIVEN** the retired version 2 of the recipe `R-100`
- **WHEN** an Operator runs `ExportVersion` for version 2 of `R-100`
- **THEN** the system rejects the command with `E-WEB-002`

#### Scenario: Operator renames a recipe
- **GIVEN** the recipe `R-100`
- **WHEN** an Operator runs `RenameRecipe` on `R-100`
- **THEN** the system rejects the command with `E-WEB-002`

#### Scenario: Operator asks for a load
- **GIVEN** the approved version 3 of the recipe `R-100` on `M12` and a fresh link state `ready_for_changeover`
- **WHEN** an Operator runs `RequestRecipeLoad` for the version 3 of `R-100`
- **THEN** the system stores one `pending` load request
- **AND** the recipe `R-100` and its versions do not change

### Requirement: Session timeouts
The system SHALL end a session after 30 minutes without a request. The system SHALL end every session 12 hours after its login. An ended session SHALL get `E-WEB-004`. A passive request SHALL NOT count as a request for the idle limit. A passive request is a `GET` of `/machines/{code}` or `/machines/by-id/{uuid}` with `HX-Request: true` and `HX-Target: machine-link-panel`. The system SHALL still check the session of a passive request (A21, A55).

#### Scenario: Idle session
- **GIVEN** the session of the Operator `mhoffman` with the last request 31 minutes ago
- **WHEN** an Operator runs `ListRecipes`
- **THEN** the system rejects the query with `E-WEB-004`

#### Scenario: Machine page with only panel polls
- **GIVEN** the session of the Operator `mhoffman` with the last request that is not passive 31 minutes ago
- **AND** a machine page that sent a passive request every 5 seconds since then
- **WHEN** the page sends the next passive request
- **THEN** the system rejects the request with `E-WEB-004`
- **AND** the answer has `HX-Redirect: /login`

#### Scenario: Passive request does not renew the session
- **GIVEN** the session of the Operator `mhoffman` with the last request 10 minutes ago
- **WHEN** the machine page sends a passive request
- **THEN** the system answers the request
- **AND** the idle limit of the session does not move

#### Scenario: Form post with the poll headers
- **GIVEN** the session of the Operator `mhoffman` with the last request 10 minutes ago and a `pending` load request of `M12`
- **WHEN** `mhoffman` sends the cancel form with `HX-Request: true` and `HX-Target: machine-link-panel`
- **THEN** the system runs the cancel as a normal request
- **AND** the idle limit of the session moves

#### Scenario: Adapter route with the poll headers
- **GIVEN** the active `ExportClient` account `plc-m12`
- **WHEN** a client sends `POST /adapter/v1/reports` with `HX-Request: true` and `HX-Target: machine-link-panel`
- **THEN** the navigation guard returns `403 E-WEB-007`

### Requirement: CSRF rejection
A mutating request without a valid token SHALL return `E-WEB-007`. The system SHALL write a security event of kind `csrf_rejected` for that request. The token check SHALL skip exactly the `Adapter` routes under `/adapter/v1/`, which authenticate only by HTTP Basic. It SHALL NOT compare the path text (A33, A55).

#### Scenario: Missing token
- **GIVEN** the Operator `mhoffman` with an open session
- **WHEN** an Operator runs `CreateRecipe` without a CSRF token
- **THEN** the system rejects the command with `E-WEB-007`
- **AND** the system writes a security event of kind `csrf_rejected`

#### Scenario: Adapter route without a token
- **GIVEN** the active `ExportClient` account `plc-m12` and the live machine `M12`
- **WHEN** `plc-m12` sends a valid status report without a CSRF token
- **THEN** the system stores the link state of `M12`
- **AND** the system writes no `csrf_rejected` event

#### Scenario: Path with a dot segment
- **GIVEN** the Administrator `akowal` with an open session
- **WHEN** a client sends `POST /adapter/v1/../users/mhoffman/deactivate` or one of its percent-encoded forms without a CSRF token
- **THEN** no handler runs
- **AND** `mhoffman` stays active

#### Scenario: Load form without a token
- **GIVEN** the Operator `mhoffman` with an open session and the approved version 3 of `R-100`
- **WHEN** `mhoffman` sends `POST /recipes/R-100/versions/3/load` without a CSRF token
- **THEN** the server rejects the request with `E-WEB-007`
- **AND** the system stores no load request

### Requirement: Origin check for mutating requests
A mutating request is a request with any method except `GET`, `HEAD`, and `OPTIONS`. The system SHALL reject a mutating request that has no `Origin` header matching the site and no `Sec-Fetch-Site` value of `same-origin`. An `Origin` header matches the site when it equals `https://` followed by the `Host` header. A rejected request SHALL get `E-WEB-007` and SHALL write a security event of kind `csrf_rejected`. Only the `Adapter` routes SHALL skip the check (R38, A55).

#### Scenario: Request from another site
- **GIVEN** the Operator `mhoffman` with an open session
- **WHEN** an Operator runs `CreateRecipe` with an `Origin` header of another site
- **THEN** the system rejects the request

#### Scenario: Request without an origin header
- **GIVEN** the Administrator `akowal` with an open session and a valid CSRF token
- **WHEN** `akowal` sends `POST /users/mhoffman/deactivate` without `Origin` and without `Sec-Fetch-Site`
- **THEN** the server rejects the request with `E-WEB-007`
- **AND** the system writes a security event of kind `csrf_rejected`
- **AND** `mhoffman` stays active

#### Scenario: Origin with another scheme
- **GIVEN** the setting `session.secure=true`
- **AND** the Administrator `akowal` with an open session and a valid CSRF token
- **WHEN** `akowal` sends `POST /users/mhoffman/deactivate` with `Host: recipes.plant.local` and `Origin: http://recipes.plant.local`
- **THEN** the server rejects the request with `E-WEB-007`

#### Scenario: Origin of the same host
- **GIVEN** the Administrator `akowal` with an open session and a valid CSRF token
- **WHEN** `akowal` sends `POST /users/mhoffman/deactivate` with `Host: recipes.plant.local` and `Origin: https://recipes.plant.local`
- **THEN** the server accepts the request

#### Scenario: Adapter client without browser headers
- **GIVEN** the active `ExportClient` account `plc-m12` and a `pending` request of `M12`
- **WHEN** `plc-m12` claims the request without `Origin` and without `Sec-Fetch-Site`
- **THEN** the server accepts the request

#### Scenario: Path next to the adapter prefix
- **GIVEN** the Operator `mhoffman` with an open session and a valid CSRF token
- **WHEN** `mhoffman` sends `POST /adapter/v1x/reports` without `Origin` and without `Sec-Fetch-Site`
- **THEN** the server rejects the request with `E-WEB-007`

### Requirement: Request body limits
The server SHALL accept an HTML form body of at most 256 KiB. The four routes that carry a parameter value SHALL accept at most `http.body_limit.value` (16 MiB). The route `POST .../translations/{lang}` SHALL accept at most 64 KiB. An adapter route under `/adapter/v1/` SHALL accept a JSON body of at most `http.body_limit.adapter` (64 KiB). A larger request SHALL get `E-WEB-003` (R30, R38, A55, ADR-0211).

#### Scenario: Form above the limit
- **GIVEN** the recipe `R-100` and a ProcessEngineer with an open session
- **WHEN** the ProcessEngineer sends a `RenameRecipe` form with a body of 300 KiB
- **THEN** the server rejects the request with `E-WEB-003`
- **AND** the name of `R-100` does not change

#### Scenario: Value form above the limit
- **GIVEN** the parameter `PROFILE` in the draft version 3 of `R-100` and a ProcessEngineer with an open session
- **WHEN** the ProcessEngineer sends a `SetParameterValue` form with a body of 17 MiB
- **THEN** the server rejects the request with `E-WEB-003`
- **AND** the value of `PROFILE` does not change

#### Scenario: Status report above the adapter limit
- **GIVEN** the active `ExportClient` account `plc-m12` and the live machine `M12`
- **WHEN** `plc-m12` sends `POST /adapter/v1/reports` with a body of 65 KiB
- **THEN** the server rejects the request with `413 E-WEB-003`
- **AND** the link state of `M12` does not change

### Requirement: Security event after an early end of the request
The rule covers `Login`, `ChangeOwnPassword`, `UnlockLogin`, and the Basic check of an export or adapter route. After the first transaction of one of them commits, the system SHALL write its security event also when the request ends early. After an early end, the system SHALL NOT create the session of `Login` or write the new password of `ChangeOwnPassword`. A failed event write SHALL give one `WARN` line. A closed connection and `http.request_timeout` end a request early (R20, R35, A33, A55).

#### Scenario: Client closes the connection during the password check
- **GIVEN** the active user `mhoffman`
- **WHEN** a client sends `POST /login` with the login `mhoffman` and a wrong password
- **AND** the client closes the connection before the password check ends
- **THEN** the system writes one login attempt row for `mhoffman`
- **AND** the system writes a security event of kind `login_failed`

#### Scenario: Client closes the connection before the session exists
- **GIVEN** the active user `mhoffman`
- **WHEN** a client sends `POST /login` with the login `mhoffman` and the right password
- **AND** the client closes the connection before the password check ends
- **THEN** the system creates no session for `mhoffman`

#### Scenario: Request limit after the commit of an unlock
- **GIVEN** the Administrator `kadmin` with an open session, and the login `mhoffman` with 3 failed attempts
- **WHEN** `kadmin` sends `POST /users/mhoffman/unlock`
- **AND** the limit `http.request_timeout` ends the request after the commit of `LoginUnlocked`
- **THEN** the system writes a security event of kind `unlock`

#### Scenario: Basic check with a closed connection
- **GIVEN** the active `ExportClient` account `xfer01`
- **WHEN** a transfer client sends `GET /export/v1/machines/M1/recipes/R-100/approved` with Basic credentials of `xfer01` and a wrong password
- **AND** the client closes the connection before the password check ends
- **THEN** the system writes a security event of kind `login_failed`

#### Scenario: Adapter Basic check with a closed connection
- **GIVEN** the active `ExportClient` account `plc-m12`
- **WHEN** an adapter sends `POST /adapter/v1/reports` with Basic credentials of `plc-m12` and a wrong password
- **AND** the client closes the connection before the password check ends
- **THEN** the system writes a security event of kind `login_failed`

## ADDED Requirements

### Requirement: Permission of the load requests
`RequestRecipeLoad`, `CancelRecipeLoad`, and `ReleaseRecipeLoad` SHALL check the permission `RecipeLoadRequest` as their first statement. Administrator, ProcessEngineer, and Operator SHALL hold it. ExportClient SHALL NOT hold it (R28, A55).

#### Scenario: Role without the permission
- **GIVEN** a test role without `RecipeLoadRequest`
- **WHEN** the generated matrix test runs `RequestRecipeLoad`, `CancelRecipeLoad`, and `ReleaseRecipeLoad` with that role
- **THEN** each use case returns `E-WEB-002` before it reads data

#### Scenario: Every person role
- **GIVEN** the roles Administrator, ProcessEngineer, and Operator
- **WHEN** the role test reads the permissions of each role
- **THEN** each role holds `RecipeLoadRequest`
