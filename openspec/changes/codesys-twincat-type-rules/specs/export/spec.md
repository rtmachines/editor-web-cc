## ADDED Requirements

### Requirement: STRING encoding in the export document v2
The v2 document of a `Utf8` machine SHALL hold `machine.string_encoding` with the value `Utf8`, directly after `machine.plc_platform`. The scope hashed body SHALL then hold the field too. The v2 document of a `Latin1` machine SHALL not hold the field, and ADR-0202 SHALL add no byte to it. The v2 schema file SHALL permit the field only on `CodesysIec` and `TwinCat3`. The system SHALL read the encoding from the machine at each export (R03, R31, A2, ADR-0202).

#### Scenario: Unit document of a Utf8 machine
- **GIVEN** the approved version 1 of `R-300` on the machine `M10` with the platform `CodesysIec` and the encoding `Utf8`
- **WHEN** the `ExportClient` account `plc-m10` sends `GET /export/v2/machines/M10/recipes/R-300/approved/units/U1`
- **THEN** the document holds `machine.string_encoding` with the value `Utf8` directly after `machine.plc_platform`
- **AND** `scope_sha256` covers a scope hashed body that holds `string_encoding`

#### Scenario: Unit document of a Latin1 machine
- **GIVEN** the approved version 3 of `R-100` on the machine `M12` and a `VersionExported` entry of `U1.2` from before this change
- **AND** no stored value of version 3 breaks the `WSTRING` rule of ADR-0201
- **WHEN** the `ExportClient` account `plc-m12` sends `GET /export/v2/machines/M12/recipes/R-100/approved/units/U1.2`
- **THEN** the document holds no field `machine.string_encoding`
- **AND** `scope_sha256` equals the value of the earlier `VersionExported` entry
- **AND** the document has the bytes of the same export in the previous release (ADR-0202 (j2))

#### Scenario: Unit document of a Latin1 machine on CODESYS
- **GIVEN** the approved version 1 of `R-200` on a `CodesysIec` machine with the encoding `Latin1` and the `STRING(4)` value `"é"`
- **WHEN** the `ExportClient` account of that machine runs `ExportVersion` for the unit `U1`
- **THEN** the document holds no field `machine.string_encoding`
- **AND** the document passes the v2 schema file

#### Scenario: Encoding Utf8 in a document of an S7 machine
- **GIVEN** a v2 document of an `S7_300`, `S7_1200`, or `S7_1500` machine with `machine.string_encoding` with the value `Utf8`
- **WHEN** a consumer checks the document against the v2 schema file
- **THEN** the check fails (ADR-0202 (j3))

#### Scenario: Document of a CODESYS or TwinCAT machine without the field
- **GIVEN** a v2 document of a `CodesysIec` or `TwinCat3` machine with no field `machine.string_encoding`
- **WHEN** a consumer checks the document against the v2 schema file
- **THEN** the check passes

### Requirement: Loadable flag of a v1 document of a Utf8 machine
A v1 document of a `Utf8` machine SHALL hold `loadable` with the value `false`. The system SHALL set this flag before it writes the document, on every v1 path. The v1 hashed body and `content_sha256` SHALL not change. The encoding SHALL not count as a rule fault. The v2 document of the same version SHALL keep the result of the current rules (R03, R31, A2, ADR-0202).

#### Scenario: v1 document of a Utf8 machine
- **GIVEN** the approved version 1 of `R-300` on `M10` with the encoding `Utf8` and no part that breaks a current rule
- **WHEN** an Operator runs `ExportVersion` through `GET /export/v1/recipes/R-300/approved`
- **THEN** the document holds `loadable` with the value `false`
- **AND** the document holds the stored `content_sha256` of version 1

#### Scenario: v1 document by version number of a Utf8 machine
- **GIVEN** the approved version 1 of `R-300` on `M10` with the encoding `Utf8` and no part that breaks a current rule
- **WHEN** a ProcessEngineer runs `ExportVersion` through `GET /export/v1/recipes/R-300/versions/1`
- **THEN** the document holds `loadable` with the value `false`
- **AND** the version page of version 1 shows no rule notice

