# web-ui Specification

## Purpose
The web UI renders HTML fragments for the recipe editor and protects drafts from overwrites. It maps errors to HTTP responses and selects the UI language.

## Requirements

### Requirement: Row version in every form
The system SHALL render the current `row_version` of a version into the hidden field `row_version` of every form on the version page. Every mutating request on that version SHALL send `row_version` (R39, I12).

#### Scenario: Hidden field on the version page
- **GIVEN** the draft version 3 of the recipe `R-100` with `row_version` 7
- **WHEN** a ProcessEngineer sends a request for `GET /recipes/R-100/versions/3`
- **THEN** every form of the page holds the hidden field `row_version` with the value 7

### Requirement: New row version after a success
On success, the response SHALL set the header `HX-Trigger` to `{"rowVersion": <new>}`. The script `row-version.js` SHALL set every hidden `row_version` field of the page to the new value (R39, I12).

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

#### Scenario: Reload link without user data in an hx attribute
- **GIVEN** a conflict on the draft version 3 of the recipe `R-100`
- **WHEN** the server renders `conflict.html`
- **THEN** the button Reload version is a link with `href="/recipes/R-100/versions/3"` and `hx-boost="true"`
- **AND** no `hx-*` attribute of the fragment holds `R-100`

#### Scenario: Time and editor of the last change
- **GIVEN** the draft version 3 of the recipe `R-100`, which `anowak` changed last on 2026-10-14 at 10:15:03 UTC
- **WHEN** a ProcessEngineer `bnowicki` sends a change with an old `row_version`
- **THEN** the fragment shows the login `anowak` and the time `2026-10-14 10:15 UTC` in the format of `design-system.md` section 5
- **AND** the fragment says that the reload discards the unsaved input

### Requirement: Parameter change sets a new row version
A change of a parameter value SHALL change the `row_version` of the version, not only the parameter row. Two users who change two different parameters of one draft SHALL get a conflict on the second save (R39, I12).

#### Scenario: Two parameters of one draft
- **GIVEN** the draft version 3 of the recipe `R-100` with `row_version` 7
- **AND** a ProcessEngineer saved a parameter value, so the `row_version` became 8
- **WHEN** a second ProcessEngineer sends a change of another parameter with `row_version` 7
- **THEN** the server returns `409 E-WEB-001` with the fragment `conflict.html`

### Requirement: Second request of a form
Every approve form, every save form, and every retire, discard, delete, and restore form SHALL have the attribute `hx-sync="closest form:drop"`. The browser SHALL drop a second request of such a form while the first request is in flight. No form SHALL have the attribute `hx-disabled-elt`, so a refused request keeps the focus on the button (R39, I12, ADR-0049).

#### Scenario: Second request while a request is in flight
- **GIVEN** a save form of the draft version 3 with a request in flight
- **WHEN** a ProcessEngineer sends the same save form again
- **THEN** the browser drops the second request

#### Scenario: Retire form
- **GIVEN** the page of the approved version 2 of `R-100` for the Administrator `admin`
- **WHEN** the test reads the retire form
- **THEN** the form has `hx-sync="closest form:drop"` and no `hx-disabled-elt`

### Requirement: Indicator of a form in flight
The approve, retire, discard, delete, and restore forms SHALL have an indicator with the text of `form-in-progress`. Three save forms of a draft SHALL have that indicator too. They are the edit form and the new form of a parameter, and the save form of a translation row (R39, ADR-0049, ADR-0158).

#### Scenario: Indicator of a save form
- **GIVEN** the ProcessEngineer `anowak` with an open session and the draft version 3 of `R-100` with the parameter `LIMIT`
- **WHEN** `anowak` sends `GET` for the edit page of `LIMIT`
- **THEN** the edit form and the save form of each translation row hold an element with the class `htmx-indicator` and the text of `form-in-progress`
- **AND** no form of the page has `hx-disabled-elt`

### Requirement: Idempotent approval
When `ApproveVersion` returns `AlreadyApproved`, the server SHALL render the version page with status `200` (R39, I12).

#### Scenario: Repeated approval
- **GIVEN** the approved version 3 of the recipe `R-100` with `row_version` 9
- **WHEN** a ProcessEngineer sends an approval of version 3 with `row_version` 8
- **THEN** the server renders the version page with status `200`

### Requirement: HTMX error headers
For an HTMX request with a status of 400 or higher, the response SHALL set `HX-Retarget` to `#error-panel` and `HX-Reswap` to `innerHTML show:#error-panel:top`, except for `E-WEB-004`. The same response SHALL set `HX-Push-Url` to `false`, so a refused form keeps the URL of its page (R39, ADR-0037).

#### Scenario: Concurrent edit response headers
- **GIVEN** a ProcessEngineer sends an HTMX save with a stale `row_version` of the draft version 3
- **WHEN** the server returns `409 E-WEB-001`
- **THEN** the response sets `HX-Retarget` to `#error-panel`
- **AND** the response sets `HX-Reswap` to `innerHTML show:#error-panel:top`

#### Scenario: Refused machine form keeps the URL
- **GIVEN** the ProcessEngineer `anowak` with an open session on the page `/machines/new`
- **WHEN** `anowak` sends an HTMX `POST /machines` with the code `X1`
- **THEN** the server returns `422 E-WEB-008`
- **AND** the response sets `HX-Push-Url` to `false`

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

### Requirement: Error fragment content
Every error fragment SHALL show the error code `E-<CTX>-<nnn>`, one sentence in the UI language that tells the user what to do, and the `request_id`. The fragment of `E-WEB-015` is the one exception among the fragments: it has no `request_id`, because no answer arrived. Each code of the routes of `api.md` sections 2.2 and 2.3 SHALL have its sentence in the bundles `pl` and `en`. The code `E-WEB-015` SHALL also have its sentence in both bundles (R10, R36, R39).

#### Scenario: Validation error fragment
- **GIVEN** a ProcessEngineer with the UI language `en`
- **WHEN** the ProcessEngineer sends a request that gets a validation error
- **THEN** the error fragment shows the code `E-WEB-008`
- **AND** the fragment shows one English sentence that tells the user what to do
- **AND** the fragment shows the `request_id`
- **AND** the structured log of the request holds the same code

#### Scenario: Plant error in Polish
- **GIVEN** the ProcessEngineer `anowak` with an open session and the UI language `pl`
- **WHEN** `anowak` sends an HTMX request for `GET /machines/M7/units/X1`
- **THEN** the server returns `422 E-PLT-017`
- **AND** the fragment shows the sentence of `error-E-PLT-017` from the bundle `pl`

#### Scenario: Fragment of a lost answer
- **GIVEN** the UI language `en`
- **WHEN** the server renders the template `#transport-error` of a full page
- **THEN** the fragment shows `E-WEB-015` and the sentence of `error-E-WEB-015` from the bundle `en`
- **AND** the fragment has no element `.alert-ref`

### Requirement: Full error page
A request without the header `HX-Request` SHALL get a full error page. The page SHALL have the content of the error fragment and the page layout (R01, R36, R39).

#### Scenario: Missing recipe without HTMX
- **GIVEN** no recipe `R-999` exists
- **WHEN** a user sends `GET /recipes/R-999` without the header `HX-Request`
- **THEN** the server returns `404 E-WEB-005` with a full error page
- **AND** the page shows the code and the `request_id`

### Requirement: JSON error on export routes
Every answer of an export route with an error code SHALL be a JSON body. Its fields SHALL be `code`, `message`, and `request_id` in this order. `message` SHALL be the sentence of the code in the UI language. The answer SHALL hold no `HX-*` header (R10, R36).

#### Scenario: Recipe without an approved version
- **GIVEN** the recipe `R-100` with no approved version
- **WHEN** a user sends `GET /export/v1/recipes/R-100/approved`
- **THEN** the server returns `404` with a JSON error body
- **AND** the field `code` of the body holds `E-EXP-001`

#### Scenario: Message in the UI language
- **GIVEN** the Operator `mhoffman` with the UI language `pl` and the recipe `R-100` with no approved version
- **WHEN** `mhoffman` sends `GET /export/v1/recipes/R-100/approved`
- **THEN** the field `message` of the body holds the Polish sentence of `E-EXP-001`

#### Scenario: Operator on the versions route
- **GIVEN** the Operator `mhoffman` with an open session and the retired version 2 of `R-100`
- **WHEN** `mhoffman` sends `GET /export/v1/recipes/R-100/versions/2`
- **THEN** the server returns `403` with a JSON error body and the code `E-WEB-002`
- **AND** the system writes one `forbidden` security event

#### Scenario: HTMX header on an export route
- **GIVEN** the Operator `mhoffman` with an open session and the recipe `R-100` with no approved version
- **WHEN** `mhoffman` sends `GET /export/v1/recipes/R-100/approved` with the header `HX-Request: true`
- **THEN** the server returns `403` with a JSON error body and the code `E-WEB-007`
- **AND** the answer has no header `HX-Retarget`, `HX-Reswap`, or `HX-Push-Url`
- **AND** the system writes one `csrf_rejected` security event

### Requirement: Read-only error response
In read-only mode, a command other than `Login`, `Logout`, and `ExportVersion` SHALL return `503 E-WEB-006` with the fragment `error.html` and the read-only banner (A24).

#### Scenario: Save in read-only mode
- **GIVEN** the application in read-only mode
- **WHEN** a ProcessEngineer sends a change of a parameter value
- **THEN** the server returns `503 E-WEB-006` with the fragment `error.html`
- **AND** the page shows the read-only banner

### Requirement: Routes available in read-only mode
In read-only mode, queries, the export routes, `POST /login`, and `POST /logout` SHALL stay available (A24).

#### Scenario: Version page in read-only mode
- **GIVEN** the application in read-only mode
- **WHEN** an Operator sends a request for `GET /recipes/R-100/versions/3`
- **THEN** the server renders the version page with status `200`

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
`POST /lang` SHALL return `303` to the field `next` when `next` is a same-origin relative path. The path SHALL start with `/`, and its second character SHALL be neither `/` nor `\`. The path SHALL hold no `\` and no control or white-space character. Otherwise the server SHALL return `303` to `/recipes` and SHALL NOT use the `Referer` header (R38, A17).

#### Scenario: Redirect to an external address
- **GIVEN** a user with no session
- **WHEN** a user sends `POST /lang` with the language `en` and the field `next` set to `https://other.example/page`
- **THEN** the response sets the cookie `recipe_lang` to `en`
- **AND** the response returns `303` to `/recipes`

#### Scenario: Redirect to a same-origin path
- **GIVEN** a user with no session
- **WHEN** a user sends `POST /lang` with the language `pl` and the field `next` set to `/recipes/R-100/versions/3`
- **THEN** the response returns `303` to `/recipes/R-100/versions/3`

#### Scenario: Backslash after the first slash
- **GIVEN** a user with no session
- **WHEN** a user sends `POST /lang` with the language `en` and the field `next` set to `/\evil.example`
- **THEN** the response returns `303` to `/recipes`

#### Scenario: Tab after the first slash
- **GIVEN** a user with no session
- **WHEN** a user sends `POST /lang` with the language `en` and the field `next` set to `/`, a tab character, and `/evil.example`
- **THEN** the response returns `303` to `/recipes`

### Requirement: Language switch without session or audit
`POST /lang` SHALL work before login and in read-only mode. It SHALL write no audit entry, because it writes no database row (A17, A24).

#### Scenario: Language change in read-only mode
- **GIVEN** the application in read-only mode and a user with no session
- **WHEN** a user sends `POST /lang` with the language `en`
- **THEN** the response sets the cookie `recipe_lang` to `en` and returns `303`
- **AND** the system writes no audit entry

### Requirement: Translation fallback
A parameter name or description that a page shows without a label SHALL show the translation for the UI language when a translation exists. Otherwise the page SHALL show the English base text with the marker `(en)` (R10, A17). With the UI language `en`, the page SHALL show the English base text without the marker (ADR-0126). A labeled field of the English base text SHALL show the English base text in each UI language (ADR-0126 (f)).

#### Scenario: Missing Polish translation
- **GIVEN** the parameter `SPEED` with the English name `Line speed` and no `pl` translation
- **WHEN** a user sends `GET /recipes/R-100/versions/3` with the cookie `recipe_lang` set to `pl`
- **THEN** the page shows `Line speed (en)`

#### Scenario: Stored Polish translation
- **GIVEN** the parameter `SPEED` with the English name `Line speed` and the stored `pl` translation with the name `Prędkość linii`
- **WHEN** a user sends `GET /recipes/R-100/versions/3` with the cookie `recipe_lang` set to `pl`
- **THEN** the row of `SPEED` shows `Prędkość linii` without the marker `(en)`
- **AND** the row of `SPEED` does not show `Line speed`

#### Scenario: English UI language
- **GIVEN** the parameter `SPEED` with the English name `Line speed` and the stored `pl` translation with the name `Prędkość linii`
- **WHEN** a user sends `GET /recipes/R-100/versions/3` with the cookie `recipe_lang` set to `en`
- **THEN** the row of `SPEED` shows `Line speed` without the marker `(en)`

#### Scenario: Row of a new parameter
- **GIVEN** a live `Draft`, a user with `ParameterEdit`, and the cookie `recipe_lang` set to `pl`
- **WHEN** the user adds the parameter `FEED` with the English name `Feed rate` through an HTMX request
- **THEN** the row fragment shows `Feed rate (en)`

#### Scenario: Added parameter with a Polish translation in the diff
- **GIVEN** the retired version 2 and the approved version 3 of `R-100`
- **AND** version 3 adds the parameter `FEED` with the English name `Feed rate` and the stored `pl` translation with the name `Posuw`
- **WHEN** the Operator `mhoffman` sends `GET /recipes/R-100/versions/3/diff/2` with the cookie `recipe_lang` set to `pl`
- **THEN** the row of `FEED` in the added section shows `Posuw` without the marker `(en)`
- **AND** the row of `FEED` in the added section does not show `Feed rate`

#### Scenario: Added parameter without a translation in the approval preview
- **GIVEN** the approved version 2 and the draft version 3 of `R-100`
- **AND** the draft adds the parameter `FEED` with the English name `Feed rate` and no `pl` translation
- **WHEN** the Administrator `admin` sends `GET /recipes/R-100/versions/3/approve` with the cookie `recipe_lang` set to `pl`
- **THEN** the row of `FEED` in the added section shows `Feed rate (en)`
- **AND** the element that holds `Feed rate` has the attribute `lang="en"`
- **AND** the marker `(en)` stands outside that element

#### Scenario: Old-only parameter on an English page
- **GIVEN** the retired version 2 of `R-100` with the parameter `SPEED`, the English name `Line speed`, and the stored `pl` translation with the name `Prędkość linii`
- **AND** the approved version 3 of `R-100` without `SPEED`
- **WHEN** `mhoffman` sends `GET /recipes/R-100/versions/3/diff/2` with the cookie `recipe_lang` set to `en`
- **THEN** the row of `SPEED` in the section of the parameters only in the old version shows `Line speed` without the marker `(en)`
- **AND** the cell of that name has no `lang` attribute

### Requirement: Language tag of the English base text
The English base text in a page of another language SHALL carry the language tag `en` (R39, A44).

#### Scenario: Language tag of the English base text
- **GIVEN** the parameter `SPEED` with the English name `Line speed` and no `pl` translation
- **WHEN** a user sends `GET /recipes/R-100/versions/3` with the cookie `recipe_lang` set to `pl`
- **THEN** the element that holds `Line speed` has the attribute `lang="en"`
- **AND** the marker `(en)` stands outside that element

#### Scenario: English base text inputs of the parameter form
- **GIVEN** the live draft version 3 of `R-100` with the parameter `SPEED` and the ProcessEngineer `anowak` with an open session
- **WHEN** `anowak` opens the edit form of `SPEED` with the cookie `recipe_lang` set to `pl`
- **THEN** the input `name_en` has the attribute `lang="en"`
- **AND** the text area `description_en` has the attribute `lang="en"`
- **AND** the labels of the two fields carry no `lang` attribute

### Requirement: Messages from the fluent bundles
Error messages, status words, and permission names SHALL come from the `fluent` bundles `pl` and `en`. An error code SHALL NOT change with the UI language. The values that a message shows as words SHALL also come from the bundles. These values are a field reason, a missing entity, a password reason, a task stage, and an operation name. Only data SHALL stay data, for example a login, a code, or a request id (R10, R36, A17).

#### Scenario: Error message in English
- **GIVEN** the cookie `recipe_lang` set to `en`
- **WHEN** an Operator sends a request that needs `ParameterEdit`
- **THEN** the error fragment shows the English message of `E-WEB-002`
- **AND** the error fragment shows the code `E-WEB-002`

#### Scenario: Missing user in Polish
- **GIVEN** the Administrator `akowal` with an open session and the UI language `pl`
- **WHEN** `akowal` sends an HTMX request for `GET /users/nobody`
- **THEN** the server returns `404 E-WEB-005`
- **AND** the fragment shows `Użytkownik nobody` and no English word

### Requirement: English UI text style
English UI strings and English error message templates SHALL follow the structural ASD-STE100 rules of `reliability.md` section 8 (R40).

#### Scenario: English error template
- **GIVEN** the English message template of `E-WEB-005`
- **WHEN** a user sends a request for `GET /recipes/R-999`
- **THEN** the error fragment shows the English template with the recipe reference
- **AND** the template has at most 25 words and no semicolon

### Requirement: Security headers on every response
Every response SHALL carry the headers `Content-Security-Policy`, `X-Content-Type-Options`, `Referrer-Policy`, `Strict-Transport-Security`, and `X-Request-Id` with the values of `api.md` section 9. Every response except `GET /static/*` SHALL carry `Cache-Control: no-store` (R38, R39).

#### Scenario: Headers of the login page
- **GIVEN** a browser without a session
- **WHEN** a user sends `GET /login`
- **THEN** the response sets `Content-Security-Policy` to the value of `api.md` section 9
- **AND** the response sets `Cache-Control` to `no-store`
- **AND** the response sets `X-Request-Id` to the `request_id` of the request

#### Scenario: Headers of an error response
- **GIVEN** a browser without a session
- **WHEN** a user sends `GET /users`
- **THEN** the response carries the same security headers as a success response

### Requirement: Static files
`GET /static/*` SHALL serve the vendored HTMX 2 file, the style sheets `app.css` and `components.css`. It SHALL also serve the scripts `row-version.js`, `tree.js`, and `request-state.js`, and the 8 font files of `design-system.md` section 5. Each file SHALL be unchanged and SHALL have its media type. The route SHALL need no session. The response SHALL set `Cache-Control: public, max-age` with the value of `static.cache_max_age` (R39).

#### Scenario: HTMX file without a session
- **GIVEN** a browser without a session
- **WHEN** a user sends `GET /static/htmx.min.js`
- **THEN** the server returns `200` with the file
- **AND** the response sets `Cache-Control` to `public, max-age=86400`

#### Scenario: Style sheet without a session
- **GIVEN** a browser without a session
- **WHEN** a user sends `GET /static/app.css`
- **THEN** the server returns `200` with the media type `text/css; charset=utf-8` and `X-Content-Type-Options: nosniff`
- **AND** the response sets `Cache-Control` to `public, max-age=86400`

#### Scenario: Component style sheet without a session
- **GIVEN** a browser without a session
- **WHEN** a user sends `GET /static/components.css`
- **THEN** the server returns `200` with the media type `text/css; charset=utf-8` and `X-Content-Type-Options: nosniff`

#### Scenario: Font file
- **GIVEN** a browser without a session
- **WHEN** a user sends `GET /static/fonts/atkinson-hyperlegible-mono-latin-400-normal.woff2`
- **THEN** the server returns `200` with the media type `font/woff2`
- **AND** the SHA-256 of the body equals the hash that `recipe-web` records for the file

#### Scenario: Unknown static file
- **GIVEN** a browser without a session
- **WHEN** a user sends `GET /static/fonts/other.woff2`
- **THEN** the server returns `404` with `Cache-Control: no-store`

#### Scenario: Request state script without a session
- **GIVEN** a browser without a session
- **WHEN** a user sends `GET /static/request-state.js`
- **THEN** the server returns `200` with a JavaScript media type and `X-Content-Type-Options: nosniff`

### Requirement: HTMX configuration of every page
Every full page SHALL hold the `htmx-config` meta tag with the values of `api.md` section 9. The values SHALL include `timeout` with the value of `htmx.request_timeout` in milliseconds. Its `<body>` SHALL have `hx-history="false"`. With a session, the `<body>` of every full page except the full error page SHALL also have `hx-headers` with the header `X-CSRF-Token`. Every full page SHALL hold the template `#transport-error` outside `main` (R38, R39).

#### Scenario: Profile page
- **GIVEN** the Operator `mhoffman` with an open session
- **WHEN** an Operator sends `GET /me`
- **THEN** the page holds the `htmx-config` meta tag with `"allowEval":false`, `"selfRequestsOnly":true`, and `"timeout":45000`
- **AND** the `<body>` holds `hx-history="false"` and the CSRF token of the session in `hx-headers`

#### Scenario: Full error page with a session
- **GIVEN** the Administrator `akowal` with an open session
- **WHEN** `akowal` sends `GET /users/nobody` without the header `HX-Request`
- **THEN** the page holds the `htmx-config` meta tag and `hx-history="false"`
- **AND** the `<body>` holds no `hx-headers`

#### Scenario: Template of the lost answer
- **GIVEN** a browser without a session
- **WHEN** a user sends `GET /login`
- **THEN** the page holds `<template id="transport-error">` after `main`

### Requirement: No inline script or style
A page SHALL load only the scripts `htmx.min.js`, `row-version.js`, `tree.js`, and `request-state.js` from `/static/`. `row-version.js` SHALL have at most 20 lines, `tree.js` SHALL have at most 50 lines, and `request-state.js` SHALL have at most 40 lines. A page SHALL hold no inline script, no `<style>` element, no `style` attribute, and no inline event handler (R38).

#### Scenario: User list page
- **GIVEN** the Administrator `akowal` with an open session
- **WHEN** `akowal` sends `GET /users`
- **THEN** the page holds no `<script>` element without a `src` attribute
- **AND** the page holds no `style` attribute and no attribute that starts with `on`

#### Scenario: Scripts of a page
- **GIVEN** the Operator `mhoffman` with an open session
- **WHEN** `mhoffman` sends `GET /recipes`
- **THEN** the page loads `/static/htmx.min.js`, `/static/row-version.js`, `/static/tree.js`, and `/static/request-state.js` and no other script
- **AND** the page holds no `<style>` element

#### Scenario: Size of the scripts
- **GIVEN** the files `static/row-version.js`, `static/tree.js`, and `static/request-state.js`
- **WHEN** the script test counts their lines
- **THEN** `row-version.js` has at most 20 lines, `tree.js` has at most 50 lines, and `request-state.js` has at most 40 lines

