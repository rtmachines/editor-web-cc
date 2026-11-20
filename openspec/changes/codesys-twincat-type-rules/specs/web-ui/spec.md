## ADDED Requirements

### Requirement: STRING encoding field of the machine form
`GET /machines/new` SHALL show the field `string_encoding` with the options `Latin1` and `Utf8`. The form SHALL propose `Latin1`. A rejected HTMX `POST /machines` with `E-PLT-053` SHALL put the error into `#error-panel` and SHALL show a line under the field `string_encoding` (R15, R39, ADR-0041, ADR-0202).

#### Scenario: Form proposes Latin1
- **GIVEN** the ProcessEngineer `anowak` with an open session
- **WHEN** `anowak` sends `GET /machines/new`
- **THEN** the field `string_encoding` has the options `Latin1` and `Utf8`
- **AND** the form selects the option `Latin1`

#### Scenario: UTF-8 on an S7 machine through HTMX
- **GIVEN** the ProcessEngineer `anowak` with an open session and no live machine `M7`
- **WHEN** `anowak` sends an HTMX `POST /machines` with the code `M7`, the platform `S7_1500`, the data block number `100`, and the encoding `Utf8`
- **THEN** the server returns `422 E-PLT-053` with `HX-Retarget: #error-panel`
- **AND** the answer swaps the form out of band with a line under the field `string_encoding`

### Requirement: STRING encoding on the machine page
`GET /machines/{code}` SHALL show the `STRING` encoding of the machine. On a live `CodesysIec` or `TwinCat3` machine without a version row, a user with `MachineManage` SHALL get a form that sets the encoding, except in read-only mode (A24). A valid HTMX `POST /machines/{code}/string-encoding` SHALL answer `200` with the new encoding, a success message in `#toast`, and an empty `#error-panel`. A request without `HX-Request` SHALL get `303` to `/machines/{code}` (R15, R39, ADR-0202).

#### Scenario: Page shows the encoding
- **GIVEN** a user with `RecipeView` and the live machine `M5` with the encoding `Latin1`
- **WHEN** the user sends `GET /machines/M5`
- **THEN** the page shows the encoding `Latin1`

#### Scenario: Set through HTMX
- **GIVEN** the ProcessEngineer `anowak` with an open session and the live machine `M9` with the platform `CodesysIec` and no recipe
- **WHEN** `anowak` sends an HTMX `POST /machines/M9/string-encoding` with the encoding `Utf8`
- **THEN** the server returns `200` with the encoding `Utf8`
- **AND** the answer puts the icon `alert-success` into `#toast` and empties `#error-panel`

#### Scenario: Set without HTMX
- **GIVEN** the ProcessEngineer `anowak` with an open session and the live machine `M9` with no recipe
- **WHEN** `anowak` sends `POST /machines/M9/string-encoding` with the encoding `Utf8` and no header `HX-Request`
- **THEN** the server answers `303` to `/machines/M9`

#### Scenario: No form on a machine with a version
- **GIVEN** the ProcessEngineer `anowak` with an open session and the live machine `M9` with the draft version 1 of `R-300`
- **WHEN** `anowak` sends `GET /machines/M9`
- **THEN** the page shows the encoding of `M9`
- **AND** the page holds no form that sends a `POST` request to `/machines/M9/string-encoding`

#### Scenario: Frozen encoding through HTMX
- **GIVEN** the ProcessEngineer `anowak` with an open session and the live machine `M9` with the draft version 1 of `R-300`
- **WHEN** `anowak` sends an HTMX `POST /machines/M9/string-encoding` with the encoding `Utf8`
- **THEN** the server returns `409 E-PLT-054` with `HX-Retarget: #error-panel`
- **AND** the answer removes the encoding form out of band

#### Scenario: No encoding form on an S7 machine
- **GIVEN** the ProcessEngineer `anowak` with an open session and the live machine `M5` with the platform `S7_1500` and no recipe
- **WHEN** `anowak` sends `GET /machines/M5`
- **THEN** the page holds no form that sends a `POST` request to `/machines/M5/string-encoding`

#### Scenario: Read-only mode
- **GIVEN** the system in read-only mode and the live machine `M9` with the platform `CodesysIec` and no recipe
- **WHEN** the ProcessEngineer `anowak` sends `GET /machines/M9`
- **THEN** the page holds no form that sends a `POST` request to `/machines/M9/string-encoding`
- **AND** a direct `POST /machines/M9/string-encoding` with a valid token gives `503 E-WEB-006`
- **AND** the encoding of `M9` and the audit log do not change

#### Scenario: Operator without MachineManage
- **GIVEN** the Operator `jkowal` with an open session and the live machine `M9` with no recipe
- **WHEN** `jkowal` sends `POST /machines/M9/string-encoding` with a valid CSRF token and the encoding `Utf8`
- **THEN** the server answers `403 E-WEB-002`
- **AND** the table `security_event` holds one new row of the refusal
- **AND** the encoding of `M9` does not change

#### Scenario: Missing CSRF token
- **GIVEN** the ProcessEngineer `anowak` with an open session and the live machine `M9` with no recipe
- **WHEN** `anowak` sends `POST /machines/M9/string-encoding` with no CSRF token or with a bad token
- **THEN** the server answers `403 E-WEB-007`
- **AND** the table `security_event` holds one new row `csrf_rejected`
- **AND** the encoding of `M9` does not change

