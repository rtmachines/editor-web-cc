## ADDED Requirements

### Requirement: Cap of a limit bound text
The body limit of each route is in `api.md` §7.6. For a form body within that limit, the server SHALL refuse a `min`, `max`, or `max_length` text above `form.limit_text_max` bytes as a changed body. The count SHALL use the decoded UTF-8 bytes before the trim. The answer SHALL be one line `422 E-WEB-008` with the reason text of the cap, no out-of-band form, and no grid. The command SHALL write nothing. A text at the cap SHALL pass this check (R38, A26).

#### Scenario: Long minimum in the edit form
- **GIVEN** the ProcessEngineer `anowak` with an open session and the parameter `SPEED_TABLE` of type `ARRAY[1..8] OF INT` in the draft version 3 of `R-100`
- **WHEN** `anowak` sends an HTMX `POST /recipes/R-100/versions/3/parameters/{key}/definition` with the limit row `[*]` and a `min` of `form.limit_text_max` + 1 bytes
- **THEN** the server returns `422 E-WEB-008` with one line in `#error-panel`
- **AND** the line shows the reason text of the cap
- **AND** the answer holds no out-of-band form and no grid
- **AND** the parameter keeps its definition, its value, and its limits

#### Scenario: Long minimum without HTMX
- **GIVEN** the parameter `SPEED_TABLE` of type `ARRAY[1..8] OF INT` in the draft version 3 of `R-100`
- **WHEN** `anowak` sends `POST /recipes/R-100/versions/3/parameters/{key}/definition` without `HX-Request`, with the limit row `[*]` and a `min` of `form.limit_text_max` + 1 bytes
- **THEN** the server returns `422 E-WEB-008` with one line
- **AND** the page holds no grid
- **AND** the parameter keeps its limits

#### Scenario: Long minimum with the button of the value fields
- **GIVEN** the parameter `SPEED_TABLE` of type `ARRAY[1..8] OF INT` in the draft version 3 of `R-100`
- **WHEN** `anowak` sends the edit form with `fields=1`, the limit row `[*]`, and a `min` of `form.limit_text_max` + 1 bytes
- **THEN** the server returns `422 E-WEB-008` with one line
- **AND** the answer holds no grid

#### Scenario: Padded minimum above the cap
- **GIVEN** the parameter `SPEED_TABLE` of type `ARRAY[1..8] OF INT` in the draft version 3 of `R-100`
- **WHEN** `anowak` sends the edit form with the limit row `[*]` and a `min` of `1` and spaces, `form.limit_text_max` + 1 bytes long
- **THEN** the server returns `422 E-WEB-008` with one line
- **AND** the parameter keeps its limits

#### Scenario: Long minimum in the add form
- **GIVEN** the draft version 3 of `R-100`
- **WHEN** `anowak` sends an HTMX `POST /recipes/R-100/versions/3/parameters` for a parameter of type `ARRAY[1..8] OF INT` with the limit row `[*]` and a `min` of `form.limit_text_max` + 1 bytes
- **THEN** the server returns `422 E-WEB-008` with one line in `#error-panel`
- **AND** the answer holds no grid
- **AND** the draft gets no parameter

#### Scenario: Long maximum length in the limits form
- **GIVEN** the parameter `LABEL` of type `STRING(20)` in the draft version 3 of `R-100`
- **WHEN** the ProcessEngineer `anowak` sends an HTMX `POST /recipes/R-100/versions/3/parameters/{key}/limits` with a `max_length` of `form.limit_text_max` + 1 bytes
- **THEN** the server returns `422 E-WEB-008` with one line in `#error-panel`
- **AND** the line shows the reason text of the cap, not the line of a bad maximum length
- **AND** the parameter keeps its limits

#### Scenario: Bound text at the cap
- **GIVEN** the parameter `SPEED_TABLE` of type `ARRAY[1..8] OF INT` and a valid edit form
- **WHEN** `anowak` sends the edit form with the limit row `[*]` and a `min` of `form.limit_text_max` bytes, which is `1.` and zeros
- **THEN** the reader accepts the text
- **AND** the stored limit row `[*]` has the minimum 1
