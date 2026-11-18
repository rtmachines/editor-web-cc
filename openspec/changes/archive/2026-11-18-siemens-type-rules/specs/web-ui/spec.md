## ADDED Requirements

### Requirement: Expected form of a wide text value on S7
The message of `E-RCP-005` for a `WCHAR` or `WSTRING` value on `S7_1200` or `S7_1500` SHALL name the range of the platform in the UI language. The `WCHAR` form SHALL name `U+D7FF`. The `WSTRING` form SHALL name the length and the range `U+0001` to `U+CFFF`. Both texts SHALL come from the bundles `pl` and `en` (R10, R36, ADR-0180).

#### Scenario: Refused WSTRING character in English
- **GIVEN** the parameter `NOTE` of type `WSTRING(10)` in the draft version 3 of `R-100` on the machine `M5` with the platform `S7_1500`
- **AND** a ProcessEngineer with the UI language `en`
- **WHEN** the ProcessEngineer saves the value `"A\uE000"` for `NOTE`
- **THEN** `#error-panel` shows `E-RCP-005` with a text that names the length 10 and the range `U+0001` to `U+CFFF`

#### Scenario: Refused WCHAR in Polish
- **GIVEN** the parameter `MARK` of type `WCHAR` in the draft version 3 of `R-100` on `M5`
- **AND** a ProcessEngineer with the UI language `pl`
- **WHEN** the ProcessEngineer saves the value `"\uE000"` for `MARK`
- **THEN** `#error-panel` shows `E-RCP-005` with the Polish text of the bundle `pl` that names `U+D7FF`

#### Scenario: Refused WSTRING character on CODESYS
- **GIVEN** the parameter `NOTE` of type `WSTRING(10)` in the draft version 3 of `R-100` on the machine `M9` with the platform `CodesysIec`
- **WHEN** a ProcessEngineer saves the value `"A\u0000"` for `NOTE`
- **THEN** `#error-panel` shows `E-RCP-005` with the text of ADR-0137 that names the length 10 and no range

### Requirement: S7 nesting limit in the message of E-PLT-018
The message of `E-PLT-018` for a type over `type.s7_max_nesting` SHALL name the limit 8 and the S7 count in the UI language. The text SHALL come from the bundles `pl` and `en`. The message of the limit `json.max_depth` SHALL keep its text (R10, R36, ADR-0179).

#### Scenario: Nine levels on S7 in English
- **GIVEN** the machine `M5` with the platform `S7_1500` and a ProcessEngineer with the UI language `en`
- **WHEN** the ProcessEngineer saves a struct `ST_Top` whose type has nine levels in the S7 count
- **THEN** `#error-panel` shows `E-PLT-018` with a text that names 8 levels of the S7 count

#### Scenario: Nine levels on S7 in Polish
- **GIVEN** the machine `M5` with the platform `S7_1500` and a ProcessEngineer with the UI language `pl`
- **WHEN** the ProcessEngineer saves a struct `ST_Top` whose type has nine levels in the S7 count
- **THEN** `#error-panel` shows `E-PLT-018` with the Polish text of the bundle `pl` that names 8 levels of the S7 count

### Requirement: Line and link of E-RCP-058
A form answer with `E-RCP-058` SHALL show its line under the input `s7_address`. The approval preview SHALL link the parameter of the tag that `E-RCP-058` names (R10, R36, ADR-0041, ADR-0049, ADR-0178).

#### Scenario: Array bound above 32767 in the parameter form
- **GIVEN** the machine `M3` with the platform `S7_300` and the draft version 3 of `R-100`
- **WHEN** a ProcessEngineer saves the parameter form with the type `ARRAY[32760..32770] OF BYTE` and the address `DB1.DBB0`
- **THEN** the line of `E-RCP-058` stands under the input `s7_address`

#### Scenario: Array bound above 32767 in the approval preview
- **GIVEN** the draft version 3 of `R-100` on `M3` with the parameter `BUF` of type `ARRAY[32760..32770] OF BYTE` at `DB1.DBB0`, stored before ADR-0178
- **WHEN** a ProcessEngineer opens the approval preview of version 3
- **THEN** the preview shows `E-RCP-058` with a link to the parameter `BUF`