### Requirement: Notice of a v1 document of a Utf8 machine
The version page of an Approved version of a `Utf8` machine SHALL show a notice in the export area. The notice SHALL say that the v1 document is not loadable and that the v2 document is the export of this machine. The export link SHALL name the notice in `aria-describedby`. A version with a rule fault SHALL show the rule notice with its rule codes and the encoding notice. The link SHALL then name both (R03, R39, ADR-0202).

#### Scenario: Approved version of a Utf8 machine
- **GIVEN** the approved version 1 of `R-300` on the machine `M10` with the encoding `Utf8`
- **WHEN** `mhoffman` sends `GET /recipes/R-300/versions/1`
- **THEN** the export area shows the notice of the v1 document
- **AND** the export link names the notice in `aria-describedby`

#### Scenario: Approved version of a Latin1 machine
- **GIVEN** the approved version 2 of `R-100` with the parameter `SPEED : INT` and the value `1200` on a `Latin1` machine
- **WHEN** `mhoffman` sends `GET /recipes/R-100/versions/2`
- **THEN** the export area shows no notice

#### Scenario: Rule fault on a Utf8 machine
- **GIVEN** the approved version 1 of `R-300` on `M10` with the encoding `Utf8` and a stored part that breaks a current rule with `E-RCP-005`
- **WHEN** `mhoffman` sends `GET /recipes/R-300/versions/1`
- **THEN** the export area shows the rule notice with the code `E-RCP-005` and the encoding notice
- **AND** the export link names both notices in `aria-describedby`

#### Scenario: Rule fault on a Latin1 machine
- **GIVEN** the approved version 2 of `R-100` on the `Latin1` machine `M8` with a stored `WSTRING(10)` value `"A😀"`
- **WHEN** `mhoffman` sends `GET /recipes/R-100/versions/2`
- **THEN** the export area shows only the rule notice with its rule codes

### Requirement: Texts of the encoding notice
The notice of a v1 document of a `Utf8` machine SHALL use the key `version-export-encoding-notice` of `en.ftl` and `pl.ftl`, beside the key `version-export-rule-notice` (R36, R39, ADR-0202).

#### Scenario: English text of the notice
- **GIVEN** the approved version 1 of `R-300` on `M10` with the encoding `Utf8` and the UI language `en`
- **WHEN** `mhoffman` sends `GET /recipes/R-300/versions/1`
- **THEN** the notice reads "This machine uses UTF-8 for STRING values. The v1 export file marks this version as not loadable into a PLC. Use the v2 export of each unit."

#### Scenario: Polish text of the notice
- **GIVEN** the approved version 1 of `R-300` on `M10` with the encoding `Utf8` and the UI language `pl`
- **WHEN** `mhoffman` sends `GET /recipes/R-300/versions/1`
- **THEN** the notice reads "Ta maszyna używa UTF-8 dla wartości STRING. Plik eksportu v1 oznacza, że tej wersji nie można załadować do PLC. Użyj eksportu v2 każdej jednostki."

### Requirement: Texts of the new type reasons and expected forms
The form line of `E-PLT-013` SHALL name the reason "reserved member name" or "too few members" in the UI language. The line of `E-RCP-005` SHALL name the two character ranges of a `WSTRING` on `CodesysIec` and `TwinCat3`. On a `Utf8` machine, it SHALL name the length of a `STRING` in bytes (R36, R39, ADR-0201, ADR-0202, ADR-0203, ADR-0204).

#### Scenario: Keyword as a member name in the struct form
- **GIVEN** the ProcessEngineer `anowak` with an open session and the machine `M9` with the platform `CodesysIec`
- **WHEN** `anowak` sends an HTMX `POST` of the struct form for `ST_Coil` with the member `END_IF`
- **THEN** the answer shows `E-PLT-013` with the text of the reason "reserved member name"

#### Scenario: One member in the struct form
- **GIVEN** the ProcessEngineer `anowak` with an open session and the machine `M9` with the platform `CodesysIec`
- **WHEN** `anowak` sends an HTMX `POST` of the struct form for `ST_One` with one member
- **THEN** the answer shows `E-PLT-013` with the text of the reason "too few members"

#### Scenario: WSTRING character outside the plane in the parameter form
- **GIVEN** the ProcessEngineer `anowak` with an open session and a draft of `M8` with the parameter `NOTE` of type `WSTRING(10)`
- **WHEN** `anowak` sends an HTMX `POST` of the value form of `NOTE` with a text that holds `U+1F600`
- **THEN** the answer shows `E-RCP-005` with a line that names the length 10 and the two character ranges

#### Scenario: STRING bytes in the parameter form of a Utf8 machine
- **GIVEN** the ProcessEngineer `anowak` with an open session and a draft of `M10` with the encoding `Utf8` and the parameter `LABEL` of type `STRING(4)`
- **WHEN** `anowak` sends an HTMX `POST` of the value form of `LABEL` with the text `ééé`
- **THEN** the answer shows `E-RCP-005` with a line that names the length 4 in UTF-8 bytes
