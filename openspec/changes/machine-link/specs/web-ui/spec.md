## MODIFIED Requirements

### Requirement: Routes available in read-only mode
In read-only mode, queries, the export routes, `POST /login`, and `POST /logout` SHALL stay available. The adapter routes `POST /adapter/v1/reports` and `POST /adapter/v1/load-requests/{request_id}/result` SHALL also stay available (A24, A55).

#### Scenario: Version page in read-only mode
- **GIVEN** the application in read-only mode
- **WHEN** an Operator sends a request for `GET /recipes/R-100/versions/3`
- **THEN** the server renders the version page with status `200`

#### Scenario: Link panel in read-only mode
- **GIVEN** the application in read-only mode and a fresh link state of `M12`
- **WHEN** an Operator opens the page of `M12`
- **THEN** the page holds the link panel with the link state of `M12`

### Requirement: Machine page
`GET /machines/{code}` SHALL show the code, the name, the PLC platform, the unit tree, and a link to the type list of the live machine. A user with `MachineManage` SHALL also get the rename form, the data block form, and the archive form inside a `details` element. The page of an archived machine SHALL show the state "Archived" and no link panel. It SHALL hold no form and no link to a route under `/machines/{code}` (R05, R06, R15, A32, A48, A55).

#### Scenario: Page of a live machine for an Operator
- **GIVEN** the Operator `mhoffman` with an open session and the live machine `M7` without an active load request
- **WHEN** `mhoffman` sends `GET /machines/M7`
- **THEN** the page shows `M7`, its name, its platform, and its unit tree
- **AND** the page holds no form that sends a `POST` request to `/machines/M7`

#### Scenario: Page of an archived machine
- **GIVEN** the ProcessEngineer `anowak` with an open session and the archived machine `M12`
- **WHEN** `anowak` sends `GET /machines/by-id/` with the identifier of `M12`
- **THEN** the page shows the large badge with the word of `status-archived`
- **AND** the page holds no form, no link panel, and no link to `/machines/M12` or to a path under it

#### Scenario: Live machine by identifier
- **GIVEN** the ProcessEngineer `anowak` with an open session and the live machine `M7`
- **WHEN** `anowak` sends `GET /machines/by-id/` with the identifier of `M7`
- **THEN** the page shows no badge of `status-archived`
- **AND** the page holds the rename form of `M7`

#### Scenario: Malformed machine code
- **GIVEN** the Operator `mhoffman` with an open session
- **WHEN** `mhoffman` sends `GET /machines/X7`
- **THEN** the server returns `404 E-WEB-005`

### Requirement: Plant forms without user data in an hx attribute
Every HTMX request of a machine, unit, or type page SHALL come from a form `action` or a link `href` with `hx-boost="true"`. The one exception SHALL be the passive poll of the status part of the link panel, which uses the literal `hx-get=""`. Each `hx-*` attribute that a template of these pages writes SHALL hold a literal. No URL of these pages SHALL hold the CSRF token (R38, A55).

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

#### Scenario: Poll of the link panel
- **GIVEN** the Operator `mhoffman` with an open session and the live machine `M7`
- **WHEN** `mhoffman` sends `GET /machines/M7`
- **THEN** the status part of the link panel has `hx-get=""` and `data-passive`
- **AND** no `hx-*` attribute of the link panel holds `M7` or a load request id

### Requirement: Message of a lost answer
When an HTMX request ends with a network error or with a timeout, the page SHALL put the message `E-WEB-015` into `#error-panel` and SHALL empty `#toast`. The page SHALL also show the top of the panel. The message SHALL have `role="alert"`, the code, and one sentence in the UI language. It SHALL say that the result is not known and SHALL have no reference. The next HTMX request SHALL delete the message. A `data-passive` request SHALL not change the message (R31, R39, A44, A55).

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

#### Scenario: Poll after a lost cancel answer
- **GIVEN** the machine page of `M12` that shows the message `E-WEB-015` after a lost answer of the cancel form
- **WHEN** the status part of the link panel sends its next poll
- **THEN** `#error-panel` still holds the message `E-WEB-015`

