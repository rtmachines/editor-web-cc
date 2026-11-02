## ADDED Requirements

### Requirement: Value column of the parameter table
The parameter table of the version page SHALL have the column Value after the column Type. An elementary parameter SHALL show the text of its value as the parameter page shows it. The cell SHALL show at most `table.value_chars` characters and SHALL end a longer text with `…`. An empty text SHALL show `parameters-value-empty`. An array or a struct SHALL show `parameters-value-composite`. A stored value that does not match its type tree SHALL give `500 E-DB-008` (R08, R39, A43).

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

#### Scenario: Stored value that does not match its type
- **GIVEN** the approved version 2 of `R-100` with the parameter `SPEED : INT`, whose stored value is the text `abc`
- **WHEN** `mhoffman` sends `GET /recipes/R-100/versions/2`
- **THEN** the server returns `500 E-DB-008`

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
