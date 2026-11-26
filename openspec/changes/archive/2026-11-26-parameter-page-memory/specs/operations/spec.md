## ADDED Requirements

### Requirement: Peak memory of a grid answer
A grid answer SHALL hold at most two of its parts at the same time. The parts are the row model, the HTML of the form, and the HTML of the page. A valid save SHALL build no sent grid. Route tests SHALL pin the length and the peak heap of each grid answer of the largest valid parameter of A26 (R30, R31, A26).

#### Scenario: Edit page of the largest valid parameter
- **GIVEN** the parameter with 32 type levels, 16 384 `LDT` leaves, leaf paths of 128 bytes, and a limit per element at each array level
- **WHEN** the ProcessEngineer `anowak` opens its edit page
- **THEN** the page holds 32 mode rows and 16 384 leaf rows
- **AND** the page length and its SHA-256 equal the pins of the route test
- **AND** the peak heap of the request is at most the pin of the route test

#### Scenario: Refused form of the largest valid parameter
- **GIVEN** the parameter `BIG` of the scalar type `LDT`
- **AND** its edit form with the type of the largest valid parameter and 4 600 000 characters in `description_en`
- **AND** the form goes without `HX-Request` in the language `pl`
- **WHEN** the server refuses the form
- **THEN** the server returns `422` with a line for `description_en`
- **AND** the peak heap of the request is at most the pin of the route test

#### Scenario: Save that answers 303
- **GIVEN** the edit form of the largest valid parameter with valid fields, sent to `/definition` without `HX-Request`
- **WHEN** the server saves the form
- **THEN** the server returns `303` and builds no grid

#### Scenario: HTMX success of a definition save
- **GIVEN** the edit form of the largest valid parameter with valid fields, sent to `/definition` with `HX-Request`
- **WHEN** the server saves the form
- **THEN** the server returns `200` with the clean form out of band
- **AND** the server builds no grid of the sent fields
