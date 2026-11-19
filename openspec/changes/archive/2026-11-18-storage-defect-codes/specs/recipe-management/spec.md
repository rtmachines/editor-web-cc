## ADDED Requirements

### Requirement: Draft commands on a storage defect
A draft command can read a stored part that it does not write. When that part fails the earlier rule set of ADR-0120, the command SHALL give `500 E-DB-008` and write nothing. The stored parts are the value, the limits, the type, and the S7 address of a parameter. The parameter rows and the type rows of the draft are stored parts too. A stored parameter type that the platform does not hold SHALL keep `422 E-PLT-013` (R14, R35, R36).

#### Scenario: Stored limit after the earlier range
- **GIVEN** the draft version 3 of `R-100` on the platform `S7_1500` with the stored `DATE` limit `max` `72900` in the parameter `DUE`
- **WHEN** a ProcessEngineer runs `SetParameterValue` on `DUE` with the value `D#2100-01-01`
- **THEN** the system returns `500 E-DB-008`
- **AND** the value of `DUE` does not change

#### Scenario: Stored value after the earlier range
- **GIVEN** the draft version 3 of `R-100` on the platform `S7_1500` with the stored `DATE` value `72900` in the parameter `DUE`
- **WHEN** a ProcessEngineer runs `ReplaceLimits` on `DUE` without limits
- **THEN** the system returns `500 E-DB-008`
- **AND** the limits of `DUE` do not change

#### Scenario: Stored address of another parameter with the wrong width
- **GIVEN** the draft version 3 of `R-100` on the platform `S7_300` with the stored parameter `SPEED` of type `DINT` at `DB1.DBW0`
- **WHEN** a ProcessEngineer runs `SetParameterValue` on the parameter `FEED` with a valid value
- **THEN** the system returns `500 E-DB-008`

#### Scenario: Two stored parameters with one tag
- **GIVEN** the draft version 3 of `R-100` with two stored parameters that have the tag `SPEED`
- **WHEN** a ProcessEngineer runs `UpsertTranslation` on one of them
- **THEN** the system returns `500 E-DB-008`

#### Scenario: Stored type with a cycle
- **GIVEN** the draft version 3 of `R-100` with the parameter `ZONE` of a stored struct type that contains itself
- **WHEN** a ProcessEngineer runs `SetParameterValue` on `ZONE`
- **THEN** the system returns `500 E-DB-008`

#### Scenario: Stored array type with a cycle of array rows
- **GIVEN** the draft version 3 of `R-100` with the parameter `GRID` of a stored array type whose element rows form a cycle without a struct
- **WHEN** a ProcessEngineer runs `ReplaceLimits` on `GRID`
- **THEN** the system returns `500 E-DB-008`

#### Scenario: Stored member type that the platform does not hold
- **GIVEN** the draft version 3 of `R-100` with the parameter `ZONE` of a stored struct type with a member type that the platform does not hold
- **WHEN** a ProcessEngineer runs `SetParameterValue` on `ZONE`
- **THEN** the system rejects the command with `E-PLT-013`

#### Scenario: Two stored type rows with one identifier
- **GIVEN** the draft version 3 of `R-100` on the machine `M12`, and the type rows of `M12` hold two kept rows with one identifier
- **WHEN** a ProcessEngineer runs `SetParameterValue` on the parameter `RATE`
- **THEN** the system returns `500 E-DB-008`

#### Scenario: Rebind of a stored value of the wrong form
- **GIVEN** the draft version 3 of `R-100` with the parameter `ZONE` of type `ST_Zone@1` and a stored value that is no JSON object
- **WHEN** a ProcessEngineer runs `RebindTypeRevision` from `ST_Zone@1` to `ST_Zone@2`
- **THEN** the system returns `500 E-DB-008`
- **AND** the answer holds no list of entries

#### Scenario: Rebind of a stored limit on a path that the old type does not have
- **GIVEN** the draft version 3 of `R-100` with the parameter `ZONE` of type `ST_Zone@1` and a stored limit on the leaf path `.ghost`, which `ST_Zone@1` does not have
- **WHEN** a ProcessEngineer runs `RebindTypeRevision` from `ST_Zone@1` to `ST_Zone@2`
- **THEN** the system returns `500 E-DB-008`
- **AND** the answer holds no entry `E-RCP-029`

