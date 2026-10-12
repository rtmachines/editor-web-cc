# web-ui

## Purpose

The web UI renders HTML fragments for the recipe editor and protects drafts from overwrites. It maps errors to HTTP responses and selects the UI language.

## ADDED Requirements

### Requirement: Row version in every form
The system SHALL render the current `row_version` of a version into the hidden field `row_version` of every form on the version page. Every mutating request on that version SHALL send `row_version` (R39, I12).

#### Scenario: Hidden field on the version page
- **GIVEN** the draft version 3 of the recipe `R-100` with `row_version` 7
- **WHEN** a ProcessEngineer sends a request for `GET /recipes/R-100/versions/3`
- **THEN** every form of the page holds the hidden field `row_version` with the value 7

### Requirement: Row version update after a success
On success, the response SHALL set the header `HX-Trigger` to `{"rowVersion": <new>}`. The script `row-version.js` SHALL update every hidden `row_version` field of the page (R39, I12).

#### Scenario: Save with the current row version
- **GIVEN** the draft version 3 of the recipe `R-100` with `row_version` 7
- **WHEN** a ProcessEngineer sends a change of a parameter value with `row_version` 7
- **THEN** the response sets the header `HX-Trigger` to `{"rowVersion": 8}`
- **AND** every hidden `row_version` field of the page holds the value 8

### Requirement: Conflict fragment
On a mismatch of `row_version`, the server SHALL return `409 E-WEB-001` with the fragment `conflict.html`. The fragment SHALL show the current `row_version`, the login of the last editor, and the time of the last change. The fragment SHALL show one button, Reload version, that sends a GET request to the version page (R36, R39, I12).

#### Scenario: Stale save keeps the input
- **GIVEN** the draft version 3 of the recipe `R-100` with `row_version` 8 and the last editor `anowak`
- **WHEN** a ProcessEngineer sends a change of a parameter value with `row_version` 7
- **THEN** the server returns `409 E-WEB-001` with the fragment `conflict.html`
- **AND** the fragment shows the `row_version` 8 and the login `anowak`
- **AND** the unsaved input stays in the form until the reload

### Requirement: Parameter edit changes the version token
A change of a parameter value SHALL change the `row_version` of the version, not only the parameter row. Two users who edit two different parameters of one draft SHALL get a conflict on the second save (R39, I12).

#### Scenario: Two parameters of one draft
- **GIVEN** the draft version 3 of the recipe `R-100` with `row_version` 7
- **AND** a ProcessEngineer saved a parameter value, so the `row_version` became 8
- **WHEN** a second ProcessEngineer sends a change of another parameter with `row_version` 7
- **THEN** the server returns `409 E-WEB-001` with the fragment `conflict.html`

### Requirement: Double submit of a form
Every approve form and every save form SHALL have the attribute `hx-sync="closest form:drop"`. The browser SHALL drop a second submit of such a form while its request is in flight (R39, I12).

#### Scenario: Second submit during a request
- **GIVEN** a save form of the draft version 3 with a request in flight
- **WHEN** a ProcessEngineer submits the same save form again
- **THEN** the browser drops the second submit

### Requirement: Idempotent approval
An approval can carry a `row_version` one lower than the current value of an approved version. The server SHALL then render the version page with status `200` (R39, I12).

#### Scenario: Repeated approval
- **GIVEN** the approved version 3 of the recipe `R-100` with `row_version` 9
- **WHEN** a ProcessEngineer sends an approval of version 3 with `row_version` 8
- **THEN** the server renders the version page with status `200`

### Requirement: HTMX error headers
For an HTMX request with a status of 400 or higher, the response SHALL set `HX-Retarget` to `#error-panel` and `HX-Reswap` to `innerHTML`, except for `E-WEB-004` (R39).

#### Scenario: Concurrent edit response headers
- **GIVEN** a ProcessEngineer sends an HTMX save with a stale `row_version` of the draft version 3
- **WHEN** the server returns `409 E-WEB-001`
- **THEN** the response sets `HX-Retarget` to `#error-panel`
- **AND** the response sets `HX-Reswap` to `innerHTML`

### Requirement: Session end response
The server SHALL return `401 E-WEB-004` when a session expires or does not exist. For an HTMX request, the response SHALL set `HX-Redirect` to `/login`. For another request, the server SHALL return `303` to `/login`. Only `E-WEB-004` SHALL use `HX-Redirect` (R39).

