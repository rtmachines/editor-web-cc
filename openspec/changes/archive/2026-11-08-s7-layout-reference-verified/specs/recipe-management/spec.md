## MODIFIED Requirements

### Requirement: S7 overlap and alignment
The system SHALL reject a parameter whose S7 address overlaps another parameter of the same version. The system SHALL reject an address of an element wider than one byte that does not start on an even byte. A command that writes parameters SHALL report only an overlap that holds a parameter that it writes. `ApproveVersion` SHALL report each overlap of the draft (R14, A31, I20, ADR-0149).

#### Scenario: Overlapping address
- **GIVEN** the parameter `SPEED` at `DB1.DBW4` in the draft version 3 of `R-100`
- **WHEN** a ProcessEngineer runs `AddParameter` on version 3 with the tag `TEMP` and the address `DB1.DBW4`
- **THEN** the system rejects the command with `E-RCP-022`

#### Scenario: Odd start byte
- **GIVEN** the machine `M3` with the platform `S7_300`
- **WHEN** a ProcessEngineer runs `AddParameter` on a draft of `M3` with the tag `SPEED`, the type `INT`, and the address `DB1.DBW3`
- **THEN** the system rejects the command with `E-RCP-023`

#### Scenario: Edit of a parameter outside a stored overlap
- **GIVEN** the draft version 3 of `R-100` on `S7_1500` with the parameters `BYTES` of type `ST_Bytes@1` at `DB1.DBB0` and `MODE` of type `BYTE` at `DB1.DBB4`, stored before ADR-0147
- **AND** the parameter `SPEED` of type `INT` at `DB1.DBW8`
- **WHEN** a ProcessEngineer runs `SetParameterValue` on `SPEED` with the value 100
- **THEN** the system stores the value 100

#### Scenario: Edit of a parameter of a stored overlap
- **GIVEN** the draft version 3 of `R-100` on `S7_1500` with the parameters `BYTES` of type `ST_Bytes@1` at `DB1.DBB0` and `MODE` of type `BYTE` at `DB1.DBB4`, stored before ADR-0147
- **WHEN** a ProcessEngineer runs `SetParameterValue` on `MODE` with the value 3
- **THEN** the system rejects the command with `E-RCP-022`

#### Scenario: Move of one parameter of a stored overlap
- **GIVEN** the draft version 3 of `R-100` on `S7_1500` with the parameters `BYTES` of type `ST_Bytes@1` at `DB1.DBB0` and `MODE` of type `BYTE` at `DB1.DBB4`, stored before ADR-0147
- **AND** the parameters `GRID` of type `ARRAY[1..3,1..3] OF BOOL` at `DB2.DBB0` and `X` of type `BYTE` at `DB2.DBB2`, stored before ADR-0147
- **WHEN** a ProcessEngineer runs `UpdateParameterDefinition` on `MODE` with the address `DB1.DBB6`
- **THEN** the system stores the address `DB1.DBB6` of `MODE`

#### Scenario: Approval of a draft with a stored overlap
- **GIVEN** the draft version 3 of `R-100` on `S7_1500` with the parameters `BYTES` of type `ST_Bytes@1` at `DB1.DBB0` and `MODE` of type `BYTE` at `DB1.DBB4`, stored before ADR-0147
- **WHEN** an Administrator runs `ApproveVersion` on version 3
- **THEN** the system rejects the command with `E-RCP-022`

### Requirement: S7 layout size of a struct and an array
The system SHALL compute the S7 layout size of a struct or an array with the `S7Layout` rules of `domain-model.md` section 2.1. Up to 8 consecutive `BOOL` members SHALL share one byte. A member wider than 1 byte SHALL start on an even byte. A `SINT` or `USINT` member of a struct SHALL start on an even byte. In a `BOOL` array with two dimensions, each line SHALL start at the next byte. A struct and an array SHALL end on an even byte (R14, A31, I20).

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

#### Scenario: SINT and USINT members start on an even byte
- **GIVEN** the machine `M5` with the platform `S7_1500` and the struct `ST_Bytes` of `M5`
- **AND** `ST_Bytes` has the members `a` of type `BYTE`, `b` of type `CHAR`, `c` of type `SINT`, and `d` of type `USINT`
- **AND** the parameter `BYTES` of type `ST_Bytes@1` at the address `DB1.DBB0` in the draft version 3 of `R-100`
- **WHEN** a ProcessEngineer runs `AddParameter` on version 3 with the tag `MODE`, the type `BYTE`, and the address `DB1.DBB4`
- **THEN** the system rejects the command with `E-RCP-022`

#### Scenario: Parameter after a struct with SINT and USINT members
- **GIVEN** the parameter `BYTES` of type `ST_Bytes@1` at the address `DB1.DBB0` in the draft version 3 of `R-100` on `S7_1500`
- **WHEN** a ProcessEngineer runs `AddParameter` on version 3 with the tag `TEMP`, the type `INT`, and the address `DB1.DBW6`
- **THEN** the system adds the parameter `TEMP`

#### Scenario: Lines of a BOOL array with two dimensions
- **GIVEN** the parameter `GRID` of type `ARRAY[1..3,1..3] OF BOOL` at the address `DB1.DBB0` in the draft version 3 of `R-100` on `S7_300`
- **WHEN** a ProcessEngineer runs `AddParameter` on version 3 with the tag `MODE`, the type `BYTE`, and the address `DB1.DBB2`
- **THEN** the system rejects the command with `E-RCP-022`

#### Scenario: Parameter after a BOOL array with two dimensions
- **GIVEN** the parameter `GRID` of type `ARRAY[1..3,1..3] OF BOOL` at the address `DB1.DBB0` in the draft version 3 of `R-100` on `S7_300`
- **WHEN** a ProcessEngineer runs `AddParameter` on version 3 with the tag `MODE`, the type `BYTE`, and the address `DB1.DBB4`
- **THEN** the system adds the parameter `MODE`
