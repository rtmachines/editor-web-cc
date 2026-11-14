## ADDED Requirements

### Requirement: A new version row is a draft
The database SHALL accept a new row of `recipe_version` only with the status `draft`, the `row_version` 1, and no deletion time. It SHALL reject any other new row with `E-RCP-037`. The rule SHALL hold for every role (R17, R29, I4).

#### Scenario: Insert of an approved version
- **GIVEN** the recipe `R-100` and a session of the role `recipe_app`
- **WHEN** an `INSERT` statement adds a version of `R-100` with the status `approved`, an approval time, an approver, and a hash
- **THEN** the database rejects the statement with `E-RCP-037`
- **AND** `R-100` has no new version

#### Scenario: Insert of a new draft
- **GIVEN** the recipe `R-100` with no live draft and a session of the role `recipe_app`
- **WHEN** a ProcessEngineer runs `CreateVersion` on `R-100`
- **THEN** the database accepts the new version with the status `draft` and the `row_version` 1