### Requirement: Approval on a storage defect
`ApproveVersion` and `GetApprovalPreview` SHALL give `500 E-DB-008` when two stored parameters of the draft share a tag, a key, or an identifier. The system SHALL write nothing (R35, R36).

#### Scenario: Approval of a draft with two stored parameters with one tag
- **GIVEN** the draft version 3 of `R-100` with two stored parameters that have the tag `SPEED`
- **WHEN** an Administrator runs `ApproveVersion` on version 3
- **THEN** the system returns `500 E-DB-008`
- **AND** version 3 stays a draft

#### Scenario: Approval preview of a draft with two stored parameters with one identifier
- **GIVEN** the draft version 3 of `R-100` with two stored parameters that have one identifier
- **WHEN** an Administrator opens the approval preview of version 3
- **THEN** the system returns `500 E-DB-008`

### Requirement: Request part next to a storage defect
A draft command SHALL give the code of the broken rule for the part of the request, as ADR-0120 says (R38).

#### Scenario: New limit that breaks the type range
- **GIVEN** the draft version 3 of `R-100` with the stored `INT` value `5` in the parameter `RATE`
- **WHEN** a ProcessEngineer runs `ReplaceLimits` on `RATE` with the `max` value `40000`
- **THEN** the system rejects the command with `E-RCP-006`

#### Scenario: Request address with the wrong width
- **GIVEN** the draft version 3 of `R-100` on the platform `S7_300`
- **WHEN** a ProcessEngineer runs `AddParameter` with the tag `FEED`, the type `DINT`, and the address `DB2.DBW0`
- **THEN** the system rejects the command with `E-RCP-054`

### Requirement: Log line of a storage defect in a command
The system SHALL write one `ERROR` line for a storage defect of a draft command. The line SHALL hold the code `E-DB-008`, the request id, and the key of the parameter when the defect has one (R35).

#### Scenario: Log line of a stored address with the wrong width
- **GIVEN** the draft version 3 of `R-100` on the platform `S7_300` with the stored parameter `SPEED` of type `DINT` at `DB1.DBW0`
- **WHEN** a ProcessEngineer runs `SetParameterValue` on the parameter `FEED` with a valid value
- **THEN** the system writes one `ERROR` line with the code `E-DB-008`, the request id, and the key of `SPEED`

### Requirement: New type for a parameter whose type does not resolve
`UpdateParameterDefinition` SHALL replace the type of a parameter whose stored type does not resolve. When the old type has no tree, `before_state.value` of the audit entry SHALL hold the whole stored value under the key `""` (R29, R35).

#### Scenario: New type for a parameter with a type cycle
- **GIVEN** the draft version 3 of `R-100` with the parameter `ZONE` of a stored struct type that contains itself
- **WHEN** a ProcessEngineer runs `UpdateParameterDefinition` on `ZONE` with the type `INT` and the value `0`
- **THEN** the system stores `ZONE` with the type `INT` and the value `0`
- **AND** `before_state.value` of the audit entry `ParameterDefinitionUpdated` holds the old stored value under the key `""`

### Requirement: Parameter form on a stored type that does not resolve
For every purpose except `UpdateParameterDefinition`, the parameter form query SHALL give `500 E-DB-008` in three cases. The resolution of the stored parameter type fails, the stored type has no type text, or two kept type rows share one identifier. A stored member type that the platform does not hold SHALL keep `422 E-PLT-013`. The routes of `SetParameterValue`, `ReplaceLimits`, `RemoveParameter`, `UpsertTranslation`, and `RemoveTranslation` run this query first (R35, R36).

#### Scenario: Value form of a parameter with a type cycle
- **GIVEN** the draft version 3 of `R-100` with the parameter `ZONE` of a stored struct type that contains itself
- **WHEN** a ProcessEngineer sends the value form of `ZONE`
- **THEN** the system returns `500 E-DB-008`

#### Scenario: Removal of a parameter with a type cycle
- **GIVEN** the draft version 3 of `R-100` with the parameter `ZONE` of a stored struct type that contains itself
- **WHEN** a ProcessEngineer sends the removal form of `ZONE`
- **THEN** the system returns `500 E-DB-008`
- **AND** `ZONE` stays in the draft

