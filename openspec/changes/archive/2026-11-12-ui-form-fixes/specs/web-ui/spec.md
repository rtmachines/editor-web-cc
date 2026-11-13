## MODIFIED Requirements

### Requirement: Form of a new parameter
On a version page that holds `#parameter-rows`, the link Add a parameter SHALL load the form of a new parameter into `#parameter-form`. The button Show value fields SHALL run `GetParameterForm` with the sent type and SHALL write nothing. `AddParameter` SHALL answer the row of the new parameter and SHALL clear `#parameter-form` out of band (R08, R25).

A version page after the last page of its parameter list has no `#parameter-rows`. On that page the link Add a parameter SHALL load the page form of a new parameter into `#content` and SHALL push its address. That form SHALL target `#content` and no other region of the version page (R25, R39).

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

### Requirement: UDT form with member rows
`GET /machines/{code}/types/new` SHALL accept only the field `from`. The form SHALL hold the name of the struct and one `fieldset` of member rows. Each row SHALL hold a name input and a type input with a `datalist` of the types of the machine. The button "Add a member" SHALL send the form with `POST` and `add=1`. The server SHALL then create no type and SHALL show the sent rows and one more empty row (R13, R39).

The button that creates the type or the revision SHALL be the first submit button of the form in the DOM order. The button "Add a member" SHALL come after it. Enter in an input SHALL therefore send the form without `add` at every count of rows (R13, R39, A44).

The `plant-model` requirement "Struct revision" states the refusal of an unchanged revision with `E-PLT-052`. For that refusal the server SHALL return `409`. The server SHALL show the error in the summary of `#error-panel` with no field line. The form SHALL keep the sent rows (R06, R07, ADR-0159).

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

### Requirement: Addition of a unit
The link "Add a sub-unit" SHALL load the unit form into `#unit-form` with an `h2` that names the parent and `autofocus` on its first input. A valid HTMX `POST /machines/{code}/units` SHALL answer `200` with the whole tree, the ancestors of the new unit expanded, and `autofocus` on its link. The answer SHALL put a success message into `#toast` and SHALL empty `#error-panel` (R06, R07, R39, A30).

The unit form SHALL keep the parent of its link in a hidden field `parent_path` and SHALL show no control that changes the parent. The form SHALL show the parent in a text line. The `h2`, the text line, the proposed segment, and the sent parent SHALL always name the same parent. The form of an error answer SHALL keep that parent. An error of the parent SHALL stand in the summary without a link (R06, R07, R39, ADR-0041).

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

#### Scenario: Proposed segment saves in the browser
- **GIVEN** the ProcessEngineer `anowak` on the machine page of `M7` with the units `U1` and `U1.1`
- **WHEN** `anowak` follows "Add a sub-unit" of `U1`, types a name, and sends the form with the proposed segment
- **THEN** the tree shows the new unit `U1.2`
- **AND** `#error-panel` is empty