#### Scenario: Network error of a poll
- **GIVEN** the machine page of `M12` with the success message of a cancel in `#toast`
- **WHEN** a poll of the status part fails with a network error
- **THEN** `#error-panel` holds no message `E-WEB-015`
- **AND** `#toast` keeps the success message

### Requirement: No inline script or style
A page SHALL load only the scripts `htmx.min.js`, `row-version.js`, `tree.js`, and `request-state.js` from `/static/`. `row-version.js` SHALL have at most 20 lines, `tree.js` SHALL have at most 50 lines, and `request-state.js` SHALL have at most 45 lines. A page SHALL hold no inline script, no `<style>` element, no `style` attribute, and no inline event handler (R38, A55).

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
- **THEN** `row-version.js` has at most 20 lines, `tree.js` has at most 50 lines, and `request-state.js` has at most 45 lines

### Requirement: Unavailable response
When a timeout, a deadlock, pool exhaustion, or a database outage occurs, the server SHALL return `503` with the fragment `error.html`. The header `Retry-After` SHALL hold the value of the setting `http.retry_after`, which is 5 seconds. On a path under `/export/v1/` and under `/adapter/v1/`, the answer SHALL be the JSON error body with the same header. A passive request SHALL get `204` instead (R36, R39, A55).

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

#### Scenario: Pool exhausted during a status report
- **GIVEN** a connection pool with no free connection
- **WHEN** `plc-m12` sends `POST /adapter/v1/reports`
- **THEN** the answer is a JSON object with the code `E-DB-004` and the status `503`
- **AND** the answer has the header `Retry-After: 5` and no `HX-*` header

#### Scenario: Pool exhausted during a poll
- **GIVEN** a connection pool with no free connection
- **WHEN** the machine page of `M12` sends a passive request
- **THEN** the server returns `204`

### Requirement: Internal error response
A caught panic, a failed `spawn_blocking` task, or an unexpected database error SHALL return `500` with the fragment `error.html`. The response SHALL show the error code and the `request_id` only. A path under `/export/v1/` or `/adapter/v1/` SHALL get the JSON error body instead. A passive request SHALL get `204` instead. A panic outside the error layer SHALL get a plain `500` text with `E-WEB-010` and the header `X-Request-Id` only (R30, R36, A41, A55).

#### Scenario: Unexpected database error
- **GIVEN** a database error with an unexpected SQLSTATE
- **WHEN** a ProcessEngineer sends a request for `GET /recipes/R-100/versions/3`
- **THEN** the server returns `500 E-DB-008` with the fragment `error.html`
- **AND** the response shows the `request_id`
- **AND** the response shows no SQL, no error chain, and no file path

#### Scenario: Unexpected database error of a claim
- **GIVEN** a database error with an unexpected SQLSTATE
- **WHEN** `plc-m12` sends `POST /adapter/v1/load-requests/{request_id}/claim`
- **THEN** the answer is a JSON object with the code `E-DB-008` and the status `500`
- **AND** the answer has no `HX-*` header

#### Scenario: Panic during an HTMX request
- **GIVEN** an HTMX request whose handler panics
- **WHEN** the answer arrives
- **THEN** the answer has the status 500, the code `E-WEB-010`, and the header `HX-Retarget: #error-panel`

#### Scenario: Panic outside the error layer
- **GIVEN** a request whose panic occurs in a layer outside `render_errors`
- **WHEN** the backstop catches the panic
- **THEN** the answer has the status 500, the text of `E-WEB-010`, and the header `X-Request-Id`
- **AND** the answer has no `HX-*` header and no JSON body

#### Scenario: Panic during a poll
- **GIVEN** a passive request whose handler panics inside the error layer
- **WHEN** the answer arrives
- **THEN** the server returns `204`

### Requirement: Read-only error response
In read-only mode, a command other than `Login`, `Logout`, `ExportVersion`, `ExportPlcConnection`, and `ReportLoadResult` SHALL return `503 E-WEB-006` with the fragment `error.html` and the read-only banner. The status report SHALL also apply. Under `/adapter/v1/`, the answer SHALL be the JSON error body (A24, A55).

