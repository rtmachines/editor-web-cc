## ADDED Requirements

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

## MODIFIED Requirements

### Requirement: Machine form and creation
`GET /machines/new` SHALL show a form with the code, the name, and a list of the five PLC platforms. The form SHALL also show the data block number and the name of the main unit. The list SHALL start with an empty option, so the user chooses the platform. The hint of the data block field SHALL say that only `S7_300`, `S7_1200`, and `S7_1500` use it. A valid `POST /machines` SHALL answer `303` to `/machines/{code}`. A rejected HTMX `POST /machines` SHALL put the error into `#error-panel` and SHALL keep the form with the input of the user (R05, R06, R15, R39, A48).

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

### Requirement: Machine page
`GET /machines/{code}` SHALL show the code, the name, the PLC platform, the unit tree, and a link to the type list of the live machine. The page of a machine on `S7_300`, `S7_1200`, or `S7_1500` SHALL also show its data block number. A user with `MachineManage` SHALL also get the rename form, the data block form, and the archive form inside a `details` element. The page of an archived machine SHALL show the state "Archived", no form, and no link to a route under `/machines/{code}` (R05, R06, R15, A32, A48).

#### Scenario: Page of a live machine for an Operator
- **GIVEN** the Operator `mhoffman` with an open session and the live machine `M7`
- **WHEN** `mhoffman` sends `GET /machines/M7`
- **THEN** the page shows `M7`, its name, its platform, and its unit tree
- **AND** the page holds no form that sends a `POST` request to `/machines/M7`

#### Scenario: Data block number of an S7 machine
- **GIVEN** the Operator `mhoffman` with an open session and the live machine `M5` with the platform `S7_1500` and the data block number `100`
- **WHEN** `mhoffman` sends `GET /machines/M5`
- **THEN** the facts of the machine show the data block number `100`

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
