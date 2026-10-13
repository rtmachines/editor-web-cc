# Spec Delta: recipe-management

## ADDED Requirements

### Requirement: JSON form of a leaf value
The system SHALL accept a leaf value only in the JSON form of its leaf class. A `BOOL` leaf SHALL accept only `true` and `false`. A `CHAR` or `WCHAR` leaf SHALL accept one Unicode scalar inside the range of its type. A `STRING` leaf SHALL accept only Latin-1 characters, and the number of characters SHALL fit the type length. A `WSTRING` value SHALL fit the type length in UTF-16 units (R11, R12, R38).

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

### Requirement: Value inside the type range
The system SHALL check each leaf against the range of its data type before it checks the process limit. A value that the data type cannot hold SHALL give `E-RCP-005`. A number with a fraction is outside the range of an integer type (R11, R12, R38, I7).

#### Scenario: INT value above the type range
- **GIVEN** the parameter `SPEED` of type `INT` in the draft version 3 of `R-100`
- **WHEN** a ProcessEngineer runs `SetParameterValue` on `SPEED` with the value `40000`
- **THEN** the system rejects the command with `E-RCP-005`

#### Scenario: Negative value of an unsigned type
- **GIVEN** the parameter `COUNT` of type `USINT` in the draft version 3 of `R-100`
- **WHEN** a ProcessEngineer runs `SetParameterValue` on `COUNT` with the value `-1`
- **THEN** the system rejects the command with `E-RCP-005`

#### Scenario: Fraction for an integer type
- **GIVEN** the parameter `SPEED` of type `INT` in the draft version 3 of `R-100`
- **WHEN** a ProcessEngineer runs `SetParameterValue` on `SPEED` with the value `1.5`
- **THEN** the system rejects the command with `E-RCP-005`

### Requirement: Change note of a version
Each version SHALL have a change note from the user, also version 1. A change note SHALL have 1 to 4 000 characters and no control character except a line end. A line end SHALL be LF or CRLF. A change note of spaces only SHALL count as empty. The system SHALL reject any other change note with `E-WEB-008` (R17, R38).

#### Scenario: Change note of spaces only
- **GIVEN** the approved version 2 of `R-100` and no draft
- **WHEN** a ProcessEngineer runs `CreateVersion` on `R-100` with a change note of three spaces
- **THEN** the system rejects the command with `E-WEB-008`

#### Scenario: Empty change note
- **GIVEN** the approved version 2 of `R-100` and no draft
- **WHEN** a ProcessEngineer runs `CreateVersion` on `R-100` with an empty change note
- **THEN** the system rejects the command with `E-WEB-008`
- **AND** the system creates no version

#### Scenario: Change note with two lines from a browser form
- **GIVEN** the approved version 2 of `R-100` and no draft
- **WHEN** a ProcessEngineer runs `CreateVersion` on `R-100` with a change note of two lines separated by CRLF
- **THEN** the system creates draft version 3 with that change note

### Requirement: Base of a new version
`CreateVersion` SHALL accept any version of the recipe as its base, also a discarded draft. Without a base, the new draft SHALL have no parameters (R17, A28).

#### Scenario: Discarded draft as the base
- **GIVEN** the recipe `R-100` with the approved version 2 and the discarded draft version 3 with the parameter `TEMP`
- **WHEN** a ProcessEngineer runs `CreateVersion` on `R-100` based on version 3
- **THEN** the system creates draft version 4 with the parameter `TEMP`

#### Scenario: No base
- **GIVEN** the recipe `R-100` with the approved version 2 and no draft
- **WHEN** a ProcessEngineer runs `CreateVersion` on `R-100` without a base
- **THEN** the system creates draft version 3 without parameters

### Requirement: Recipe name
A recipe SHALL have a name of 1 to 200 characters with no control character. A name of spaces only SHALL count as empty. The system SHALL reject any other name with `E-WEB-008` (R16, R38).

#### Scenario: Name of 201 characters
- **GIVEN** the recipe `R-100` with the name `Line A`
- **WHEN** a ProcessEngineer runs `RenameRecipe` on `R-100` with a name of 201 characters
- **THEN** the system rejects the command with `E-WEB-008`
- **AND** the name of `R-100` stays `Line A`

### Requirement: Sort order of a parameter
Each parameter SHALL have a sort order from 0 to 100 000. Two parameters of one unit SHALL be able to share a sort order. The system SHALL reject a sort order outside the range with `E-WEB-008` (R38, R39).

#### Scenario: Sort order above the range
- **GIVEN** the draft version 3 of `R-100`
- **WHEN** a ProcessEngineer runs `AddParameter` on version 3 with the sort order 100 001
- **THEN** the system rejects the command with `E-WEB-008`
- **AND** the system creates no parameter

#### Scenario: Two parameters with one sort order
- **GIVEN** the parameter `SPEED` with the sort order 5 on the unit `U1` in the draft version 3 of `R-100`
- **WHEN** a ProcessEngineer runs `AddParameter` on version 3 with the tag `TEMP`, the unit `U1`, and the sort order 5
- **THEN** the system adds the parameter `TEMP`

### Requirement: One limit for each leaf path
A parameter SHALL have at most one process limit for each leaf path. The system SHALL reject a limit list with two rows for one leaf path with `E-WEB-008`. The system SHALL also reject a `[*]` row and an index row for the same array level with `E-WEB-008` (R09, A36).

