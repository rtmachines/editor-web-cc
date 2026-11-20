## ADDED Requirements

### Requirement: WSTRING characters on CODESYS and TwinCAT
On `CodesysIec` and `TwinCat3`, each UTF-16 code unit of a `WSTRING` value SHALL be in `0x0001..=0xD7FF` or `0xE000..=0xFFFF`. A break SHALL give `E-RCP-005` with the expected form of the type and the platform. The rule SHALL apply to a JSON value, to the text of a form field, and to a new leaf of a rebind. The earlier rule set of ADR-0120 SHALL keep ADR-0137 for a stored value (R03, R11, R19, R38, ADR-0201).

#### Scenario: Character outside the Basic Multilingual Plane on TwinCAT
- **GIVEN** the parameter `NOTE` of type `WSTRING(10)` in the draft version 3 of `R-100` on the machine `M8` with the platform `TwinCat3`
- **WHEN** a ProcessEngineer runs `SetParameterValue` on `NOTE` with the JSON string `"A😀"`
- **THEN** the system rejects the command with `E-RCP-005`
- **AND** the value of `NOTE` and the row version of the draft do not change

#### Scenario: Character outside the Basic Multilingual Plane on CODESYS
- **GIVEN** the parameter `NOTE` of type `WSTRING(10)` in the draft version 3 of `R-100` on the machine `M9` with the platform `CodesysIec`
- **WHEN** a ProcessEngineer runs `SetParameterValue` on `NOTE` with the JSON string `"Z𝄞"`
- **THEN** the system rejects the command with `E-RCP-005`

#### Scenario: Characters at the ends of the two ranges
- **GIVEN** the parameter `NOTE` of type `WSTRING(4)` in the draft version 3 of `R-100` on `M8`
- **WHEN** a ProcessEngineer runs `SetParameterValue` on `NOTE` with the JSON string `"\u0001퟿￿"`
- **THEN** the system stores the value

#### Scenario: Form text of a new parameter on TwinCAT
- **GIVEN** the draft version 3 of `R-100` on `M8`
- **WHEN** a ProcessEngineer runs `AddParameter` with the tag `HOST`, the type `WSTRING(10)`, and a value text that holds `U+1F600`
- **THEN** the system rejects the command with `E-RCP-005`
- **AND** the draft holds no parameter `HOST`

#### Scenario: New WSTRING member of a rebind on CODESYS
- **GIVEN** the draft version 3 of `R-100` on `M9` with the parameter `CFG` of type `ST_Cfg@1`
- **AND** the revision `ST_Cfg@2` with the new member `note` of type `WSTRING(10)`
- **WHEN** a ProcessEngineer runs `RebindTypeRevision` on version 3 from `ST_Cfg@1` to `ST_Cfg@2` with the value `"😀"` for the leaf `note` of `CFG`
- **THEN** the system rejects the command with `E-RCP-005`
- **AND** `CFG` keeps the type `ST_Cfg@1`

#### Scenario: Stored WSTRING of the earlier rule set in a draft
- **GIVEN** the draft version 3 of `R-100` on `M8` with the parameter `NOTE` of type `WSTRING(10)` and the value `"A😀"`, stored before ADR-0201
- **WHEN** a ProcessEngineer runs `SetParameterValue` on another parameter of version 3
- **THEN** the system stores the new value
- **AND** `NOTE` keeps its value

#### Scenario: Stored WSTRING of the earlier rule set at approval
- **GIVEN** the draft version 3 of `R-100` on `M8` with the parameter `NOTE` of type `WSTRING(10)` and the value `"A😀"`, stored before ADR-0201
- **WHEN** a ProcessEngineer runs `ApproveVersion` on version 3
- **THEN** the system rejects the command with `E-RCP-020`
- **AND** the list of items names `NOTE`

#### Scenario: New version from a base with such a WSTRING
- **GIVEN** the approved version 2 of `R-100` on `M8` with the parameter `NOTE` of type `WSTRING(10)` and the value `"A😀"`, stored before ADR-0201
- **WHEN** a ProcessEngineer runs `CreateVersion` from version 2
- **THEN** the system creates the draft version 3 with the value `"A😀"` of `NOTE`

### Requirement: STRING values on a Utf8 machine
On a machine with the `STRING` encoding `Utf8`, a `STRING` character SHALL be any Unicode scalar except `U+0000`. The UTF-8 bytes of the value SHALL fit the type length. A break SHALL give `E-RCP-005` with the expected form in bytes. The snapshot check, the approval, and the export check SHALL use the same rule (R11, R38, ADR-0202).

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

