## ADDED Requirements

### Requirement: Machine list
The query `ListMachines` SHALL return the live machines in the order of the code number, `page.machines` rows per page. A search text SHALL match a part of the code or of the name, with no difference between upper and lower case. With `show_archived`, the list SHALL also hold the archived machines, each with its identifier (R05, R24, A32).

#### Scenario: Order of the code number
- **GIVEN** the live machines `M10`, `M2`, and `M1`
- **WHEN** the Operator `mhoffman` runs `ListMachines` with no search text
- **THEN** the list holds `M1`, `M2`, and `M10` in this order

#### Scenario: Search text with another case
- **GIVEN** the live machine `M4` with the name `Piec hartowniczy` and the live machine `M5` with the name `Prasa`
- **WHEN** a ProcessEngineer runs `ListMachines` with the search text `PIEC`
- **THEN** the list holds `M4` and not `M5`

#### Scenario: Archived machine
- **GIVEN** the archived machine `M12` and the live machine `M12` that replaces it
- **WHEN** a ProcessEngineer runs `ListMachines` with `show_archived`
- **THEN** the list holds both machines `M12`
- **AND** the row of the archived machine holds its identifier and its archive time

#### Scenario: Second page
- **GIVEN** 101 live machines and `page.machines` set to 100
- **WHEN** a ProcessEngineer runs `ListMachines` for the second page
- **THEN** the list holds one machine and no next page

#### Scenario: Page after the end
- **GIVEN** 3 live machines and `page.machines` set to 100
- **WHEN** a ProcessEngineer runs `ListMachines` for the third page
- **THEN** the list holds no machine and no next page

### Requirement: Archive message
The query `ListMachines` SHALL carry a machine code as an archive message only when an archived machine has that code and no live machine has it. The query `GetMachine` SHALL carry a unit path as an archive message only when the machine holds that unit archived (R05, R06, R39).

#### Scenario: Message of an archived machine
- **GIVEN** the archived machine `M7` and no live machine `M7`
- **WHEN** a ProcessEngineer runs `ListMachines` with the archive message `M7`
- **THEN** the result carries the archive message `M7`

#### Scenario: Code of a live machine
- **GIVEN** the live machine `M7`
- **WHEN** a ProcessEngineer runs `ListMachines` with the archive message `M7`
- **THEN** the result carries no archive message

#### Scenario: Path of a live unit
- **GIVEN** the live machine `M7` with the live unit `U1.2`
- **WHEN** a ProcessEngineer runs `GetMachine` for `M7` with the archive message `U1.2`
- **THEN** the result carries no archive message

### Requirement: Unit tree of a machine
The queries `GetMachine` and `GetUnitTree` SHALL return the machine with the units of its tree. For a live machine, the tree SHALL hold the live units. For an archived machine that `GetMachine` finds by its identifier, the tree SHALL hold every unit of the machine (R06, R07, A30, A32).

#### Scenario: Tree of five levels
- **GIVEN** the live machine `M7` with the live units `U1`, `U1.2`, `U1.2.4`, `U1.2.4.5`, and `U1.2.4.5.6`
- **WHEN** the Operator `mhoffman` runs `GetUnitTree` for `M7`
- **THEN** the tree holds the five units, each under its parent

#### Scenario: Archived unit of a live machine
- **GIVEN** the live machine `M7` with the live unit `U1` and the archived unit `U2`
- **WHEN** a ProcessEngineer runs `GetMachine` for the code `M7`
- **THEN** the tree holds `U1` and not `U2`

#### Scenario: Archived machine by identifier
- **GIVEN** the archived machine `M12` with the unit `U1` and the archived unit `U2`
- **WHEN** a ProcessEngineer runs `GetMachine` with the identifier of the archived `M12`
- **THEN** the tree holds `U1` and `U2`
- **AND** `U2` carries its archive time

#### Scenario: Unknown machine code
- **GIVEN** no live machine `M99`
- **WHEN** a ProcessEngineer runs `GetUnitTree` for `M99`
- **THEN** the system rejects the query with `E-WEB-005`