### Requirement: Escaped user text
A page SHALL show every text from a request or from the database as escaped text. User text SHALL NOT appear inside an `hx-*` attribute or a `<script>` element. A template SHALL use no unescaped output except `|safe` on a literal (R38).

#### Scenario: Script in a search field
- **GIVEN** the Administrator `akowal` with an open session
- **WHEN** `akowal` sends `GET /users` with `q` set to `<script>alert(1)</script>`
- **THEN** the response holds no `<script>alert(1)</script>` element
- **AND** the response holds the escaped text or the error `E-WEB-008`

#### Scenario: Quote in the field next of the language switch
- **GIVEN** the Operator `mhoffman` with an open session
- **WHEN** `mhoffman` sends `GET /me?x='onfocus=alert(1)'`
- **THEN** the field `next` of the language switch holds `/me?x=&#39;onfocus=alert(1)&#39;`
- **AND** each `<svg` of the response equals the fixed tag of its icon in `design-system.md` section 8

#### Scenario: Unescaped output in a template
- **GIVEN** every template and every Rust source of `recipe-web`
- **WHEN** the template test reads them
- **THEN** no template uses `{% filter safe %}`, an `escape` filter with an argument, or `|safe` on a variable
- **AND** no Rust source sets `escape` or `ext` on a template or wraps a value in `Safe`, `HtmlSafe`, or `MaybeSafe`

### Requirement: Login redirect
A successful `POST /login` SHALL set the session cookie and return `303` to `/recipes`. When the user has `must_change_password`, it SHALL return `303` to `/me/password` (R21, R39).

#### Scenario: Login with a one-time password
- **GIVEN** the user `mhoffman` with `must_change_password` set to true
- **WHEN** a user sends `POST /login` with the login `mhoffman` and the right password
- **THEN** the server returns `303` to `/me/password`
- **AND** the response sets the cookie `__Host-recipe_session`

### Requirement: Password change response
A successful HTMX `POST /me/password` SHALL return `200` with the header `HX-Refresh: true` and the new session cookie. A request without `HX-Request` SHALL get `303` to `/recipes` with the new session cookie (R21, R39).

#### Scenario: Password change through HTMX
- **GIVEN** the Operator `mhoffman` with an open session
- **WHEN** `mhoffman` sends an HTMX `POST /me/password` with the right current password and a valid new password
- **THEN** the server returns `200` with `HX-Refresh: true`
- **AND** the response sets a new cookie `__Host-recipe_session`

### Requirement: Page layout
Every full page SHALL show, in this order, a skip link to the main content, a top bar, and `main` with the id `content`. `main` SHALL start with `#error-panel` and the status region `#toast`, then one `h1`. The top bar SHALL show the application name. Every full page SHALL link the style sheets `/static/app.css` and `/static/components.css` in this order (R39, A44).

#### Scenario: Layout of the profile page
- **GIVEN** the Operator `mhoffman` with an open session
- **WHEN** `mhoffman` sends `GET /me`
- **THEN** the page holds a skip link to `#content` before the top bar
- **AND** `main` with the id `content` holds `#error-panel`, `#toast`, and one `h1` in this order
- **AND** the page links the style sheet `/static/app.css` and then the style sheet `/static/components.css`

#### Scenario: Layout of a full error page
- **GIVEN** the Administrator `akowal` with an open session
- **WHEN** `akowal` sends `GET /users/nobody` without the header `HX-Request` and gets `404 E-WEB-005`
- **THEN** the page shows the top bar with the application name and no navigation link
- **AND** the page shows the error in `#error-panel`, one `h1`, and a link to `/recipes`
- **AND** the title of the page holds the code `E-WEB-005` and the application name

### Requirement: Navigation by permission
With a session, the top bar SHALL show the link to a page only when the role holds the permission of that page. The links SHALL come in the order recipes, machines, users, audit, own profile. The link to `/audit` SHALL show when the row of `ListAuditEntries` allows the role. The use case of that page SHALL still check the permission. A session with `must_change_password` SHALL show no navigation link (R28, R39, A20).

#### Scenario: Navigation of an Operator
- **GIVEN** the Operator `mhoffman` with an open session
- **WHEN** `mhoffman` sends `GET /recipes`
- **THEN** the top bar shows the links to `/recipes`, `/machines`, and `/me`
- **AND** the top bar shows no link to `/users` and no link to `/audit`
- **AND** the link to `/recipes` has `aria-current="page"`

#### Scenario: Navigation of an Administrator
- **GIVEN** the Administrator `akowal` with an open session
- **WHEN** `akowal` sends `GET /me`
- **THEN** the top bar shows the links to `/recipes`, `/machines`, `/users`, `/audit`, and `/me` in this order

#### Scenario: Navigation of a ProcessEngineer
- **GIVEN** the ProcessEngineer `anowak` with an open session
- **WHEN** `anowak` sends `GET /recipes`
- **THEN** the top bar shows the links to `/recipes`, `/machines`, `/audit`, and `/me` in this order

#### Scenario: Navigation during a password change
- **GIVEN** the user `mhoffman` with `must_change_password` set to true and an open session
- **WHEN** `mhoffman` sends `GET /me/password`
- **THEN** the top bar shows no navigation link
- **AND** the top bar shows the logout button

### Requirement: Current navigation link
The current link of the top bar SHALL have `aria-current="page"` and SHALL show a bottom border and the weight 700 (R39).

#### Scenario: Current link on a machine page
- **GIVEN** the ProcessEngineer `anowak` with an open session
- **WHEN** `anowak` sends `GET /machines`
- **THEN** the link to `/machines` has `aria-current="page"`

#### Scenario: Look of the current link
- **GIVEN** the style sheet `/static/app.css`
- **WHEN** the style test reads the rule of `[aria-current]` in the top bar
- **THEN** the rule sets a bottom border of 3 px and the weight 700

### Requirement: Language switch
Every full page that answers a `GET` request SHALL hold a form that sends `POST /lang`. The form SHALL send the CSRF token of the session, or the field `csrf_pre` on the login page. The field `next` SHALL hold the path and the query of the request. A full page that answers another method SHALL hold no switch. The login page SHALL always hold the switch with `next` set to `/login`, also as the answer of a failed `POST /login` (R10, A17).

#### Scenario: Language switch on the user list
- **GIVEN** the Administrator `akowal` with an open session
- **WHEN** `akowal` sends `GET /users?q=ab`
- **THEN** the page holds a form that sends `POST /lang` with the field `csrf_token` of the session
- **AND** the field `next` of the form holds `/users?q=ab`

#### Scenario: Language switch on the login page
- **GIVEN** a browser without a session
- **WHEN** a user sends `GET /login`
- **THEN** the page holds a form that sends `POST /lang` with the field `csrf_pre`
- **AND** the field `csrf_pre` holds the nonce of the cookie `__Host-recipe_csrf` of the same response

#### Scenario: Language switch after a failed login
- **GIVEN** a browser without a session
- **WHEN** a user sends `POST /login` with a wrong password and without the header `HX-Request`
- **THEN** the login page holds a form that sends `POST /lang` with the field `csrf_pre`
- **AND** the field `next` of the form holds `/login`

#### Scenario: No switch on the page of a one-time password
- **GIVEN** the Administrator `akowal` with an open session
- **WHEN** `akowal` sends `POST /users/anowak/reset-password` without the header `HX-Request`
- **THEN** the page shows the one-time password
- **AND** the page holds no form that sends `POST /lang`

### Requirement: Language labels
The button of each language SHALL show the name of the language in that language, with the attribute `lang` of that language. The button of the current language SHALL have `aria-current="true"` and the look of the current navigation link (R10, A17, A44).

#### Scenario: Labels on an English page
- **GIVEN** the cookie `recipe_lang` set to `en` and the Operator `mhoffman` with an open session
- **WHEN** `mhoffman` sends `GET /me`
- **THEN** the button `Polski` has `lang="pl"` and no `aria-current` attribute
- **AND** the button `English` has `lang="en"` and `aria-current="true"`

### Requirement: State shown by color, icon, and word
A version state, a user state, and the state "Archived" of a machine or a unit SHALL show a color, an icon, and a word. Each error, warning, information, and success message SHALL also show a color, an icon, and a word. No element SHALL show a state by its color alone (R39, A44).

#### Scenario: State of a draft version
- **GIVEN** the draft version 3 of the recipe `R-100` and the UI language `pl`
- **WHEN** a page shows the state of version 3
- **THEN** the page shows the icon `status-draft` and the word `Wersja robocza` next to it

#### Scenario: State of an inactive user
- **GIVEN** the Administrator `akowal` with an open session and the inactive user `anowak`
- **WHEN** `akowal` sends `GET /users` with the UI language `en`
- **THEN** the row of `anowak` shows the icon `user-inactive` and the word `Inactive`

#### Scenario: State of an archived unit
- **GIVEN** the ProcessEngineer `anowak` with an open session, the UI language `en`, and the archived machine `M12` with the archived unit `U2`
- **WHEN** `anowak` sends `GET /machines/by-id/` with the identifier of `M12`
- **THEN** the node of `U2` shows the icon `status-retired` and the word `Archived`

#### Scenario: Error fragment as one alert
- **GIVEN** the Operator `mhoffman` with an open session
- **WHEN** `mhoffman` sends an HTMX request for `GET /users` and gets `403 E-WEB-002`
- **THEN** the fragment is one element with `role="alert"`
- **AND** that element holds the icon `alert-error`, the code `E-WEB-002`, one sentence, and the reference

### Requirement: Field error lines
Each line of `field-errors.html` SHALL show the label of the field from the bundles, not the name of the form field. In an HTMX fragment and on the login page, the line SHALL link to the input of the field. A field of a row of a list SHALL show the row number in its label and SHALL link to the input of that row. The full error page SHALL show the label without a link (R36, A44).

#### Scenario: Too long current password
- **GIVEN** the Operator `mhoffman` with an open session and the UI language `en`
- **WHEN** `mhoffman` sends an HTMX `POST /me/password` with a current password of 129 characters
- **THEN** the server returns `422 E-WEB-008` with the fragment `field-errors.html`
- **AND** the line shows the label `Current password` with a link to `#current_password`

#### Scenario: Malformed member name
- **GIVEN** the ProcessEngineer `anowak` with an open session, the UI language `en`, and the machine `M7`
- **WHEN** `anowak` sends an HTMX `POST /machines/M7/types` with the name `ST_Zone` and a first member with the name `9speed`
- **THEN** the server returns `422 E-WEB-008` with the fragment `field-errors.html`
- **AND** the line shows the label `Name of member 1` with a link to `#members-0-name`

### Requirement: Contrast of the color tokens
Each row of the contrast table of `design-system.md` section 4 SHALL reach its minimum WCAG 2.2 ratio with the values of `/static/app.css`. A text pair SHALL reach 4.5:1. A pair of a UI element or an icon SHALL reach 3:1 (A44).

#### Scenario: Muted text on the canvas
- **GIVEN** the style sheet `/static/app.css`
- **WHEN** the contrast test computes the ratio of `--color-text-muted` on `--color-canvas`
- **THEN** the ratio is at least 4.5:1

#### Scenario: Edge of a draft badge
- **GIVEN** the style sheet `/static/app.css`
- **WHEN** the contrast test computes the ratio of `--color-draft-edge` on `--color-canvas`
- **THEN** the ratio is at least 3:1

### Requirement: Design tokens equal the reference
The style sheet `app.css` SHALL define each token of the token tables of `design-system.md` with the value of the table. Only `app.css` SHALL hold `:root`. A rule outside `:root` of either style sheet SHALL name no color value except `transparent`, `currentColor`, and the system colors of the forced-colors mode (R39, A44).

#### Scenario: Token of the action color
- **GIVEN** the row `--color-action` with the value `#0B4C86` in `design-system.md` section 3
- **WHEN** the token test reads `:root` of `/static/app.css`
- **THEN** `:root` sets `--color-action` to `#0B4C86`

#### Scenario: Color value outside the tokens
- **GIVEN** a rule of `/static/app.css` outside `:root`
- **WHEN** the token test reads the rule
- **THEN** the rule holds no hex color, no color function, and no named color outside the permitted list

#### Scenario: Color value in the component style sheet
- **GIVEN** a rule of `/static/components.css`
- **WHEN** the color test reads the rule
- **THEN** the rule holds no `:root`, no hex color, no color function, and no named color outside the permitted list

### Requirement: Touch targets and focus indicator
Every button, input, select, navigation item, tree node, and link outside a sentence SHALL have a minimum height and width of 44 CSS px. Adjacent targets SHALL keep a gap of at least 8 CSS px. Every focusable element SHALL show the focus ring of `design-system.md` section 7 on keyboard focus (A44).

#### Scenario: Size of a stand-alone link
- **GIVEN** the style sheet `/static/app.css`
- **WHEN** the style test reads the rule of `.target-link`
- **THEN** the rule sets `display` to `inline-flex` and `min-height` and `min-width` to `var(--target)`
- **AND** `:root` sets `--target` to `44px`

#### Scenario: Gap between adjacent targets
- **GIVEN** the style sheet `/static/app.css`
- **WHEN** the style test reads the rule of `.actions`
- **THEN** the rule sets `gap` to `var(--target-gap)`

#### Scenario: Focus ring
- **GIVEN** the style sheet `/static/app.css`
- **WHEN** the style test reads the rule of `:focus-visible`
- **THEN** the rule draws the ring with `--color-focus` and `--color-focus-ring`
- **AND** the rule sets a transparent outline for the forced-colors mode

### Requirement: Reflow of data
A data value in the mono face SHALL break inside the word when it does not fit its line. A page SHALL NOT scroll sideways at a width of 320 CSS px, except inside a table frame (A44).

#### Scenario: Long login in the top bar
- **GIVEN** the style sheet `/static/app.css`
- **WHEN** the style test reads the rule of the class `.data`
- **THEN** the rule sets `overflow-wrap` to `anywhere`

### Requirement: Texts only from the bundles
A template SHALL hold no literal word. Every UI text SHALL come from the `fluent` bundles `pl` and `en`. Each message key that a template uses SHALL exist in both bundles (R10, R40, A17).

#### Scenario: Template without a literal word
- **GIVEN** every template of `recipe-web`
- **WHEN** the text test strips the askama tags and the markup
- **THEN** no letter remains
- **AND** the attributes `aria-label`, `title`, `placeholder`, and `alt` hold no letter outside an askama expression

#### Scenario: Status word in both languages
- **GIVEN** the bundles `pl` and `en`
- **WHEN** the server loads them
- **THEN** the key `status-draft` gives `Wersja robocza` in `pl` and `Draft` in `en`

### Requirement: Recipe list page
`GET /recipes` SHALL run the query `ListRecipes`, which checks `RecipeView` first. The server SHALL render the recipe list page with the page layout. The page SHALL show `page.recipes` rows per page with the pager. A list without a recipe SHALL show the empty state sentence of the bundles (R24, R28, R39).

#### Scenario: Recipe list of an Operator
- **GIVEN** the Operator `mhoffman` with an open session and the UI language `en`
- **WHEN** `mhoffman` sends `GET /recipes`
- **THEN** the server returns `200` with the heading `Recipes`
- **AND** the page shows the empty state sentence

#### Scenario: Recipe list without a session
- **GIVEN** a browser without a session
- **WHEN** a user sends `GET /recipes`
- **THEN** the server returns `303` to `/login`

#### Scenario: Recipe list during a password change
- **GIVEN** the user `mhoffman` with `must_change_password` set to true and an open session
- **WHEN** `mhoffman` sends `GET /recipes`
- **THEN** the server returns `303` to `/me/password`

#### Scenario: Recipe list in read-only mode
- **GIVEN** the application in read-only mode and the ProcessEngineer `anowak` with an open session
- **WHEN** `anowak` sends `GET /recipes`
- **THEN** the server returns `200` with the recipe list page

#### Scenario: Second page of the recipe list
- **GIVEN** the ProcessEngineer `anowak` with an open session and 101 recipes
- **WHEN** `anowak` sends `GET /recipes?page=2`
- **THEN** the page shows one table row and a link to the first page

### Requirement: Status region
Every full page SHALL hold the status region `#toast` with `role="status"` and `aria-live="polite"`. No element inside it SHALL have a role of its own. A success message SHALL go into `#toast` with an icon and one sentence, and the same answer SHALL empty `#error-panel` (R39, A44).

#### Scenario: Status region of the login page
- **GIVEN** a browser without a session
- **WHEN** a user sends `GET /login`
- **THEN** the page holds the empty element `#toast` with `role="status"` and `aria-live="polite"`

#### Scenario: Success after an error
- **GIVEN** the Administrator `akowal` with an open session and the inactive user `anowak`
- **WHEN** `akowal` sends an HTMX `POST /users/anowak/activate` and gets `200`
- **THEN** the answer puts the icon `alert-success` and one sentence into `#toast`
- **AND** the answer empties `#error-panel`

### Requirement: Machine list page
`GET /machines` SHALL show the machine list in a table with the code, the name, and the PLC platform. The page SHALL hold a search form with `q` and `show_archived` and links to the previous and the next page. An archived machine SHALL show the state "Archived" and a link to `/machines/by-id/{uuid}`. A list without a row SHALL show the empty state (R05, R24, R39, A44).

#### Scenario: Machine list of an Operator
- **GIVEN** the Operator `mhoffman` with an open session and the live machine `M7`
- **WHEN** `mhoffman` sends `GET /machines`
- **THEN** the server returns `200` with a table row of `M7` and a link to `/machines/M7`
- **AND** the page holds no link to `/machines/new`

#### Scenario: Archived machine in the list
- **GIVEN** the ProcessEngineer `anowak` with an open session and the archived machine `M12`
- **WHEN** `anowak` sends `GET /machines?show_archived=1`
- **THEN** the row of `M12` shows the icon `status-retired` and the word of `status-archived`
- **AND** the row links to `/machines/by-id/` and the identifier of the archived `M12`

#### Scenario: Malformed page
- **GIVEN** the ProcessEngineer `anowak` with an open session
- **WHEN** `anowak` sends an HTMX request for `GET /machines?page=0`
- **THEN** the server returns `422 E-WEB-008` with the fragment `field-errors.html`

#### Scenario: Search text in the pager links
- **GIVEN** the ProcessEngineer `anowak` with an open session and 101 live machines with the name `a&b #+%Ł`
- **WHEN** `anowak` sends `GET /machines` with `q` set to `a&b #+%Ł`
- **THEN** the link to the next page holds the percent-encoded search text

### Requirement: Search text and pages of a list
A search text of spaces only SHALL count as no search text. A page after the last page of a list SHALL show the text of `list-page-after-end`. The lists are the machine list, the usage list, the recipe list, and a version list. The page SHALL also show a link to the first page (R24, R39).

#### Scenario: Search text of spaces only
- **GIVEN** the ProcessEngineer `anowak` with an open session and the live machine `M7`
- **WHEN** `anowak` sends `GET /machines` with `q` set to three spaces
- **THEN** the server returns `200` with a table row of `M7`

#### Scenario: Page after the end
- **GIVEN** the ProcessEngineer `anowak` with an open session and one live machine
- **WHEN** `anowak` sends `GET /machines?page=2`
- **THEN** the page shows the text of `list-page-after-end` and a link to `/machines?page=1`
- **AND** the page shows no text of the empty state

#### Scenario: Recipe list after the end
- **GIVEN** the ProcessEngineer `anowak` with an open session and the recipe `R-100`
- **WHEN** `anowak` sends `GET /recipes?page=2`
- **THEN** the page shows the text of `list-page-after-end` and a link to `/recipes?page=1`

### Requirement: Content answer of a page
An HTMX request with `HX-Target: content` to a plant page or a recipe page SHALL get the content of `main` and the `title` element. The answer SHALL also update the field `next` of the language switch out of band. The `h1` of that answer SHALL have `tabindex="-1"` and `autofocus`. A request without `HX-Request` SHALL get the full page (R39, A44).

#### Scenario: Machine page after a create
- **GIVEN** the ProcessEngineer `anowak` with an open session and the live machine `M7`
- **WHEN** `anowak` sends an HTMX request for `GET /machines/M7` with `HX-Target: content`
- **THEN** the answer holds the `title` element, `#error-panel`, `#toast`, and the `h1` with `autofocus`
- **AND** the answer holds no top bar and no skip link
- **AND** the answer holds the field `next` with the value `/machines/M7` and `hx-swap-oob="true"`

#### Scenario: Version page after a create
- **GIVEN** the ProcessEngineer `anowak` with an open session and the draft version 3 of the recipe `R-100`
- **WHEN** `anowak` sends an HTMX request for `GET /recipes/R-100/versions/3` with `HX-Target: content`
- **THEN** the answer holds the `title` element and the `h1` with `autofocus`
- **AND** the answer holds no top bar

### Requirement: Machine form and creation
`GET /machines/new` SHALL show a form with the code, the name, and a list of the five PLC platforms. The form SHALL also show the data block number and the name of the main unit. The list SHALL start with an empty option, so the user chooses the platform. A valid `POST /machines` SHALL answer `303` to `/machines/{code}`. A rejected HTMX `POST /machines` SHALL put the error into `#error-panel` and SHALL keep the form with the input of the user (R05, R06, R15, R39, A48).

#### Scenario: Create an S7-300 machine
- **GIVEN** the ProcessEngineer `anowak` with an open session and no live machine `M7`
- **WHEN** `anowak` sends `POST /machines` with the code `M7`, the name `Press line`, the platform `S7_300`, the data block number `10`, and the main unit name `Infeed`
- **THEN** the server returns `303` to `/machines/M7`

#### Scenario: S7-300 machine without a number
- **GIVEN** the ProcessEngineer `anowak` with an open session and no live machine `M7`
- **WHEN** `anowak` sends an HTMX `POST /machines` with the code `M7`, the platform `S7_300`, and an empty field `db_number`
- **THEN** the server returns `422 E-PLT-050` with `HX-Retarget: #error-panel`
- **AND** the answer keeps the input of `anowak` and shows a line under the field `db_number`

#### Scenario: Code of a live machine
- **GIVEN** the ProcessEngineer `anowak` with an open session and the live machine `M7`
- **WHEN** `anowak` sends an HTMX `POST /machines` with the code `M7`
- **THEN** the server returns `409 E-PLT-001` with `HX-Retarget: #error-panel`

#### Scenario: No platform chosen
- **GIVEN** the ProcessEngineer `anowak` with an open session
- **WHEN** `anowak` sends `GET /machines/new`
- **THEN** the first option of the platform list has an empty value and is selected
- **AND** an HTMX `POST /machines` with an empty `plc_platform` gets `422 E-WEB-008` with a line for the platform field

### Requirement: Data block hint of the machine form
The machine form of `GET /machines/new` SHALL show a hint under the data block field. The hint SHALL say that only `S7_300`, `S7_1200`, and `S7_1500` use it (R15, A48).

#### Scenario: Hint of the data block field
- **GIVEN** the ProcessEngineer `anowak` with an open session
- **WHEN** `anowak` sends `GET /machines/new`
- **THEN** the hint of the data block field names `S7_300`, `S7_1200`, and `S7_1500`