#### Scenario: Save in read-only mode
- **GIVEN** the application in read-only mode
- **WHEN** a ProcessEngineer sends a change of a parameter value
- **THEN** the server returns `503 E-WEB-006` with the fragment `error.html`
- **AND** the page shows the read-only banner

#### Scenario: Claim in read-only mode
- **GIVEN** the application in read-only mode and a `pending` load request of `M12`
- **WHEN** `plc-m12` claims the request
- **THEN** the answer is a JSON object with the code `E-WEB-006` and the status `503`

### Requirement: Body limit of the other routes
The route that writes a translation SHALL accept up to `http.body_limit.translation`. An adapter route under `/adapter/v1/` SHALL accept up to `http.body_limit.adapter`. Every other route SHALL accept up to `http.body_limit.form`. A larger body SHALL give `413 E-WEB-003` (R30, R38, A26, A55, ADR-0211).

#### Scenario: Large rename
- **GIVEN** the recipe `R-100`
- **WHEN** `anowak` sends `POST /recipes/R-100/rename` with a body of 300 KiB
- **THEN** the server returns `413 E-WEB-003`

#### Scenario: Status report above the adapter limit
- **GIVEN** the active `ExportClient` account `plc-m12` and the live machine `M12`
- **WHEN** `plc-m12` sends a status report of 100 KiB, which is above `http.body_limit.adapter` and below `http.body_limit.form`
- **THEN** the server returns `413 E-WEB-003`
- **AND** the system stores no link state

#### Scenario: Status report with the maximum entry count
- **GIVEN** the active `ExportClient` account `plc-m12` and 32 live machines
- **WHEN** `plc-m12` sends a valid status report with 32 entries and a body below `http.body_limit.adapter`
- **THEN** the system applies the report

### Requirement: Language cookie value
The cookie `recipe_lang` SHALL hold `pl` or `en`, and the server SHALL read it on every request, except a request under `/adapter/v1/`. An adapter route SHALL read no cookie and SHALL use the language `pl`. A missing cookie or another value SHALL select the UI language `pl`. The server SHALL ignore the header `Accept-Language` (R10, A17, A55).

#### Scenario: Unknown cookie value
- **GIVEN** the cookie `recipe_lang` with the value `de` and the header `Accept-Language: en-US`
- **WHEN** a user sends a request for `GET /recipes`
- **THEN** the UI uses the language `pl`

#### Scenario: Adapter error with a language cookie
- **GIVEN** the active `ExportClient` account `plc-m12` and the cookie `recipe_lang` with the value `en`
- **WHEN** `plc-m12` sends a status report with a JSON syntax error
- **THEN** the server returns `422 E-LNK-009`
- **AND** `message` of the JSON error body is in the language `pl`

## ADDED Requirements

### Requirement: Link panel of the machine page
The page of a live machine SHALL hold the link panel. The status part `#machine-link-panel` SHALL show the link state with an icon and a text, the report time, and its render time. It SHALL show the loaded recipe with its comparison and the active and last finished request. The action part `#machine-link-actions` SHALL hold the forms outside the status part. No state SHALL show by color alone (R39, A55).

#### Scenario: Machine without a report
- **GIVEN** the live machine `M12` with no status report
- **WHEN** an Operator opens the page of `M12`
- **THEN** the link panel shows the state "no link" as an icon and a text

#### Scenario: Loaded version differs from the approved version
- **GIVEN** the approved version 3 of `R-100` and a fresh report of `M12` that names version 2 of `R-100`
- **WHEN** an Operator opens the page of `M12`
- **THEN** the link panel shows that the PLC holds another version than the approved version

#### Scenario: Request with an unknown result
- **GIVEN** a load request of `M12` in the state `claimed` after its result deadline
- **WHEN** an Operator opens the page of `M12`
- **THEN** the link panel shows the state "unknown result" and the instruction to check the machine
- **AND** `#machine-link-actions` holds the release form of that request

#### Scenario: Last finished request
- **GIVEN** a load request of `M12` in the state `succeeded` and no active request
- **WHEN** an Operator opens the page of `M12`
- **THEN** the status part shows the state "succeeded" of that request as an icon and a text