### Requirement: Proposal of a new unit
The query `GetUnitForm` SHALL offer as a parent each live unit of depth 1 to 4. It SHALL propose the lowest segment from 1 to 64 that no unit under the parent uses, live or archived. A parent that is not a live unit SHALL give `E-WEB-005`. A live parent of depth 5 SHALL give `E-PLT-002` (R06, R07, A30).

#### Scenario: Segment after an archived unit
- **GIVEN** the live machine `M7` with the live unit `U1.1` and the archived unit `U1.2`
- **WHEN** a ProcessEngineer runs `GetUnitForm` for `M7` with the parent `U1`
- **THEN** the form proposes the segment `3`

#### Scenario: All segments in use
- **GIVEN** the live machine `M7` with 64 units under `U1`
- **WHEN** a ProcessEngineer runs `GetUnitForm` for `M7` with the parent `U1`
- **THEN** the form proposes no segment

#### Scenario: Parent of depth 5
- **GIVEN** the live machine `M7` with the live unit `U1.2.4.5.6`
- **WHEN** a ProcessEngineer runs `GetUnitForm` for `M7` with the parent `U1.2.4.5.6`
- **THEN** the system rejects the query with `E-PLT-002`

#### Scenario: List of parents
- **GIVEN** the live machine `M7` with the live unit `U1.2.4.5.6`
- **WHEN** a ProcessEngineer runs `GetUnitForm` for `M7`
- **THEN** the list of parents does not hold `U1.2.4.5.6`

#### Scenario: Form without the permission
- **GIVEN** the live machine `M7`
- **WHEN** the Operator `mhoffman` runs `GetUnitForm` for `M7`
- **THEN** the system rejects the query with `E-WEB-002`

### Requirement: Type list of a machine
The query `ListTypes` SHALL return one row per struct revision of the machine. The rows SHALL come in the byte order of the struct name. Inside one name, the latest revision SHALL come first and SHALL carry a mark (R13, R19).

#### Scenario: Two revisions of one struct
- **GIVEN** the machine `M12` with the struct `ST_Zone` in the revisions 1 and 2 and the struct `ST_Axis` in the revision 1
- **WHEN** the Operator `mhoffman` runs `ListTypes` for `M12`
- **THEN** the rows are `ST_Axis@1`, `ST_Zone@2`, and `ST_Zone@1` in this order
- **AND** the rows `ST_Axis@1` and `ST_Zone@2` carry the mark of the latest revision

### Requirement: Type tree of a revision
The query `GetTypeTree` SHALL return the resolved type tree of the latest revision of a struct, or of the revision that the request names. A struct or a revision that the machine does not hold SHALL give `E-WEB-005` (R13, R19).

#### Scenario: Earlier revision
- **GIVEN** the machine `M12` with the struct `ST_Zone` in the revisions 1 and 2
- **WHEN** the Operator `mhoffman` runs `GetTypeTree` for `ST_Zone` with the revision 1
- **THEN** the system returns the type tree of `ST_Zone@1`
- **AND** the result lists the revisions 2 and 1

#### Scenario: Unknown revision
- **GIVEN** the machine `M12` with the struct `ST_Zone` in the revision 2
- **WHEN** the Operator `mhoffman` runs `GetTypeTree` for `ST_Zone` with the revision 9
- **THEN** the system rejects the query with `E-WEB-005`

### Requirement: Form of a new revision
The query `GetUdtForm` with the name of a struct SHALL return the members of the latest revision of that struct. Each member SHALL carry its name and its type as a type expression that parses to the same type. The type proposals of that form SHALL hold no revision of that struct (R13, R19, ADR-0038).

#### Scenario: Members of the latest revision
- **GIVEN** the machine `M12` with the struct `ST_Zone@2` and its members `temp_max` of the type `REAL` and `names` of the type `ARRAY[1..4] OF STRING(16)`
- **WHEN** a ProcessEngineer runs `GetUdtForm` for `M12` with the name `ST_Zone`
- **THEN** the form holds `temp_max` with `REAL` and `names` with `ARRAY[1..4] OF STRING(16)` in this order