### Requirement: Machine page
`GET /machines/{code}` SHALL show the code, the name, the PLC platform, the unit tree, and a link to the type list of the live machine. A user with `MachineManage` SHALL also get the rename form, the data block form, and the archive form inside a `details` element. The page of an archived machine SHALL show the state "Archived", no form, and no link to a route under `/machines/{code}` (R05, R06, R15, A32, A48).

#### Scenario: Page of a live machine for an Operator
- **GIVEN** the Operator `mhoffman` with an open session and the live machine `M7`
- **WHEN** `mhoffman` sends `GET /machines/M7`
- **THEN** the page shows `M7`, its name, its platform, and its unit tree
- **AND** the page holds no form that sends a `POST` request to `/machines/M7`

#### Scenario: Page of an archived machine
- **GIVEN** the ProcessEngineer `anowak` with an open session and the archived machine `M12`
- **WHEN** `anowak` sends `GET /machines/by-id/` with the identifier of `M12`
- **THEN** the page shows the large badge with the word of `status-archived`
- **AND** the page holds no form and no link to `/machines/M12` or to a path under it

#### Scenario: Live machine by identifier
- **GIVEN** the ProcessEngineer `anowak` with an open session and the live machine `M7`
- **WHEN** `anowak` sends `GET /machines/by-id/` with the identifier of `M7`
- **THEN** the page shows no badge of `status-archived`
- **AND** the page holds the rename form of `M7`

#### Scenario: Malformed machine code
- **GIVEN** the Operator `mhoffman` with an open session
- **WHEN** `mhoffman` sends `GET /machines/X7`
- **THEN** the server returns `404 E-WEB-005`

### Requirement: Data block number on the machine page
The page `GET /machines/{code}` of a machine on `S7_300`, `S7_1200`, or `S7_1500` SHALL show the data block number of the machine (R15, A48).

#### Scenario: Data block number of an S7 machine
- **GIVEN** the Operator `mhoffman` with an open session and the live machine `M5` with the platform `S7_1500` and the data block number `100`
- **WHEN** `mhoffman` sends `GET /machines/M5`
- **THEN** the facts of the machine show the data block number `100`

### Requirement: Rename and archive of a machine
The rename input SHALL hold the current name. A valid HTMX rename SHALL answer `200` with the new name, a success message in `#toast`, and an empty `#error-panel`. A valid archive SHALL answer `303` to `/machines?archived={code}`. The list page SHALL then show the success message of the archive in `#toast` (R05, R39, A14).

#### Scenario: Rename form with the current name
- **GIVEN** the ProcessEngineer `anowak` with an open session and the live machine `M7` with the name `Press line`
- **WHEN** `anowak` sends `GET /machines/M7`
- **THEN** the input `machine_name` of the rename form has the value `Press line`

#### Scenario: Rename through HTMX
- **GIVEN** the ProcessEngineer `anowak` with an open session and the live machine `M7`
- **WHEN** `anowak` sends an HTMX `POST /machines/M7/rename` with the name `Press line 2`
- **THEN** the server returns `200` with the name `Press line 2`
- **AND** the answer puts the icon `alert-success` into `#toast` and empties `#error-panel`

#### Scenario: Archive with a message
- **GIVEN** the ProcessEngineer `anowak` with an open session, the UI language `en`, and the live machine `M7` without a draft
- **WHEN** `anowak` sends `POST /machines/M7/archive`
- **THEN** the server returns `303` to `/machines?archived=M7`
- **AND** the page `GET /machines?archived=M7` shows the sentence `Machine M7 is archived.` in `#toast`

#### Scenario: Archive with a live draft
- **GIVEN** the ProcessEngineer `anowak` with an open session and the machine `M7` with a recipe that has a live draft
- **WHEN** `anowak` sends an HTMX `POST /machines/M7/archive`
- **THEN** the server returns `409 E-PLT-011` with `HX-Retarget: #error-panel`

### Requirement: Unit tree with the disclosure pattern
The unit tree SHALL be nested lists inside `.tree` with no `role="tree"` and no `role="treeitem"`. Each node with children SHALL be a `button` with `aria-expanded` and `aria-controls`, next to a link to the unit page. The server SHALL render the whole tree, and the nodes of depth 1 and 2 SHALL be expanded. The targets of a row SHALL keep a gap of 8 CSS px. Two rows SHALL keep a gap of 16 CSS px (R06, R07, R39, A30, A44).

#### Scenario: Tree of five levels and 64 units
- **GIVEN** the Operator `mhoffman` with an open session and the machine `M7` with 64 live units down to `U1.2.4.5.6`
- **WHEN** `mhoffman` sends an HTMX request for `GET /machines/M7/units`
- **THEN** the fragment holds 64 links to a unit page
- **AND** the groups of the nodes of depth 3 and 4 have the attribute `hidden`
- **AND** each `aria-controls` names a group with the prefix `tree-group-` and a UUID

#### Scenario: Links to add a sub-unit
- **GIVEN** the ProcessEngineer `anowak` with an open session and the machine `M7` with the unit `U1.2.4.5.6`
- **WHEN** `anowak` sends `GET /machines/M7`
- **THEN** the node `U1.2.4.5` holds a link to `/machines/M7/units/new?parent_path=U1.2.4.5`
- **AND** the node `U1.2.4.5.6` holds no such link

#### Scenario: Tree of an Operator
- **GIVEN** the Operator `mhoffman` with an open session and the machine `M7`
- **WHEN** `mhoffman` sends `GET /machines/M7`
- **THEN** the tree holds no link to `/machines/M7/units/new`

#### Scenario: Gap in a row of the tree
- **GIVEN** the style sheet `/static/components.css`
- **WHEN** the style test reads the rule of a tree row
- **THEN** the rule sets `display` to `flex` and `gap` to `var(--target-gap)`

### Requirement: Addition of a unit
The link "Add a sub-unit" SHALL load the unit form into `#unit-form` with an `h2` that names the parent and `autofocus` on its first input. A valid HTMX `POST /machines/{code}/units` SHALL answer `200` with the whole tree, the ancestors of the new unit expanded, and `autofocus` on its link. The answer SHALL put a success message into `#toast` and SHALL empty `#error-panel` (R06, R07, R39, A30).

#### Scenario: Unit form in the machine page
- **GIVEN** the ProcessEngineer `anowak` with an open session and the machine `M7` with the unit `U1.2`
- **WHEN** `anowak` sends an HTMX request for `GET /machines/M7/units/new?parent_path=U1.2` with `HX-Target: unit-form`
- **THEN** the answer is the form with an `h2` that holds `U1.2`
- **AND** the first input of the form has the attribute `autofocus`

#### Scenario: New unit on level 5
- **GIVEN** the ProcessEngineer `anowak` with an open session and the machine `M7` with the unit `U1.2.4.5`
- **WHEN** `anowak` sends an HTMX `POST /machines/M7/units` with the parent `U1.2.4.5`, the segment `6`, and a name
- **THEN** the server returns `200` with the tree
- **AND** the link of `U1.2.4.5.6` has the attribute `autofocus` and its ancestors are expanded

#### Scenario: Sixth level
- **GIVEN** the ProcessEngineer `anowak` with an open session and the machine `M7` with the unit `U1.2.4.5.6`
- **WHEN** `anowak` sends an HTMX `POST /machines/M7/units` with the parent `U1.2.4.5.6` and the segment `7`
- **THEN** the server returns `422 E-PLT-002` with `HX-Retarget: #error-panel`
- **AND** the system creates no unit

#### Scenario: All numbers in use
- **GIVEN** the ProcessEngineer `anowak` with an open session and the machine `M7` with 64 units under `U1`
- **WHEN** `anowak` sends `GET /machines/M7/units/new?parent_path=U1`
- **THEN** the form shows the sentence of `unit-no-free-segment` and no create button

#### Scenario: Proposed segment saves in the browser
- **GIVEN** the ProcessEngineer `anowak` on the machine page of `M7` with the units `U1` and `U1.1`
- **WHEN** `anowak` follows "Add a sub-unit" of `U1`, types a name, and sends the form with the proposed segment
- **THEN** the tree shows the new unit `U1.2`
- **AND** `#error-panel` is empty

### Requirement: Parent of the unit form
The unit form SHALL keep the parent of its link in a hidden field `parent_path` and SHALL show no control that changes the parent. The form SHALL show the parent in a text line. The `h2`, the text line, the proposed segment, and the sent parent SHALL always name the same parent. The form of an error answer SHALL keep that parent. An error of the parent SHALL stand in the summary without a link (R06, R07, R39, ADR-0041).

#### Scenario: Fixed parent of a sub-unit form
- **GIVEN** the ProcessEngineer `anowak` with an open session and the machine `M7` with the units `U1` and `U1.1`
- **WHEN** `anowak` sends `GET /machines/M7/units/new?parent_path=U1`
- **THEN** the form holds a hidden field `parent_path` with the value `U1` and no `select`
- **AND** the `h2` and the text line of the parent hold `U1`, and the segment input holds `2`

#### Scenario: Fixed parent of a main unit form
- **GIVEN** the ProcessEngineer `anowak` with an open session and the machine `M7` with the main unit `U1`
- **WHEN** `anowak` sends `GET /machines/M7/units/new`
- **THEN** the form holds a hidden field `parent_path` with an empty value and no `select`
- **AND** the `h2` names a new main unit, the text line shows the text of `unit-parent-none`, and the segment input holds `2`

#### Scenario: Error answer keeps the parent
- **GIVEN** the ProcessEngineer `anowak` with an open session and the machine `M7` with the units `U1` and `U1.1`
- **WHEN** `anowak` sends an HTMX `POST /machines/M7/units` with the parent `U1`, the segment `1`, and a name
- **THEN** the server returns `409 E-PLT-004` with `HX-Retarget: #error-panel`
- **AND** the form of the answer holds the hidden field `parent_path` with the value `U1`

#### Scenario: Plain error page keeps the parent
- **GIVEN** the ProcessEngineer `anowak` with an open session and the machine `M7` with the unit `U1`
- **WHEN** `anowak` sends `POST /machines/M7/units` without `HX-Request` with the parent `U1`, the segment `2`, and an empty name
- **THEN** the page holds the form with the text line of the parent `U1` and the hidden field `parent_path` with the value `U1`

#### Scenario: Parent error without a link
- **GIVEN** the ProcessEngineer `anowak` with an open session and the machine `M7` with the archived unit `U2`
- **WHEN** `anowak` sends an HTMX `POST /machines/M7/units` with the parent `U2`, the parent `U9` that does not exist, or the parent `U1..2`
- **THEN** the summary of `#error-panel` holds the line of `E-PLT-009`, `E-PLT-003`, or `E-PLT-017`
- **AND** the answer holds no `href="#parent_path"`

### Requirement: Places of the unit form
The machine page and the full page of `GET /machines/{code}/units` SHALL both hold `#unit-tree` and an empty `#unit-form`. So the link "Add a sub-unit" works on both pages (R06, R39).

#### Scenario: Add link on the full tree page
- **GIVEN** the ProcessEngineer `anowak` with an open session and the machine `M7`
- **WHEN** `anowak` sends `GET /machines/M7/units` without `HX-Request`
- **THEN** the page holds `#unit-tree` with the tree and an empty `#unit-form`
- **AND** the node `U1.2` holds a link to `/machines/M7/units/new?parent_path=U1.2`

### Requirement: Unit page
`GET /machines/{code}/units/{path}` SHALL show the path, the name, the parent, and the live children of the unit. A user with `UnitManage` SHALL also get the rename form and the archive form inside a `details` element with the count of the sub-units. A valid archive SHALL answer `303` to `/machines/{code}?archived={path}`, and the machine page SHALL then show the success message (R06, R07, A14, A30).

#### Scenario: Path with six segments
- **GIVEN** the Operator `mhoffman` with an open session and the machine `M7`
- **WHEN** `mhoffman` sends `GET /machines/M7/units/U1.2.4.5.6.7`
- **THEN** the server returns `422 E-PLT-002`

#### Scenario: Archive with a message
- **GIVEN** the ProcessEngineer `anowak` with an open session, the UI language `en`, and the unused unit `U1.2` of the machine `M7`
- **WHEN** `anowak` sends `POST /machines/M7/units/U1.2/archive`
- **THEN** the server returns `303` to `/machines/M7?archived=U1.2`
- **AND** the page `GET /machines/M7?archived=U1.2` shows the sentence `Unit U1.2 and its sub-units are archived.` in `#toast`

#### Scenario: Archive of the last main unit
- **GIVEN** the ProcessEngineer `anowak` with an open session and the machine `M7` with the one main unit `U1`
- **WHEN** `anowak` sends an HTMX `POST /machines/M7/units/U1/archive`
- **THEN** the server returns `409 E-PLT-008` with `HX-Retarget: #error-panel`

### Requirement: UDT form with member rows
`GET /machines/{code}/types/new` SHALL accept only the field `from`. The form SHALL hold the name of the struct and one `fieldset` of member rows. Each row SHALL hold a name input and a type input with a `datalist` of the types of the machine. The button "Add a member" SHALL send the form with `POST` and `add=1`. The server SHALL then create no type and SHALL show the sent rows and one more empty row (R13, R39).

#### Scenario: Add a member row
- **GIVEN** the ProcessEngineer `anowak` with an open session and the machine `M7`
- **WHEN** `anowak` sends `POST /machines/M7/types` with the name `ST_Zone`, the row `temp_max` with `REAL`, and `add=1`
- **THEN** the server returns `200` with the name `ST_Zone`, the row `temp_max` with `REAL`, and one empty row
- **AND** the first input of the empty row has the id `members-1-name` and the attribute `autofocus`
- **AND** the machine `M7` holds no struct `ST_Zone`

#### Scenario: Form of a new revision
- **GIVEN** the ProcessEngineer `anowak` with an open session and the struct `ST_Zone@2` of the machine `M7`
- **WHEN** `anowak` sends `GET /machines/M7/types/new?from=ST_Zone`
- **THEN** the form holds the members of `ST_Zone@2`
- **AND** the form sends `POST /machines/M7/types/ST_Zone/revisions`

#### Scenario: CSRF token in the query
- **GIVEN** the ProcessEngineer `anowak` with an open session and the machine `M7`
- **WHEN** `anowak` sends `GET /machines/M7/types/new?csrf_token=abc`
- **THEN** the server returns `422 E-WEB-008`

### Requirement: Create button of the UDT form
The button that creates the type or the revision SHALL be the first submit button of the form in the DOM order. The button "Add a member" SHALL come after it. Enter in an input SHALL therefore send the form without `add` at every count of rows (R13, R39, A44).

#### Scenario: Create button first at each count of rows
- **GIVEN** the ProcessEngineer `anowak` with an open session and the machine `M7`
- **WHEN** the server renders the UDT form with 1 row, with 1023 rows, or with 1024 rows
- **THEN** the first submit button of the form is the button that creates the type
- **AND** the form with 1 row and the form with 1023 rows have the button "Add a member" after it

#### Scenario: Create button first in the revision form
- **GIVEN** the ProcessEngineer `anowak` with an open session and the struct `ST_Zone@2` of the machine `M7`
- **WHEN** `anowak` sends `GET /machines/M7/types/new?from=ST_Zone`
- **THEN** the first submit button of the form is the button that creates the revision

#### Scenario: Enter in the form of a new type in the browser
- **GIVEN** the ProcessEngineer `anowak` with the UDT form of the machine `M7`, the name `ST_Enter`, and 1, 1023, or 1024 rows of `INT`
- **WHEN** `anowak` presses Enter in the name input of the struct or in a type input of a row
- **THEN** the browser sends exactly one `POST` and that `POST` has no field `add`
- **AND** the machine `M7` holds the struct `ST_Enter@1` with that count of members

#### Scenario: Enter in a revision form in the browser
- **GIVEN** the ProcessEngineer `anowak` with the revision form of `ST_Zone@2` of the machine `M7` with 1, 1023, or 1024 rows
- **WHEN** `anowak` changes the type of one row and presses Enter in a type input
- **THEN** the browser sends exactly one `POST` and that `POST` has no field `add`
- **AND** the machine `M7` holds the struct `ST_Zone@3`

### Requirement: Refusal of an unchanged revision in the UDT form
The `plant-model` requirement "Struct revision" states the refusal of an unchanged revision with `E-PLT-052`. For that refusal the server SHALL return `409`. The server SHALL show the error in the summary of `#error-panel` with no field line. The form SHALL keep the sent rows (R06, R07, ADR-0159).

#### Scenario: Unchanged revision refused
- **GIVEN** the ProcessEngineer `anowak` with an open session and the struct `ST_Zone@2` of the machine `M7`
- **WHEN** `anowak` sends `POST /machines/M7/types/ST_Zone/revisions` with the members of `ST_Zone@2` in the same order
- **THEN** the server returns `409 E-PLT-052` with `HX-Retarget: #error-panel`
- **AND** the summary of `#error-panel` holds the error with no field line
- **AND** the form keeps the sent rows

#### Scenario: Revision with a changed member case
- **GIVEN** the ProcessEngineer `anowak` with an open session and the struct `ST_Zone@2` with the member `temp_max`
- **WHEN** `anowak` sends the revision with the member `Temp_Max` and the same type
- **THEN** the server returns `303` to the type page of `ST_Zone@3`

### Requirement: UDT definition through the form
A valid `POST /machines/{code}/types` SHALL answer `303` to the type page. A valid `POST /machines/{code}/types/{name}/revisions` SHALL answer `303` to the type page of the new revision. The server SHALL skip a member row with an empty name and an empty type. A rejected HTMX request SHALL put the error into `#error-panel` and SHALL keep the form (R13, R19, R39).

#### Scenario: Struct with a cycle
- **GIVEN** the ProcessEngineer `anowak` with an open session and the machine `M7` that holds two structs that contain each other
- **WHEN** `anowak` sends an HTMX `POST /machines/M7/types` with a member of one of the two structs
- **THEN** the server returns `422 E-PLT-005` with `HX-Retarget: #error-panel`
- **AND** the system creates no type

#### Scenario: Duplicate member name
- **GIVEN** the ProcessEngineer `anowak` with an open session and the machine `M7`
- **WHEN** `anowak` sends an HTMX `POST /machines/M7/types` with two members of the name `speed`
- **THEN** the server returns `422 E-PLT-021` with `HX-Retarget: #error-panel`

#### Scenario: Empty row
- **GIVEN** the ProcessEngineer `anowak` with an open session and the machine `M7`
- **WHEN** `anowak` sends `POST /machines/M7/types` with the name `ST_Zone`, the row `temp_max` with `REAL`, and one empty row
- **THEN** the server returns `303` to `/machines/M7/types/ST_Zone`
- **AND** the struct `ST_Zone@1` has one member

#### Scenario: Reserved name
- **GIVEN** the ProcessEngineer `anowak` with an open session and the machine `M7`
- **WHEN** `anowak` sends an HTMX `POST /machines/M7/types` with the name `new` and one valid member row
- **THEN** the server returns `422 E-WEB-008` with `HX-Retarget: #error-panel`
- **AND** the system creates no type

### Requirement: Type pages
`GET /machines/{code}/types` SHALL show the struct revisions in a table. `GET /machines/{code}/types/{name}` SHALL show the type tree with the disclosure pattern, the list of revisions, and a link to the usage page. A user with `UdtManage` SHALL also get a link to the form of a new revision. `GET .../usage` SHALL show each usage row with the state of its version (R13, R19, R39, A44).

#### Scenario: Type tree page
- **GIVEN** the Operator `mhoffman` with an open session and the struct `ST_Zone@2` of the machine `M7`
- **WHEN** `mhoffman` sends `GET /machines/M7/types/ST_Zone`
- **THEN** the page shows the type tree of `ST_Zone@2` and the links to the revisions 2 and 1
- **AND** the page holds no link to `/machines/M7/types/new?from=ST_Zone`

#### Scenario: Usage page with a row
- **GIVEN** the Operator `mhoffman` with an open session and the draft of the recipe `R-100` with the parameter `LINE` of the type `ST_Line@1`, which holds `ST_Zone@2`
- **WHEN** `mhoffman` sends `GET /machines/M7/types/ST_Zone/usage?rev=2`
- **THEN** the table shows a row with `R-100`, the icon `status-draft`, and the tag `LINE`

#### Scenario: Usage page without a row
- **GIVEN** the Operator `mhoffman` with an open session and the struct `ST_Zone@1` that no parameter uses
- **WHEN** `mhoffman` sends `GET /machines/M7/types/ST_Zone/usage?rev=1`
- **THEN** the page shows the empty state sentence

### Requirement: Plant forms without user data in an hx attribute
Every HTMX request of a machine, unit, or type page SHALL come from a form `action` or a link `href` with `hx-boost="true"`. Each `hx-*` attribute that a template of these pages writes SHALL hold a literal. No URL of these pages SHALL hold the CSRF token (R38).

#### Scenario: Rename form of a unit
- **GIVEN** the ProcessEngineer `anowak` with an open session and the unit `U1.2` of the machine `M7`
- **WHEN** `anowak` sends `GET /machines/M7/units/U1.2`
- **THEN** the rename form has `action="/machines/M7/units/U1.2/rename"` and `hx-boost="true"`
- **AND** no `hx-*` attribute that the page template writes holds `M7` or `U1.2`

#### Scenario: Forms that hold the token
- **GIVEN** the ProcessEngineer `anowak` with an open session and the machine `M7`
- **WHEN** `anowak` sends `GET /machines/M7/types/new`
- **THEN** each form that holds `csrf_token` sends `POST`
- **AND** no `href`, `action`, or field `next` of the page holds `csrf_token`

### Requirement: Filters of the recipe list
`GET /recipes` SHALL keep a recipe when `q` matches a part of its reference number or name with no difference of case. `machine` SHALL keep the recipes of one live machine. `status` SHALL keep a recipe with a version in that state. The rows SHALL come in the byte order of the reference number (R16, R24, R39).

#### Scenario: Search by a part of the name
- **GIVEN** the ProcessEngineer `anowak` with an open session and the recipes `R-100` "Linia A" and `R-200` "Prasa"
- **WHEN** `anowak` sends `GET /recipes?q=LINIA`
- **THEN** the page shows a row of `R-100` and no row of `R-200`

#### Scenario: Unknown state filter
- **GIVEN** the ProcessEngineer `anowak` with an open session
- **WHEN** `anowak` sends `GET /recipes?status=open`
- **THEN** the server returns `422 E-WEB-008`

### Requirement: Deleted recipes in the list
The recipe list SHALL hide a deleted recipe. `show_deleted=1` SHALL run `ListDeletedRecipes` with `RecipeRestore` and SHALL add the deleted recipes with the state Deleted. A user without `RecipeRestore` SHALL get `403 E-WEB-002` (A14, R28).