#### Scenario: v1 document of the machine route of a Utf8 machine
- **GIVEN** the approved version 1 of `R-300` on `M10` with the encoding `Utf8` and no part that breaks a current rule
- **WHEN** the `ExportClient` account `plc-m10` sends `GET /export/v1/machines/M10/recipes/R-300/approved`
- **THEN** the document holds `loadable` with the value `false`

#### Scenario: v2 document of the same version
- **GIVEN** the approved version 1 of `R-300` on `M10` with the encoding `Utf8` and no part that breaks a current rule
- **WHEN** the `ExportClient` account `plc-m10` sends `GET /export/v2/machines/M10/recipes/R-300/approved/units/U1`
- **THEN** the document holds `loadable` with the value `true`

### Requirement: WSTRING characters of CODESYS and TwinCAT in the export
The export check SHALL apply the `WSTRING` rule of ADR-0201 to each stored value on `CodesysIec` and `TwinCat3`. A stored value that breaks it SHALL give `loadable` `false`. The hashed body and `content_sha256` SHALL not change (R03, R31, A2, ADR-0201).

#### Scenario: Stored WSTRING outside the Basic Multilingual Plane
- **GIVEN** the approved version 2 of `R-100` on `M8` with the parameter `NOTE` of type `WSTRING(10)` and the value `"A😀"`, stored before ADR-0201
- **WHEN** a ProcessEngineer runs `ExportVersion` through `GET /export/v1/recipes/R-100/versions/2`
- **THEN** the document holds `loadable` with the value `false`
- **AND** the document holds the value `"A😀"` and the stored `content_sha256`

### Requirement: Type positions of the schema v2
The schema file v2 SHALL permit only the primitive names of the platform at the three type positions `parameters[].type_id`, `types[].members[].type_id`, and `types[].element_type_id`. A UUID reference SHALL stay valid at each position. A bare name `STRING` or `WSTRING` SHALL fail on every platform (R03, R11, A2, ADR-0208).

#### Scenario: CHAR in a CODESYS document
- **GIVEN** a valid v2 document of a `CodesysIec` machine
- **AND** the document changes only one type position to the name `CHAR`: `parameters[].type_id`, `types[].members[].type_id`, or `types[].element_type_id`
- **WHEN** a consumer checks each changed document against the v2 schema file
- **THEN** each check fails

#### Scenario: UUID reference at a type position
- **GIVEN** a valid v2 document of a `TwinCat3` machine with a UUID reference to a `types[]` entry at each of the three type positions
- **WHEN** a consumer checks the document against the v2 schema file
- **THEN** the check passes

#### Scenario: Bare STRING name at a type position
- **GIVEN** a valid v2 document of a `CodesysIec` machine
- **AND** the document changes only `parameters[].type_id` of one parameter to the name `STRING` or `WSTRING`
- **WHEN** a consumer checks the document against the v2 schema file
- **THEN** the check fails (ADR-0208 (a2))

### Requirement: Type and value rules of the schema v2
The schema file v2 SHALL apply the `StringLength` table to `types[].string_length` and SHALL permit at most `type.max_members` members. A parameter whose `type_id` is a primitive name SHALL have the JSON kind of its leaf class. The file SHALL hold no range rule and no rule that a stored row can break (R03, R11, R12, A2, ADR-0208).

#### Scenario: WSTRING length on S7-300
- **GIVEN** a v2 document of an `S7_300` machine with a `WSTRING` type entry and a `string_length`
- **WHEN** a consumer checks the document against the v2 schema file
- **THEN** the check fails

#### Scenario: Struct with too many members
- **GIVEN** a v2 document with a struct entry of 1025 members
- **WHEN** a consumer checks the document against the v2 schema file
- **THEN** the check fails

