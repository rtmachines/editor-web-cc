## ADDED Requirements

### Requirement: STRING encoding of a machine
Each machine SHALL have one `STRING` encoding: `Latin1` or `Utf8`. A machine on `S7_300`, `S7_1200`, or `S7_1500` SHALL have `Latin1`. `CreateMachine` SHALL take the encoding and SHALL reject `Utf8` on an S7 platform with `422 E-PLT-053`. A machine from before this change SHALL have `Latin1` (R11, R15, R38, ADR-0202).

#### Scenario: CODESYS machine with UTF-8
- **GIVEN** no live machine `M10`
- **WHEN** a ProcessEngineer runs `CreateMachine` with the code `M10`, the platform `CodesysIec`, the encoding `Utf8`, and the main unit name `Infeed`
- **THEN** the system creates the machine `M10` with the encoding `Utf8`
- **AND** the entry `MachineCreated` holds the encoding `Utf8`

#### Scenario: UTF-8 on an S7 machine
- **GIVEN** no live machine `M7`
- **WHEN** a ProcessEngineer runs `CreateMachine` with the code `M7`, the platform `S7_1500`, the data block number `100`, and the encoding `Utf8`
- **THEN** the system rejects the command with `422 E-PLT-053` for the field `string_encoding`
- **AND** the system creates no machine

#### Scenario: Database refuses UTF-8 on an S7 machine
- **GIVEN** the role `recipe_app` and the live machine `M5` with the platform `S7_1500`
- **WHEN** an `UPDATE` statement sets the `string_encoding` of `M5` to `Utf8`
- **THEN** the database rejects the statement with SQLSTATE `23514`

#### Scenario: Database refuses an unknown encoding
- **GIVEN** the role `recipe_app` and the live machine `M9` with the platform `CodesysIec`
- **WHEN** an `UPDATE` statement sets the `string_encoding` of `M9` to `Ascii`
- **THEN** the database rejects the statement with SQLSTATE `23514`

### Requirement: Change of the STRING encoding
`SetMachineStringEncoding` SHALL set the `STRING` encoding of a live machine only while no recipe of the machine has a version row. Each row SHALL count: every status, a discarded draft, and a version of a deleted recipe. A version row SHALL give `409 E-PLT-054`. Each successful run SHALL write one audit entry `MachineStringEncodingSet` with the machine before and after the change (R11, R20, R31, ADR-0202).

#### Scenario: New encoding of a machine without a version
- **GIVEN** the live machine `M9` with the platform `CodesysIec`, the encoding `Latin1`, and no recipe
- **WHEN** a ProcessEngineer runs `SetMachineStringEncoding` on `M9` with the encoding `Utf8`
- **THEN** the machine `M9` has the encoding `Utf8`
- **AND** the audit log holds one entry `MachineStringEncodingSet` with `Latin1` before and `Utf8` after

#### Scenario: Machine with a draft version
- **GIVEN** the live machine `M9` with the recipe `R-300` and its draft version 1
- **WHEN** a ProcessEngineer runs `SetMachineStringEncoding` on `M9` with the encoding `Utf8`
- **THEN** the system rejects the command with `409 E-PLT-054`
- **AND** the encoding of `M9` and the audit log do not change

#### Scenario: Machine with only a discarded draft
- **GIVEN** the live machine `M9` with the recipe `R-300` and only its discarded draft version 1
- **WHEN** a ProcessEngineer runs `SetMachineStringEncoding` on `M9` with the encoding `Utf8`
- **THEN** the system rejects the command with `409 E-PLT-054`

#### Scenario: Machine with a version of a deleted recipe
- **GIVEN** the live machine `M9` with the deleted recipe `R-300` and its version 1
- **WHEN** a ProcessEngineer runs `SetMachineStringEncoding` on `M9` with the encoding `Utf8`
- **THEN** the system rejects the command with `409 E-PLT-054`

#### Scenario: UTF-8 on an S7 machine
- **GIVEN** the live machine `M5` with the platform `S7_1500` and no recipe
- **WHEN** a ProcessEngineer runs `SetMachineStringEncoding` on `M5` with the encoding `Utf8`
- **THEN** the system rejects the command with `422 E-PLT-053`
- **AND** the audit log does not change

