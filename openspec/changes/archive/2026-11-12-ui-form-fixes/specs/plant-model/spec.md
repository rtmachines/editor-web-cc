## MODIFIED Requirements

### Requirement: Struct revision
A change of a struct SHALL create a new revision with the next revision number. A parameter that uses the old revision SHALL keep it (R19, A29, I22).

`DefineUdt` in the mode `Revision` SHALL refuse members equal to the members of the latest revision with `E-PLT-052`. The command SHALL then write nothing (R06, R07, R19, A29, ADR-0159). Two member lists are equal when they have the same count and the same member name and type at each position. The name comparison uses the exact letter case (ADR-0159).

#### Scenario: New revision
- **GIVEN** the machine `M12` with the struct `ST_Zone` in the revision 2
- **AND** a draft with a parameter of the type `ST_Zone@2`
- **WHEN** a ProcessEngineer runs `DefineUdt` for a new revision of `ST_Zone`
- **THEN** the system creates the revision 3 of `ST_Zone`, which supersedes the revision 2
- **AND** the parameter keeps the type `ST_Zone@2`
- **AND** the system marks the parameter as outdated
- **AND** the audit log holds the action `UdtDefined`

#### Scenario: Unchanged revision
- **GIVEN** the machine `M12` with the struct `ST_Zone` in the revision 2
- **WHEN** a ProcessEngineer runs `DefineUdt` for a new revision of `ST_Zone` with the members of `ST_Zone@2` in the same order
- **THEN** the system rejects the command with `E-PLT-052`
- **AND** the system creates no type row and no derived type
- **AND** the system writes no audit entry

#### Scenario: Changed letter case of a member name
- **GIVEN** the machine `M12` with the struct `ST_Zone` in the revision 2 with the member `temp_max`
- **WHEN** a ProcessEngineer runs `DefineUdt` for a new revision of `ST_Zone` with the members of `ST_Zone@2` and the name `Temp_Max` in place of `temp_max`
- **THEN** the system creates the revision 3 of `ST_Zone`
