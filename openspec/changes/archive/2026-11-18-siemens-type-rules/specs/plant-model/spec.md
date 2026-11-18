## ADDED Requirements

### Requirement: No array of an array on S7-300
On a machine with `S7_300`, the system SHALL reject a written type text that holds an array as the element of an array. It SHALL use the codes of the rule on `S7_1200` and `S7_1500`. A stored type of this kind SHALL stay readable, and approval and export SHALL not check it (R12, R13, R19, R38, A38, ADR-0177).

#### Scenario: Parameter of an array of an array on S7-300
- **GIVEN** the machine `M3` with the platform `S7_300` and the draft version 3 of `R-100`
- **WHEN** a ProcessEngineer runs `AddParameter` on version 3 with the type `ARRAY[1..2] OF ARRAY[1..3] OF BYTE` and the address `DB1.DBB0`
- **THEN** the system rejects the command with `E-PLT-013` and the reason "the type does not exist on the platform of the machine"
- **AND** the system creates no type

#### Scenario: Member of an array of an array on S7-300
- **GIVEN** the machine `M3` with the platform `S7_300`
- **WHEN** a ProcessEngineer runs `DefineUdt` for `ST_Grid` with the member `cells` of type `ARRAY[1..3] OF ARRAY[1..3] OF BOOL`
- **THEN** the system rejects the command with `E-PLT-006` for the member `cells`
- **AND** the system creates no type

#### Scenario: Stored array of an array on S7-300 stays approvable
- **GIVEN** the draft version 3 of `R-100` on `M3` with the platform `S7_300`
- **AND** the parameter `GRID` of type `ARRAY[1..2] OF ARRAY[1..3] OF BYTE` at `DB1.DBB0`, stored before ADR-0177
- **WHEN** a ProcessEngineer runs `ApproveVersion` on version 3
- **THEN** the system approves version 3

#### Scenario: Array of an array on CODESYS
- **GIVEN** the machine `M9` with the platform `CodesysIec` and the draft version 3 of `R-100`
- **WHEN** a ProcessEngineer runs `AddParameter` on version 3 with the type `ARRAY[1..2] OF ARRAY[1..3] OF BYTE`
- **THEN** the system adds the parameter

### Requirement: Nesting limit of a type on S7
On a machine with `S7_300`, `S7_1200`, or `S7_1500`, the system SHALL reject a written type with more than `type.s7_max_nesting` levels with `E-PLT-018`. A struct SHALL count one level, also at the top. An array of a struct SHALL count three levels in place of one. A primitive, a string, and an array of them SHALL count no level. A stored type over the limit SHALL stay readable and approvable (R12, R13, R19, R38, A38, ADR-0179).

#### Scenario: New struct with nine levels
- **GIVEN** the machine `M5` with the platform `S7_1500` and the struct `ST_Deep@1` with eight levels in the S7 count
- **WHEN** a ProcessEngineer runs `DefineUdt` for `ST_Top` with the member `inner` of type `ST_Deep@1`
- **THEN** the system rejects the command with `E-PLT-018` with the name `ST_Top` and the limit 8
- **AND** the system creates no type

#### Scenario: Parameter of an array of a deep struct
- **GIVEN** the machine `M5` with the platform `S7_1500`, the draft version 3 of `R-100`, and the struct `ST_Mid@1` with seven levels in the S7 count
- **WHEN** a ProcessEngineer runs `AddParameter` on version 3 with the type `ARRAY[1..2] OF ST_Mid@1`
- **THEN** the system rejects the command with `E-PLT-018` with the type text and the limit 8
- **AND** version 3 holds no new parameter

#### Scenario: Type with eight levels
- **GIVEN** the machine `M5` with the platform `S7_1500`, the draft version 3 of `R-100`, and the struct `ST_Mid@1` with six levels in the S7 count
- **WHEN** a ProcessEngineer runs `AddParameter` on version 3 with the type `ARRAY[1..2] OF ST_Mid@1`
- **THEN** the system adds the parameter

#### Scenario: No nesting limit on TwinCAT
- **GIVEN** the machine `M8` with the platform `TwinCat3` and the struct `ST_Deep@1` with eight levels in the S7 count
- **WHEN** a ProcessEngineer runs `DefineUdt` for `ST_Top` with the member `inner` of type `ST_Deep@1`
- **THEN** the system creates `ST_Top@1`

#### Scenario: Stored struct over the limit stays readable
- **GIVEN** the machine `M5` with the platform `S7_1500` and the struct `ST_Top@1` with nine levels in the S7 count, stored before ADR-0179
- **WHEN** a ProcessEngineer opens the page of the type `ST_Top`
- **THEN** the system shows `ST_Top@1` and its members

#### Scenario: Stored parameter over the limit stays approvable
- **GIVEN** the draft version 3 of `R-100` on `M5` with the platform `S7_1500`
- **AND** the parameter `TOP` of type `ST_Top@1` with nine levels in the S7 count, stored before ADR-0179
- **WHEN** a ProcessEngineer runs `ApproveVersion` on version 3
- **THEN** the system approves version 3