#### Scenario: Administrator shows the deleted recipes
- **GIVEN** the Administrator `admin` with an open session and the deleted recipe `R-300`
- **WHEN** `admin` sends `GET /recipes?show_deleted=1`
- **THEN** the page shows a row of `R-300` with the icon `status-retired` and the word of `status-deleted`

#### Scenario: ProcessEngineer asks for the deleted recipes
- **GIVEN** the ProcessEngineer `anowak` with an open session
- **WHEN** `anowak` sends `GET /recipes?show_deleted=1`
- **THEN** the server returns `403 E-WEB-002`
- **AND** the system writes one `security_event` of kind `forbidden`

### Requirement: Pages of a deleted recipe
An Administrator SHALL read the recipe page, the version pages, the diff pages, and the history of a deleted recipe. These pages SHALL show the state Deleted and no form. Every `GET` route with `{ref}` SHALL answer `409 E-RCP-012` to every other role. The approval preview of a deleted recipe SHALL answer `409 E-RCP-012` to every role with `VersionApprove` (A14, R24).

#### Scenario: Operator opens a deleted recipe
- **GIVEN** the Operator `mhoffman` with an open session and the deleted recipe `R-300`
- **WHEN** `mhoffman` sends `GET /recipes/R-300`
- **THEN** the server returns `409 E-RCP-012`

#### Scenario: Administrator opens a deleted recipe
- **GIVEN** the Administrator `admin` with an open session and the deleted recipe `R-300`
- **WHEN** `admin` sends `GET /recipes/R-300`
- **THEN** the server returns `200` with the state Deleted and no form

#### Scenario: History of a deleted recipe
- **GIVEN** the deleted recipe `R-300`
- **WHEN** the Administrator `admin` and the ProcessEngineer `anowak` each send `GET /recipes/R-300/history`
- **THEN** `admin` gets `200` with the history
- **AND** `anowak` gets `409 E-RCP-012`

#### Scenario: Preview of a deleted recipe
- **GIVEN** the Administrator `admin` with an open session and the deleted recipe `R-300` with the draft version 2
- **WHEN** `admin` sends `GET /recipes/R-300/versions/2/approve`
- **THEN** the server returns `409 E-RCP-012`

### Requirement: Recipe form and creation
`GET /recipes/new` SHALL show a form with a select of the live machines, the reference number, the name, and the change note. `POST /recipes` SHALL run `CreateRecipe` and SHALL answer `303` to `/recipes/{ref}/versions/1` (R16, R25).

#### Scenario: New recipe
- **GIVEN** the ProcessEngineer `anowak` with an open session and the live machine `M7`
- **WHEN** `anowak` sends `POST /recipes` with `machine_code=M7`, `reference_number=r-500`, a name, and a change note
- **THEN** the server returns `303` to `/recipes/R-500/versions/1`

#### Scenario: Unknown machine
- **GIVEN** the ProcessEngineer `anowak` with an open session
- **WHEN** `anowak` sends `POST /recipes` with `machine_code=M99`
- **THEN** the server returns `404 E-PLT-019` with the fragment `error.html`

### Requirement: Recipe page
`GET /recipes/{ref}` SHALL show the name, the machine, and the version list with `page.versions` rows per page. Each row SHALL show the number, the state badge, and the change note. The page SHALL show the rename form only to a user with `RecipeEdit` and the link New version only to a user with `VersionCreate` (R17, R24, R25).

#### Scenario: Operator reads a recipe
- **GIVEN** the Operator `mhoffman` with an open session and the recipe `R-100` with three versions
- **WHEN** `mhoffman` sends `GET /recipes/R-100`
- **THEN** the page shows three version rows with their state badges
- **AND** the page shows no form and no link to `/recipes/R-100/versions/new`

### Requirement: Rename of a recipe
`POST /recipes/{ref}/rename` SHALL run `RenameRecipe`. An HTMX request SHALL get the new name, the toast, and an empty `#error-panel`. A request without `HX-Request` SHALL get `303` to `/recipes/{ref}` (R16, R25).

#### Scenario: Rename by HTMX
- **GIVEN** the ProcessEngineer `anowak` with an open session and the recipe `R-100`
- **WHEN** `anowak` sends an HTMX `POST /recipes/R-100/rename` with a new name
- **THEN** the server returns `200` with the new name and the toast

### Requirement: Version form and creation
`GET /recipes/{ref}/versions/new` SHALL show a select of the version numbers of the recipe, the choice empty draft, and the change note. `POST /recipes/{ref}/versions` SHALL run `CreateVersion`. A `based_on` that names no version SHALL give `422 E-WEB-008` with the field `based_on` (R17, A7).

#### Scenario: Base version that does not exist
- **GIVEN** the ProcessEngineer `anowak` with an open session and the recipe `R-100` with three versions
- **WHEN** `anowak` sends `POST /recipes/R-100/versions` with `based_on=9`
- **THEN** the server returns `422 E-WEB-008` with a line for the field `based_on`

#### Scenario: Snapshot check fails
- **GIVEN** the approved version 2 of `R-100` with a parameter on the archived unit `U3`
- **WHEN** `anowak` sends `POST /recipes/R-100/versions` with `based_on=2`
- **THEN** the server returns `422 E-RCP-020` with one line for each item

### Requirement: Version page
`GET /recipes/{ref}/versions/{n}` SHALL show the header of the version and the parameter table with `page.parameters` rows grouped by unit. The rows SHALL come in the order unit path, ordinal, and tag. Only a live `Draft` and a user with `ParameterEdit` SHALL get the edit forms of parameters. The page SHALL link to the history of the recipe (R17, R24, R25, A28).

#### Scenario: Operator reads a draft
- **GIVEN** the Operator `mhoffman` with an open session and the draft version 3 of `R-100`
- **WHEN** `mhoffman` sends `GET /recipes/R-100/versions/3`
- **THEN** the page shows the parameter table and no form
- **AND** the page shows no link to `/recipes/R-100/versions/3/approve`
- **AND** the page shows a link to `/recipes/R-100/history`

#### Scenario: Large draft
- **GIVEN** the ProcessEngineer `anowak` with an open session and a draft with 4 000 parameters in 40 units
- **WHEN** `anowak` sends `GET /recipes/R-100/versions/3`
- **THEN** the page shows the parameters of one unit and a link to each unit with the count of its parameters
- **AND** the page shows at most 100 parameter rows
- **AND** the link of the shown unit has `aria-current="page"`

### Requirement: Actions of a version page
The version page SHALL show an action form or an action link when the matrix row of its command allows the role. The state of the version SHALL also permit the transition (R24, R25, R28).

#### Scenario: Administrator reads an approved version
- **GIVEN** the Administrator `admin` with an open session and the approved version 2 of `R-100`
- **WHEN** `admin` sends `GET /recipes/R-100/versions/2`
- **THEN** the page shows the retire form in a `details` element with the class `danger`
- **AND** the page shows no edit form of a parameter

### Requirement: Form of a new parameter
On a version page that holds `#parameter-rows`, the link Add a parameter SHALL load the form of a new parameter into `#parameter-form`. The button Show value fields SHALL run `GetParameterForm` with the sent type and SHALL write nothing. `AddParameter` SHALL answer the row of the new parameter and SHALL clear `#parameter-form` out of band (R08, R25).

#### Scenario: Value fields of a typed array
- **GIVEN** the ProcessEngineer `anowak` with the form of a new parameter of the draft version 3 of `R-100`
- **WHEN** `anowak` sends the form with `type=ARRAY[1..8] OF INT` and `fields=1`
- **THEN** the server returns `200` with the form and eight value rows `[1]` to `[8]` and one row `[*]`
- **AND** the draft keeps its `row_version`

#### Scenario: New parameter saved
- **GIVEN** the ProcessEngineer `anowak` with the form of a new parameter of type `INT` and a value
- **WHEN** `anowak` sends the form
- **THEN** the server returns `200` with the row of the new parameter and the header `HX-Trigger` with the new `row_version`

#### Scenario: Value fields in read-only mode
- **GIVEN** the application in read-only mode and the form of a new parameter
- **WHEN** the ProcessEngineer `anowak` sends the form with `fields=1`
- **THEN** the server returns `503 E-WEB-006`

### Requirement: Form of a new parameter after the last page
A version page after the last page of its parameter list has no `#parameter-rows`. On that page the link Add a parameter SHALL load the page form of a new parameter into `#content` and SHALL push its address. That form SHALL target `#content` and no other region of the version page (R25, R39).

#### Scenario: Add link after the last page
- **GIVEN** the ProcessEngineer `anowak` with an open session and the draft version 3 of `R-100` with 3 parameters
- **WHEN** `anowak` sends `GET /recipes/R-100/versions/3?page=2`
- **THEN** the page shows the text of a page after the last page and no `#parameter-rows`
- **AND** the link Add a parameter has `hx-target="#content"` and `hx-push-url="true"`

#### Scenario: Add link on a page with rows
- **GIVEN** the ProcessEngineer `anowak` with an open session and the draft version 3 of `R-100` with 3 parameters
- **WHEN** `anowak` sends `GET /recipes/R-100/versions/3`
- **THEN** the page holds `#parameter-rows`
- **AND** the link Add a parameter has `hx-target="#parameter-form"`

#### Scenario: Add link on the first page of an empty draft
- **GIVEN** the ProcessEngineer `anowak` with an open session and the draft version 4 of `R-100` without parameters
- **WHEN** `anowak` sends `GET /recipes/R-100/versions/4`
- **THEN** the page shows the empty text and holds `#parameter-rows`
- **AND** the link Add a parameter has `hx-target="#parameter-form"`

#### Scenario: Save from a stale page in the browser
- **GIVEN** the ProcessEngineer `anowak` on the page `/recipes/R-100/versions/3?page=2` of a draft with 3 parameters
- **WHEN** `anowak` follows Add a parameter, shows the value fields of `INT`, and saves the form with a value
- **THEN** the browser sends exactly one `POST` with `fields=1` and exactly one `POST` that saves
- **AND** the console shows no `htmx:targetError`
- **AND** the version page then lists the new parameter

### Requirement: Form rendered again keeps the input
A form that the server renders again without a write SHALL keep every sent text and the sent `row_version`. Only a `GET` of the page SHALL give a new `row_version`. A save from a stale form SHALL get `409 E-WEB-001` (R31, R39, I12).

#### Scenario: Limit mode switch after another save
- **GIVEN** the ProcessEngineer `anowak` with the edit form of `ZONES` at `row_version` 7, and a second user saved the draft at `row_version` 8
- **WHEN** `anowak` sends the form with `limit_mode=[*]`, then saves the answer form
- **THEN** the switch answer holds the sent texts and `row_version` 7
- **AND** the save gets `409 E-WEB-001`

#### Scenario: Type changed without the value fields
- **GIVEN** the edit form of `SPEED` with `value_type=INT`
- **WHEN** `anowak` sends the form with `type=REAL` and no `fields`
- **THEN** the server returns `200` with the value fields of `REAL` and a notice, and writes nothing

### Requirement: Leaf grid
The value and the limits of a parameter SHALL show in one table with one row per leaf. A row SHALL show the leaf path, the type, the value, and only the limit inputs of the class of its leaf. Each input SHALL name its row header and its column header in `aria-labelledby` (R09, R13, A36, A44).

#### Scenario: Struct in a struct
- **GIVEN** the parameter `LINE_A` of type `ST_Line@1` with the member `zone` of type `ST_Zone@2` in the draft version 3 of `R-100`
- **WHEN** the ProcessEngineer `anowak` opens the page of `LINE_A`
- **THEN** the grid shows the rows `.name`, `.speed`, `.zone.setpoint`, `.zone.hold`, and `.zone.enabled`
- **AND** the row `.zone.enabled` shows a select with TRUE and FALSE and no limit input

#### Scenario: Time leaf as an IEC literal
- **GIVEN** the leaf `.zone.hold` of type `TIME` with the stored count 90000
- **WHEN** the ProcessEngineer `anowak` opens the page of `LINE_A`
- **THEN** the value input of `.zone.hold` holds `T#1m30s`

#### Scenario: Multidimensional array
- **GIVEN** the parameter `GRID` of type `ARRAY[0..1,1..2] OF REAL`
- **WHEN** the ProcessEngineer `anowak` opens the page of `GRID`
- **THEN** the grid shows the rows `[0,1]`, `[0,2]`, `[1,1]`, and `[1,2]` and one row `[*]`

### Requirement: Limit modes of an array level
Each array level of the grid SHALL have one limit for all elements, in a row `[*]`, or a limit per element. The page SHALL show a limit per element when the limits hold an index at that level. A button of the level SHALL switch the mode and SHALL write nothing (R09, A36).

#### Scenario: Switch to a limit per element
- **GIVEN** the parameter `ZONES` of type `ARRAY[1..4] OF ST_Zone@2` with a limit 20 to 250 on `[*].setpoint`
- **WHEN** the ProcessEngineer `anowak` sends the edit form with `limit_mode=[*]`
- **THEN** the server returns `200` with limit inputs on each row `[1].setpoint` to `[4].setpoint`
- **AND** each of these inputs holds the limit 20 to 250
- **AND** the draft keeps its `row_version`

#### Scenario: Switch back with unequal limits
- **GIVEN** the limits 20 to 250 on `[1].setpoint` and 30 to 260 on `[2].setpoint`
- **WHEN** `anowak` switches the level back to one limit for all elements
- **THEN** the row `[*]` holds empty limit inputs and the page shows a notice

### Requirement: Groups of a large array
An array with more than `form.array_chunk` elements SHALL split into groups with a toggle button of the disclosure pattern, unless it stands inside a group. An array inside a group SHALL show all its rows in that group, with no toggle of its own. A group SHALL be closed, except the first group and each group with an error line. An input in a closed group SHALL submit with the form (R13, A26, A44).

#### Scenario: Array of 1 000 elements
- **GIVEN** the parameter `PROFILE` of type `ARRAY[1..1000] OF INT`
- **WHEN** the ProcessEngineer `anowak` opens the page of `PROFILE`
- **THEN** the grid shows ten groups and only the first group is open
- **AND** each toggle button has `aria-expanded` and `aria-controls`

#### Scenario: Array inside a group
- **GIVEN** the parameter `ZONE_TABLE` of type `ARRAY[1..120] OF ST_Zone`, where `ST_Zone` holds the member `profile` of type `ARRAY[1..120] OF INT`
- **WHEN** the ProcessEngineer `anowak` opens the page of `ZONE_TABLE`
- **THEN** the grid shows two groups of the outer array and no other group
- **AND** the first group holds the 120 rows of `profile` of each of its elements
- **AND** no `tbody` of the grid holds another `tbody`

### Requirement: Value fields of a form
A form SHALL send one value field per leaf with the name `value` and the leaf path. A key outside the fields of the route, a malformed leaf path, a `[*]` segment, or a duplicate key SHALL give `422 E-WEB-008`. A missing leaf or a leaf path that the type does not hold SHALL give `422 E-RCP-005`. The server SHALL keep the text of a `STRING` or `CHAR` leaf as sent (R38, A11, A43).

#### Scenario: Partial value
- **GIVEN** the parameter `SPEED_TABLE` of type `ARRAY[1..8] OF INT`
- **WHEN** the ProcessEngineer `anowak` sends the edit form without the field `value[5]`
- **THEN** the server returns `422 E-RCP-005` with a line for the leaf `[5]`

#### Scenario: Spaces in a STRING leaf
- **GIVEN** the leaf `.name` of type `STRING(20)`
- **WHEN** `anowak` sends `value.name` with the text `  Linia A  `
- **THEN** the stored value of `.name` is `  Linia A  `

#### Scenario: Flood of pairs
- **GIVEN** the parameter `SPEED` of type `INT`
- **WHEN** `anowak` sends the edit form with 100 000 extra pairs
- **THEN** the server returns `422 E-WEB-008` with one line and reads no more pairs

### Requirement: One edit form of a parameter
The parameter page of a live `Draft` SHALL hold one edit form with the definition fields, the S7 address, and the leaf grid. The form SHALL send the value and the limits together to `/definition`. The page SHALL hold no second form with a definition field of that parameter (R09, R25, I12).

#### Scenario: Value and limits in one save
- **GIVEN** the parameter `SPEED` of type `INT` with the value 1500 and the limit 0 to 3000
- **WHEN** the ProcessEngineer `anowak` sends the edit form with the value 3500 and the limit 3000 to 4000
- **THEN** the server returns `200` with the row of `SPEED` and the header `HX-Trigger`
- **AND** the answer swaps the form out of band with the saved texts and no error line

#### Scenario: Enter saves
- **GIVEN** the edit form of `SPEED`
- **WHEN** the browser sends the form for Enter in a value input
- **THEN** the request holds no field `fields` and no field `limit_mode`

### Requirement: S7 address field per platform
The parameter forms SHALL show the S7 address as a required field on `S7_300` and as an optional field on `S7_1200` and `S7_1500`. They SHALL show no S7 address field on `CodesysIec` and `TwinCat3` (R14, A31).

#### Scenario: TwinCAT machine
- **GIVEN** the ProcessEngineer `anowak` with an open session and a draft of a recipe of a live machine with the platform `TwinCat3`
- **WHEN** `anowak` opens the form of a new parameter
- **THEN** the form holds no field `s7_address`

#### Scenario: S7-300 machine
- **GIVEN** a draft of a recipe of the machine `M7` with the platform `S7_300`
- **WHEN** `anowak` opens the form of a new parameter
- **THEN** the label of the field `s7_address` holds the word for required

### Requirement: Translation rows
The parameter page SHALL show one translation row per tag of `i18n.translation_langs` and per other tag of the parameter. Each row SHALL be one form that sends `POST` to `.../translations/{lang}`. A deletion SHALL send `POST` to `.../translations/{lang}/delete` from a danger disclosure (R10, A17).

#### Scenario: Polish translation saved
- **GIVEN** the parameter `LINE` of the draft version 3 of `R-100` and `i18n.translation_langs` with `pl`
- **WHEN** the ProcessEngineer `anowak` sends the row `pl` with a name and a description
- **THEN** the server returns `200` with the translation fragment and the header `HX-Trigger`

#### Scenario: Tag outside the list
- **GIVEN** `i18n.translation_langs` with `pl`
- **WHEN** `anowak` sends `POST .../translations/de`
- **THEN** the server returns `422 E-WEB-008`

#### Scenario: Delete a tag outside the list
- **GIVEN** `i18n.translation_langs` with `pl` and the parameter `LINE` with a translation `de`
- **WHEN** `anowak` sends `POST .../translations/de/delete`
- **THEN** the server returns `200` and `LINE` has no translation `de`

### Requirement: Deletion of a parameter
`POST .../parameters/{key}/delete` SHALL run `RemoveParameter`. The form SHALL stand in a danger disclosure whose button names the tag. The answer SHALL be the parameter table of the version (R23, R25).

#### Scenario: Delete a parameter
- **GIVEN** the ProcessEngineer `anowak` with an open session and the parameter `LINE` of the draft version 3 of `R-100`
- **WHEN** `anowak` sends `POST /recipes/R-100/versions/3/parameters/<key>/delete` with the current `row_version`
- **THEN** the server returns `200` with the parameter table without `LINE`

### Requirement: Rebind form of an outdated draft
The page of a draft with outdated parameters SHALL list them with the state Outdated. It SHALL show one rebind form per outdated struct revision. The form SHALL hold one field per new member leaf of each affected parameter (R19, A29).

#### Scenario: New member without a value
- **GIVEN** the draft version 3 of `R-100` with the parameter `ZONE_1` of type `ST_Zone@2` and the newest revision `ST_Zone@3` with the new member `ramp`
- **WHEN** the ProcessEngineer `anowak` sends the rebind form without the field of `.ramp`
- **THEN** the server returns `422 E-RCP-028` with one line for `ZONE_1`

### Requirement: Bound of the rebind forms
`GetVersion` SHALL compute the new member leaves of a rebind only for a user with `ParameterEdit` on a live `Draft`. The sum of the new member fields of all rebind forms of one version page SHALL be at most `form.rebind_leaves`. Over the bound, each rebind form SHALL show the notice `rebind-too-large` and SHALL hold no field of a new member and no submit button (A26).

#### Scenario: Operator reads an outdated draft
- **GIVEN** the Operator `mhoffman` with an open session and the draft version 3 of `R-100` with the outdated parameter `ZONE_1`
- **WHEN** `mhoffman` sends `GET /recipes/R-100/versions/3`
- **THEN** the page lists `ZONE_1` with the state Outdated
- **AND** the page shows no rebind form and no field of a new member

#### Scenario: Rebind over the bound
- **GIVEN** the ProcessEngineer `anowak` with an open session and the draft version 3 of `R-100`
- **AND** the outdated parameters of the draft need 20 000 new member leaves for their rebind
- **WHEN** `anowak` sends `GET /recipes/R-100/versions/3`
- **THEN** each rebind form shows the notice `rebind-too-large` with the bound 16 384
- **AND** the page holds no field of a new member and no rebind submit button

### Requirement: Every field error at once
On the recipe and plant forms, a changed body SHALL give one line `422 E-WEB-008` and no out-of-band form. A changed body has a fault of ADR-0138 (b) to (d), for example an unknown key or a `[*]` segment. For another body, the answer SHALL list every bad field and every fault of the value and the limits in the stages of ADR-0041. `#error-panel` SHALL show at most 20 lines and the count of the others. The out-of-band form SHALL show a line under each bad input (R36, R38, R39, A44).

#### Scenario: Two bad leaves
- **GIVEN** the parameter `SPEED_TABLE` with the limit 0 to 3000 on `[*]`
- **WHEN** the ProcessEngineer `anowak` sends the edit form with 3500 in `[3]` and 4000 in `[6]`
- **THEN** the server returns `422 E-RCP-004` with two lines in `#error-panel`
- **AND** the inputs of `[3]` and `[6]` have `aria-invalid="true"` and an error line

#### Scenario: Two bad member rows of a UDT
- **GIVEN** the ProcessEngineer `anowak` with an open session and the machine `M7`
- **WHEN** `anowak` sends an HTMX `POST /machines/M7/types` with the bad member names `9a` and `9b`
- **THEN** the server returns `422 E-WEB-008` with two lines
- **AND** each member name input has an error line under it

#### Scenario: Unknown key in the add form
- **GIVEN** the ProcessEngineer `anowak` with an open session and the draft version 3 of `R-100`
- **WHEN** `anowak` sends an HTMX `POST /recipes/R-100/versions/3/parameters` with the tag `bad tag`, the unknown key `bogus`, and the value key `value[*]`
- **THEN** the server returns `422 E-WEB-008` with one line in `#error-panel`
- **AND** the answer holds no error line under an input and no out-of-band form
- **AND** the draft does not change

