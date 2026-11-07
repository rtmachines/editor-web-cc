## MODIFIED Requirements

### Requirement: JSON form of a leaf value
The system SHALL accept a leaf value only in the JSON form of its leaf class. A `BOOL` leaf SHALL accept only `true` and `false`. A `CHAR` or `WCHAR` leaf SHALL accept one Unicode scalar from `U+0001` to the end of its type range. A `STRING` leaf SHALL accept only Latin-1 characters, and the number of characters SHALL fit the type length. A `WSTRING` value SHALL fit the type length in UTF-16 units. A `STRING` or `WSTRING` value SHALL hold no `U+0000` (R11, R12, R31, R38).

#### Scenario: Number for a BOOL leaf
- **GIVEN** the parameter `ENABLE` of type `BOOL` in the draft version 3 of `R-100`
- **WHEN** a ProcessEngineer runs `SetParameterValue` on `ENABLE` with the JSON number `1`
- **THEN** the system rejects the command with `E-RCP-005`

#### Scenario: Two characters for a CHAR leaf
- **GIVEN** the parameter `SEP` of type `CHAR` in the draft version 3 of `R-100`
- **WHEN** a ProcessEngineer runs `SetParameterValue` on `SEP` with the JSON string `"ab"`
- **THEN** the system rejects the command with `E-RCP-005`

#### Scenario: Character outside Latin-1 in a STRING leaf
- **GIVEN** the parameter `LABEL` of type `STRING(10)` in the draft version 3 of `R-100`
- **WHEN** a ProcessEngineer runs `SetParameterValue` on `LABEL` with the JSON string `"Łódź"`
- **THEN** the system rejects the command with `E-RCP-005`

#### Scenario: STRING value longer than its type
- **GIVEN** the parameter `LABEL` of type `STRING(4)` in the draft version 3 of `R-100`
- **WHEN** a ProcessEngineer runs `SetParameterValue` on `LABEL` with the JSON string `"ABCDE"`
- **THEN** the system rejects the command with `E-RCP-005`

#### Scenario: WSTRING length in UTF-16 units
- **GIVEN** the parameter `NOTE` of type `WSTRING(2)` in the draft version 3 of `R-100`
- **WHEN** a ProcessEngineer runs `SetParameterValue` on `NOTE` with the JSON string `"a😀"`, which has 3 UTF-16 units
- **THEN** the system rejects the command with `E-RCP-005`

#### Scenario: STRING length in characters and not in UTF-8 bytes
- **GIVEN** the parameter `LABEL` of type `STRING(4)` in the draft version 3 of `R-100`
- **WHEN** a ProcessEngineer runs `SetParameterValue` on `LABEL` with the JSON string `"éééé"`, which has 8 UTF-8 bytes
- **THEN** the system stores the value `"éééé"`

#### Scenario: U+0000 for a CHAR leaf
- **GIVEN** the parameter `SEP` of type `CHAR` in the draft version 3 of `R-100`
- **WHEN** a ProcessEngineer runs `SetParameterValue` on `SEP` with the JSON string `"\u0000"`
- **THEN** the system rejects the command with `E-RCP-005`
- **AND** the value of `SEP` and the row version of the draft do not change

#### Scenario: U+0000 for a WCHAR leaf
- **GIVEN** the parameter `MARK` of type `WCHAR` in a draft version of a recipe on a machine with the platform `S7_1500`
- **WHEN** a ProcessEngineer runs `SetParameterValue` on `MARK` with the JSON string `"\u0000"`
- **THEN** the system rejects the command with `E-RCP-005`

#### Scenario: U+0000 inside a STRING value
- **GIVEN** the parameter `LABEL` of type `STRING(10)` in the draft version 3 of `R-100`
- **WHEN** a ProcessEngineer runs `SetParameterValue` on `LABEL` with the JSON string `"AB\u0000"`
- **THEN** the system rejects the command with `E-RCP-005`

#### Scenario: U+0000 inside a WSTRING value
- **GIVEN** the parameter `NOTE` of type `WSTRING(10)` in the draft version 3 of `R-100`
- **WHEN** a ProcessEngineer runs `SetParameterValue` on `NOTE` with the JSON string `"A\u0000"`
- **THEN** the system rejects the command with `E-RCP-005`

#### Scenario: U+0000 in the form text of a new parameter
- **GIVEN** the draft version 3 of `R-100`
- **WHEN** a ProcessEngineer runs `AddParameter` with the tag `HOST`, the type `STRING(10)`, and a value text that holds `U+0000`
- **THEN** the system rejects the command with `E-RCP-005`
- **AND** the draft holds no parameter `HOST`

#### Scenario: U+0000 in the value of a new type
- **GIVEN** the parameter `SPEED` of type `INT` in the draft version 3 of `R-100`
- **WHEN** a ProcessEngineer runs `UpdateParameterDefinition` on `SPEED` with the type `CHAR` and a value text that holds `U+0000`
- **THEN** the system rejects the command with `E-RCP-005`
- **AND** the type of `SPEED` stays `INT`

#### Scenario: U+0000 in a new member of a rebind
- **GIVEN** the parameter `ZONE_1` of type `ST_Zone@2` and the revision `ST_Zone@3` with the new member `label` of type `STRING(10)`
- **WHEN** a ProcessEngineer runs `RebindTypeRevision` with a text that holds `U+0000` for `ZONE_1` and `.label`
- **THEN** the system rejects the command with `E-RCP-005`
- **AND** the type of `ZONE_1` stays `ST_Zone@2`

#### Scenario: Lowest character of a CHAR leaf
- **GIVEN** the parameter `SEP` of type `CHAR` in the draft version 3 of `R-100`
- **WHEN** a ProcessEngineer runs `SetParameterValue` on `SEP` with the JSON string `"\u0001"`
- **THEN** the system stores the value `"\u0001"`

## ADDED Requirements

### Requirement: Unit of the max_length of a string limit
The system SHALL count `max_length` of a process limit in the unit of the type length. A `STRING` limit SHALL count characters. A `WSTRING` limit SHALL count UTF-16 code units, and a character outside the Basic Multilingual Plane SHALL count 2 (R09, R11, R38).

#### Scenario: max_length of a WSTRING counts UTF-16 units
- **GIVEN** the parameter `NOTE` of type `WSTRING(10)` with the process limit `max_length` 2 in the draft version 3 of `R-100`
- **WHEN** a ProcessEngineer runs `SetParameterValue` on `NOTE` with the JSON string `"a😀"`, which has 3 UTF-16 units
- **THEN** the system rejects the command with `E-RCP-004`

#### Scenario: max_length of a STRING counts characters
- **GIVEN** the parameter `LABEL` of type `STRING(4)` with the process limit `max_length` 2 in the draft version 3 of `R-100`
- **WHEN** a ProcessEngineer runs `SetParameterValue` on `LABEL` with the JSON string `"éé"`, which has 4 UTF-8 bytes
- **THEN** the system stores the value `"éé"`
