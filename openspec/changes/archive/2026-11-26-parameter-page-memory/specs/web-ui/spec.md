## ADDED Requirements

### Requirement: One mode row for each type level
The leaf grid SHALL show one mode row for each type level. A type level is the path of an array level with `[*]` for each index, for example `[*][*]`. The button of the edit form SHALL switch the mode of each array level of the type level. A switch back SHALL join the limits of each array level alone. A `limit_mode` with an index segment SHALL give one `E-WEB-008` line (R09, R39, A26).

#### Scenario: Nested array with a limit per element
- **GIVEN** the parameter `NEST` of type `ARRAY[1..2] OF ARRAY[1..3] OF INT` with a limit on each of its six leaves
- **WHEN** the ProcessEngineer `anowak` opens the page of `NEST`
- **THEN** the grid shows two mode rows, one for the type level `[*]` and one for the type level `[*][*]`
- **AND** the grid shows the six leaf rows `[1][1]` to `[2][3]`

#### Scenario: Switch of an inner type level
- **GIVEN** the edit form of `NEST` with a limit per element at the type level `[*][*]`
- **WHEN** `anowak` sends the form with `limit_mode=[*][*]`
- **THEN** the server returns `200` with one row `[*]` for each of the array levels `[1][*]` and `[2][*]`
- **AND** the draft keeps its `row_version`

#### Scenario: Switch back with unequal limits in one array level
- **GIVEN** the edit form of `NEST` with the limit 0 to 10 on `[1][1]` to `[1][3]`
- **AND** the limits 0 to 5, 0 to 6, and 0 to 7 on `[2][1]` to `[2][3]`
- **WHEN** `anowak` sends the form with `limit_mode=[*][*]`
- **THEN** the row `[1][*]` holds the limit 0 to 10
- **AND** the row `[2][*]` is empty
- **AND** one notice names the type level `[*][*]` and the count 1 of the array levels with unequal limits

#### Scenario: A switch of one array level
- **GIVEN** the edit form of `NEST`
- **WHEN** `anowak` sends the form with `limit_mode=[1][*]`
- **THEN** the server answers with one `E-WEB-008` line
- **AND** the server writes nothing

### Requirement: Mixed modes in one type level
In the edit form, a type level SHALL show a limit per element when it mixes a `[*]` limit and an index limit. It SHALL copy each `[*]` limit to its element rows. One notice SHALL name the type level. A save above `budget.limit_rows_per_version` SHALL give `E-RCP-021`. The read-only grid SHALL show the mode texts and the copied limits, with no button and no notice (R09, R39, A26).

#### Scenario: Mixed modes in one type level
- **GIVEN** the parameter `NEST` with the limit 0 to 10 on `[1][*]` and a limit on each of `[2][1]`, `[2][2]`, and `[2][3]`
- **WHEN** `anowak` opens the page of `NEST`
- **THEN** the type level `[*][*]` shows a limit per element
- **AND** the rows `[1][1]`, `[1][2]`, and `[1][3]` hold the limit 0 to 10
- **AND** a notice names the type level `[*][*]`

#### Scenario: Read-only grid of a mixed type level
- **GIVEN** the parameter `NEST` with the limit 0 to 10 on `[1][*]` and a limit on each of `[2][1]`, `[2][2]`, and `[2][3]`
- **WHEN** the Operator `mhoffman` opens the page of `NEST`
- **THEN** the grid shows the mode texts of the two type levels and no `limit_mode` button
- **AND** the rows `[1][1]`, `[1][2]`, and `[1][3]` show the limit 0 to 10 as text
- **AND** the grid shows no notice

#### Scenario: Copied limits above the limit row budget
- **GIVEN** a draft with 16 385 limit rows outside the parameter `WIDE` of type `ARRAY[1..2] OF ARRAY[1..8192] OF INT`
- **AND** `WIDE` has a limit on `[1][*]` and on each of `[2][1]` to `[2][8192]`
- **WHEN** `anowak` saves the edit form of `WIDE` without a change
- **THEN** the server returns `422` with `E-RCP-021` and its summary line
- **AND** the draft keeps its limits and its `row_version`

## MODIFIED Requirements

### Requirement: Limit modes of an array level
Each array level of the grid SHALL have one limit for all elements, in a row `[*]`, or a limit per element. The page SHALL show a limit per element when the limits hold an index at that level. The button of its type level SHALL switch the mode and SHALL write nothing (R09, A36).

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
