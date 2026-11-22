## MODIFIED Requirements

### Requirement: JSON form of a leaf value
The system SHALL accept a leaf value only in the JSON form of its leaf class. A `BOOL` leaf SHALL accept only `true` and `false`. A `CHAR` or `WCHAR` leaf SHALL accept one Unicode scalar from `U+0001` to the end of its type range, except LF (`U+000A`) and CR (`U+000D`). On a `Latin1` machine, a `STRING` leaf SHALL accept only Latin-1 characters, and their number SHALL fit the type length. A `WSTRING` value SHALL fit the type length in UTF-16 units. A `STRING` or `WSTRING` value SHALL hold no `U+0000`, no LF, and no CR. The rule on LF and CR SHALL apply only to a value that a command receives. A stored value SHALL stay readable, approvable, and loadable (R11, R12, R31, R38, ADR-0202, ADR-0210).

#### Scenario: Number for a BOOL leaf
- **GIVEN** the parameter `ENABLE` of type `BOOL` in the draft version 3 of `R-100`
- **WHEN** a ProcessEngineer runs `SetParameterValue` on `ENABLE` with the JSON number `1`
- **THEN** the system rejects the command with `E-RCP-005`

#### Scenario: Two characters for a CHAR leaf
- **GIVEN** the parameter `SEP` of type `CHAR` in the draft version 3 of `R-100`
- **WHEN** a ProcessEngineer runs `SetParameterValue` on `SEP` with the JSON string `"ab"`
- **THEN** the system rejects the command with `E-RCP-005`

#### Scenario: Character outside Latin-1 in a STRING leaf
- **GIVEN** the parameter `LABEL` of type `STRING(10)` in the draft version 3 of `R-100` on `M8` with the encoding `Latin1`
- **WHEN** a ProcessEngineer runs `SetParameterValue` on `LABEL` with the JSON string `"Łódź"`
- **THEN** the system rejects the command with `E-RCP-005`

#### Scenario: STRING value longer than its type
- **GIVEN** the parameter `LABEL` of type `STRING(4)` in the draft version 3 of `R-100` on `M8` with the encoding `Latin1`
- **WHEN** a ProcessEngineer runs `SetParameterValue` on `LABEL` with the JSON string `"ABCDE"`
- **THEN** the system rejects the command with `E-RCP-005`

#### Scenario: WSTRING length in UTF-16 units
- **GIVEN** the parameter `NOTE` of type `WSTRING(2)` in the draft version 3 of `R-100`
- **WHEN** a ProcessEngineer runs `SetParameterValue` on `NOTE` with the JSON string `"a😀"`, which has 3 UTF-16 units
- **THEN** the system rejects the command with `E-RCP-005`

#### Scenario: STRING length in characters and not in UTF-8 bytes
- **GIVEN** the parameter `LABEL` of type `STRING(4)` in the draft version 3 of `R-100` on `M8` with the encoding `Latin1`
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
- **GIVEN** the parameter `LABEL` of type `STRING(10)` in the draft version 3 of `R-100` on `M8` with the encoding `Latin1`
- **WHEN** a ProcessEngineer runs `SetParameterValue` on `LABEL` with the JSON string `"AB\u0000"`
- **THEN** the system rejects the command with `E-RCP-005`

#### Scenario: U+0000 inside a WSTRING value
- **GIVEN** the parameter `NOTE` of type `WSTRING(10)` in the draft version 3 of `R-100`
- **WHEN** a ProcessEngineer runs `SetParameterValue` on `NOTE` with the JSON string `"A\u0000"`
- **THEN** the system rejects the command with `E-RCP-005`

#### Scenario: U+0000 in the form text of a new parameter
- **GIVEN** the draft version 3 of `R-100` on `M8` with the encoding `Latin1`
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

#### Scenario: STRING length in UTF-8 bytes on a Utf8 machine
- **GIVEN** the parameter `LABEL` of type `STRING(4)` in the draft version 3 of `R-100` on the machine `M9` with the platform `CodesysIec` and the encoding `Utf8`
- **WHEN** a ProcessEngineer runs `SetParameterValue` on `LABEL` with the JSON string `"ééé"`, which has 6 UTF-8 bytes
- **THEN** the system rejects the command with `E-RCP-005`

#### Scenario: LF inside a STRING value
- **GIVEN** the parameter `LABEL` of type `STRING(10)` in the draft version 3 of `R-100` on `M8` with the encoding `Latin1`
- **WHEN** a ProcessEngineer runs `SetParameterValue` on `LABEL` with the JSON string `"A\nB"`
- **THEN** the system rejects the command with `E-RCP-005`
- **AND** the value of `LABEL` and the row version of the draft do not change

