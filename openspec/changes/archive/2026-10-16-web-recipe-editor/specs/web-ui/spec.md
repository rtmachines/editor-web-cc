## MODIFIED Requirements

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

## ADDED Requirements

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
An Administrator SHALL read the recipe page and the version pages of a deleted recipe with the state Deleted and no form. Every `GET` route with `{ref}` SHALL answer `409 E-RCP-012` to every other role (A14, R24).

#### Scenario: Operator opens a deleted recipe
- **GIVEN** the Operator `mhoffman` with an open session and the deleted recipe `R-300`
- **WHEN** `mhoffman` sends `GET /recipes/R-300`
- **THEN** the server returns `409 E-RCP-012`

#### Scenario: Administrator opens a deleted recipe
- **GIVEN** the Administrator `admin` with an open session and the deleted recipe `R-300`
- **WHEN** `admin` sends `GET /recipes/R-300`
- **THEN** the server returns `200` with the state Deleted and no form

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
`GET /recipes/{ref}/versions/{n}` SHALL show the header of the version and the parameter table with `page.parameters` rows grouped by unit. The rows SHALL come in the order unit path, ordinal, and tag. Only a live `Draft` and a user with `ParameterEdit` SHALL get the forms (R17, R24, R25).

#### Scenario: Operator reads a draft
- **GIVEN** the Operator `mhoffman` with an open session and the draft version 3 of `R-100`
- **WHEN** `mhoffman` sends `GET /recipes/R-100/versions/3`
- **THEN** the page shows the parameter table and no form

#### Scenario: Large draft
- **GIVEN** the ProcessEngineer `anowak` with an open session and a draft with 4 000 parameters in 40 units
- **WHEN** `anowak` sends `GET /recipes/R-100/versions/3`
- **THEN** the page shows the parameters of one unit and a link to each unit with the count of its parameters
- **AND** the page shows at most 100 parameter rows
- **AND** the link of the shown unit has `aria-current="page"`

### Requirement: Form of a new parameter
The link Add a parameter of the version page SHALL load the form of a new parameter into `#parameter-form`. The button Show value fields SHALL run `GetParameterForm` with the sent type and SHALL write nothing. `AddParameter` SHALL answer the row of the new parameter and SHALL clear `#parameter-form` out of band (R08, R25).

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
A form answer SHALL list every bad field and every fault of the value and the limits at once. The answer SHALL put the summary into `#error-panel` and SHALL swap the form out of band with an error line under each bad input. The summary SHALL show at most 20 lines and the count of the others. This SHALL apply to the recipe forms and the plant forms (R36, R39, A44).

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
