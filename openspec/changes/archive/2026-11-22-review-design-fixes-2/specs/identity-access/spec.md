## MODIFIED Requirements

### Requirement: Request body limits
The server SHALL accept an HTML form body of at most 256 KiB. The four routes that carry a parameter value SHALL accept at most `http.body_limit.value` (16 MiB). The route `POST .../translations/{lang}` SHALL accept at most 64 KiB. A larger request SHALL get `E-WEB-003` (R30, R38, ADR-0211).

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

### Requirement: Origin check for mutating requests
A mutating request is a request with any method except `GET`, `HEAD`, and `OPTIONS`. The system SHALL reject a mutating request that has no `Origin` header matching the site and no `Sec-Fetch-Site` value of `same-origin`. An `Origin` header matches the site when it equals `https://` followed by the `Host` header. With `session.secure=false` in a `dev` build, an `Origin` header also matches when it equals `http://` followed by the `Host` header. A rejected request SHALL get `E-WEB-007` and SHALL write a security event of kind `csrf_rejected` (R38, ADR-0220).

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

#### Scenario: Plain HTTP origin in a dev build
- **GIVEN** a `dev` build with the setting `session.secure=false`
- **AND** the Administrator `akowal` with an open session and a valid CSRF token
- **WHEN** `akowal` sends `POST /users/mhoffman/deactivate` with `Host: recipes.test:18082` and `Origin: http://recipes.test:18082`
- **THEN** the server accepts the request

#### Scenario: Origin of the same host
- **GIVEN** the Administrator `akowal` with an open session and a valid CSRF token
- **WHEN** `akowal` sends `POST /users/mhoffman/deactivate` with `Host: recipes.plant.local` and `Origin: https://recipes.plant.local`
- **THEN** the server accepts the request
