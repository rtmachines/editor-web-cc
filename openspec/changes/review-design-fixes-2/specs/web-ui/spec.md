## MODIFIED Requirements

### Requirement: Body limit per route
The four routes that carry a parameter value SHALL accept a body up to `http.body_limit.value` (16 MiB). This value SHALL hold the largest edit form of a value whose JSON text has at most `budget.value_bytes`, inside the budgets of A26 and `budget.leaf_path_bytes`. The route that writes a translation SHALL accept up to `http.body_limit.translation`. Every other route SHALL accept up to `http.body_limit.form`. A larger body SHALL give `413 E-WEB-003`. At most `http.value_body_permits` requests on the four value routes SHALL wait for, read, or hold a body at the same time. A request that gets no permit within `http.body_read_timeout` SHALL get `503 E-WEB-009`, and nothing SHALL change (R30, R38, A26, ADR-0211).

#### Scenario: Value of 16 384 leaves
- **GIVEN** the parameter `PROFILE` of type `ARRAY[1..16384] OF INT`
- **WHEN** the ProcessEngineer `anowak` sends the edit form with all leaves, a body of about 500 KiB
- **THEN** the server does not answer `413`

#### Scenario: Largest edit form
- **GIVEN** a parameter with 16 384 leaves, each leaf path 128 bytes long, and a value whose JSON text has `budget.value_bytes`
- **WHEN** `anowak` sends the edit form with every value field and every limit field filled to its longest text
- **THEN** the body is not larger than `http.body_limit.value`
- **AND** the system stores the new value of the parameter

#### Scenario: Large rename
- **GIVEN** the recipe `R-100`
- **WHEN** `anowak` sends `POST /recipes/R-100/rename` with a body of 300 KiB
- **THEN** the server returns `413 E-WEB-003`

#### Scenario: Value request without a free permit
- **GIVEN** the parameter `PROFILE` in the draft version 3 of `R-100`, and `http.value_body_permits` value requests that hold a permit
- **WHEN** `anowak` sends a `SetParameterValue` form on `PROFILE` and no permit becomes free within `http.body_read_timeout`
- **THEN** the server returns `503 E-WEB-009`
- **AND** the value of `PROFILE` and the row version of the draft do not change

#### Scenario: Form route without a permit
- **GIVEN** `http.value_body_permits` value requests that hold a permit
- **WHEN** `anowak` sends `POST /recipes/R-100/rename` with a new name
- **THEN** the system renames `R-100` without a wait for a permit