### Requirement: Nesting limit of a moved type on S7
On `S7_300`, `S7_1200`, and `S7_1500`, `RebindTypeRevision` SHALL count the resulting type of each parameter that it moves. A resulting type over `type.s7_max_nesting` SHALL give `E-PLT-018`, and the command SHALL change nothing. A parameter that the command does not move SHALL keep its stored type (R12, R13, R19, R38, A38, ADR-0179).

#### Scenario: Rebind to a deeper revision on S7
- **GIVEN** the machine `M5` with the platform `S7_1500` and the draft version 3 of `R-100`
- **AND** the struct `ST_Mid@1` with six levels and its revision `ST_Mid@2` with eight levels in the S7 count
- **AND** the parameter `BUF` of type `ARRAY[1..2] OF ST_Mid@1`
- **WHEN** a ProcessEngineer runs `RebindTypeRevision` on version 3 from `ST_Mid@1` to `ST_Mid@2`
- **THEN** the system rejects the command with `E-PLT-018` with the type text `ARRAY[1..2] OF ST_Mid@2` and the limit 8
- **AND** `BUF` keeps the type `ARRAY[1..2] OF ST_Mid@1`

### Requirement: Siemens reserved struct names on S7
On `S7_300`, `S7_1200`, and `S7_1500`, the system SHALL reject a new struct name that equals a Siemens reserved name of ADR-0186 in any case. It SHALL also reject a written struct reference with such a name. Both SHALL give `E-PLT-013` with the reason "reserved name". A member name SHALL stay free of this rule. A stored struct with such a name SHALL stay readable (R11, R12, R13, R38, A50, ADR-0186).

#### Scenario: Siemens timer type as a struct name
- **GIVEN** the machine `M5` with the platform `S7_1500`
- **WHEN** a ProcessEngineer runs `DefineUdt` for `TON_TIME` or `pid_compact`
- **THEN** the system rejects the command with `E-PLT-013` and the reason "reserved name"
- **AND** the system creates no type

#### Scenario: Siemens telegram name as a struct name
- **GIVEN** the machine `M5` with the platform `S7_1500`
- **WHEN** a ProcessEngineer runs `DefineUdt` for `PD_TEL3`
- **THEN** the system rejects the command with `E-PLT-013` and the reason "reserved name"
- **AND** the system creates no type

#### Scenario: Siemens name on CODESYS
- **GIVEN** the machine `M9` with the platform `CodesysIec`
- **WHEN** a ProcessEngineer runs `DefineUdt` for `TON_TIME`
- **THEN** the system creates `TON_TIME@1`

#### Scenario: Siemens name as a member name
- **GIVEN** the machine `M5` with the platform `S7_1500`
- **WHEN** a ProcessEngineer runs `DefineUdt` for `ST_Clock` with the member `TIMER` of type `INT`
- **THEN** the system creates `ST_Clock@1`

#### Scenario: Parameter of a stored struct with a Siemens name
- **GIVEN** the machine `M5` with the platform `S7_1500`, the struct `PID_Compact@1`, stored before ADR-0186, and the draft version 3 of `R-100`
- **WHEN** a ProcessEngineer runs `AddParameter` on version 3 with the type `PID_Compact@1`
- **THEN** the system rejects the command with `E-PLT-013` and the reason "reserved name"
- **AND** the type proposals of the parameter form do not hold `PID_Compact@1`

#### Scenario: Rebind of a stored struct with a Siemens name
- **GIVEN** the machine `M5` with the platform `S7_1500` and the draft version 3 of `R-100`
- **AND** the struct `PID_Compact@1` and its revision `PID_Compact@2`, both stored before ADR-0186
- **AND** the parameter `PID` of type `PID_Compact@1`
- **WHEN** a ProcessEngineer runs `RebindTypeRevision` on version 3 from `PID_Compact@1` to `PID_Compact@2`
- **THEN** the system moves `PID` to `PID_Compact@2`

#### Scenario: Stored struct with a Siemens name stays readable
- **GIVEN** the machine `M5` with the platform `S7_1500` and the struct `PID_Compact@1`, stored before ADR-0186
- **WHEN** a ProcessEngineer opens the page of the type `PID_Compact`
- **THEN** the system shows `PID_Compact@1` and its members

### Requirement: STRING(n) notation of a type text
A type text SHALL give the length of a `STRING` or a `WSTRING` only in round brackets. The system SHALL reject the notation `STRING[n]` and `WSTRING[n]` with `E-PLT-013` (R11, R13, R38, ADR-0183).

#### Scenario: STRING with square brackets
- **GIVEN** the machine `M5` with the platform `S7_1500` and the draft version 3 of `R-100`
- **WHEN** a ProcessEngineer runs `AddParameter` on version 3 with the type `STRING[10]`
- **THEN** the system rejects the command with `E-PLT-013`
- **AND** version 3 holds no new parameter

#### Scenario: STRING with round brackets
- **GIVEN** the machine `M5` with the platform `S7_1500` and the draft version 3 of `R-100`
- **WHEN** a ProcessEngineer runs `AddParameter` on version 3 with the type `STRING(10)`
- **THEN** the system adds the parameter
