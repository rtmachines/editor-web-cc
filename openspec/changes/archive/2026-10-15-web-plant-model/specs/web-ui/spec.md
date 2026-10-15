## ADDED Requirements

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
A search text of spaces only SHALL count as no search text. A page after the last page of the machine list or the usage list SHALL show the text of `list-page-after-end`. It SHALL also show a link to the first page (R24, R39).

#### Scenario: Search text of spaces only
- **GIVEN** the ProcessEngineer `anowak` with an open session and the live machine `M7`
- **WHEN** `anowak` sends `GET /machines` with `q` set to three spaces
- **THEN** the server returns `200` with a table row of `M7`

#### Scenario: Page after the end
- **GIVEN** the ProcessEngineer `anowak` with an open session and one live machine
- **WHEN** `anowak` sends `GET /machines?page=2`
- **THEN** the page shows the text of `list-page-after-end` and a link to `/machines?page=1`
- **AND** the page shows no text of the empty state

### Requirement: Content answer of a page
An HTMX request with `HX-Target: content` to a plant page SHALL get the content of `main` and the `title` element. The answer SHALL also update the field `next` of the language switch out of band. The `h1` of that answer SHALL have `tabindex="-1"` and `autofocus`. A request without `HX-Request` SHALL get the full page (R39, A44).

#### Scenario: Machine page after a create
- **GIVEN** the ProcessEngineer `anowak` with an open session and the live machine `M7`
- **WHEN** `anowak` sends an HTMX request for `GET /machines/M7` with `HX-Target: content`
- **THEN** the answer holds the `title` element, `#error-panel`, `#toast`, and the `h1` with `autofocus`
- **AND** the answer holds no top bar and no skip link
- **AND** the answer holds the field `next` with the value `/machines/M7` and `hx-swap-oob="true"`

### Requirement: Machine form and creation
`GET /machines/new` SHALL show a form with the code, the name, a list of the five PLC platforms, and the name of the main unit. The list SHALL start with an empty option, so the user chooses the platform. A valid `POST /machines` SHALL answer `303` to `/machines/{code}`. A rejected HTMX `POST /machines` SHALL put the error into `#error-panel` and SHALL keep the form with the input of the user (R05, R06, R15, R39).

#### Scenario: Create an S7-300 machine
- **GIVEN** the ProcessEngineer `anowak` with an open session and no live machine `M7`
- **WHEN** `anowak` sends `POST /machines` with the code `M7`, the name `Press line`, the platform `S7_300`, and the main unit name `Infeed`
- **THEN** the server returns `303` to `/machines/M7`

#### Scenario: Code of a live machine
- **GIVEN** the ProcessEngineer `anowak` with an open session and the live machine `M7`
- **WHEN** `anowak` sends an HTMX `POST /machines` with the code `M7`
- **THEN** the server returns `409 E-PLT-001` with `HX-Retarget: #error-panel`

#### Scenario: No platform chosen
- **GIVEN** the ProcessEngineer `anowak` with an open session
- **WHEN** `anowak` sends `GET /machines/new`
- **THEN** the first option of the platform list has an empty value and is selected
- **AND** an HTMX `POST /machines` with an empty `plc_platform` gets `422 E-WEB-008` with a line for the platform field

### Requirement: Machine page
`GET /machines/{code}` SHALL show the code, the name, the PLC platform, the unit tree, and a link to the type list of the live machine. A user with `MachineManage` SHALL also get the rename form and the archive form inside a `details` element. The page of an archived machine SHALL show the state "Archived", no form, and no link to a route under `/machines/{code}` (R05, R06, R15, A32).

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

## MODIFIED Requirements

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

### Requirement: Error fragment content
Every error fragment SHALL show the error code `E-<CTX>-<nnn>`, one sentence in the UI language that tells the user what to do, and the `request_id`. Each code of the routes of `api.md` sections 2.2 and 2.3 SHALL have its sentence in the bundles `pl` and `en` (R10, R36).

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

### Requirement: Static files
`GET /static/*` SHALL serve the vendored HTMX 2 file, the style sheets `app.css` and `components.css`, the scripts `row-version.js` and `tree.js`, and the 8 font files of `design-system.md` section 5. Each file SHALL be unchanged and SHALL have its media type. The route SHALL need no session. The response SHALL set `Cache-Control: public, max-age` with the value of `static.cache_max_age` (R39).

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
With a session, the top bar SHALL show the link to a page only when the role holds the permission of that page. The links SHALL come in the order recipes, machines, users, own profile. The use case of that page SHALL still check the permission. The current link SHALL have `aria-current="page"` and SHALL show a bottom border and the weight 700. A session with `must_change_password` SHALL show no navigation link (R28, R39).

#### Scenario: Navigation of an Operator
- **GIVEN** the Operator `mhoffman` with an open session
- **WHEN** `mhoffman` sends `GET /recipes`
- **THEN** the top bar shows the links to `/recipes`, `/machines`, and `/me`
- **AND** the top bar shows no link to `/users`
- **AND** the link to `/recipes` has `aria-current="page"`

#### Scenario: Navigation of an Administrator
- **GIVEN** the Administrator `akowal` with an open session
- **WHEN** `akowal` sends `GET /me`
- **THEN** the top bar shows the links to `/recipes`, `/machines`, `/users`, and `/me` in this order

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

### Requirement: State shown by color, icon, and word
A version state, a user state, and the state "Archived" of a machine or a unit SHALL show a color, an icon, and a word. Each error, warning, information, and success message SHALL also show a color, an icon, and a word. No element SHALL show a state by its color alone (R39, A44).

#### Scenario: State of a draft version
- **GIVEN** the draft version 3 of the recipe `R-100` and the UI language `pl`
- **WHEN** a page shows the state of version 3
- **THEN** the page shows the icon `status-draft` and the word `Szkic` next to it

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
