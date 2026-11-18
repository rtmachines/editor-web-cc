## ADDED Requirements

### Requirement: Array bounds of a parameter with an S7 address
On `S7_300`, `S7_1200`, and `S7_1500`, every array bound in the type tree of a parameter with an address SHALL be in `-32768..=32767`. A break SHALL give `E-RCP-058` with the tag, the address, and the reason "an array bound is outside -32768 to 32767". The system SHALL check the parameter that a command writes or rebinds, and every parameter at `ApproveVersion`. The rule SHALL run after the end rule of the address (R03, R12, R31, R38, A38, ADR-0178).

#### Scenario: Array bound above 32767 with an address
- **GIVEN** the machine `M3` with the platform `S7_300` and the draft version 3 of `R-100`
- **WHEN** a ProcessEngineer runs `AddParameter` on version 3 with the tag `BUF`, the type `ARRAY[32760..32770] OF BYTE`, and the address `DB1.DBB0`
- **THEN** the system rejects the command with `E-RCP-058`
- **AND** the message names `BUF`, `DB1.DBB0`, and the reason "an array bound is outside -32768 to 32767"
- **AND** version 3 holds no new parameter

#### Scenario: Array bound above 32767 in a struct member
- **GIVEN** the machine `M3` with the platform `S7_300`, the draft version 3 of `R-100`, and the struct `ST_Buf@1` with the member `data` of type `ARRAY[40000..40001] OF BYTE`
- **WHEN** a ProcessEngineer runs `AddParameter` on version 3 with the type `ST_Buf@1` and the address `DB1.DBB0`
- **THEN** the system rejects the command with `E-RCP-058`

#### Scenario: Array bound above 32767 without an address
- **GIVEN** the machine `M5` with the platform `S7_1500` and the draft version 3 of `R-100`
- **WHEN** a ProcessEngineer runs `AddParameter` on version 3 with the type `ARRAY[32760..32770] OF BYTE` and no address
- **THEN** the system adds the parameter

#### Scenario: Stored array bound above 32767 at approval
- **GIVEN** the draft version 3 of `R-100` on `M3` with the platform `S7_300`
- **AND** the parameter `BUF` of type `ARRAY[32760..32770] OF BYTE` at `DB1.DBB0`, stored before ADR-0178
- **WHEN** a ProcessEngineer runs `ApproveVersion` on version 3
- **THEN** the system rejects the command with `E-RCP-058` for `BUF`
- **AND** version 3 stays a draft

#### Scenario: Stored array bound of another parameter
- **GIVEN** the draft version 3 of `R-100` on `M3` with the parameter `BUF` of type `ARRAY[32760..32770] OF BYTE` at `DB1.DBB0`, stored before ADR-0178
- **WHEN** a ProcessEngineer runs `AddParameter` on version 3 with the tag `MODE`, the type `INT`, and the address `DB2.DBW0`
- **THEN** the system adds the parameter `MODE`

#### Scenario: New version from a base with such a bound
- **GIVEN** the approved version 2 of `R-100` on `M3` with the parameter `BUF` of type `ARRAY[32760..32770] OF BYTE` at `DB1.DBB0`, stored before ADR-0178
- **WHEN** a ProcessEngineer runs `CreateVersion` from version 2
- **THEN** the system creates the draft version 3 with the parameter `BUF`

### Requirement: BOOL array with three dimensions and an S7 address
On `S7_300`, `S7_1200`, and `S7_1500`, the type tree of a parameter with an address SHALL hold no `BOOL` array with three dimensions. A break SHALL give `E-RCP-058` with the reason "a BOOL array with three dimensions has no verified S7 layout". The rule SHALL run with the rule of the array bounds (R03, R12, R31, A1, A38, ADR-0187).

