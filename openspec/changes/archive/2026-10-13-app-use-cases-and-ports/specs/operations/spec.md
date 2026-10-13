# Spec Delta: operations

## ADDED Requirements

### Requirement: Storage error of a known constraint
The database can reject a duplicate key or a missing reference. Then the system SHALL return the domain error that the constraint name gives for the command. `reliability.md` section 1.1 lists these constraint names (R35, R36).

#### Scenario: Duplicate reference number found by the database
- **GIVEN** two requests that run `CreateRecipe` with the reference number `R-100` at the same time
- **WHEN** the database rejects the second insert with the constraint `recipe_reference_unique`
- **THEN** the system rejects the second command with `E-RCP-001`

#### Scenario: Missing parent unit found by the database
- **GIVEN** a request that runs `AddUnit` on `M12` with the parent path `U1`
- **WHEN** the database rejects the insert with the constraint `unit_parent_exists`
- **THEN** the system rejects the command with `E-PLT-003`

### Requirement: Storage error of an unknown constraint
When the database rejects a duplicate key of a constraint that the command does not know, the system SHALL return `E-DB-006`. When it rejects a missing reference of a constraint that the command does not know, the system SHALL return `E-DB-007` (R35, R36).

#### Scenario: Duplicate key of an unknown constraint
- **GIVEN** a command whose insert breaks a unique constraint that the command does not map
- **WHEN** the database rejects the insert
- **THEN** the system rejects the command with `E-DB-006`
- **AND** the transaction of the command writes nothing

#### Scenario: Missing reference of an unknown constraint
- **GIVEN** a command whose insert breaks a foreign key that the command does not map
- **WHEN** the database rejects the insert
- **THEN** the system rejects the command with `E-DB-007`
- **AND** the transaction of the command writes nothing