### Requirement: Refresh of the link panel
The status part of the link panel SHALL refresh every `ui.link_poll_interval` by HTMX polling. Its `hx-*` attributes SHALL hold only literals, with an empty `hx-get`, and it SHALL have `data-passive`. It SHALL send its hidden input `machine_id`. A passive request of a live machine with that id SHALL give only the status part (R39, A55).

#### Scenario: Panel request
- **GIVEN** the live machine `M12` and an Operator with an open session
- **WHEN** the browser sends `GET /machines/M12` with `HX-Request: true` and `HX-Target: machine-link-panel`
- **THEN** the server returns only the status part with status `200`
- **AND** the answer holds no form

#### Scenario: Panel request by identifier
- **GIVEN** the live machine `M12` and an Operator with an open session
- **WHEN** the browser sends `GET /machines/by-id/` with the identifier of `M12`, `HX-Request: true`, and `HX-Target: machine-link-panel`
- **THEN** the server returns only the status part with status `200`

#### Scenario: Literal attributes
- **GIVEN** the template of the machine page
- **WHEN** the template test reads each `hx-*` attribute of the link panel
- **THEN** each attribute holds a literal and no machine code
- **AND** the interval of `hx-trigger` equals `ui.link_poll_interval`

#### Scenario: Poll forms stay in place
- **GIVEN** the machine page of `M12` with the keyboard focus on the cancel button of a `pending` request
- **WHEN** the status part refreshes
- **THEN** the cancel button keeps the focus

### Requirement: End of the poll
A passive request for an archived or missing machine SHALL give `200` with a terminal status part. A passive request with a missing, malformed, or other `machine_id` SHALL give the same answer. The terminal status part SHALL say that the machine changed and that the page needs a reload. It SHALL have no `hx-get` and no `hx-trigger` (R39, A32, A55).

#### Scenario: Poll after an archive
- **GIVEN** the page of the live machine `M12`, opened by `GET /machines/by-id/` with its identifier
- **WHEN** another person archives `M12` and the status part sends its next poll
- **THEN** the server returns `200` with the terminal status part
- **AND** the page sends no further poll

#### Scenario: Poll after a reuse of the code
- **GIVEN** the page of `M12` that the server rendered before `M12` was archived and a new machine got the code `M12`
- **WHEN** the status part sends its next poll with the `machine_id` of the old machine
- **THEN** the server returns `200` with the terminal status part
- **AND** the answer holds no state of the new machine

### Requirement: Failed poll of the link panel
A passive request that fails SHALL get `204`, except for `E-WEB-004`. The page SHALL drop the swap of an error answer to a passive request. The status part SHALL then keep its content (R31, R39, A55).

#### Scenario: Failed poll
- **GIVEN** the machine page of `M12` with a status part rendered at 10:00:05
- **WHEN** a poll of the status part gets a database outage
- **THEN** the server returns `204`
- **AND** the status part keeps its content and the render time 10:00:05

#### Scenario: Proxy answer during a restart
- **GIVEN** the machine page of `M12` and the service `app` that does not answer
- **WHEN** a poll of the status part gets the `502` answer of the proxy with `HX-Retarget: #error-panel`
- **THEN** the status part keeps its content
- **AND** `#error-panel` and `#toast` do not change

### Requirement: Load form of an approved version
The version page SHALL hold the load form `POST /recipes/{ref}/versions/{n}/load` for a person with `RecipeLoadRequest`, only for a loadable `Approved` version of a live machine. The form SHALL hold the hidden field `load_request_id` with a new UUID of each render. It SHALL have `hx-sync="closest form:drop"`, no `hx-disabled-elt`, `hx-target="#version-load"`, and `hx-swap="outerHTML"` (R31, R39, A55, ADR-0049).

#### Scenario: Load form for an Operator
- **GIVEN** the approved version 3 of `R-100` and an Operator with an open session
- **WHEN** the Operator opens the page of version 3 of `R-100`
- **THEN** the page holds the load form with a `load_request_id`

#### Scenario: No load form on a draft
- **GIVEN** the draft version 4 of `R-100` and an Operator with an open session
- **WHEN** the Operator opens the page of version 4 of `R-100`
- **THEN** the page holds no load form