#### Scenario: Encoding of an archived machine
- **GIVEN** the archived machine `M4` and no live machine with the code `M4`
- **WHEN** a ProcessEngineer sends `POST /machines/M4/string-encoding`
- **THEN** the system answers `404 E-WEB-005` at the lookup
- **AND** the encoding of `M4` does not change
- **AND** the audit log holds no success entry

#### Scenario: Machine archived after the lookup
- **GIVEN** the live machine `M9` with the platform `CodesysIec` and no recipe
- **AND** an Administrator archives `M9` after the lookup of `SetMachineStringEncoding` and before its machine lock
- **WHEN** the command takes the machine lock
- **THEN** the system rejects the command with `409 E-PLT-010` (ADR-0202 (d2))
- **AND** the encoding of `M9` does not change
- **AND** the audit log holds no entry `MachineStringEncodingSet`

#### Scenario: Failed audit insert
- **GIVEN** the audit insert of `SetMachineStringEncoding` fails
- **WHEN** a ProcessEngineer runs `SetMachineStringEncoding` on `M9` with the encoding `Utf8`
- **THEN** the encoding of `M9` does not change

### Requirement: No new version during a change of the STRING encoding
`SetMachineStringEncoding` SHALL lock the machine row before it reads the version rows. `CreateRecipe`, `RestoreRecipe`, and `CreateVersion` SHALL wait for that lock. Thus no version row SHALL appear between the check and the commit (R31, ADR-0015, ADR-0202).

#### Scenario: Encoding change and a new recipe at the same time
- **GIVEN** the live machine `M9` with no recipe
- **AND** `SetMachineStringEncoding` on `M9` holds the lock of the machine row
- **WHEN** a ProcessEngineer runs `CreateRecipe` on `M9`
- **THEN** `CreateRecipe` waits until `SetMachineStringEncoding` commits
- **AND** the system creates the recipe on `M9` with the encoding `Utf8`

#### Scenario: New recipe first
- **GIVEN** the live machine `M9` with no recipe
- **AND** `CreateRecipe` on `M9` holds its lock of the machine row
- **WHEN** a ProcessEngineer runs `SetMachineStringEncoding` on `M9` with the encoding `Utf8`
- **THEN** `SetMachineStringEncoding` waits until `CreateRecipe` commits
- **AND** the system rejects the command with `409 E-PLT-054`

### Requirement: Permission to set the STRING encoding
`SetMachineStringEncoding` SHALL check `MachineManage` as the first statement (R28, A9).

#### Scenario: Operator sets an encoding
- **GIVEN** a user with the role Operator
- **WHEN** the user sends `POST /machines/M9/string-encoding`
- **THEN** the system answers `403 E-WEB-002`
- **AND** the system writes one `security_event` row and does not change the machine

### Requirement: STRING encoding in read-only mode
In read-only mode, `SetMachineStringEncoding` SHALL return `503 E-WEB-006` and write nothing (A24).

#### Scenario: Set in read-only mode
- **GIVEN** the system is in read-only mode
- **WHEN** a ProcessEngineer sends `POST /machines/M9/string-encoding`
- **THEN** the system answers `503 E-WEB-006`
- **AND** the encoding of `M9` and the audit log do not change

### Requirement: Reserved names on CODESYS and TwinCAT
On `CodesysIec` and `TwinCat3`, a command SHALL reject a struct name of the 15 names of ADR-0203 (c), in any case. It SHALL reject a member name of the 184 keywords of ADR-0203 (d), in any case. It SHALL check the data that it writes and the stored struct revision that a written type text names directly. A stored struct stays readable. The preview of the parameter form SHALL run the same check. An unchanged stored type and `RebindTypeRevision` SHALL keep it (R11, R13, R38, ADR-0203).

#### Scenario: Compiler type name as a struct name on TwinCAT
- **GIVEN** the machine `M8` with the platform `TwinCat3`
- **WHEN** a ProcessEngineer runs `DefineUdt` for `VERSION` with the members `major` and `minor` of type `INT`
- **THEN** the system rejects the command with `E-PLT-013` and the reason "reserved name"
- **AND** the system creates no type

#### Scenario: Library type name in another case on CODESYS
- **GIVEN** the machine `M9` with the platform `CodesysIec`
- **WHEN** a ProcessEngineer runs `DefineUdt` for `st_amsaddr` with the members `port` and `net` of type `UINT`
- **THEN** the system rejects the command with `E-PLT-013` and the reason "reserved name"