#### Scenario: Unknown key in the edit form
- **GIVEN** the ProcessEngineer `anowak` with an open session and the parameter `SPEED` of the draft version 3 of `R-100`
- **WHEN** `anowak` sends an HTMX `POST /recipes/R-100/versions/3/parameters/{key}/definition` of `SPEED` with the tag `bad tag`, the unknown key `bogus`, and the value key `value[*]`
- **THEN** the server returns `422 E-WEB-008` with one line in `#error-panel`
- **AND** the answer holds no error line under an input and no out-of-band form
- **AND** the parameter keeps its definition, its value, and its limits

#### Scenario: Unknown key in the UDT form
- **GIVEN** the ProcessEngineer `anowak` with an open session and the machine `M7`
- **WHEN** `anowak` sends an HTMX `POST /machines/M7/types` with the bad member names `9a` and `9b` and the unknown key `bogus`
- **THEN** the server returns `422 E-WEB-008` with one line
- **AND** no member name input has an error line under it

#### Scenario: Unknown key in the value form
- **GIVEN** the ProcessEngineer `anowak` with an open session and the parameter `SPEED` of the draft version 3 of `R-100`
- **WHEN** `anowak` sends an HTMX `POST /recipes/R-100/versions/3/parameters/{key}/value` of `SPEED` with the value `abc` and the unknown key `bogus`
- **THEN** the server returns `422 E-WEB-008` with one line in `#error-panel`
- **AND** the answer holds no error line under an input
- **AND** the parameter keeps its value

#### Scenario: Unknown key in the machine form
- **GIVEN** the ProcessEngineer `anowak` with an open session
- **WHEN** `anowak` sends an HTMX `POST /machines` with an empty machine code and the unknown key `bogus`
- **THEN** the server returns `422 E-WEB-008` with one line in `#error-panel`
- **AND** the answer holds no error line under the input `code` and no out-of-band form
- **AND** the server creates no machine

### Requirement: Refused form page and toast
A request without HTMX with field errors of a form and a session SHALL get a full page in the layout of the session. The page SHALL hold the navigation, the CSRF token in `hx-headers`, and the login. It SHALL hold the summary with a link to each input, one `h1`, and the form with the sent texts and the error lines. Every other error and a request without a session SHALL get the full error page. Every HTMX error answer SHALL end with an empty `#toast` out of band (R36, R39, A44).

#### Scenario: Refused form without HTMX
- **GIVEN** the ProcessEngineer `anowak` with an open session
- **WHEN** `anowak` sends `POST /recipes` without `HX-Request`, with an empty machine code, and with the name `Kept name`
- **THEN** the server returns `422` with the navigation, the login, and `hx-headers` with the CSRF token
- **AND** `#error-panel` holds a line that links to `#machine_code`
- **AND** the page holds one `h1` and the recipe form with the name `Kept name` and the error lines
- **AND** the application writes nothing

#### Scenario: Other error without HTMX
- **GIVEN** the Operator `mhoffman` with an open session
- **WHEN** `mhoffman` sends a form route without `HX-Request` and gets `403 E-WEB-002`
- **THEN** the answer is the full error page with no form

#### Scenario: Error answer after a success toast
- **GIVEN** the ProcessEngineer `anowak` with a success message in `#toast` of the version page
- **WHEN** `anowak` sends an HTMX save that gets `422`, `409`, or `404`
- **THEN** the answer ends with an empty `#toast` element with `hx-swap-oob="innerHTML"`
- **AND** `#toast` keeps `role="status"` and `aria-live="polite"`

### Requirement: Permission before the fields of a form route
A route of a form SHALL run the form query of its command before it reads its fields. The form query SHALL check the permission of the command that it serves. A user without that permission SHALL get `403 E-WEB-002` and one `forbidden` event that names the command, and SHALL get no form. This SHALL also apply to a field that the form does not know (R25, R28).

#### Scenario: Operator with a bad field on a form route
- **GIVEN** the Operator `mhoffman` with an open session and the recipe `R-100`
- **WHEN** `mhoffman` sends an HTMX `POST /recipes/R-100/rename` with an empty name and the unknown field `extra`
- **THEN** the server returns `403 E-WEB-002` and not `422`
- **AND** `security_event` holds one `forbidden` event that names `RenameRecipe`
- **AND** the body holds no form and no input

#### Scenario: Operator on a parameter route
- **GIVEN** the Operator `mhoffman` with an open session and the parameter `LINE` of the draft version 3 of `R-100`
- **WHEN** `mhoffman` sends `POST .../parameters/<key>/translations/pl` with the unknown field `extra`
- **THEN** the server returns `403 E-WEB-002` with no translation form
- **AND** `security_event` holds one `forbidden` event that names `UpsertTranslation`

### Requirement: Body limit per route
The four routes that carry a parameter value SHALL accept a body up to `http.body_limit.value`. The route that writes a translation SHALL accept up to `http.body_limit.translation`. Every other route SHALL accept up to `http.body_limit.form`. A larger body SHALL give `413 E-WEB-003` (R30, R38).

#### Scenario: Value of 16 384 leaves
- **GIVEN** the parameter `PROFILE` of type `ARRAY[1..16384] OF INT`
- **WHEN** the ProcessEngineer `anowak` sends the edit form with all leaves, a body of about 500 KiB
- **THEN** the server does not answer `413`

#### Scenario: Large rename
- **GIVEN** the recipe `R-100`
- **WHEN** `anowak` sends `POST /recipes/R-100/rename` with a body of 300 KiB
- **THEN** the server returns `413 E-WEB-003`

### Requirement: Recipe pages without user data in an hx attribute
Every HTMX request of a recipe, version, or parameter page SHALL come from a form `action` or a link `href` with `hx-boost="true"`. Each `hx-*` attribute of these pages SHALL hold a literal. No URL SHALL hold the CSRF token. Every form that holds `csrf_token` SHALL send `POST` (R38).

#### Scenario: Parameter page
- **GIVEN** the ProcessEngineer `anowak` with an open session and the parameter `LINE` of the draft version 3 of `R-100`
- **WHEN** `anowak` opens the page of `LINE`
- **THEN** no `hx-*` attribute of the page holds `R-100` or the key of `LINE`
- **AND** each form that holds `csrf_token` sends `POST`

### Requirement: Messages of the recipe codes
Every error code that a recipe, version, or parameter route returns SHALL have a message in the bundles `pl` and `en`. Each word of a message, such as a state, a budget, a platform, or an expected form, SHALL come from the bundles (R10, R36, R40).

#### Scenario: Value of the wrong shape in Polish
- **GIVEN** the ProcessEngineer `anowak` with the UI language `pl`
- **WHEN** `anowak` sends the text `abc` for a leaf of type `INT`
- **THEN** the line of `E-RCP-005` holds only Polish words and the data of the request

### Requirement: Approval preview page
The approval preview SHALL show the diff against the current Approved version with the limits first. With the approve form, it SHALL show the outdated parameters and the version that the approval retires. The checkbox `allow_outdated` SHALL show only with an outdated parameter. Without a blocker, the page SHALL hold one approve form with `csrf_token`, `row_version`, and `hx-sync="closest form:drop"` (R27, A29, A34, ADR-0049).

#### Scenario: Preview of a changed limit
- **GIVEN** the approved version 1 of `R-100` and its draft version 2, in which `SPEED_TABLE` has the limit row `[*]` with `min` 10 in place of 0
- **WHEN** the Administrator `admin` sends `GET /recipes/R-100/versions/2/approve`
- **THEN** the limits section shows one row of `SPEED_TABLE` with the path `[*]`
- **AND** the old limit text of the row holds the minimum `0`, and the new limit text holds the minimum `10`
- **AND** the row shows the word of `diff-state-limit` and the icon `diff-limit`
- **AND** the page names version 1 as the version that the approval retires

#### Scenario: Outdated parameter
- **GIVEN** the draft version 2 of `R-100` with the outdated parameter `ZONES`
- **WHEN** the Administrator `admin` opens the approval preview of version 2
- **THEN** the page lists `ZONES` and shows the checkbox `allow_outdated`

#### Scenario: No outdated parameter
- **GIVEN** the draft version 2 of `R-100` without an outdated parameter
- **WHEN** the Administrator `admin` opens the approval preview of version 2
- **THEN** the page shows no checkbox `allow_outdated`

### Requirement: Blockers on the approval preview
The approval preview SHALL run the checks of the approval without a write. Separation of duties can be on, and the user can be the creator or an editor of the draft. Then the page SHALL show the notice of `E-RCP-015`. For each fault of `check_for_approval`, the page SHALL show one line with the code and a link to the parameter page. With a blocker, the page SHALL show no approve form (R27, A34, A36, ADR-0049).

#### Scenario: Editor of the draft opens the preview
- **GIVEN** the draft version 2 of `R-100`, which the ProcessEngineer `anowak` edited
- **WHEN** `anowak` opens the approval preview of version 2
- **THEN** the page shows the code `E-RCP-015` and no approve form

#### Scenario: Missing limit
- **GIVEN** the draft version 2 of `R-100` with a numeric leaf of `HOLD` without a limit
- **WHEN** the ProcessEngineer `jsmith` opens the approval preview of version 2
- **THEN** the page shows a line of `E-RCP-016` that links to the parameter page of `HOLD`
- **AND** the page shows no approve form

### Requirement: Bound of the blocker list
The blocker notice of the approval preview SHALL show at most 20 lines. When more lines exist, the notice SHALL show one sentence with the count of the other lines (R30, ADR-0041, ADR-0049).

#### Scenario: More than 20 blocker lines
- **GIVEN** a draft with 25 numeric leaves without a limit
- **WHEN** the Administrator `admin` opens the approval preview of the draft
- **THEN** the page shows 20 lines of `E-RCP-016`
- **AND** the page shows one sentence with the count 5

### Requirement: Layout of the approval preview
The blockers and the notices of the approval preview SHALL stand right after the header. The blockers SHALL stand in one notice with the heading of `approve-blockers-heading` and without `role="alert"`. Without a blocker, the first item of the summary SHALL link to the heading of the approve form. In a section, only the first row of a parameter SHALL link its tag (R39, A44, ADR-0049).

#### Scenario: Blockers before the diff
- **GIVEN** the draft version 2 of `R-100`, which the ProcessEngineer `anowak` edited
- **WHEN** `anowak` opens the approval preview of version 2
- **THEN** the notice of the blockers stands before the summary of the diff
- **AND** no element of the page has `role="alert"`

#### Scenario: Link to the approve form
- **GIVEN** the draft version 2 of `R-100` without a blocker
- **WHEN** the Administrator `admin` opens the approval preview of version 2
- **THEN** the first item of the summary shows the text of `approve-go-to-form` and links to the heading of the approve form

### Requirement: Answer of the approval
`POST /recipes/{ref}/versions/{n}/approve` SHALL run `ApproveVersion`. A success SHALL answer `200` with the version page and a toast that names the approved version and the retired version. An HTMX answer SHALL set `HX-Push-Url` to the path of the version page. When the answer holds a toast, its `h1` SHALL have `aria-describedby="toast"`. An `AlreadyApproved` result SHALL answer the same page with the toast `version-already-approved` (R27, R39, I12, ADR-0049).

#### Scenario: Approval by HTMX
- **GIVEN** the draft version 2 of `R-100`, which `anowak` edited, and the approved version 1
- **WHEN** the Administrator `admin` sends an HTMX approval of version 2 with its `row_version`
- **THEN** the server returns `200` with the version page of version 2 and a toast that names versions 2 and 1
- **AND** the answer sets `HX-Push-Url` to `/recipes/R-100/versions/2`
- **AND** the `h1` of the answer has `aria-describedby="toast"`
- **AND** the audit log holds one entry `VersionApproved` of version 2

#### Scenario: Double click
- **GIVEN** an approval of version 2 of `R-100` that succeeded with `row_version` 8
- **WHEN** `admin` sends the same approval with `row_version` 8 again
- **THEN** the server returns `200` with the toast `version-already-approved`
- **AND** the audit log still holds one entry `VersionApproved` of version 2

### Requirement: Errors of the approval
The approval SHALL answer `409 E-RCP-015` to the creator and to each editor of the draft while separation of duties is on. It SHALL answer `409 E-WEB-001` with `conflict.html` for a stale `row_version` in the order of the next requirement. It SHALL answer `422 E-RCP-024` for an outdated parameter without `allow_outdated`. An Operator SHALL get `403 E-WEB-002`, and the system SHALL write one security event of kind `forbidden` (R27, R28, A29, A34).

#### Scenario: Creator approves the own draft
- **GIVEN** the draft version 2 of `R-100`, which `anowak` created
- **WHEN** `anowak` sends the approval of version 2
- **THEN** the server returns `409 E-RCP-015` and writes no audit entry

#### Scenario: Two sessions approve one draft
- **GIVEN** the approval preview of version 2 in two sessions of two Administrators
- **WHEN** the first approves and the second sends the approval with the same `row_version`
- **THEN** the second gets `200` with the toast `version-already-approved`

#### Scenario: Draft changed after the preview
- **GIVEN** the approval preview of version 2 of `R-100` with `row_version` 7 in the session of `admin`
- **WHEN** `anowak` saves a value of version 2 in a second session and `admin` then sends the approval with `row_version` 7
- **THEN** `admin` gets `409 E-WEB-001` with `conflict.html`
- **AND** version 2 stays a draft

#### Scenario: Outdated parameter without the flag
- **GIVEN** the draft version 2 of `R-100` with the outdated parameter `ZONES`
- **WHEN** `admin` sends the approval of version 2 without `allow_outdated`
- **THEN** the server returns `422 E-RCP-024` in `#error-panel`
- **AND** the summary line shows the label of `field-allow-outdated` and links to the checkbox `allow_outdated`

#### Scenario: Operator sends an approval
- **GIVEN** the Operator `mhoffman` with an open session
- **WHEN** `mhoffman` sends `POST /recipes/R-100/versions/2/approve` with a valid CSRF token
- **THEN** the server returns `403 E-WEB-002`
- **AND** the system writes one security event of kind `forbidden` that names `ApproveVersion`

### Requirement: Order of the approval errors
The approval SHALL check the draft once before its transaction, and before the compare of the `row_version`. A fault of that first check SHALL give its own code, also with a stale `row_version`. Under the locks, the approval SHALL compare the `row_version` before the second check, so a stale `row_version` there SHALL give `409 E-WEB-001` (R27, R28, A29).

#### Scenario: Stale row version and an outdated parameter
- **GIVEN** the draft version 2 of `R-100` with the outdated parameter `ZONES`, and a `row_version` that a later save made stale
- **WHEN** `admin` sends the approval of version 2 with the stale `row_version` and without `allow_outdated`
- **THEN** the server returns `422 E-RCP-024` with the line in the summary and no approve form
- **AND** the summary line has no link, and version 2 stays a draft

### Requirement: Danger actions of a version
The page of an Approved version SHALL show the retire action to a role with `VersionRetire`. The page of a live `Draft` SHALL show the discard action to a role with `VersionDiscard`. Each action SHALL be one `details` element with the class `danger`. Its form SHALL hold `csrf_token`, `row_version`, and `hx-sync="closest form:drop"`. The button SHALL name the version (R17, R39, A28, ADR-0049).

#### Scenario: ProcessEngineer reads an approved version
- **GIVEN** the ProcessEngineer `anowak` with an open session and the approved version 2 of `R-100`
- **WHEN** `anowak` sends `GET /recipes/R-100/versions/2`
- **THEN** the page shows no retire form

#### Scenario: Discard of a draft
- **GIVEN** the draft version 3 of `R-100`
- **WHEN** the ProcessEngineer `anowak` sends the discard form of version 3
- **THEN** the server returns `303` to `/recipes/R-100`
- **AND** version 3 has the state Discarded

#### Scenario: Retire with a stale row version
- **GIVEN** the approved version 2 of `R-100` with `row_version` 9
- **WHEN** `admin` sends the retire form of version 2 with `row_version` 8
- **THEN** the server returns `409 E-WEB-001` with `conflict.html`

### Requirement: Notice of a danger action
The notice of a danger action SHALL have an id that is unique in the page and no role. The danger button SHALL name its notice in `aria-describedby`. A page MAY hold more than one danger action. The id `danger-notice` SHALL occur at most once in a page. Each other notice id SHALL start with `danger-notice-` and end with a stable suffix (R39, A44, ADR-0049, ADR-0110).

#### Scenario: Notice of the retire action
- **GIVEN** the page of the approved version 2 of `R-100` for the Administrator `admin`
- **WHEN** the test reads the retire form
- **THEN** the page holds one element with the id `danger-notice`
- **AND** the retire button has `aria-describedby="danger-notice"`

#### Scenario: Two danger actions on one page
- **GIVEN** the ProcessEngineer `anowak` and the parameter `SPEED` of the draft version 3 of `R-100` with the stored translation `de`
- **WHEN** `anowak` opens the page of `SPEED`
- **THEN** the page holds one element with the id `danger-notice-parameter` and one element with the id `danger-notice-translation-de`
- **AND** the parameter delete button has `aria-describedby="danger-notice-parameter"`
- **AND** the translation delete button has `aria-describedby="danger-notice-translation-de"`
- **AND** every `id` of the page occurs once

### Requirement: Delete action of a recipe
The recipe page SHALL show the delete action to a role with `RecipeDelete` in one `details` element with the class `danger`. Its form SHALL hold `csrf_token` and `hx-sync="closest form:drop"` and no `row_version`. A success SHALL answer `303` to `/recipes` (R23, A13, ADR-0049).

#### Scenario: ProcessEngineer and a retired version
- **GIVEN** the recipe `R-100` with the retired version 1 and no approved version
- **WHEN** the ProcessEngineer `anowak` sends the delete form of `R-100`
- **THEN** the server returns `409 E-RCP-026`

#### Scenario: Administrator and an approved version
- **GIVEN** the recipe `R-100` with the approved version 2
- **WHEN** the Administrator `admin` sends the delete form of `R-100`
- **THEN** the server returns `409 E-RCP-010`

#### Scenario: Administrator and only retired versions
- **GIVEN** the recipe `R-100` with the retired version 1 and no approved version
- **WHEN** `admin` sends the delete form of `R-100`
- **THEN** the server returns `303` to `/recipes`

### Requirement: Restore from the list of deleted recipes
The list `/recipes?show_deleted=1` SHALL show a restore form in each deleted row. The form SHALL hold `csrf_token` and `hx-sync="closest form:drop"`. Its secondary button SHALL name the reference number. A success SHALL answer `303` to `/recipes/{ref}` (R26, A14, ADR-0049).

#### Scenario: Administrator restores a recipe
- **GIVEN** the Administrator `admin` with an open session and the deleted recipe `R-300` on a live machine
- **WHEN** `admin` sends the restore form of `R-300`
- **THEN** the server returns `303` to `/recipes/R-300`

#### Scenario: Restore on an archived machine
- **GIVEN** the deleted recipe `R-300` on the archived machine `M12`
- **WHEN** `admin` sends the restore form of `R-300`
- **THEN** the server returns `409 E-PLT-010`

### Requirement: Diff page
`GET /recipes/{ref}/versions/{n}/diff/{m}` SHALL show the changes from version `m` to version `n`. The sections SHALL be limits, values, definitions, translations, added parameters, and parameters only in the old version. A summary SHALL give the counts and link to each section. The table of the limits section SHALL have a border of `--color-error-edge`. Each change SHALL show the color, the icon, and the word of its state (R27, R39, A43, ADR-0050).

#### Scenario: Changed value inside a struct
- **GIVEN** the versions 1 and 2 of `R-100`, in which `LINE_A` has `.zone.setpoint` 85.5 in version 1 and 90 in version 2
- **WHEN** the Operator `mhoffman` sends `GET /recipes/R-100/versions/2/diff/1`
- **THEN** the values section shows a row of `LINE_A` with the path `.zone.setpoint`, the old text `85.5`, and the new text `90`

#### Scenario: Unknown kind
- **GIVEN** the versions 1 and 2 of `R-100`
- **WHEN** `mhoffman` sends `GET /recipes/R-100/versions/2/diff/1?kind=other`
- **THEN** the server returns `422 E-WEB-008`

#### Scenario: Same version
- **GIVEN** the version 2 of `R-100`
- **WHEN** `mhoffman` sends `GET /recipes/R-100/versions/2/diff/2`
- **THEN** the page shows the empty state of the diff

### Requirement: Pages of a large diff
A page of a diff SHALL show at most `page.diff` parameters over all its sections. A section SHALL show at most `diff.leaf_rows` rows of one parameter. It SHALL show the count of the other rows with links to the parameter pages of both versions. A page after the last page of a section SHALL show the text of `list-page-after-end` (R30, A26, ADR-0050).

#### Scenario: Array of 2 000 changed elements
- **GIVEN** two versions of `R-100` that differ in 2 000 elements of one array parameter
- **WHEN** a user opens the values section of the diff
- **THEN** the section shows 1 000 rows of that parameter and the count 1 000 of the other rows

### Requirement: Review state of a paged preview
Each link of the approval preview to another page SHALL carry the query key `rv` with the `row_version` of its first page. The approve form SHALL send that value as `row_version`. A later page can read another `row_version`. Then the page SHALL show a notice and a link to the first page, and no approve form (R31, I12, ADR-0050).

#### Scenario: Draft changes during the review
- **GIVEN** the approval preview of version 2 of `R-100` with `rv=7` on its second page link
- **WHEN** a draft command commits and the Administrator `admin` opens the second page
- **THEN** the page shows the notice of a changed draft and a link to the first page
- **AND** the page shows no approve form

### Requirement: Recipe history page
`GET /recipes/{ref}/history` SHALL show the audit entries of the recipe, newest first, at most `page.history` rows, with the cursor pager. A row SHALL show the time, the actor, the action, and the label of the entity, and no state. For a role with `AuditView` or `AuditViewRecipes`, each row SHALL link to `/audit/{id}` (R20, R24, A20, ADR-0051).

#### Scenario: Operator reads the history
- **GIVEN** the recipe `R-100` with the approval of version 2
- **WHEN** the Operator `mhoffman` sends `GET /recipes/R-100/history`
- **THEN** the page shows a row with the word of the action `VersionApproved`
- **AND** the page shows no link to `/audit/`

#### Scenario: ProcessEngineer reads the history
- **GIVEN** the recipe `R-100` with the approval of version 2
- **WHEN** the ProcessEngineer `anowak` sends `GET /recipes/R-100/history`
- **THEN** each row links to its page `/audit/{id}`

### Requirement: Audit list page
`GET /audit` SHALL show a filter form that sends `GET` without a token and without `before`, and the list of entries with the cursor pager. The selects of `entity_type` and `action` SHALL offer only the values of the scope of the role. For a role with `AuditView`, the page SHALL link to `/security-events` (R20, R28, A20, ADR-0051).

#### Scenario: Operator asks for the audit list
- **GIVEN** the Operator `mhoffman` with an open session
- **WHEN** `mhoffman` sends `GET /audit`
- **THEN** the server returns `403 E-WEB-002`
- **AND** the system writes one security event of kind `forbidden`

