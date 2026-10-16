## MODIFIED Requirements

### Requirement: Navigation by permission
With a session, the top bar SHALL show the link to a page only when the role holds the permission of that page. The links SHALL come in the order recipes, machines, users, audit, own profile. The link to `/audit` SHALL show when the row of `ListAuditEntries` allows the role. The use case of that page SHALL still check the permission. The current link SHALL have `aria-current="page"` and SHALL show a bottom border and the weight 700. A session with `must_change_password` SHALL show no navigation link (R28, R39, A20).

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

#### Scenario: Current link on a machine page
- **GIVEN** the ProcessEngineer `anowak` with an open session
- **WHEN** `anowak` sends `GET /machines`
- **THEN** the link to `/machines` has `aria-current="page"`

#### Scenario: Navigation during a password change
- **GIVEN** the user `mhoffman` with `must_change_password` set to true and an open session
- **WHEN** `mhoffman` sends `GET /me/password`
- **THEN** the top bar shows no navigation link
- **AND** the top bar shows the logout button

#### Scenario: Look of the current link
- **GIVEN** the style sheet `/static/app.css`
- **WHEN** the style test reads the rule of `[aria-current]` in the top bar
- **THEN** the rule sets a bottom border of 3 px and the weight 700

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

### Requirement: Version page
`GET /recipes/{ref}/versions/{n}` SHALL show the header of the version and the parameter table with `page.parameters` rows grouped by unit. The rows SHALL come in the order unit path, ordinal, and tag. Only a live `Draft` and a user with `ParameterEdit` SHALL get the edit forms of parameters. The page SHALL show an action form or an action link when the matrix row of its command allows the role. The state of the version SHALL also permit the transition. The page SHALL link to the history of the recipe (R17, R24, R25, A28).

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

#### Scenario: Administrator reads an approved version
- **GIVEN** the Administrator `admin` with an open session and the approved version 2 of `R-100`
- **WHEN** `admin` sends `GET /recipes/R-100/versions/2`
- **THEN** the page shows the retire form in a `details` element with the class `danger`
- **AND** the page shows no edit form of a parameter

### Requirement: Second request of a form
Every approve form, every save form, and every retire, discard, delete, and restore form SHALL have the attribute `hx-sync="closest form:drop"`. The browser SHALL drop a second request of such a form while the first request is in flight. The approve, retire, discard, delete, and restore forms SHALL also have the attribute `hx-disabled-elt="find button"` and an indicator with the text of `form-in-progress` (R39, I12, ADR-0049).

#### Scenario: Second request while a request is in flight
- **GIVEN** a save form of the draft version 3 with a request in flight
- **WHEN** a ProcessEngineer sends the same save form again
- **THEN** the browser drops the second request

#### Scenario: Retire form
- **GIVEN** the page of the approved version 2 of `R-100` for the Administrator `admin`
- **WHEN** the test reads the retire form
- **THEN** the form has `hx-sync="closest form:drop"` and `hx-disabled-elt="find button"`

## ADDED Requirements

### Requirement: Approval preview page
The approval preview SHALL show the diff against the current Approved version with the limits first. It SHALL show the outdated parameters and the version that the approval retires. The checkbox `allow_outdated` SHALL show only with an outdated parameter. Without a blocker, the page SHALL hold one approve form with `csrf_token`, `row_version`, and `hx-sync="closest form:drop"` (R27, A29, A34, ADR-0049).

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
The approval SHALL answer `409 E-RCP-015` to the creator and to each editor of the draft while separation of duties is on. It SHALL answer `409 E-WEB-001` with `conflict.html` for a stale `row_version`. It SHALL answer `422 E-RCP-024` for an outdated parameter without `allow_outdated`, with the error line under that checkbox. An Operator SHALL get `403 E-WEB-002`, and the system SHALL write one security event of kind `forbidden` (R27, R28, A29, A34).

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
The notice of a danger action SHALL have the id `danger-notice` and no role. The danger button SHALL name the notice in `aria-describedby`. A page SHALL hold at most one danger action (R39, A44, ADR-0049).

#### Scenario: Notice of the retire action
- **GIVEN** the page of the approved version 2 of `R-100` for the Administrator `admin`
- **WHEN** the test reads the retire form
- **THEN** the page holds one element with the id `danger-notice`
- **AND** the retire button has `aria-describedby="danger-notice"`

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
