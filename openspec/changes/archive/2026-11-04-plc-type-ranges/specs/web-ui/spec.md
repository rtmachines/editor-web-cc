## MODIFIED Requirements

### Requirement: Value column of the parameter table
The parameter table of the version page SHALL have the column Value after the column Type. An elementary parameter SHALL show its value text as on the parameter page. The cell SHALL show at most `table.value_chars` characters and end a longer text with `…`. An empty text SHALL show `parameters-value-empty`. An array or a struct SHALL show `parameters-value-composite`. A value that breaks only a newer rule SHALL get a field line. An S7 address that breaks only a newer rule SHALL get a field line under the address. Another fault SHALL give `500 E-DB-008`. A read check of the table that passes `timeout.validation` SHALL give `503 E-WEB-009` (R08, R12, R30, R39, A43).

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

## ADDED Requirements

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
The version page of an Approved version SHALL check every value, limit, and S7 address with the current rules. When a part breaks a rule, the export area SHALL show a notice with the code of each broken rule. The export link SHALL name the notice in `aria-describedby` (R12, R28, R39).

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