#### Scenario: BOOL array with three dimensions and an address
- **GIVEN** the machine `M3` with the platform `S7_300` and the draft version 3 of `R-100`
- **WHEN** a ProcessEngineer runs `AddParameter` on version 3 with the type `ARRAY[1..3,1..3,1..3] OF BOOL` and the address `DB1.DBB0`
- **THEN** the system rejects the command with `E-RCP-058` and the reason "a BOOL array with three dimensions has no verified S7 layout"

#### Scenario: Struct member of a BOOL array with three dimensions
- **GIVEN** the machine `M3` with the platform `S7_300` and the draft version 3 of `R-100`
- **AND** the struct `ST_Cube@1` with the member `bits` of type `ARRAY[1..2,1..2,1..2] OF BOOL`
- **WHEN** a ProcessEngineer runs `AddParameter` on version 3 with the type `ST_Cube@1` and the address `DB1.DBB0`
- **THEN** the system rejects the command with `E-RCP-058`

#### Scenario: BOOL array with three dimensions without an address
- **GIVEN** the machine `M5` with the platform `S7_1500` and the draft version 3 of `R-100`
- **WHEN** a ProcessEngineer runs `AddParameter` on version 3 with the type `ARRAY[1..3,1..3,1..3] OF BOOL` and no address
- **THEN** the system adds the parameter

#### Scenario: BOOL array with two dimensions and an address
- **GIVEN** the machine `M3` with the platform `S7_300` and the draft version 3 of `R-100`
- **WHEN** a ProcessEngineer runs `AddParameter` on version 3 with the type `ARRAY[1..3,1..3] OF BOOL` and the address `DB1.DBB0`
- **THEN** the system adds the parameter

### Requirement: WCHAR and WSTRING characters on S7
On `S7_1200` and `S7_1500`, a `WCHAR` value SHALL be one Unicode scalar in `U+0001..=U+D7FF`. Each UTF-16 code unit of a `WSTRING` value SHALL be in `0x0001..=0xCFFF`. A break SHALL give `E-RCP-005` with the expected form of the type. `CodesysIec` and `TwinCat3` SHALL keep the rules of ADR-0137. The earlier rule set of ADR-0120 SHALL keep ADR-0137 for a stored value (R03, R11, R12, R19, R38, ADR-0180).

#### Scenario: WCHAR above U+D7FF on S7
- **GIVEN** the parameter `MARK` of type `WCHAR` in the draft version 3 of `R-100` on the machine `M5` with the platform `S7_1500`
- **WHEN** a ProcessEngineer runs `SetParameterValue` on `MARK` with the JSON string `"\uE000"`
- **THEN** the system rejects the command with `E-RCP-005`
- **AND** the value of `MARK` and the row version of the draft do not change

#### Scenario: Last WCHAR on S7
- **GIVEN** the parameter `MARK` of type `WCHAR` in the draft version 3 of `R-100` on `M5`
- **WHEN** a ProcessEngineer runs `SetParameterValue` on `MARK` with the JSON string `"\uD7FF"`
- **THEN** the system stores the value

#### Scenario: WSTRING character from U+D000 on S7
- **GIVEN** the parameter `NOTE` of type `WSTRING(10)` in the draft version 3 of `R-100` on the machine `M6` with the platform `S7_1200`
- **WHEN** a ProcessEngineer runs `SetParameterValue` on `NOTE` with the JSON string `"A\uD000"`
- **THEN** the system rejects the command with `E-RCP-005`

#### Scenario: WSTRING character outside the Basic Multilingual Plane on S7
- **GIVEN** the parameter `NOTE` of type `WSTRING(10)` in the draft version 3 of `R-100` on `M5`
- **WHEN** a ProcessEngineer runs `SetParameterValue` on `NOTE` with the JSON string `"\uD83D\uDE00"`
- **THEN** the system rejects the command with `E-RCP-005`

#### Scenario: WSTRING character below U+D000 on S7
- **GIVEN** the parameter `NOTE` of type `WSTRING(10)` in the draft version 3 of `R-100` on `M5`
- **WHEN** a ProcessEngineer runs `SetParameterValue` on `NOTE` with the JSON string `"Ab\u20AC\uCFFF"`
- **THEN** the system stores the value