### Requirement: Long prefixes of IEC literals
The system SHALL accept the 8 long prefixes of ADR-0205 in an IEC literal on every platform. A long prefix SHALL be upper case and SHALL give the same count as its short prefix. The system SHALL show the literal with the short prefix. A limit SHALL accept the long prefixes too (R09, R11, R38, A43, ADR-0205).

#### Scenario: LTIME value with a long prefix
- **GIVEN** the parameter `DWELL` of type `LTIME` in the draft version 3 of `R-100` on `M9`
- **WHEN** a ProcessEngineer runs `SetParameterValue` on `DWELL` with the form text `LTIME#1h`
- **THEN** the system stores the same value as for the form text `LT#1h`
- **AND** the parameter table shows the literal with the prefix `LT#`

#### Scenario: TIME_OF_DAY value with a long prefix on S7
- **GIVEN** the parameter `START` of type `TIME_OF_DAY` in the draft version 3 of `R-100` on the machine `M5` with the platform `S7_1500`
- **WHEN** a ProcessEngineer runs `SetParameterValue` on `START` with the form text `TIME_OF_DAY#06:00:00`
- **THEN** the system stores the same value as for the form text `TOD#06:00:00`

#### Scenario: Long prefix in lower case
- **GIVEN** the parameter `DWELL` of type `LTIME` in the draft version 3 of `R-100` on `M9`
- **WHEN** a ProcessEngineer runs `SetParameterValue` on `DWELL` with the form text `ltime#1us`
- **THEN** the system rejects the command with `E-RCP-005`

#### Scenario: Limit with a long prefix
- **GIVEN** the parameter `CYCLE` of type `TIME` in the draft version 3 of `R-100` on `M9`
- **WHEN** a ProcessEngineer runs `ReplaceLimits` on `CYCLE` with the `min` value `TIME#1s`
- **THEN** the system stores the `min` value with the same count as `T#1s`

#### Scenario: No long prefix for S5TIME
- **GIVEN** the parameter `DELAY` of type `S5TIME` in the draft version 3 of `R-100` on `M5`
- **WHEN** a ProcessEngineer runs `SetParameterValue` on `DELAY` with the form text `S5TIME#1s`
- **THEN** the system rejects the command with `E-RCP-005`

### Requirement: Order of array dimensions in a JSON value
The first dimension of an array type SHALL be the outer level of its JSON value. The last index SHALL change fastest (R13, R38, ADR-0209).

#### Scenario: Two rows of three elements
- **GIVEN** the parameter `GRID` of type `ARRAY[1..2,1..3] OF INT` in the draft version 3 of `R-100`
- **WHEN** a ProcessEngineer runs `SetParameterValue` on `GRID` with the JSON value `[[1,2,3],[4,5,6]]`
- **THEN** the system stores the value
- **AND** the leaf `GRID[2,1]` holds `4`

#### Scenario: Three rows of two elements
- **GIVEN** the parameter `GRID` of type `ARRAY[1..2,1..3] OF INT` in the draft version 3 of `R-100`
- **WHEN** a ProcessEngineer runs `SetParameterValue` on `GRID` with the JSON value `[[1,2],[3,4],[5,6]]`
- **THEN** the system rejects the command with `E-RCP-005`

### Requirement: No NaN and no infinity in a REAL or LREAL leaf
A `REAL` or `LREAL` leaf SHALL reject a value text that names NaN or an infinity, with `E-RCP-005` (R11, R12, R38).

#### Scenario: NaN for a REAL leaf
- **GIVEN** the parameter `RATIO` of type `REAL` in the draft version 3 of `R-100` on `M9`
- **WHEN** a ProcessEngineer runs `SetParameterValue` on `RATIO` with the JSON string `"NaN"`
- **THEN** the system rejects the command with `E-RCP-005`

#### Scenario: Infinity for an LREAL leaf
- **GIVEN** the parameter `GAIN` of type `LREAL` in the draft version 3 of `R-100` on `M8`
- **WHEN** a ProcessEngineer runs `SetParameterValue` on `GAIN` with the JSON string `"-Infinity"`
- **THEN** the system rejects the command with `E-RCP-005`

### Requirement: Last Latin-1 character of a STRING value
On a `Latin1` machine, `U+00FF` SHALL be the last permitted character of a `STRING` value. `U+0100` SHALL give `E-RCP-005` (R11, R12, R38, ADR-0137).

