## MODIFIED Requirements

### Requirement: Export document as the only machine output
The system SHALL provide the JSON export document and the JSON recipe list of the export routes as its only machine-readable outputs of recipe data. The JSON answers of the adapter routes of the capability `machine-link` SHALL hold no recipe data. The system SHALL NOT communicate with a PLC (R03, A2, A55).

#### Scenario: Export of an approved version
- **GIVEN** the approved version 3 of the recipe `R-100` on the machine `M12`
- **WHEN** an Operator runs `ExportVersion` through `GET /export/v1/recipes/R-100/approved`
- **THEN** the system returns the export document as an HTTP response
- **AND** the system starts no connection to a PLC

#### Scenario: Recipe list of a transfer client
- **GIVEN** the approved version 3 of the recipe `R-100` on the machine `M12`
- **WHEN** the `ExportClient` account `plc-m12` runs `ListExportableRecipes` through `GET /export/v1/machines/M12/recipes`
- **THEN** the system returns the recipe list as an HTTP response
- **AND** the system starts no connection to a PLC

#### Scenario: Pending load request in a report answer
- **GIVEN** a `pending` load request of `M12` for the approved version 3 of `R-100`
- **WHEN** `plc-m12` sends a status report for `M12` through `POST /adapter/v1/reports`
- **THEN** the answer names `R-100`, the version 3, its `content_sha256`, and the path `/export/v1/machines/M12/recipes/R-100/approved`
- **AND** the answer holds no parameter value

### Requirement: Bound on parallel Basic checks
At most `export.basic_permits` Basic checks of the transfer routes SHALL wait for or hold an Argon2 permit at the same time. At most `adapter.basic_permits` Basic checks of the adapter routes SHALL do the same. A check that gets no permit within `argon2.acquire_timeout` SHALL give `503 E-WEB-009` with `Retry-After` (R30, A45, A55).

#### Scenario: Busy Basic check
- **GIVEN** one Basic check of a transfer route that holds the Basic permit for longer than `argon2.acquire_timeout`
- **WHEN** a second client sends a request with Basic credentials to a transfer route
- **THEN** the system returns `503 E-WEB-009` with `Retry-After`
- **AND** a login through `POST /login` in the same time gets an Argon2 permit

#### Scenario: Adapter report during a busy export check
- **GIVEN** one Basic check of a transfer route that holds the permit of `export.basic_permits`
- **WHEN** an adapter sends `POST /adapter/v1/reports` with Basic credentials
- **THEN** the Basic check of the report takes the permit of `adapter.basic_permits`
- **AND** the system applies the report

#### Scenario: Two parallel adapter reports
- **GIVEN** one Basic check of an adapter route that holds the permit of `adapter.basic_permits` for longer than `argon2.acquire_timeout`
- **WHEN** a second adapter sends `POST /adapter/v1/reports`
- **THEN** the system returns `503 E-WEB-009` with `Retry-After` as the JSON error body
- **AND** a login through `POST /login` in the same time gets an Argon2 permit
