# Spec Delta

## MODIFIED Requirements

### Requirement: Unavailable response
When a timeout, a deadlock, pool exhaustion, or a database outage occurs, the server SHALL return `503` with the fragment `error.html`. The header `Retry-After` SHALL hold the value of the setting `http.retry_after`, which is 5 seconds. On a path under `/export/v1/`, the answer SHALL be the JSON error body with the same header (R36, R39).

#### Scenario: Pool exhausted
- **GIVEN** a connection pool with no free connection
- **WHEN** a ProcessEngineer sends a request for `GET /recipes`
- **THEN** the server returns `503` with the fragment `error.html`
- **AND** the response sets `Retry-After` to 5 seconds

#### Scenario: Timeout of an export
- **GIVEN** a request to `/export/v1/recipes/R-100/approved` that runs longer than `timeout.export`, which is 10 seconds
- **WHEN** the timeout ends the request
- **THEN** the answer is a JSON object with the code `E-WEB-009`
- **AND** the answer has the header `Retry-After: 5`

### Requirement: Internal error response
A caught panic, a failed `spawn_blocking` task, or an unexpected database error SHALL return `500` with the fragment `error.html`. The response SHALL show the error code and the `request_id` only. A path under `/export/v1/` SHALL get the JSON error body instead. A panic outside the error layer SHALL get a plain `500` text with `E-WEB-010` and the header `X-Request-Id` only (R30, R36, A41).

#### Scenario: Unexpected database error
- **GIVEN** a database error with an unexpected SQLSTATE
- **WHEN** a ProcessEngineer sends a request for `GET /recipes/R-100/versions/3`
- **THEN** the server returns `500 E-DB-008` with the fragment `error.html`
- **AND** the response shows the `request_id`
- **AND** the response shows no SQL, no error chain, and no file path

#### Scenario: Panic during an HTMX request
- **GIVEN** an HTMX request whose handler panics
- **WHEN** the answer arrives
- **THEN** the answer has the status 500, the code `E-WEB-010`, and the header `HX-Retarget: #error-panel`

#### Scenario: Panic outside the error layer
- **GIVEN** a request whose panic occurs in a layer outside `render_errors`
- **WHEN** the backstop catches the panic
- **THEN** the answer has the status 500, the text of `E-WEB-010`, and the header `X-Request-Id`
- **AND** the answer has no `HX-*` header and no JSON body

## ADDED Requirements

### Requirement: Read-only banner
While the system is in read-only mode, every page and every full page answer SHALL show one banner in the UI language. The banner SHALL say that changes are blocked and that viewing, login, logout, and export still work. It SHALL use text and an icon, not color alone (R39, A24).

#### Scenario: Banner after a disk check
- **GIVEN** an Operator with a session and the system in read-only mode
- **WHEN** the Operator opens the page `/recipes`
- **THEN** the page shows the read-only banner
- **AND** the banner holds no button

#### Scenario: Banner on the login page
- **GIVEN** a user without a session and the system in read-only mode
- **WHEN** the user opens the page `/login`
- **THEN** the page shows the read-only banner

#### Scenario: No banner in normal mode
- **GIVEN** a ProcessEngineer with a session and the system not in read-only mode
- **WHEN** the ProcessEngineer opens the page `/recipes`
- **THEN** the page shows no read-only banner

### Requirement: Code of a slow request body
A request body that does not arrive within 10 seconds SHALL give `E-WEB-009`, not `E-WEB-003` (R30, R36).

#### Scenario: Slow login form
- **GIVEN** a `POST /login` whose body stops after 10 bytes
- **WHEN** 10 seconds pass
- **THEN** the answer has the code `E-WEB-009`