#### Scenario: Keyword as a member name on CODESYS
- **GIVEN** the machine `M9` with the platform `CodesysIec`
- **WHEN** a ProcessEngineer runs `DefineUdt` for `ST_Coil` with the members `END_IF` and `S` of type `BOOL`
- **THEN** the system rejects the command with `E-PLT-013` and the reason "reserved member name"
- **AND** the system creates no type

#### Scenario: UNION as a member name on TwinCAT
- **GIVEN** the machine `M8` with the platform `TwinCat3`
- **WHEN** a ProcessEngineer runs `DefineUdt` for `ST_Pair` with the members `union` and `b` of type `INT`
- **THEN** the system rejects the command with `E-PLT-013` and the reason "reserved member name"

#### Scenario: Type name as a member name on CODESYS
- **GIVEN** the machine `M9` with the platform `CodesysIec`
- **WHEN** a ProcessEngineer runs `DefineUdt` for `ST_Flags` with the members `BIT` and `XWORD` of type `BOOL`
- **THEN** the system creates the struct `ST_Flags@1`

#### Scenario: Keyword as a member name on S7
- **GIVEN** the machine `M5` with the platform `S7_1500`
- **WHEN** a ProcessEngineer runs `DefineUdt` for `ST_Coil` with the members `END_IF` and `S` of type `BOOL`
- **THEN** the system creates the struct `ST_Coil@1`

#### Scenario: Type text with a stored reserved struct
- **GIVEN** the machine `M8` with the struct `VERSION@1`, stored before ADR-0203, and the draft version 3 of `R-100`
- **WHEN** a ProcessEngineer runs `AddParameter` on version 3 with the type `VERSION@1`
- **THEN** the system rejects the command with `E-PLT-013` and the reason "reserved name"

#### Scenario: Valid struct name with a stored keyword member
- **GIVEN** the machine `M9` with the platform `CodesysIec` and the struct `ST_Coil@1` with the members `END_IF` and `S`, stored before ADR-0203
- **AND** the draft version 3 of `R-100` on `M9`
- **WHEN** a ProcessEngineer runs `AddParameter` on version 3 with the type `ST_Coil@1`
- **THEN** the system rejects the command with `E-PLT-013` and the reason "reserved member name"
- **AND** the draft does not change

#### Scenario: Member type that names a stored reserved struct
- **GIVEN** the machine `M8` with the struct `VERSION@1`, stored before ADR-0203
- **WHEN** a ProcessEngineer runs `DefineUdt` for `ST_Build` with the members `ver` of type `VERSION@1` and `count` of type `INT`
- **THEN** the system rejects the command with `E-PLT-013` and the reason "reserved name"
- **AND** the system creates no type

#### Scenario: Stored reserved struct in the preview of the parameter form
- **GIVEN** the draft version 3 of `R-100` on `M8` with the parameter `VER` of type `VERSION@1` and the parameter `START` of type `INT`, both stored before ADR-0203
- **WHEN** a ProcessEngineer sends the preview `fields=1` of the add form, and of the edit form of `START`, with the type text `VERSION@1`
- **THEN** each form shows `E-PLT-013` with the reason "reserved name" (ADR-0203 (j1))
- **AND** the preview of the edit form of `VER` with the type text `VERSION@1` shows the value fields and no error

#### Scenario: Stored keyword member in the preview of the parameter form
- **GIVEN** the struct `ST_Coil@1` on `M9` with the members `END_IF` and `S`, stored before ADR-0203
- **AND** the draft version 3 of `R-100` on `M9` with the parameter `COIL` of type `ST_Coil@1` and the parameter `START` of type `INT`
- **WHEN** a ProcessEngineer sends the preview `fields=1` of the add form, and of the edit form of `START`, with the type text `ST_Coil@1`
- **THEN** each form shows `E-PLT-013` with the reason "reserved member name" (ADR-0203 (j1))
- **AND** the preview of the edit form of `COIL` with the type text `ST_Coil@1` shows the value fields and no error

#### Scenario: Unchanged stored type keeps a reserved struct
- **GIVEN** the draft version 3 of `R-100` on `M8` with the parameter `VER` of type `VERSION@1`, stored before ADR-0203
- **WHEN** a ProcessEngineer runs `UpdateParameterDefinition` on `VER` with a new description and the same type text
- **THEN** the system stores the new description
- **AND** the parameter `VER` keeps the type `VERSION@1`