#### Scenario: Proposals without the revised struct
- **GIVEN** the machine `M12` with the struct `ST_Zone` in the revisions 1 and 2
- **WHEN** a ProcessEngineer runs `GetUdtForm` for `M12` with the name `ST_Zone`
- **THEN** the type proposals hold no revision of `ST_Zone`

#### Scenario: Unknown struct
- **GIVEN** the machine `M12` without the struct `ST_Axis`
- **WHEN** a ProcessEngineer runs `GetUdtForm` for `M12` with the name `ST_Axis`
- **THEN** the system rejects the query with `E-WEB-005`

### Requirement: Type usage
The query `GetTypeUsage` SHALL return each parameter whose type uses the struct revision, directly or through another type, once. It SHALL list the versions `Draft`, `Approved`, and `Retired` of the recipes that are not deleted, `page.type_usage` rows per page. A row SHALL name the recipe, the version, its state, the tag, and the type of the parameter (R19, A29).

#### Scenario: Use through another struct
- **GIVEN** the machine `M12` with the struct `ST_Line@1` that has a member of the type `ARRAY[1..4] OF ST_Zone@2`
- **AND** the draft version 3 of the recipe `R-100` with the parameter `LINE` of the type `ST_Line@1`
- **WHEN** the Operator `mhoffman` runs `GetTypeUsage` for `ST_Zone` with the revision 2
- **THEN** the list holds the row `R-100`, version 3, `Draft`, `LINE`, `ST_Line@1`

#### Scenario: Two members of one struct
- **GIVEN** the struct `ST_Pair@1` with the members `a` and `b`, both of the type `ST_Zone@2`
- **AND** the draft version 1 of the recipe `R-400` with the parameter `PAIR` of the type `ST_Pair@1`
- **WHEN** a ProcessEngineer runs `GetTypeUsage` for `ST_Zone` with the revision 2
- **THEN** the list holds one row for the parameter `PAIR`

#### Scenario: Deleted recipe and discarded draft
- **GIVEN** the deleted recipe `R-200` with a parameter of the type `ST_Zone@2`
- **AND** a discarded draft of the recipe `R-300` with a parameter of the type `ST_Zone@2`
- **WHEN** a ProcessEngineer runs `GetTypeUsage` for `ST_Zone` with the revision 2
- **THEN** the list holds no row of `R-200` and no row of the discarded draft

#### Scenario: Revision without use
- **GIVEN** the struct `ST_Zone@1` that no parameter uses
- **WHEN** a ProcessEngineer runs `GetTypeUsage` for `ST_Zone` with the revision 1
- **THEN** the list is empty

## MODIFIED Requirements

### Requirement: Struct without a cycle
A struct SHALL NOT contain itself, directly or through its members. Two revisions of one struct SHALL count as one type for this rule (R13, I8, ADR-0038).

#### Scenario: Circular member
- **GIVEN** the machine `M12`
- **WHEN** a ProcessEngineer runs `DefineUdt` for `ST_Loop` with a member whose type tree contains `ST_Loop`
- **THEN** the system rejects the command with `E-PLT-005`
- **AND** the system creates no type

#### Scenario: Earlier revision of the same struct
- **GIVEN** the machine `M12` with the struct `ST_Zone` in the revisions 1 and 2
- **WHEN** a ProcessEngineer runs `DefineUdt` for a new revision of `ST_Zone` with a member of the type `ST_Zone@1`
- **THEN** the system rejects the command with `E-PLT-005` for `ST_Zone`
- **AND** the system creates no type

#### Scenario: One struct on two paths
- **GIVEN** the machine `M12` with the struct `ST_Zone@2`
- **WHEN** a ProcessEngineer runs `DefineUdt` for `ST_Pair` with the members `a` and `b`, both of the type `ST_Zone@2`
- **THEN** the system creates `ST_Pair@1`