#### Scenario: Value form of an array parameter whose element row is missing
- **GIVEN** the draft version 3 of `R-100` with the parameter `GRID` of a stored array type whose element type row the platform does not hold
- **AND** the stored type of `GRID` therefore has no type text
- **WHEN** a ProcessEngineer sends the value form of `GRID`
- **THEN** the system returns `500 E-DB-008`

#### Scenario: Value form of a parameter with a member type that the platform does not hold
- **GIVEN** the draft version 3 of `R-100` with the parameter `ZONE` of a stored struct type with a member type that the platform does not hold
- **WHEN** a ProcessEngineer sends the value form of `ZONE`
- **THEN** the system returns `422 E-PLT-013`

### Requirement: Edit form on a stored type that does not resolve
For the purpose `UpdateParameterDefinition`, the parameter form query SHALL return the form for a stored type that does not resolve. The form SHALL hold no rule faults of the stored parameter. Two kept type rows with one identifier SHALL still give `500 E-DB-008` (R35, R36).

#### Scenario: Edit form of a parameter with a type cycle
- **GIVEN** the draft version 3 of `R-100` with the parameter `ZONE` of a stored struct type that contains itself
- **WHEN** a ProcessEngineer runs the parameter form query for `UpdateParameterDefinition` on `ZONE` with the type `INT`
- **THEN** the query returns the form with the preview of `INT`
- **AND** the form holds no rule faults of `ZONE`

#### Scenario: Edit form of a parameter whose type row is missing
- **GIVEN** the draft version 3 of `R-100` with the parameter `ZONE` whose stored type row the platform does not hold
- **WHEN** a ProcessEngineer runs the parameter form query for `UpdateParameterDefinition` on `ZONE` with the type `INT`
- **THEN** the query returns the form with the preview of `INT`
- **AND** the form holds no type text of the stored type

### Requirement: Platform, width, and alignment rules on every parameter
`AddParameter`, `UpdateParameterDefinition`, `SetParameterValue`, `ReplaceLimits`, and `RebindTypeRevision` SHALL apply the platform rule to every parameter of the draft. They SHALL apply the width rule and the alignment rule to every parameter whose type resolves. A fault of these three rules in a stored address that the request does not supply SHALL give `500 E-DB-008` (ADR-0195) (R12, R14, R38).

#### Scenario: New parameter next to a stored address with the wrong width
- **GIVEN** the draft version 3 of `R-100` on the platform `S7_300` with the stored parameter `SPEED` of type `DINT` at `DB1.DBW0`
- **WHEN** a ProcessEngineer runs `AddParameter` with the tag `FEED`, the type `INT`, and the address `DB2.DBW0`
- **THEN** the system returns `500 E-DB-008`
- **AND** the draft holds no parameter `FEED`

## MODIFIED Requirements

### Requirement: Draft commands on stored data that breaks a newer rule
A draft command SHALL check the value or the limit that it writes with the current rules (R12, R14, R38). It SHALL check a stored value or limit that it reads and does not write with the earlier rule set of ADR-0120. The process-limit check between the two parts SHALL still run. A draft command SHALL apply the data block rule, the end rule, and the standard-access rule only to the parameters that it writes (ADR-0116 (e1)).

#### Scenario: Command on another parameter
- **GIVEN** the draft version 3 of `R-100` on the platform `S7_1500` with the stored `DATE` value `72700` in the parameter `DUE`
- **WHEN** a ProcessEngineer runs `SetParameterValue` on the parameter `RATE` of type `REAL` with the JSON number `12.5`
- **THEN** the system stores the value `12.5`

#### Scenario: Fix of the value before the limit
- **GIVEN** the draft version 3 of `R-100` on the platform `S7_1500` with the stored value `72700` and the stored limit `max` `72700` in `DUE`
- **WHEN** a ProcessEngineer runs `SetParameterValue` on `DUE` with the value `D#2168-12-31`
- **THEN** the system stores the value `72683`

#### Scenario: Fix of the limit after the value
- **GIVEN** the draft version 3 of `R-100` on the platform `S7_1500` with the stored value `72683` and the stored limit `max` `72700` in `DUE`
- **WHEN** a ProcessEngineer runs `ReplaceLimits` on `DUE` with the `max` value `D#2168-12-31`
- **THEN** the system stores the limit `max` `72683`