#### Scenario: Filter of a ProcessEngineer
- **GIVEN** the ProcessEngineer `anowak` with an open session
- **WHEN** `anowak` sends `GET /audit`
- **THEN** the select of `entity_type` offers only `recipe`, `recipe_version`, and `parameter`
- **AND** the page shows no link to `/security-events`

#### Scenario: Last page
- **GIVEN** the Administrator `admin` and an audit log with 30 entries
- **WHEN** `admin` sends `GET /audit`
- **THEN** the page shows 30 rows, the link "Newest entries", and the text of `audit-no-older`

### Requirement: Filter values of the audit list page
The filter form of `/audit` SHALL be a boosted form with literal `hx-*` attributes. A malformed filter value SHALL answer `422 E-WEB-008` in `#error-panel` with the label of the field, and the form SHALL stay. A well-formed `actor` that names no user SHALL give the empty list. A `to` of 9999-12-31 SHALL set no upper bound (R38, A42, ADR-0051).

#### Scenario: Malformed date in the filter
- **GIVEN** the Administrator `admin` on the audit list
- **WHEN** `admin` sends the filter form by HTMX with `from` set to `2026-13-01`
- **THEN** the server returns `422 E-WEB-008` in `#error-panel` with the label of `field-from`
- **AND** the filter form stays on the page

#### Scenario: Actor that names no user
- **GIVEN** the Administrator `admin` and no user with the login `ghost`
- **WHEN** `admin` sends `GET /audit?actor=ghost`
- **THEN** the server returns `200` with the text of `audit-empty`

#### Scenario: Last day of the calendar
- **GIVEN** the Administrator `admin` and an audit log with 30 entries
- **WHEN** `admin` sends `GET /audit?to=9999-12-31`
- **THEN** the page shows the 30 rows

### Requirement: Audit entry page
`GET /audit/{id}` SHALL show the fields of the entry and `before` and `after` as formatted JSON of the stored record in a `pre` element. Each text SHALL be escaped. The page SHALL show no `entry_hash` and no `prev_hash`, no password hash, no session token, and no cookie value. It SHALL show `chain_seq`, `client_ip`, and `session_ref` only to a role with `AuditView` (R20, A20, A43, `reliability.md` 1.1, ADR-0051).

#### Scenario: Entry with markup in a state
- **GIVEN** an entry `RecipeRenamed` whose `after` holds the name `<script>alert(1)</script>`
- **WHEN** the Administrator `admin` opens the page of the entry
- **THEN** the page holds the text `&#60;script&#62;` inside the `pre` element and no `<script>` element of the name

#### Scenario: ProcessEngineer reads a recipe entry
- **GIVEN** the entry `RecipeRenamed` of `R-100`
- **WHEN** the ProcessEngineer `anowak` opens the page of the entry
- **THEN** the page shows no chain position, no client address, and no session reference

#### Scenario: Entry outside the recipe scope
- **GIVEN** the entry `MachineRenamed` of the machine `M12`
- **WHEN** the ProcessEngineer `anowak` sends `GET /audit/{id}` of that entry
- **THEN** the server returns `404 E-WEB-005`

### Requirement: Security event page
`GET /security-events` SHALL show the filter form and the security events, newest first, with the cursor pager. A row SHALL show the time, the kind, the count, the client address, the user agent, and the request id. It SHALL also show the login or the words of `security-login-unknown`, and `detail` as formatted JSON. The page SHALL never show `login_hmac` (R35, A33, ADR-0051).

#### Scenario: ProcessEngineer asks for the security events
- **GIVEN** the ProcessEngineer `anowak` with an open session
- **WHEN** `anowak` sends `GET /security-events`
- **THEN** the server returns `403 E-WEB-002`

#### Scenario: Event of an unknown login
- **GIVEN** a security event `login_failed` of the typed login `ghost`, which matches no account
- **WHEN** the Administrator `admin` sends `GET /security-events?login=ghost`
- **THEN** the page shows the row with the words of `security-login-unknown`
- **AND** the page holds no text of the `login_hmac` of the row

### Requirement: Read-only mode for the lifecycle actions
In read-only mode, the approve, retire, discard, delete, and restore routes SHALL answer `503 E-WEB-006` and write nothing. The preview, the diff, the history, the audit, and the security event pages SHALL stay available (A24).

#### Scenario: Approval in read-only mode
- **GIVEN** the read-only mode
- **WHEN** the Administrator `admin` sends the approval of version 2 of `R-100`
- **THEN** the server returns `503 E-WEB-006`
- **AND** version 2 stays a draft

### Requirement: Texts of the approval and audit pages
Every error code that a route of this change returns SHALL have a message in the bundles `pl` and `en`. Each word of these pages SHALL come from the bundles. Examples are an action, an entity type, a kind of security event, and a state of the diff (R10, R36, R40).

#### Scenario: Action word in Polish
- **GIVEN** the ProcessEngineer `anowak` with the UI language `pl`
- **WHEN** `anowak` opens the history of `R-100`
- **THEN** each action word of the page comes from the bundle `pl`

### Requirement: Approval and audit pages without user data in an hx attribute
Each `hx-*` attribute of the approval preview, the diff, the history, the audit, and the security event pages SHALL hold a literal. No URL SHALL hold the CSRF token. Every form that holds `csrf_token` SHALL send `POST` (R38, ADR-0037).

#### Scenario: Approval preview
- **GIVEN** the approval preview of version 2 of `R-100`
- **WHEN** the test reads every `hx-*` attribute of the page
- **THEN** no attribute holds `R-100`, a version number, or a parameter tag

### Requirement: Redirects on the export routes
An export request without a valid session SHALL get `303` to `/login` and SHALL end the session cookie. An export request of a session with `must_change_password` SHALL get `303` to `/me/password`. An HTMX header SHALL NOT change these answers (R28, R39).

#### Scenario: Expired session
- **GIVEN** the Operator `mhoffman` with an expired session
- **WHEN** `mhoffman` sends `GET /export/v1/recipes/R-100/approved`
- **THEN** the server returns `303` with the location `/login`

#### Scenario: Forced password change
- **GIVEN** the ProcessEngineer `anowak` with a session that has `must_change_password`
- **WHEN** `anowak` sends `GET /export/v1/recipes/R-100/versions/2`
- **THEN** the server returns `303` with the location `/me/password`

### Requirement: Export link of a version page
The page of an `Approved` version SHALL show a link to `/export/v1/recipes/{ref}/approved` to each role with `VersionExport`. The page of a `Retired` version SHALL show a link to `/export/v1/recipes/{ref}/versions/{n}` only to a role with `VersionExportHistory`, with a notice on the prefix `RETIRED_`. A `Draft`, a discarded draft, and a version of a deleted recipe SHALL show no export link. The link SHALL stay in read-only mode (R24, R28, A24).

#### Scenario: Operator reads an approved version
- **GIVEN** the Operator `mhoffman` with an open session and the approved version 3 of `R-100`
- **WHEN** `mhoffman` sends `GET /recipes/R-100/versions/3`
- **THEN** the page shows a link to `/export/v1/recipes/R-100/approved`

#### Scenario: Operator reads a retired version
- **GIVEN** the Operator `mhoffman` with an open session and the retired version 2 of `R-100`
- **WHEN** `mhoffman` sends `GET /recipes/R-100/versions/2`
- **THEN** the page shows no export link

#### Scenario: ProcessEngineer reads a retired version
- **GIVEN** the ProcessEngineer `anowak` with an open session and the retired version 2 of `R-100`
- **WHEN** `anowak` sends `GET /recipes/R-100/versions/2`
- **THEN** the page shows a link to `/export/v1/recipes/R-100/versions/2`
- **AND** the link names the notice on the prefix `RETIRED_` in `aria-describedby`

#### Scenario: Draft version
- **GIVEN** the Administrator `admin` with an open session and the draft version 4 of `R-100`
- **WHEN** `admin` sends `GET /recipes/R-100/versions/4`
- **THEN** the page shows no export link

#### Scenario: Discarded draft
- **GIVEN** the Administrator `admin` with an open session and the discarded draft version 4 of `R-100`
- **WHEN** `admin` sends `GET /recipes/R-100/versions/4`
- **THEN** the page shows no export link

#### Scenario: Read-only mode
- **GIVEN** the application in read-only mode, the Operator `mhoffman` with an open session, and the approved version 3 of `R-100`
- **WHEN** `mhoffman` sends `GET /recipes/R-100/versions/3`
- **THEN** the page shows a link to `/export/v1/recipes/R-100/approved`

#### Scenario: Version of a deleted recipe
- **GIVEN** the Administrator `admin` with an open session and the retired version 2 of the deleted recipe `R-700`
- **WHEN** `admin` sends `GET /recipes/R-700/versions/2`
- **THEN** the page shows no export link

### Requirement: Export link without HTMX
The export link SHALL be a plain link with the literal `hx-boost="false"`. No ancestor of the link SHALL have `hx-boost="true"`. The link SHALL have no `download` attribute and no `target` attribute. Its visible text SHALL name the version and the file type (R39, A44).

#### Scenario: Attributes of the export link
- **GIVEN** the ProcessEngineer `anowak` with an open session in the UI language `pl` and the approved version 3 of `R-100`
- **WHEN** `anowak` sends `GET /recipes/R-100/versions/3`
- **THEN** the export link has `hx-boost="false"` and no `download` and no `target` attribute
- **AND** no ancestor of the link has `hx-boost="true"`
- **AND** the link text is `Eksportuj wersję 3 do pliku JSON`

### Requirement: Navigation guard of the export routes
A request on an export route with a `Sec-Fetch-Site` header other than `same-origin` or `none`, or with `HX-Request: true`, SHALL get `403` with the code `E-WEB-007` as a JSON body. The system SHALL write one `csrf_rejected` security event, SHALL NOT run `ExportVersion`, and SHALL write no `VersionExported` entry. A request without `Sec-Fetch-Site` SHALL pass the guard (R20, A8).

#### Scenario: Same-site request
- **GIVEN** the Operator `mhoffman` with an open session and the approved version 3 of `R-100`
- **WHEN** a page of a sibling host sends `GET /export/v1/recipes/R-100/approved` with `Sec-Fetch-Site: same-site` and the session cookie of `mhoffman`
- **THEN** the server returns `403` with a JSON error body and the code `E-WEB-007`
- **AND** the system writes one `csrf_rejected` security event
- **AND** the audit log holds no new `VersionExported` entry

#### Scenario: Bookmark
- **GIVEN** the Operator `mhoffman` with an open session and the approved version 3 of `R-100`
- **WHEN** `mhoffman` opens `GET /export/v1/recipes/R-100/approved` from a bookmark with `Sec-Fetch-Site: none`
- **THEN** the server returns `200` with the export document

#### Scenario: Request without a site header
- **GIVEN** the Operator `mhoffman` with an open session and the approved version 3 of `R-100`
- **WHEN** `mhoffman` sends `GET /export/v1/recipes/R-100/approved` without a `Sec-Fetch-Site` header
- **THEN** the server returns `200` with the export document

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

### Requirement: Value column of the parameter table
The parameter table of the version page SHALL have the column Value after the column Type. An elementary parameter SHALL show its value text as on the parameter page. The cell SHALL show at most `table.value_chars` characters and end a longer text with `…`. An empty text SHALL show `parameters-value-empty`. An array or a struct SHALL show `parameters-value-composite` (R08, R12, R30, R39, A43).

#### Scenario: Elementary value in an approved version
- **GIVEN** the Operator `mhoffman` with an open session and the approved version 2 of `R-100` with the parameter `SPEED : INT` and the value `1200`
- **WHEN** `mhoffman` sends `GET /recipes/R-100/versions/2`
- **THEN** the row of `SPEED` shows `1200` in the column Value

#### Scenario: Struct value
- **GIVEN** the draft version 3 of `R-100` with the parameter `LINE_A` of the type `ST_Zone@2`
- **WHEN** the ProcessEngineer `anowak` sends `GET /recipes/R-100/versions/3`
- **THEN** the row of `LINE_A` shows the word of `parameters-value-composite` in the column Value

#### Scenario: Long text
- **GIVEN** the draft version 3 of `R-100` with the parameter `NOTE : STRING(254)` and a value of 200 characters
- **WHEN** `anowak` sends `GET /recipes/R-100/versions/3`
- **THEN** the row of `NOTE` shows the first `table.value_chars` characters of the value and then `…`

#### Scenario: Row of a new parameter
- **GIVEN** `anowak` with the form of a new parameter of the draft version 3 of `R-100`
- **WHEN** `anowak` adds the parameter `TEMP : REAL` with the value `85.50`
- **THEN** the answer row of `TEMP` shows `85.5` in the column Value

### Requirement: Faults of the value column
A value that breaks only a newer rule SHALL get a field line. An S7 address that breaks only a newer rule SHALL get a field line under the address. Another fault SHALL give `500 E-DB-008`. A read check of the table that passes `timeout.validation` SHALL give `503 E-WEB-009` (R08, R12, R30, R39, A43).

#### Scenario: Stored value that does not match its type
- **GIVEN** the approved version 2 of `R-100` with the parameter `SPEED : INT`, whose stored value is the text `abc`
- **WHEN** `mhoffman` sends `GET /recipes/R-100/versions/2`
- **THEN** the server returns `500 E-DB-008`

#### Scenario: Stored DATE after the current range
- **GIVEN** the draft version 3 of `R-100` on the platform `S7_1500` with the stored `DATE` value `72700` in the parameter `DUE`
- **WHEN** `anowak` sends `GET /recipes/R-100/versions/3`
- **THEN** the server answers with status 200
- **AND** the row of `DUE` shows `D#2169-01-17` and a field line with `E-RCP-005`

#### Scenario: Stored DATE after the earlier range
- **GIVEN** the draft version 3 of `R-100` on the platform `S7_1500` with the stored `DATE` value `72900` in the parameter `DUE`
- **WHEN** `anowak` sends `GET /recipes/R-100/versions/3`
- **THEN** the server returns `500 E-DB-008`

#### Scenario: The table shows an address line under an address past the data block
- **GIVEN** a draft of an S7-1500 machine with a parameter `FILL` of type `DINT` at the stored address `DB1.DBD65534`
- **WHEN** the user opens the version page of the draft
- **THEN** the answer is 200
- **AND** the address cell of `FILL` shows `DB1.DBD65534` and a field line with `E-RCP-056` under it

#### Scenario: Read check of the table passes its limit
- **GIVEN** the draft version 3 of `R-100`, and the read check of one page of its parameter table runs longer than `timeout.validation`
- **WHEN** `anowak` sends `GET /recipes/R-100/versions/3/parameters`
- **THEN** the server returns `503 E-WEB-009`

### Requirement: Content hash of a version
The header of an Approved or a Retired version SHALL show its `content_sha256` as 64 lower-case hexadecimal characters. The text SHALL be equal to `content_sha256` of the export document of the version. The header of a Draft SHALL show no content hash (R17, R39).

#### Scenario: Approved version
- **GIVEN** the Operator `mhoffman` with an open session and the approved version 2 of `R-100`
- **WHEN** `mhoffman` sends `GET /recipes/R-100/versions/2`
- **THEN** the header shows the content hash of version 2
- **AND** the text is equal to `content_sha256` of `GET /export/v1/recipes/R-100/versions/2`

#### Scenario: Retired version keeps its hash
- **GIVEN** the version 2 of `R-100`, which an approval of version 3 retired
- **WHEN** the Administrator `admin` sends `GET /recipes/R-100/versions/2`
- **THEN** the header shows the same content hash as before the retirement

#### Scenario: Draft
- **GIVEN** the draft version 3 of `R-100`
- **WHEN** `mhoffman` sends `GET /recipes/R-100/versions/3`
- **THEN** the header shows no content hash

### Requirement: Compare form of the version page
The version page of a recipe with two or more versions SHALL show a compare form after the row of links, outside `<p class="actions">`. The form SHALL use `GET` on `/recipes/{ref}/compare` with `hx-boost="true" hx-target="#content" hx-push-url="true"`, without `csrf_token`. Lists `n` and `m` SHALL hold each version number. `n` SHALL select the shown version. `m` SHALL select the base version when it exists and differs, else the highest other one. A deleted recipe SHALL keep it (R27, R39, A44).

#### Scenario: Draft with a base version
- **GIVEN** the Operator `mhoffman` with an open session
- **AND** the versions 1, 2, and 3 of `R-100`, where version 3 has the base version 1
- **WHEN** `mhoffman` sends `GET /recipes/R-100/versions/3`
- **THEN** the compare form has `action="/recipes/R-100/compare"` and `method="get"`
- **AND** the list `n` selects 3 and the list `m` selects 1

#### Scenario: Recipe with one version
- **GIVEN** the recipe `R-200` with only the version 1
- **WHEN** `mhoffman` sends `GET /recipes/R-200/versions/1`
- **THEN** the page shows no compare form

#### Scenario: Administrator on a deleted recipe
- **GIVEN** the Administrator `admin` with an open session and the deleted recipe `R-300` with the versions 1 and 2
- **WHEN** `admin` sends `GET /recipes/R-300/versions/2`
- **THEN** the page shows the compare form

### Requirement: Compare route
`GET /recipes/{ref}/compare` SHALL answer `303` with the location `/recipes/{ref}/versions/{max(n,m)}/diff/{min(n,m)}`. The route SHALL read no recipe data and SHALL write nothing. A missing key, an unknown key, or a value that is not a version number SHALL give `422 E-WEB-008`. The diff route SHALL then check the permission and the versions (R27, R38).

#### Scenario: Two version numbers
- **GIVEN** the Operator `mhoffman` with an open session
- **WHEN** `mhoffman` sends `GET /recipes/R-100/compare?n=3&m=1`
- **THEN** the server returns `303` with the location `/recipes/R-100/versions/3/diff/1`

#### Scenario: Lower number first
- **GIVEN** `mhoffman` with an open session
- **WHEN** `mhoffman` sends `GET /recipes/R-100/compare?n=1&m=3`
- **THEN** the server returns `303` with the location `/recipes/R-100/versions/3/diff/1`

#### Scenario: Bad version number
- **GIVEN** `mhoffman` with an open session
- **WHEN** `mhoffman` sends `GET /recipes/R-100/compare?n=3&m=x`
- **THEN** the server returns `422 E-WEB-008`

#### Scenario: Unknown key
- **GIVEN** `mhoffman` with an open session
- **WHEN** `mhoffman` sends `GET /recipes/R-100/compare?n=3&m=1&kind=limits`
- **THEN** the server returns `422 E-WEB-008`

### Requirement: Row version in the success toast of a draft command
Each success toast of a command after which the draft stays a live draft SHALL name the new `row_version` of the version. `SetParameterValue` and `ReplaceLimits` SHALL have a success toast. The number SHALL be equal to `rowVersion` in the header `HX-Trigger` of the same answer (R39, I12).

#### Scenario: Saved value
- **GIVEN** the ProcessEngineer `anowak` with the parameter page of `SPEED` of the draft version 3 of `R-100` at `row_version` 7
- **WHEN** `anowak` saves the value `1250`
- **THEN** the toast names the row version 8
- **AND** the header `HX-Trigger` holds `{"rowVersion": 8}`

#### Scenario: Saved translation
- **GIVEN** `anowak` with the translation form `de` of `SPEED` of the draft version 3 at `row_version` 8
- **WHEN** `anowak` saves the translation
- **THEN** the toast names the row version 9

### Requirement: S7 layout of a type page
On a machine with `S7_300`, `S7_1200`, or `S7_1500`, the type page SHALL show the table S7 layout after the type tree. Each member SHALL have one row in the order of the tree with its path, its type, and its offset as `<byte>.<bit>`. An array SHALL have one row. The page SHALL show the size of the type in bytes. Another platform SHALL get no S7 layout (R12, R13, R14).

#### Scenario: Struct with two BOOL members
- **GIVEN** the Operator `mhoffman` with an open session and the struct `ST_Zone@2` of the machine `M7` on `S7_300` with the members `active : BOOL`, `ready : BOOL`, `setpoint : REAL`, and `count : INT`
- **WHEN** `mhoffman` sends `GET /machines/M7/types/ST_Zone`
- **THEN** the S7 layout shows the offsets `0.0`, `0.1`, `2.0`, and `6.0` in this order
- **AND** the page shows the size 8 bytes

#### Scenario: Nested struct and array
- **GIVEN** the struct `ST_Line@1` of `M7` with the members `zone : ST_Zone@2` and `ramps : ARRAY[1..3] OF INT`
- **WHEN** `mhoffman` sends `GET /machines/M7/types/ST_Line`
- **THEN** the S7 layout shows the rows `.zone`, `.zone.active`, `.zone.ready`, `.zone.setpoint`, `.zone.count`, and `.ramps`
- **AND** the row `.ramps` shows the offset `8.0`

#### Scenario: Platform without S7 layout
- **GIVEN** the struct `ST_Zone@1` of the machine `M8` on `CodesysIec`
- **WHEN** `mhoffman` sends `GET /machines/M8/types/ST_Zone`
- **THEN** the page shows no S7 layout

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

### Requirement: Password reset in a danger disclosure
The user page SHALL show the form of `ResetPassword` inside a danger disclosure of `design-system.md` section 10.5. The `summary` SHALL name the action. The warning SHALL have the id `danger-notice-reset`. The button SHALL name the login and SHALL name that id in `aria-describedby`. The form SHALL keep `hx-sync="closest form:drop"`. The deactivate form SHALL stay outside a disclosure (R28, R39, A44, ADR-0110).

#### Scenario: Closed disclosure on the user page
- **GIVEN** the Administrator `akowal` with an open session and the active user `anowak`
- **WHEN** `akowal` sends `GET /users/anowak`
- **THEN** the form that sends `POST /users/anowak/reset-password` stands inside a `details` element with no attribute `open`
- **AND** its button names `anowak` and has `aria-describedby="danger-notice-reset"`
- **AND** the page holds one element with the id `danger-notice-reset`

### Requirement: User page state after a user command
This requirement applies to a success of `SetUserRole`, `DeactivateUser`, `ActivateUser`, or `UnlockLogin` from the user page. An HTMX answer SHALL replace the facts of the user and the action forms out of band. The new facts SHALL come from `GetUser` after the commit. The answer SHALL keep `#user-result` as its target and SHALL NOT set `HX-Refresh`. An answer to a request without `HX-Request` SHALL be the full result page and SHALL hold no out-of-band element (R28, R39, A22).

#### Scenario: Status after a deactivation
- **GIVEN** the Administrator `akowal` on the page of the active user `anowak`
- **WHEN** `akowal` sends an HTMX `POST /users/anowak/deactivate` and gets `200`
- **THEN** the answer replaces the facts of `anowak` out of band with the state `Inactive`
- **AND** the answer replaces the action forms out of band with the activate form

#### Scenario: Failed attempts after an unlock
- **GIVEN** the Administrator `akowal` on the page of the user `anowak` with 4 failed attempts
- **WHEN** `akowal` sends an HTMX `POST /users/anowak/unlock` and gets `200`
- **THEN** the answer replaces the facts of `anowak` out of band with 0 failed attempts
- **AND** the replaced action forms hold no unlock form