#### Scenario: CR for a WCHAR leaf
- **GIVEN** the parameter `MARK` of type `WCHAR` in a draft version of a recipe on a machine with the platform `S7_1500`
- **WHEN** a ProcessEngineer runs `SetParameterValue` on `MARK` with the JSON string `"\r"`
- **THEN** the system rejects the command with `E-RCP-005`

#### Scenario: LF in the form text of a new parameter
- **GIVEN** the draft version 3 of `R-100` on `M8` with the encoding `Latin1`
- **WHEN** a ProcessEngineer runs `AddParameter` with the tag `HOST`, the type `STRING(10)`, and the value text `A` LF `B`
- **THEN** the system rejects the command with `E-RCP-005`
- **AND** the draft holds no parameter `HOST`

#### Scenario: CR in a new member of a rebind
- **GIVEN** the parameter `ZONE_1` of type `ST_Zone@2` and the revision `ST_Zone@3` with the new member `label` of type `STRING(10)`
- **WHEN** a ProcessEngineer runs `RebindTypeRevision` with a text that holds CR for `ZONE_1` and `.label`
- **THEN** the system rejects the command with `E-RCP-005`
- **AND** the type of `ZONE_1` stays `ST_Zone@2`

#### Scenario: Stored value with LF stays loadable
- **GIVEN** the `Approved` version 2 of `R-100` with the stored value `"A\nB"` of `LABEL` from before ADR-0210
- **WHEN** an ExportClient runs `ExportVersion` on version 2
- **THEN** the system exports version 2
- **AND** the export holds the value `"A\nB"` of `LABEL`

#### Scenario: Stored value with LF in a draft
- **GIVEN** the draft version 3 of `R-100` with the stored value `"A\nB"` of `LABEL` from before ADR-0210, and every other approval rule kept
- **WHEN** an Administrator runs `ApproveVersion` on version 3
- **THEN** the system approves version 3

### Requirement: STRING values on a Utf8 machine
On a machine with the `STRING` encoding `Utf8`, a `STRING` character SHALL be any Unicode scalar except `U+0000`. A value that a command receives SHALL also hold no LF and no CR (ADR-0210). The UTF-8 bytes of the value SHALL fit the type length. A break SHALL give `E-RCP-005` with the expected form in bytes. The snapshot check, the approval, and the export check SHALL use the same rule without the rule on LF and CR (R11, R38, ADR-0202, ADR-0210).

#### Scenario: Polish letters in a STRING of a Utf8 machine
- **GIVEN** the parameter `LABEL` of type `STRING(10)` in the draft version 3 of `R-100` on the machine `M9` with the platform `CodesysIec` and the encoding `Utf8`
- **WHEN** a ProcessEngineer runs `SetParameterValue` on `LABEL` with the JSON string `"Łódź"`, which has 7 UTF-8 bytes
- **THEN** the system stores the value `"Łódź"`

#### Scenario: Character outside the Basic Multilingual Plane in a STRING of a Utf8 machine
- **GIVEN** the parameter `LABEL` of type `STRING(4)` in the draft version 3 of `R-100` on `M9` with the encoding `Utf8`
- **WHEN** a ProcessEngineer runs `SetParameterValue` on `LABEL` with the JSON string `"😀"`, which has 4 UTF-8 bytes
- **THEN** the system stores the value

#### Scenario: Too many UTF-8 bytes in a form text
- **GIVEN** the draft version 3 of `R-100` on `M9` with the encoding `Utf8`
- **WHEN** a ProcessEngineer runs `AddParameter` with the tag `HOST`, the type `STRING(2)`, and the value text `ąę`, which has 4 UTF-8 bytes
- **THEN** the system rejects the command with `E-RCP-005`
- **AND** the expected form names the length 2 in bytes

#### Scenario: U+0000 in a STRING of a Utf8 machine
- **GIVEN** the parameter `LABEL` of type `STRING(10)` in the draft version 3 of `R-100` on `M9` with the encoding `Utf8`
- **WHEN** a ProcessEngineer runs `SetParameterValue` on `LABEL` with the JSON string `"A\u0000"`
- **THEN** the system rejects the command with `E-RCP-005`