#### Scenario: Two rows for one leaf path
- **GIVEN** the parameter `ZONE_1` of the struct `ST_Zone` with the member `setpoint` of type `INT`
- **WHEN** a ProcessEngineer runs `ReplaceLimits` on `ZONE_1` with two rows for the leaf path `.setpoint`
- **THEN** the system rejects the command with `E-WEB-008`
- **AND** the limits of `ZONE_1` do not change

#### Scenario: All elements and one element of one array level
- **GIVEN** the parameter `ZONE_1` of the struct `ST_Zone` with the member `ramps` of type `ARRAY[1..3] OF INT`
- **WHEN** a ProcessEngineer runs `ReplaceLimits` on `ZONE_1` with rows for the leaf paths `.ramps[*]` and `.ramps[2]`
- **THEN** the system rejects the command with `E-WEB-008`

### Requirement: Outdated parameter
A parameter SHALL be outdated when a type in its resolved type tree has a newer revision. The tree SHALL hold the parameter type, every member type, and every element type at any depth. A successful `CreateVersion` SHALL list the outdated parameters of the new draft (R19, A29).

#### Scenario: Array of an old struct revision
- **GIVEN** the machine `M12` with the struct `ST_Zone` in the revisions 2 and 3
- **AND** the parameter `ZONES` of type `ARRAY[1..4] OF ST_Zone@2` in the draft version 3 of `R-100`
- **WHEN** a ProcessEngineer runs `ApproveVersion` on version 3 without `allow_outdated`
- **THEN** the system rejects the command with `E-RCP-024`

#### Scenario: Member of an old struct revision
- **GIVEN** the struct `ST_Line@1` of `M12` with the member `zone` of type `ST_Zone@2`, and the newer revision `ST_Zone@3`
- **AND** the parameter `LINE` of type `ST_Line@1` in the draft version 3 of `R-100`
- **WHEN** a ProcessEngineer runs `ApproveVersion` on version 3 without `allow_outdated`
- **THEN** the system rejects the command with `E-RCP-024`

#### Scenario: New draft lists the outdated parameters
- **GIVEN** the approved version 2 of `R-100` with the parameters `SPEED` of type `INT` and `ZONE_1` of type `ST_Zone@2`, and the newer revision `ST_Zone@3`
- **WHEN** a ProcessEngineer runs `CreateVersion` on `R-100` based on version 2
- **THEN** the system creates draft version 3
- **AND** the result lists `ZONE_1` and does not list `SPEED`

## MODIFIED Requirements

### Requirement: Allowed version transitions
The database SHALL allow only the status changes from Draft to Approved and from Approved to Retired. It SHALL also allow a deletion time on a draft. It SHALL reject every other status change. The system SHALL reject a command that needs another transition. `ApproveVersion` and `DiscardDraft` SHALL return `E-RCP-007`, `RetireVersion` SHALL return `E-RCP-009`, and a draft command SHALL return `E-RCP-002` (R17, A12, A28, I3, I4).

#### Scenario: Retired version becomes approved
- **GIVEN** the retired version 1 of `R-100`
- **WHEN** an `UPDATE` statement changes the status of version 1 to `approved`
- **THEN** the database rejects the statement with `E-RCP-033`

#### Scenario: Approve a retired version
- **GIVEN** the retired version 1 of `R-100`
- **WHEN** an Administrator runs `ApproveVersion` on version 1
- **THEN** the system rejects the command with `E-RCP-007`
- **AND** version 1 stays retired

#### Scenario: Approve a discarded draft
- **GIVEN** the draft version 3 of `R-100`, which a ProcessEngineer discarded
- **WHEN** an Administrator runs `ApproveVersion` on version 3
- **THEN** the system rejects the command with `E-RCP-007`

#### Scenario: Retire a retired version
- **GIVEN** the retired version 1 of `R-100`
- **WHEN** an Administrator runs `RetireVersion` on version 1
- **THEN** the system rejects the command with `E-RCP-009`

#### Scenario: Discard a retired version
- **GIVEN** the retired version 1 of `R-100`
- **WHEN** a ProcessEngineer runs `DiscardDraft` on version 1
- **THEN** the system rejects the command with `E-RCP-007`

#### Scenario: Change a parameter of a discarded draft
- **GIVEN** the draft version 3 of `R-100` with the parameter `SPEED`, which a ProcessEngineer discarded
- **WHEN** a ProcessEngineer runs `SetParameterValue` on `SPEED` in version 3
- **THEN** the system rejects the command with `E-RCP-002`

### Requirement: Separation of duties at approval
The system SHALL record each user who runs a draft command in `draft_editor`. The system SHALL reject approval by a user who created the draft or who is in `draft_editor`, while `approval.separation_of_duties` is on (R27, A5, A34, I16).

#### Scenario: Creator approves the draft
- **GIVEN** the draft version 3 of `R-100` created by a ProcessEngineer
- **WHEN** the same ProcessEngineer runs `ApproveVersion` on version 3
- **THEN** the system rejects the command with `E-RCP-015`

#### Scenario: Editor approves the draft
- **GIVEN** the draft version 3 of `R-100` and an Administrator who ran `SetParameterValue` on it
- **WHEN** the same Administrator runs `ApproveVersion` on version 3
- **THEN** the system rejects the command with `E-RCP-015`

#### Scenario: Separation of duties off
- **GIVEN** the setting `approval.separation_of_duties` with the value `false`
- **AND** the valid draft version 3 of `R-100` created by a ProcessEngineer
- **WHEN** the same ProcessEngineer runs `ApproveVersion` on version 3
- **THEN** the system sets the status of version 3 to `Approved`