#### Scenario: WSTRING character from U+D000 on CODESYS
- **GIVEN** the parameter `NOTE` of type `WSTRING(10)` in the draft version 3 of `R-100` on the machine `M9` with the platform `CodesysIec`
- **WHEN** a ProcessEngineer runs `SetParameterValue` on `NOTE` with the JSON string `"A\uD000"`
- **THEN** the system stores the value

#### Scenario: New WCHAR member of a rebind on S7
- **GIVEN** the draft version 3 of `R-100` on `M5` with the parameter `CFG` of type `ST_Cfg@1`
- **AND** the revision `ST_Cfg@2` with the new member `mark` of type `WCHAR`
- **WHEN** a ProcessEngineer runs `RebindTypeRevision` on version 3 from `ST_Cfg@1` to `ST_Cfg@2` with the value `"\uE000"` for the leaf `mark` of `CFG`
- **THEN** the system rejects the command with `E-RCP-005`
- **AND** `CFG` keeps the type `ST_Cfg@1`

#### Scenario: Stored WCHAR of the earlier rule set in a draft
- **GIVEN** the draft version 3 of `R-100` on `M5` with the parameter `MARK` of type `WCHAR` and the value `"\uE000"`, stored before ADR-0180
- **WHEN** a ProcessEngineer runs `SetParameterValue` on another parameter of version 3
- **THEN** the system stores the new value
- **AND** `MARK` keeps its value

#### Scenario: Stored WCHAR of the earlier rule set at approval
- **GIVEN** the draft version 3 of `R-100` on `M5` with the parameter `MARK` of type `WCHAR` and the value `"\uE000"`, stored before ADR-0180
- **WHEN** a ProcessEngineer runs `ApproveVersion` on version 3
- **THEN** the system rejects the command with `E-RCP-020`
- **AND** the list of items names `MARK`

#### Scenario: Stored WSTRING of the earlier rule set at approval
- **GIVEN** the draft version 3 of `R-100` on `M6` with the platform `S7_1200` and the parameter `NOTE` of type `WSTRING(10)` with the value `"A\uD000"`, stored before ADR-0180
- **WHEN** a ProcessEngineer runs `ApproveVersion` on version 3
- **THEN** the system rejects the command with `E-RCP-020`
- **AND** the list of items names `NOTE`

#### Scenario: New version from a base with such a WCHAR
- **GIVEN** the approved version 2 of `R-100` on `M5` with the parameter `MARK` of type `WCHAR` and the value `"\uE000"`, stored before ADR-0180
- **WHEN** a ProcessEngineer runs `CreateVersion` from version 2
- **THEN** the system creates the draft version 3 with the value `"\uE000"` of `MARK`

### Requirement: Decimal text of a numeric leaf only
The text of a `Numeric` leaf SHALL be decimal text. The system SHALL reject a based literal, a typed literal, and a digit separator with `E-RCP-005` (R11, R12, R38, ADR-0183).

#### Scenario: Based literal for a BYTE leaf
- **GIVEN** the parameter `MASK` of type `BYTE` in the draft version 3 of `R-100`
- **WHEN** a ProcessEngineer runs `SetParameterValue` on `MASK` with the text `16#FF`
- **THEN** the system rejects the command with `E-RCP-005`

#### Scenario: Typed literal for a REAL leaf
- **GIVEN** the parameter `GAIN` of type `REAL` in the draft version 3 of `R-100`
- **WHEN** a ProcessEngineer runs `SetParameterValue` on `GAIN` with the text `REAL#1.5`
- **THEN** the system rejects the command with `E-RCP-005`

#### Scenario: Digit separator for a DWORD leaf
- **GIVEN** the parameter `COUNT` of type `DWORD` in the draft version 3 of `R-100`
- **WHEN** a ProcessEngineer runs `SetParameterValue` on `COUNT` with the text `1_000`
- **THEN** the system rejects the command with `E-RCP-005`
