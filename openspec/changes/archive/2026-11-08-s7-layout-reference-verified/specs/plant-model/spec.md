## ADDED Requirements

### Requirement: No array of an array on S7-1200 and S7-1500
On a machine with `S7_1200` or `S7_1500`, the system SHALL reject a written type text that holds an array as the element of an array. The type of `AddParameter`, a changed type of `UpdateParameterDefinition`, and a member type of `DefineUdt` are such texts. An array with two or three dimensions in one prefix SHALL stay valid (R12, R13, R38, A38, ADR-0148).

#### Scenario: Parameter of an array of an array
- **GIVEN** the machine `M5` with the platform `S7_1500` and the draft version 3 of `R-100`
- **WHEN** a ProcessEngineer runs `AddParameter` on version 3 with the type `ARRAY[1..2] OF ARRAY[1..3] OF BYTE`
- **THEN** the system rejects the command with `E-PLT-013` and the reason "the type does not exist on the platform of the machine"
- **AND** the system creates no type

#### Scenario: Stored array type in a new parameter
- **GIVEN** the machine `M5` with the platform `S7_1500` and the draft version 3 of `R-100`
- **AND** the array type `ARRAY[1..2] OF ARRAY[1..3] OF BYTE` of `M5`, stored before ADR-0148
- **WHEN** a ProcessEngineer runs `AddParameter` on version 3 with the type `ARRAY[1..2] OF ARRAY[1..3] OF BYTE`
- **THEN** the system rejects the command with `E-PLT-013`
- **AND** version 3 holds no new parameter

#### Scenario: Member of an array of an array
- **GIVEN** the machine `M6` with the platform `S7_1200`
- **WHEN** a ProcessEngineer runs `DefineUdt` for `ST_Grid` with the member `cells` of type `ARRAY[1..3] OF ARRAY[1..3] OF BOOL`
- **THEN** the system rejects the command with `E-PLT-006` for the member `cells`
- **AND** the system creates no type

#### Scenario: New revision of a stored struct with an array of an array
- **GIVEN** the machine `M5` with the platform `S7_1500` and the struct `ST_Grid@1` with the member `cells` of type `ARRAY[1..2] OF ARRAY[1..3] OF BYTE`, stored before ADR-0148
- **WHEN** a ProcessEngineer runs `DefineUdt` for `ST_Grid` with the members of `ST_Grid@1` and the new member `mode` of type `BYTE`
- **THEN** the system rejects the command with `E-PLT-006` for the member `cells`
- **AND** the system creates no revision `ST_Grid@2`

#### Scenario: Changed type to an array of an array
- **GIVEN** the draft version 3 of `R-100` on `M5` with the parameter `SPEED` of type `INT`
- **WHEN** a ProcessEngineer runs `UpdateParameterDefinition` on `SPEED` with the type `ARRAY[1..2] OF ARRAY[1..2] OF INT`
- **THEN** the system rejects the command with `E-PLT-013`
- **AND** `SPEED` keeps the type `INT`

#### Scenario: Array with two dimensions
- **GIVEN** the machine `M5` with the platform `S7_1500` and the draft version 3 of `R-100`
- **WHEN** a ProcessEngineer runs `AddParameter` on version 3 with the type `ARRAY[1..2,1..3] OF BYTE`
- **THEN** the system adds the parameter

#### Scenario: Array of a struct with an array member
- **GIVEN** the machine `M5` with the struct `ST_Zone@1` and its member `ramps` of type `ARRAY[1..3] OF INT`
- **WHEN** a ProcessEngineer runs `AddParameter` on a draft of `M5` with the type `ARRAY[1..2] OF ST_Zone@1`
- **THEN** the system adds the parameter

#### Scenario: Array of an array on TwinCAT 3
- **GIVEN** the machine `M12` with the platform `TwinCat3`
- **WHEN** a ProcessEngineer runs `AddParameter` on a draft of `M12` with the type `ARRAY[1..2] OF ARRAY[1..3] OF INT`
- **THEN** the system adds the parameter

### Requirement: Stored array of an array on S7-1200 and S7-1500
The system SHALL read a stored array type on `S7_1200` or `S7_1500` whose element is an array. A stored parameter type SHALL keep such a type while a command does not change the type. `RebindTypeRevision` SHALL keep the array levels of such a parameter type (R19, R31, ADR-0120, ADR-0148).

#### Scenario: Type page of a stored array of an array
- **GIVEN** the machine `M5` with the platform `S7_1500` and the struct `ST_Grid@1` with the member `cells` of type `ARRAY[1..2] OF ARRAY[1..3] OF BYTE`, stored before ADR-0148
- **WHEN** a ProcessEngineer opens the page of the type `ST_Grid`
- **THEN** the system shows `ST_Grid@1` and its member `cells`

#### Scenario: Parameter keeps a stored array of an array
- **GIVEN** the draft version 3 of `R-100` on `M5` with the parameter `CELLS` of type `ARRAY[1..2] OF ARRAY[1..3] OF BYTE`, stored before ADR-0148
- **WHEN** a ProcessEngineer runs `UpdateParameterDefinition` on `CELLS` with the same type and a new name
- **THEN** the system stores the new name
- **AND** `CELLS` keeps its type

#### Scenario: Rebind of a stored array of an array
- **GIVEN** the draft version 3 of `R-100` on `M5` with the parameter `ZONES` of type `ARRAY[1..2] OF ARRAY[1..2] OF ST_Zone@1`, stored before ADR-0148
- **AND** the struct `ST_Zone@2` with the members of `ST_Zone@1`
- **WHEN** a ProcessEngineer runs `RebindTypeRevision` on version 3 from `ST_Zone@1` to `ST_Zone@2`
- **THEN** `ZONES` gets the type `ARRAY[1..2] OF ARRAY[1..2] OF ST_Zone@2`

## MODIFIED Requirements

### Requirement: Type on the platform of the machine
Every type that a parameter or a struct member uses SHALL exist on the PLC platform of the machine. A type text that a command writes SHALL also follow the rule of ADR-0148. On `S7_1200` and `S7_1500`, the element of an array is not an array. A stored type SHALL keep the earlier rule set of ADR-0120, so a stored array of an array stays valid (R11, R12, I6, ADR-0148).

#### Scenario: Parameter type not on the platform
- **GIVEN** the machine `M3` with the platform `S7_300`
- **WHEN** a ProcessEngineer runs `AddParameter` with the type `WSTRING(10)` on a draft of a recipe of `M3`
- **THEN** the system rejects the command with `E-PLT-013`

#### Scenario: Member type not on the platform
- **GIVEN** the machine `M3` with the platform `S7_300`
- **WHEN** a ProcessEngineer runs `DefineUdt` for `ST_Label` with a member of the type `WSTRING(10)`
- **THEN** the system rejects the command with `E-PLT-006`

#### Scenario: Database backstop
- **GIVEN** a draft of a recipe of the machine `M3` with the platform `S7_300`
- **WHEN** an `INSERT` statement adds a parameter with a type that does not exist on `S7_300`
- **THEN** the database rejects the statement with `E-PLT-031`

#### Scenario: Type of another machine
- **GIVEN** the machines `M3` and `M4` with the platform `S7_300`
- **AND** the type `STRING(20)` of `M4` and a draft of a recipe of `M3`
- **WHEN** an `INSERT` statement adds a parameter of the type `STRING(20)` of `M4` to the draft of `M3`
- **THEN** the database rejects the statement with `E-PLT-032`
