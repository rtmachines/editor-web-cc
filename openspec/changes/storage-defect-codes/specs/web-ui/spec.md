## ADDED Requirements

### Requirement: Parameter page of a type that does not resolve
The page `GET .../parameters/{key}` SHALL show the definition of a parameter whose stored type does not resolve, without the value grid. A user with `ParameterEdit` on a live draft of a live recipe SHALL get the definition form, when the mode permits `UpdateParameterDefinition`. Every other reader SHALL get the read-only definition (R29, R35).

#### Scenario: Edit form of a parameter with a type cycle
- **GIVEN** the ProcessEngineer `anowak` with an open session, and the draft version 3 of `R-100` with the parameter `ZONE` of a stored struct type that contains itself
- **WHEN** `anowak` opens `GET /recipes/R-100/versions/3/parameters/{key}` for `ZONE`
- **THEN** the server returns `200` with the definition form of `ZONE`
- **AND** the page shows no value grid

#### Scenario: Read-only page of a parameter with a type cycle
- **GIVEN** the Operator `mhoffman` with an open session, and the draft version 3 of `R-100` with the parameter `ZONE` of a stored struct type that contains itself
- **WHEN** `mhoffman` opens `GET /recipes/R-100/versions/3/parameters/{key}` for `ZONE`
- **THEN** the server returns `200` with the read-only definition of `ZONE`
- **AND** the page shows no value grid

#### Scenario: Page of a parameter with a type cycle in read-only mode
- **GIVEN** the ProcessEngineer `anowak` with an open session, and the draft version 3 of `R-100` with the parameter `ZONE` of a stored struct type that contains itself
- **AND** the system is in read-only mode
- **WHEN** `anowak` opens `GET /recipes/R-100/versions/3/parameters/{key}` for `ZONE`
- **THEN** the server returns `200` with the read-only definition of `ZONE`
- **AND** the page shows no value grid

#### Scenario: New type through the page
- **GIVEN** the ProcessEngineer `anowak` with an open session, and the draft version 3 of `R-100` with the parameter `ZONE` of a stored struct type that contains itself
- **AND** `ZONE` stands at `DB6.DBB0` and has a translation in `pl`
- **WHEN** `anowak` opens the page of `ZONE` and sends its edit form with the type `INT` and the S7 address `DB6.DBW0`
- **THEN** the server shows the edit form with the leaf grid of `INT`
- **AND** the server writes nothing
- **WHEN** `anowak` sends that form with its `row_version` and the value `0`
- **THEN** the server stores `ZONE` with the type `INT` and the value `0`
- **AND** the page of `ZONE` then shows the value grid of `INT`, the old key, and the translation in `pl`

#### Scenario: Page of an approved version with a type cycle
- **GIVEN** the ProcessEngineer `anowak` with an open session, and the approved version 2 of `R-100` with the parameter `ZONE` of a stored struct type that contains itself
- **WHEN** `anowak` opens `GET /recipes/R-100/versions/2/parameters/{key}` for `ZONE`
- **THEN** the server returns `200` with the read-only definition of `ZONE`
- **AND** the page shows no value grid

### Requirement: Parameter table of a type that does not resolve
The parameter table of a version that is not approved SHALL show a parameter whose stored type does not resolve without a value text. A row without a type text SHALL get an empty value cell. A row with a type text SHALL show the word of `parameters-value-composite`. The table of an approved version SHALL keep the `500 E-DB-008` of `GetVersion` (ADR-0118 (d), ADR-0119 (l)) (R29, R35).

#### Scenario: Parameter table with a type cycle
- **GIVEN** the ProcessEngineer `anowak` with an open session, and the draft version 3 of `R-100` with the parameter `ZONE` of a stored struct type that contains itself
- **WHEN** `anowak` opens `GET /recipes/R-100/versions/3/parameters`
- **THEN** the server returns `200` with a row of `ZONE` without a value text
- **AND** the value cell of `ZONE` shows the word of `parameters-value-composite`

#### Scenario: Parameter table with a missing type row
- **GIVEN** the ProcessEngineer `anowak` with an open session, and the draft version 3 of `R-100` with the parameter `GRID`
- **AND** the platform does not hold the stored type row of `GRID`
- **WHEN** `anowak` opens `GET /recipes/R-100/versions/3/parameters`
- **THEN** the server returns `200` with a row of `GRID` without a type text
- **AND** the value cell of `GRID` is empty

#### Scenario: Parameter table of an approved version with a missing type row
- **GIVEN** the ProcessEngineer `anowak` with an open session, and the approved version 2 of `R-100` with the parameter `ZONE`
- **AND** the platform does not hold the stored type row of `ZONE`
- **WHEN** `anowak` opens `GET /recipes/R-100/versions/2/parameters`
- **THEN** the server returns `500 E-DB-008`

## MODIFIED Requirements

### Requirement: One edit form of a parameter
The parameter page of a live `Draft` SHALL hold one edit form with the definition fields, the S7 address, and the leaf grid. The form SHALL send the value and the limits together to `/definition`. The page SHALL hold no second form with a definition field of that parameter. A parameter whose stored type does not resolve follows the requirement "Parameter page of a type that does not resolve" (ADR-0195) (R09, R25, I12).

#### Scenario: Value and limits in one save
- **GIVEN** the parameter `SPEED` of type `INT` with the value 1500 and the limit 0 to 3000
- **WHEN** the ProcessEngineer `anowak` sends the edit form with the value 3500 and the limit 3000 to 4000
- **THEN** the server returns `200` with the row of `SPEED` and the header `HX-Trigger`
- **AND** the answer swaps the form out of band with the saved texts and no error line

#### Scenario: Enter saves
- **GIVEN** the edit form of `SPEED`
- **WHEN** the browser sends the form for Enter in a value input
- **THEN** the request holds no field `fields` and no field `limit_mode`

### Requirement: Field lines of a stored part that breaks a newer rule
The parameter page, the diff page, and the approval preview SHALL show a stored value, limit, or S7 address unchanged. A part that fails only the current rule set SHALL get a field line with its code, and the answer SHALL stay 200. A part that fails the earlier rule set of ADR-0120 SHALL give `500 E-DB-008`. A parameter whose stored type does not resolve follows the page and table requirements of such a type (ADR-0195) (R12, R38, R39).

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

### Requirement: Faults of the value column
A value that breaks only a newer rule SHALL get a field line. An S7 address that breaks only a newer rule SHALL get a field line under the address. Another fault SHALL give `500 E-DB-008`. A row whose stored type does not resolve SHALL skip the read check and show no value text, except in an approved version (ADR-0195). A read check of the table that passes `timeout.validation` SHALL give `503 E-WEB-009` (R08, R12, R30, R39, A43).

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