#### Scenario: Ended session in an HTMX request
- **GIVEN** a session that expires
- **WHEN** a ProcessEngineer sends an HTMX request for `GET /recipes`
- **THEN** the server returns `401 E-WEB-004`
- **AND** the response sets `HX-Redirect` to `/login`

### Requirement: Validation error response
A malformed field, a missing field, or a request with a wrong content type SHALL return `422` with the fragment `field-errors.html`. The fragment SHALL show one line for each field (R36, R39).

#### Scenario: Missing field
- **GIVEN** a form that omits a required field
- **WHEN** a ProcessEngineer sends the form
- **THEN** the server returns `422 E-WEB-008` with the fragment `field-errors.html`
- **AND** the fragment shows one line that names the field

### Requirement: Client error statuses
The server SHALL return `409` for a state conflict and `403` for a missing permission. It SHALL return `404` for a missing resource in the URL and `413` for a request above the body limit. These responses SHALL use the fragment `error.html` (R39).

#### Scenario: Missing permission
- **GIVEN** a user with the role Operator
- **WHEN** an Operator sends a request for an operation that needs `ParameterEdit`
- **THEN** the server returns `403 E-WEB-002` with the fragment `error.html`
- **AND** the fragment names the operation

### Requirement: Login throttle response
When the login throttle is active, the server SHALL return `429 E-IDN-005` with the fragment `error.html` and the header `Retry-After` (R39).

#### Scenario: Throttled login
- **GIVEN** the login throttle is active for the login `anowak` and the client address
- **WHEN** a user sends `POST /login` with the login `anowak`
- **THEN** the server returns `429 E-IDN-005`
- **AND** the response sets the header `Retry-After`

### Requirement: Unavailable response
When a timeout, a deadlock, pool exhaustion, or a database outage occurs, the server SHALL return `503` with the fragment `error.html`. The header `Retry-After` SHALL hold the value of the setting `http.retry_after`, which is 5 seconds (R39).

#### Scenario: Pool exhausted
- **GIVEN** a connection pool with no free connection
- **WHEN** a ProcessEngineer sends a request for `GET /recipes`
- **THEN** the server returns `503` with the fragment `error.html`
- **AND** the response sets `Retry-After` to 5 seconds

### Requirement: Internal error response
A caught panic, a failed background task, or an unexpected database error SHALL return `500` with the fragment `error.html`. The response SHALL show the error code and the `request_id` only (R36).

#### Scenario: Unexpected database error
- **GIVEN** a database error that has no domain code
- **WHEN** a ProcessEngineer sends a request for `GET /recipes/R-100/versions/3`
- **THEN** the server returns `500 E-DB-008` with the fragment `error.html`
- **AND** the response shows the `request_id`
- **AND** the response shows no SQL, no error chain, and no file path

### Requirement: Error fragment content
Every error fragment SHALL show the error code `E-<CTX>-<nnn>`, one sentence in the UI language that tells the user what to do, and the `request_id` (R36).

#### Scenario: Validation error fragment
- **GIVEN** a ProcessEngineer with the UI language `en`
- **WHEN** the ProcessEngineer sends a request that gets a validation error
- **THEN** the error fragment shows the code `E-WEB-008`
- **AND** the fragment shows one English sentence that tells the user what to do
- **AND** the fragment shows the `request_id`
- **AND** the structured log of the request holds the same code

### Requirement: Full error page
A request without the header `HX-Request` SHALL get a full error page with the same content as the error fragment and the page layout (R36, R39).

#### Scenario: Missing recipe without HTMX
- **GIVEN** no recipe `R-999` exists
- **WHEN** a user sends `GET /recipes/R-999` without the header `HX-Request`
- **THEN** the server returns `404 E-WEB-005` with a full error page
- **AND** the page shows the code and the `request_id`

### Requirement: JSON error on export routes
The export routes SHALL return a JSON error body with the fields `code`, `message`, and `request_id` (R36).

#### Scenario: Recipe without an approved version
- **GIVEN** the recipe `R-100` with no approved version
- **WHEN** a user sends `GET /export/v1/recipes/R-100/approved`
- **THEN** the server returns `404` with a JSON error body
- **AND** the field `code` of the body holds `E-EXP-001`

### Requirement: Read-only error response
In read-only mode, a command other than `Login`, `Logout`, and `ExportVersion` SHALL return `503 E-WEB-006` with the fragment `error.html` and the read-only banner (A24).