#### Scenario: Latin1 machine on TwinCAT keeps Latin-1
- **GIVEN** the parameter `LABEL` of type `STRING(10)` in the draft version 3 of `R-100` on `M8` with the encoding `Latin1`
- **WHEN** a ProcessEngineer runs `SetParameterValue` on `LABEL` with the JSON string `"Łódź"`
- **THEN** the system rejects the command with `E-RCP-005`

#### Scenario: Too many UTF-8 bytes in a JSON value
- **GIVEN** the parameter `LABEL` of type `STRING(6)` in the draft version 3 of `R-100` on `M9` with the encoding `Utf8`
- **WHEN** a ProcessEngineer runs `SetParameterValue` on `LABEL` with the JSON string `"Łódź"`, which has 7 UTF-8 bytes
- **THEN** the system rejects the command with `E-RCP-005`

#### Scenario: LF in a STRING of a Utf8 machine
- **GIVEN** the parameter `LABEL` of type `STRING(10)` in the draft version 3 of `R-100` on `M9` with the encoding `Utf8`
- **WHEN** a ProcessEngineer runs `SetParameterValue` on `LABEL` with the JSON string `"Ł\nź"`
- **THEN** the system rejects the command with `E-RCP-005`

### Requirement: Type change needs a new value
When `UpdateParameterDefinition` changes the type of a parameter, the request SHALL carry a new value and new limits. Without a type change, the value and the limits SHALL stay, unless the request carries a new value and new limits. Then the system SHALL replace them and check them in the same way as after a type change. Without a type change, the value fields and the limit fields SHALL come only as a pair. A type with no leaf that accepts a limit has no limit fields. A request that carries only one part of the pair SHALL get `422 E-WEB-008`, and the parameter SHALL NOT change (R08, R09, R19, R38, I7, ADR-0213).

#### Scenario: Type change without a value
- **GIVEN** the parameter `SPEED` of type `INT` in the draft version 3 of `R-100`
- **WHEN** a ProcessEngineer runs `UpdateParameterDefinition` on `SPEED` with the type `DINT` and no value
- **THEN** the system rejects the command with `E-RCP-005`

#### Scenario: Same type without a value
- **GIVEN** the parameter `SPEED` of type `INT` with the value `120` in the draft version 3 of `R-100`
- **WHEN** a ProcessEngineer runs `UpdateParameterDefinition` on `SPEED` with the type `INT`, a new name, and no value
- **THEN** the value and the limits of `SPEED` stay unchanged

#### Scenario: Same type with a new value
- **GIVEN** the parameter `SPEED` of type `INT` with the value `120` in the draft version 3 of `R-100`
- **WHEN** a ProcessEngineer runs `UpdateParameterDefinition` on `SPEED` with the type `INT`, the value `130`, and the limit `min 10, max 250`
- **THEN** the value of `SPEED` is `130`
- **AND** the audit entry `ParameterDefinitionUpdated` holds the old and the new value

#### Scenario: Same type with a value outside the new limit
- **GIVEN** the parameter `SPEED` of type `INT` in the draft version 3 of `R-100`
- **WHEN** a ProcessEngineer runs `UpdateParameterDefinition` on `SPEED` with the type `INT`, the value `300`, and the limit `min 10, max 250`
- **THEN** the system rejects the command with `E-RCP-004`
- **AND** the parameter `SPEED` does not change

#### Scenario: Same type with a new value and no limit fields
- **GIVEN** the parameter `SPEED` of type `INT` with the value `120` and the limit `min 10, max 250` in the draft version 3 of `R-100`
- **WHEN** a ProcessEngineer runs `UpdateParameterDefinition` on `SPEED` with the type `INT`, the value `130`, and no field `limits[i]`
- **THEN** the system rejects the request with `422 E-WEB-008`
- **AND** the value `120` and the limit `min 10, max 250` of `SPEED` stay unchanged

#### Scenario: Same type with limit fields and no value
- **GIVEN** the parameter `SPEED` of type `INT` with the value `120` in the draft version 3 of `R-100`
- **WHEN** a ProcessEngineer runs `UpdateParameterDefinition` on `SPEED` with the type `INT`, the limit `min 10, max 250`, and no value field
- **THEN** the system rejects the request with `422 E-WEB-008`
- **AND** the parameter `SPEED` does not change

#### Scenario: Same type without limits on a BOOL parameter
- **GIVEN** the parameter `ENABLE` of type `BOOL` with the value `false` in the draft version 3 of `R-100`
- **WHEN** a ProcessEngineer runs `UpdateParameterDefinition` on `ENABLE` with the type `BOOL` and the value `true`
- **THEN** the value of `ENABLE` is `true`