#### Scenario: No load form on an archived machine
- **GIVEN** the approved version 3 of `R-100` on the archived machine `M12` and an Operator with an open session
- **WHEN** the Operator opens the page of version 3 of `R-100`
- **THEN** the page holds no load form

#### Scenario: No load form on a version that is not loadable
- **GIVEN** the approved version 3 of `R-100` on `M12` with `loadable = false` and an Operator with an open session
- **WHEN** the Operator opens the page of version 3 of `R-100`
- **THEN** the page holds no load form

#### Scenario: Refused load
- **GIVEN** the approved version 3 of `R-100` on `M12` and a fresh link state `running`
- **WHEN** the Operator sends the load form of version 3
- **THEN** the server returns `409` with the error fragment of `E-LNK-002` in `#error-panel`

#### Scenario: Retry of the load form
- **GIVEN** a load form of version 3 of `R-100` whose first answer was lost after the commit
- **WHEN** the Operator sends the same form again
- **THEN** the server returns the fragment of the stored request
- **AND** the system stores no second load request

### Requirement: Answer of the load form
A success of the load form SHALL replace `#version-load` with a fragment and a fresh load form with a new `load_request_id`. The fragment SHALL name the state of the request and link to the machine page. Without HTMX, the form SHALL give `303` to the machine page. For a machine with the string encoding `Utf8`, the version page SHALL show a notice and no load form (R31, R39, A55).

#### Scenario: Success of the load form
- **GIVEN** the approved version 3 of `R-100` on `M12` and a fresh link state `ready_for_changeover`
- **WHEN** the Operator sends the load form of version 3
- **THEN** `#version-load` holds the fragment with the state `pending` and a load form with a new `load_request_id`

#### Scenario: Known id of a finished request
- **GIVEN** a load request `L1` of version 3 of `R-100` in the state `succeeded`
- **WHEN** a client sends the load form with the `load_request_id` `L1` again
- **THEN** the fragment names the state `succeeded` of `L1`

#### Scenario: Machine with the encoding Utf8
- **GIVEN** the approved version 1 of `R-300` on the live machine `M14` with the string encoding `Utf8`
- **WHEN** an Operator opens the page of version 1 of `R-300`
- **THEN** the page shows the notice that the machine gets no load and holds no load form

### Requirement: Cancel form of a pending load request
The action part of the link panel SHALL hold a cancel form `POST /machines/{code}/load-requests/{request_id}/cancel` for a request in the effective state `pending` and a person with `RecipeLoadRequest`. A success SHALL give a fresh action part. A malformed or unknown `{request_id}`, or a request of another machine, SHALL give `404 E-WEB-005` (R38, R39, A55).

#### Scenario: Cancel form of a pending request
- **GIVEN** a `pending` load request of `M12` and an Operator with an open session
- **WHEN** the Operator opens the page of `M12`
- **THEN** the action part holds the cancel form of that request

#### Scenario: Malformed id
- **GIVEN** an Operator with an open session
- **WHEN** the Operator sends `POST /machines/M12/load-requests/abc/cancel`
- **THEN** the server returns `404 E-WEB-005`

#### Scenario: Request of another machine
- **GIVEN** a `pending` load request `L1` of `M13` and an Operator with an open session
- **WHEN** the Operator sends `POST /machines/M12/load-requests/L1/cancel`
- **THEN** the server returns `404 E-WEB-005`
- **AND** `L1` keeps the state `pending`

### Requirement: Release form of a load request with an unknown result
The action part of the link panel SHALL hold a release form `POST /machines/{code}/load-requests/{request_id}/release` for a request in the effective state `unknown` and a person with `RecipeLoadRequest`. The form SHALL show the instruction to check the machine before the release. A success SHALL give a fresh action part (R31, R39, A55).

#### Scenario: Release form of an unknown request
- **GIVEN** a load request of `M12` in the stored state `claimed` after its result deadline and an Operator with an open session
- **WHEN** the Operator opens the page of `M12`
- **THEN** the action part holds the release form and the instruction to check the machine