#### Scenario: Boolean value as a string
- **GIVEN** a v2 document with a parameter of `type_id` `BOOL` and the value `"true"`
- **WHEN** a consumer checks the document against the v2 schema file
- **THEN** the check fails

#### Scenario: Value outside the range
- **GIVEN** a v2 document with a parameter of `type_id` `INT` and the value `"40000"`
- **WHEN** a consumer checks the document against the v2 schema file
- **THEN** the check passes

#### Scenario: Stored rows of the earlier rule set
- **GIVEN** the v2 document of an approved version with three stored rows of the earlier rule set
- **AND** the rows are a `WSTRING` value outside the Basic Multilingual Plane, the struct `VERSION@1`, and a struct with one member
- **WHEN** a consumer checks the document against the v2 schema file
- **THEN** the check passes

#### Scenario: Encoding Latin1 in a document
- **GIVEN** a v2 document with `machine.string_encoding` with the value `Latin1`
- **WHEN** a consumer checks the document against the v2 schema file
- **THEN** the check fails

### Requirement: No layout in the export of CODESYS and TwinCAT
An export document of a `CodesysIec` or `TwinCat3` machine SHALL hold `s7_address` with the value `null` for each parameter. The transfer system writes each leaf through its symbol. Thus the system SHALL model no layout, no `pack_mode`, and no `EXTENDS` on these platforms (A31, A53, ADR-0207).

#### Scenario: Unit document of a CODESYS machine
- **GIVEN** the approved version 1 of `R-300` on the machine `M10` with the platform `CodesysIec` and the encoding `Utf8`
- **WHEN** the `ExportClient` account `plc-m10` sends `GET /export/v2/machines/M10/recipes/R-300/approved/units/U1`
- **THEN** each parameter of the document holds `s7_address` with the value `null`
- **AND** the document passes the v2 schema file

### Requirement: UTF-16 code units of WSTRING and WCHAR values
The export documentation SHALL state that a transfer client writes each `WSTRING` or `WCHAR` character as its UTF-16 code units. A `WSTRING(n)` length SHALL count these code units. The export document SHALL keep the JSON text of the value (R03, R11, A2, ADR-0209).

#### Scenario: WSTRING value with the euro sign
- **GIVEN** the approved version 3 of `R-100` with the parameter `NOTE` of type `WSTRING(10)` and the value `"€"`
- **WHEN** an Operator runs `ExportVersion` through `GET /export/v1/recipes/R-100/approved`
- **THEN** the document holds the value `"€"` for `NOTE`
- **AND** the export documentation gives the code unit `16#20AC` for `€`

## MODIFIED Requirements

### Requirement: Latin-1 bytes of STRING and CHAR values
The export documentation SHALL state the bytes of a `STRING` or `CHAR` value. For a `Latin1` machine, a transfer client writes each character as one ISO 8859-1 byte with the scalar value of the character. For a `Utf8` machine, it writes each `STRING` character as its UTF-8 bytes. The export document SHALL keep the JSON text of the value (R03, R11, A2, ADR-0181, ADR-0202).

#### Scenario: STRING value with a Latin-1 letter
- **GIVEN** the approved version 3 of `R-100` on a machine with the encoding `Latin1` and the parameter `LABEL` of type `STRING(4)` with the value `"é"`
- **WHEN** an Operator runs `ExportVersion` through `GET /export/v1/recipes/R-100/approved`
- **THEN** the document holds the value `"é"` for `LABEL`
- **AND** the export documentation gives the byte `16#E9` for `é`

#### Scenario: STRING value of a Utf8 machine
- **GIVEN** the approved version 1 of `R-300` on the machine `M10` with the encoding `Utf8` and the parameter `LABEL` of type `STRING(4)` with the value `"é"`
- **WHEN** the `ExportClient` account `plc-m10` sends `GET /export/v2/machines/M10/recipes/R-300/approved/units/U1`
- **THEN** the document holds the value `"é"` for `LABEL`
- **AND** the export documentation gives the bytes `16#C3` and `16#A9` for `é`