#### Scenario: Request without HTMX
- **GIVEN** the Administrator `akowal` on the page of the active user `anowak`
- **WHEN** `akowal` sends `POST /users/anowak/deactivate` without the header `HX-Request`
- **THEN** the answer is the full result page
- **AND** the answer holds no element with `hx-swap-oob`

### Requirement: User page state after a password reset
This requirement applies to a success of `ResetPassword` from the user page. An HTMX answer SHALL replace the facts of the user out of band. The facts SHALL come from `GetUser` before the command and SHALL show the forced password change from the role. The answer SHALL NOT replace the action forms and SHALL NOT replace the reset disclosure. The answer SHALL keep `#user-result` as its target and SHALL NOT set `HX-Refresh` (R28, R39, A22, ADR-0109).

#### Scenario: One-time password stays visible
- **GIVEN** the Administrator `akowal` on the page of the user `anowak`
- **WHEN** `akowal` sends an HTMX `POST /users/anowak/reset-password` and gets `200`
- **THEN** `#user-result` shows the one-time password
- **AND** the answer replaces the facts of `anowak` out of band, and they show the forced password change
- **AND** the answer replaces no action form and no reset disclosure
- **AND** the answer has no header `HX-Refresh`

#### Scenario: Reset of an export client
- **GIVEN** the Administrator `akowal` on the page of the user `plc-line1` with the role `ExportClient`
- **WHEN** `akowal` sends an HTMX `POST /users/plc-line1/reset-password` and gets `200`
- **THEN** `#user-result` shows the one-time password
- **AND** the facts of `plc-line1` show no forced password change

#### Scenario: Reset when the read fails
- **GIVEN** the Administrator `akowal` and a store that fails the read of the user `anowak`
- **WHEN** `akowal` sends an HTMX `POST /users/anowak/reset-password`
- **THEN** the answer is the error fragment of the read error
- **AND** the password of `anowak` does not change

### Requirement: Focus after a user command
After a deactivation or an activation, the focus SHALL stay on the status button. After an unlock, the next Tab SHALL reach the first focusable element of the new answer in `#user-result`. After a reset, the focus SHALL stay on the reset button (R28, A22).

#### Scenario: Focus after a deactivation
- **GIVEN** the Administrator `akowal` on the page of the active user `anowak`, with the focus on the deactivate button
- **WHEN** `akowal` activates the button and the swap ends
- **THEN** the focus stands on the element with the id `user-status-submit`

#### Scenario: Focus after an unlock
- **GIVEN** the Administrator `akowal` on the page of the user `anowak` with 4 failed attempts, with the focus on the unlock button
- **WHEN** `akowal` activates the button, the swap ends, and `akowal` presses Tab
- **THEN** the focus stands on the first link inside `#user-result`

#### Scenario: Focus after a reset
- **GIVEN** the Administrator `akowal` on the page of the user `anowak`, with the focus on the reset button
- **WHEN** `akowal` activates the button and the swap ends
- **THEN** the focus stands on the reset button
- **AND** the one-time password is visible in `#user-result`

### Requirement: Applicable actions of the user page
The user page SHALL show the deactivate form only for an active user and the activate form only for an inactive user. The two submit buttons SHALL have the same fixed id. The page SHALL show the unlock form only when the user has at least one failed attempt (R28, R39, A22).

#### Scenario: Active user without failed attempts
- **GIVEN** the Administrator `akowal` and the active user `anowak` with no failed attempt
- **WHEN** `akowal` sends `GET /users/anowak`
- **THEN** the page holds the deactivate form and no activate form
- **AND** the page holds no unlock form

#### Scenario: Inactive user with failed attempts
- **GIVEN** the Administrator `akowal` and the inactive user `anowak` with 4 failed attempts
- **WHEN** `akowal` sends `GET /users/anowak`
- **THEN** the page holds the activate form, no deactivate form, and the unlock form

### Requirement: Machine unit column of the parameter table
The parameter table of the version page SHALL show the unit path under the header of the key `parameters-col-unit`. The key SHALL give `Jednostka maszyny` in `pl` and `Machine unit` in `en`. A user then does not read the column as a unit of measure (R06, R10, R39).

#### Scenario: Header in Polish
- **GIVEN** the Operator `mhoffman` with an open session and the UI language `pl`
- **WHEN** `mhoffman` sends `GET /recipes/R-100/versions/2`
- **THEN** the parameter table has the column header `Jednostka maszyny`

### Requirement: Parent link in the page header
Each parameter, version, unit, recipe, and machine page SHALL show a link to the parent page in its header. The link SHALL follow the `h1`. A page with a large state badge keeps the badge next to the `h1`. On that page, the link SHALL follow the row of the badge. The link SHALL stand before the first `dl` of the page. The link SHALL have the target and the text of the parent link at the end of the page. The link at the end SHALL stay (R39, A44).

#### Scenario: Parameter page
- **GIVEN** the ProcessEngineer `anowak` with an open session and the parameter `SPEED` of the draft version 3 of `R-100`
- **WHEN** `anowak` opens the page of `SPEED`
- **THEN** the element after the `h1` holds a link to `/recipes/R-100/versions/3`

#### Scenario: Version page with a state badge
- **GIVEN** the Operator `mhoffman` with an open session and the approved version 2 of `R-100`
- **WHEN** `mhoffman` sends `GET /recipes/R-100/versions/2`
- **THEN** the row of the large state badge follows the `h1`
- **AND** the element after that row holds a link to `/recipes/R-100`

#### Scenario: Machine page
- **GIVEN** the Operator `mhoffman` with an open session and the live machine `M7`
- **WHEN** `mhoffman` sends `GET /machines/M7`
- **THEN** the element after the row of the large state badge holds a link to `/machines`

### Requirement: Heading of a parameter page
The `h1` and the `title` of a parameter page SHALL name the tag, the reference number of the recipe, and the number of the version. The key `parameter-title` SHALL take the arguments `tag`, `reference`, and `number`. This applies to the page and to every answer that renders the form page of a parameter, such as the answer for `fields=1` (R16, R39).

#### Scenario: Heading in English
- **GIVEN** the ProcessEngineer `anowak` with the UI language `en` and the parameter `SPEED` of the draft version 3 of `R-100`
- **WHEN** `anowak` opens the page of `SPEED`
- **THEN** the `h1` holds `SPEED`, `R-100`, and `3`
- **AND** the `title` holds `SPEED`, `R-100`, and `3`

#### Scenario: Heading of the form page
- **GIVEN** the ProcessEngineer `anowak` with the UI language `pl` and the parameter `SPEED` of the draft version 3 of `R-100`
- **WHEN** `anowak` sends the request for the form page of `SPEED` with `fields=1`
- **THEN** the `h1` and the `title` hold `SPEED`, `R-100`, and `3`
- **AND** the page shows no unresolved message key

### Requirement: Links of the type usage page
Each row of the type usage page SHALL link the reference number to the recipe page. It SHALL link the version number to the version page and the tag to the parameter page. Each link SHALL put the user data only into its `href` (R13, R19, R39).

#### Scenario: Row with links
- **GIVEN** the Operator `mhoffman` with an open session and the draft version 3 of `R-100` with the parameter `LINE` of the type `ST_Line@1`, which holds `ST_Zone@2`
- **WHEN** `mhoffman` sends `GET /machines/M7/types/ST_Zone/usage?rev=2`
- **THEN** the row of `LINE` links `R-100` to `/recipes/R-100`
- **AND** the row links `3` to `/recipes/R-100/versions/3`
- **AND** the row links `LINE` to the parameter page of `LINE`

### Requirement: Language of translated text
Each stored text of a translation row and each input of a translation form SHALL have the attribute `lang` with the language code of its row. The label of the input SHALL keep the UI language (R10, A17, A44).

#### Scenario: German translation form
- **GIVEN** the ProcessEngineer `anowak` with the UI language `pl`, the language `de` in the list of translation languages, and the parameter `SPEED` with a German translation
- **WHEN** `anowak` opens the page of `SPEED`
- **THEN** the name input and the description input of the row `de` have `lang="de"`
- **AND** their labels have no attribute `lang`

### Requirement: Link to the save button after the leaf grid
A parameter form that shows the leaf grid SHALL show a link after the grid. The `href` of the link SHALL be `#` followed by the id of the save button of the form. The link SHALL have the class `target-link` and SHALL hold no `hx-*` attribute (R39, A44).

#### Scenario: Edit form of a parameter
- **GIVEN** the ProcessEngineer `anowak` with an open session and the parameter `SPEED` of the draft version 3 of `R-100`
- **WHEN** `anowak` opens the page of `SPEED`
- **THEN** the edit form holds a link with `href="#parameter-save-submit"` after the leaf grid

#### Scenario: Jump on a narrow screen
- **GIVEN** the page of `SPEED` at a width of 320 CSS px with the end of the leaf grid in view
- **WHEN** the user activates the link after the leaf grid
- **THEN** the save button is in view
- **AND** the browser sends no request

### Requirement: Empty state of a filtered recipe list
A recipe list is filtered when `q`, `machine`, or `status` holds a non-empty value, or when `show_deleted` is on. When a filtered list has no matching row, the page SHALL show the sentence of `recipes-empty-filtered` in place of `recipes-empty`. The page SHALL also show a link to `/recipes` without a query, outside the empty table row. A list that is not filtered SHALL show `recipes-empty` and no such link (R24, R39).

#### Scenario: No match for a search text
- **GIVEN** the Operator `mhoffman` with an open session and the recipe `R-100`
- **WHEN** `mhoffman` sends `GET /recipes?q=ZZZ`
- **THEN** the page shows the sentence of `recipes-empty-filtered`
- **AND** the page holds a link to `/recipes`

#### Scenario: No recipe without a filter
- **GIVEN** the Operator `mhoffman` with an open session and no recipe
- **WHEN** `mhoffman` sends `GET /recipes`
- **THEN** the page shows the sentence of `recipes-empty`

#### Scenario: Submit of the filter form with empty values
- **GIVEN** the Operator `mhoffman` with an open session and no recipe
- **WHEN** `mhoffman` sends `GET /recipes?q=&machine=&status=`
- **THEN** the page shows the sentence of `recipes-empty`
- **AND** the page holds no link to `/recipes` in the empty state

### Requirement: Word of an empty text value
The key `parameters-value-empty` SHALL name an empty text, not a missing value. The key SHALL give `Pusty tekst` in `pl` and `Empty text` in `en` (R39, A43).

#### Scenario: Empty string in the parameter table
- **GIVEN** the Operator `mhoffman` with the UI language `en` and the approved version 2 of `R-100` with the parameter `NOTE : STRING(254)` and an empty value
- **WHEN** `mhoffman` sends `GET /recipes/R-100/versions/2`
- **THEN** the row of `NOTE` shows `Empty text` in the column Value

### Requirement: Recipe link of a machine page
The page of a live machine SHALL show a link to `/recipes?machine={code}`. The page of an archived machine SHALL NOT show this link (R05, R24, R39).

#### Scenario: Live machine
- **GIVEN** the Operator `mhoffman` with an open session and the live machine `M7`
- **WHEN** `mhoffman` sends `GET /machines/M7`
- **THEN** the page holds a link to `/recipes?machine=M7`

#### Scenario: Archived machine
- **GIVEN** the Administrator `akowal` with an open session and the archived machine `M8`
- **WHEN** `akowal` opens the page of `M8`
- **THEN** the page holds no link to `/recipes?machine=M8`

### Requirement: Pager of the user list
The user list SHALL use the page pager of `design-system.md` section 10.12. The pager SHALL be a `nav` element with the accessible name of `pager-label` and SHALL show the number of the current page. A page after the last page SHALL show the notice `list-page-after-end` and a link to the first page (R39, A44).

#### Scenario: Second page of users
- **GIVEN** the Administrator `akowal` with an open session and 150 users
- **WHEN** `akowal` sends `GET /users?page=2`
- **THEN** the page holds a `nav` element with the class `pager` and a link to the first page
- **AND** the pager shows the number 2 with `aria-current="page"`

#### Scenario: Page after the last page
- **GIVEN** the Administrator `akowal` with an open session and 150 users
- **WHEN** `akowal` sends `GET /users?page=99`
- **THEN** the page shows the notice `list-page-after-end`
- **AND** the page holds a link to the first page

### Requirement: Role select of the user forms
The role select of the set-role form and of the new-user form SHALL use the select field of `design-system.md` section 10.5. The field SHALL have its label, the required mark, and the error line wiring of that section. A role outside the list SHALL give `422` and an answer that shows the error line under the select, with `aria-invalid="true"` and `aria-describedby` on the select (R28, A44).

#### Scenario: New-user form
- **GIVEN** the Administrator `akowal` with an open session
- **WHEN** `akowal` sends `GET /users/new`
- **THEN** the select `role` has a `label` and `aria-required="true"`

#### Scenario: Set-role form
- **GIVEN** the Administrator `akowal` with an open session and the user `mhoffman`
- **WHEN** `akowal` sends `GET /users/mhoffman`
- **THEN** the select `role` has a `label` and `aria-required="true"`
- **AND** the option of the current role of `mhoffman` is selected

#### Scenario: Role outside the list
- **GIVEN** the Administrator `akowal` with an open session and the user `mhoffman`
- **WHEN** `akowal` sends `POST /users/mhoffman/role` with a role that is not in the list
- **THEN** the answer has the status `422`
- **AND** the select `role` has `aria-invalid="true"` and `aria-describedby="role-error"`
- **AND** the element `role-error` holds the error code and one sentence

### Requirement: Focus on the login page
`GET /login` SHALL render the login input with the attribute `autofocus`. A login page with field errors SHALL NOT set `autofocus` on an input (R39, A44).

#### Scenario: Empty login page
- **GIVEN** a browser without a session
- **WHEN** a user sends `GET /login`
- **THEN** the login input has the attribute `autofocus`

#### Scenario: Login page after a failed login
- **GIVEN** a browser without a session
- **WHEN** a user sends `POST /login` without a password and without the header `HX-Request`
- **THEN** no input of the page has the attribute `autofocus`

### Requirement: State words of the type list
The state column of the type list SHALL show the word of `type-latest` for the latest revision of a struct. Every other revision SHALL show the word of `type-superseded`. Each word SHALL start with a capital letter (R13, R19).

#### Scenario: Two revisions of one struct
- **GIVEN** the Operator `mhoffman` with the UI language `en` and the struct `ST_Zone` with the revisions 1 and 2 on the machine `M7`
- **WHEN** `mhoffman` sends `GET /machines/M7/types`
- **THEN** the row `ST_Zone@2` shows `Latest`
- **AND** the row `ST_Zone@1` shows `Superseded`

### Requirement: Empty state of a unit page
A unit page without a live sub-unit SHALL show the sentence of `unit-children-empty` in the empty state element of `design-system.md` section 10.10 (R06, R39).

#### Scenario: Unit without sub-units
- **GIVEN** the Operator `mhoffman` with an open session and the unit `U1.2` of the machine `M7` with no sub-unit
- **WHEN** `mhoffman` sends `GET /machines/M7/units/U1.2`
- **THEN** the page shows the sentence of `unit-children-empty` in an element with the class `empty`

### Requirement: Warning of a delete disclosure
The delete disclosure of a parameter SHALL give its warning the id `danger-notice-parameter`. The delete disclosure of a translation SHALL give its warning the id `danger-notice-translation-` followed by the language code of the row. The delete button SHALL name that id in `aria-describedby` (R10, R39, A44, ADR-0110).

#### Scenario: Parameter page with two translations
- **GIVEN** the ProcessEngineer `anowak` and the parameter `SPEED` of the draft version 3 of `R-100` with the translations `de` and `fr`
- **WHEN** `anowak` opens the page of `SPEED`
- **THEN** the delete buttons name `danger-notice-parameter`, `danger-notice-translation-de`, and `danger-notice-translation-fr` in `aria-describedby`
- **AND** the page holds each of these ids once

### Requirement: Polish terms for a draft and a tag
The bundle `pl` SHALL name the state Draft `Wersja robocza` and SHALL name a parameter tag `Tag`. No value of `pl` SHALL hold a form of the word `szkic` or the word `znacznik`. The bundle `en` SHALL stay as it is (R10, A17, A44).

#### Scenario: Words of the Polish bundles
- **GIVEN** the bundles `pl` and `pl-approval`
- **WHEN** the server loads them
- **THEN** no value holds `szkic` or `znacznik` in any letter case
- **AND** the keys `field-tag`, `parameters-col-tag`, and `diff-field-tag` give `Tag`

#### Scenario: Field error for a tag
- **GIVEN** the ProcessEngineer `anowak` with the UI language `pl`
- **WHEN** `anowak` sends a new parameter with an invalid tag
- **THEN** the error line names the field `Tag`

### Requirement: Answer of a translation deletion
An HTMX success of `RemoveTranslation` SHALL replace only the row of the deleted tag. The new row SHALL show the tag without a stored translation. The answer SHALL NOT replace another row, so the unsaved input of the other rows SHALL stay. The answer SHALL set `HX-Trigger` with the new `row_version` (R10, R31, R39, A17, ADR-0111).

#### Scenario: Unsaved input of another row stays
- **GIVEN** the ProcessEngineer `anowak` on the page of `SPEED` of the draft version 3 of `R-100` with the stored translations `pl` and `de`
- **AND** `anowak` typed a new name into the row `pl` and did not save it
- **WHEN** `anowak` deletes the translation `de` and gets `200`
- **THEN** the answer holds the row with the id `translation-de` and no row with the id `translation-pl`
- **AND** the row `pl` of the page still shows the typed name

#### Scenario: Deleted tag of the list
- **GIVEN** `i18n.translation_langs` with `pl` and `de` and the parameter `SPEED` with the stored translation `de`
- **WHEN** `anowak` sends an HTMX `POST .../parameters/<key>/translations/de/delete` with the current `row_version`
- **THEN** the answer row `translation-de` holds an empty form that sends `POST .../translations/de`
- **AND** the answer row holds no delete form

#### Scenario: Deleted tag outside the list
- **GIVEN** `i18n.translation_langs` with `pl` and the parameter `LINE` with the stored translation `fr`
- **WHEN** `anowak` sends an HTMX `POST .../parameters/<key>/translations/fr/delete` with the current `row_version`
- **THEN** the answer row `translation-fr` shows the sentence of `translation-none` and holds no form

#### Scenario: New row version for the other rows
- **GIVEN** the draft version 3 of `R-100` with `row_version` 7
- **WHEN** `anowak` deletes the translation `de` and gets `200`
- **THEN** the answer has the header `HX-Trigger` with `rowVersion` 8
- **AND** the field `row_version` of the form `pl` holds 8 without a page load
- **AND** a later save of the row `pl` with `row_version` 8 gives `200`

#### Scenario: Deletion of the last stored translation
- **GIVEN** `i18n.translation_langs` with `pl` and `de` and the parameter `SPEED` with only the stored translation `pl`
- **WHEN** `anowak` deletes the translation `pl` and gets `200`
- **THEN** the answer row `translation-pl` holds an empty form and no delete form

#### Scenario: Error answer keeps every row
- **GIVEN** another session changed the draft version 3 of `R-100` after `anowak` loaded the page
- **WHEN** `anowak` deletes the translation `de`
- **THEN** the server returns `409 E-WEB-001` with `HX-Retarget: #error-panel`
- **AND** no translation row of the page changes

### Requirement: Focus after a translation deletion
The heading of the answer row of a translation deletion SHALL have `tabindex="-1"` and `autofocus`. After the swap the focus SHALL be on that heading (A44, ADR-0111).

#### Scenario: Focus on the heading of the row
- **GIVEN** `anowak` on the page of `SPEED` with the stored translation `de`
- **WHEN** `anowak` opens the delete disclosure of `de` and activates the delete button with the keyboard
- **THEN** the heading of the row `translation-de` has the focus

### Requirement: English base text after a save of the definition
An HTMX success of `UpdateParameterDefinition` SHALL also replace the English base text above the translation rows out of band. The answer SHALL NOT replace a translation row, so the unsaved input of the rows SHALL stay (R08, R10, R31, R39, ADR-0111).

#### Scenario: New English name above the translations
- **GIVEN** `anowak` on the page of `SPEED` of the draft version 3 of `R-100` with the stored translation `pl`
- **AND** `anowak` typed a new name into the row `pl` and did not save it
- **WHEN** `anowak` saves the edit form with the new `name_en` `Press speed` and gets `200`
- **THEN** the answer holds the block `translations-base` with `hx-swap-oob="true"` and the text `Press speed`
- **AND** the answer holds no element with the class `translation-row`
- **AND** the row `pl` of the page still shows the typed name

### Requirement: Multi-line text field
The fields `description_en`, the `description` of a translation row, and `change_note` SHALL be multi-line text fields with four rows and a maximum of 4 000 characters. Each field SHALL show a limit line with the maximum count and SHALL name its hint, its limit line, and its error line in `aria-describedby`. Every other text field SHALL stay a single-line input (R08, R10, R39, A44, ADR-0112).

#### Scenario: Description field of the edit form
- **GIVEN** the ProcessEngineer `anowak` with an open session
- **WHEN** `anowak` opens the page of `SPEED` of the draft version 3 of `R-100`
- **THEN** the edit form holds a `textarea` with the id `description_en`, `rows="4"`, and `maxlength="4000"`
- **AND** its `aria-describedby` names `description_en-limit`
- **AND** the field `name_en` stays an `input` with `type="text"`

#### Scenario: Change note of a new version
- **GIVEN** `anowak` with an open session
- **WHEN** `anowak` sends `GET /recipes/R-100/versions/new`
- **THEN** the form holds a `textarea` with the id `change_note` and `maxlength="4000"`

#### Scenario: Stored text with a line end in the field
- **GIVEN** the parameter `SPEED` with the description `Line 1` LF `Line 2`
- **WHEN** `anowak` opens the page of `SPEED`
- **THEN** the value of the `textarea` `description_en` is `Line 1` LF `Line 2`

#### Scenario: Leading line end and markup in the field
- **GIVEN** the parameter `SPEED` with a description that starts with LF and holds `<b>`
- **WHEN** `anowak` opens the page of `SPEED`
- **THEN** the value of the `textarea` `description_en` starts with LF
- **AND** the page holds `&lt;b&gt;` and no element `b` inside the `textarea`

### Requirement: Line ends of a multi-line field
The server SHALL convert each CRLF of a multi-line text field to LF before it checks the text. A lone CR SHALL stay, and the check SHALL refuse it. The limit of 4 000 characters SHALL count each line end as one character. The limit SHALL count Unicode characters, not UTF-16 code units (R31, R38, ADR-0112).

#### Scenario: CRLF stored as LF
- **GIVEN** `anowak` on the page of `SPEED` of the draft version 3 of `R-100`
- **WHEN** `anowak` saves the edit form with the description `Line 1` CRLF `Line 2`
- **THEN** the server returns `200`
- **AND** the stored description of `SPEED` is `Line 1` LF `Line 2`