#### Scenario: Release of a request with a running claim
- **GIVEN** a load request of `M12` in the state `claimed` before its result deadline
- **WHEN** the Operator sends the release form of that request
- **THEN** the server returns `409` with the error fragment of `E-LNK-017` in `#error-panel`

### Requirement: Attributes and answers of the link forms
The cancel form and the release form SHALL have `hx-sync="closest form:drop"`, no `hx-disabled-elt`, and the literals `hx-target="#machine-link-actions"`, `hx-swap="outerHTML"`, and `hx-push-url="false"`. Each handler SHALL build its answer from the command result and SHALL read nothing after the commit. Without HTMX, each form SHALL give `303` to `/machines/{code}` (R31, R38, R39, A55).

#### Scenario: Literal attributes of the cancel form
- **GIVEN** a `pending` load request of `M12` and an Operator with an open session
- **WHEN** the template test reads the cancel form of the page of `M12`
- **THEN** the form has `hx-push-url="false"` and `hx-target="#machine-link-actions"`

#### Scenario: Two polls after a cancel
- **GIVEN** the page of `M12` with a `pending` load request
- **WHEN** the Operator cancels the request and the status part sends two polls
- **THEN** the address of the page stays `/machines/M12`
- **AND** each poll updates the status part

#### Scenario: Failed read after the commit
- **GIVEN** a `pending` load request of `M12` and a read port that fails after the commit
- **WHEN** the Operator cancels the request
- **THEN** the server returns the fresh action part with status `200`

### Requirement: Load controls in read-only mode
In read-only mode, the version page SHALL hold no load form. The link panel SHALL hold no cancel form and no release form. The status part SHALL show no hint to reload the page for a release (A24, A55, ADR-0113).

#### Scenario: Version page with a full disk
- **GIVEN** the application in read-only mode and the approved version 3 of `R-100`
- **WHEN** an Operator opens the page of version 3 of `R-100`
- **THEN** the page holds the export link and no load form

#### Scenario: Machine page with a full disk
- **GIVEN** the application in read-only mode and a load request of `M12` in the stored state `claimed` after its result deadline
- **WHEN** an Operator opens the page of `M12`
- **THEN** the link panel shows the state "unknown result" and holds no release form
- **AND** the status part shows no hint to reload the page

### Requirement: Failure reason without free text
The link panel SHALL show the reason of a failed load request as a word of the UI bundle. It SHALL show the extent of the write as a word of the bundle, also for `unknown`, and the vendor code as a number. It SHALL show no text that the adapter sent (R38, A55).

#### Scenario: Failed request with a vendor code
- **GIVEN** a load request of `M12` in the state `failed` with the reason `interlock_refused`, `written` `none`, and the vendor code 1793
- **WHEN** an Operator with the UI language `en` opens the page of `M12`
- **THEN** the link panel shows the word of `interlock_refused` from the bundle `en` and the number 1793

### Requirement: Load request entries on the audit pages
For a role with `AuditView`, the selects of `/audit` SHALL offer the entity type `load_request` and the seven actions of a load request. The entry page SHALL label such an entry with the machine code, the reference, and the version. A role with only `AuditViewRecipes` SHALL NOT see these entries, because their aggregate type is `machine` (R20, R28, A55, ADR-0051).

#### Scenario: Filter of an Administrator
- **GIVEN** the Administrator `admin` with an open session
- **WHEN** `admin` sends `GET /audit`
- **THEN** the select of `entity_type` offers `load_request`
- **AND** the select of `action` offers `RecipeLoadClaimed`

#### Scenario: Entry page of a claim
- **GIVEN** the entry `RecipeLoadClaimed` of a load request of `M12` for the version 3 of `R-100`
- **WHEN** `admin` opens the entry page of that entry
- **THEN** the page labels the object with `M12`, `R-100`, and the version 3

#### Scenario: ProcessEngineer and a load request entry
- **GIVEN** the entry `RecipeLoadClaimed` of a load request of `M12`
- **WHEN** the ProcessEngineer `anowak` sends `GET /audit`
- **THEN** the select of `entity_type` does not offer `load_request`
- **AND** the list holds no `RecipeLoadClaimed` entry