#### Scenario: Fix of the limit before the value
- **GIVEN** the draft version 3 of `R-100` on the platform `S7_1500` with the stored value `72700` and the stored limit `max` `72700` in `DUE`
- **WHEN** a ProcessEngineer runs `ReplaceLimits` on `DUE` without a `max` value
- **THEN** the system stores the limits of `DUE` without `max`

#### Scenario: New value that breaks the current range
- **GIVEN** the draft version 3 of `R-100` on the platform `S7_1500` with the stored limit `max` `72700` in `DUE`
- **WHEN** a ProcessEngineer runs `SetParameterValue` on `DUE` with the value `D#2169-01-17`
- **THEN** the system rejects the command with `E-RCP-005`

#### Scenario: New value outside an affected stored limit
- **GIVEN** the draft version 3 of `R-100` with the stored `REAL` limit `max` `100.00001` in the parameter `RATE`
- **WHEN** a ProcessEngineer runs `SetParameterValue` on `RATE` with the JSON number `200`
- **THEN** the system rejects the command with `E-RCP-004`

#### Scenario: New parameter next to an affected address
- **GIVEN** the draft version 3 of `R-100` on the platform `S7_300` with the stored parameter `SPEED` of type `DINT` at `DB1.DBD65534`
- **WHEN** a ProcessEngineer runs `AddParameter` with the tag `FEED`, the type `INT`, and the address `DB2.DBW0`
- **THEN** the system adds the parameter

### Requirement: Every fault of a value and of its limits
`AddParameter`, `UpdateParameterDefinition`, `SetParameterValue`, and `ReplaceLimits` SHALL check every leaf and every limit row. They SHALL return every fault of one parameter in one list. The faults of the leaves SHALL come first in preorder, and the faults of the limit rows SHALL follow in their order. A stored part that the command only reads SHALL instead give one `500 E-DB-008` and no list when it fails the earlier rule set (ADR-0195) (R36, A36).

#### Scenario: Two leaves out of the limit
- **GIVEN** the parameter `SPEED_TABLE` of type `ARRAY[1..8] OF INT` with the limit 0 to 3000 on `[*]`
- **WHEN** a ProcessEngineer runs `SetParameterValue` with 3500 in `[3]` and 4000 in `[6]`
- **THEN** the system rejects the command with a list of two `E-RCP-004` entries for `[3]` and `[6]`
- **AND** the value of `SPEED_TABLE` does not change

### Requirement: Rebind needs values and changes nothing on failure
The system SHALL return one list entry for each parameter that fails a rebind. The entry SHALL hold `E-RCP-028` for a new member without a value or `E-RCP-029` for a limit that the new revision drops. The system SHALL change no parameter when any entry exists. A stored defect of a value, of a limit, or of the I7 check SHALL give `500 E-DB-008` and no list, also when parameters have entries. An address fault comes after a plan without entries (ADR-0195) (A29).

#### Scenario: New member without a value
- **GIVEN** the draft version 3 of `R-100` with the parameters `SPEED` and `TEMP` of type `ST_Zone@2`, and the revision 3 adds the member `temp_max`
- **WHEN** a ProcessEngineer runs `RebindTypeRevision` from `ST_Zone@2` to `ST_Zone@3` without a value for `temp_max`
- **THEN** the system rejects the command with `E-RCP-028`
- **AND** the list holds one entry for each of `SPEED` and `TEMP`
- **AND** the type of `SPEED` stays `ST_Zone@2`

#### Scenario: Missing value next to a stored defect
- **GIVEN** the draft version 3 of `R-100` with the parameters `SPEED` and `TEMP` of type `ST_Zone@2`, and the revision 3 adds the member `temp_max`
- **AND** the stored value of `TEMP` is above its stored limit
- **WHEN** a ProcessEngineer runs `RebindTypeRevision` from `ST_Zone@2` to `ST_Zone@3` without a value for `temp_max`
- **THEN** the system returns `500 E-DB-008`
- **AND** the answer holds no list of entries
- **AND** the type of `SPEED` stays `ST_Zone@2`