#### Scenario: Rebind keeps a reserved struct
- **GIVEN** the draft version 3 of `R-100` on `M8` with the parameter `VER` of type `VERSION@1`, stored before ADR-0203
- **WHEN** a ProcessEngineer runs `RebindTypeRevision` on version 3
- **THEN** the system does not reject the command with `E-PLT-013`
- **AND** the parameter `VER` keeps the type `VERSION@1`

#### Scenario: Stored reserved struct stays approvable
- **GIVEN** the draft version 3 of `R-100` on `M8` with the parameter `VER` of type `VERSION@1`, stored before ADR-0203
- **WHEN** a ProcessEngineer runs `ApproveVersion` on version 3
- **THEN** the system approves version 3

#### Scenario: Proposals leave out a stored reserved struct
- **GIVEN** the machine `M8` with the structs `VERSION@1`, stored before ADR-0203, and `ST_Zone@1`
- **WHEN** a ProcessEngineer opens the parameter form of a draft of `M8`
- **THEN** the type proposals hold `ST_Zone@1`
- **AND** the type proposals do not hold `VERSION@1`

### Requirement: Two members of a struct on CODESYS
On `CodesysIec`, the system SHALL create a struct only with 2 to 1024 members. One member SHALL give `E-PLT-013` with the reason "too few members". No member SHALL still give `E-PLT-016` on every platform. A command SHALL also apply the rule to the stored struct revision that a written type text names directly. The preview of the parameter form SHALL run the same check. A stored struct with one member SHALL stay readable, and `RebindTypeRevision` SHALL keep it (R11, R13, R38, ADR-0204).

#### Scenario: One member on CODESYS
- **GIVEN** the machine `M9` with the platform `CodesysIec`
- **WHEN** a ProcessEngineer runs `DefineUdt` for `ST_One` with the member `value` of type `INT`
- **THEN** the system rejects the command with `E-PLT-013` and the reason "too few members"
- **AND** the system creates no type

#### Scenario: One member on TwinCAT
- **GIVEN** the machine `M8` with the platform `TwinCat3`
- **WHEN** a ProcessEngineer runs `DefineUdt` for `ST_One` with the member `value` of type `INT`
- **THEN** the system creates the struct `ST_One@1`

#### Scenario: No member on CODESYS
- **GIVEN** the machine `M9` with the platform `CodesysIec`
- **WHEN** a ProcessEngineer runs `DefineUdt` for `ST_Empty` with no member
- **THEN** the system rejects the command with `E-PLT-016`

#### Scenario: Type text with a stored one-member struct
- **GIVEN** the machine `M9` with the struct `ST_One@1` with one member, stored before ADR-0204, and the draft version 3 of `R-100`
- **WHEN** a ProcessEngineer runs `AddParameter` on version 3 with the type `ARRAY[1..4] OF ST_One@1`
- **THEN** the system rejects the command with `E-PLT-013` and the reason "too few members"

#### Scenario: Member type that names a stored one-member struct
- **GIVEN** the machine `M9` with the struct `ST_One@1` with one member, stored before ADR-0204
- **WHEN** a ProcessEngineer runs `DefineUdt` for `ST_Pair` with the members `first` of type `ST_One@1` and `second` of type `INT`
- **THEN** the system rejects the command with `E-PLT-013` and the reason "too few members"
- **AND** the system creates no type

#### Scenario: Stored one-member struct in the preview of the parameter form
- **GIVEN** the struct `ST_One@1` with one member on `M9`, stored before ADR-0204
- **AND** the draft version 3 of `R-100` on `M9` with the parameter `ONE` of type `ST_One@1` and the parameter `START` of type `INT`
- **WHEN** a ProcessEngineer sends the preview `fields=1` of the add form, and of the edit form of `START`, with the type text `ST_One@1`
- **THEN** each form shows `E-PLT-013` with the reason "too few members" (ADR-0204 (d1))
- **AND** the preview of the edit form of `ONE` with the type text `ST_One@1` shows the value fields and no error

#### Scenario: Stored one-member struct stays approvable
- **GIVEN** the draft version 3 of `R-100` on `M9` with the parameter `ONE` of type `ST_One@1`, stored before ADR-0204
- **WHEN** a ProcessEngineer runs `ApproveVersion` on version 3
- **THEN** the system approves version 3

