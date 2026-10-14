# Spec Delta: plant-model

## ADDED Requirements

### Requirement: Stable identifiers of the seed types
Each primitive row of the matrix in `data-model.md` section 7 SHALL have one identifier that never changes. A second run of the migrator SHALL NOT add, change, or delete a seed row (R11, R12, A3).

#### Scenario: Seed rows after the first migration
- **GIVEN** the database `recipe` after the first run of `recipe-server migrate`
- **WHEN** a user reads the global primitive rows
- **THEN** the system has 31 global primitive rows: one row for each other type of the matrix, and two rows each for `STRING` and `WSTRING`
- **AND** each row has the identifier of the seed file

#### Scenario: Second run of the migrator
- **GIVEN** the database `recipe` with the seed rows
- **WHEN** the operator runs `recipe-server migrate` again
- **THEN** the seed rows keep their identifiers
- **AND** the number of global primitive rows stays the same
