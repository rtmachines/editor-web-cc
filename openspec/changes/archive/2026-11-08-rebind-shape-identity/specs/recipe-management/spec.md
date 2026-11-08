## ADDED Requirements

### Requirement: Rebind keeps a member of one type shape
`RebindTypeRevision` SHALL keep the value and the limits of a member of one type shape. Such a member SHALL have its name in both revisions, and its two types SHALL have one type shape. Two leaf types SHALL have one shape with the same primitive and the same length. The default length of the platform SHALL count as an explicit length. Two array types SHALL need the same dimensions and element types of one shape. Two struct types SHALL need the same row (R09, R19, R31, A29).

#### Scenario: Default length of CodesysIec after an explicit length
- **GIVEN** the machine `M12` with the platform `CodesysIec`, the revision `ST_Text@1` with the member `label` of type `STRING(80)`, and the revision `ST_Text@2` with the member `label` of type `STRING`
- **AND** the parameter `LABEL_1` of type `ST_Text@1` with the value `"ABC"` and the process limit `max_length` 40 on `.label` in a draft version
- **WHEN** a ProcessEngineer runs `RebindTypeRevision` from `ST_Text@1` to `ST_Text@2` without new member values
- **THEN** the type of `LABEL_1` becomes `ST_Text@2`
- **AND** the value of `.label` stays `"ABC"` and the limit on `.label` keeps its content
- **AND** the audit entry `TypeRevisionRebound` of `LABEL_1` holds no change of a value leaf or a limit row

#### Scenario: Explicit length after the default length of CodesysIec
- **GIVEN** the machine `M12` with the platform `CodesysIec`, the revision `ST_Text@1` with the member `label` of type `STRING`, and the revision `ST_Text@2` with the member `label` of type `STRING(80)`
- **AND** the parameter `LABEL_1` of type `ST_Text@1` with the value `"ABC"` and the process limit `max_length` 40 on `.label` in a draft version
- **WHEN** a ProcessEngineer runs `RebindTypeRevision` from `ST_Text@1` to `ST_Text@2` without new member values
- **THEN** the type of `LABEL_1` becomes `ST_Text@2`
- **AND** the value of `.label` stays `"ABC"` and the limit on `.label` keeps its content

#### Scenario: Default length of S7_1500
- **GIVEN** a machine with the platform `S7_1500`, the revision `ST_Text@1` with the member `label` of type `STRING(254)`, and the revision `ST_Text@2` with the member `label` of type `STRING`
- **AND** the parameter `LABEL_1` of type `ST_Text@1` with the value `"ABC"` in a draft version
- **WHEN** a ProcessEngineer runs `RebindTypeRevision` from `ST_Text@1` to `ST_Text@2` without new member values
- **THEN** the type of `LABEL_1` becomes `ST_Text@2` and the value of `.label` stays `"ABC"`

#### Scenario: Default length of WSTRING on TwinCat3
- **GIVEN** a machine with the platform `TwinCat3`, the revision `ST_Text@1` with the member `label` of type `WSTRING(80)`, and the revision `ST_Text@2` with the member `label` of type `WSTRING`
- **AND** the parameter `LABEL_1` of type `ST_Text@1` with the value `"ABC"` in a draft version
- **WHEN** a ProcessEngineer runs `RebindTypeRevision` from `ST_Text@1` to `ST_Text@2` without new member values
- **THEN** the type of `LABEL_1` becomes `ST_Text@2` and the value of `.label` stays `"ABC"`

#### Scenario: Rebind form without a field for a kept member
- **GIVEN** the machine `M12` with the platform `CodesysIec`, the revision `ST_Text@1` with the member `label` of type `STRING(80)`, and the revision `ST_Text@2` with the member `label` of type `STRING`
- **AND** the parameter `LABEL_1` of type `ST_Text@1` in a draft version
- **WHEN** a ProcessEngineer opens the version page of the draft
- **THEN** the rebind form from `ST_Text@1` to `ST_Text@2` holds no field for `.label` of `LABEL_1`

#### Scenario: Another length is a new member
- **GIVEN** the machine `M12` with the platform `CodesysIec`, the revision `ST_Text@1` with the member `label` of type `STRING(80)`, and the revision `ST_Text@2` with the member `label` of type `STRING(81)`
- **AND** the parameter `LABEL_1` of type `ST_Text@1` with the process limit `max_length` 40 on `.label` in a draft version
- **WHEN** a ProcessEngineer runs `RebindTypeRevision` from `ST_Text@1` to `ST_Text@2` without new member values
- **THEN** the system rejects the command with `E-RCP-028` for `.label` and `E-RCP-029` for the limit on `.label`
- **AND** the type of `LABEL_1` stays `ST_Text@1`

#### Scenario: Array member with the default length
- **GIVEN** the machine `M12` with the platform `CodesysIec`, the revision `ST_Text@1` with the member `lines` of type `ARRAY[1..2] OF STRING(80)`, and the revision `ST_Text@2` with the member `lines` of type `ARRAY[1..2] OF STRING`
- **AND** the parameter `LABEL_1` of type `ST_Text@1` with the value `["A", "B"]` for `.lines` in a draft version
- **WHEN** a ProcessEngineer runs `RebindTypeRevision` from `ST_Text@1` to `ST_Text@2` without new member values
- **THEN** the type of `LABEL_1` becomes `ST_Text@2` and the value of `.lines` stays `["A", "B"]`

#### Scenario: Member of a newer struct revision is a new member
- **GIVEN** the revisions `ST_Zone@2` and `ST_Zone@3` with equal members, the revision `ST_Line@1` with the member `zone` of type `ST_Zone@2`, and the revision `ST_Line@2` with the member `zone` of type `ST_Zone@3`
- **AND** the parameter `LINE_1` of type `ST_Line@1` in a draft version
- **WHEN** a ProcessEngineer runs `RebindTypeRevision` from `ST_Line@1` to `ST_Line@2` without new member values
- **THEN** the system rejects the command with `E-RCP-028` for each leaf of `.zone`
- **AND** the type of `LINE_1` stays `ST_Line@1`