#### Scenario: Proposals leave out a stored one-member struct
- **GIVEN** the machine `M9` with the structs `ST_One@1` with one member, stored before ADR-0204, and `ST_Zone@1` with two members
- **WHEN** a ProcessEngineer opens the parameter form of a draft of `M9`
- **THEN** the type proposals hold `ST_Zone@1`
- **AND** the type proposals do not hold `ST_One@1`

#### Scenario: Proposals leave out a name whose latest revision is refused
- **GIVEN** the machine `M9` with the struct `ST_X@1` with two members and the struct `ST_X@2` with one member, both stored before ADR-0204
- **WHEN** a ProcessEngineer opens the parameter form of a draft of `M9`
- **THEN** the type proposals hold no revision of `ST_X`

### Requirement: Vendor aliases of type names
A written type text SHALL accept the aliases `TOD`, `DT`, `LTIME_OF_DAY`, and `LDATE_AND_TIME` of a primitive name on every platform. The system SHALL store, show, and export the catalog name. `DT@1` SHALL stay a struct reference text. A written `DT@1`, `TOD@1`, `LTIME_OF_DAY@1`, or `LDATE_AND_TIME@1` SHALL give `E-PLT-013` with the reason "reserved name". `T`, `D`, and `LD` SHALL stay grammar errors (R11, R12, R38, ADR-0206).

#### Scenario: TOD as a parameter type
- **GIVEN** the draft version 3 of `R-100` on `M9`
- **WHEN** a ProcessEngineer runs `AddParameter` on version 3 with the tag `START` and the type `TOD`
- **THEN** the system adds the parameter `START` with the type `TIME_OF_DAY`
- **AND** the export document of a later approved version holds `TIME_OF_DAY` for `START`

#### Scenario: Alias as the element of an array
- **GIVEN** the draft version 3 of `R-100` on `M5`
- **WHEN** a ProcessEngineer runs `AddParameter` on version 3 with the tag `STAMPS` and the type `ARRAY[1..2] OF DT`
- **THEN** the system adds the parameter `STAMPS` with the type `ARRAY[1..2] OF DATE_AND_TIME`

#### Scenario: Alias as a member type
- **GIVEN** the machine `M8` with the platform `TwinCat3`
- **WHEN** a ProcessEngineer runs `DefineUdt` for `ST_Shift` with the members `start` of type `LTIME_OF_DAY` and `stamp` of type `LDATE_AND_TIME`
- **THEN** the system creates the struct `ST_Shift@1` with the member types `LTOD` and `LDT`

#### Scenario: Alias with a revision number
- **GIVEN** the draft version 3 of `R-100` on `M9`
- **WHEN** a ProcessEngineer runs `AddParameter` on version 3 with the type `DT@1`
- **THEN** the system rejects the command with `E-PLT-013` and the reason "reserved name" (ADR-0206 (d1))
- **AND** the draft does not change

#### Scenario: Alias in the parameter form
- **GIVEN** the draft version 3 of `R-100` on `M9` with the parameter `START` of type `INT`
- **WHEN** a ProcessEngineer sends the edit form of `START` with the type text `TOD`
- **THEN** the system stores the type `TIME_OF_DAY` for `START`

#### Scenario: Literal prefix as a type name
- **GIVEN** the draft version 3 of `R-100` on `M9`
- **WHEN** a ProcessEngineer runs `AddParameter` on version 3 with the type `T`
- **THEN** the system rejects the command with `E-PLT-013` and the reason "the text does not match the type grammar"

### Requirement: Reserved struct reference in the preview of the parameter form
The preview of the parameter form SHALL check the type text of the add form and a changed type text of the edit form. A parsed struct reference with a reserved name of ADR-0143 (c), or of ADR-0186 (a) on S7, SHALL give `E-PLT-013` with the reason "reserved name". An unchanged type text SHALL keep its stored struct. Its parsed canonical text equals the stored type text of the edited parameter (R11, R12, R38, ADR-0206).

#### Scenario: Alias with a revision number in the preview of the parameter form
- **GIVEN** the draft version 3 of `R-100` on `M9`, and `M9` has no struct `DT@1`
- **WHEN** a ProcessEngineer sends the preview `fields=1` of the add form with the type text `DT@1`
- **THEN** the form shows `E-PLT-013` with the reason "reserved name" and not the reason "unknown struct" (ADR-0206 (d3))

