# Spec Delta: plant-model

## ADDED Requirements

### Requirement: Order of the derived type locks
A command that can insert a type row SHALL take the advisory locks of derived types in a fixed order. The order is the struct name, `STRING`, `WSTRING`, and `ARRAY`. The command SHALL take every lock that its type expressions need before its first insert of a type row. `EnsureDerivedType` SHALL take no lock of its own (R29, R31).

#### Scenario: Array of sized strings in a new parameter
- **GIVEN** the machine `M12` with the platform `TwinCat3` and the draft version 3 of `R-100`
- **WHEN** a ProcessEngineer runs `AddParameter` on version 3 with the type `ARRAY[1..4] OF STRING(16)`
- **THEN** the command takes the lock `STRING` of `M12` and then the lock `ARRAY` of `M12`
- **AND** the command takes both locks before it inserts the type `STRING(16)`

#### Scenario: Struct with an array member
- **GIVEN** the machine `M12` with the platform `TwinCat3`
- **WHEN** a ProcessEngineer runs `DefineUdt` for `ST_Zone` with the member `temps` of type `ARRAY[1..2] OF REAL`
- **THEN** the command takes the lock `ST_Zone` of `M12` and then the lock `ARRAY` of `M12`
- **AND** the command takes both locks before it inserts a type row

#### Scenario: Seed string without a length
- **GIVEN** the machine `M12` with the platform `TwinCat3` and the draft version 3 of `R-100`
- **WHEN** a ProcessEngineer runs `AddParameter` on version 3 with the type `STRING`
- **THEN** the command uses the seed row `STRING(80)`
- **AND** the command takes no advisory lock of derived types

### Requirement: Archive of an archived unit
The system SHALL reject `ArchiveUnit` with `E-PLT-009` when the unit is already archived. The command SHALL then change no unit and write no audit entry (R06, R29).

#### Scenario: Unit archived before
- **GIVEN** the machine `M12` with the archived unit `U1.2`
- **WHEN** a ProcessEngineer runs `ArchiveUnit` on `U1.2`
- **THEN** the system rejects the command with `E-PLT-009`
- **AND** the archive time of `U1.2` does not change
