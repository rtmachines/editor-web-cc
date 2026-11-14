## MODIFIED Requirements

### Requirement: Every field error at once
On the recipe and plant forms, a changed body SHALL give one line `422 E-WEB-008` and no out-of-band form. A changed body has a fault of ADR-0138 (b) to (d), for example an unknown key or a `[*]` segment. For another body, the answer SHALL list every bad field and every fault of the value and the limits in the stages of ADR-0041. `#error-panel` SHALL show at most 20 lines and the count of the others. The out-of-band form SHALL show a line under each bad input (R36, R38, R39, A44).

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

#### Scenario: Member row with a bad name and no type
- **GIVEN** the ProcessEngineer `anowak` with an open session and the machine `M7`
- **WHEN** `anowak` sends an HTMX `POST /machines/M7/types` with one member row with the name `9bad` and an empty type
- **THEN** the server returns `422 E-WEB-008` with two lines
- **AND** the name input and the type input of the row each have an error line under them
- **AND** the server creates no type

#### Scenario: Unknown key in the add form
- **GIVEN** the ProcessEngineer `anowak` with an open session and the draft version 3 of `R-100`
- **WHEN** `anowak` sends an HTMX `POST /recipes/R-100/versions/3/parameters` with the tag `bad tag`, the unknown key `bogus`, and the value key `value[*]`
- **THEN** the server returns `422 E-WEB-008` with one line in `#error-panel`
- **AND** the answer holds no error line under an input and no out-of-band form
- **AND** the draft does not change

#### Scenario: Unknown key in the edit form
- **GIVEN** the ProcessEngineer `anowak` with an open session and the parameter `SPEED` of the draft version 3 of `R-100`
- **WHEN** `anowak` sends an HTMX `POST /recipes/R-100/versions/3/parameters/{key}/definition` of `SPEED` with the tag `bad tag`, the unknown key `bogus`, and the value key `value[*]`
- **THEN** the server returns `422 E-WEB-008` with one line in `#error-panel`
- **AND** the answer holds no error line under an input and no out-of-band form
- **AND** the parameter keeps its definition, its value, and its limits

#### Scenario: Unknown key in the UDT form
- **GIVEN** the ProcessEngineer `anowak` with an open session and the machine `M7`
- **WHEN** `anowak` sends an HTMX `POST /machines/M7/types` with the bad member names `9a` and `9b` and the unknown key `bogus`
- **THEN** the server returns `422 E-WEB-008` with one line
- **AND** no member name input has an error line under it

#### Scenario: Unknown key in the value form
- **GIVEN** the ProcessEngineer `anowak` with an open session and the parameter `SPEED` of the draft version 3 of `R-100`
- **WHEN** `anowak` sends an HTMX `POST /recipes/R-100/versions/3/parameters/{key}/value` of `SPEED` with the value `abc` and the unknown key `bogus`
- **THEN** the server returns `422 E-WEB-008` with one line in `#error-panel`
- **AND** the answer holds no error line under an input
- **AND** the parameter keeps its value

#### Scenario: Unknown key in the machine form
- **GIVEN** the ProcessEngineer `anowak` with an open session
- **WHEN** `anowak` sends an HTMX `POST /machines` with an empty machine code and the unknown key `bogus`
- **THEN** the server returns `422 E-WEB-008` with one line in `#error-panel`
- **AND** the answer holds no error line under the input `code` and no out-of-band form
- **AND** the server creates no machine