#### Scenario: Alias with a revision number of a stored struct in the preview of the add form
- **GIVEN** the draft version 3 of `R-100` on `M9`, and `M9` holds the struct `DT@1`, stored before ADR-0143
- **WHEN** a ProcessEngineer sends the preview `fields=1` of the add form with the type text `DT@1`
- **THEN** the form shows `E-PLT-013` with the reason "reserved name" (ADR-0206 (d3))

#### Scenario: Changed type text with an alias and a revision number in the preview of the edit form
- **GIVEN** the draft version 3 of `R-100` on `M9` with the parameter `START` of type `INT`, and `M9` holds the struct `DT@1`, stored before ADR-0143
- **WHEN** a ProcessEngineer sends the preview `fields=1` of the edit form of `START` with the type text `DT@1`
- **THEN** the form shows `E-PLT-013` with the reason "reserved name" (ADR-0206 (d3))

#### Scenario: Unchanged type text with an alias and a revision number in the preview of the edit form
- **GIVEN** the draft version 3 of `R-100` on `M9` with the parameter `STAMP` of type `DT@1`, and `M9` holds the struct `DT@1`, stored before ADR-0143
- **WHEN** a ProcessEngineer sends the preview `fields=1` of the edit form of `STAMP` with the type text `DT@1`
- **THEN** the form shows the value fields of `DT@1` and no error (ADR-0143 (i2), ADR-0206 (d3))

#### Scenario: Siemens reserved name in the preview of the add form
- **GIVEN** the draft version 3 of `R-100` on `M5` with the platform `S7_1500`, and a draft of `M8` with the platform `TwinCat3`
- **WHEN** a ProcessEngineer sends the preview `fields=1` of the add form of each draft with the type text `BEGIN@9`
- **THEN** the form of `M5` shows `E-PLT-013` with the reason "reserved name" (ADR-0186 (a), ADR-0206 (d3))
- **AND** the form of `M8` shows `E-PLT-013` with the reason "unknown struct"

#### Scenario: Canonical text of an unchanged type in the preview of the edit form
- **GIVEN** the struct `DT@1` on `M9`, stored before ADR-0143
- **AND** the draft version 3 of `R-100` on `M9` with the parameter `STAMPS` of type `ARRAY[0..7] OF DT@1`
- **WHEN** a ProcessEngineer sends the preview `fields=1` of the edit form of `STAMPS` with the type text `ARRAY[-0..007] OF DT@1`
- **THEN** the form shows the value fields of `ARRAY[0..7] OF DT@1` and no error, because the parsed text writes `ARRAY[0..7] OF DT@1` (ADR-0206 (d3))

#### Scenario: Primitive name in exact case in the preview of the add form
- **GIVEN** the draft version 3 of `R-100` on `M9`
- **WHEN** a ProcessEngineer sends the preview `fields=1` of the add form with the type text `INT@1`
- **THEN** the form shows `E-PLT-013` with the reason "unknown struct" of the request parser (ADR-0206 (d3))

### Requirement: Type texts outside the type grammar
The system SHALL reject an open array, a pointer, and a reference as a type text with `E-PLT-013` on every platform (R11, R12, R38, ADR-0209).

#### Scenario: Open array
- **GIVEN** the draft version 3 of `R-100` on `M9`
- **WHEN** a ProcessEngineer runs `AddParameter` on version 3 with the type `ARRAY[*] OF INT`
- **THEN** the system rejects the command with `E-PLT-013`

#### Scenario: Pointer and reference
- **GIVEN** the draft version 3 of `R-100` on `M8`
- **WHEN** a ProcessEngineer runs `AddParameter` on version 3 with the type `POINTER TO INT` or `REFERENCE TO INT`
- **THEN** the system rejects each command with `E-PLT-013`

### Requirement: WSTRING length of zero
The system SHALL reject the type text `WSTRING(0)` with `E-PLT-013` on every platform with `WSTRING` (R11, R12, R38).

#### Scenario: WSTRING of length zero on TwinCAT
- **GIVEN** the draft version 3 of `R-100` on `M8`
- **WHEN** a ProcessEngineer runs `AddParameter` on version 3 with the type `WSTRING(0)`
- **THEN** the system rejects the command with `E-PLT-013`
