# Spec Delta: recipe-management

## ADDED Requirements

### Requirement: IEC literal of a time value
The system SHALL accept a `Duration` or `Instant` value only as an IEC literal of `domain-model.md` section 2.1. The prefix SHALL name the type of the leaf. The count SHALL be inside the range of the type on the platform of the machine. The system SHALL show each value as its canonical literal (R11, R12, R38, A43).

#### Scenario: Parts in the wrong order
- **GIVEN** the parameter `CYCLE` of type `TIME` in the draft version 3 of `R-100` on the platform `TwinCat3`
- **WHEN** a ProcessEngineer runs `SetParameterValue` on `CYCLE` with the value `T#2h1d`
- **THEN** the system rejects the command with `E-RCP-005`

#### Scenario: Prefix of another type
- **GIVEN** the parameter `CYCLE` of type `TIME` in the draft version 3 of `R-100`
- **WHEN** a ProcessEngineer runs `SetParameterValue` on `CYCLE` with the value `LT#5s`
- **THEN** the system rejects the command with `E-RCP-005`

#### Scenario: Negative TIME on a platform with an unsigned range
- **GIVEN** the parameter `CYCLE` of type `TIME` in the draft version 3 of `R-100` on the platform `TwinCat3`
- **WHEN** a ProcessEngineer runs `SetParameterValue` on `CYCLE` with the value `T#-1s`
- **THEN** the system rejects the command with `E-RCP-005`

#### Scenario: Canonical literal of a stored value
- **GIVEN** the parameter `CYCLE` of type `TIME` in the draft version 3 of `R-100` on the platform `TwinCat3`
- **WHEN** a ProcessEngineer runs `SetParameterValue` on `CYCLE` with the value `T#90m`
- **THEN** the system stores the value `5400000`
- **AND** the version page shows the value `T#1h30m`

### Requirement: Leaf path of a parameter type
A leaf path SHALL name one scalar leaf of the type tree of a parameter, with the grammar of `domain-model.md` section 2.1. A leaf path with `[*]` SHALL name every element of one array level. An index segment SHALL hold one index for each dimension of its array (R09, A38).

#### Scenario: Limit on every element of an array
- **GIVEN** the parameter `ZONE_1` of the struct `ST_Zone` with the member `ramps` of type `ARRAY[1..3] OF INT`
- **AND** the process limit `min` 0 and `max` 100 on the leaf path `.ramps[*]`
- **WHEN** a ProcessEngineer runs `SetParameterValue` on `ZONE_1` with the value `[10, 150, 30]` for `ramps`
- **THEN** the system rejects the command with `E-RCP-004`

#### Scenario: Index outside the array bounds
- **GIVEN** the parameter `ZONE_1` of the struct `ST_Zone` with the member `ramps` of type `ARRAY[1..3] OF INT`
- **WHEN** a ProcessEngineer runs `ReplaceLimits` on `ZONE_1` with the leaf path `.ramps[4]`
- **THEN** the system rejects the command with `E-RCP-017`

#### Scenario: Index count different from the dimension count
- **GIVEN** the parameter `GRID` of type `ARRAY[0..1,1..2] OF INT`
- **WHEN** a ProcessEngineer runs `ReplaceLimits` on `GRID` with the leaf path `[1]`
- **THEN** the system rejects the command with `E-RCP-017`

### Requirement: S7 layout size of a struct and an array
The system SHALL compute the S7 layout size of a struct or an array with the `S7Layout` rules of `domain-model.md` section 2.1. Up to 8 consecutive `BOOL` members SHALL share one byte. A member wider than 1 byte SHALL start on an even byte. A struct and an array SHALL end on an even byte (R14, A31, I20).

#### Scenario: Three BOOL members occupy two bytes
- **GIVEN** the machine `M3` with the platform `S7_300` and the struct `ST_Flags` with three `BOOL` members
- **AND** the parameter `FLAGS` of type `ST_Flags@1` at the address `DB1.DBB0` in the draft version 3 of `R-100`
- **WHEN** a ProcessEngineer runs `AddParameter` on version 3 with the tag `MODE`, the type `BYTE`, and the address `DB1.DBB1`
- **THEN** the system rejects the command with `E-RCP-022`

#### Scenario: Parameter after the end of a struct
- **GIVEN** the parameter `FLAGS` of type `ST_Flags@1` at the address `DB1.DBB0` in the draft version 3 of `R-100` on `S7_300`
- **WHEN** a ProcessEngineer runs `AddParameter` on version 3 with the tag `TEMP`, the type `INT`, and the address `DB1.DBW2`
- **THEN** the system adds the parameter `TEMP`

#### Scenario: Padding before a member wider than one byte
- **GIVEN** the struct `ST_Mix` of `M3` with the members `flag` of type `BOOL`, `speed` of type `INT`, and `mode` of type `BYTE`
- **AND** the parameter `MIX` of type `ST_Mix@1` at the address `DB1.DBB0` in the draft version 3 of `R-100`
- **WHEN** a ProcessEngineer runs `AddParameter` on version 3 with the tag `EXTRA`, the type `BYTE`, and the address `DB1.DBB4`
- **THEN** the system rejects the command with `E-RCP-022`

### Requirement: S7 address grammar
The system SHALL accept an S7 address only with the grammar of `domain-model.md` section 2.1. The data block number SHALL be 1 to 65535. The offset SHALL be 0 to 65535. The bit SHALL be 0 to 7, and only the width `X` SHALL have a bit (R14, R38, A31).

#### Scenario: Data block zero
- **GIVEN** the machine `M3` with the platform `S7_300`
- **WHEN** a ProcessEngineer runs `AddParameter` on a draft of `M3` with the type `INT` and the address `DB0.DBW0`
- **THEN** the system rejects the command with `E-WEB-008`

#### Scenario: Bit outside 0 to 7
- **GIVEN** the machine `M3` with the platform `S7_300`
- **WHEN** a ProcessEngineer runs `AddParameter` on a draft of `M3` with the type `BOOL` and the address `DB1.DBX0.8`
- **THEN** the system rejects the command with `E-WEB-008`

#### Scenario: Bit with a byte width
- **GIVEN** the machine `M3` with the platform `S7_300`
- **WHEN** a ProcessEngineer runs `AddParameter` on a draft of `M3` with the type `INT` and the address `DB1.DBW0.1`
- **THEN** the system rejects the command with `E-WEB-008`

### Requirement: S7 address width matches the type
The width letter of an S7 address SHALL match the type of the parameter, with the width table of `domain-model.md` section 2.1 (R14, A31).

#### Scenario: Double word address of an INT
- **GIVEN** the machine `M3` with the platform `S7_300`
- **WHEN** a ProcessEngineer runs `AddParameter` on a draft of `M3` with the type `INT` and the address `DB1.DBD0`
- **THEN** the system rejects the command with `E-RCP-054`

#### Scenario: Byte address of a BOOL
- **GIVEN** the machine `M3` with the platform `S7_300`
- **WHEN** a ProcessEngineer runs `AddParameter` on a draft of `M3` with the type `BOOL` and the address `DB1.DBB0`
- **THEN** the system rejects the command with `E-RCP-054`

#### Scenario: Double word address of a REAL
- **GIVEN** the machine `M3` with the platform `S7_300`
- **WHEN** a ProcessEngineer runs `AddParameter` on a draft of `M3` with the tag `RATE`, the type `REAL`, and the address `DB1.DBD4`
- **THEN** the system adds the parameter `RATE`