#### Scenario: Save in read-only mode
- **GIVEN** the application in read-only mode
- **WHEN** a ProcessEngineer sends a change of a parameter value
- **THEN** the server returns `503 E-WEB-006` with the fragment `error.html`
- **AND** the page shows the read-only banner

### Requirement: Routes available in read-only mode
In read-only mode, queries, the export routes, `POST /login`, `POST /logout`, and `POST /lang` SHALL stay available (A24).

#### Scenario: Version page in read-only mode
- **GIVEN** the application in read-only mode
- **WHEN** an Operator sends a request for `GET /recipes/R-100/versions/3`
- **THEN** the server returns the version page with status `200`

### Requirement: Language cookie value
The cookie `recipe_lang` SHALL hold `pl` or `en`, and the server SHALL read it on every request. A missing cookie or another value SHALL select the UI language `pl`. The server SHALL ignore the header `Accept-Language` (R10, A17).

#### Scenario: Unknown cookie value
- **GIVEN** the cookie `recipe_lang` with the value `de` and the header `Accept-Language: en-US`
- **WHEN** a user sends a request for `GET /recipes`
- **THEN** the UI uses the language `pl`

### Requirement: Language cookie attributes
The cookie `recipe_lang` SHALL have a `Max-Age` of 365 days, `Secure`, `SameSite=Strict`, and `Path=/` (R10, A17).

#### Scenario: Cookie set by a language change
- **GIVEN** a user with no session
- **WHEN** a user sends `POST /lang` with the language `en`
- **THEN** the response sets the cookie `recipe_lang` with a `Max-Age` of 365 days
- **AND** the cookie has the attributes `Secure`, `SameSite=Strict`, and `Path=/`

### Requirement: Language switch redirect
`POST /lang` SHALL return `303` to the field `next` when `next` is a same-origin relative path. The path SHALL start with `/` and SHALL NOT start with `//`. Otherwise the server SHALL return `303` to `/recipes` and SHALL NOT use the `Referer` header (R38, A17).

#### Scenario: Redirect to an external address
- **GIVEN** a user with no session
- **WHEN** a user sends `POST /lang` with the language `en` and the field `next` set to `https://other.example/page`
- **THEN** the response sets the cookie `recipe_lang` to `en`
- **AND** the response returns `303` to `/recipes`

#### Scenario: Redirect to a same-origin path
- **GIVEN** a user with no session
- **WHEN** a user sends `POST /lang` with the language `pl` and the field `next` set to `/recipes/R-100/versions/3`
- **THEN** the response returns `303` to `/recipes/R-100/versions/3`

### Requirement: Language switch without session or audit
`POST /lang` SHALL work before login and in read-only mode. It SHALL write no audit entry, because it writes no database row (A17, A24).

#### Scenario: Language change in read-only mode
- **GIVEN** the application in read-only mode and a user with no session
- **WHEN** a user sends `POST /lang` with the language `en`
- **THEN** the response sets the cookie `recipe_lang` to `en` and returns `303`
- **AND** the system writes no audit entry

### Requirement: Translation fallback
A parameter name or description SHALL show the translation for the UI language when a translation exists. Otherwise the page SHALL show the English base text with the marker `(en)` (R10, A17).

#### Scenario: Missing Polish translation
- **GIVEN** the parameter `SPEED` with the English name `Line speed` and no `pl` translation
- **WHEN** a user sends `GET /recipes/R-100/versions/3` with the cookie `recipe_lang` set to `pl`
- **THEN** the page shows `Line speed (en)`

### Requirement: Messages from the fluent bundles
Error messages, status words, and permission names SHALL come from the `fluent` bundles `pl` and `en`. An error code SHALL NOT change with the UI language (R10, R36, A17).

#### Scenario: Error message in English
- **GIVEN** the cookie `recipe_lang` set to `en`
- **WHEN** an Operator sends a request that needs `ParameterEdit`
- **THEN** the error fragment shows the English message of `E-WEB-002`
- **AND** the error fragment shows the code `E-WEB-002`

### Requirement: English UI text style
English UI strings and English error message templates SHALL follow the structural ASD-STE100 rules of `reliability.md` section 8 (R40).

#### Scenario: English error template
- **GIVEN** the English message template of `E-WEB-005`
- **WHEN** a user sends a request for `GET /recipes/R-999`
- **THEN** the error fragment shows the English template with the recipe reference
- **AND** the template has at most 25 words and no semicolon
