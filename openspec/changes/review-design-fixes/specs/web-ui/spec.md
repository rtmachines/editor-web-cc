## ADDED Requirements

### Requirement: Notice after a committed change with a failed read
When a command commits and the read of its answer fails, the server SHALL answer an HTMX request with `200` and a notice in `#toast`. The notice SHALL say "The change is saved. Reload the page." The answer SHALL empty `#error-panel` and SHALL set `HX-Trigger` with `rowVersion` when the command gives the new value. The server SHALL write one `WARN` line with the code of the read fault and the `request_id`. A request without HTMX SHALL get `303` to the page of the changed object (R31, R35, R39).

#### Scenario: Parameter removal with a failed read of the version page
- **GIVEN** the ProcessEngineer `anowak` with an open session and the parameter `SPEED` of the draft version 3 of `R-100` with `row_version` 7
- **AND** a read of the version page after the commit that fails with `E-DB-004`
- **WHEN** `anowak` sends an HTMX request that removes `SPEED` with `row_version` 7
- **THEN** the server returns `200` with the notice in `#toast`
- **AND** the response empties `#error-panel` out of band
- **AND** the response sets the header `HX-Trigger` to `{"rowVersion": 8}`
- **AND** the draft version 3 has no parameter `SPEED`
- **AND** the event log holds one `WARN` line with `E-DB-004` and the `request_id` of the request

#### Scenario: Approval with a failed read of the version page
- **GIVEN** the Administrator `kadmin` with an open session and the draft version 4 of `R-100`
- **AND** a read of the version page after the commit that fails with `E-DB-004`
- **WHEN** `kadmin` sends an HTMX request that approves version 4
- **THEN** the server returns `200` with the notice in `#toast`
- **AND** the response has no header `HX-Trigger`
- **AND** version 4 has the status `Approved`

### Requirement: Create form of a user drops a second submit
The form of `POST /users` SHALL drop a second submit while its first request runs. The answer SHALL stay the full result page that shows the one-time password once (R39).

#### Scenario: Double click on the create button
- **GIVEN** the Administrator `kadmin` with an open session on the page `/users/new`
- **WHEN** `kadmin` clicks the create button two times before the first answer arrives
- **THEN** the browser sends one request `POST /users`
- **AND** the page shows the one-time password of the new user

## MODIFIED Requirements

### Requirement: Language cookie attributes
The cookie `recipe_lang` SHALL have a `Max-Age` of 365 days, `SameSite=Strict`, and `Path=/`. It SHALL have `Secure`, except when `session.secure` is false in a build with the `dev` feature (R10, A17).

#### Scenario: Cookie set by a language change
- **GIVEN** a user with no session
- **WHEN** a user sends `POST /lang` with the language `en`
- **THEN** the response sets the cookie `recipe_lang` with a `Max-Age` of 365 days
- **AND** the cookie has the attributes `Secure`, `SameSite=Strict`, and `Path=/`

#### Scenario: Language cookie in a dev build without Secure
- **GIVEN** a build with the `dev` feature and `session.secure` set to false
- **WHEN** a user sends `POST /lang` with the language `en`
- **THEN** the response sets the cookie `recipe_lang` with `SameSite=Strict` and `Path=/`
- **AND** the cookie has no attribute `Secure`