#### Scenario: Last Latin-1 character
- **GIVEN** the parameter `LABEL` of type `STRING(4)` in the draft version 3 of `R-100` on `M8` with the encoding `Latin1`
- **WHEN** a ProcessEngineer runs `SetParameterValue` on `LABEL` with the JSON string `"ÿ"`
- **THEN** the system stores the value

#### Scenario: First character after Latin-1
- **GIVEN** the parameter `LABEL` of type `STRING(4)` in the draft version 3 of `R-100` on `M8` with the encoding `Latin1`
- **WHEN** a ProcessEngineer runs `SetParameterValue` on `LABEL` with the JSON string `"Ā"`
- **THEN** the system rejects the command with `E-RCP-005`

#### Scenario: First character after Latin-1 on a Utf8 machine
- **GIVEN** the parameter `LABEL` of type `STRING(4)` in the draft version 3 of `R-100` on `M9` with the encoding `Utf8`
- **WHEN** a ProcessEngineer runs `SetParameterValue` on `LABEL` with the JSON string `"Ā"`, which has 2 UTF-8 bytes
- **THEN** the system stores the value `"Ā"`

## MODIFIED Requirements

### Requirement: JSON form of a leaf value
The system SHALL accept a leaf value only in the JSON form of its leaf class. A `BOOL` leaf SHALL accept only `true` and `false`. A `CHAR` or `WCHAR` leaf SHALL accept one Unicode scalar from `U+0001` to the end of its type range. On a `Latin1` machine, a `STRING` leaf SHALL accept only Latin-1 characters, and the number of characters SHALL fit the type length. On a `Utf8` machine, the UTF-8 bytes of a `STRING` value SHALL fit the type length. A `WSTRING` value SHALL fit the type length in UTF-16 units. A `STRING` or `WSTRING` value SHALL hold no `U+0000` (R11, R12, R31, R38, ADR-0202).

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


### Requirement: Unit of the max_length of a string limit
The system SHALL count `max_length` of a process limit in the unit of the type length. A `STRING` limit SHALL count characters on a `Latin1` machine and UTF-8 bytes on a `Utf8` machine. A `WSTRING` limit SHALL count UTF-16 code units, and a stored character outside the Basic Multilingual Plane SHALL count 2 (R09, R11, R38, ADR-0201, ADR-0202).

#### Scenario: max_length of a WSTRING counts UTF-16 units
- **GIVEN** the parameter `NOTE` of type `WSTRING(10)` with the value `"a😀"`, stored before ADR-0201, in the draft version 3 of `R-100` on `M8`
- **WHEN** a ProcessEngineer runs `ReplaceLimits` on `NOTE` with the `max_length` 2
- **THEN** the system rejects the command with `E-RCP-004`, because the value has 3 UTF-16 units

#### Scenario: max_length of a STRING counts characters
- **GIVEN** the parameter `LABEL` of type `STRING(4)` with the process limit `max_length` 2 in the draft version 3 of `R-100` on `M8` with the encoding `Latin1`
- **WHEN** a ProcessEngineer runs `SetParameterValue` on `LABEL` with the JSON string `"éé"`, which has 4 UTF-8 bytes
- **THEN** the system stores the value `"éé"`

#### Scenario: max_length of a STRING counts UTF-8 bytes on a Utf8 machine
- **GIVEN** the parameter `LABEL` of type `STRING(10)` with the process limit `max_length` 3 in the draft version 3 of `R-100` on `M9` with the encoding `Utf8`
- **WHEN** a ProcessEngineer runs `SetParameterValue` on `LABEL` with the JSON string `"éé"`, which has 4 UTF-8 bytes
- **THEN** the system rejects the command with `E-RCP-004`


### Requirement: WCHAR and WSTRING characters on S7
On `S7_1200` and `S7_1500`, a `WCHAR` value SHALL be one Unicode scalar in `U+0001..=U+D7FF`. Each UTF-16 code unit of a `WSTRING` value SHALL be in `0x0001..=0xCFFF`. A break SHALL give `E-RCP-005` with the expected form of the type. `CodesysIec` and `TwinCat3` SHALL use the rule of ADR-0201. The earlier rule set of ADR-0120 SHALL keep ADR-0137 for a stored value (R03, R11, R12, R19, R38, ADR-0180, ADR-0201).

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