#### Scenario: Text at the limit with line ends
- **GIVEN** a change note of 3 990 letters and 10 line ends, sent as CRLF
- **WHEN** `anowak` sends `POST /recipes` for a new recipe with that change note
- **THEN** the server creates the recipe
- **AND** the stored change note of its version 1 has 4 000 characters

#### Scenario: Characters outside the Basic Multilingual Plane
- **GIVEN** a change note of 4 000 characters `𝄞` (U+1D11E)
- **WHEN** `anowak` sends `POST /recipes` for a new recipe with that change note
- **THEN** the server creates the recipe

#### Scenario: Lone CR refused
- **GIVEN** `anowak` on the page of `SPEED`
- **WHEN** `anowak` saves the edit form with the description `Line 1` CR `Line 2`
- **THEN** the server returns `422` with a field error of `description_en`
- **AND** the stored description does not change

### Requirement: Save keeps the line ends of a description
A save of the edit form of a parameter SHALL keep each line end of the stored `description_en`. This rule SHALL also apply when the user changes only a value (R08, R31, ADR-0112).

#### Scenario: Value save with a two-line description
- **GIVEN** the parameter `SPEED` with the description `Line 1` LF `Line 2`
- **WHEN** `anowak` changes only the value of `SPEED` and saves the edit form
- **THEN** the stored description of `SPEED` is `Line 1` LF `Line 2`

### Requirement: Multi-line text outside a form
A page SHALL show each line end of a description or a change note outside a form as a new line (R39, ADR-0112).

#### Scenario: Description in the parameter summary
- **GIVEN** the parameter `SPEED` with the description `Line 1` LF `Line 2`
- **WHEN** the Operator `mhoffman` opens the page of `SPEED`
- **THEN** the summary shows `Line 1` and `Line 2` on two lines

#### Scenario: Change note on the version page
- **GIVEN** the version 3 of `R-100` with the change note `Step 1` LF `Step 2`
- **WHEN** `mhoffman` opens the page of the version 3 of `R-100`
- **THEN** the page shows `Step 1` and `Step 2` on two lines

### Requirement: Controls of a blocked command in read-only mode
In read-only mode a page SHALL show no control of a command that the mode blocks. A control is a form that sends the command, or a link or button whose only target is the form of the command. The login form, the logout form, the language switch, and the export link SHALL stay. Every form with `method="get"`, every pager, and every navigation link SHALL also stay (R28, R31, R39, A24, ADR-0113).

#### Scenario: Parameter page of a draft
- **GIVEN** the application in read-only mode and the ProcessEngineer `anowak` with an open session
- **WHEN** `anowak` opens the page of `SPEED` of the draft version 3 of `R-100`
- **THEN** the page holds the read-only grid of the value and the limits
- **AND** the page holds the logout form and the language switch
- **AND** the page holds no other form with `method="post"`
- **AND** the page shows the read-only banner

#### Scenario: Version page of a draft
- **GIVEN** the application in read-only mode and `anowak` with an open session
- **WHEN** `anowak` opens the page of the draft version 3 of `R-100`
- **THEN** the page holds no link "Add a parameter", no link "Approve", no rebind form, and no danger disclosure
- **AND** the page holds the compare form

#### Scenario: Links to the form of a new version or revision
- **GIVEN** the application in read-only mode and `anowak` with an open session
- **WHEN** `anowak` opens the page of `R-100` and the page of a UDT type
- **THEN** the recipe page holds no link to the form of a new version
- **AND** the type page holds no link to the form of a new revision

#### Scenario: Export link of an approved version
- **GIVEN** the application in read-only mode and the Operator `mhoffman` with an open session
- **WHEN** `mhoffman` opens the page of the `Approved` version 2 of `R-100`
- **THEN** the page holds the export link

#### Scenario: Same page in normal mode
- **GIVEN** the application not in read-only mode and `anowak` with an open session
- **WHEN** `anowak` opens the page of `SPEED` of the draft version 3 of `R-100`
- **THEN** the page holds the edit form and the translation forms

### Requirement: Form route in read-only mode
In read-only mode a route that answers the form of a blocked command SHALL run its permission check first. It SHALL then answer `200` with its heading and the notice `readonly-form-unavailable` in place of the form. The notice SHALL use text and an icon, not color alone (R28, R39, A24, A44, ADR-0113).

#### Scenario: Form of a new recipe
- **GIVEN** the application in read-only mode and `anowak` with an open session
- **WHEN** `anowak` sends `GET /recipes/new`
- **THEN** the server returns `200` with the heading of the page and the notice `readonly-form-unavailable`
- **AND** the page holds no form that sends `POST /recipes`

#### Scenario: Fragment of the form of a new parameter
- **GIVEN** the application in read-only mode and `anowak` with an open session
- **WHEN** `anowak` sends an HTMX `GET .../parameters/new` with `HX-Target: parameter-form`
- **THEN** the server returns `200` with the notice `readonly-form-unavailable`
- **AND** the answer holds no form that sends `POST .../parameters`

#### Scenario: Form route without the permission
- **GIVEN** the application in read-only mode and the Operator `mhoffman` with an open session
- **WHEN** `mhoffman` sends `GET /recipes/new`
- **THEN** the server returns `403 E-WEB-002`

#### Scenario: Forced password change in read-only mode
- **GIVEN** the application in read-only mode and the user `mhoffman` with `must_change_password` set to true
- **WHEN** `mhoffman` sends `GET /me/password`
- **THEN** the page shows the notice `readonly-form-unavailable` and the logout button
- **AND** the page holds no form that sends `POST /me/password`

### Requirement: Field lines of a stored part that breaks a newer rule
The parameter page, the diff page, and the approval preview SHALL show a stored value, limit, or S7 address unchanged. A part that fails only the current rule set SHALL get a field line with its code, and the answer SHALL stay 200. A part that fails the earlier rule set of ADR-0120 SHALL give `500 E-DB-008` (R12, R38, R39).

#### Scenario: Value with too many digits on the parameter page
- **GIVEN** the approved version 2 of `R-100` with the stored `REAL` value `16777217` in the parameter `RATE`
- **WHEN** `mhoffman` opens the parameter page of `RATE` in version 2
- **THEN** the page shows the value `16777217` and a field line with `E-RCP-055`

#### Scenario: Limit after the DATE range on the parameter page
- **GIVEN** the draft version 3 of `R-100` on the platform `S7_1500` with the stored limit `max` `72700` in the parameter `DUE`
- **WHEN** `anowak` opens the parameter page of `DUE` in version 3
- **THEN** the page shows the limit `max` `D#2169-01-17` and a field line with `E-RCP-006` under it

#### Scenario: Address after the end of the data block
- **GIVEN** the draft version 3 of `R-100` on the platform `S7_1500` with the parameter `SPEED` of type `DINT` at `DB1.DBD65534`
- **WHEN** `anowak` opens the parameter page of `SPEED` in version 3
- **THEN** the page shows a field line with `E-RCP-056` under the field `s7_address`

#### Scenario: Diff with an affected limit
- **GIVEN** the versions 2 and 3 of `R-100` on the platform `S7_1500`, and version 3 holds the stored limit `max` `72700` on `DUE`
- **WHEN** `anowak` opens the diff of version 2 and version 3
- **THEN** the server answers with status 200
- **AND** the limit row of `DUE` shows a field line with `E-RCP-006`

#### Scenario: Approval preview of a draft with an affected value
- **GIVEN** the draft version 3 of `R-100` on the platform `S7_1500` with the stored `DATE` value `72700` in the parameter `DUE`
- **WHEN** an Administrator opens the approval preview of version 3
- **THEN** the preview lists the blocker `E-RCP-005` for `DUE`

#### Scenario: Approval preview of a fixed draft on an affected base
- **GIVEN** the approved version 2 of `R-100` with the stored `DATE` value `72700` in `DUE`, and the draft version 3 with `DUE` at `D#2168-12-31`
- **WHEN** an Administrator opens the approval preview of version 3
- **THEN** the server answers with status 200
- **AND** the diff shows a field line with `E-RCP-005` on the side of version 2

### Requirement: Notice of an approved version that is not loadable
The version page of an Approved version SHALL check every value, limit, and S7 address with the current rules. The address check SHALL include the overlap check of I20. When a part breaks a rule, the export area SHALL show a notice with the code of each broken rule. The export link SHALL name the notice in `aria-describedby` (R12, R28, R39, ADR-0149).

#### Scenario: Approved version with an affected value
- **GIVEN** the approved version 2 of `R-100` on the platform `S7_1500` with the stored `DATE` value `72700` in `DUE`
- **WHEN** `mhoffman` sends `GET /recipes/R-100/versions/2`
- **THEN** the export area shows a notice with `E-RCP-005`
- **AND** the export link names the notice in `aria-describedby`

#### Scenario: Approved version without an affected part
- **GIVEN** the approved version 2 of `R-100` with the parameter `SPEED : INT` and the value `1200`
- **WHEN** `mhoffman` sends `GET /recipes/R-100/versions/2`
- **THEN** the export area shows no notice
- **AND** the export link has no `aria-describedby`

#### Scenario: Approved version with an overlap after a change of the layout rules
- **GIVEN** the approved version 2 of `R-100` on the platform `S7_1500` with the stored parameters `BYTES` of type `ST_Bytes@1` at `DB1.DBB0` and `MODE` of type `BYTE` at `DB1.DBB4`, approved before ADR-0147
- **AND** `ST_Bytes@1` has the members `a` of type `BYTE`, `b` of type `CHAR`, `c` of type `SINT`, and `d` of type `USINT`
- **WHEN** `mhoffman` sends `GET /recipes/R-100/versions/2`
- **THEN** the export area shows a notice with `E-RCP-022`
- **AND** the export link names the notice in `aria-describedby`

### Requirement: Field answer of the own password form
`POST /me/password` SHALL check the current password and the new password before it answers. Its answer SHALL list each bad field at once, with a line under each bad field. A wrong current password (`E-IDN-008`) SHALL give a line under the field `current_password` and SHALL mark that input with `aria-invalid="true"`. `POST /login` SHALL keep one line per answer (R36, A44).

#### Scenario: Two bad fields of the own password form
- **GIVEN** the Operator `mhoffman` with an open session and the UI language `en`
- **WHEN** `mhoffman` sends an HTMX `POST /me/password` with a current password of 129 characters and a new password of 3 characters
- **THEN** the server returns `422` with the fragment `field-errors.html`
- **AND** the fragment holds one line for `current_password` and one line for `password`
- **AND** the line for `password` holds the code `E-IDN-006`

#### Scenario: New password on the breach list
- **GIVEN** the Operator `mhoffman` with an open session and the UI language `en`
- **WHEN** `mhoffman` sends an HTMX `POST /me/password` with a wrong current password and a new password of the breach list
- **THEN** the server returns `422` with one line `E-IDN-006` under `password`
- **AND** the server writes no login attempt row

#### Scenario: Wrong current password
- **GIVEN** the Operator `mhoffman` with an open session and the UI language `en`
- **WHEN** `mhoffman` sends an HTMX `POST /me/password` with a wrong current password and a valid new password
- **THEN** the server returns `422 E-IDN-008`
- **AND** the line of `E-IDN-008` links to `#current_password`
- **AND** the input `current_password` has `aria-invalid="true"`

#### Scenario: Two empty fields of the login form
- **GIVEN** a browser with a valid pre-session token
- **WHEN** a user sends `POST /login` with an empty `login` and an empty `password`
- **THEN** the server rejects the request with `E-WEB-008`
- **AND** the answer holds one line

### Requirement: Language switch after a pushed address
An HTMX content answer to a `POST` that pushes a path through `HX-Push-Url` SHALL also set the field `next` out of band. The field `next` of the language switch SHALL hold the pushed path. The approval, the retire, and the deletion of a parameter SHALL follow this rule (R10, R39, A17, ADR-0049 (e)).

#### Scenario: Language switch after an HTMX approval
- **GIVEN** the draft version 2 of `R-500` and the Administrator `admin` with an open session
- **WHEN** `admin` sends an HTMX approval of version 2 with its `row_version` and `HX-Target: content`
- **THEN** the answer sets `HX-Push-Url` to `/recipes/R-500/versions/2`
- **AND** the answer holds one field `next` with the value `/recipes/R-500/versions/2` and `hx-swap-oob="true"`

#### Scenario: Language switch after an HTMX deletion of a parameter
- **GIVEN** the live draft version 3 of `R-100` with the parameter `SPEED` and the ProcessEngineer `anowak` with an open session
- **WHEN** `anowak` deletes `SPEED` through an HTMX request with `HX-Target: content`
- **THEN** the answer sets `HX-Push-Url` to `/recipes/R-100/versions/3`
- **AND** the answer holds one field `next` with the value `/recipes/R-100/versions/3` and `hx-swap-oob="true"`

#### Scenario: Approval without HTMX
- **GIVEN** the draft version 2 of `R-500` and the Administrator `admin` with an open session
- **WHEN** `admin` sends the approval of version 2 without the header `HX-Request`
- **THEN** the server returns the full version page
- **AND** the page holds no language switch

### Requirement: Submit button of an out-of-band form
Each form whose start tag carries the attribute `hx-swap-oob` SHALL give each of its submit buttons a fixed `id`. Then HTMX keeps the focus on the activated button after the swap (R39, A44, ADR-0041 (e)).

#### Scenario: Refused change of the own password
- **GIVEN** the Operator `mhoffman` with an open session and the UI language `en`
- **WHEN** `mhoffman` sends an HTMX `POST /me/password` with a new password of 3 characters
- **THEN** the server returns `422` and swaps `#password-form` out of band
- **AND** the submit button of `#password-form` has the attribute `id="password-submit"`

#### Scenario: Every out-of-band form of the templates
- **GIVEN** the templates of `recipe-web`
- **WHEN** the template test reads each form whose start tag has the attribute `hx-swap-oob`
- **THEN** each submit button of that form has a fixed `id`
- **AND** each submit button of a macro that the form calls has a fixed `id`

### Requirement: Data block form of a machine
The machine page of a live S7 machine SHALL give a user with `MachineManage` a form that sets the data block number. The input SHALL hold the current number. A valid HTMX `POST /machines/{code}/data-block` SHALL answer `200` with the new number, a success message in `#toast`, and an empty `#error-panel`. A rejected request SHALL put the error into `#error-panel` and SHALL keep the form with a line under the field (R39, A48, ADR-0041).

#### Scenario: Form with the current number
- **GIVEN** the ProcessEngineer `anowak` with an open session and the live machine `M5` with the data block number `100`
- **WHEN** `anowak` sends `GET /machines/M5`
- **THEN** the input `db_number` of the data block form has the value `100`

#### Scenario: Set through HTMX
- **GIVEN** the ProcessEngineer `anowak` with an open session and the live machine `M5` with the platform `S7_1500`
- **WHEN** `anowak` sends an HTMX `POST /machines/M5/data-block` with the number `200`
- **THEN** the server returns `200` with the number `200`
- **AND** the answer puts the icon `alert-success` into `#toast` and empties `#error-panel`

#### Scenario: Number above the limit through HTMX
- **GIVEN** the ProcessEngineer `anowak` with an open session and the live machine `M3` with the platform `S7_300`
- **WHEN** `anowak` sends an HTMX `POST /machines/M3/data-block` with the number `16001`
- **THEN** the server returns `422 E-PLT-024` with `HX-Retarget: #error-panel`
- **AND** the answer swaps the form out of band with a line under the field `db_number`

#### Scenario: No data block form on an IEC machine
- **GIVEN** the ProcessEngineer `anowak` with an open session and the live machine `M8` with the platform `TwinCat3`
- **WHEN** `anowak` sends `GET /machines/M8`
- **THEN** the page holds no form that sends a `POST` request to `/machines/M8/data-block`

### Requirement: Connection form after a new data block number
A new data block number SHALL bump the connection record of the machine. A save of a connection form that the user loaded before that change SHALL fail with `409 E-WEB-013` (R28, A48).

#### Scenario: Connection form after a new data block number
- **GIVEN** the ProcessEngineer `anowak` with an open session and the machine `M5` with a PLC connection record and the data block number `100`
- **AND** `anowak` loaded the connection form of `M5`
- **AND** a ProcessEngineer then set the data block number of `M5` to `200`
- **WHEN** `anowak` saves the loaded connection form through HTMX
- **THEN** the server returns `409 E-WEB-013` with `HX-Retarget: #error-panel`
- **AND** the PLC connection record of `M5` keeps its values

### Requirement: Request state of a form
While an HTMX request of a form is in flight, the form SHALL have `aria-busy="true"`. Each text input and each `textarea` of the form SHALL be `readonly`. Each `select`, check box, and radio button of the form SHALL refuse a change. The focus SHALL stay on its control. After the last end of the request, the form SHALL lose `aria-busy` and each `readonly` that the lock set. The ends are an answer, a network error, an abort, a timeout, and a failed handling of the answer (R31, R39, A44).

#### Scenario: Typing during a save that answers 200
- **GIVEN** the ProcessEngineer `anowak` with the edit form of `LIMIT` and a save request in flight
- **WHEN** `anowak` types `22` into the value input
- **THEN** the value input keeps the sent text and the focus
- **AND** after the answer `200` the value input shows the saved value and is not `readonly`

#### Scenario: Typing during a save that answers 422
- **GIVEN** the ProcessEngineer `anowak` with the edit form of `LIMIT`, the value `abc`, and a save request in flight
- **WHEN** `anowak` types `33` into the value input
- **THEN** the value input keeps `abc`
- **AND** after the answer `422 E-RCP-005` the value input shows `abc` with its error line and is not `readonly`

#### Scenario: Typing during a save of a translation row
- **GIVEN** the ProcessEngineer `anowak` with the translation row `de` and a save request in flight
- **WHEN** `anowak` types a text into the name input of the row
- **THEN** the name input keeps the sent text
- **AND** after the answer `200` the name input is not `readonly`

#### Scenario: Typing during a translation save that answers 422
- **GIVEN** the ProcessEngineer `anowak` with the translation row `de`, a name that the server refuses, and a save request in flight
- **WHEN** `anowak` types a text into the description input of the row
- **THEN** the description input keeps the sent text
- **AND** after the answer `422` the row shows its error line and no input of the row is `readonly`

#### Scenario: Enter and click start the same lock
- **GIVEN** the ProcessEngineer `anowak` with the edit form of `LIMIT`
- **WHEN** `anowak` sends the form once with Enter in the value input and once with a click on the save button
- **THEN** in both cases the form has `aria-busy="true"` while the request is in flight

#### Scenario: Unlock after a network error
- **GIVEN** the ProcessEngineer `anowak` with the edit form of `LIMIT` and the value `44`
- **WHEN** the save request fails with a network error
- **THEN** the form has no `aria-busy`, the value input is not `readonly`, and it holds `44`

#### Scenario: Unlock after an abort
- **GIVEN** the ProcessEngineer `anowak` with the edit form of `LIMIT` and a save request in flight
- **WHEN** the browser aborts the request
- **THEN** the form has no `aria-busy` and the value input is not `readonly`
- **AND** the value input keeps its text and the focus

#### Scenario: Unlock after an answer with a missing target
- **GIVEN** the ProcessEngineer `anowak` with the edit form of `LIMIT`
- **WHEN** the save answer carries `HX-Retarget` with an id that the page does not hold
- **THEN** the form has no `aria-busy` and the value input is not `readonly`

#### Scenario: Readonly of the page stays
- **GIVEN** the edit form of `LIMIT` with one input that has `readonly` before the send
- **WHEN** a save request of the form ends with a network error
- **THEN** that input is still `readonly`
- **AND** each other text input of the form is not `readonly`

### Requirement: Lock rules of a busy form
A form with several requests in flight SHALL keep its lock until the last request ends. The lock SHALL let through Tab, each key with Ctrl, Alt, or Meta, and the keys F1 to F12. So the shortcuts of the browser work (R31, R39, A44).

#### Scenario: Two requests of one form
- **GIVEN** a form with two requests in flight, for example a boosted filter form and a boosted link inside it
- **WHEN** the first request ends
- **THEN** the form still has `aria-busy="true"` and its text inputs are still `readonly`
- **AND** after the second request ends the form has no `aria-busy` and no input that the lock set is `readonly`

#### Scenario: Shortcuts of the browser during a request
- **GIVEN** the ProcessEngineer `anowak` with the edit form of `LIMIT`, a `select` in it, and a save request in flight
- **WHEN** `anowak` presses Ctrl+R or F5 on the `select`
- **THEN** the script does not call `preventDefault`, so the browser keeps its shortcut
- **AND** a press of the Arrow Down key on the `select` stays refused

### Requirement: Message of a lost answer
When an HTMX request ends with a network error or with a timeout, the page SHALL put the message `E-WEB-015` into `#error-panel` and SHALL empty `#toast`. The page SHALL also show the top of the panel. The message SHALL have `role="alert"`, the code, and one sentence in the UI language. It SHALL say that the result is not known and SHALL have no reference. The next HTMX request SHALL delete the message (R31, R39, A44).

#### Scenario: Network error before the send
- **GIVEN** the ProcessEngineer `anowak` with the edit form of `LIMIT`, the UI language `en`, and the value `44`
- **WHEN** the save request fails with a network error before the server gets it
- **THEN** `#error-panel` holds a fragment with `role="alert"` and the code `E-WEB-015`
- **AND** the value input holds `44`

#### Scenario: Answer lost after the server saved
- **GIVEN** the ProcessEngineer `anowak` with the edit form of `LIMIT` and the value `55`
- **WHEN** the server saves `55` and the browser gets a network error in place of the answer
- **THEN** `#error-panel` holds the message `E-WEB-015` that the result is not known
- **AND** a new load of the page shows `55`

#### Scenario: Timeout of a request
- **GIVEN** the ProcessEngineer `anowak` with the edit form of `LIMIT`
- **WHEN** the save request gets no answer within `htmx.request_timeout`
- **THEN** `#error-panel` holds the message `E-WEB-015`
- **AND** the form has no `aria-busy`

#### Scenario: Lost answer after a success
- **GIVEN** the ProcessEngineer `anowak` on the edit page of `LIMIT` with the success message of a save in `#toast`
- **WHEN** the next save request fails with a network error
- **THEN** `#toast` is empty
- **AND** `#error-panel` holds the message `E-WEB-015`

#### Scenario: Next request deletes the message
- **GIVEN** a page that shows the message `E-WEB-015` after a network error before the send
- **WHEN** the user sends the same form again and the server answers `200`
- **THEN** `#error-panel` holds no message `E-WEB-015`

#### Scenario: Message in Polish
- **GIVEN** the UI language `pl`
- **WHEN** the server renders a full page
- **THEN** the template `#transport-error` holds the code `E-WEB-015` and the sentence of `error-E-WEB-015` from the bundle `pl`
